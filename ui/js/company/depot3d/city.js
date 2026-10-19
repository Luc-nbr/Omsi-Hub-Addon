// The town round the depot: its streets in a grid round the depot's plot - the street at the
// front the one the gate opens onto -, pavements with their kerbs, the road's paint (the
// centre's dashes, the stop lines and the zebra crossings at every junction), street lamps,
// trees and parked cars; and between the streets the blocks: next to the depot the trading
// estate (halls, their car parks), further out the town's blocks of flats round their yards, a
// park now and then. The marks of use go on a layer of their own (`renderer.js`'s decals):
// cracks, tyre tracks, oil, manholes.
//
// How far it reaches is the graphics level's (`quality.js`'s `town`: rings of blocks round the
// depot). It is the same town every time for the same yard (seeded).

import { MAT, wearing } from './palette.js';

/** A street from kerb to kerb, a pavement either side. */
const ROAD = 10;
const WALK = 2.5;
const LANE = ROAD + WALK * 2;
/** A block of the town: across and deep. */
const BLOCK_X = 64;
const BLOCK_Z = 56;
/** A storey of the town's houses. */
const STOREY = 3.2;

/** A small seeded random, so the town stands as it stood. */
export function seeded(seed) {
  let s = seed >>> 0 || 1;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

/** A tree: a trunk and two crowns, a little apart in size and green. */
export function tree(m, pal, rnd, x, z, y = 0) {
  const s = 0.8 + rnd() * 0.6;
  m.prism(x, z, 0.25 * s, 0.2 * s, y, y + 2 * s, pal.trunk, { n: 5 });
  const tint = pal.leaves.map((c, i) => Math.min(1, c * (0.88 + rnd() * 0.24) + (i === 1 ? 0.02 : 0)));
  m.prism(x, z, 2.4 * s, 0, y + 1.4 * s, y + 6.2 * s, tint, { n: 7, turn: rnd() * 6 });
  m.prism(x, z, 1.8 * s, 0, y + 4 * s, y + 8 * s, tint, { n: 7, turn: rnd() * 6 });
}

/** A colour a little apart from `c`, wearing what it wears. */
const vary = (c, rnd, k = 0.14) => wearing(c.map((v) => Math.min(1, v * (1 - k + rnd() * k * 2))), c.mat);
const scale = (c, f) => wearing(c.map((v, i) => Math.min(1, v * f[i])), c.mat);

/** The cars' paints (on a bright day; the evening darkens them as it does the rest). */
const CARS = ['#d9dde3', '#2b2f36', '#8a929e', '#b8342e', '#2f5ea8', '#e8e4da', '#3b6b47', '#c9a227', '#5b6470'];
/** The town's brick and plaster, as tints of the palette's own. */
const BRICKS = [
  [1, 1, 1],
  [0.86, 0.82, 0.8],
  [1.18, 1.08, 0.86],
  [0.95, 0.78, 0.7],
];
const PLASTERS = [
  [1, 1, 1],
  [0.92, 0.96, 1.02],
  [1.02, 0.95, 0.86],
  [0.86, 0.9, 0.92],
  [1.03, 0.98, 0.9],
];

/**
 * The town round the depot's plot `plot` ({ x0, x1, z0, z1 }: the plot's grass, its front at
 * z1) and the street at its front (`street`: { z0, z1 }, kerb to kerb), drawn into `m` with its
 * marks in `marks`; `gate` ({ x0, x1 }) where the yard's lane crosses the pavement. Returns the
 * town's bounds.
 */
export function buildTown(m, marks, { plot, street, gate }, pal, level = { town: 2, trees: true }, seed = 1) {
  const rnd = seeded(seed);
  const rings = Math.max(1, level.town || 1);
  const night = pal.light.night;
  const car = (h) => {
    const c = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
    return night ? c.map((v) => v * 0.45) : c;
  };

  // the grid: across (x) and from the back to the front (z), each a list of [from, to, kind]
  const xs = [[plot.x0, plot.x1, 'depot']];
  const zs = [[plot.z0, street.z0 - WALK, 'depot'], [street.z0 - WALK, street.z1 + WALK, 'road']];
  const grow = (list, before, size) => {
    for (let k = 0; k < rings; k++) {
      const edge = before ? list[0][0] : list[list.length - 1][1];
      const road = before ? [edge - LANE, edge, 'road'] : [edge, edge + LANE, 'road'];
      // (the front street is there already: the first ring at the front starts with a block)
      if (!(list[list.length - 1][2] === 'road' && !before && k === 0)) before ? list.unshift(road) : list.push(road);
      const e = before ? list[0][0] : list[list.length - 1][1];
      before ? list.unshift([e - size, e, 'block']) : list.push([e, e + size, 'block']);
    }
  };
  grow(xs, true, BLOCK_X);
  grow(xs, false, BLOCK_X);
  grow(zs, true, BLOCK_Z);
  grow(zs, false, BLOCK_Z);
  const world = { x0: xs[0][0], x1: xs[xs.length - 1][1], z0: zs[0][0], z1: zs[zs.length - 1][1] };
  const dx = xs.findIndex((c) => c[2] === 'depot');
  const dz = zs.findIndex((c) => c[2] === 'depot');

  // the ground: the town's plinth, its edge of earth
  m.box(world.x0, -2.4, world.z0, world.x1, 0, world.z1, pal.earth, { top: pal.grass });

  const lamp = (x, z) => {
    m.box(x - 0.09, 0.15, z - 0.09, x + 0.09, 6.2, z + 0.09, pal.fence);
    m.box(x - 0.35, 6.0, z - 0.2, x + 0.35, 6.25, z + 0.2, pal.fence);
    m.box(x - 0.3, 5.92, z - 0.15, x + 0.3, 6.0, z + 0.15, pal.lamp, { glow: 1 });
  };
  /** A car parked: its centre, along x (`along`) or z. */
  const parked = (x, z, along) => {
    const c = car(CARS[Math.floor(rnd() * CARS.length)]);
    const [hw, hl] = along ? [2.15, 0.88] : [0.88, 2.15];
    m.box(x - hw, 0.25, z - hl, x + hw, 1.0, z + hl, c);
    const [cw, cl] = along ? [1.15, 0.78] : [0.78, 1.15];
    const off = (rnd() - 0.5) * 0.4;
    m.box(x - cw + (along ? off : 0), 1.0, z - cl + (along ? 0 : off), x + cw + (along ? off : 0), 1.5, z + cl + (along ? 0 : off), pal.busGlass);
    for (const sx of [-1, 1]) for (const sz of [-1, 1]) m.box(x + sx * hw * 0.62 - 0.3, 0, z + sz * hl * 0.62 - 0.3, x + sx * hw * 0.62 + 0.3, 0.55, z + sz * hl * 0.62 + 0.3, pal.tyre);
  };

  // the streets: a stretch between two junctions, or a junction
  xs.forEach(([x0, x1, kx], i) =>
    zs.forEach(([z0, z1, kz], j) => {
      if (kx !== 'road' && kz !== 'road') return;
      const front = kz === 'road' && j === dz + 1 && kx === 'depot';
      if (kx === 'road' && kz === 'road') return junction(x0, x1, z0, z1);
      // (across: the street runs along x; else along z)
      const alongX = kz === 'road';
      const a0 = alongX ? x0 : z0;
      const a1 = alongX ? x1 : z1;
      const b0 = alongX ? z0 : x0;
      const rect = (p0, p1, q0, q1, y, col, area) => (alongX ? m.flat(p0, q0, p1, q1, y, col, area) : m.flat(q0, p0, q1, p1, y, col, area));
      const box = (p0, p1, q0, q1, y0, y1, col, o) => (alongX ? m.box(p0, y0, q0, p1, y1, q1, col, o) : m.box(q0, y0, p0, q1, y1, p1, col, o));
      const mark = (pa, qb, w, l, turn, mat, st) => (alongX ? marks.decal(pa, qb, w, l, turn, mat, st) : marks.decal(qb, pa, w, l, turn + Math.PI / 2, mat, st));
      rect(a0, a1, b0 + WALK, b0 + WALK + ROAD, 0.02, pal.street);
      // the pavements, the yard's lane across the near one at the gate
      for (const q0 of [b0, b0 + WALK + ROAD]) {
        if (front && q0 === b0) {
          box(a0, gate.x0, q0, q0 + WALK, 0, 0.15, pal.kerb, { top: pal.pavement });
          box(gate.x1, a1, q0, q0 + WALK, 0, 0.15, pal.kerb, { top: pal.pavement });
          rect(gate.x0, gate.x1, q0, q0 + WALK, 0.04, pal.asphalt);
        } else box(a0, a1, q0, q0 + WALK, 0, 0.15, pal.kerb, { top: pal.pavement });
      }
      // the paint: the centre's dashes (3 m of every 9), the edges' lines
      const mid = b0 + WALK + ROAD / 2;
      // (clear of the zebras and stop lines at either end)
      for (let a = a0 + 7; a < a1 - 9; a += 9) rect(a, a + 3, mid - 0.06, mid + 0.06, 0.045, pal.paint);
      for (const q of [b0 + WALK + 0.35, b0 + WALK + ROAD - 0.35]) rect(a0 + 6.5, a1 - 6.5, q - 0.06, q + 0.06, 0.045, pal.paint);
      // lamps on one side, trees on the other (not at the yard's own front: its verge has them)
      for (let a = a0 + 8; a < a1 - 4; a += 26) lamp(alongX ? a : b0 + WALK + ROAD + WALK - 0.6, alongX ? b0 + WALK + ROAD + WALK - 0.6 : a);
      if (level.trees && !front) for (let a = a0 + 6 + rnd() * 4; a < a1 - 4; a += 11 + rnd() * 5) tree(m, pal, rnd, alongX ? a : b0 + 1.2, alongX ? b0 + 1.2 : a, 0.15);
      // cars along the far kerb, a gap now and then (none before the depot: its buses wait there)
      if (!front)
        for (let a = a0 + 4; a < a1 - 5; a += 5.6) if (rnd() < 0.55) alongX ? parked(a + 2.5, b0 + WALK + ROAD - 1.25, true) : parked(b0 + WALK + ROAD - 1.25, a + 2.5, false);
      // what use leaves: a manhole now and then, cracks, the tyres' tracks
      for (let a = a0 + 10 + rnd() * 10; a < a1 - 6; a += 28 + rnd() * 16) mark(a, mid + (rnd() - 0.5) * 3, 0.9, 0.9, rnd() * 6, MAT.manhole, 1);
      for (let a = a0 + 5; a < a1 - 5; a += 7 + rnd() * 12) if (rnd() < 0.5) mark(a, b0 + WALK + 1.5 + rnd() * (ROAD - 3), 3 + rnd() * 3, 3 + rnd() * 3, rnd() * 6, MAT.cracks, 0.5 + rnd() * 0.5);
      for (const q of [mid - 2.6, mid + 2.6]) mark((a0 + a1) / 2, q, 1.6, a1 - a0 - 4, Math.PI / 2, MAT.tyres, 0.35);
    }),
  );

  /** A junction: its asphalt, the corners' pavements, a zebra and a stop line on every arm. */
  function junction(x0, x1, z0, z1) {
    m.flat(x0, z0, x1, z1, 0.02, pal.street);
    for (const [cx, cz] of [
      [x0, z0],
      [x1 - WALK, z0],
      [x0, z1 - WALK],
      [x1 - WALK, z1 - WALK],
    ])
      m.box(cx, 0, cz, cx + WALK, 0.15, cz + WALK, pal.kerb, { top: pal.pavement });
    // the zebras just outside the junction, the stop lines before them
    const zebra = (along, c, at, dir) => {
      for (let k = 0; k < 5; k++) {
        const q = c - ROAD / 2 + 0.8 + k * 1.9;
        if (along) m.flat(at + (dir > 0 ? 0.8 : -4.8), q, at + (dir > 0 ? 4.8 : -0.8), q + 0.55, 0.045, pal.paint);
        else m.flat(q, at + (dir > 0 ? 0.8 : -4.8), q + 0.55, at + (dir > 0 ? 4.8 : -0.8), 0.045, pal.paint);
      }
      // (the stop line across the lane coming in: traffic keeps to the right)
      const s = at + dir * 5.6;
      const [q0, q1] = along === dir > 0 ? [c - ROAD / 2, c] : [c, c + ROAD / 2];
      if (along) m.flat(Math.min(s, s + dir * 0.5), q0, Math.max(s, s + dir * 0.5), q1, 0.045, pal.paint);
      else m.flat(q0, Math.min(s, s + dir * 0.5), q1, Math.max(s, s + dir * 0.5), 0.045, pal.paint);
    };
    const cx = (x0 + x1) / 2;
    const cz = (z0 + z1) / 2;
    zebra(true, cz, x1, 1);
    zebra(true, cz, x0, -1);
    zebra(false, cx, z1, 1);
    zebra(false, cx, z0, -1);
    marks.decal(cx + (rnd() - 0.5) * 4, cz + (rnd() - 0.5) * 4, 9, 9, rnd() * 6, MAT.tyres, 0.45);
    if (rnd() < 0.6) marks.decal(cx + (rnd() - 0.5) * 5, cz + (rnd() - 0.5) * 5, 5, 5, rnd() * 6, MAT.cracks, 0.7);
  }

  // the blocks
  xs.forEach(([x0, x1, kx], i) =>
    zs.forEach(([z0, z1, kz], j) => {
      if (kx === 'road' || kz === 'road' || (kx === 'depot' && kz === 'depot')) return;
      const ring = Math.max(Math.abs(i - dx), Math.abs(j - dz)) >> 1;
      const r = { x0: x0 + 1, x1: x1 - 1, z0: z0 + 1, z1: z1 - 1 };
      const pick = rnd();
      if (ring <= 1 && pick < 0.75) estate(r);
      else if (pick < 0.16) park(r);
      else flats(r);
    }),
  );

  /** A hall of the trading estate with its car park. */
  function estate(r) {
    const w = r.x1 - r.x0;
    const d = r.z1 - r.z0;
    const hx = r.x0 + 4 + rnd() * 6;
    const hz = r.z0 + 4 + rnd() * 6;
    const hw = w * (0.45 + rnd() * 0.25);
    const hd = d * (0.45 + rnd() * 0.2);
    const h = 6 + rnd() * 5;
    const wall = vary(pal.townHall, rnd, 0.12);
    m.box(hx, 0, hz, hx + hw, h, hz + hd, wall, { top: pal.roof });
    m.box(hx + hw * 0.1, 0, hz + hd - 0.05, hx + hw * 0.1 + 4.5, 4.6, hz + hd + 0.08, pal.door);
    m.box(hx + hw * 0.1 + 6, 0, hz + hd - 0.05, hx + hw * 0.1 + 10.5, 4.6, hz + hd + 0.08, pal.door);
    m.box(hx - 0.05, h - 1.6, hz + hd - 0.05, hx + hw + 0.05, h - 1.0, hz + hd + 0.1, vary(pal.trim, rnd, 0.2), { glow: 0.3 });
    // its car park, beside it
    const px0 = Math.min(r.x1 - 6, hx + hw + 3);
    if (r.x1 - px0 > 10) {
      m.flat(px0, r.z0 + 2, r.x1 - 2, r.z1 - 2, 0.02, pal.asphalt);
      for (let z = r.z0 + 4; z < r.z1 - 6; z += 2.6) {
        m.flat(px0 + 1, z - 0.06, px0 + 6, z + 0.06, 0.045, pal.paint);
        if (rnd() < 0.5) parked(px0 + 3.5, z + 1.3, true);
        if (rnd() < 0.3) marks.decal(px0 + 3.5, z + 1.3, 2.2, 2.2, rnd() * 6, MAT.oil, 0.6);
      }
      marks.decal((px0 + r.x1) / 2, (r.z0 + r.z1) / 2, r.x1 - px0, r.z1 - r.z0 - 6, 0, MAT.tyres, 0.25);
    }
    if (level.trees) for (let x = r.x0 + 3; x < r.x1 - 2; x += 9 + rnd() * 6) if (rnd() < 0.7) tree(m, pal, rnd, x, r.z0 + 1.5);
  }

  /** A park: grass, paths across it, trees, benches. */
  function park(r) {
    const cx = (r.x0 + r.x1) / 2;
    const cz = (r.z0 + r.z1) / 2;
    m.flat(r.x0, cz - 1.2, r.x1, cz + 1.2, 0.03, pal.pavement);
    m.flat(cx - 1.2, r.z0, cx + 1.2, r.z1, 0.03, pal.pavement);
    m.prism(cx, cz, 4, 4, 0, 0.5, pal.concrete, { n: 10 });
    m.prism(cx, cz, 3.4, 3.4, 0.5, 0.55, pal.washLane, { n: 10, cap: true });
    if (level.trees)
      for (let k = 0; k < 18; k++) {
        const x = r.x0 + 3 + rnd() * (r.x1 - r.x0 - 6);
        const z = r.z0 + 3 + rnd() * (r.z1 - r.z0 - 6);
        if (Math.abs(x - cx) > 3 && Math.abs(z - cz) > 3) tree(m, pal, rnd, x, z);
      }
  }

  /** A block of flats: houses round its edges, each its own height and front, a yard inside. */
  function flats(r) {
    const DEEP = 11;
    const side = (a0, a1, along, q0, q1, dir) => {
      let a = a0;
      while (a < a1 - 6) {
        const len = Math.min(a1 - a, 9 + rnd() * 14);
        const floors = 2 + Math.floor(rnd() * (rnd() < 0.15 ? 9 : 5));
        const h = floors * STOREY + 0.6;
        const brick = rnd() < 0.5;
        const front = scale(brick ? pal.brickFront : pal.plasterFront, (brick ? BRICKS : PLASTERS)[Math.floor(rnd() * (brick ? BRICKS : PLASTERS).length)]);
        const [x0, z0, x1, z1] = along ? [a, q0, a + len, q1] : [q0, a, q1, a + len];
        const pitched = floors <= 4 && rnd() < 0.55;
        m.box(x0, 0, z0, x1, h, z1, front, { top: pitched ? pal.roof : pal.townRoof });
        if (pitched) {
          // (the ridge along the street)
          if (along) m.gable(x0, z0, x1, z1, h, h + 3.2, pal.roof);
          else gableZ(x0, z0, x1, z1, h, h + 3.2);
        } else m.box(x0 + 1, h, z0 + 1, Math.min(x1 - 1, x0 + 3.5), h + 1.4, Math.min(z1 - 1, z0 + 3), pal.metal);
        // the front door on the street
        const d = along ? [x0 + len / 2 - 0.7, dir > 0 ? z1 - 0.05 : z0 - 0.08, x0 + len / 2 + 0.7, dir > 0 ? z1 + 0.08 : z0 + 0.05] : [dir > 0 ? x1 - 0.05 : x0 - 0.08, z0 + len / 2 - 0.7, dir > 0 ? x1 + 0.08 : x0 + 0.05, z0 + len / 2 + 0.7];
        m.box(d[0], 0, d[1], d[2], 2.3, d[3], pal.door);
        a += len;
      }
    };
    side(r.x0, r.x1, true, r.z1 - DEEP, r.z1, 1);
    side(r.x0, r.x1, true, r.z0, r.z0 + DEEP, -1);
    side(r.z0 + DEEP, r.z1 - DEEP, false, r.x0, r.x0 + DEEP, -1);
    side(r.z0 + DEEP, r.z1 - DEEP, false, r.x1 - DEEP, r.x1, 1);
    if (level.trees) tree(m, pal, rnd, (r.x0 + r.x1) / 2 + (rnd() - 0.5) * 8, (r.z0 + r.z1) / 2 + (rnd() - 0.5) * 6);
  }

  /** A roof's ridge along z (`Mesh.gable` has it along x). */
  function gableZ(x0, z0, x1, z1, y0, y1) {
    const g = new m.constructor();
    g.gable(z0, x0, z1, x1, y0, y1, pal.roof);
    m.append(g, { pt: ([x, y, z]) => [z, y, x], dir: ([x, y, z]) => [z, y, x] });
  }

  return world;
}
