// The live dashboard (Luc, 2026-10-19): what the launcher shows while a drive it started runs -
// a shift, a tour, a free drive or a company's duty alike. Six cards: the duty (its route, where
// it goes, what the IBIS is set to, start and how far it has come, the next break), the
// navigator (`navigator.js`; the whole duty's map before the bus moves), the drive's figures (this
// drive, the week, the month), the next stops with when the bus gets there, the alerts, and
// quick actions. It stands in place of the launcher's pages until the driver goes back to them
// (a pill in the corner brings it back) and while the window rests in the background.
//
// The view comes from hub-api (`hub_live`, `live.rs`), asked every second (every few while the
// window is in the background): the plugin's `dash.json` of the game, the duty's plan from the
// map's timetable, the driver's trip records. It says what is so; this words it.

import { html, useEffect, useLayoutEffect, useMemo, useRef, useState } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { S, set, useStore } from '../store.js';
import { Icon, tr } from './bits.js';
import { SYMBOLS } from '../symbols.js';
import { dark } from './theme.js';
import { flipLook } from './bar.js';
import { Navigator } from './navigator.js';

let cssLoaded = false;
function loadCss() {
  if (cssLoaded) return;
  cssLoaded = true;
  const sheet = document.createElement('link');
  sheet.rel = 'stylesheet';
  sheet.href = 'css/hub/dash.css';
  document.head.append(sheet);
}

// --- the words of numbers --------------------------------------------------------------------

const lang = () => S.lang || 'en';
const imperial = () => S.settings?.units === 'imperial';
const fmt = (n, places = 0) => new Intl.NumberFormat(lang(), { minimumFractionDigits: places, maximumFractionDigits: places }).format(n);

/** Seconds of the day as `08:24`. */
export function hhmm(secs) {
  if (secs == null || !isFinite(secs)) return '--:--';
  const m = ((Math.floor(secs / 60) % 1440) + 1440) % 1440;
  return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
}

/** A while: `38 min`, `4 h 15 min`. */
export function duration(secs) {
  const m = Math.max(0, Math.round((secs || 0) / 60));
  if (m < 60) return tr('%{m} min', { m });
  const h = Math.floor(m / 60);
  return m % 60 ? tr('%{h} h %{m} min', { h, m: m % 60 }) : tr('%{h} h', { h });
}

/** Seconds from the clock `now` to `t` (both of the day, a later day's above 86 400): a time
 * just after midnight is the next day's, one up to twelve hours before is past (negative). */
export function ahead(t, now) {
  const d = ((((t - now) % 86_400) + 86_400) % 86_400);
  return d > 43_200 ? d - 86_400 : d;
}

/** A distance and its unit: [`1.2`, `km`]. */
function distance(metres) {
  if (imperial()) {
    const mi = metres / 1609.344;
    // (feet to the ten: the number formatted as a whole, not a zero put after it)
    return mi < 0.2 ? [fmt(Math.round((metres * 3.28084) / 10) * 10), 'ft'] : [fmt(mi, mi < 10 ? 1 : 0), 'mi'];
  }
  return metres < 1000 ? [fmt(Math.round(metres / 10) * 10), 'm'] : [fmt(metres / 1000, metres < 100_000 ? 1 : 0), 'km'];
}
const distanceText = (m) => distance(m).join(' ');
/** The words of numbers the navigator says. */
const WORDS = { distance: (m) => distanceText(m), hhmm: (s) => hhmm(s), delay: (d) => delayText(d) };
const speedOf = (kmh) => (imperial() ? [fmt(kmh / 1.609344), 'mph'] : [fmt(kmh), 'km/h']);

/** How late (`+2 min`), early (`−1 min`) or on time. */
function delayText(d) {
  if (d == null) return '';
  const m = Math.round(Math.abs(d) / 60);
  if (m === 0) return tr('on time');
  return d > 0 ? tr('+%{m} min late', { m }) : tr('−%{m} min early', { m });
}

// --- the view, asked again and again ---------------------------------------------------------

/** `hub_live` every second while the dashboard is drawn (every few in the background). */
function useLive() {
  useEffect(() => {
    let alive = true;
    let timer = 0;
    // (an answer like the last changes nothing: not the whole page drawn again for it)
    let last = '';
    const tick = async () => {
      const v = await call('hub_live').catch(() => null);
      if (!alive) return;
      const text = v ? JSON.stringify(v) : '';
      if (v && text !== last) ((last = text), set({ live: v }));
      // (the game ended without the page hearing it: back to the launcher)
      if (v && v.running === false && S.game?.running) set({ game: { ...S.game, running: false } });
      timer = setTimeout(tick, document.hasFocus() ? 1000 : 3000);
    };
    tick();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, []);
}

/** Whether the dashboard is the window's: a game runs and the driver did not go back. */
export function dashShown(s) {
  return !!s.game?.running && !s.liveDash?.hidden;
}

export function showDash(on = true) {
  set({ liveDash: { ...(S.liveDash || {}), hidden: !on, from: null } });
}

// --- the cards -------------------------------------------------------------------------------

function Card({ icon, title, sub, tools, cls = '', children }) {
  return html`<section class=${'dash-card ' + cls}>
    <header><${Icon} name=${icon} size=${22} class="dash-ic" /><div class="t"><h3>${title}</h3>${sub && html`<p>${sub}</p>`}</div>${tools && html`<div class="tools">${tools}</div>`}</header>
    <div class="dash-body">${children}</div>
  </section>`;
}

const STATUS = {
  in_service: ['In service', 'ok'],
  at_stop: ['At a stop', 'ok'],
  layover: ['Layover', 'warn'],
  waiting: ['Not started yet', 'dim'],
  starting: ['The game is starting…', 'dim'],
  done: ['Duty done', 'early'],
  free: ['Free drive', 'early'],
};

function modeLine(v) {
  if (v.mode === 'company') return [v.duty && tr('Duty %{duty}', { duty: v.duty }), v.company].filter(Boolean).join(' · ');
  if (v.mode === 'shift') return tr('Your shift');
  if (v.mode === 'tour') return tr('Line %{line}, tour %{tour}', { line: v.route?.line || '', tour: v.route?.tour || '' });
  return tr('A drive of your own');
}

const pad = (n, k) => (n == null ? '' : String(n).padStart(k, '0'));

/** What the driver types into the IBIS for the trip (`v.ibis`, from the bus's depot file): the
 * keys and their numbers, what the IBIS then shows, and whether the bus's IBIS is set so. */
export function IbisPanel({ v, full = false }) {
  const i = v.ibis;
  if (!i || (!i.trip && !i.codes)) return null;
  const c = i.codes;
  // (a free drive's route has no departure: it is the trip there is)
  const when = i.for === 'now' || i.departure == null ? tr('This trip') : i.for === 'next' ? tr('Next trip, %{time}', { time: hhmm(i.departure) }) : tr('First trip, %{time}', { time: hhmm(i.departure) });
  const chip = i.matches === true ? html`<span class="dash-chip ok"><${Icon} name="check_circle" size=${13} />${tr('IBIS set')}</span>` : i.matches === false ? html`<span class="dash-chip warn"><${Icon} name="warning" size=${13} />${tr('Check the IBIS')}</span>` : null;
  let body;
  if (!c) {
    const why = !i.hof ? tr('The bus names no depot file, so its IBIS knows no destination.') : !i.file ? tr('The bus has no depot file %{hof}: its IBIS knows no destination.', { hof: i.hof }) : tr('The depot file %{hof} has no destination for this trip.', { hof: i.hof });
    body = html`<p class="ibis-none">${why}</p>`;
  } else {
    const key = (label, value, note) => html`<div class="ibis-key"><span>${label}</span><b>${value}</b>${note && html`<small>${note}</small>`}</div>`;
    body = html`<div class="ibis-row">
      ${key(tr('Linie/Kurs'), c.line ?? '–', i.kurs ? tr('Kurs %{kurs}', { kurs: pad(i.kurs, 2) }) : '')}
      ${c.route != null ? key(tr('Route'), pad(c.route, 2), c.route_code) : key(tr('Ziel'), pad(c.terminus ?? c.terminus_code, 3), '')}
      <div class="ibis-lcd" title=${c.route_name || ''}><b class="cut">${c.display || c.terminus_name}</b><small>${tr('Ziel %{code}', { code: pad(c.terminus_code, 3) })}${i.stop != null ? ` · ${tr('stop %{n}', { n: i.stop + 1 })}` : ''}</small></div>
    </div>`;
  }
  const set = i.set;
  const shows = i.matches === false && set ? tr('The IBIS shows line %{line}, destination %{dest}.', { line: set.line != null ? Math.round(set.line) : '–', dest: set.terminus != null ? pad(Math.round(set.terminus), 3) : '–' }) : '';
  return html`<div class=${'dash-ibis' + (full ? ' full' : '')}>
    <div class="ibis-head"><b>IBIS</b><span class="cut">${[when, i.hof && i.hof + '.hof'].filter(Boolean).join(' · ')}</span>${chip}</div>
    ${body}
    ${shows && html`<p class="ibis-none warn">${shows}</p>`}
    ${full && c && (c.front || []).length > 0 && html`<div class="ibis-front">${c.front.map((r) => html`<span>${r}</span>`)}</div>`}
    ${full && i.next?.codes && html`<p class="ibis-after">${tr('Next trip %{time}', { time: hhmm(i.next.departure) })}: ${tr('Linie/Kurs')} <b>${i.next.codes.line ?? '–'}</b>${i.next.codes.route != null ? html` · ${tr('Route')} <b>${pad(i.next.codes.route, 2)}</b>` : html` · ${tr('Ziel')} <b>${pad(i.next.codes.terminus ?? i.next.codes.terminus_code, 3)}</b>`} · ${i.next.codes.display}</p>`}
  </div>`;
}

function DutyCard({ v }) {
  const [word, tone] = STATUS[v.status] || STATUS.waiting;
  const r = v.route || {};
  const free = v.mode === 'free';
  const total = v.start != null && v.end != null ? Math.max(v.end - v.start, 60) : null;
  const elapsed = free ? v.today?.seconds || 0 : v.elapsed || 0;
  const share = total ? Math.min(1, elapsed / total) : 0;
  const b = v.next_break;
  const start = free ? v.series?.[0]?.[0] : v.start;
  const trip = v.plan?.[(r.trip || 1) - 1];
  const to = r.to || trip?.terminus || v.end_place || '';
  const route = r.number || (free ? '–' : r.line || '–');
  const sub = [modeLine(v), v.vehicle?.number, v.vehicle?.name].filter(Boolean).join(' · ');
  return html`<${Card} icon="calendar_month" title=${tr('Duty summary')} sub=${sub} cls="duty"
    tools=${html`<span class=${'dash-chip ' + tone}>${tr(word)}</span>`}>
    <div class="dash-facts">
      <div><span>${tr('Route')}</span><b class=${'big cut' + (route.length > 6 ? ' long' : '')} title=${route}>${route}</b><small class="cut">${free ? r.free_route || tr('Free drive') : r.from ? tr('from %{place}', { place: r.from }) : ''}</small></div>
      <div><span>${tr('Destination')}</span><b class="mid cut" title=${to}>${to || '–'}</b><small class="cut">${r.trips > 0 && r.trip ? tr('Trip %{n} of %{of}', { n: r.trip, of: r.trips }) : ''}</small></div>
      <div><span>${tr('Start time')}</span><b class="big">${hhmm(start)}</b><small class="cut">${v.start_place || ''}</small></div>
    </div>
    <${IbisPanel} v=${v} />
    <div class="dash-progress">
      <div class="row"><span>${free ? tr('Driving for') : tr('Time elapsed')}</span><b>${total ? `${duration(elapsed)} / ${duration(total)}` : duration(elapsed)}</b></div>
      ${total && html`<div class="bar" role="progressbar" aria-valuenow=${Math.round(share * 100)} aria-valuemin="0" aria-valuemax="100"><i style=${`width:${share * 100}%`}></i></div>`}
      ${v.end != null && html`<div class="row faint"><span>${b ? `${b.kind === 'layover' ? tr('Next layover') : tr('Next break')} ${hhmm(b.from)}${v.clock != null ? ` (${ahead(b.from, v.clock) > 0 ? tr('in %{t}', { t: duration(ahead(b.from, v.clock)) }) : tr('now')})` : ''}` : tr('No break planned')}</span><span>${tr('Ends %{time} at %{place}', { time: hhmm(v.end), place: v.end_place || '' })}</span></div>`}
    </div>
  </${Card}>`;
}

/** The size an element is drawn at (CSS px of the stage). */
function useSize(ref) {
  const [size, setSize] = useState([0, 0]);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return undefined;
    const ro = new ResizeObserver(() => setSize([el.clientWidth, el.clientHeight]));
    ro.observe(el);
    setSize([el.clientWidth, el.clientHeight]);
    return () => ro.disconnect();
  }, []);
  return size;
}

/** The map's roads and the duty's routes under the bus (`hub_route_map`, as the drive's steps
 * draw them): read once a map, day and duty, and kept while the next one is read (the plan
 * comes a moment after the drive began). Its points are metres from its `origin`. */
const drawn = new Map();
function useRoads(v) {
  const map = v.map?.map || '';
  // (a free drive along a line has no plan: the route it follows is its trip)
  const names = (v.plan || []).map((p) => p.name);
  if (!names.length && v.route?.free_route) names.push(v.route.free_route);
  const trips = [...new Set(names.filter(Boolean))].join('|');
  const key = `${map}|${v.date || ''}|${trips}`;
  const [roads, setRoads] = useState(() => drawn.get(key)?.done || null);
  useEffect(() => {
    if (!map) return undefined;
    let alive = true;
    if (!drawn.has(key)) {
      // (`a|b|`: each trip by its name, one trip too - the navigator follows the one under way)
      const e = { read: call('hub_route_map', { map, date: v.date || '', trip: `${trips}|` }).catch(() => null) };
      // (one that could not be read is read again the next time it is asked for)
      e.read.then((r) => (r ? (e.done = r) : drawn.delete(key)));
      drawn.set(key, e);
    }
    drawn.get(key).read.then((r) => alive && r && setRoads(r));
    return () => {
      alive = false;
    };
  }, [key]);
  return roads;
}

/** The roads and the routes, drawn again only when the map or the scale changes (`u`: metres
 * a pixel, in steps). */
function Roads({ g, u }) {
  return html`<g>
    ${roadPaths(g).map((e) => html`<path d=${e.d} fill="none" stroke=${e.main ? 'var(--dash-road-main)' : 'var(--dash-road)'} stroke-width=${Math.max(e.w, u * 2)} stroke-linecap="round" stroke-linejoin="round" />`)}
    ${(g.route || []).map((p) => html`<polyline points=${flat(p)} fill="none" stroke="var(--hub-accent)" stroke-opacity="0.35" stroke-width=${u * 6} stroke-linecap="round" stroke-linejoin="round" />`)}
    ${(g.stops || []).filter((s) => s.at).map((s) => html`<circle cx=${s.at[0]} cy=${-s.at[1]} r=${u * 3.5} fill="var(--hub-panel)" stroke="var(--hub-accent)" stroke-opacity="0.6" stroke-width=${u * 1.5} />`)}
  </g>`;
}

/** The roads as a few paths, one a kind and width of road (`[{ main, w, d }]`): thousands of
 * lines of their own made the map slow to draw. Kept a map read. */
const paths = new WeakMap();
function roadPaths(g) {
  let out = paths.get(g);
  if (out) return out;
  const by = new Map();
  for (const r of g.roads || []) {
    const w = r.w || 6;
    const key = (r.main ? 'm' : 's') + w;
    let e = by.get(key);
    if (!e) by.set(key, (e = { main: !!r.main, w, parts: [] }));
    if (r.p.length >= 4) e.parts.push('M' + flat(r.p));
  }
  // (the side streets first, the main roads over them)
  out = [...by.values()].sort((a, b) => a.main - b.main).map((e) => ({ main: e.main, w: e.w, d: e.parts.join('') }));
  paths.set(g, out);
  return out;
}

/** `[x, y, x, y …]` (y north) as an SVG polyline's points. */
function flat(p) {
  let s = '';
  for (let i = 0; i + 1 < p.length; i += 2) s += `${p[i]},${-p[i + 1]} `;
  return s;
}

/** The bus on the map: the map's roads and the duty's route, the way the bus went, where it
 * stood at its stops, where it is and where it heads. `zoom` 1 shows all of the route; closer,
 * the bus in the middle. */
export function LiveMap({ v, big = false }) {
  const ref = useRef(null);
  const [w, h] = useSize(ref);
  const [zoom, setZoom] = useState(1);
  const m = v.map || {};
  const g = useRoads(v);
  // (the game's world metres; the drawing's are from its origin)
  const o = g?.origin || [0, 0];
  const rel = (p) => [p[0] - o[0], p[1] - o[1]];
  const trail = (m.trail || []).map(rel);
  const bus = m.bus && rel(m.bus);
  const stood = (m.stood || []).map((s) => ({ ...s, at: rel(s.at) }));
  const route = [];
  for (const p of g?.route || []) for (let i = 0; i + 1 < p.length; i += 2) route.push([p[i], p[i + 1]]);
  let pts = [...route, ...trail, ...(bus ? [bus] : []), ...stood.map((s) => s.at)];
  if (!pts.length && g?.lo && g?.hi && (g.roads || []).length) pts = [g.lo, g.hi];
  const step = (x) => 2 ** (Math.round(Math.log2(x) * 2) / 2);
  const fitOf = (lo, hi) => Math.max((hi[0] - lo[0]) / Math.max(w, 1), (hi[1] - lo[1]) / Math.max(h, 1), 300 / Math.max(w, 1)) * 1.25;
  let lo = [Infinity, Infinity];
  let hi = [-Infinity, -Infinity];
  for (const p of pts) {
    lo = [Math.min(lo[0], p[0]), Math.min(lo[1], p[1])];
    hi = [Math.max(hi[0], p[0]), Math.max(hi[1], p[1])];
  }
  const uq = pts.length && w ? step(fitOf(lo, hi) / zoom) : 1;
  const roads = useMemo(() => g && html`<${Roads} g=${g} u=${uq} />`, [g, uq]);
  const tools = html`<div class="dash-maptools">
    <button type="button" title=${tr('Closer')} onClick=${() => setZoom((z) => Math.min(z * 1.6, 24))}><${Icon} name="add" size=${18} /></button>
    <button type="button" title=${tr('Further')} onClick=${() => setZoom((z) => Math.max(z / 1.6, 1))}><${Icon} name="remove" size=${18} /></button>
    <button type="button" title=${tr('The whole way')} class=${zoom === 1 ? 'on' : ''} onClick=${() => setZoom(1)}><${Icon} name="my_location" size=${18} /></button>
  </div>`;
  if (!pts.length || !w) {
    return html`<div class="dash-map empty" ref=${ref}><p>${!v.live ? tr('The map follows the bus once you drive.') : tr('Where the bus is shows here once it moves.')}</p></div>`;
  }
  // (metres a pixel; the middle of the view)
  const mpp = fitOf(lo, hi) / zoom;
  const c = zoom > 1 && bus ? bus : [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2];
  const vw = w * mpp;
  const vh = h * mpp;
  const view = `${c[0] - vw / 2} ${-c[1] - vh / 2} ${vw} ${vh}`;
  const line = trail.map((p) => `${p[0]},${-p[1]}`).join(' ');
  // where the bus heads: from the way's last bends
  let heading = m.heading;
  if (heading == null && trail.length > 1) {
    const l = trail[trail.length - 1];
    const p = [...trail].reverse().find((q) => Math.hypot(l[0] - q[0], l[1] - q[1]) >= 8);
    if (p) heading = (Math.atan2(l[0] - p[0], l[1] - p[1]) * 180) / Math.PI;
  }
  const grid = 100 * 2 ** Math.max(0, Math.round(Math.log2((mpp * 60) / 100)));
  const u = mpp;
  const labels = big || stood.length <= 6;
  return html`<div class="dash-map" ref=${ref}>
    <svg viewBox=${view} width=${w} height=${h} aria-label=${tr('The bus on the map')}>
      <defs><pattern id=${big ? 'dash-grid-big' : 'dash-grid'} width=${grid} height=${grid} patternUnits="userSpaceOnUse" x=${0} y=${0}><path d=${`M ${grid} 0 L 0 0 0 ${grid}`} fill="none" stroke="var(--dash-grid)" stroke-width=${u} /></pattern></defs>
      <rect x=${c[0] - vw} y=${-c[1] - vh} width=${vw * 2} height=${vh * 2} fill=${`url(#${big ? 'dash-grid-big' : 'dash-grid'})`} />
      ${roads}
      ${trail.length > 1 && html`<polyline points=${line} fill="none" stroke="var(--hub-accent)" stroke-opacity="0.3" stroke-width=${u * 9} stroke-linecap="round" stroke-linejoin="round" />
        <polyline points=${line} fill="none" stroke="var(--hub-accent)" stroke-width=${u * 3.5} stroke-linecap="round" stroke-linejoin="round" />`}
      ${trail.length > 1 && html`<circle cx=${trail[0][0]} cy=${-trail[0][1]} r=${u * 6} fill="var(--hub-ok)" stroke="var(--hub-panel)" stroke-width=${u * 2.5} />`}
      ${stood.map((s) => html`<g>
        <circle cx=${s.at[0]} cy=${-s.at[1]} r=${u * 4.5} fill="var(--hub-panel)" stroke="var(--hub-accent)" stroke-width=${u * 2.5} />
        ${labels && html`<text x=${s.at[0] + u * 9} y=${-s.at[1] + u * 4} font-size=${u * 11} class="dash-maplabel" stroke-width=${u * 3}>${s.name}</text>`}
      </g>`)}
      ${bus && html`<g transform=${`translate(${bus[0]} ${-bus[1]})`}>
        <circle r=${u * 16} fill="var(--hub-accent)" opacity="0.18" class="dash-pulse" />
        ${heading != null && html`<path d=${`M 0 ${-u * 17} L ${u * 7} ${-u * 8} L ${-u * 7} ${-u * 8} Z`} fill="var(--hub-accent)" transform=${`rotate(${heading})`} />`}
        <circle r=${u * 10} fill="var(--hub-accent)" stroke="var(--hub-panel)" stroke-width=${u * 3} />
        <svg x=${-u * 7} y=${-u * 7} width=${u * 14} height=${u * 14} viewBox="0 -960 960 960"><path d=${SYMBOLS.directions_bus} fill="var(--hub-on-accent)" /></svg>
      </g>`}
    </svg>
    ${tools}
  </div>`;
}

/** The navigator once the bus is on the map, else (or asked for) the whole duty's routes. */
function NavOrMap({ v, big = false }) {
  const g = useRoads(v);
  const [overview, setOverview] = useState(false);
  if (v.map?.bus && !overview) return html`<${Navigator} v=${v} g=${g} words=${WORDS} big=${big} onOverview=${() => setOverview(true)} />`;
  return html`<div class="dash-mapwrap">
    <${LiveMap} v=${v} big=${big} />
    ${v.map?.bus && html`<button type="button" class="hub-btn dash-backnav" onClick=${() => setOverview(false)}><${Icon} name="navigation" size=${16} />${tr('Back to the navigator')}</button>`}
  </div>`;
}

function MapCard({ v, onOpen }) {
  const live = !!v.map?.bus;
  const sub = live ? (v.route?.to ? tr('Route %{route} to %{place}', { route: v.route.number || v.route.line || '', place: v.route.to }) : tr('Following the bus')) : tr('The duty’s routes; it follows the bus once you drive');
  return html`<${Card} icon=${live ? 'navigation' : 'map'} title=${live ? tr('Navigator') : tr('Route map')} sub=${sub} cls="map"
    tools=${html`<button type="button" class="dash-iconbtn" title=${tr('Open the navigator')} onClick=${onOpen}><${Icon} name="fullscreen" size=${18} /></button>`}>
    <${NavOrMap} v=${v} />
  </${Card}>`;
}

/** The big navigator: the map, and beside it the trip's IBIS and its stops ahead. */
function NavDialog({ v, onClose }) {
  const ahead = (v.trip_stops || []).filter((s) => s.stops && !s.passed);
  const late = (v.delay || 0) >= 60;
  return html`<${Dialog} icon="navigation" title=${tr('Navigator')} onClose=${onClose} wide>
    <div class="dash-navbig">
      <div class="map"><${NavOrMap} v=${v} big /></div>
      <aside>
        <${IbisPanel} v=${v} full />
        <h4 class="dash-h4">${v.route?.to ? tr('Stops to %{place}', { place: v.route.to }) : tr('Next stops')}</h4>
        ${ahead.length
          ? html`<ol class="nav-stops">${ahead.map((s, i) => html`<li class=${i === 0 ? 'first' : ''}>
              <b class="cut">${s.name}</b>
              <span>${s.metres != null ? distanceText(s.metres) : ''}</span>
              <span class="t">${late ? html`<s>${hhmm(s.due)}</s> ` : ''}${hhmm(s.eta)}</span>
            </li>`)}</ol>`
          : html`<p class="dash-empty">${v.mode === 'free' ? tr('A free drive has no timetable to follow.') : tr('The stops show here once the duty has started.')}</p>`}
      </aside>
    </div>
  </${Dialog}>`;
}

// --- the figures -----------------------------------------------------------------------------

/** A change against the period before: the arrow and the words. */
function Delta({ now, before, points = false }) {
  if (now == null || before == null || (!points && !before)) return html`<span class="dash-delta none"></span>`;
  const d = points ? now - before : ((now - before) / before) * 100;
  const r = Math.round(d);
  if (!isFinite(d)) return html`<span class="dash-delta none"></span>`;
  const word = points ? tr('%{n} pts', { n: Math.abs(r) }) : `${Math.abs(r)}%`;
  const cls = r > 0 ? 'up' : r < 0 ? 'down' : 'same';
  return html`<span class=${'dash-delta ' + cls}>${r !== 0 && html`<${Icon} name="arrow_upward" size=${13} />`}${r === 0 ? tr('same') : word}</span>`;
}

function Tile({ label, value, unit, delta, note }) {
  return html`<div class="dash-tile"><span>${label}</span><b>${value}${unit && html`<small>${unit}</small>`}</b>${delta || html`<span class="dash-delta none">${note || ''}</span>`}</div>`;
}

/** The speed over the drive (game clock across), with a crosshair and its numbers. */
function SpeedChart({ series }) {
  const ref = useRef(null);
  const [w, h] = useSize(ref);
  const [hover, setHover] = useState(null);
  // (a drive over midnight: the clock after it goes on above 24:00, not back to the left edge)
  let day = 0;
  const pts = [];
  for (const p of series || []) {
    if (p[0] == null) continue;
    const prev = pts[pts.length - 1];
    if (prev && p[0] + day < prev[0] - 43_200) day += 86_400;
    pts.push([p[0] + day, ...p.slice(1)]);
  }
  if (pts.length < 2 || !w) return html`<div class="dash-chart empty" ref=${ref}><p>${tr('The speed over the drive shows here as you drive.')}</p></div>`;
  const pad = [30, 8, 8, 22];
  const x0 = pts[0][0];
  const x1 = Math.max(pts[pts.length - 1][0], x0 + 60);
  const top = Math.max(40, Math.ceil(Math.max(...pts.map((p) => p[1])) / 20) * 20);
  const X = (t) => pad[0] + ((t - x0) / (x1 - x0)) * (w - pad[0] - pad[1]);
  const Y = (s) => pad[2] + (1 - s / top) * (h - pad[2] - pad[3]);
  const line = pts.map((p) => `${X(p[0]).toFixed(1)},${Y(p[1]).toFixed(1)}`).join(' ');
  const area = `M ${X(pts[0][0])},${Y(0)} L ${line.split(' ').join(' L ')} L ${X(pts[pts.length - 1][0])},${Y(0)} Z`;
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((k) => x0 + k * (x1 - x0));
  const ys = [0, top / 4, top / 2, (3 * top) / 4, top];
  const move = (e) => {
    const r = e.currentTarget.getBoundingClientRect();
    const scale = r.width / w;
    const x = (e.clientX - r.left) / scale;
    let best = pts[0];
    for (const p of pts) if (Math.abs(X(p[0]) - x) < Math.abs(X(best[0]) - x)) best = p;
    setHover(best);
  };
  const last = pts[pts.length - 1];
  const hp = hover || null;
  return html`<div class="dash-chart" ref=${ref}>
    <svg width=${w} height=${h} onMouseMove=${move} onMouseLeave=${() => setHover(null)} role="img" aria-label=${tr('Speed over the drive')}>
      <defs><linearGradient id="dash-area" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="var(--hub-accent)" stop-opacity="0.35" /><stop offset="1" stop-color="var(--hub-accent)" stop-opacity="0.02" /></linearGradient></defs>
      ${ys.map((s) => html`<line x1=${pad[0]} x2=${w - pad[1]} y1=${Y(s)} y2=${Y(s)} class="grid" /><text x=${pad[0] - 6} y=${Y(s) + 3.5} class="axis" text-anchor="end">${fmt(imperial() ? s / 1.609344 : s)}</text>`)}
      ${ticks.map((t, i) => html`<text x=${X(t)} y=${h - 6} class="axis" text-anchor=${i === 0 ? 'start' : i === 4 ? 'end' : 'middle'}>${hhmm(t)}</text>`)}
      <path d=${area} fill="url(#dash-area)" />
      <polyline points=${line} fill="none" stroke="var(--hub-accent)" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
      <circle cx=${X(last[0])} cy=${Y(last[1])} r="4" fill="var(--hub-accent)" stroke="var(--hub-panel)" stroke-width="2" />
      ${hp && html`<line x1=${X(hp[0])} x2=${X(hp[0])} y1=${pad[2]} y2=${h - pad[3]} class="cross" /><circle cx=${X(hp[0])} cy=${Y(hp[1])} r="4.5" fill="var(--hub-accent)" stroke="var(--hub-panel)" stroke-width="2" />`}
    </svg>
    ${hp && html`<div class="dash-tip" style=${`left:${Math.min(Math.max(X(hp[0]), 70), w - 70)}px`}><b>${hhmm(hp[0])}</b><span>${speedOf(hp[1]).join(' ')}</span>${hp[2] != null && html`<span>${delayText(hp[2])}</span>`}${hp[3] != null && html`<span>${tr('%{n} on board', { n: Math.round(hp[3]) })}</span>`}</div>`}
  </div>`;
}

/** The distance of each day of the week or month. */
function DaysChart({ days }) {
  const ref = useRef(null);
  const [w, h] = useSize(ref);
  const [hover, setHover] = useState(null);
  if (!w) return html`<div class="dash-chart" ref=${ref}></div>`;
  const pad = [30, 8, 8, 22];
  const n = days.length;
  const most = Math.max(10, Math.ceil(Math.max(...days, 0) / 10) * 10);
  const slot = (w - pad[0] - pad[1]) / n;
  const bw = Math.max(3, Math.min(28, slot - 2));
  const Y = (k) => pad[2] + (1 - k / most) * (h - pad[2] - pad[3]);
  const day = (i) => {
    const d = new Date(Date.now() - (n - 1 - i) * 86_400_000);
    return d.toLocaleDateString(lang(), n > 7 ? { day: 'numeric', month: 'short' } : { weekday: 'short' });
  };
  const ys = [0, most / 2, most];
  return html`<div class="dash-chart" ref=${ref}>
    <svg width=${w} height=${h} role="img" aria-label=${tr('Distance a day')} onMouseLeave=${() => setHover(null)}>
      ${ys.map((k) => html`<line x1=${pad[0]} x2=${w - pad[1]} y1=${Y(k)} y2=${Y(k)} class="grid" /><text x=${pad[0] - 6} y=${Y(k) + 3.5} class="axis" text-anchor="end">${fmt(imperial() ? k / 1.609344 : k)}</text>`)}
      ${days.map((k, i) => {
        const x = pad[0] + i * slot + (slot - bw) / 2;
        const y = Y(k);
        const r = Math.min(4, bw / 2, (Y(0) - y) / 2);
        const d = k > 0 ? `M ${x},${Y(0)} V ${y + r} Q ${x},${y} ${x + r},${y} H ${x + bw - r} Q ${x + bw},${y} ${x + bw},${y + r} V ${Y(0)} Z` : '';
        return html`<g onMouseEnter=${() => setHover(i)}>
          <rect x=${pad[0] + i * slot} y=${pad[2]} width=${slot} height=${h - pad[2] - pad[3]} fill="transparent" />
          ${d && html`<path d=${d} fill="var(--hub-accent)" opacity=${hover == null || hover === i ? 1 : 0.55} />`}
          ${(n <= 7 || (n - 1 - i) % 5 === 0) && html`<text x=${x + bw / 2} y=${h - 6} class="axis" text-anchor="middle">${day(i)}</text>`}
        </g>`;
      })}
    </svg>
    ${hover != null && html`<div class="dash-tip" style=${`left:${Math.min(Math.max(pad[0] + (hover + 0.5) * slot, 60), w - 60)}px`}><b>${day(hover)}</b><span>${distanceText(days[hover] * 1000)}</span></div>`}
  </div>`;
}

const KEPT_TAB = 'omsihub.dash.tab';

function StatsCard({ v }) {
  const [tab, setTab] = useState(() => {
    try {
      return localStorage.getItem(KEPT_TAB) || 'today';
    } catch {
      return 'today';
    }
  });
  const pick = (t) => {
    setTab(t);
    try {
      localStorage.setItem(KEPT_TAB, t);
    } catch {
      // (a private window: this session only)
    }
  };
  const p = tab === 'week' ? v.week : tab === 'month' ? v.month : null;
  const now = p ? p.now : v.today || {};
  const before = p ? p.before : null;
  const month = v.month?.now || {};
  const [dv, du] = distance(now.metres || 0);
  const [sv, su] = now.avg_speed != null ? speedOf(now.avg_speed) : ['–', ''];
  const sub = tab === 'today' ? tr('This drive') : tab === 'week' ? tr('The last 7 days') : tr('The last 30 days');
  const tabs = html`<div class="dash-tabs" role="tablist">${[['week', '7D'], ['month', '30D'], ['today', tr('Today')]].map(([id, label]) => html`<button type="button" role="tab" aria-selected=${tab === id} class=${tab === id ? 'on' : ''} onClick=${() => pick(id)}>${label}</button>`)}</div>`;
  return html`<${Card} icon="trending_up" title=${tr('Statistics')} sub=${sub} tools=${tabs} cls="stats">
    ${p ? html`<${DaysChart} days=${p.days || []} />` : html`<${SpeedChart} series=${v.series} />`}
    <div class="dash-tiles">
      <${Tile} label=${tr('Distance')} value=${dv} unit=${du} delta=${p && html`<${Delta} now=${now.metres} before=${before?.metres} />`} />
      <${Tile} label=${tr('On time')} value=${now.on_time != null ? `${fmt(now.on_time)}%` : '–'} delta=${html`<${Delta} now=${now.on_time} before=${p ? before?.on_time : month.on_time} points />`} />
      <${Tile} label=${tr('Avg. speed')} value=${sv} unit=${su} delta=${html`<${Delta} now=${now.avg_speed} before=${p ? before?.avg_speed : month.avg_speed} />`} />
      <${Tile} label=${tr('Passengers')} value=${fmt(now.boarded || 0)} delta=${p && html`<${Delta} now=${now.boarded} before=${before?.boarded} />`} note=${!p && v.passengers != null ? tr('%{n} on board', { n: Math.round(v.passengers) }) : ''} />
    </div>
  </${Card}>`;
}

// --- the stops, the alerts, the actions --------------------------------------------------------

function StopsCard({ v }) {
  const stops = v.stops || [];
  const late = (v.delay || 0) >= 60;
  const sub = v.route?.number ? tr('Upcoming stops on route %{route}', { route: v.route.number }) : tr('Upcoming stops');
  let body;
  if (!stops.length) {
    // (the next trip: the first of the plan to leave after the clock, midnight between them or not)
    const nextTrip = v.clock != null ? v.plan?.find((p) => ahead(p.departure, v.clock) > 0) : null;
    const why = v.mode === 'free' ? tr('A free drive has no timetable to follow.') : v.status === 'layover' ? tr('The trip is done: the next one starts at %{time}.', { time: hhmm(nextTrip?.departure) }) : v.status === 'done' ? tr('Every trip of the duty is driven.') : tr('The stops show here once the duty has started.');
    body = html`<p class="dash-empty">${why}</p>`;
  } else {
    body = html`<ol class="dash-stops">${stops.map((s, i) => {
      const now = i === 0 && v.status === 'at_stop';
      const m = s.minutes;
      const right = now ? tr('Here') : m == null ? '' : m < 1 ? tr('due') : tr('%{m} min', { m: Math.round(m) });
      const when = s.index === 0 ? tr('Departs %{time}', { time: hhmm(s.eta) }) : tr('Arrive %{time}', { time: hhmm(s.eta) });
      return html`<li class=${i === 0 ? 'first' : ''}>
        <i></i>
        <div class="what"><b class="cut">${s.name}</b><span>${when}${s.metres != null ? ` · ${distanceText(s.metres)}` : ''}${late ? html` · <s>${hhmm(s.due)}</s>` : ''}</span></div>
        <span class="when">${right}</span>
      </li>`;
    })}</ol>`;
  }
  return html`<${Card} icon="location_on" title=${tr('Next stops')} sub=${sub} cls="stops">${body}</${Card}>`;
}

/** The alerts: what the bus's state says now, the on-time card's, the company's notices and
 * messages - the newest first. */
export function alertsOf(v) {
  const out = [];
  const c = v.clock;
  if (v.running && v.plugin === false) out.push({ id: 'plugin', kind: 'warning', title: tr('The Omsi-Hub plugin is off'), text: tr('Switch it on under Add-on to follow the drive here.') });
  else if (!v.live) out.push({ id: 'waiting', kind: 'info', title: tr('Waiting for the game'), text: tr('The dashboard fills in once you sit in the bus.') });
  const d = v.delay;
  const nextStop = v.stops?.[0];
  if (d != null && d >= 180) out.push({ id: 'late', kind: 'warning', title: tr('Running %{m} min late', { m: Math.round(d / 60) }), text: nextStop ? tr('Expected at %{stop} at %{time}.', { stop: nextStop.name, time: hhmm(nextStop.eta) }) : '', at: c });
  else if (d != null && d <= -60) out.push({ id: 'early', kind: 'info', title: tr('Running %{m} min early', { m: Math.round(-d / 60) }), text: tr('Wait at the stop until it is time to leave.'), at: c });
  const ic = v.ibis?.codes;
  if (v.ibis?.matches === false && ic) {
    const keys = `${tr('Linie/Kurs')} ${ic.line ?? '–'} · ${ic.route != null ? `${tr('Route')} ${pad(ic.route, 2)}` : `${tr('Ziel')} ${pad(ic.terminus ?? ic.terminus_code, 3)}`}`;
    out.push({ id: 'ibis', kind: 'warning', title: v.ibis.for === 'now' ? tr('Set the IBIS for this trip') : tr('Set the IBIS for the next trip'), text: `${keys} (${ic.display})`, at: c });
  }
  const b = v.next_break;
  if (b && c != null && ahead(b.from, c) > 0 && ahead(b.from, c) <= 15 * 60) {
    out.push({ id: 'break', kind: 'info', title: b.kind === 'layover' ? tr('Layover at %{time}', { time: hhmm(b.from) }) : tr('Break at %{time}', { time: hhmm(b.from) }), text: tr('At %{place}, until %{time}.', { place: b.at || '', time: hhmm(b.to) }), at: c });
  }
  for (const a of v.brief || []) out.push({ id: 'brief:' + a.id, kind: a.priority === 'high' ? 'warning' : 'info', title: tr('On time'), text: tr(a.template || '', a.args || {}), at: c });
  for (const t of v.told || []) out.push({ ...t, title: t.title || tr('Your company'), at: t.at });
  if (v.mode === 'company' && v.end != null) out.push({ id: 'end', kind: 'info', title: tr('Depot reminder'), text: tr('End of duty at %{time}. Return to %{place}.', { time: hhmm(v.end), place: v.end_place || '' }), at: v.start });
  else if (v.route?.trip && v.route.trip === v.route.trips && v.route.trips > 1) out.push({ id: 'last', kind: 'info', title: tr('Last trip of the duty'), text: tr('Your duty ends at %{place} at %{time}.', { place: v.end_place || '', time: hhmm(v.end) }), at: c });
  return out;
}

const KIND_ICON = { alert: 'error', warning: 'warning', info: 'info' };

function AlertsCard({ v }) {
  const list = alertsOf(v);
  return html`<${Card} icon="campaign" title=${tr('Alerts & updates')} sub=${tr('Important information')} cls="alerts"
    tools=${list.length > 0 && html`<span class="dash-count">${list.length}</span>`}>
    ${list.length
      ? html`<ul class="dash-alerts">${list.slice(0, 6).map((a) => html`<li class=${a.kind}>
          <span class="dot"><${Icon} name=${KIND_ICON[a.kind] || 'info'} size=${16} /></span>
          <div class="what"><b>${a.title}</b>${a.text && html`<span>${a.text}</span>`}</div>
          <span class="at">${a.at != null ? hhmm(a.at) : ''}</span>
        </li>`)}</ul>`
      : html`<p class="dash-empty">${tr('Nothing to report: a good drive.')}</p>`}
  </${Card}>`;
}

function ActionsCard({ onMap, onTimetable, onMessages }) {
  const act = (icon, label, on, primary) => html`<button type="button" class=${'dash-action' + (primary ? ' primary' : '')} onClick=${on}><${Icon} name=${icon} size=${24} /><span>${label}</span><${Icon} name="arrow_forward" size=${18} class="go" /></button>`;
  return html`<${Card} icon="bolt" title=${tr('Quick actions')} sub=${tr('Tools for your shift')} cls="actions">
    <div class="dash-actions">
      ${act('navigation', tr('Open navigator'), onMap, true)}
      ${act('departure_board', tr('View timetable'), onTimetable)}
      ${act('forum', tr('Messages'), onMessages)}
      ${act('settings', tr('Settings'), () => set({ page: 'settings', liveDash: { ...(S.liveDash || {}), hidden: true, from: 'settings' } }))}
    </div>
  </${Card}>`;
}

// --- the dialogs -----------------------------------------------------------------------------

function Dialog({ icon, title, onClose, wide, children, foot }) {
  useEffect(() => {
    const k = (e) => e.key === 'Escape' && onClose();
    addEventListener('keydown', k);
    return () => removeEventListener('keydown', k);
  }, []);
  return html`<div class="overlay" onMouseDown=${(e) => e.target === e.currentTarget && onClose()}>
    <div class=${'dialog dash-dialog' + (wide ? ' wide' : '')} role="dialog" aria-label=${title}>
      <div class="dialog-head"><h2><${Icon} name=${icon} size=${18} />${title}</h2><span class="grow"></span><button type="button" class="dash-iconbtn" title=${tr('Close')} onClick=${onClose}><${Icon} name="close" size=${18} /></button></div>
      <div class="dialog-body">${children}</div>
      ${foot && html`<div class="dialog-foot">${foot}</div>`}
    </div>
  </div>`;
}

function TimetableDialog({ v, onClose }) {
  const cur = v.route?.trip;
  const late = Math.max(v.delay || 0, 0);
  return html`<${Dialog} icon="departure_board" title=${tr('Timetable')} onClose=${onClose} wide>
    ${(v.trip_stops || []).length > 0 && html`<h4 class="dash-h4">${tr('This trip')}${v.route?.to ? ` · ${v.route.number || ''} ${v.route.to}` : ''}</h4>
      <table class="dash-table"><thead><tr><th>${tr('Stop')}</th><th>${tr('Due')}</th><th>${tr('Expected')}</th></tr></thead>
        <tbody>${v.trip_stops.filter((s) => s.stops).map((s) => html`<tr class=${s.passed ? 'passed' : ''}><td>${s.name}</td><td>${hhmm(s.due)}</td><td class=${!s.passed && late >= 60 ? 'late' : ''}>${s.passed ? '✓' : hhmm(s.eta)}</td></tr>`)}</tbody></table>`}
    ${(v.plan || []).length > 0
      ? html`<h4 class="dash-h4">${tr('The duty')}</h4>
        <table class="dash-table"><thead><tr><th>${tr('Departs')}</th><th>${tr('Line')}</th><th>${tr('From')}</th><th>${tr('To')}</th><th>${tr('Arrives')}</th></tr></thead>
          <tbody>${v.plan.map((p, i) => html`<tr class=${i + 1 === cur ? 'now' : cur && i + 1 < cur ? 'passed' : ''}><td>${hhmm(p.departure)}</td><td><span class="dash-line">${p.number || p.line}</span></td><td>${p.from}</td><td>${p.terminus}</td><td>${hhmm(p.arrival)}</td></tr>`)}</tbody></table>`
      : !(v.trip_stops || []).length && html`<p class="dash-empty">${v.mode === 'free' ? tr('A free drive has no timetable to follow.') : tr('The timetable is still being read.')}</p>`}
  </${Dialog}>`;
}

function MessagesDialog({ v, onClose }) {
  const list = alertsOf(v);
  return html`<${Dialog} icon="forum" title=${tr('Messages')} onClose=${onClose}>
    ${list.length
      ? html`<ul class="dash-alerts">${list.map((a) => html`<li class=${a.kind}><span class="dot"><${Icon} name=${KIND_ICON[a.kind] || 'info'} size=${16} /></span><div class="what"><b>${a.title}</b>${a.text && html`<span class="selectable">${a.text}</span>`}</div><span class="at">${a.at != null ? hhmm(a.at) : ''}</span></li>`)}</ul>`
      : html`<p class="dash-empty">${tr('No messages yet.')}</p>`}
    ${v.mode === 'company' && html`<p class="dash-note">${tr('Answer the control room in the game: F10 gives the panels the mouse.')}</p>`}
  </${Dialog}>`;
}

function EndDialog({ onClose }) {
  const [busy, setBusy] = useState(false);
  const end = async () => {
    setBusy(true);
    await call('hub_game_stop').catch(() => null);
    onClose();
  };
  return html`<${Dialog} icon="stop_circle" title=${tr('End the drive?')} onClose=${onClose}
    foot=${html`<button type="button" class="hub-btn" onClick=${onClose}>${tr('Keep driving')}</button><button type="button" class="hub-btn dash-danger" disabled=${busy} onClick=${end}>${tr('End the drive')}</button>`}>
    <p>${tr('openOMSI is closed. The trips you finished are kept; the one under way is not.')}</p>
  </${Dialog}>`;
}

// --- the page --------------------------------------------------------------------------------

function greeting(clock) {
  const h = clock != null ? Math.floor(clock / 3600) % 24 : new Date().getHours();
  if (h >= 5 && h < 12) return 'Good morning, %{name}!';
  if (h >= 12 && h < 18) return 'Good afternoon, %{name}!';
  return 'Good evening, %{name}!';
}

function dateText(date) {
  const d = date ? new Date(`${date}T12:00:00`) : new Date();
  if (isNaN(d)) return date || '';
  return d.toLocaleDateString(lang(), { weekday: 'short', day: 'numeric', month: 'short', year: 'numeric' });
}

export function DriveDash({ still = false }) {
  loadCss();
  useLive();
  const v = useStore((s) => s.live);
  const [open, setOpen] = useState(null);
  if (!v || !v.running) {
    return html`<div class="dash"><div class="dash-wait"><${Icon} name="directions_bus" size=${40} /><p>${tr('The game is starting…')}</p>
      <button type="button" class="hub-btn" onClick=${() => showDash(false)}><${Icon} name="chevron_left" size=${18} />${tr('Back to the launcher')}</button></div></div>`;
  }
  const close = () => setOpen(null);
  return html`<div class=${'dash' + (still ? ' still' : '')}>
    <header class="dash-head">
      <div><h1>${tr(greeting(v.clock), { name: v.driver || tr('Driver') })}</h1><p>${tr("Here's your duty overview and what's ahead.")}</p></div>
      <span class="grow"></span>
      <div class="dash-tools">
        <div class="dash-tabs dash-look" role="radiogroup" aria-label=${tr('Dark mode')}>
          ${[[false, 'light_mode', 'Day mode'], [true, 'dark_mode', 'Night mode']].map(([night, icon, label]) => html`<button type="button" role="radio" aria-checked=${dark() === night} class=${dark() === night ? 'on' : ''} title=${tr(label)} onClick=${(e) => dark() !== night && flipLook(e)}><${Icon} name=${icon} size=${16} /><span>${tr(label)}</span></button>`)}
        </div>
        <button type="button" class="hub-btn" onClick=${() => showDash(false)}><${Icon} name="chevron_left" size=${18} />${tr('Launcher')}</button>
        <button type="button" class="hub-btn" onClick=${() => setOpen('end')}><${Icon} name="stop_circle" size=${18} />${tr('End the drive')}</button>
      </div>
      <div class="dash-clock"><span>${dateText(v.date)}</span><b>${hhmm(v.clock)}</b>${v.speed != null && html`<small>${speedOf(v.speed).join(' ')}${v.delay != null ? ` · ${delayText(v.delay)}` : ''}</small>`}</div>
    </header>
    <div class="dash-grid">
      <${DutyCard} v=${v} />
      <${MapCard} v=${v} onOpen=${() => setOpen('map')} />
      <${StatsCard} v=${v} />
      <${StopsCard} v=${v} />
      <${AlertsCard} v=${v} />
      <${ActionsCard} onMap=${() => setOpen('map')} onTimetable=${() => setOpen('timetable')} onMessages=${() => setOpen('messages')} />
    </div>
    ${open === 'map' && html`<${NavDialog} v=${v} onClose=${close} />`}
    ${open === 'timetable' && html`<${TimetableDialog} v=${v} onClose=${close} />`}
    ${open === 'messages' && html`<${MessagesDialog} v=${v} onClose=${close} />`}
    ${open === 'end' && html`<${EndDialog} onClose=${close} />`}
  </div>`;
}

/** The way back to the dashboard while the driver is on the launcher's pages. */
export function LivePill() {
  const shown = useStore((s) => !!s.game?.running && !!s.liveDash?.hidden);
  return shown ? html`<${Pill} />` : null;
}

function Pill() {
  loadCss();
  useLive();
  const v = useStore((s) => s.live) || {};
  const r = v.route || {};
  return html`<button type="button" class="dash-pill" onClick=${() => showDash(true)}>
    <i class="live"></i><b>${tr('Live duty')}</b>
    ${r.number && html`<span class="dash-line">${r.number}</span>`}
    ${v.clock != null && html`<span>${hhmm(v.clock)}</span>`}
    ${v.delay != null && html`<span class="faint">${delayText(v.delay)}</span>`}
    <${Icon} name="chevron_right" size=${16} />
  </button>`;
}
