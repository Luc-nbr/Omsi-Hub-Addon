// The inside of a building opened on the depot page, as a doll's house (the Sims' walls cut
// down): the offices' floor chosen (the floors under it whole, the ones over it and the roof
// left off) or a workshop bay without its roof, the outer walls cut at hip height; and the rooms
// the player laid out on it (`hub_core::company::rooms`), each its floor's colour, its walls
// cut as low with a door towards the middle of the building, and its furniture: desks, a
// meeting table, sofas and a kitchenette, lockers, toilets, a counter, shelves, benches and
// racks. Everything here is drawn in the building's own frame (x across, z from its back, in
// metres; `scene.js` turns it onto the yard), the rooms' rectangles as the layout lists them.

import { Mesh } from './mesh.js';
import { MAT, wearing } from './palette.js';

/** How high the walls are cut, and a storey of the offices (`scene.js` draws them so). */
export const CUT = 1.15;
export const STOREY = 3.6;
/** Where a floor of the offices lies. */
export const floorY = (floor) => 0.15 + floor * STOREY;

/** The rooms' floors and their furniture's main colour, by kind, on a bright day. */
const LOOK = {
  office: ['#c9b48f', '#e9e5dc'],
  meeting: ['#b9a07a', '#6b4f3a'],
  rest: ['#a8b98f', '#c46a4a'],
  lockers: ['#9fb0c4', '#5f7fa8'],
  toilets: ['#d9e2e8', '#f4f6f8'],
  reception: ['#c7b9a6', '#3d4f6b'],
  archive: ['#b0aaa0', '#8a7a5c'],
  tools: ['#9aa1a9', '#c83c32'],
  parts: ['#9aa1a9', '#3c6fb4'],
  mechanics: ['#b4a58a', '#e0d4bd'],
  foreman: ['#c9b48f', '#e9e5dc'],
};
const hex = (h, k) => [1, 3, 5].map((i) => (parseInt(h.slice(i, i + 2), 16) / 255) * k);

/** The building's own frame's point under a point of the yard (metres), as `scene.js`'s
 * `frame(rot, x0, z0, lw, ld)` turns it: the inverse of its `pt`. */
export function toLocal(rot, x0, z0, lw, ld, wx, wz) {
  const dx = wx - x0;
  const dz = wz - z0;
  return [
    [dx, dz],
    [lw - dz, dx],
    [lw - dx, ld - dz],
    [dz, ld - dx],
  ][((rot % 4) + 4) % 4];
}

/** The offices opened at `floor`: `r` the module's own rectangle (its building from 4 m on);
 * drawn into `m` in its frame. */
export function officesInside(m, r, pal, floor, A) {
  const z0 = r.z0 + 4;
  const fy = floorY(floor);
  // the floors under this one, whole; their windows' band at the top
  if (floor > 0) {
    m.box(r.x0, 0, z0, r.x1, fy, r.z1, pal.wall, { area: A, top: pal.concrete });
    for (let k = 0; k < floor; k++) m.box(r.x0 - 0.05, 1.9 + k * STOREY, z0 + 1, r.x1 + 0.05, 1.9 + k * STOREY + 1.6, r.z1 - 1, pal.glass, { area: A, glow: 0.8 });
  } else m.box(r.x0, 0, z0, r.x1, fy, r.z1, pal.wall, { area: A, top: pal.concrete });
  // this floor: its screed, its outer walls cut low, the way in at the front on the ground floor
  m.flat(r.x0 + 0.25, z0 + 0.25, r.x1 - 0.25, r.z1 - 0.25, fy + 0.01, pal.hallFloor, A);
  walls(m, r.x0, z0, r.x1, r.z1, fy, pal.wall, 0.25, floor === 0 ? 'front' : null, A);
  // the stairs in its back corner, on every floor
  for (let k = 0; k < 6; k++) m.box(r.x1 - 2.4, fy, z0 + 0.3 + k * 0.45, r.x1 - 0.3, fy + 0.18 * (k + 1), z0 + 0.75 + k * 0.45, pal.concrete, { area: A });
}

/** A workshop bay opened: its floor, its pit, its walls cut low (open at its front). */
export function bayInside(m, r, pal, A) {
  m.flat(r.x0 + 0.3, r.z0 + 0.3, r.x1 - 0.3, r.z1, 0.04, pal.hallFloor, A);
  const cx = (r.x0 + r.x1) / 2;
  m.flat(cx - 0.5, r.z0 + 7, cx + 0.5, r.z1 - 3, 0.06, pal.pit, A);
  const t = 0.5;
  m.box(r.x0, 0, r.z0, r.x1, CUT, r.z0 + t, pal.wall, { area: A, top: pal.roof });
  m.box(r.x0, 0, r.z0, r.x0 + t, CUT, r.z1, pal.wall, { area: A, top: pal.roof });
  m.box(r.x1 - t, 0, r.z0, r.x1, CUT, r.z1, pal.wall, { area: A, top: pal.roof });
}

/** Walls round a rectangle from `y` up to the cut, `t` thick; `door` the side left open in its
 * middle ('front', 'back', 'left', 'right'). */
function walls(m, x0, z0, x1, z1, y, col, t, door, A = 0) {
  const top = y + CUT;
  const opt = { area: A, top: col };
  const gap = (a0, a1) => {
    const c = (a0 + a1) / 2;
    return [
      [a0, c - 0.5],
      [c + 0.5, a1],
    ];
  };
  const along = (a0, a1, open, draw) => (open ? gap(a0, a1).forEach(([p, q]) => q > p && draw(p, q)) : draw(a0, a1));
  along(x0, x1, door === 'back', (p, q) => m.box(p, y, z0, q, top, z0 + t, col, opt));
  along(x0, x1, door === 'front', (p, q) => m.box(p, y, z1 - t, q, top, z1, col, opt));
  along(z0, z1, door === 'left', (p, q) => m.box(x0, y, p, x0 + t, top, q, col, opt));
  along(z0, z1, door === 'right', (p, q) => m.box(x1 - t, y, p, x1, top, q, col, opt));
}

/**
 * The rooms on a floor (`rooms`: the layout's, in metres of the building's frame), at height
 * `y`, in a building whose part to lay out is `room` (the rooms' doors face its middle). The
 * day (`pal.day`) darkens their colours; their floors glow a little at night, lit inside.
 */
export function roomsMesh(m, rooms, room, y, pal) {
  const day = 0.45 + 0.55 * (pal.day ?? 1);
  const mx = room.x + room.w / 2;
  const mz = room.y + room.h / 2;
  for (const o of rooms) {
    const [floor, main] = (LOOK[o.kind] || LOOK.office).map((h) => hex(h, day));
    const x0 = o.x;
    const z0 = o.y;
    const x1 = o.x + o.w;
    const z1 = o.y + o.h;
    m.flat(x0 + 0.06, z0 + 0.06, x1 - 0.06, z1 - 0.06, y + 0.025, wearing(floor, MAT.tiles), 0, 0.35);
    // its door on the side towards the building's middle
    const cx = (x0 + x1) / 2;
    const cz = (z0 + z1) / 2;
    const door = Math.abs(mx - cx) / room.w > Math.abs(mz - cz) / room.h ? (mx > cx ? 'right' : 'left') : mz > cz ? 'front' : 'back';
    walls(m, x0, z0, x1, z1, y, pal.wall, 0.12, door);
    furnish(m, o, { x0: x0 + 0.5, z0: z0 + 0.5, x1: x1 - 0.5, z1: z1 - 0.5 }, y, main, pal, door);
  }
}

/** A room's furniture within `r` (its walls kept clear), at floor height `y`. */
function furnish(m, o, r, y, main, pal, door) {
  const w = r.x1 - r.x0;
  const d = r.z1 - r.z0;
  const dark = pal.tyre;
  const metal = pal.metal;
  const block = (x0, z0, x1, z1, h, col, glow = 0) => m.box(x0, y, z0, x1, y + h, z1, col, { glow });
  // a row of things along the back wall, unless the door is there
  const backWall = door === 'back' ? 'front' : 'back';
  const alongWall = (depth, h, col, step, size) => {
    const z = backWall === 'back' ? r.z0 : r.z1 - depth;
    for (let x = r.x0; x + size <= r.x1 + 0.01; x += step) block(x, z, x + size, z + depth, h, col);
  };
  switch (o.kind) {
    case 'office':
    case 'foreman': {
      // desks of 1.6 by 0.8, a chair and a screen each, 2.4 m apart
      for (let z = r.z0 + 0.2; z + 1.4 <= r.z1; z += 2.4)
        for (let x = r.x0; x + 1.6 <= r.x1 + 0.01; x += 2.2) {
          block(x, z, x + 1.6, z + 0.8, 0.74, main);
          block(x + 0.6, z + 0.15, x + 1.0, z + 0.2, 1.1, dark, 0.5);
          block(x + 0.55, z + 1.0, x + 1.05, z + 1.5, 0.5, dark);
        }
      if (o.kind === 'foreman') block(r.x1 - 0.5, r.z0, r.x1, r.z0 + 1.2, 1.6, metal);
      break;
    }
    case 'meeting': {
      const tw = Math.max(0.8, w - 1.2);
      const td = Math.min(1.2, Math.max(0.6, d - 1.6));
      const tz = (r.z0 + r.z1) / 2 - td / 2;
      block(r.x0 + 0.6, tz, r.x0 + 0.6 + tw, tz + td, 0.74, main);
      for (let x = r.x0 + 0.9; x < r.x0 + 0.6 + tw - 0.3; x += 0.9) {
        block(x, tz - 0.6, x + 0.45, tz - 0.15, 0.48, dark);
        block(x, tz + td + 0.15, x + 0.45, tz + td + 0.6, 0.48, dark);
      }
      break;
    }
    case 'rest':
    case 'mechanics': {
      // a kitchenette along the back wall, a sofa (or a table) and a low table before it
      alongWall(0.6, 0.9, pal.wall, Math.max(1, w), Math.min(w, 2.4));
      const cz = (r.z0 + r.z1) / 2;
      if (o.kind === 'rest') {
        block(r.x0, cz - 0.45, r.x0 + Math.min(2.2, w), cz + 0.45, 0.45, main);
        block(r.x0, cz + 0.25, r.x0 + Math.min(2.2, w), cz + 0.45, 0.85, main);
        block(r.x0 + 0.4, cz - 1.4, r.x0 + 1.4, cz - 0.8, 0.4, pal.door);
      } else {
        block(r.x0 + 0.3, cz - 0.5, r.x0 + Math.min(1.9, w), cz + 0.5, 0.74, main);
        block(r.x1 - 0.5, r.z0 + 0.8, r.x1, r.z0 + 1.2, 1.0, dark, 0.6);
      }
      break;
    }
    case 'lockers': {
      alongWall(0.5, 1.9, main, 0.5, 0.48);
      if (d > 2) block(r.x0 + 0.2, (r.z0 + r.z1) / 2 - 0.2, r.x1 - 0.2, (r.z0 + r.z1) / 2 + 0.2, 0.45, pal.door);
      break;
    }
    case 'toilets': {
      // cubicles of 1.1 m along the back wall, their partitions; basins on the other side
      const z = backWall === 'back' ? r.z0 : r.z1 - 1.4;
      for (let x = r.x0; x + 1.1 <= r.x1 + 0.01; x += 1.1) {
        block(x, z, x + 0.05, z + 1.4, 1.9, main);
        block(x + 0.35, backWall === 'back' ? z : z + 1.0, x + 0.75, backWall === 'back' ? z + 0.4 : z + 1.4, 0.45, main);
      }
      const bz = backWall === 'back' ? r.z1 - 0.45 : r.z0;
      for (let x = r.x0 + 0.2; x + 0.5 <= r.x1; x += 0.9) block(x, bz, x + 0.5, bz + 0.45, 0.85, main);
      break;
    }
    case 'reception': {
      const cz = (r.z0 + r.z1) / 2;
      block(r.x0 + 0.4, cz, r.x1 - 0.4, cz + 0.6, 1.05, main);
      block(r.x0 + 0.4, cz - 1.2, r.x0 + 1.0, cz, 1.05, main);
      for (let x = r.x0 + 0.6; x < r.x1 - 1; x += 0.7) block(x, r.z1 - 0.5, x + 0.45, r.z1, 0.45, pal.door);
      break;
    }
    case 'archive':
    case 'parts': {
      // rows of shelves, 1.2 m apart; boxes in the parts' racks
      for (let z = r.z0; z + 0.45 <= r.z1 + 0.01; z += 1.25) {
        block(r.x0, z, r.x1, z + 0.45, 1.9, o.kind === 'parts' ? metal : main);
        if (o.kind === 'parts') for (let x = r.x0 + 0.1; x + 0.4 <= r.x1; x += 0.55) block(x, z + 0.05, x + 0.4, z + 0.4, 2.0, main);
      }
      break;
    }
    case 'tools': {
      // a bench along the back wall, red tool cabinets beside it
      alongWall(0.7, 0.9, metal, Math.max(1, w), Math.min(w, 3));
      for (let x = r.x0; x + 0.7 <= r.x1 + 0.01 && x < r.x0 + 3; x += 0.8) block(x, (r.z0 + r.z1) / 2, x + 0.7, (r.z0 + r.z1) / 2 + 0.5, 1.0, main);
      break;
    }
    default:
  }
}

/** The grid of the part to lay out (`room`) at height `y`, a line a metre, its edge bolder. */
export function roomGrid(room, y, pal) {
  const m = new Mesh();
  for (let x = room.x; x <= room.x + room.w; x++) m.flat(x - 0.025, room.y, x + 0.025, room.y + room.h, y + 0.04, pal.paint);
  for (let z = room.y; z <= room.y + room.h; z++) m.flat(room.x, z - 0.025, room.x + room.w, z + 0.025, y + 0.04, pal.paint);
  m.dashed(room.x + 0.1, room.y + 0.1, room.x + room.w - 0.1, room.y + room.h - 0.1, pal.paint, 0, { w: 0.12, y: y + 0.05 });
  return m;
}

/** A room being drawn: its floor and its walls' outline, to be tinted. */
export function roomGhost(r, y, pal) {
  const m = new Mesh();
  m.box(r.x + 0.04, y + 0.02, r.y + 0.04, r.x + r.w - 0.04, y + 0.12, r.y + r.h - 0.04, pal.wall);
  walls(m, r.x, r.y, r.x + r.w, r.y + r.h, y, pal.wall, 0.12, null);
  return m;
}
