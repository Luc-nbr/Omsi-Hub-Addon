// The page's icon table, ui/js/symbols.js: Google's Material Symbols Rounded (filled,
// Apache-2.0) by their own names, and our own icons drawn as lines (the livery studio's tools).
// The pages name the icons as Material does (`directions_bus`, `schedule` …); the bus company's
// views send such names too, so every symbol of the folder goes in.
//
//   node tools/symbols.mjs <folder of Material SVGs> [<folder of our stroked SVGs>]
//
// A Material SVG is the 48 px "fill1" one, viewBox 0 -960 960 960
// (https://fonts.gstatic.com/s/i/short-term/release/materialsymbolsrounded/<name>/fill1/48px.svg);
// a stroked one is drawn on a 24 grid with fill="none".

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const out = join(here, 'ui', 'js', 'symbols.js');
const [material, custom] = process.argv.slice(2);
if (!material) {
  console.error('usage: node tools/symbols.mjs <material svg folder> [<stroked svg folder>]');
  process.exit(2);
}

const paths = (svg) => [...svg.matchAll(/<path[^>]*\sd="([^"]+)"/g)].map((m) => m[1]);
const svgs = (dir) => readdirSync(dir).filter((f) => f.endsWith('.svg')).sort();

const symbols = {};
for (const f of svgs(material)) {
  const d = paths(readFileSync(join(material, f), 'utf8')).join(' ');
  if (d) symbols[basename(f, '.svg')] = d;
}
const strokes = {};
if (custom) {
  for (const f of svgs(custom)) {
    const svg = readFileSync(join(custom, f), 'utf8');
    const ds = paths(svg);
    if (ds.length && svg.includes('viewBox="0 0 24 24"') && svg.includes('fill="none"')) strokes[basename(f, '.svg')] = ds;
  }
}

writeFileSync(
  out,
  '// Google\'s Material Symbols Rounded (filled, Apache-2.0) by their own names, viewBox\n' +
    '// 0 -960 960 960; and our own icons drawn as lines (viewBox 0 0 24 24, stroke 1.9, round).\n' +
    '// Made by tools/symbols.mjs: do not edit.\n' +
    'export const SYMBOLS = ' + JSON.stringify(symbols, null, 1) + ';\n' +
    'export const STROKES = ' + JSON.stringify(strokes, null, 1) + ';\n' +
    '/** (the editors\' name for the same) */\n' +
    'export const STROKED = STROKES;\n',
);
console.log(`${Object.keys(symbols).length} symbols, ${Object.keys(strokes).length} stroked -> ${out}`);
