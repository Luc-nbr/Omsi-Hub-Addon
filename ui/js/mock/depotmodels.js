// The mock's depot models (`b3d_depot_has`, the bytes `b3d_depot`): in a browser there is no
// OMSI 2 to bake a bus of, so a bus is drawn here as a plain one would look - its body in its
// livery's colours, the windows, the doors on the door side, the wheels, the windscreen and the
// display - into the five views of `bake.js`, and its hull made from them as the app makes it.
// For working on the 3D depot without the app.

import { ATLAS, bakeViews, hullFrom, packModel } from '../hub/bus3d/bake.js';
import { project } from '../hub/bus3d/math.js';

/** The liveries the mock's buses wear: body, band. */
const PAINTS = {
  grundorf: [[242, 140, 40], [40, 53, 147]],
  weiss: [[238, 240, 243], [150, 156, 166]],
  bvg: [[246, 200, 30], [30, 30, 30]],
  hamburg: [[214, 38, 38], [245, 245, 245]],
};
const paintOf = (name) => PAINTS[String(name || '').toLowerCase()] || PAINTS.weiss;

/** The bus's shape: its length, and where it is in each kind of view. */
function shape(file) {
  const L = /NG272|articul|gelenk/i.test(file) ? 18 : 12;
  const half = L / 2;
  const top = (z) => (z > half - 0.6 ? 3.0 - (z - (half - 0.6)) * 0.55 : 3.0);
  const axles = L > 15 ? [half - 2.7, -0.6, -half + 2.6] : [half - 2.7, -half + 3.1];
  const wheel = (y, z) => y < 0.4 && axles.some((a) => Math.abs(z - a) < 0.52);
  const mirror = (z) => z > half - 0.75 && z < half - 0.45;
  return {
    L,
    box: { min: [-1.55, 0, -half], max: [1.55, 3.1, half] },
    side: (y, z) => Math.abs(z) < half && ((y > 0.32 && y < top(z)) || wheel(y, z) || (mirror(z) && y > 1.9 && y < 2.4)),
    roof: (x, z) => (Math.abs(x) < 1.25 && Math.abs(z) < half) || (mirror(z) && Math.abs(x) < 1.55),
    end: (x, y) => (Math.abs(x) < 1.25 && y > 0.32 && y < 3.0) || (Math.abs(x) < 1.55 && y > 1.9 && y < 2.4 && Math.abs(x) > 1.25),
    top,
    wheel,
    axles,
  };
}

/** The colour of the bus at a point of a view (r, g, b). */
function colourAt(v, p, s, paint) {
  const [body, band] = paint;
  const [x, y, z] = p;
  const half = s.L / 2;
  if (v === 'roof') {
    if (Math.abs(x) > 1.25) return [30, 30, 34];
    if (Math.abs(z + 0.5) < 1.6 && Math.abs(x) < 0.85) return [190, 194, 200];
    return body.map((c) => Math.round(c * 0.92 + 18));
  }
  if (v === 'front' || v === 'rear') {
    if (Math.abs(x) > 1.25) return [30, 30, 34];
    if (v === 'front' && y > 2.55 && y < 2.85 && Math.abs(x) < 1.05) return [255, 176, 60];
    if (y > (v === 'front' ? 0.95 : 1.3) && y < 2.45 && Math.abs(x) < 1.12) return [28, 36, 48];
    if (y < 0.75) return [44, 46, 52];
    if (y > 0.9 && y < 1.0 && Math.abs(x) > 0.85) return [250, 248, 230];
    return body;
  }
  // the sides
  if (s.wheel(y, z) || (y < 0.75 && s.axles.some((a) => Math.abs(z - a) < 0.6))) return Math.hypot(y - 0.5, Math.min(...s.axles.map((a) => Math.abs(z - a)))) < 0.22 ? [150, 150, 156] : [24, 24, 28];
  if (y > 1.9 && y < 2.4 && z > half - 0.75) return [30, 30, 34];
  const doors = v === 'right' ? (s.L > 15 ? [half - 1.2, 0.8, -half + 4.6] : [half - 1.2, -0.4]) : [];
  if (doors.some((d) => Math.abs(z - d) < 0.65) && y > 0.4 && y < 2.55) return Math.abs(y - 1.5) < 0.9 && doors.some((d) => Math.abs(z - d) < 0.58) ? [34, 42, 56] : [70, 72, 80];
  if (y > 1.15 && y < 2.4 && z < half - 0.3 && z > -half + 0.4) {
    // the windows, a pillar every 1.6 m
    const k = ((z + half) % 1.6) / 1.6;
    return k < 0.06 ? body : [30, 38, 52];
  }
  if (y > 0.75 && y < 0.95) return band;
  if (y < 0.5) return [52, 54, 60];
  return body;
}

const made = new Map();

/** The model and atlas of `file` in `paint`: `{ model, png }` (the PNG's bytes, a promise). */
function make(file, paint) {
  const key = `${file}|${paint}`;
  if (made.has(key)) return made.get(key);
  const s = shape(file);
  const p = paintOf(paint);
  const views = bakeViews(s.box);
  const canvas = new OffscreenCanvas(ATLAS, ATLAS);
  const ctx = canvas.getContext('2d');
  const masks = {};
  for (const v of views) {
    const { w, h } = v.rect;
    const mask = new Uint8Array(w * h);
    const img = new ImageData(w, h);
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const nx = ((x + 0.5) / w) * 2 - 1;
        const ny = 1 - ((y + 0.5) / h) * 2;
        const q = project(v.inv, [nx, ny, 0]);
        const on = v.name === 'roof' ? s.roof(q[0], q[2]) : v.name === 'front' || v.name === 'rear' ? s.end(q[0], q[1]) : s.side(q[1], q[2]);
        if (!on) continue;
        mask[y * w + x] = 1;
        const c = colourAt(v.name, q, s, p);
        const o = (y * w + x) * 4;
        img.data.set([c[0], c[1], c[2], 255], o);
      }
    masks[v.name] = mask;
    ctx.putImageData(img, v.rect.x, v.rect.y);
  }
  const model = packModel(hullFrom(s.box, views, masks));
  const out = { model, png: canvas.convertToBlob({ type: 'image/png' }).then((b) => b.arrayBuffer()) };
  made.set(key, out);
  return out;
}

/** `b3d_depot_has { bus, paint }`. */
export const depotHas = (a) => (typeof OffscreenCanvas === 'undefined' ? { has: false } : { has: true, model: make(a.bus, a.paint).model });

/** The bytes `b3d_depot { bus, paint }`. */
export const depotAtlas = (a) => make(a.bus, a.paint).png;
