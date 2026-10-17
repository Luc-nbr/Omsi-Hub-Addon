// The mark's parts for the page, ui/js/hub/mark.js, from our brand drawing
// (assets/brand/logo-light.svg, made by tools/brand.mjs): the ring and the line, the bus, the
// two words and the stops, each apart, so the page can colour them by the theme and the opening
// can draw them one after another (ui/js/hub/intro.js).
//
//   node tools/mark.mjs

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const svg = readFileSync(join(here, 'assets', 'brand', 'logo-light.svg'), 'utf8');
const out = join(here, 'ui', 'js', 'hub', 'mark.js');

// (the brand drawing has some air round it: tools/brand.mjs's `pad`)
const PAD = 24;
const [vx, vy, vw, vh] = /viewBox="([^"]+)"/.exec(svg)[1].split(/\s+/).map(Number);
const view = [vx + PAD, vy + PAD, +(vw - 2 * PAD).toFixed(1), vh - 2 * PAD];

const body = svg.slice(svg.indexOf('</title>') + 8, svg.lastIndexOf('</svg>')).trim();
const paths = [...body.matchAll(/<path d="([^"]+)"([^>]*)\/>/g)];
const [route, bus, light, heavy] = paths;
const stroke = Number(/stroke-width="([\d.]+)"/.exec(route[2])[1]);

// the ring's foot and the line's end: "M… A r r 0 1 0 <ringEnd> <foot> L <lineEnd> <foot>"
const r = /A[\d.\s]+ ([\d.]+) ([\d.]+) L([\d.]+) [\d.]+/.exec(route[1]);
const [ringEnd, foot, lineEnd] = [Number(r[1]), Number(r[2]), Number(r[3])];

/** The box of a path, by its end points (enough for the absolute M L H V A C Q Z it is drawn
 * with: the control points stay inside the outline here). */
function bounds(d) {
  const lo = [Infinity, Infinity];
  const hi = [-Infinity, -Infinity];
  let at = [0, 0];
  const take = (x, y) => {
    at = [x, y];
    lo[0] = Math.min(lo[0], x);
    lo[1] = Math.min(lo[1], y);
    hi[0] = Math.max(hi[0], x);
    hi[1] = Math.max(hi[1], y);
  };
  for (const [, c, args] of d.matchAll(/([MLHVACQSTZ])([^MLHVACQSTZ]*)/g)) {
    const n = args.trim().split(/[\s,]+/).filter(Boolean).map(Number);
    if (c === 'H') n.forEach((x) => take(x, at[1]));
    else if (c === 'V') n.forEach((y) => take(at[0], y));
    else if (c === 'A') for (let i = 0; i + 6 < n.length; i += 7) take(n[i + 5], n[i + 6]);
    else for (let i = 0; i + 1 < n.length; i += 2) take(n[i], n[i + 1]);
  }
  return [lo[0], lo[1], hi[0], hi[1]].map((v) => +v.toFixed(2));
}

const busBox = bounds(bus[1]);
const word = (text, d) => [[text, bounds(d), d]];
const words = [word('Omsi-', light[1]), word('Hub', heavy[1])];
const caps = words.map(([[, b]]) => Math.round(b[3] - b[1]));

// the stops: what follows the words, one drawing each - the first as it is, the others moved
// along the line (`<g transform="translate(dx 0)">`)
const rest = body.slice(body.indexOf(heavy[0]) + heavy[0].length);
const stops = [];
const first = rest.indexOf('<g ');
stops.push(rest.slice(0, first < 0 ? rest.length : first));
for (const m of rest.matchAll(/<g transform="translate\(([-\d.]+) 0\)">([\s\S]*?)<\/g>(?=<g |$)/g)) stops.push([Number(m[1]), m[2]]);
const marks = stops.map((s, k) => {
  if (k === 0) {
    const cx = Number(/cx="([\d.]+)"/.exec(s)[1]);
    return [cx, cx, s];
  }
  const [dx, inner] = s;
  const was = Number(/cx="([\d.]+)"/.exec(inner)[1]);
  return [+(was + dx).toFixed(1), was, inner];
});

const MARK = {
  view, route: route[1], stroke, foot, ringEnd, lineEnd,
  bus: bus[1], busAt: [(busBox[0] + busBox[2]) / 2, (busBox[1] + busBox[3]) / 2],
  caps, words, stops: marks,
};
writeFileSync(
  out,
  "// Omsi-Hub's mark in its parts, from assets/brand/logo-light.svg: the view box, the ring and\n" +
    '// the line (one stroke), the bus, the words ("Omsi-" and "Hub": each its text, bounds and\n' +
    '// outline) and the stops (where on the line, where the drawing stands, the drawing). Made by\n' +
    '// tools/mark.mjs: do not edit.\n' +
    'export const MARK = ' + JSON.stringify(MARK) + ';\n',
);
console.log(`mark: ${words.length} words, ${marks.length} stops -> ${out}`);
