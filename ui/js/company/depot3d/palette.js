// The depot's colours (sRGB, 0 to 1) and its light, for the light look (a bright day) and the
// dark one (the evening: a cooler, dimmer light, the windows, lamps and screens lit). The
// accent is the one chosen in the settings (`--hub-accent`).

const hex = (h) => {
  const v = parseInt(String(h).trim().replace(/^#/, ''), 16);
  return Number.isFinite(v) ? [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255] : [0.16, 0.46, 0.97];
};

/** `#rrggbb` as [r, g, b] from 0 to 1. */
export const rgb01 = hex;

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
  light: { sun: [0.62, 0.68, 0.86], sky: [0.3, 0.34, 0.46], ground: [0.12, 0.12, 0.14], night: 1 },
};

/** The palette of the look now on the page. */
export function palette() {
  const root = document.documentElement;
  const dark = root.dataset.theme !== 'light';
  const accent = getComputedStyle(root).getPropertyValue('--hub-accent');
  return { ...(dark ? EVENING : DAY), accent: hex(accent || '#2a75f7'), dark };
}
