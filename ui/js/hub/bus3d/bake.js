// A bus's depot model (the 3D depot, `company/depot3d/`): the bus as the game draws it, in its
// livery, made into a light model a yard of a hundred buses can show. The renderer takes five
// flat pictures of it (orthographic, lit evenly: `draw.js`'s bake) - both sides, the roof, the
// front and the rear - and a mask of each (white where the bus is). From the masks comes its
// shape: along its length, column by column, how high its roof and how low its skirt and wheels
// reach (the sides), and how wide it is (the roof, its mirrors left out); round that a hull of a
// few hundred triangles, its faces wearing the pictures (one atlas).
//
// Nothing here touches the GPU or `window`: the renderer's worker uses it, and the mock.
//
// The frames: the 3D view's world (right-handed, X = -x of the o3d: the door side at -X, the
// nose at +Z). The model comes out as the depot draws a bus: centred on its length and width,
// standing on y = 0, its nose at +z.

import { inverse, lookAt, mul, ortho, project } from './math.js';

/** The atlas: one square picture, the views in their rectangles (pixels, rows from the top). */
export const ATLAS = 1024;
const RECTS = {
  left: { x: 0, y: 0, w: 1024, h: 256 },
  right: { x: 0, y: 256, w: 1024, h: 256 },
  roof: { x: 0, y: 512, w: 1024, h: 256 },
  front: { x: 0, y: 768, w: 384, h: 256 },
  rear: { x: 384, y: 768, w: 384, h: 256 },
};
/** The way the depot models are made: raised when the views, the atlas or the hull change. */
export const BAKE_STYLE = 1;
/** Glass seen through to nothing (the far side's window): dark, slightly blue. */
const GLASS = [22, 28, 38];

/** The five views of `box` ({ min, max } in the world), each fitted to it exactly. */
export function bakeViews(box, scale = 1) {
  const mn = box.min;
  const mx = box.max;
  const mid = [(mn[0] + mx[0]) / 2, (mn[1] + mx[1]) / 2, (mn[2] + mx[2]) / 2];
  const size = [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]];
  const r = Math.max(...size) + 10;
  // (eye: where it looks from; up; how wide and high the picture is in the world)
  const ways = {
    left: { eye: [1, 0, 0], up: [0, 1, 0], b: size[2], h: size[1] },
    right: { eye: [-1, 0, 0], up: [0, 1, 0], b: size[2], h: size[1] },
    roof: { eye: [0, 1, 0], up: [1, 0, 0], b: size[2], h: size[0] },
    front: { eye: [0, 0, 1], up: [0, 1, 0], b: size[0], h: size[1] },
    rear: { eye: [0, 0, -1], up: [0, 1, 0], b: size[0], h: size[1] },
  };
  return Object.entries(ways).map(([name, k]) => {
    const eye = [mid[0] + k.eye[0] * r, mid[1] + k.eye[1] * r, mid[2] + k.eye[2] * r];
    const view = lookAt(eye, mid, k.up);
    const proj = ortho(-k.b / 2, k.b / 2, -k.h / 2, k.h / 2, 0.1, 2 * r);
    const viewProj = mul(proj, view);
    const rect = RECTS[name];
    return { name, atlas: ATLAS * scale, rect: { x: rect.x * scale, y: rect.y * scale, w: rect.w * scale, h: rect.h * scale }, cam: { view, proj, viewProj, eye, target: mid, near: 0.1, far: 2 * r }, inv: inverse(viewProj) };
  });
}

/**
 * A view's picture for the atlas, from what the renderer read (premultiplied, rows from the
 * bottom): rows from the top, the colour as it is, opaque where the mask is - glass seen
 * through to nothing turns dark glass -, and the colour run on a few pixels past the edge so
 * the hull's border never samples the empty background. Also the mask, rows from the top.
 */
export function composeView(colour, maskPx, w, h) {
  const rgba = new Uint8ClampedArray(w * h * 4);
  const mask = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    const from = (h - 1 - y) * w;
    for (let x = 0; x < w; x++) {
      const i = (from + x) * 4;
      const o = (y * w + x) * 4;
      const m = maskPx[i] > 127;
      mask[y * w + x] = m ? 1 : 0;
      if (!m) continue;
      const a = colour[i + 3] / 255;
      // premultiplied over dark glass
      for (let c = 0; c < 3; c++) rgba[o + c] = colour[i + c] + GLASS[c] * (1 - a);
      rgba[o + 3] = 255;
    }
  }
  // (three rounds of running the colour outwards, alpha left at 0)
  for (let round = 0; round < 3; round++) {
    const had = rgba.slice();
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const o = (y * w + x) * 4;
        if (had[o + 3] || had[o] || had[o + 1] || had[o + 2]) continue;
        for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
          const nx = x + dx;
          const ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
          const n = (ny * w + nx) * 4;
          if (had[n] || had[n + 1] || had[n + 2]) {
            rgba[o] = had[n];
            rgba[o + 1] = had[n + 1];
            rgba[o + 2] = had[n + 2];
            break;
          }
        }
      }
  }
  return { rgba, mask };
}

/** A world point of a view's picture (pixel x, y from the top) on the view's middle plane. */
function unproject(v, px, py) {
  const nx = ((px + 0.5) / v.rect.w) * 2 - 1;
  const ny = 1 - ((py + 0.5) / v.rect.h) * 2;
  return project(v.inv, [nx, ny, 0]).slice(0, 3);
}

/** Where a world point lies in a view's picture (pixels, rows from the top). */
function pixel(v, p) {
  const q = project(v.cam.viewProj, p);
  return [(q[0] * 0.5 + 0.5) * v.rect.w, (0.5 - q[1] * 0.5) * v.rect.h];
}

const median = (list) => {
  const s = [...list].sort((a, b) => a - b);
  return s[s.length >> 1];
};

/**
 * The hull of the bus from its views' masks (`masks[name]`: rows from the top, 1 where the bus
 * is), `columns` along its length: `{ verts: Float32Array (x y z, normal, u v), length, width,
 * height }`, or null when the masks show no bus.
 */
export function hullFrom(box, views, masks, columns = 48) {
  const by = Object.fromEntries(views.map((v) => [v.name, v]));
  const roof = by.roof;
  const mid = [(box.min[0] + box.max[0]) / 2, (box.min[1] + box.max[1]) / 2, (box.min[2] + box.max[2]) / 2];
  const cols = [];
  for (let i = 0; i < columns; i++) {
    const z = box.min[2] + ((i + 0.5) / columns) * (box.max[2] - box.min[2]);
    // the sides: the highest and lowest row with the bus in it, either side
    let top = null;
    let bot = null;
    for (const v of [by.left, by.right]) {
      const [px] = pixel(v, [mid[0], mid[1], z]);
      const x = Math.min(v.rect.w - 1, Math.max(0, Math.floor(px)));
      const m = masks[v.name];
      for (let y = 0; y < v.rect.h; y++)
        if (m[y * v.rect.w + x]) {
          const wy = unproject(v, x, y)[1];
          top = top === null ? wy : Math.max(top, wy);
          bot = bot === null ? wy : Math.min(bot, wy);
        }
    }
    // the roof: how far across, from one side to the other
    let lo = null;
    let hi = null;
    {
      const [px] = pixel(roof, [mid[0], mid[1], z]);
      const x = Math.min(roof.rect.w - 1, Math.max(0, Math.floor(px)));
      const m = masks.roof;
      for (let y = 0; y < roof.rect.h; y++)
        if (m[y * roof.rect.w + x]) {
          const wx = unproject(roof, x, y)[0];
          lo = lo === null ? wx : Math.min(lo, wx);
          hi = hi === null ? wx : Math.max(hi, wx);
        }
    }
    cols.push(top === null || lo === null ? null : { z, top, bot, lo, hi });
  }
  const found = cols.filter(Boolean);
  if (found.length < 3) return null;
  // (an empty column - a gap in the mask - takes its neighbours'; the mirrors left out: no
  // column wider than the bus is at most of its length)
  for (let i = 0; i < cols.length; i++) if (!cols[i]) cols[i] = { ...(cols[i - 1] || found[0]), z: box.min[2] + ((i + 0.5) / columns) * (box.max[2] - box.min[2]) };
  const half = median(cols.map((c) => (c.hi - c.lo) / 2)) * 1.01;
  const cx = median(cols.map((c) => (c.hi + c.lo) / 2));
  for (const c of cols) {
    c.lo = Math.max(c.lo, cx - half);
    c.hi = Math.min(c.hi, cx + half);
  }
  const ground = Math.min(...cols.map((c) => c.bot));
  // the ends: the first and last column put on the box's ends
  cols[0].z = box.min[2];
  cols[cols.length - 1].z = box.max[2];

  const out = [];
  const zc = mid[2];
  const put = (p, n, v) => {
    const [px, py] = pixel(v, p);
    const u = (v.rect.x + Math.min(v.rect.w - 0.5, Math.max(0.5, px))) / v.atlas;
    const t = (v.rect.y + Math.min(v.rect.h - 0.5, Math.max(0.5, py))) / v.atlas;
    out.push(p[0] - cx, p[1] - ground, p[2] - zc, n[0], n[1], n[2], u, t);
  };
  const ring = (c) => {
    const r = Math.min(0.2, (c.top - c.bot) / 5, (c.hi - c.lo) / 5);
    return [
      [c.lo, c.bot, c.z],
      [c.lo, c.top - r, c.z],
      [c.lo + r, c.top, c.z],
      [c.hi - r, c.top, c.z],
      [c.hi, c.top - r, c.z],
      [c.hi, c.bot, c.z],
    ];
  };
  // which picture each band round the bus wears: the door side, the roof, the other side
  const band = [by.right, roof, roof, roof, by.left];
  const tri = (a, b, c, v, out0) => {
    const e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    const e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
    const l = Math.hypot(...n) || 1;
    n = n.map((x) => x / l);
    // (outwards: away from the bus's middle line)
    const m = [(a[0] + b[0] + c[0]) / 3 - out0[0], (a[1] + b[1] + c[1]) / 3 - out0[1], 0];
    if (n[0] * m[0] + n[1] * m[1] < 0) n = n.map((x) => -x);
    put(a, n, v);
    put(b, n, v);
    put(c, n, v);
  };
  for (let i = 0; i + 1 < cols.length; i++) {
    const a = ring(cols[i]);
    const b = ring(cols[i + 1]);
    const centre = [(cols[i].lo + cols[i].hi) / 2, (cols[i].bot + cols[i].top) / 2 - 0.4];
    for (let s = 0; s < 5; s++) {
      tri(a[s], b[s], b[s + 1], band[s], centre);
      tri(a[s], b[s + 1], a[s + 1], band[s], centre);
    }
  }
  // the front and the rear, flat
  for (const [c, v, nz] of [
    [cols[cols.length - 1], by.front, 1],
    [cols[0], by.rear, -1],
  ]) {
    const pts = ring(c);
    const o = [(c.lo + c.hi) / 2, (c.bot + c.top) / 2, c.z];
    for (let k = 0; k < pts.length; k++) {
      const p = pts[k];
      const q = pts[(k + 1) % pts.length];
      for (const x of [o, p, q]) put(x, [0, 0, nz], v);
    }
  }
  return { verts: new Float32Array(out), length: box.max[2] - box.min[2], width: 2 * half, height: Math.max(...cols.map((c) => c.top)) - ground };
}

/** The model as it is kept (`b3d_depot_put`): plain numbers, rounded. */
export function packModel(m) {
  return { style: BAKE_STYLE, length: m.length, width: m.width, height: m.height, verts: Array.from(m.verts, (x) => Math.round(x * 10000) / 10000) };
}

/** A kept model back (null when it is of another style or none). */
export function unpackModel(o) {
  if (!o || o.style !== BAKE_STYLE || !Array.isArray(o.verts) || o.verts.length % 8) return null;
  return { length: o.length, width: o.width, height: o.height, verts: new Float32Array(o.verts) };
}
