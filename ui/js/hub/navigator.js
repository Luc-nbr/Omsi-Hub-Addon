// The live dashboard's navigator (Luc, 2026-10-20), after openOMSI's in-game one (its MIT code of
// before 2026-10-14, `omsi-app/src/navigator.rs`): the map turns with the bus (or keeps north up)
// and zooms out with speed, the bus a little below the middle so the road ahead fills it; the
// trip's route lies on the roads, the part driven dimmed, its stops ahead as markers (the last a
// chequered flag); a plate says the next turn and how far, a bar the next stop - how far along
// the route, when the bus gets there, how late it runs.
//
// The roads and the trip's route come from `hub_route_map` (the duty's trips each by its name:
// `content::mapview::trips_page`), the bus from `hub_live_pos` thirty times a second (the
// plugin's `pos.json`; `hub_live`'s place every second where there is none). Every frame of the
// screen the bus goes on at the pace of its last two places, and what that was off from the next
// place is taken up over a moment, so it neither lags nor jumps (Luc, 2026-10-24). The roads are drawn once for a zoom
// (`roadlayer.js`) and that picture turned under the bus.

import { html, useEffect, useLayoutEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { Icon, tr } from './bits.js';
import { roadLayer, SETTLE } from './roadlayer.js';
import { SYMBOLS } from '../symbols.js';

// --- the trip's way --------------------------------------------------------------------------

/** Metres between two looks at the way's heading, for its turns. */
const STEP = 5;
/** A turn: the way turns more than this (degrees) within `TURN_SPAN` metres. */
const TURN_DEG = 40;
const TURN_SPAN = 30;
/** How far ahead turns are looked for (m). */
const TURN_REACH = 1500;
/** Further from the route than this (m), the bus is off it. */
const OFF_ROUTE = 35;

const wrap = (d) => ((((d + 180) % 360) + 360) % 360) - 180;
/** The heading of a direction, degrees clockwise from north (as the game's). */
const headingOf = (dx, dy) => (Math.atan2(dx, dy) * 180) / Math.PI;

/** The nearest point of the way `w` to `q` among the stretch from `from` to `to` metres:
 * `{ s, d }` (metres along it, metres off it). */
export function project(w, q, from = -Infinity, to = Infinity) {
  let best = null;
  const p = w.pts;
  for (let i = 0; i + 1 < p.length; i++) {
    if (w.cum[i + 1] < from || w.cum[i] > to) continue;
    const [ax, ay] = p[i];
    const [bx, by] = p[i + 1];
    const [dx, dy] = [bx - ax, by - ay];
    const l2 = dx * dx + dy * dy;
    const k = l2 > 0 ? Math.max(0, Math.min(1, ((q[0] - ax) * dx + (q[1] - ay) * dy) / l2)) : 0;
    const d = Math.hypot(ax + dx * k - q[0], ay + dy * k - q[1]);
    if (!best || d < best.d) best = { s: w.cum[i] + k * (w.cum[i + 1] - w.cum[i]), d };
  }
  return best;
}

/** The point `s` metres along the way. */
export function pointAt(w, s) {
  const p = w.pts;
  let i = 0;
  while (i + 2 < p.length && w.cum[i + 1] < s) i++;
  const l = w.cum[i + 1] - w.cum[i] || 1;
  const k = Math.max(0, Math.min(1, (s - w.cum[i]) / l));
  return [p[i][0] + (p[i + 1][0] - p[i][0]) * k, p[i][1] + (p[i + 1][1] - p[i][1]) * k];
}

/** A trip's way from its points (flat `[x0, y0, …]`, metres) and its stops' places (each
 * `[x, y]` or null, one a station): the metres to each point, the stops' metres along it (each
 * after the one before), the way's heading every `STEP` metres. */
export function makeWay(flat, stops) {
  const pts = [];
  for (let i = 0; i + 1 < flat.length; i += 2) {
    const q = [flat[i], flat[i + 1]];
    const l = pts[pts.length - 1];
    if (!l || Math.hypot(q[0] - l[0], q[1] - l[1]) > 0.05) pts.push(q);
  }
  if (pts.length < 2) return null;
  const cum = [0];
  for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const w = { pts, cum, len: cum[cum.length - 1], at: [], hs: [] };
  let after = 0;
  for (const at of stops || []) {
    const p = at && project(w, at, after - 1);
    // (a stop far off the route is no stop on it)
    if (p && p.d < 80) after = p.s;
    w.at.push(p && p.d < 80 ? p.s : null);
  }
  for (let s = 0; s + STEP <= w.len; s += STEP) {
    const [a, b] = [pointAt(w, s), pointAt(w, s + STEP)];
    w.hs.push(headingOf(b[0] - a[0], b[1] - a[1]));
  }
  return w;
}

/** The trip `name`'s way on the map `g` (`hub_route_map`): the duty's answer names its trips;
 * a single trip's answer is its lanes in order, its stops by their place in the trip. */
const ways = new WeakMap();
export function wayOf(g, name) {
  if (!g || !name) return null;
  let kept = ways.get(g);
  if (!kept) ways.set(g, (kept = new Map()));
  if (kept.has(name)) return kept.get(name);
  let w = null;
  const t = (g.trips || []).find((x) => x.name === name);
  if (t && t.route != null) w = makeWay(g.route[t.route] || [], t.stops || []);
  else if (!g.trips && (g.route || []).length) {
    const stops = [];
    for (const s of g.stops || []) if (s.k >= 0) stops[s.k] = s.at;
    w = makeWay(g.route.flat(), Array.from(stops, (x) => x || null));
  }
  kept.set(name, w);
  return w;
}

/** The first turn of the way after `s` (within `TURN_REACH`): `{ dir, angle, at }` - `dir` -1
 * left, 1 right, 2 back; `at` the metres to it. Gentle bends of a road are no turns. */
export function turnAhead(w, s) {
  if (!w || !w.hs.length) return null;
  const span = Math.round(TURN_SPAN / STEP);
  const j0 = Math.max(0, Math.floor(s / STEP) - Math.floor(span / 2));
  const last = Math.min(w.hs.length - 1 - span, j0 + TURN_REACH / STEP);
  for (let j = j0; j <= last; j++) {
    const d = wrap(w.hs[j + span] - w.hs[j]);
    if (Math.abs(d) < TURN_DEG) continue;
    // (how far it turns in all: over twice the span)
    let angle = d;
    for (let k = span; k <= 2 * span && j + k < w.hs.length; k++) {
      const e = wrap(w.hs[j + k] - w.hs[j]);
      if (Math.abs(e) > Math.abs(angle)) angle = e;
    }
    // (it turns where half the turn is done: a corner's own place)
    let mid = 0;
    while (mid < 2 * span && j + mid < w.hs.length && Math.abs(wrap(w.hs[j + mid] - w.hs[j])) < Math.abs(angle) / 2) mid++;
    const at = (j + mid) * STEP - s;
    if (at < -TURN_SPAN / 2) continue;
    return { dir: Math.abs(angle) > 150 ? 2 : angle > 0 ? 1 : -1, angle: Math.abs(angle), at: Math.max(0, at) };
  }
  return null;
}

/** The icon of a turn, as the game's navigator has them. */
export function turnIcon(t) {
  if (!t) return 'straight';
  if (t.dir === 2) return 'u_turn_left';
  if (t.angle < 60) return t.dir < 0 ? 'turn_slight_left' : 'turn_slight_right';
  return t.dir < 0 ? 'turn_left' : 'turn_right';
}

// --- the navigator ---------------------------------------------------------------------------

const KEPT_NORTH = 'omsihub.nav.north';
/** How often where the bus is is asked while the window has the focus (ms; a second else). */
const POS_EVERY = 33;
/** How long an error of the drawn bus takes to fall to a third (s). */
const CATCH = 0.1;
/** A bus turns no faster than this (degrees a second): a heading that seems to is a jolt. */
const TURN_RATE = 90;

function keptNorth() {
  try {
    return localStorage.getItem(KEPT_NORTH) === '1';
  } catch {
    return false;
  }
}

/** Where the bus is now (`hub_live_pos`: `{ n, t, x, y, heading, speed }`), asked thirty times a
 * second; null while the plugin says none (one of before it did, the game in its menu). */
function useBusPos() {
  const [p, setP] = useState(null);
  useEffect(() => {
    let alive = true;
    let timer = 0;
    let last = null;
    const tick = async () => {
      const q = await call('hub_live_pos').catch(() => null);
      if (!alive) return;
      const ok = q && Number.isFinite(q.x) && Number.isFinite(q.y) ? q : null;
      // (the same place again changes nothing)
      if ((ok?.n ?? null) !== last) ((last = ok?.n ?? null), setP(ok));
      timer = setTimeout(tick, document.hasFocus() ? POS_EVERY : 1000);
    };
    tick();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, []);
  return p;
}

/** The size an element is drawn at (CSS px). */
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

/** The look's colours the canvas draws with (CSS custom properties of `el`). */
function colours(el) {
  const cs = getComputedStyle(el);
  const get = (n, d) => cs.getPropertyValue(n).trim() || d;
  return {
    ground: get('--nav-ground', '#1b1b1f'),
    road: get('--nav-road', '#555'),
    main: get('--nav-road-main', '#777'),
    casing: get('--nav-casing', 'rgba(0,0,0,0.5)'),
    route: get('--hub-accent', '#3d7bff'),
    driven: get('--nav-driven', 'rgba(127,127,127,0.5)'),
    panel: get('--hub-panel', '#fff'),
    text: get('--hub-text', '#111'),
    halo: get('--nav-halo', '#000'),
  };
}

const busGlyph = typeof Path2D !== 'undefined' ? new Path2D(SYMBOLS.directions_bus || '') : null;
const flagGlyph = typeof Path2D !== 'undefined' ? new Path2D(SYMBOLS.sports_score || '') : null;

/** The navigator for the drive `v` (`hub_live`) on the map `g` (`hub_route_map`). `words`:
 * the words of numbers (`distance`, `hhmm`, `delay`) the dashboard has. */
export function Navigator({ v, g, words, big = false, onOverview }) {
  const box = useRef(null);
  const canvas = useRef(null);
  const [w, h] = useSize(box);
  const [north, setNorth] = useState(keptNorth);
  const [zoom, setZoom] = useState(1);
  const st = useRef(null);
  if (!st.current) st.current = { layer: roadLayer(), shown: null, to: null, err: null, s: null, mpp: null, raf: 0, last: 0, zoomedAt: 0 };
  const m = v.map || {};
  const o = g?.origin || [0, 0];
  const rel = (p) => p && [p[0] - o[0], p[1] - o[1]];
  // (the plugin's place thirty times a second; the dashboard's of every second where there is none)
  const live = useBusPos();
  const src = live ? { n: live.n, t: live.t, h: live.heading, speed: live.speed } : { n: null, t: null, h: m.heading, speed: v.speed };
  const bus = rel(live ? [live.x, live.y] : m.bus);
  const name = v.route?.trip_name || '';
  const way = wayOf(g, name) || (v.mode === 'free' ? wayOf(g, v.route?.free_route) : null);
  const trail = (m.trail || []).map(rel);

  // where the bus is on the route (from the last place on: a way that comes back past itself)
  let on = null;
  if (way && bus) {
    const s0 = st.current.s;
    on = (s0 != null && project(way, bus, s0 - 60, s0 + 400)) || null;
    if (!on || on.d > OFF_ROUTE) on = project(way, bus);
  }
  const onRoute = !!on && on.d <= OFF_ROUTE;
  const nextStop = (v.stops || [])[0];
  const nextAt = nextStop && way ? way.at[nextStop.index] : null;
  const toNext = onRoute && nextAt != null ? Math.max(0, nextAt - on.s) : nextStop?.metres ?? null;
  const turn = onRoute ? turnAhead(way, on.s) : null;
  // (a turn past the next stop is said once the bus has left it)
  const turnShown = turn && (toNext == null || turn.at <= toNext + 30 || v.status !== 'at_stop') ? turn : null;

  // a new place: the bus goes on from it at the pace of the last two, and where it is drawn off
  // from it is taken up over a moment
  useEffect(() => {
    const k = st.current;
    if (!bus) return;
    const now = performance.now();
    const heading = src.h ?? (onRoute ? way.hs[Math.min(way.hs.length - 1, Math.floor(on.s / STEP))] : null) ?? k.to?.h ?? 0;
    // (the game's seconds between two places where the plugin says them, else the page's)
    const t = src.t ?? now / 1000;
    const to = { x: bus[0], y: bus[1], h: heading, speed: src.speed || 0, t, at: now, vx: 0, vy: 0, vh: 0, coast: 0.5 };
    const p = k.to;
    const jump = !k.shown || Math.hypot(k.shown.x - to.x, k.shown.y - to.y) > 250 || document.querySelector('.dash.still');
    const dt = p ? t - p.t : 0;
    if (p && !jump && dt > 0 && dt < 3) {
      // (two places as good as at once: the pace stays the one before)
      if (dt < 0.02) Object.assign(to, { vx: p.vx, vy: p.vy, vh: p.vh });
      else Object.assign(to, { vx: (to.x - p.x) / dt, vy: (to.y - p.y) / dt, vh: Math.max(-TURN_RATE, Math.min(TURN_RATE, wrap(to.h - p.h) / dt)) });
      // (on at that pace a little longer than the places come, no further)
      to.coast = Math.max(0.25, Math.min(1.5, (1.5 * (now - p.at)) / 1000));
    }
    k.err = jump ? { x: 0, y: 0, h: 0 } : { x: k.shown.x - to.x, y: k.shown.y - to.y, h: wrap(k.shown.h - to.h) };
    if (jump) k.shown = { x: to.x, y: to.y, h: to.h };
    k.to = to;
    if (onRoute) k.s = on.s;
    k.wake?.();
  }, [src.n, bus?.[0], bus?.[1], src.h, g]);

  // the drawing, a frame at a time while something moves
  useEffect(() => {
    const k = st.current;
    const c = canvas.current;
    if (!c || !w || !h) return undefined;
    let alive = true;
    // (the look's colours once a drawing, not every frame)
    const pal = colours(c);
    const frame = () => {
      k.raf = 0;
      if (!alive || !k.to) return;
      const now = performance.now();
      const dt = Math.min(0.1, Math.max(0, (now - (k.last || now)) / 1000));
      k.last = now;
      const T = k.to;
      const ahead = Math.min(T.coast, (now - T.at) / 1000);
      const e = k.err;
      const fade = Math.exp(-dt / CATCH);
      e.x *= fade;
      e.y *= fade;
      e.h *= fade;
      const pos = { x: T.x + T.vx * ahead + e.x, y: T.y + T.vy * ahead + e.y, h: T.h + T.vh * ahead + e.h };
      k.shown = pos;
      // the zoom: further out the faster the bus goes, in steps (the roads drawn again at each)
      const vis = Math.max(220, Math.min(720, 220 + (k.to.speed || 0) * 7)) / zoom;
      const want = 2 ** (Math.round(Math.log2(vis / h) * 4) / 4);
      if (k.mpp == null) k.mpp = want;
      let moving = (ahead < T.coast && (Math.hypot(T.vx, T.vy) > 0.05 || Math.abs(T.vh) > 0.5)) || Math.hypot(e.x, e.y) > 0.01 || Math.abs(e.h) > 0.05;
      if (Math.abs(k.mpp - want) / want > 0.002) {
        k.mpp += (want - k.mpp) * 0.12;
        k.zoomedAt = now;
        moving = true;
      } else k.mpp = want;
      draw(c, pal, pos, k.mpp, now);
      // (every frame of the screen while the window has the focus, at rest too; in the
      // background only while something moves)
      if (moving || document.hasFocus() || now - k.zoomedAt < SETTLE + 40) k.raf = requestAnimationFrame(frame);
      else k.last = 0;
    };
    k.wake = () => {
      if (!k.raf && alive) k.raf = requestAnimationFrame(frame);
    };
    k.wake();
    return () => {
      alive = false;
      cancelAnimationFrame(k.raf);
      k.raf = 0;
      k.wake = null;
    };
  });

  useEffect(() => () => st.current.layer.drop(), []);

  /** One frame: the roads turned under the bus, the route (driven and ahead), the stops ahead,
   * the bus. */
  function draw(c, pal, pos, mpp, now) {
    const dpr = Math.min(window.devicePixelRatio || 1, big ? 1.5 : 2);
    if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) Object.assign(c, { width: Math.round(w * dpr), height: Math.round(h * dpr) });
    const ctx = c.getContext('2d');
    const cam = north ? 0 : pos.h;
    const rad = (cam * Math.PI) / 180;
    // (heading up: the bus below the middle, the road ahead before it)
    const ahead = north ? 0 : h * 0.2 * mpp;
    const cx = pos.x + Math.sin((pos.h * Math.PI) / 180) * ahead;
    const cy = pos.y + Math.cos((pos.h * Math.PI) / 180) * ahead;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = pal.ground;
    ctx.fillRect(0, 0, c.width, c.height);
    const D = Math.ceil(Math.hypot(w, h));
    if (g?.roads?.length) {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.translate(w / 2, h / 2);
      ctx.rotate(-rad);
      ctx.translate(-D / 2, -D / 2);
      const look = `${pal.road}|${pal.main}|${pal.casing}`;
      st.current.layer.put(ctx, {
        roads: g.roads, look, cam: { cx, cy, mpp }, W: D, H: D, dpr, zoomedAt: st.current.zoomedAt,
        later: () => st.current.wake?.(),
        paint: (gg, gs, u) => {
          gg.strokeStyle = pal.casing;
          for (const e of gs) ((gg.lineWidth = Math.max(e.w, u * 2) + u * 2), gg.stroke(e.path));
          for (const e of gs) if (!e.main) ((gg.strokeStyle = pal.road), (gg.lineWidth = Math.max(e.w, u * 2)), gg.stroke(e.path));
          for (const e of gs) if (e.main) ((gg.strokeStyle = pal.main), (gg.lineWidth = Math.max(e.w, u * 2)), gg.stroke(e.path));
        },
      });
    }
    // the world in metres from here on: x east, y north
    const world = () => {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.translate(w / 2, h / 2);
      ctx.rotate(-rad);
      ctx.scale(1 / mpp, -1 / mpp);
      ctx.translate(-cx, -cy);
    };
    const screen = (q) => {
      const [dx, dy] = [(q[0] - cx) / mpp, -(q[1] - cy) / mpp];
      const [cs, sn] = [Math.cos(-rad), Math.sin(-rad)];
      return [w / 2 + dx * cs - dy * sn, h / 2 + dx * sn + dy * cs];
    };
    const line = (pts, width, colour) => {
      if (pts.length < 2) return;
      ctx.beginPath();
      ctx.moveTo(pts[0][0], pts[0][1]);
      for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]);
      ctx.lineWidth = width * mpp;
      ctx.strokeStyle = colour;
      ctx.stroke();
    };
    world();
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    const s = onRoute && way ? project(way, [pos.x, pos.y], (st.current.s ?? 0) - 80, (st.current.s ?? 0) + 300)?.s ?? on.s : null;
    if (way) {
      const cut = s ?? 0;
      const i = way.cum.findIndex((x) => x > cut);
      const k = i < 0 ? way.pts.length : i;
      const here = s != null ? [pointAt(way, s)] : [];
      const driven = s != null ? [...way.pts.slice(0, k), ...here] : [];
      const rest = s != null ? [...here, ...way.pts.slice(k)] : way.pts;
      line(driven, 6, pal.driven);
      line(rest, 10, pal.casing);
      line(rest, 7, pal.route);
    } else if (trail.length > 1) {
      line(trail, 6, pal.driven);
    }
    // the stops ahead: the next one bigger, the last a flag
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (way) {
      const ahead = (v.stops || []).map((x) => ({ ...x, s: way.at[x.index] })).filter((x) => x.s != null);
      const lastIdx = way.at.length - 1;
      ahead.forEach((x, n) => {
        const [px, py] = screen(pointAt(way, x.s));
        if (px < -20 || py < -20 || px > w + 20 || py > h + 20) return;
        const r = n === 0 ? 10 : 7;
        ctx.beginPath();
        ctx.arc(px, py, r + 2, 0, Math.PI * 2);
        ctx.fillStyle = pal.panel;
        ctx.fill();
        ctx.beginPath();
        ctx.arc(px, py, r, 0, Math.PI * 2);
        ctx.fillStyle = n === 0 ? pal.route : pal.driven;
        ctx.fill();
        const glyph = x.index === lastIdx ? flagGlyph : busGlyph;
        if (glyph) {
          ctx.save();
          const gs = (r * 1.3) / 960;
          ctx.translate(px - r * 0.65, py + r * 0.65);
          ctx.scale(gs, gs);
          ctx.fillStyle = pal.panel;
          ctx.fill(glyph);
          ctx.restore();
        }
        if (n < (big ? 3 : 2)) {
          ctx.font = `600 ${n === 0 ? 12 : 11}px system-ui, sans-serif`;
          ctx.lineWidth = 3;
          ctx.strokeStyle = pal.halo;
          ctx.fillStyle = pal.text;
          ctx.strokeText(x.name, px + r + 6, py + 4);
          ctx.fillText(x.name, px + r + 6, py + 4);
        }
      });
    }
    // the bus: an arrow the way it heads
    const [bx, by] = screen([pos.x, pos.y]);
    ctx.save();
    ctx.translate(bx, by);
    ctx.rotate(((pos.h - cam) * Math.PI) / 180);
    ctx.beginPath();
    ctx.arc(0, 0, 22, 0, Math.PI * 2);
    ctx.fillStyle = pal.route;
    ctx.globalAlpha = 0.16 + 0.06 * Math.sin(now / 400);
    ctx.fill();
    ctx.globalAlpha = 1;
    ctx.beginPath();
    ctx.moveTo(0, -15);
    ctx.lineTo(11, 11);
    ctx.lineTo(0, 5);
    ctx.lineTo(-11, 11);
    ctx.closePath();
    ctx.lineJoin = 'round';
    ctx.lineWidth = 3;
    ctx.strokeStyle = pal.panel;
    ctx.stroke();
    ctx.fillStyle = pal.route;
    ctx.fill();
    ctx.restore();
  }

  const flip = () => {
    const n = !north;
    setNorth(n);
    try {
      localStorage.setItem(KEPT_NORTH, n ? '1' : '0');
    } catch {
      // (a private window: this session only)
    }
  };
  const camDeg = north ? 0 : st.current.shown?.h ?? m.heading ?? 0;
  const late = v.delay != null && Math.abs(v.delay) >= 60;
  return html`<div class=${'dash-nav' + (big ? ' big' : '')} ref=${box}>
    <canvas ref=${canvas} style=${`width:${w}px;height:${h}px`} aria-label=${tr('Navigator')}></canvas>
    ${way && onRoute && html`<div class="nav-turn">
      <${Icon} name=${turnIcon(turnShown)} size=${big ? 30 : 24} />
      <div><b>${turnShown ? words.distance(turnShown.at) : toNext != null ? words.distance(toNext) : ''}</b><span>${turnShown ? (turnShown.dir === 2 ? tr('Turn round') : turnShown.dir < 0 ? tr('Turn left') : tr('Turn right')) : tr('Straight on')}</span></div>
    </div>`}
    ${way && !onRoute && bus && html`<div class="nav-turn off"><${Icon} name="alt_route" size=${22} /><div><b>${words.distance(on?.d || 0)}</b><span>${tr('Off the route')}</span></div></div>`}
    <div class="nav-tools">
      <button type="button" title=${north ? tr('North up') : tr('Heading up')} class="nav-compass" onClick=${flip}><span style=${`transform:rotate(${-camDeg}deg)`}><${Icon} name="navigation" size=${18} /></span><small>${north ? 'N' : ''}</small></button>
      <button type="button" title=${tr('Closer')} onClick=${() => setZoom((z) => Math.min(z * 1.5, 6))}><${Icon} name="add" size=${18} /></button>
      <button type="button" title=${tr('Further')} onClick=${() => setZoom((z) => Math.max(z / 1.5, 0.25))}><${Icon} name="remove" size=${18} /></button>
      ${onOverview && html`<button type="button" title=${tr('The whole duty')} onClick=${onOverview}><${Icon} name="fit_screen" size=${18} /></button>`}
    </div>
    ${nextStop && html`<div class="nav-next">
      <span class="dot"><${Icon} name="directions_bus" size=${16} /></span>
      <div class="what"><b class="cut">${nextStop.name}</b><span>${v.status === 'at_stop' ? tr('At the stop') : [toNext != null && words.distance(toNext), nextStop.minutes != null && (nextStop.minutes < 1 ? tr('due') : tr('%{m} min', { m: Math.round(nextStop.minutes) }))].filter(Boolean).join(' · ')}</span></div>
      <div class="when"><b>${words.hhmm(nextStop.eta)}</b>${late && html`<span class=${v.delay > 0 ? 'late' : 'early'}>${words.delay(v.delay)}</span>`}</div>
    </div>`}
  </div>`;
}
