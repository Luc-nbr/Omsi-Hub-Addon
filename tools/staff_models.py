"""The depot's staff figures (`ui/models/staff/<role>.bin`) from their GLBs.

Each role was made in Higgsfield (concept by GPT Image, mesh by Meshy 7) and rigged twice by
Meshy, once with its idle and once with its walk: `<role>-idle.glb` and `<role>-walk.glb`, the
same mesh with a skeleton of 24 bones each. The two rigs differ a little (each is guessed again),
so a figure keeps both skins: the idle is drawn on the idle's bones, the walk on the walk's.

What it writes, little-endian (read by `ui/js/company/depot3d/figures.js`):

    "OHSF", version (u32), the JSON's length (u32), the JSON (padded to 4), the bytes

The JSON names the parts of the bytes (`parts`: name -> [offset, length]): `position` (f32 x3),
`normal` (i8 x4, normalised), `uv` (u16 x2, normalised), `joints_idle` and `joints_walk` (u8 x4),
`weights_idle` and `weights_walk` (u8 x4, normalised), `index` (u16 x3 a triangle), `bones`
(f32: a row a frame, each joint's matrix as its first three rows) and `texture` (a JPEG). `clips`
says where each clip's frames start in `bones`, how many there are and how long it lasts; `fps`
how many frames a second; `height` the figure's height in metres; `speed` how fast the walk goes
(metres a second, from the feet), so the feet do not slide.

    python3 tools/staff_models.py <folder with the GLBs> ui/models/staff
"""

import io
import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROLES = ["driver", "mechanic", "researcher", "cleaner", "service", "inspector"]
FPS = 15
TEXTURE = 512


def load(path):
    b = Path(path).read_bytes()
    (jlen,) = struct.unpack("<I", b[12:16])
    j = json.loads(b[20 : 20 + jlen])
    (blen,) = struct.unpack("<I", b[20 + jlen : 24 + jlen])
    return j, b[28 + jlen : 28 + jlen + blen]


def accessor(j, blob, i):
    a = j["accessors"][i]
    bv = j["bufferViews"][a["bufferView"]]
    n = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}[a["type"]]
    dt = {5126: np.float32, 5125: np.uint32, 5123: np.uint16, 5121: np.uint8}[a["componentType"]]
    if bv.get("byteStride") not in (None, n * np.dtype(dt).itemsize):
        raise ValueError("interleaved buffers are not read")
    out = np.frombuffer(blob, dtype=dt, count=a["count"] * n, offset=bv.get("byteOffset", 0) + a.get("byteOffset", 0)).reshape(a["count"], n)
    if a.get("normalized"):
        out = out.astype(np.float32) / np.iinfo(dt).max
    return out


def quat_matrix(q):
    x, y, z, w = q
    return np.array(
        [
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ]
    )


def trs(t, r, s):
    m = np.eye(4)
    m[:3, :3] = quat_matrix(r) * np.asarray(s)
    m[:3, 3] = t
    return m


def sample(times, values, t, path):
    """A channel at time t (linear; spherical for rotations)."""
    if t <= times[0]:
        return values[0]
    if t >= times[-1]:
        return values[-1]
    k = int(np.searchsorted(times, t)) - 1
    f = (t - times[k]) / (times[k + 1] - times[k])
    a, b = values[k], values[k + 1]
    if path != "rotation":
        return a + (b - a) * f
    if np.dot(a, b) < 0:
        b = -b
    q = a + (b - a) * f
    return q / np.linalg.norm(q)


class Rig:
    """A GLB's skeleton, its skin and its one clip."""

    def __init__(self, path):
        self.j, self.blob = load(path)
        j = self.j
        self.parent = {}
        for k, n in enumerate(j["nodes"]):
            for c in n.get("children", []):
                self.parent[c] = k
        skin = j["skins"][0]
        self.joints = skin["joints"]
        self.ibm = accessor(j, self.blob, skin["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
        prim = j["meshes"][0]["primitives"][0]
        self.prim = prim
        anim = j["animations"][0]
        self.channels = []
        self.length = 0.0
        for c in anim["channels"]:
            s = anim["samplers"][c["sampler"]]
            times = accessor(j, self.blob, s["input"])[:, 0]
            self.length = max(self.length, float(times[-1]))
            self.channels.append((c["target"]["node"], c["target"]["path"], times, accessor(j, self.blob, s["output"])))

    def attr(self, name):
        return accessor(self.j, self.blob, self.prim["attributes"][name])

    def pose(self, t=None):
        """Each node's local (t, r, s) at time t, as rest with the clip on it (None: at rest)."""
        local = []
        for n in self.j["nodes"]:
            local.append([np.array(n.get("translation", [0, 0, 0]), float), np.array(n.get("rotation", [0, 0, 0, 1]), float), np.array(n.get("scale", [1, 1, 1]), float)])
        for node, path, times, values in self.channels if t is not None else []:
            local[node][{"translation": 0, "rotation": 1, "scale": 2}[path]] = sample(times, values, t, path)
        return local

    def globals(self, local):
        out = {}

        def world(k):
            if k not in out:
                m = trs(*local[k])
                out[k] = world(self.parent[k]) @ m if k in self.parent else m
            return out[k]

        for k in range(len(local)):
            world(k)
        return out

    def bones(self, t=None):
        """The skin's joint matrices at time t (24 x 4 x 4; None: at rest)."""
        g = self.globals(self.pose(t))
        return np.stack([g[jk] @ self.ibm[i] for i, jk in enumerate(self.joints)])

    def joint(self, name, t):
        g = self.globals(self.pose(t))
        k = next(i for i, n in enumerate(self.j["nodes"]) if n.get("name") == name)
        return g[k][:3, 3]

    def texture(self):
        img = self.j["images"][self.j["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["index"]]
        bv = self.j["bufferViews"][img["bufferView"]]
        off = bv.get("byteOffset", 0)
        return Image.open(io.BytesIO(self.blob[off : off + bv["byteLength"]])).convert("RGB")


def frames(rig):
    """The clip at FPS, the last frame left out where it is the first again (it loops)."""
    n = max(1, round(rig.length * FPS))
    return [rig.bones(rig.length * k / n) for k in range(n)]


def walk_speed(rig):
    """How fast the walk would go: the feet move back under the body while they stand on the
    ground (the clip walks on the spot), metres a second."""
    n = max(2, round(rig.length * 30))
    dt = rig.length / n
    speeds = []
    for foot in ("LeftFoot", "RightFoot"):
        pts = np.array([rig.joint(foot, rig.length * k / n) for k in range(n + 1)])
        low = pts[:, 1].min() + 0.02
        for k in range(n):
            if pts[k, 1] < low and pts[k + 1, 1] < low:
                speeds.append(-(pts[k + 1, 2] - pts[k, 2]) / dt)
    return float(np.median(speeds)) if speeds else 1.1


def skin_of(rig):
    joints = rig.attr("JOINTS_0").astype(np.uint8)
    w = rig.attr("WEIGHTS_0").astype(np.float64)
    w = w / np.maximum(w.sum(1, keepdims=True), 1e-9)
    q = np.round(w * 255).astype(np.int32)
    # (the bytes sum to 255 again: the rounding's rest on the heaviest)
    q[np.arange(len(q)), q.argmax(1)] += 255 - q.sum(1)
    return joints, q.astype(np.uint8)


def convert(role, src, out):
    idle = Rig(src / f"{role}-idle.glb")
    walk = Rig(src / f"{role}-walk.glb")
    pos = walk.attr("POSITION").astype(np.float32)
    if not np.allclose(pos, idle.attr("POSITION"), atol=1e-4):
        raise ValueError(f"{role}: the two rigs are not of the same mesh")
    # the bind pose is the mesh as it is: a check that the bones are read right
    jw, ww = skin_of(walk)
    for rig, joints, weights in ((idle, *skin_of(idle)), (walk, jw, ww)):
        skinned = np.einsum("vk,vkij,vj->vi", weights / 255.0, rig.bones()[joints], np.c_[pos, np.ones(len(pos))])[:, :3]
        if np.abs(skinned - pos).max() > 1e-3:
            raise ValueError(f"{role}: the bones at rest do not give the mesh")
    nrm = walk.attr("NORMAL")
    nrm = np.round(nrm / np.maximum(np.linalg.norm(nrm, axis=1, keepdims=True), 1e-9) * 127).astype(np.int8)
    nrm = np.c_[nrm, np.zeros(len(nrm), np.int8)]
    uv = np.round(np.clip(walk.attr("TEXCOORD_0"), 0, 1) * 65535).astype(np.uint16)
    ji, wi = skin_of(idle)
    index = accessor(walk.j, walk.blob, walk.prim["indices"]).astype(np.uint16)
    fi, fw = frames(idle), frames(walk)
    bones = np.stack(fi + fw)[:, :, :3, :].astype(np.float32)  # frames x joints x 3 x 4
    pic = walk.texture().resize((TEXTURE, TEXTURE), Image.LANCZOS)
    jpg = io.BytesIO()
    pic.save(jpg, "JPEG", quality=88, optimize=True)
    parts = {}
    blob = bytearray()
    for name, data in [
        ("position", pos.tobytes()),
        ("normal", nrm.tobytes()),
        ("uv", uv.tobytes()),
        ("joints_idle", ji.tobytes()),
        ("weights_idle", wi.tobytes()),
        ("joints_walk", jw.tobytes()),
        ("weights_walk", ww.tobytes()),
        ("index", index.tobytes()),
        ("bones", bones.tobytes()),
        ("texture", jpg.getvalue()),
    ]:
        parts[name] = [len(blob), len(data)]
        blob += data
        blob += b"\0" * (-len(blob) % 4)
    head = {
        "role": role,
        "verts": len(pos),
        "triangles": len(index) // 3,
        "joints": len(walk.joints),
        "fps": FPS,
        "height": round(float(pos[:, 1].max() - pos[:, 1].min()), 3),
        "speed": round(walk_speed(walk), 3),
        "clips": {"idle": {"row": 0, "frames": len(fi), "length": idle.length}, "walk": {"row": len(fi), "frames": len(fw), "length": walk.length}},
        "parts": parts,
    }
    js = json.dumps(head, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    (out / f"{role}.bin").write_bytes(b"OHSF" + struct.pack("<II", 1, len(js)) + js + bytes(blob))
    print(f"  {head['triangles']} triangles, {head['height']} m, walks at {head['speed']} m/s, {(12 + len(js) + len(blob)) / 1e6:.2f} MB")


def main():
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    for role in ROLES:
        convert(role, src, out)


if __name__ == "__main__":
    main()
