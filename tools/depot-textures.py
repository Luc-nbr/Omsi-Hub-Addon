#!/usr/bin/env python3
"""The 3D depot's textures, made into the one picture the renderer reads
(ui/img/depot/detail.jpg): a square layer each, one under the other, in the order of
`MATERIALS` in ui/js/company/depot3d/palette.js.

Run:  python3 tools/depot-textures.py <folder>
The folder holds a picture per material named after it (`asphalt.png`, `cracks.png`, ...): the
ones made for Omsi-Hub with Higgsfield (Luc's account; the prompts are in docs/DEPOT-TEXTURES.md).

Each is made a greyscale picture that tints the colour it lies on rather than replacing it:
* a surface ("plane"): its light evened out (what is broader than an eighth of it taken away),
  its contrast set alike for all, and made to repeat without a seam - its last strip faded into
  its first, the strip chosen so a pattern's rows and columns (bricks, tiles, ribs) still meet;
* the road's paint: its contrast stretched (the paint white, the asphalt showing through dark),
  made to repeat the same way;
* a mark on the ground ("decal"): its white made the picture's white (no change where nothing
  is), its edges faded out, not repeated.
Needs Pillow and NumPy.
"""

import pathlib
import re
import sys

import numpy as np
from PIL import Image, ImageFilter

ROOT = pathlib.Path(__file__).resolve().parent.parent
PALETTE = ROOT / "ui" / "js" / "company" / "depot3d" / "palette.js"
OUT = ROOT / "ui" / "img" / "depot" / "detail.jpg"
SIDE = 512


def materials():
    """(name, kind) of every layer, from palette.js."""
    text = PALETTE.read_text(encoding="utf-8")
    block = text[text.index("export const MATERIALS") :]
    block = block[: block.index("];")]
    return re.findall(r"\['(\w+)', [\d.]+, [\d.]+, '(\w+)'\]", block)


def period(profile, lo, hi):
    """The pattern's repeat along a line (its mean), in pixels between lo and hi; None when it
    has none worth keeping."""
    p = profile - profile.mean()
    if not p.any():
        return None
    ac = np.correlate(p, p, "full")[len(p) - 1 :]
    ac /= ac[0]
    k = lo + int(np.argmax(ac[lo:hi]))
    return k if ac[k] > 0.25 else None


def seamless(a, axis):
    """`a` made to repeat along `axis`: its last strip faded into its first, the picture that
    much shorter. The strip is chosen so a regular pattern keeps its rows (a whole number of
    repeats remains)."""
    n = a.shape[axis]
    lo, hi = n // 12, n // 4
    rep = period(a.mean(axis=1 - axis), 8, n // 3)
    strip = n // 8
    if rep:
        # the remaining length a whole number of repeats, the strip in [lo, hi]
        best = min(range(lo, hi), key=lambda b: ((n - b) % rep) and min((n - b) % rep, rep - (n - b) % rep))
        strip = best
    a = np.moveaxis(a, axis, 0)
    keep = a[: n - strip].copy()
    w = (np.arange(strip) / strip)[:, None]
    # the first `strip` lines become the faded end of the last ones
    keep[:strip] = a[n - strip :] * (1 - w) + a[:strip] * w
    return np.moveaxis(keep, 0, axis)


def plane(img, flat=True):
    a = np.asarray(img.convert("L"), dtype=np.float64)
    if flat:
        broad = np.asarray(img.convert("L").filter(ImageFilter.GaussianBlur(img.width / 16)), dtype=np.float64)
        a = a - broad
        a = a / max(a.std(), 1e-3) * 38 + 128
    else:
        lo, hi = np.percentile(a, [2, 98])
        a = (a - lo) / max(hi - lo, 1) * 255
    a = seamless(seamless(a, 0), 1)
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).resize((SIDE, SIDE), Image.LANCZOS)


def decal(img):
    a = np.asarray(img.convert("L").resize((SIDE, SIDE), Image.LANCZOS), dtype=np.float64)
    white = np.percentile(a, 90)
    a = np.clip(a / max(white, 1) * 255, 0, 255)
    # (the edges clear)
    r = np.minimum(np.arange(SIDE), np.arange(SIDE)[::-1]) / (SIDE * 0.08)
    edge = np.clip(np.minimum.outer(r, r), 0, 1)
    a = 255 - (255 - a) * edge
    return Image.fromarray(a.astype(np.uint8))


def main(folder):
    src = pathlib.Path(folder)
    layers = []
    for name, kind in materials():
        path = next((p for p in sorted(src.glob(name + ".*"))), None)
        if path is None:
            print(f"missing: {name}", file=sys.stderr)
            return 1
        img = Image.open(path)
        layers.append(decal(img) if kind == "decal" else plane(img, flat=kind == "plane"))
    out = Image.new("L", (SIDE, SIDE * len(layers)))
    for k, layer in enumerate(layers):
        out.paste(layer, (0, k * SIDE))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    out.save(OUT, quality=88, optimize=True, progressive=False)
    print(f"{OUT.relative_to(ROOT)}: {len(layers)} layers, {OUT.stat().st_size // 1024} KB")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
