// Omsi-Hub's brand files, from the one drawing of the mark (the website's assets/mark.js, itself
// copied from the app's logo.js). Writes assets/brand/*.svg:
//   logo.svg        the mark with the words, for light backgrounds
//   logo-light.svg  the same for dark backgrounds (light words)
//   icon.svg        the ring and the bus on the app's dark tile
//   mark.svg        the ring and the bus alone, no background
//
//   node tools/brand.mjs <path to the website's assets/mark.js>

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const out = join(here, 'assets', 'brand');
const { MARK } = await import(pathToFileURL(process.argv[2] || join(here, '..', 'omsi-hub', 'website', 'assets', 'mark.js')).href);

const BLUE = '#2a75f7';
const ORANGE = '#F58620';
const INK_DARK = '#201e24';
const INK_LIGHT = '#eef1f5';
const TILE = '#0a142e';

const stops = () => MARK.stops.map(([x, was, inner]) => (x === was ? inner : `<g transform="translate(${(x - was).toFixed(1)} 0)">${inner}</g>`)).join('');

function logo(ink, title) {
  const [vx, vy, vw, vh] = MARK.view;
  // (a little air around the drawing, so nothing touches the edge)
  const pad = 24;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vx - pad} ${vy - pad} ${vw + 2 * pad} ${vh + 2 * pad}" role="img" aria-label="${title}">
<title>${title}</title>
<path d="${MARK.route}" fill="none" stroke="${BLUE}" stroke-width="${MARK.stroke}" stroke-linecap="round"/>
<path d="${MARK.bus}" fill="${ink}"/>
<path d="${MARK.words[0].map((l) => l[2]).join('')}" fill="${ink}"/>
<path d="${MARK.words[1].map((l) => l[2]).join('')}" fill="${ORANGE}" stroke="${ORANGE}" stroke-width="7" stroke-linejoin="round"/>
${stops()}
</svg>
`;
}

// the ring alone: its arc from the open end round to the foot, and a short stub of the line
const RING = 'M485.35 362.58 A167 167 0 1 0 334 459 L420 459';
const [cx, cy] = MARK.busAt;
function mark(bg) {
  const s = 560;
  const x = cx - s / 2;
  const y = cy - s / 2 + 18;
  const tile = bg ? `<rect x="${x}" y="${y}" width="${s}" height="${s}" rx="${s * 0.22}" fill="${bg}"/>` : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${x} ${y} ${s} ${s}" role="img" aria-label="Omsi-Hub">
<title>Omsi-Hub</title>
${tile}<path d="${RING}" fill="none" stroke="${BLUE}" stroke-width="${MARK.stroke}" stroke-linecap="round"/>
<path d="${MARK.bus}" fill="${bg ? INK_LIGHT : INK_DARK}"/>
</svg>
`;
}

mkdirSync(out, { recursive: true });
writeFileSync(join(out, 'logo.svg'), logo(INK_DARK, 'Omsi-Hub'));
writeFileSync(join(out, 'logo-light.svg'), logo(INK_LIGHT, 'Omsi-Hub'));
writeFileSync(join(out, 'icon.svg'), mark(TILE));
writeFileSync(join(out, 'mark.svg'), mark(null));
console.log(`brand files in ${out}`);
