// The depot's colours (sRGB, 0 to 1) and its light, for the light look (a bright day) and the
// dark one (the evening: a cooler, dimmer light, the windows, lamps and screens lit). The
// accent is the one chosen in the settings (`--hub-accent`); the company's own look (its walls,
// roofs, trims and doors, and what the walls are made of: `plan.look`) goes over the rest.
//
// A colour of a surface carries its material (`mat`): the texture it wears, a layer of
// `img/depot/detail.jpg` (`MATERIALS`, in its order), tinted by the colour. A material from
// `WINDOWS` up is a facade of the town with its windows (`MATERIALS` less `WINDOWS`).

const hex = (h) => {
  const v = parseInt(String(h).trim().replace(/^#/, ''), 16);
  return Number.isFinite(v) ? [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255] : [0.16, 0.46, 0.97];
};

/** `#rrggbb` as [r, g, b] from 0 to 1. */
export const rgb01 = hex;

/** The textures, a layer each in this order (`tools/depot-textures.py` makes the picture): a
 * name, how many metres one covers, how strongly it marks its colour, and its kind - a surface
 * (`plane`, its picture over the ground or the walls), the road's paint (`paint`, the asphalt
 * showing through where it is worn) or a mark laid on what is under it (`decal`). */
export const MATERIALS = [
  ['asphalt', 6, 0.55, 'plane'],
  ['road', 8, 0.6, 'plane'],
  ['concrete', 6, 0.45, 'plane'],
  ['grass', 5, 0.7, 'plane'],
  ['earth', 3, 0.6, 'plane'],
  ['cladding', 4, 0.5, 'plane'],
  ['roofsheet', 3, 0.5, 'plane'],
  ['shutter', 3, 0.55, 'plane'],
  ['brick', 3.2, 0.6, 'plane'],
  ['plaster', 4, 0.35, 'plane'],
  ['paving', 1.8, 0.5, 'plane'],
  ['gravel', 2.5, 0.55, 'plane'],
  ['tiles', 1.5, 0.35, 'plane'],
  ['washfloor', 5, 0.5, 'plane'],
  ['epoxy', 6, 0.45, 'plane'],
  ['formwork', 5, 0.45, 'plane'],
  ['blocks', 3.2, 0.5, 'plane'],
  ['paint', 2, 1, 'paint'],
  ['cracks', 1, 0.85, 'decal'],
  ['tyres', 1, 0.6, 'decal'],
  ['oil', 1, 0.7, 'decal'],
  ['manhole', 1, 0.9, 'decal'],
];
/** A material's number (1 the first; 0 none). */
export const MAT = Object.fromEntries(MATERIALS.map(([name], k) => [name, k + 1]));
/** Added to a facade's material for its windows. */
export const WINDOWS = 32;

/** A colour that wears a material (a copy: the palette's own stay as they are). */
export const wearing = (c, mat) => Object.assign([...c], { mat });

/** Which palette colours wear what. */
const WEARS = {
  asphalt: 'asphalt',
  street: 'road',
  kerb: 'concrete',
  concrete: 'concrete',
  pit: 'concrete',
  grass: 'grass',
  earth: 'earth',
  roof: 'roofsheet',
  door: 'shutter',
  paint: 'paint',
  washLane: 'washfloor',
  washWall: 'tiles',
  hallFloor: 'epoxy',
  hallWall: 'blocks',
  garage: 'formwork',
  pavement: 'paving',
  townRoof: 'gravel',
  townHall: 'cladding',
};

/** The walls' finishes (`plan.look.finish`): the material and the colour they have unless the
 * company chose one. */
const FINISH = { cladding: ['cladding', null], brick: ['brick', '#a65f48'], plaster: ['plaster', '#e3d8c6'] };

const DAY = {
  grass: hex('#a9c99a'),
  earth: hex('#8f7a62'),
  asphalt: hex('#8a909b'),
  street: hex('#6b717c'),
  kerb: hex('#c9ccd2'),
  paint: hex('#f4f5f7'),
  plot: hex('#eef1f6'),
  fence: hex('#5f6673'),
  barrier: hex('#e8503a'),
  wall: hex('#e9ebef'),
  roof: hex('#b7bec9'),
  glass: hex('#5c7391'),
  concrete: hex('#c3c6cc'),
  pit: hex('#3d424c'),
  door: hex('#8c95a3'),
  lamp: hex('#f2f2ea'),
  metal: hex('#9aa2ae'),
  washLane: hex('#9db8de'),
  brush: hex('#3b82f6'),
  pump: hex('#eef0f3'),
  screen: hex('#7fd0ff'),
  charger: hex('#e9edf2'),
  ok: hex('#34b26e'),
  warn: hex('#f0a63a'),
  trunk: hex('#7a5a3e'),
  leaves: hex('#5c9a4f'),
  busGlass: hex('#2a3442'),
  busRoof: hex('#d9dde3'),
  display: hex('#ffb547'),
  tyre: hex('#1e2126'),
  bellows: hex('#2b2f36'),
  washWall: hex('#e4e9ef'),
  hallFloor: hex('#9aa1a9'),
  hallWall: hex('#d6d9de'),
  garage: hex('#b9bcc1'),
  pavement: hex('#c2c4c7'),
  townRoof: hex('#8d9096'),
  brick: hex('#a65f48'),
  plaster: hex('#e6dccb'),
  townGlass: hex('#41556e'),
  townHall: hex('#c8cdd4'),
  light: { sun: [1.15, 1.08, 0.98], sky: [0.56, 0.62, 0.74], ground: [0.3, 0.29, 0.28], night: 0 },
};

const EVENING = {
  ...DAY,
  grass: hex('#3f5a43'),
  earth: hex('#3b3330'),
  asphalt: hex('#4a505c'),
  street: hex('#3c414b'),
  kerb: hex('#6b707a'),
  paint: hex('#c9cdd6'),
  plot: hex('#aab3c4'),
  wall: hex('#a9b0bd'),
  roof: hex('#6f7786'),
  glass: hex('#e8c27a'),
  concrete: hex('#7c818b'),
  lamp: hex('#fff1c9'),
  washLane: hex('#5b77a3'),
  leaves: hex('#3f6b45'),
  washWall: hex('#9aa3b1'),
  hallFloor: hex('#5a616c'),
  hallWall: hex('#7f8692'),
  garage: hex('#6d727b'),
  pavement: hex('#6c717a'),
  townRoof: hex('#4b5059'),
  brick: hex('#6e4639'),
  plaster: hex('#9c968e'),
  townGlass: hex('#e9c47c'),
  townHall: hex('#79808c'),
  light: { sun: [0.62, 0.68, 0.86], sky: [0.3, 0.34, 0.46], ground: [0.12, 0.12, 0.14], night: 1 },
};

/** A colour the company chose, in the evening's light: as much darker as the palette's own. */
function evening(c, key) {
  return c.map((v, i) => Math.min(1, (v * EVENING[key][i]) / Math.max(0.02, DAY[key][i])));
}

/** The palette of the look now on the page, with the company's own `look` (`plan.look`:
 * `wall`, `roof`, `trim`, `door` as `#rrggbb` or null, `finish`) over it. */
export function palette(look = null) {
  const root = document.documentElement;
  const dark = root.dataset.theme !== 'light';
  const accent = hex(getComputedStyle(root).getPropertyValue('--hub-accent') || '#2a75f7');
  const pal = { ...(dark ? EVENING : DAY), accent, trim: accent, dark };
  const [finish, wall] = FINISH[look?.finish] || FINISH.cladding;
  const own = { wall: look?.wall || wall, roof: look?.roof, trim: look?.trim, door: look?.door };
  for (const [key, h] of Object.entries(own)) {
    if (!h || !/^#?[0-9a-f]{6}$/i.test(h)) continue;
    pal[key] = dark && DAY[key] ? evening(hex(h), key) : hex(h);
  }
  for (const [key, mat] of Object.entries(WEARS)) pal[key] = wearing(pal[key], MAT[mat]);
  pal.wall = wearing(pal.wall, MAT[finish]);
  pal.brickFront = wearing(pal.brick, MAT.brick + WINDOWS);
  pal.plasterFront = wearing(pal.plaster, MAT.plaster + WINDOWS);
  return pal;
}
