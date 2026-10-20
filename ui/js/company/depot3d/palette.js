// The depot's colours (sRGB, 0 to 1) and its light by the company's time of day: a bright day,
// the night (a cooler, dimmer light, the windows, lamps and screens lit), and the dawn and the
// dusk between them (`daylight`). The colours change in a few steps (the yard is built again
// for each: `palette`), the light and the sun's way over the sky with the clock (`lightAt`).
// The accent is the one chosen in the settings (`--hub-accent`); the company's own look (its
// walls, roofs, trims and doors, and what the walls are made of: `plan.look`) goes over the rest.
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
  boothWall: 'plaster',
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
  solar: hex('#22385a'),
  tank: hex('#dfe3e8'),
  adblue: hex('#2f6fd6'),
  boothWall: hex('#f2f4f6'),
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
  solar: hex('#1a2a42'),
  tank: hex('#8a909a'),
  boothWall: hex('#a9afb9'),
  light: { sun: [0.62, 0.68, 0.86], sky: [0.3, 0.34, 0.46], ground: [0.12, 0.12, 0.14], night: 1 },
};

/** A colour the company chose, at night: as much darker as the palette's own. */
function evening(c, key) {
  return c.map((v, i) => Math.min(1, (v * EVENING[key][i]) / Math.max(0.02, DAY[key][i])));
}

const mix = (a, b, t) => a.map((v, i) => v + (b[i] - v) * t);

/** How light it is at `minute` of the company's day: 0 the night, 1 the day; the sun rises
 * from 5:30 to 7:30 and sets from 18:30 to 20:30. */
export function daylight(minute) {
  const h = (((minute / 60) % 24) + 24) % 24;
  const ramp = (a, b) => Math.max(0, Math.min(1, (h - a) / (b - a)));
  return Math.min(ramp(5.5, 7.5), 1 - ramp(18.5, 20.5));
}

/** The steps the colours take between the night and the day (each is a yard built again). */
const STEPS = 4;
/** `daylight` in the colours' steps. */
export const dayStep = (minute) => Math.round(daylight(minute) * STEPS) / STEPS;

/** The light at `minute`: the sun's colour and its way (from the east in the morning, high at
 * noon, to the west; the moon's from the south-west at night), the sky's and the ground's, how
 * lit the windows are (`night`). */
export function lightAt(minute) {
  const day = daylight(minute);
  const L = { sun: mix(EVENING.light.sun.map((v) => v * 0.55), DAY.light.sun, day), sky: mix(EVENING.light.sky.map((v) => v * 0.8), DAY.light.sky, day), ground: mix(EVENING.light.ground, DAY.light.ground, day), night: Math.min(1, (1 - day) * 1.5) };
  // (the dusk's sun is warmer)
  const low = day > 0 && day < 1 ? Math.sin(day * Math.PI) * 0.25 : 0;
  L.sun = [L.sun[0] * (1 + low), L.sun[1] * (1 + low * 0.35), L.sun[2] * (1 - low * 0.4)];
  const h = (((minute / 60) % 24) + 24) % 24;
  if (h >= 6 && h <= 20) {
    // (6:00 in the east, 13:00 in the south, 20:00 in the west; as high as autumn has it)
    const a = ((h - 6) / 14) * Math.PI;
    const up = 0.18 + Math.sin(a) * 0.62;
    L.dir = [-Math.cos(a), up, 0.35 + Math.sin(a) * 0.25];
  } else L.dir = [-0.55, 0.9, 0.5];
  return L;
}

/** The sky behind the town at `minute`, as a CSS background. */
export function skyAt(minute) {
  const day = daylight(minute);
  const top = mix([12, 18, 32], [150, 186, 222], day).map(Math.round);
  const low = mix([28, 34, 52], [214, 226, 238], day).map(Math.round);
  return `linear-gradient(rgb(${top}), rgb(${low}))`;
}

/** The palette at `day` (0 the night to 1 the day, as `dayStep` gives it), with the company's
 * own `look` (`plan.look`: `wall`, `roof`, `trim`, `door` as `#rrggbb` or null, `finish`) over
 * it. */
export function palette(look = null, day = 1) {
  const root = document.documentElement;
  const accent = hex(getComputedStyle(root).getPropertyValue('--hub-accent') || '#2a75f7');
  const pal = { accent, trim: accent, dark: day < 0.5, day };
  for (const [key, c] of Object.entries(DAY)) pal[key] = Array.isArray(c) ? mix(EVENING[key], c, day) : c;
  pal.light = { ...DAY.light, night: 1 - day };
  pal.trim = accent;
  pal.accent = accent;
  const [finish, wall] = FINISH[look?.finish] || FINISH.cladding;
  const own = { wall: look?.wall || wall, roof: look?.roof, trim: look?.trim, door: look?.door };
  for (const [key, h] of Object.entries(own)) {
    if (!h || !/^#?[0-9a-f]{6}$/i.test(h)) continue;
    pal[key] = DAY[key] ? mix(evening(hex(h), key), hex(h), day) : hex(h);
  }
  for (const [key, mat] of Object.entries(WEARS)) pal[key] = wearing(pal[key], MAT[mat]);
  pal.wall = wearing(pal.wall, MAT[finish]);
  pal.brickFront = wearing(pal.brick, MAT.brick + WINDOWS);
  pal.plasterFront = wearing(pal.plaster, MAT.plaster + WINDOWS);
  return pal;
}
