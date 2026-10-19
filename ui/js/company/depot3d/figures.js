// The staff's figures on the 3D yard: one model a role (`ui/models/staff/<role>.bin`, made from
// the Higgsfield/Meshy models by `tools/staff_models.py`, which says how the file is laid out),
// each with its idle and its walk baked into bone matrices. Read once a session; a role whose
// file cannot be read is left off the yard.

const MAGIC = 0x4653484f; // "OHSF"

/** role -> the figure as it loads (null when it could not be read). */
const loaded = new Map();

/**
 * The figure of `role`: `{ role, verts, triangles, joints, fps, height, speed, clips, bytes,
 * part(name), bones, image }` - `bytes` the whole file (the renderer draws from it at the
 * offsets `part` gives), `bones` the clips' matrices, `image` its texture.
 */
export function figure(role) {
  if (!loaded.has(role)) loaded.set(role, read(role).catch(() => null));
  return loaded.get(role);
}

async function read(role) {
  const res = await fetch(new URL(`../../../models/staff/${role}.bin`, import.meta.url));
  if (!res.ok) throw new Error(`${role}: ${res.status}`);
  const bytes = await res.arrayBuffer();
  const dv = new DataView(bytes);
  if (dv.getUint32(0, true) !== MAGIC || dv.getUint32(4, true) !== 1) throw new Error(`${role}: not a figure`);
  const size = dv.getUint32(8, true);
  const head = JSON.parse(new TextDecoder().decode(new Uint8Array(bytes, 12, size)));
  const base = 12 + size;
  const part = (name) => [base + head.parts[name][0], head.parts[name][1]];
  const [bo, bl] = part('bones');
  const [to, tl] = part('texture');
  const image = await createImageBitmap(new Blob([new Uint8Array(bytes, to, tl)], { type: 'image/jpeg' }));
  return { ...head, bytes, part, bones: new Float32Array(bytes, bo, bl / 4), image };
}
