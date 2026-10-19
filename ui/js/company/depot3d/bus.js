// The buses on the yard, as one model drawn once for all of them (instanced): a body in the
// bus's colour, its band of windows, the windscreen, the wheels, the air conditioning on the
// roof and the destination display, lit at night. Its length is a fraction (z from -0.5 at the
// back to 0.5 at the nose) that each bus stretches to its own; an articulated bus shows its
// bellows, and the ring round it shows when it is the one picked or under the pointer.
//
// The model stands in for the bus itself until its own depot model is there (`skins.js`): then
// the bus is drawn as that, in its livery, and of this model only its ring shows.

import { Mesh } from './mesh.js';

/** What a vertex is (the mesh's last float): the body, the bellows, the ring, a light. */
export const PART = { body: 0, joint: 1, ring: 2, light: 3 };
/** Floats an instance: x, z, heading, length; r, g, b, state; height, articulated, its own
 * model shown. */
export const INSTANCE = 11;
/** Floats an instance of a bus's own model: x, z, heading, state. */
export const SKINNED = 4;
/** An instance's state: as it is, picked, under the pointer, waiting (in the street, or for a
 * bay). */
export const STATE = { none: 0, picked: 1, hover: 2, warn: 3 };

const HALF = 1.27;

/** The bus's model, `paint` (the mesh's area float) how much of the bus's colour it takes. */
export function busMesh(pal) {
  const m = new Mesh();
  const b = (x0, y0, z0, x1, y1, z1, col, paint, part = PART.body) => m.box(x0, y0, z0, x1, y1, z1, col, { area: paint, glow: part });
  const white = [1, 1, 1];
  // the body: skirt to the windows, and the roof above them
  b(-HALF, 0.32, -0.5, HALF, 1.08, 0.5, white, 1);
  b(-HALF, 2.42, -0.5, HALF, 3.0, 0.5, white, 1, PART.body);
  // the windows all round, a hair proud of the body; the windscreen down to the bumper
  b(-HALF - 0.02, 1.08, -0.49, HALF + 0.02, 2.42, 0.47, pal.busGlass, 0);
  b(-HALF + 0.08, 0.7, 0.47, HALF - 0.08, 2.42, 0.505, pal.busGlass, 0);
  b(-HALF + 0.1, 2.48, 0.495, HALF - 0.1, 2.85, 0.51, pal.display, 0, PART.light);
  // the pillars between the windows: every few metres of a 12 m bus
  for (let k = 1; k < 6; k++) {
    const z = -0.49 + (k / 6) * 0.96;
    b(-HALF - 0.03, 1.08, z - 0.008, HALF + 0.03, 2.42, z + 0.008, white, 1);
  }
  // the air conditioning and the roof's hatch
  b(-0.85, 3.0, -0.18, 0.85, 3.32, 0.12, pal.busRoof, 0.25);
  b(-0.45, 3.0, 0.22, 0.45, 3.08, 0.32, pal.busRoof, 0.25);
  // the wheels, front and back
  for (const z of [0.3, -0.24]) for (const x of [-1, 1]) b(x < 0 ? -HALF - 0.04 : HALF - 0.32, 0, z - 0.042, x < 0 ? -HALF + 0.32 : HALF + 0.04, 1.0, z + 0.042, pal.tyre, 0);
  // an articulated bus's bellows and its third axle
  b(-HALF + 0.05, 0.4, 0.05, HALF - 0.05, 2.9, 0.1, pal.bellows, 0, PART.joint);
  for (const x of [-1, 1]) b(x < 0 ? -HALF - 0.04 : HALF - 0.32, 0, -0.04 - 0.028, x < 0 ? -HALF + 0.32 : HALF + 0.04, 1.0, -0.04 + 0.028, pal.tyre, 0, PART.joint);
  // the ring on the ground round it
  const r = 0.9;
  const w = 0.32;
  const ring = (x0, z0, x1, z1) => m.flat(x0, z0, x1, z1, 0.08, white, 0, PART.ring);
  ring(-HALF - r - w, -0.5 - 0.06, -HALF - r, 0.5 + 0.06);
  ring(HALF + r, -0.5 - 0.06, HALF + r + w, 0.5 + 0.06);
  ring(-HALF - r - w, -0.5 - 0.075, HALF + r + w, -0.5 - 0.06);
  ring(-HALF - r - w, 0.5 + 0.06, HALF + r + w, 0.5 + 0.075);
  return m;
}

const stateOf = (b, picked, hover) => (b.id === picked ? STATE.picked : b.id === hover ? STATE.hover : b.at === 'street' || b.at === 'waiting' ? STATE.warn : STATE.none);

/** The instances of `buses` (from `scene.js`): their places, colours and states; `own(bus)`
 * whether the bus is drawn as its own model. */
export function instances(buses, colour, { picked, hover, own = () => false }) {
  const out = new Float32Array(buses.length * INSTANCE);
  buses.forEach((b, k) => {
    out.set([b.x, b.z, b.heading, b.len, colour[0], colour[1], colour[2], stateOf(b, picked, hover), b.height, b.articulated ? 1 : 0, own(b) ? 1 : 0], k * INSTANCE);
  });
  return out;
}

/** The buses drawn as their own models, by model (`keyOf(bus)`, null: none): `key ->
 * Float32Array` of their instances. */
export function skinned(buses, keyOf, { picked, hover }) {
  const groups = new Map();
  for (const b of buses) {
    const key = keyOf(b);
    if (!key) continue;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(b.x, b.z, b.heading, stateOf(b, picked, hover));
  }
  return new Map([...groups].map(([k, v]) => [k, new Float32Array(v)]));
}

/** Which bus a ray (from `o` along `d`) meets first, by its box: its index, or -1. */
export function pickBus(buses, o, d) {
  let best = -1;
  let bestT = Infinity;
  buses.forEach((b, k) => {
    // the ray in the bus's own frame
    const c = Math.cos(b.heading);
    const s = Math.sin(b.heading);
    const ox = o[0] - b.x;
    const oz = o[2] - b.z;
    const lo = [c * ox - s * oz, o[1], s * ox + c * oz];
    const ld = [c * d[0] - s * d[2], d[1], s * d[0] + c * d[2]];
    const min = [-HALF - 0.3, 0, -b.len / 2 - 0.3];
    const max = [HALF + 0.3, b.height + 0.4, b.len / 2 + 0.3];
    let t0 = 0;
    let t1 = Infinity;
    for (let i = 0; i < 3; i++) {
      if (Math.abs(ld[i]) < 1e-9) {
        if (lo[i] < min[i] || lo[i] > max[i]) return;
        continue;
      }
      let a = (min[i] - lo[i]) / ld[i];
      let e = (max[i] - lo[i]) / ld[i];
      if (a > e) [a, e] = [e, a];
      t0 = Math.max(t0, a);
      t1 = Math.min(t1, e);
      if (t0 > t1) return;
    }
    if (t0 < bestT) {
      bestT = t0;
      best = k;
    }
  });
  return best;
}
