// The yard's rules on the page (`hub_core::company::yard` has them, and decides): where a
// module of an area may be put down - on the land, not on the gate's lane, not on another
// module, its apron free and on no other's -, so the build mode shows at once whether it fits;
// and where a room may be laid out in a building (`hub_core::company::rooms`).
// The layout is the depot page's (`plan.layout`): cells, the land's columns and rows, the gate,
// the modules with their cells and aprons, the shapes of the modules to put down.

/** The cells of a module of `area` at (x, y) turned `rot` (0: its front to the street, 1: to
 * the east, 2: to the back, 3: to the west). */
export function rectOf(L, area, x, y, rot) {
  const s = L.shapes[area] || { w: 1, h: 1 };
  return rot % 2 === 0 ? { x, y, w: s.w, h: s.h } : { x, y, w: s.h, h: s.w };
}

/** Its aprons: in front and behind, as it is turned. */
export function apronsOf(L, area, x, y, rot) {
  const s = L.shapes[area];
  if (!s) return [];
  const r = rectOf(L, area, x, y, rot);
  const side = (dir, d) => [
    { x: r.x, y: r.y + r.h, w: r.w, h: d },
    { x: r.x + r.w, y: r.y, w: d, h: r.h },
    { x: r.x, y: r.y - d, w: r.w, h: d },
    { x: r.x - d, y: r.y, w: d, h: r.h },
  ][dir % 4];
  return [
    [rot, s.front],
    [rot + 2, s.back],
  ]
    .filter(([, d]) => d > 0)
    .map(([dir, d]) => side(dir, d));
}

const overlaps = (a, b) => a.w > 0 && a.h > 0 && b.w > 0 && b.h > 0 && a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
const inside = (a, b) => a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w && a.y + a.h <= b.y + b.h;

/** Why a module of `area` does not fit at (x, y) turned `rot` ('land', 'gate', 'taken',
 * 'lane'), or null when it does; `ignore` the module being moved. */
export function whyNot(L, area, x, y, rot, ignore = null) {
  if (!L.shapes[area]) return 'land';
  const r = rectOf(L, area, x, y, rot);
  const aprons = apronsOf(L, area, x, y, rot);
  const land = { x: 0, y: 0, w: L.cols, h: L.rows };
  if (!inside(r, land) || aprons.some((a) => !inside(a, land))) return 'land';
  if (overlaps(r, L.gate)) return 'gate';
  for (const o of L.modules) {
    if (o.id === ignore) continue;
    if (overlaps(r, o)) return 'taken';
    if (o.aprons.some((a) => overlaps(a, r)) || aprons.some((a) => overlaps(a, o))) return 'lane';
  }
  return null;
}

/** Why a room does not fit at `r` (metres in the building's own frame) on `floor` of module
 * `o` ('small', 'outside', 'taken'), or null when it does (`rooms::fits`). */
export function roomWhyNot(L, o, floor, r) {
  const room = L.shapes[o.area]?.inside;
  if (r.w < 2 || r.h < 2) return 'small';
  if (!room || !inside(r, room)) return 'outside';
  if ((o.rooms || []).some((x) => x.floor === floor && overlaps(x, r))) return 'taken';
  return null;
}

/** The first free spot of a module of `area` (turned to the street if it can be, from the back,
 * from the west): { x, y, rot }. */
export function spot(L, area) {
  for (const rot of [0, 2, 1, 3])
    for (let y = 0; y < L.rows; y++)
      for (let x = 0; x < L.cols; x++) if (!whyNot(L, area, x, y, rot)) return { x, y, rot };
  return null;
}

/** A module as the layout lists it. */
export function moduleAt(L, id, area, x, y, rot, extra = {}) {
  return { id, area, ...rectOf(L, area, x, y, rot), rot, aprons: apronsOf(L, area, x, y, rot), building: false, works: null, ...extra };
}
