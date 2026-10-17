// A trip's route on the map's roads, as the drive's steps and the company's duty show it beside
// their sheet: the roads in grey, the route in the accent, its stops as dots with their names,
// and the map's entry points to choose from where the step offers them.
//
// The drawing comes from hub-api (`hub_route_map {map, date, trip}`, or `read()` the page
// gives): `{ roads: [{ p: [x, y, x, y …], w, main }], route: [[x, y …]], lo: [x, y],
// hi: [x, y], entries: [{ index, name, at: [x, y] }] }` in the map's metres, y north.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { tr } from './bits.js';

const cache = new Map();

function load(map, date, trip, read) {
  const key = `${map}|${date}|${trip}`;
  if (!cache.has(key)) cache.set(key, (read ? read() : call('hub_route_map', { map, date, trip })).catch((e) => ({ error: e.message })));
  return cache.get(key);
}

/** The box the route takes (or, without one, the whole map), with some room round it. */
function frame(d) {
  let lo = [Infinity, Infinity];
  let hi = [-Infinity, -Infinity];
  const take = (p) => {
    for (let i = 0; i + 1 < p.length; i += 2) {
      lo = [Math.min(lo[0], p[i]), Math.min(lo[1], p[i + 1])];
      hi = [Math.max(hi[0], p[i]), Math.max(hi[1], p[i + 1])];
    }
  };
  (d.route || []).forEach(take);
  if (!isFinite(lo[0])) {
    if (d.lo && d.hi) [lo, hi] = [d.lo, d.hi];
    else (d.roads || []).forEach((r) => take(r.p));
  }
  if (!isFinite(lo[0])) return null;
  const pad = Math.max(hi[0] - lo[0], hi[1] - lo[1], 200) * 0.08;
  return [lo[0] - pad, lo[1] - pad, hi[0] - lo[0] + 2 * pad, hi[1] - lo[1] + 2 * pad];
}

const points = (p) => {
  let s = '';
  for (let i = 0; i + 1 < p.length; i += 2) s += `${p[i]},${-p[i + 1]} `;
  return s;
};

/** `stops`: [{ name, at: [x, y] }]; `entry`/`onEntry`: the entry point chosen and the choice;
 * `colors`: [ground, road, main road, route]; `onView(metres a pixel)` for a scale bar. */
export function RouteMap({ map, date, trip, stops = [], entry = -1, onEntry, height = 340, colors, read, onView }) {
  const [data, setData] = useState(null);
  useEffect(() => {
    if (!map) return undefined;
    let alive = true;
    setData(null);
    load(map, date, trip, read).then((d) => alive && setData(d));
    return () => {
      alive = false;
    };
  }, [map, date, trip]);
  const [ground, road, main, route] = colors || ['transparent', 'var(--hub-rail)', 'var(--hub-track)', 'var(--hub-accent)'];
  const box = data && !data.error ? frame(data) : null;
  const style = `background:${ground};${height ? `height:${height}px` : 'height:100%'}`;
  // (the map's metres a screen unit holds, about: the drawing is 600 units across)
  const unit = box ? Math.max(box[2], box[3]) / 600 : 0;
  useEffect(() => {
    if (unit) onView?.(unit);
  }, [unit]);
  if (!box) {
    return html`<div class="routemap empty" style=${style}><p class="faint">${data?.error ? tr('The map is not drawn yet.') : map ? tr('Reading the map…') : ''}</p></div>`;
  }
  const [x0, y0, w, h] = box;
  // (y north on the map, down on the screen)
  const view = `${x0} ${-(y0 + h)} ${w} ${h}`;
  return html`<div class="routemap" style=${style}>
    <svg viewBox=${view} preserveAspectRatio="xMidYMid meet" width="100%" height="100%">
      ${(data.roads || []).map((r) => html`<polyline points=${points(r.p)} fill="none" stroke=${r.main ? main : road} stroke-width=${Math.max(r.w || 6, unit * 1.5)} stroke-linecap="round" stroke-linejoin="round" />`)}
      ${(data.route || []).map((p) => html`<polyline points=${points(p)} fill="none" stroke=${route} stroke-width=${unit * 4.5} stroke-linecap="round" stroke-linejoin="round" />`)}
      ${(onEntry ? data.entries || [] : []).map((e) => html`<circle cx=${e.at[0]} cy=${-e.at[1]} r=${unit * (e.index === entry ? 7 : 5)} fill=${e.index === entry ? route : 'var(--hub-text-dim)'} style="cursor:pointer" onClick=${() => onEntry(e.index)}><title>${e.name}</title></circle>`)}
      ${stops.filter((s) => s.at).map((s) => html`<g>
        <circle cx=${s.at[0]} cy=${-s.at[1]} r=${unit * 4} fill="var(--hub-panel)" stroke=${route} stroke-width=${unit * 2} />
        <text x=${s.at[0] + unit * 7} y=${-s.at[1] + unit * 4} font-size=${unit * 11} fill="var(--hub-text)">${s.name}</text>
      </g>`)}
    </svg>
  </div>`;
}
