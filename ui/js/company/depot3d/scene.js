// The yard in metres, from the depot page's plan (`company_more::depot::plan`): the buildings
// along the back - the offices, the workshop hall a bay a level, the washing bay, the diesel
// station -, the parking's blocks of spaces with their chargers, the gate and the street at
// the front, a verge of grass with trees round it all. What is not built yet is a dashed plot;
// building work is amber. The buses stand where the plan says: in the bays, in the spaces, in
// front of the hall when they wait for it, and in the street when the yard has no room.
//
// Each area has its number (`AREAS`): the Build list's row lights it up.

import { Mesh } from './mesh.js';

export const AREAS = { parking: 1, workshop: 2, wash: 3, fuel: 4, charging: 5, offices: 6 };

/** A bus's length in metres by its size (`BusSize`). */
const LENGTH = { midi: 10.5, solo: 12, articulated: 18, double: 11.5 };
const SPACE_W = 4.2;
const SPACE_L = 20;
const VERGE = 16;

/** A small seeded random, so the trees stand where they stood. */
function seeded(seed) {
  let s = seed >>> 0 || 1;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

/** The yard: `{ mesh, buses, labels, bounds }`. `pal` is the look's palette (`palette.js`),
 * `level` how finely it is drawn (`quality.js`: without trees at the lowest). */
export function buildScene(p, pal, level = { trees: true }) {
  const m = new Mesh();
  const buses = [];
  const labels = [];
  const pk = p.parking;
  const rows = Math.max(1, Math.ceil(pk.blocks / 2));
  const per = Math.max(1, pk.per);
  const blockW = per * SPACE_W;
  const aisle = 6;
  const M = 8;
  const W = 2 * blockW + aisle + 2 * M;
  const TOP = 28;
  const parkZ = M + TOP + 14;
  const rowGap = 9;
  const D = parkZ + rows * SPACE_L + (rows - 1) * rowGap + 14;
  const street = { z0: D + 5, z1: D + 15 };

  // the base: grass with its edge of earth, the yard's asphalt on it, the street at the front
  const bx0 = -VERGE;
  const bx1 = W + VERGE;
  const bz0 = -VERGE;
  const bz1 = street.z1 + 6;
  m.box(bx0, -2.4, bz0, bx1, 0, bz1, pal.earth, { top: pal.grass });
  m.flat(0, 0, W, D, 0.02, pal.asphalt);
  m.flat(bx0, street.z0, bx1, street.z1, 0.02, pal.street);
  m.flat(bx0, street.z0 - 1.6, bx1, street.z0, 0.04, pal.kerb);
  for (let x = bx0 + 2; x < bx1 - 4; x += 6) m.flat(x, (street.z0 + street.z1) / 2 - 0.12, x + 3, (street.z0 + street.z1) / 2 + 0.12, 0.04, pal.paint);
  const gate = { x0: W / 2 - 7, x1: W / 2 + 7 };
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

  // the buildings along the back
  const g = 4;
  const avail = W - 2 * M - 3 * g;
  const ow = avail * 0.17;
  const hw = avail * 0.45;
  const ww = avail * 0.14;
  const top = { z0: M, z1: M + TOP };
  const offices = { x0: M, x1: M + ow };
  const hall = { x0: offices.x1 + g, x1: offices.x1 + g + hw };
  const wash = { x0: hall.x1 + g, x1: hall.x1 + g + ww };
  const fuel = { x0: wash.x1 + g, x1: W - M };
  const works = (r, area, h = 4) => {
    // building work: an amber outline, a site fence and a crane's mast
    m.dashed(r.x0, r.z0, r.x1, r.z1, pal.warn, area, { w: 0.4 });
    for (let x = r.x0 + 1; x < r.x1 - 1; x += 3) m.box(x, 0, r.z1 - 0.6, x + 2.2, 1.1, r.z1 - 0.45, pal.warn, { area });
    m.box(r.x0 + 1.5, 0, r.z0 + 1.5, r.x0 + 2.3, h + 10, r.z0 + 2.3, pal.warn, { area });
    m.box(r.x0 + 1.5, h + 9.4, r.z0 + 1.5, Math.min(r.x1, r.x0 + 16), h + 10, r.z0 + 2.3, pal.warn, { area });
  };
  const plot = (r, area) => m.dashed(r.x0, r.z0, r.x1, r.z1, pal.plot, area);
  const head = (b) => (b?.head ? b.head.charAt(0) + b.head.slice(1).toLowerCase() : '');
  const label = (key, x, y, z, b) => labels.push({ key, head: head(b), pos: [x, y, z], built: !!b?.built, works: b?.works || null });

  // the offices: a storey a level, windows round every floor, lit at night
  {
    const b = p.offices;
    const r = { ...offices, ...top };
    const floors = Math.max(1, b.floors || 1);
    const h = 1 + floors * 3.6;
    if (b.built) {
      m.box(r.x0, 0, r.z0 + 4, r.x1, h, r.z1, pal.wall, { area: AREAS.offices, top: pal.roof });
      for (let f = 0; f < floors; f++) {
        const y0 = 1.9 + f * 3.6;
        const y1 = y0 + 1.6;
        m.box(r.x0 - 0.05, y0, r.z0 + 5, r.x1 + 0.05, y1, r.z1 - 1, pal.glass, { area: AREAS.offices, glow: 0.8 });
        m.box(r.x0 + 1, y0, r.z1 - 0.05, r.x1 - 1, y1, r.z1 + 0.05, pal.glass, { area: AREAS.offices, glow: 0.8 });
      }
      m.box(r.x0 + 2, h, r.z0 + 7, r.x0 + 5.5, h + 1.2, r.z0 + 10, pal.metal, { area: AREAS.offices });
      m.box(r.x0 + (r.x1 - r.x0) / 2 - 2, 2.9, r.z1, r.x0 + (r.x1 - r.x0) / 2 + 2, 3.2, r.z1 + 2, pal.roof, { area: AREAS.offices });
    } else plot(r, AREAS.offices);
    if (b.works) works(r, AREAS.offices, h);
    label('offices', (r.x0 + r.x1) / 2, h + 3, (r.z0 + r.z1) / 2 + 2, b);
  }

  // the workshop hall: as wide as its bays, an open roof of trusses over them so the buses in
  // it show; the bays still to build are plots beside it
  {
    const b = p.hall;
    const max = Math.max(1, b.max);
    const bays = b.built ? Math.max(0, Math.min(max, b.bays)) : 0;
    const bayW = (hall.x1 - hall.x0) / max;
    const H = 8.5;
    const r = { x0: hall.x0, x1: hall.x0 + bays * bayW, ...top };
    if (bays > 0) {
      const t = 0.5;
      m.box(r.x0, 0, r.z0, r.x1, H, r.z0 + t, pal.wall, { area: AREAS.workshop });
      m.box(r.x0, 0, r.z0, r.x0 + t, H, r.z1, pal.wall, { area: AREAS.workshop });
      m.box(r.x1 - t, 0, r.z0, r.x1, H, r.z1, pal.wall, { area: AREAS.workshop });
      m.box(r.x0, H - 1.6, r.z1 - t, r.x1, H, r.z1, pal.wall, { area: AREAS.workshop });
      for (let z = r.z0 + 3; z < r.z1 - 1; z += 3.2) m.box(r.x0, H - 0.2, z, r.x1, H + 0.4, z + 0.45, pal.roof, { area: AREAS.workshop });
      m.box(r.x0, H - 0.1, r.z0, r.x1, H + 0.5, r.z0 + 2.2, pal.roof, { area: AREAS.workshop });
      for (let k = 0; k < bays; k++) {
        const x0 = r.x0 + k * bayW;
        if (k > 0) m.box(x0 - 0.3, 0, r.z1 - 1, x0 + 0.3, H, r.z1, pal.wall, { area: AREAS.workshop });
        m.flat(x0 + 0.8, r.z0 + 1, x0 + bayW - 0.8, r.z1 - 0.5, 0.04, pal.concrete, AREAS.workshop);
        m.flat(x0 + bayW / 2 - 0.5, r.z0 + 4, x0 + bayW / 2 + 0.5, r.z1 - 3, 0.06, pal.pit, AREAS.workshop);
        // the roller door, rolled up
        m.box(x0 + 0.6, H - 1.6, r.z1 - 0.15, x0 + bayW - 0.6, H - 0.6, r.z1 + 0.1, pal.door, { area: AREAS.workshop });
        // a lamp over each bay, lit at night
        m.box(x0 + bayW / 2 - 1.2, H - 0.7, (r.z0 + r.z1) / 2 - 0.2, x0 + bayW / 2 + 1.2, H - 0.5, (r.z0 + r.z1) / 2 + 0.2, pal.lamp, { glow: 1 });
      }
    }
    for (let k = bays; k < max; k++) plot({ x0: hall.x0 + k * bayW + 0.6, x1: hall.x0 + (k + 1) * bayW - 0.6, z0: top.z0 + 1, z1: top.z1 - 1 }, AREAS.workshop);
    if (b.works) works({ x0: r.x1 + 0.6, x1: Math.min(hall.x1, r.x1 + bayW) - 0.6, ...top }, AREAS.workshop, H);
    label('workshop', (hall.x0 + Math.max(r.x1, hall.x0 + bayW)) / 2, H + 3.5, (top.z0 + top.z1) / 2, b);
    // the buses in their bays, nose to the yard; the ones waiting for a bay in front of the hall
    p.in_ws.slice(0, bays).forEach((bus, k) => buses.push(place(bus, hall.x0 + (k + 0.5) * bayW, (top.z0 + top.z1) / 2 + 1, 0, 'workshop')));
    p.in_ws.slice(bays, bays + 6).forEach((bus, k) => buses.push(place(bus, hall.x0 + 8 + k * 14, top.z1 + 7, Math.PI / 2, 'waiting')));
  }

  // the washing bay: a portal over a lane, its brushes; a second gate with the level after
  {
    const b = p.wash;
    const r = { ...wash, ...top };
    if (b.built) {
      const cx = (r.x0 + r.x1) / 2;
      m.flat(r.x0 + 1, r.z0, r.x1 - 1, r.z1, 0.04, pal.washLane, AREAS.wash);
      m.box(r.x0, 0, r.z0, r.x0 + 0.6, 5.5, r.z1, pal.wall, { area: AREAS.wash });
      m.box(r.x1 - 0.6, 0, r.z0, r.x1, 5.5, r.z1, pal.wall, { area: AREAS.wash });
      m.box(r.x0, 5.5, r.z0, r.x1, 6.1, r.z1, pal.roof, { area: AREAS.wash });
      for (const z of b.gates ? [r.z0 + 8, r.z0 + 18] : [r.z0 + 13]) {
        m.box(r.x0 + 0.6, 4.4, z - 0.4, r.x1 - 0.6, 4.9, z + 0.4, pal.metal, { area: AREAS.wash });
        m.prism(cx - 2.1, z, 0.55, 0.55, 0.3, 4.4, pal.brush, { n: 8, area: AREAS.wash });
        m.prism(cx + 2.1, z, 0.55, 0.55, 0.3, 4.4, pal.brush, { n: 8, area: AREAS.wash });
      }
    } else plot(r, AREAS.wash);
    if (b.works) works(r, AREAS.wash, 6);
    label('wash', (r.x0 + r.x1) / 2, 9, (r.z0 + r.z1) / 2, b);
  }

  // the diesel station: a canopy on four columns over its pumps, a band in the accent
  {
    const b = p.fuel;
    const r = { ...fuel, z0: top.z0 + 4, z1: top.z1 - 2 };
    if (b.built) {
      const inset = 2;
      for (const [x, z] of [
        [r.x0 + inset, r.z0 + inset],
        [r.x1 - inset, r.z0 + inset],
        [r.x0 + inset, r.z1 - inset],
        [r.x1 - inset, r.z1 - inset],
      ])
        m.box(x - 0.3, 0, z - 0.3, x + 0.3, 5.6, z + 0.3, pal.metal, { area: AREAS.fuel });
      m.box(r.x0, 5.6, r.z0, r.x1, 6.5, r.z1, pal.roof, { area: AREAS.fuel, top: pal.wall });
      m.box(r.x0 - 0.05, 5.7, r.z1 - 0.05, r.x1 + 0.05, 6.4, r.z1 + 0.05, pal.accent, { area: AREAS.fuel, glow: 0.6 });
      m.box(r.x0 + 0.5, 5.5, r.z0 + 0.5, r.x1 - 0.5, 5.6, r.z1 - 0.5, pal.lamp, { glow: 1 });
      const pumps = Math.max(1, b.pumps || 1);
      const zc = (r.z0 + r.z1) / 2;
      for (let k = 0; k < pumps; k++) {
        const x = r.x0 + ((k + 1) * (r.x1 - r.x0)) / (pumps + 1);
        m.box(x - 1.6, 0, zc - 1.2, x + 1.6, 0.2, zc + 1.2, pal.concrete, { area: AREAS.fuel });
        m.box(x - 0.5, 0.2, zc - 0.35, x + 0.5, 2, zc + 0.35, pal.pump, { area: AREAS.fuel });
        m.box(x - 0.35, 1.3, zc + 0.35, x + 0.35, 1.8, zc + 0.4, pal.screen, { glow: 1 });
      }
    } else plot(r, AREAS.fuel);
    if (b.works) works(r, AREAS.fuel, 6);
    label('fuel', (r.x0 + r.x1) / 2, 10, (r.z0 + r.z1) / 2, b);
  }

  // the parking: rows of two blocks of spaces, a block a level, a charger at the head of the
  // spaces it serves; the bus in each space as the plan lists them
  let n = 0;
  let nextPlot = null;
  for (let blk = 0; blk < pk.blocks; blk++) {
    const x0 = M + (blk % 2) * (blockW + aisle);
    const z0 = parkZ + Math.floor(blk / 2) * (SPACE_L + rowGap);
    const r = { x0, x1: x0 + blockW, z0, z1: z0 + SPACE_L };
    const built = blk < pk.built;
    if (!built) {
      const now = blk === pk.built && pk.building;
      if (now) works(r, AREAS.parking, 0);
      else plot({ x0: r.x0 + 0.5, x1: r.x1 - 0.5, z0: r.z0 + 0.5, z1: r.z1 - 0.5 }, AREAS.parking);
      if (!nextPlot) nextPlot = { r, now };
      continue;
    }
    for (let s = 0; s <= per; s++) m.flat(r.x0 + s * SPACE_W - 0.07, r.z0, r.x0 + s * SPACE_W + 0.07, r.z1, 0.05, pal.paint, AREAS.parking);
    m.flat(r.x0, r.z0 - 0.07, r.x1, r.z0 + 0.07, 0.05, pal.paint, AREAS.parking);
    for (let s = 0; s < per; s++) {
      const cx = r.x0 + (s + 0.5) * SPACE_W;
      if (n < pk.chargers) {
        m.box(cx - 0.3, 0, r.z0 - 0.9, cx + 0.3, 1.7, r.z0 - 0.45, pal.charger, { area: AREAS.charging });
        m.box(cx - 0.22, 1.25, r.z0 - 0.45, cx + 0.22, 1.55, r.z0 - 0.41, pal.ok, { area: AREAS.charging, glow: 1 });
      }
      const bus = p.parked[n];
      if (bus) buses.push(place(bus, cx, r.z0 + SPACE_L / 2 + 0.5, 0, 'parked'));
      n++;
    }
  }
  if (nextPlot) labels.push({ key: 'parking_more', pos: [(nextPlot.r.x0 + nextPlot.r.x1) / 2, 2.5, (nextPlot.r.z0 + nextPlot.r.z1) / 2], built: false, works: nextPlot.now ? pk.works : null });
  labels.push({ key: 'parking', pos: [M + blockW / 2, 7, parkZ + SPACE_L / 2], built: pk.built > 0, works: null });
  // the buses the yard has no space for: along the street, a warning ring round each
  p.parked.slice(n, n + 10).forEach((bus, k) => buses.push(place(bus, bx0 + 12 + k * 14, street.z0 + 3, Math.PI / 2, 'street')));

  // trees on the verge, round the yard but not on the street
  const rnd = seeded(7 + pk.blocks * 31 + per);
  const tree = (x, z) => {
    const s = 0.8 + rnd() * 0.6;
    m.prism(x, z, 0.25 * s, 0.2 * s, 0, 2 * s, pal.trunk, { n: 5 });
    const tint = pal.leaves.map((c, i) => Math.min(1, c * (0.88 + rnd() * 0.24) + (i === 1 ? 0.02 : 0)));
    m.prism(x, z, 2.4 * s, 0, 1.4 * s, 6.2 * s, tint, { n: 7, turn: rnd() * 6 });
    m.prism(x, z, 1.8 * s, 0, 4 * s, 8 * s, tint, { n: 7, turn: rnd() * 6 });
  };
  if (level.trees) {
    for (let x = bx0 + 4; x < bx1 - 2; x += 7 + rnd() * 5) tree(x, bz0 + 4 + rnd() * 6);
    for (let z = bz0 + 14; z < street.z0 - 6; z += 8 + rnd() * 6) {
      tree(bx0 + 4 + rnd() * 7, z);
      tree(bx1 - 4 - rnd() * 7, z);
    }
    for (let x = bx0 + 6; x < bx1 - 2; x += 10 + rnd() * 8) if (x < gate.x0 - 8 || x > gate.x1 + 8) tree(x, street.z1 + 3 + rnd() * 2);
  }

  return { mesh: m, buses, labels, bounds: { x0: bx0, x1: bx1, z0: bz0, z1: bz1 }, yard: { w: W, d: D } };
}

/** A bus where it stands: its centre, its heading (0: nose to the street) and what it is. */
function place(bus, x, z, heading, at) {
  const len = LENGTH[bus.size] || LENGTH.solo;
  return { id: bus.id, number: bus.number, tip: bus.tip, file: bus.file || '', paint: bus.paint || '', x, z, heading, len, height: bus.size === 'double' ? 4.3 : 3.1, articulated: bus.size === 'articulated', at };
}
