// The yard in metres, from the depot page's plan (`company_more::depot::plan`) and its layout
// (`plan.layout`, `hub_core::company::yard`): every module where the player put it down, turned
// as it stands - the offices a storey a level, the workshop's bays (an open roof of trusses so
// the buses in them show), the washing bay, the diesel station, the blocks of twelve spaces with
// their chargers -, building work where a module will stand, the gate and the street at the
// front, the land still to buy beside the yard, a verge of grass with trees round it all. The
// buses stand where the plan says: in the bays, in the spaces, before the workshop when they
// wait for it, and in the street when the yard has no room.
//
// Each module is drawn in a frame of its own (x across, z from its back to its front, its front
// at +z) and turned onto the yard. Each area has its number (`AREAS`): the Build list's row
// lights it up.

import { Mesh } from './mesh.js';

export const AREAS = { parking: 1, workshop: 2, wash: 3, fuel: 4, charging: 5, offices: 6, land: 7 };

/** A bus's length in metres by its size (`BusSize`). */
const LENGTH = { midi: 10.5, solo: 12, articulated: 18, double: 11.5 };
const VERGE = 16;

/** A small seeded random, so the trees stand where they stood. */
function seeded(seed) {
  let s = seed >>> 0 || 1;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

/** A module's frame onto the yard: its own (lx, y, lz), lw wide and ld deep, turned `rot` with
 * its corner at (x0, z0). */
export function frame(rot, x0, z0, lw, ld) {
  const k = ((rot % 4) + 4) % 4;
  const pt = ([x, y, z]) => [[x0 + x, y, z0 + z], [x0 + z, y, z0 + lw - x], [x0 + lw - x, y, z0 + ld - z], [x0 + ld - z, y, z0 + x]][k];
  const dir = ([x, y, z]) => [[x, y, z], [z, y, -x], [-x, y, -z], [-z, y, x]][k];
  return { pt, dir, heading: (h) => h + (k * Math.PI) / 2 };
}

/** The yard: `{ mesh, buses, labels, bounds, picks, routes }` (`picks`: each module's place in
 * metres, to point at; `routes`: where the staff walk, `people.js`). `pal` is the look's palette
 * (`palette.js`), `level` how finely it is drawn (`quality.js`: without trees at the lowest). */
export function buildScene(p, pal, level = { trees: true }) {
  const L = p.layout;
  const cell = L.cell;
  const m = new Mesh();
  const buses = [];
  const labels = [];
  const picks = [];
  const routes = [];
  const W = L.cols * cell;
  const D = L.rows * cell;
  const MW = L.max_cols * cell;
  const street = { z0: D + 5, z1: D + 15 };

  // the base: grass with its edge of earth, the yard's asphalt on it, the street at the front
  const bx0 = -VERGE;
  const bx1 = MW + VERGE;
  const bz0 = -VERGE;
  const bz1 = street.z1 + 6;
  m.box(bx0, -2.4, bz0, bx1, 0, bz1, pal.earth, { top: pal.grass });
  m.flat(0, 0, W, D, 0.02, pal.asphalt);
  m.flat(bx0, street.z0, bx1, street.z1, 0.02, pal.street);
  m.flat(bx0, street.z0 - 1.6, bx1, street.z0, 0.04, pal.kerb);
  for (let x = bx0 + 2; x < bx1 - 4; x += 6) m.flat(x, (street.z0 + street.z1) / 2 - 0.12, x + 3, (street.z0 + street.z1) / 2 + 0.12, 0.04, pal.paint);
  // the land still to buy: its edge marked, a label to say so
  if (MW > W) {
    m.dashed(W + 1, 1, MW - 1, D - 1, pal.plot, AREAS.land, { w: 0.35 });
    for (let z = 4; z < D - 2; z += 12) m.box(MW - 1.2, 0, z, MW - 1, 1.2, z + 0.2, pal.fence, { area: AREAS.land });
    if (L.land) labels.push({ key: 'land', pos: [(W + Math.min(MW, W + 40)) / 2, 2.5, D / 2], built: false, works: null });
  }
  const gate = { x0: L.gate.x * cell, x1: (L.gate.x + L.gate.w) * cell };
  m.flat(gate.x0, D, gate.x1, street.z0 - 1.6, 0.03, pal.asphalt);
  // the fence round the yard, open at the gate; a gatehouse beside it
  const post = (x, z) => m.box(x - 0.08, 0, z - 0.08, x + 0.08, 1.9, z + 0.08, pal.fence);
  const rail = (x0, z0, x1, z1) => m.box(Math.min(x0, x1) - 0.04, 1.75, Math.min(z0, z1) - 0.04, Math.max(x0, x1) + 0.04, 1.85, Math.max(z0, z1) + 0.04, pal.fence);
  const fence = (x0, z0, x1, z1) => {
    const len = Math.hypot(x1 - x0, z1 - z0);
    const n = Math.max(1, Math.round(len / 3));
    for (let k = 0; k <= n; k++) post(x0 + ((x1 - x0) * k) / n, z0 + ((z1 - z0) * k) / n);
    rail(x0, z0, x1, z1);
  };
  fence(0, 0, W, 0);
  fence(0, 0, 0, D);
  fence(W, 0, W, D);
  fence(0, D, gate.x0, D);
  fence(gate.x1, D, W, D);
  m.box(gate.x1 + 1, 0, D - 4.5, gate.x1 + 4.5, 2.8, D - 1, pal.wall, { top: pal.roof });
  m.box(gate.x1 + 1.02, 1.1, D - 1.04, gate.x1 + 4.48, 2.1, D - 0.98, pal.glass, { glow: 0.9 });
  m.box(gate.x0 - 0.5, 0.9, D - 0.15, gate.x1 - 0.5, 1.05, D + 0.15, pal.barrier);

  // the modules, by area, from the back and the west
  const order = (a, b) => a.y - b.y || a.x - b.x;
  const of = (area) => L.modules.filter((o) => o.area === area && !o.building).sort(order);
  const shape = (area) => L.shapes[area];
  const put = (o, draw) => {
    const s = shape(o.area);
    const lw = s.w * cell;
    const ld = s.h * cell;
    const f = frame(o.rot, o.x * cell, o.y * cell, lw, ld);
    const lm = new Mesh();
    draw(lm, { x0: 0, x1: lw, z0: 0, z1: ld }, f);
    m.append(lm, f);
    picks.push({ id: o.id, area: o.area, title: o.title, what: o.what, works: o.works, building: o.building, rot: o.rot, x0: o.x * cell, z0: o.y * cell, x1: (o.x + o.w) * cell, z1: (o.y + o.h) * cell });
    return f;
  };
  // a way the staff walk (`people.js`), in a module's frame: points [lx, lz, a stop, the way
  // they face there (0: to the module's front)]
  const route = (kind, f, pts) =>
    routes.push({
      kind,
      pts: pts.map(([lx, lz, stop = false, face = null]) => {
        const [x, , z] = f.pt([lx, 0, lz]);
        return { x, z, stop, face: face === null ? null : f.heading(face) };
      }),
    });
  const tag = (key, f, lx, y, lz, o) => labels.push({ key, head: o.title, pos: f.pt([lx, y, lz]), built: !o.building, works: o.works });
  // building work: an amber outline, a site fence and a crane's mast
  const works = (mm, r, area, h = 4) => {
    mm.dashed(r.x0, r.z0, r.x1, r.z1, pal.warn, area, { w: 0.4 });
    for (let x = r.x0 + 1; x < r.x1 - 1; x += 3) mm.box(x, 0, r.z1 - 0.6, x + 2.2, 1.1, r.z1 - 0.45, pal.warn, { area });
    mm.box(r.x0 + 1.5, 0, r.z0 + 1.5, r.x0 + 2.3, h + 10, r.z0 + 2.3, pal.warn, { area });
    mm.box(r.x0 + 1.5, h + 9.4, r.z0 + 1.5, Math.min(r.x1, r.x0 + 14), h + 10, r.z0 + 2.3, pal.warn, { area });
  };
  for (const o of L.modules.filter((x) => x.building)) {
    const s = shape(o.area);
    const f = put(o, (mm, r) => works(mm, r, AREAS[o.area] || 0, 4));
    tag(o.area, f, (s.w * cell) / 2, 8, (s.h * cell) / 2, o);
  }

  // the offices: a storey a level, windows round every floor, lit at night
  for (const o of of('offices')) {
    const floors = Math.max(1, p.offices.floors || 1);
    const h = 1 + floors * 3.6;
    const f = put(o, (mm, r) => {
      const A = AREAS.offices;
      mm.box(r.x0, 0, r.z0 + 4, r.x1, h, r.z1, pal.wall, { area: A, top: pal.roof });
      for (let k = 0; k < floors; k++) {
        const y0 = 1.9 + k * 3.6;
        mm.box(r.x0 - 0.05, y0, r.z0 + 5, r.x1 + 0.05, y0 + 1.6, r.z1 - 1, pal.glass, { area: A, glow: 0.8 });
        mm.box(r.x0 + 1, y0, r.z1 - 0.05, r.x1 - 1, y0 + 1.6, r.z1 + 0.05, pal.glass, { area: A, glow: 0.8 });
      }
      mm.box(r.x0 + 2, h, r.z0 + 7, r.x0 + 5.5, h + 1.2, r.z0 + 10, pal.metal, { area: A });
      const cx = (r.x0 + r.x1) / 2;
      mm.box(cx - 2, 2.9, r.z1, cx + 2, 3.2, r.z1 + 2, pal.roof, { area: A });
      if (o.works) works(mm, r, A, h);
    });
    tag('offices', f, 10, h + 3, 14, o);
    // before its door, and along its front
    const s = shape('offices');
    const lw = s.w * cell;
    const ld = s.h * cell;
    route('offices', f, [
      [lw / 2, ld + 1.4, true, 0],
      [lw / 2 - 6, ld + 2.2, true],
      [lw / 2 + 6, ld + 2.2, true],
    ]);
    route('offices', f, [
      [2, ld + 3, true],
      [lw - 2, ld + 3, true],
    ]);
  }

  // the workshop: a hall a bay, the bays side by side make one; the buses in their bays, nose
  // to the yard, the ones waiting for a bay before the first
  const H = 8.5;
  const bays = of('workshop');
  bays.forEach((o, k) => {
    const f = put(o, (mm, r) => {
      const t = 0.5;
      const A = AREAS.workshop;
      mm.box(r.x0, 0, r.z0, r.x1, H, r.z0 + t, pal.wall, { area: A });
      mm.box(r.x0, 0, r.z0, r.x0 + t, H, r.z1, pal.wall, { area: A });
      mm.box(r.x1 - t, 0, r.z0, r.x1, H, r.z1, pal.wall, { area: A });
      mm.box(r.x0, H - 1.6, r.z1 - t, r.x1, H, r.z1, pal.wall, { area: A });
      for (let z = r.z0 + 3; z < r.z1 - 1; z += 3.2) mm.box(r.x0, H - 0.2, z, r.x1, H + 0.4, z + 0.45, pal.roof, { area: A });
      mm.box(r.x0, H - 0.1, r.z0, r.x1, H + 0.5, r.z0 + 2.2, pal.roof, { area: A });
      mm.flat(r.x0 + 0.8, r.z0 + 1, r.x1 - 0.8, r.z1 - 0.5, 0.04, pal.concrete, A);
      const cx = (r.x0 + r.x1) / 2;
      mm.flat(cx - 0.5, r.z0 + 4, cx + 0.5, r.z1 - 3, 0.06, pal.pit, A);
      mm.box(r.x0 + 0.6, H - 1.6, r.z1 - 0.15, r.x1 - 0.6, H - 0.6, r.z1 + 0.1, pal.door, { area: A });
      mm.box(cx - 1.2, H - 0.7, (r.z0 + r.z1) / 2 - 0.2, cx + 1.2, H - 0.5, (r.z0 + r.z1) / 2 + 0.2, pal.lamp, { glow: 1 });
    });
    const bus = p.in_ws[k];
    if (bus) buses.push(place(bus, f, 4, 15, 'workshop'));
    if (k === 0) tag('workshop', f, 4, H + 3.5, 14, o);
    // either side of the bus in the bay, facing it; its forecourt
    const s = shape('workshop');
    const lw = s.w * cell;
    const ld = s.h * cell;
    route('workshop', f, [
      [lw - 1.4, 7, true, -Math.PI / 2],
      [lw - 1.4, 23, true, -Math.PI / 2],
    ]);
    route('workshop', f, [
      [1.4, 7, true, Math.PI / 2],
      [1.4, 23, true, Math.PI / 2],
    ]);
    route('workshop', f, [
      [1.5, ld + 3, true, Math.PI],
      [lw - 1.5, ld + 3, true, Math.PI],
    ]);
  });
  if (bays.length) {
    const s = shape('workshop');
    const first = bays[0];
    const f = frame(first.rot, first.x * cell, first.y * cell, s.w * cell, s.h * cell);
    p.in_ws.slice(bays.length, bays.length + 6).forEach((bus, k) => buses.push(place(bus, f, 8 + k * 14, s.h * cell + 6, 'waiting', Math.PI / 2)));
  }

  // the washing bay: a portal over a lane, its brushes; a second gate with the level after
  for (const o of of('wash')) {
    const f = put(o, (mm, r) => {
      const cx = (r.x0 + r.x1) / 2;
      const A = AREAS.wash;
      mm.flat(r.x0 + 1, r.z0, r.x1 - 1, r.z1, 0.04, pal.washLane, A);
      mm.box(r.x0, 0, r.z0, r.x0 + 0.6, 5.5, r.z1, pal.wall, { area: A });
      mm.box(r.x1 - 0.6, 0, r.z0, r.x1, 5.5, r.z1, pal.wall, { area: A });
      mm.box(r.x0, 5.5, r.z0, r.x1, 6.1, r.z1, pal.roof, { area: A });
      for (const z of p.wash.gates ? [r.z0 + 8, r.z0 + 18] : [r.z0 + 13]) {
        mm.box(r.x0 + 0.6, 4.4, z - 0.4, r.x1 - 0.6, 4.9, z + 0.4, pal.metal, { area: A });
        mm.prism(cx - 2.1, z, 0.55, 0.55, 0.3, 4.4, pal.brush, { n: 8, area: A });
        mm.prism(cx + 2.1, z, 0.55, 0.55, 0.3, 4.4, pal.brush, { n: 8, area: A });
      }
      if (o.works) works(mm, r, A, 6);
    });
    tag('wash', f, 6, 9, 14, o);
    // beside the lane, clear of the brushes; before it
    const s = shape('wash');
    const lw = s.w * cell;
    const ld = s.h * cell;
    route('wash', f, [
      [1.6, 4, true, Math.PI / 2],
      [1.6, ld - 4, true, Math.PI / 2],
    ]);
    route('wash', f, [
      [2, ld + 2, true],
      [lw - 2, ld + 2, true],
    ]);
  }

  // the diesel station: a canopy on four columns over its pumps, a band in the accent
  for (const o of of('fuel')) {
    const f = put(o, (mm, r) => {
      const A = AREAS.fuel;
      const inset = 2;
      for (const [x, z] of [
        [r.x0 + inset, r.z0 + inset],
        [r.x1 - inset, r.z0 + inset],
        [r.x0 + inset, r.z1 - inset],
        [r.x1 - inset, r.z1 - inset],
      ])
        mm.box(x - 0.3, 0, z - 0.3, x + 0.3, 5.6, z + 0.3, pal.metal, { area: A });
      mm.box(r.x0, 5.6, r.z0, r.x1, 6.5, r.z1, pal.roof, { area: A, top: pal.wall });
      mm.box(r.x0 - 0.05, 5.7, r.z1 - 0.05, r.x1 + 0.05, 6.4, r.z1 + 0.05, pal.accent, { area: A, glow: 0.6 });
      mm.box(r.x0 + 0.5, 5.5, r.z0 + 0.5, r.x1 - 0.5, 5.6, r.z1 - 0.5, pal.lamp, { glow: 1 });
      const pumps = Math.max(1, p.fuel.pumps || 1);
      const zc = (r.z0 + r.z1) / 2;
      for (let k = 0; k < pumps; k++) {
        const x = r.x0 + ((k + 1) * (r.x1 - r.x0)) / (pumps + 1);
        mm.box(x - 1.6, 0, zc - 1.2, x + 1.6, 0.2, zc + 1.2, pal.concrete, { area: A });
        mm.box(x - 0.5, 0.2, zc - 0.35, x + 0.5, 2, zc + 0.35, pal.pump, { area: A });
        mm.box(x - 0.35, 1.3, zc + 0.35, x + 0.35, 1.8, zc + 0.4, pal.screen, { glow: 1 });
      }
      if (o.works) works(mm, r, A, 6);
    });
    tag('fuel', f, 10, 10, 10, o);
    // from the forecourt to beside a pump
    const s = shape('fuel');
    const lw = s.w * cell;
    const ld = s.h * cell;
    const pumps = Math.max(1, p.fuel.pumps || 1);
    for (let k = 0; k < pumps; k++) {
      const x = ((k + 1) * lw) / (pumps + 1);
      route('fuel', f, [
        [x + 1.3, ld / 2 + 2.4, true, Math.PI],
        [x + 1.3, ld + 3, true],
      ]);
    }
  }

  // the parking: blocks of spaces, a charger at the head of the spaces it serves; the bus in
  // each space as the plan lists them
  const pk = p.parking;
  const per = Math.max(1, pk.per);
  const ps = shape('parking');
  let n = 0;
  of('parking').forEach((o, k) => {
    const first = n;
    const f = put(o, (mm, r) => {
      const sw = (r.x1 - r.x0) / per;
      for (let s = 0; s <= per; s++) mm.flat(r.x0 + s * sw - 0.07, r.z0, r.x0 + s * sw + 0.07, r.z1, 0.05, pal.paint, AREAS.parking);
      mm.flat(r.x0, r.z0 - 0.07, r.x1, r.z0 + 0.07, 0.05, pal.paint, AREAS.parking);
      for (let s = 0; s < per; s++) {
        if (first + s >= pk.chargers) break;
        const cx = r.x0 + (s + 0.5) * sw;
        mm.box(cx - 0.3, 0, r.z0 + 0.25, cx + 0.3, 1.7, r.z0 + 0.7, pal.charger, { area: AREAS.charging });
        mm.box(cx - 0.22, 1.25, r.z0 + 0.7, cx + 0.22, 1.55, r.z0 + 0.74, pal.ok, { area: AREAS.charging, glow: 1 });
      }
    });
    const sw = (ps.w * cell) / per;
    for (let s = 0; s < per; s++) {
      const bus = p.parked[n];
      if (bus) buses.push(place(bus, f, (s + 0.5) * sw, (ps.h * cell) / 2 + 0.5, 'parked'));
      n++;
    }
    if (k === 0) tag('parking', f, (ps.w * cell) / 2, 7, 10, o);
    // along the lane before the spaces, and in between the buses
    const lw = ps.w * cell;
    const ld = ps.h * cell;
    route('parking', f, [
      [2, ld + 2.5, true],
      [lw / 2, ld + 2.5],
      [lw - 2, ld + 2.5, true],
    ]);
    for (let s = 1; s < per; s += 3)
      route('parking', f, [
        [s * sw, ld + 2.5, true],
        [s * sw, 5, true, 0],
      ]);
  });
  // the buses the yard has no space for: along the street, a warning ring round each
  const streetFrame = frame(0, 0, 0, 0, 0);
  p.parked.slice(n, n + 10).forEach((bus, k) => buses.push(place(bus, streetFrame, bx0 + 12 + k * 14, street.z0 + 3, 'street', Math.PI / 2)));

  // the pavement along the street, either side of the gate; beside the gatehouse
  const pave = street.z0 - 0.8;
  route('street', streetFrame, [
    [Math.max(bx0 + 4, gate.x0 - 30), pave, true],
    [gate.x0 - 2, pave, true],
  ]);
  route('street', streetFrame, [
    [gate.x1 + 2, pave, true],
    [gate.x1 + 30, pave, true],
  ]);
  route('street', streetFrame, [[gate.x1 + 2.75, D + 1, true, 0]]);

  // trees on the verge, round the land but not on the street
  if (level.trees) {
    const rnd = seeded(7 + L.cols * 31 + L.rows);
    const tree = (x, z) => {
      const s = 0.8 + rnd() * 0.6;
      m.prism(x, z, 0.25 * s, 0.2 * s, 0, 2 * s, pal.trunk, { n: 5 });
      const tint = pal.leaves.map((c, i) => Math.min(1, c * (0.88 + rnd() * 0.24) + (i === 1 ? 0.02 : 0)));
      m.prism(x, z, 2.4 * s, 0, 1.4 * s, 6.2 * s, tint, { n: 7, turn: rnd() * 6 });
      m.prism(x, z, 1.8 * s, 0, 4 * s, 8 * s, tint, { n: 7, turn: rnd() * 6 });
    };
    for (let x = bx0 + 4; x < bx1 - 2; x += 7 + rnd() * 5) tree(x, bz0 + 4 + rnd() * 6);
    for (let z = bz0 + 14; z < street.z0 - 6; z += 8 + rnd() * 6) {
      tree(bx0 + 4 + rnd() * 7, z);
      tree(bx1 - 4 - rnd() * 7, z);
    }
    for (let x = bx0 + 6; x < bx1 - 2; x += 10 + rnd() * 8) if (x < gate.x0 - 8 || x > gate.x1 + 8) tree(x, street.z1 + 3 + rnd() * 2);
  }

  return { mesh: m, buses, labels, picks, routes, bounds: { x0: bx0, x1: bx1, z0: bz0, z1: bz1 }, yard: { w: W, d: D } };
}

/** A bus where it stands, in a module's frame: its centre, its heading (0: nose to the
 * module's front) and what it is. */
function place(bus, f, lx, lz, at, heading = 0) {
  const [x, , z] = f.pt([lx, 0, lz]);
  const len = LENGTH[bus.size] || LENGTH.solo;
  return { id: bus.id, number: bus.number, tip: bus.tip, file: bus.file || '', paint: bus.paint || '', x, z, heading: f.heading(heading), len, height: bus.size === 'double' ? 4.3 : 3.1, articulated: bus.size === 'articulated', at };
}

/** The module of the yard under a point of the ground (metres), if any. */
export function moduleUnder(picks, x, z) {
  return picks.find((o) => x >= o.x0 && x <= o.x1 && z >= o.z0 && z <= o.z1) || null;
}

/** The ghost of a module being put down: a block of its shape (the renderer tints it) with a
 * band at its front, and its aprons on the ground. */
export function ghostMesh(L, area, x, y, rot, pal) {
  const cell = L.cell;
  const m = new Mesh();
  const s = L.shapes[area];
  if (!s) return m;
  const lw = s.w * cell;
  const ld = s.h * cell;
  const f = frame(rot, x * cell, y * cell, lw, ld);
  const lm = new Mesh();
  const h = { parking: 0.6, workshop: 8.5, wash: 6, fuel: 6.5, offices: 8 }[area] || 3;
  lm.box(0.2, 0.06, 0.2, lw - 0.2, h, ld - 0.2, pal.wall);
  lm.box(0.2, 0.06, ld - 0.6, lw - 0.2, Math.min(h, 1.2) + 0.05, ld - 0.2, pal.paint);
  if (s.front) lm.flat(0, ld, lw, ld + s.front * cell, 0.08, pal.paint);
  if (s.back) lm.flat(0, -s.back * cell, lw, 0, 0.08, pal.paint);
  m.append(lm, f);
  return m;
}

/** The land's grid, every apron and the gate's lane, for the build mode. */
export function gridMesh(L, pal) {
  const cell = L.cell;
  const m = new Mesh();
  const W = L.cols * cell;
  const D = L.rows * cell;
  for (let x = 0; x <= L.cols; x += 2) m.flat(x * cell - 0.04, 0, x * cell + 0.04, D, 0.06, pal.paint);
  for (let y = 0; y <= L.rows; y += 2) m.flat(0, y * cell - 0.04, W, y * cell + 0.04, 0.06, pal.paint);
  const box = (a, col, w) => m.dashed(a.x * cell + 0.3, a.y * cell + 0.3, (a.x + a.w) * cell - 0.3, (a.y + a.h) * cell - 0.3, col, 0, { w, y: 0.07 });
  for (const o of L.modules) for (const a of o.aprons) box(a, pal.warn, 0.2);
  box(L.gate, pal.barrier, 0.25);
  return m;
}
