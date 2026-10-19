// The depot in 3D on the depot page, over all of it: the yard from `scene.js` drawn by
// `renderer.js` in the light of the company's time of day (`palette.js`: the day, the dusk and
// the night, the sun on its way over the sky), its labels floating over the buildings. A click
// on a building or its label opens its card over it: what it does, the next level and what can
// be added to it, and to move it or pull it down; a click on the land to buy, its card; a click
// on a bus picks it for the workshop's list (its card: the jobs to give it), clicked again or
// on the ground, let go. In the build mode (`placing`: a module of an area to put down, or one
// to move) its ghost follows the pointer on the yard's grid, under the pointer at its own
// height, green where it fits (`yard.js`), R turns it, a click puts it down, Escape or a right
// click stops. Drag to turn the yard round (in 2D: to move it), right-drag (or Shift) to move
// it, scroll to zoom; the camera is kept for when the page comes back. Each bus is drawn as its
// own model in its livery once that is there (`skins.js`), until then as the plain one. How
// finely it is drawn is chosen in its menu (`quality.js`). Without WebGL2 the page draws its
// plan from above instead (`onFail`).

import { html, useEffect, useRef, useState, tr, Icon, IconBtn, Btn, Meter, ui, setUi } from '../kit.js';
import { actMore } from '../more.js';
import { project } from '../../hub/bus3d/math.js';
import { buildScene, AREAS, TALL, moduleUnder, ghostMesh, gridMesh } from './scene.js';
import { rectOf, whyNot } from './yard.js';
import { busMesh, instances, pickBus, skinned } from './bus.js';
import { skin, skinKey } from './skins.js';
import { DepotRenderer, VIEW, ZOOM } from './renderer.js';
import { dayStep, lightAt, palette, rgb01, skyAt } from './palette.js';
import { LEVELS, chooseQuality, chosenQuality, levelOf, stepDown } from './quality.js';

/** The menu's levels: what is chosen, its name, and what it gives up. */
const QUALITY = [
  ['auto', 'Automatic', ''],
  ['high', 'High', 'Shadows, smooth edges, full sharpness'],
  ['medium', 'Medium', 'Softer shadows, a little less sharp'],
  ['low', 'Low', 'No shadows or trees: for slower computers'],
];
const NAMES = { high: 'Now: high', medium: 'Now: medium', low: 'Now: low' };
/** Why a module does not fit, in words (as `hub_core::company::yard::fits` says it). */
const WHY = {
  land: 'It does not fit on your land here: put it elsewhere or buy more land.',
  gate: 'It would block the way in from the gate.',
  taken: 'Something stands there already.',
  lane: 'Keep the lanes in front of the buildings and spaces free.',
};
const GREEN = [0.2, 0.78, 0.42, 0.7];
const RED = [0.92, 0.26, 0.22, 0.7];
/** How far the pointer may move and still click (not drag), in pixels. */
const CLICK = 6;

/** The areas' icons, as the Build list has them (`depot::Area::icon`), for an area it does not
 * list. */
const ICONS = { parking: 'local_parking', workshop: 'construction', wash: 'water_drop', fuel: 'inventory_2', charging: 'bolt', offices: 'groups' };

const still = () => document.documentElement.classList.contains('hub-still');
const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

/**
 * `p` the plan (`company_more::depot::plan`), `rows` the Build list's areas (their levels, the
 * next one's price, what can be added), `wsBuses` the workshop's list (the jobs a bus can be
 * given), `minute` the company's time of day; `right` and `top` what the page lays over the
 * yard's right side and top (its panel, its bar), kept free of the yard's own tools and cards.
 */
export function Depot3D({ p, rows = [], wsBuses = [], w, h, selected, hover, onPick, onFail, placing, onPlaced, flat, minute = 12 * 60, right = 0, top = 0 }) {
  const canvas = useRef(null);
  const tags = useRef(null);
  const card = useRef(null);
  const anchor = useRef(null);
  /** What the page lays over the yard (its panel, its bar), for the card to keep clear of. */
  const inset = useRef(null);
  inset.current = { right, top };
  const r = useRef(null);
  const scene = useRef(null);
  const drag = useRef(null);
  const [under, setUnder] = useState(null);
  const [quality, setQuality] = useState(chosenQuality);
  const [level, setLevel] = useState(() => levelOf(chosenQuality()));
  const [menu, setMenu] = useState(false);
  const [, setBuilt] = useState(0);
  const [ghost, setGhost] = useState(null);
  const [rot, setRot] = useState(0);
  /** The card open: a module's (`{ id }`) or the land's (`{ land: true }`); a bus's is the
   * page's `selected`. */
  const [sel, setSel] = useState(null);
  const [sure, setSure] = useState(false);
  const pointer = useRef(null);
  const [skins, setSkins] = useState(0);
  const auto = useRef(quality === 'auto');
  auto.current = quality === 'auto';
  const day = dayStep(minute);
  const pal = palette(p.look, day);
  const key = JSON.stringify([p, day, pal.accent]);
  // (the light again every two minutes of the company's time)
  const lightKey = Math.floor(minute / 2);

  // the renderer, on a canvas of its own for each level (smoothing is the context's); its
  // context let go when the page goes
  useEffect(() => {
    const el = canvas.current;
    const slow = () => auto.current && level !== 'low' && setLevel(stepDown(level));
    const view = new DepotRenderer(el, LEVELS[level], { onFrame: (vp, cw, ch) => placeTags(vp, cw, ch), onSlow: slow, onLost: () => onFail?.() });
    if (!view.ok) {
      onFail?.();
      return undefined;
    }
    r.current = view;
    view.resize(w, h);
    const kept = ui('depot').cam;
    view.aim(kept || VIEW, true);
    // scrolling zooms towards the point under the pointer
    const wheel = (e) => {
      e.preventDefault();
      const g = view.goal;
      const zoom = Math.max(ZOOM[0], Math.min(ZOOM[1], g.zoom * Math.exp(e.deltaY * 0.0012)));
      const b = el.getBoundingClientRect();
      const hit = view.ground(e.clientX - b.left, e.clientY - b.top);
      const f = 1 - zoom / g.zoom;
      view.aim(hit ? { zoom, tx: g.tx + (hit[0] - g.tx) * f, tz: g.tz + (hit[1] - g.tz) * f } : { zoom }, still());
      keep();
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => {
      el.removeEventListener('wheel', wheel);
      view.destroy();
      r.current = null;
    };
  }, [level]);

  useEffect(() => r.current?.resize(w, h), [w, h]);

  // the yard again when the depot, the look or the light's colours changed
  useEffect(() => {
    const view = r.current;
    if (!view) return;
    const s = buildScene(p, pal, LEVELS[level]);
    scene.current = s;
    view.setBusModel(busMesh(pal));
    view.setYard(s.mesh, s.bounds, pal, s.world);
    view.setDecals(s.decals);
    buses(true);
    setBuilt((n) => n + 1);
  }, [key, level]);

  // the sun where the company's clock has it
  useEffect(() => r.current?.setLight(lightAt(minute)), [lightKey, level]);

  // the buses' own models, as they come
  useEffect(() => buses(true), [skins]);

  // (the labels and the card drawn again: a picture puts them where they belong)
  useEffect(() => r.current?.invalidate());

  // the picked bus and the one under the pointer
  useEffect(() => buses(false), [selected, under]);

  // a card open: Escape closes it (before the page's own keys: it does not go back)
  useEffect(() => {
    if ((!sel && selected == null) || placing) return undefined;
    const onKey = (e) => {
      if (e.key !== 'Escape' || e.target?.closest?.('input, textarea, [contenteditable]')) return;
      close();
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [sel, selected, placing]);

  /** The buses to the renderer: each as its own model where that is there (asked for where it
   * is not: `skins.js`), else as the plain one. */
  function buses(moved) {
    const view = r.current;
    const s = scene.current;
    if (!view || !s) return;
    const come = () => setSkins((n) => n + 1);
    const own = new Map();
    for (const b of s.buses) {
      const k = skin(b.file, b.paint, come);
      if (!k) continue;
      const key = skinKey(b.file, b.paint);
      view.setSkin(key, k.model, k.image);
      // (its own length and height: the ring and the pointer go by them)
      b.len = k.model.length;
      b.height = k.model.height;
      own.set(b.id, key);
    }
    const states = { picked: selected, hover: under };
    view.setBuses(instances(s.buses, rgb01(p.colour), { ...states, own: (b) => own.has(b.id) }), moved);
    view.setSkinned(skinned(s.buses, (b) => own.get(b.id), states), moved);
  }

  useEffect(() => r.current?.highlight(AREAS[hover] || 0), [hover]);

  // 2D: the yard from straight above (and back to the view from the corner)
  useEffect(() => {
    if (r.current) r.current.aim(flat ? { yaw: 0, pitch: 1.48 } : { yaw: VIEW.yaw, pitch: VIEW.pitch }, still());
  }, [flat, level]);

  // the build mode: the grid and every lane on, the ghost following the pointer; R turns it,
  // Escape stops
  useEffect(() => {
    const view = r.current;
    if (!view) return undefined;
    setGhost(null);
    view.setOverlay('ghost', null);
    if (!placing) {
      view.setOverlay('grid', null);
      return undefined;
    }
    setSel(null);
    setRot(placing.rot || 0);
    view.setOverlay('grid', gridMesh(p.layout, pal), [1, 1, 1, 0.3], 0.35);
    // (before the page's own keys: Escape here stops the build mode, it does not go back)
    const onKey = (e) => {
      if (e.target?.closest?.('input, textarea, [contenteditable]')) return;
      if (e.key === 'Escape') onPlaced?.();
      else if (e.key === 'r' || e.key === 'R') setRot((k) => (k + 1) % 4);
      else return;
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [placing?.area, placing?.move, key, level]);

  useEffect(() => {
    if (placing && pointer.current) aimGhost(pointer.current);
  }, [rot]);

  /** The ghost where the pointer is: its cells on the grid, whether it fits. The pointer meets
   * it half way up (the ground for a block of spaces), so it holds the ghost where it is seen,
   * not the ground behind it. */
  function aimGhost(e) {
    const view = r.current;
    const sc = scene.current;
    const L = p.layout;
    if (!view || !sc || !placing) return;
    const [px, py] = at(e);
    const hit = view.ground(px, py, (TALL[placing.area] || 3) / 2);
    if (!hit) return;
    const b = sc.bounds;
    const gx0 = hit[0] + (b.x0 + b.x1) / 2;
    const gz0 = hit[1] + (b.z0 + b.z1) / 2;
    const rr = rectOf(L, placing.area, 0, 0, rot);
    const x = Math.round(gx0 / L.cell - rr.w / 2);
    const y = Math.round(gz0 / L.cell - rr.h / 2);
    if (ghost && ghost.x === x && ghost.y === y && ghost.rot === rot) return;
    const why = whyNot(L, placing.area, x, y, rot, placing.move ?? null);
    setGhost({ x, y, rot, why });
    view.setOverlay('ghost', ghostMesh(L, placing.area, x, y, rot, pal), why ? RED : GREEN, 0.6);
  }

  async function putDown() {
    if (!ghost || ghost.why) return;
    const args = { x: ghost.x, y: ghost.y, rot: ghost.rot };
    const done = placing.move ? await actMore('depot_move', { id: placing.move, ...args }) : await actMore('depot_build_at', { area: placing.area, ...args });
    if (done?.ok) onPlaced?.();
  }

  /** The ground under a point of the canvas, in the yard's metres. */
  function groundAt(e) {
    const view = r.current;
    const sc = scene.current;
    if (!view || !sc) return null;
    const [px, py] = at(e);
    const hit = view.ground(px, py);
    if (!hit) return null;
    const b = sc.bounds;
    return [hit[0] + (b.x0 + b.x1) / 2, hit[1] + (b.z0 + b.z1) / 2];
  }

  /** The labels over what they name, and the open card over what it is about, where the camera
   * now shows them. */
  function placeTags(vp, cw, ch) {
    const box = tags.current;
    const s = scene.current;
    if (!box || !s) return;
    [...box.children].forEach((el, k) => {
      const t = s.labels[k];
      if (!t) return;
      const q = project(vp, t.pos);
      const on = q[3] > 0 && q[0] > -1.1 && q[0] < 1.1 && q[1] > -1.1 && q[1] < 1.1;
      el.style.display = on ? '' : 'none';
      if (on) el.style.transform = `translate(${Math.round((q[0] * 0.5 + 0.5) * cw)}px, ${Math.round((0.5 - q[1] * 0.5) * ch)}px) translate(-50%, -100%)`;
    });
    // the card: over its point when there is room, else beside it (right, then left), else under
    // it; within the yard's free part, its arrow at its point
    const c = card.current;
    const a = anchor.current;
    if (!c || !a) return;
    const q = project(vp, a);
    if (q[3] <= 0) {
      c.style.visibility = 'hidden';
      return;
    }
    const px = (q[0] * 0.5 + 0.5) * cw;
    const py = (0.5 - q[1] * 0.5) * ch;
    const cardW = c.offsetWidth;
    const cardH = c.offsetHeight;
    const pad = 10;
    const gap = 18;
    const { right: rx, top: ty } = inset.current;
    const x0 = pad;
    const x1 = Math.max(pad, cw - rx - pad - cardW);
    const y0 = ty + pad;
    const y1 = Math.max(y0, ch - pad - cardH);
    let side = 'above';
    if (py - cardH - gap < y0) side = px + gap + cardW <= cw - rx - pad ? 'right' : px - gap - cardW >= pad ? 'left' : 'below';
    const want = { above: [px - cardW / 2, py - cardH - gap], below: [px - cardW / 2, py + gap], right: [px + gap, py - cardH / 2], left: [px - gap - cardW, py - cardH / 2] }[side];
    const x = clamp(want[0], x0, x1);
    const y = clamp(want[1], y0, y1);
    c.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
    c.dataset.side = side;
    const across = side === 'above' || side === 'below';
    const along = across ? clamp(px - x, 18, cardW - 18) : clamp(py - y, 18, cardH - 18);
    c.style.setProperty('--arrow', `${Math.round(along)}px`);
    // (no arrow when the card could not stay by its point)
    c.classList.toggle('loose', Math.abs(across ? y - want[1] : x - want[0]) > 1 || Math.abs((across ? px - x : py - y) - along) > 1);
    c.style.visibility = '';
  }

  const keep = () => {
    clearTimeout(keep.t);
    keep.t = setTimeout(() => r.current && setUi('depot', { cam: { ...r.current.goal } }), 400);
  };

  const at = (e) => {
    const b = canvas.current.getBoundingClientRect();
    return [e.clientX - b.left, e.clientY - b.top];
  };
  /** The bus under a point: the one its box (a little larger than the bus) meets first, else
   * the one whose middle is nearest on the screen, within a finger's width. */
  const busAt = (e) => {
    const view = r.current;
    const s = scene.current;
    if (!view || !s) return null;
    const [x, y] = at(e);
    const { o, d } = view.ray(x, y);
    const k = pickBus(s.buses, o, d);
    if (k >= 0) return s.buses[k];
    const { vp } = view.matrices();
    let best = null;
    let near = 20 * 20;
    for (const b of s.buses) {
      const q = project(vp, [b.x, (b.height || 3) / 2, b.z]);
      if (q[3] <= 0) continue;
      const dd = ((q[0] * 0.5 + 0.5) * w - x) ** 2 + ((0.5 - q[1] * 0.5) * h - y) ** 2;
      if (dd < near) {
        near = dd;
        best = b;
      }
    }
    return best;
  };

  /** Close the card open (and let a picked bus go). */
  const close = () => {
    setSel(null);
    setSure(false);
    if (selected != null) onPick?.(selected);
  };

  /** Open a card: a module's or the land's (a bus picked let go). */
  const open = (s) => {
    setSel(s);
    setSure(false);
    setMenu(false);
    if (selected != null) onPick?.(selected);
  };

  const down = (e) => {
    if (!r.current) return;
    canvas.current.setPointerCapture?.(e.pointerId);
    drag.current = { x: e.clientX, y: e.clientY, moved: 0, pan: e.button === 2 || e.button === 1 || e.shiftKey };
  };
  const move = (e) => {
    const view = r.current;
    if (!view) return;
    const g = drag.current;
    if (!g && placing) {
      pointer.current = { clientX: e.clientX, clientY: e.clientY };
      aimGhost(e);
      return;
    }
    if (!g) {
      const b = busAt(e);
      const id = b ? b.id : null;
      if (id !== under) setUnder(id);
      return;
    }
    const dx = e.clientX - g.x;
    const dy = e.clientY - g.y;
    g.x = e.clientX;
    g.y = e.clientY;
    g.moved += Math.abs(dx) + Math.abs(dy);
    if (g.moved < CLICK) return;
    const c = view.goal;
    if (g.pan || flat) {
      const k = view.matrices().dist * 0.0016;
      const right = [Math.cos(c.yaw), -Math.sin(c.yaw)];
      const away = [-Math.sin(c.yaw), -Math.cos(c.yaw)];
      view.aim({ tx: c.tx - right[0] * dx * k + away[0] * dy * k, tz: c.tz - right[1] * dx * k + away[1] * dy * k }, true);
    } else view.aim({ yaw: c.yaw - dx * 0.008, pitch: c.pitch + dy * 0.006 }, true);
    if (under !== null) setUnder(null);
  };
  const up = (e) => {
    const g = drag.current;
    drag.current = null;
    if (!g) return;
    if (g.moved < CLICK && placing) {
      if (e.button === 2) onPlaced?.();
      else if (e.button === 0) putDown();
    } else if (g.moved < CLICK && e.button === 0) {
      const b = busAt(e);
      const s = scene.current;
      const spot = groundAt(e);
      const o = spot && s && moduleUnder(s.picks, spot[0], spot[1]);
      const land = spot && s?.land && spot[0] >= s.land.x0 && spot[0] <= s.land.x1 && spot[1] >= s.land.z0 && spot[1] <= s.land.z1;
      if (b) {
        setSel(null);
        onPick?.(b.id);
      } else if (o) open(sel?.id === o.id ? null : { id: o.id });
      else if (land) open(sel?.land ? null : { land: true });
      else close();
    } else keep();
  };

  const aim = (patch) => {
    r.current?.aim(patch, still());
    keep();
  };
  const fullscreen = () => {
    const page = canvas.current?.closest('.cm-page');
    if (!page) return;
    try {
      if (document.fullscreenElement) document.exitFullscreen();
      else page.requestFullscreen?.();
    } catch {
      // (not where the window cannot go full screen)
    }
  };

  const s = scene.current;
  const lit = (key) => r.current?.highlight(AREAS[key] || 0);
  const tagFor = (t) => {
    const k = t.key;
    const row = rows.find((a) => a.key === k);
    const n = { icon: row?.icon || ICONS[k], label: k === 'land' ? tr('More land') : row?.label || t.head };
    const on = (hover && k === hover) || (sel?.id != null && sel.id === t.id) || (k === 'land' && sel?.land);
    const pick = (e) => {
      e.stopPropagation();
      if (placing) return;
      if (k === 'land') open(sel?.land ? null : { land: true });
      else open(sel?.id === t.id ? null : { id: t.id });
    };
    const events = { onClick: pick, onMouseEnter: () => lit(k), onMouseLeave: () => lit(hover) };
    if (t.works) return html`<button type="button" class=${'dp-tag works' + (on ? ' lit' : '')} ...${events}><${Icon} name="construction" size=${13} /><span>${n.label}<small>${t.works}</small></span></button>`;
    if (!t.built) return html`<button type="button" class=${'dp-tag plot' + (on ? ' lit' : '')} ...${events}><${Icon} name="add" size=${13} /><span>${n.label}</span></button>`;
    return html`<button type="button" class=${'dp-tag' + (on ? ' lit' : '')} ...${events}><${Icon} name=${n.icon || 'info'} size=${13} /><span>${n.label}</span></button>`;
  };
  const tip = under && under !== selected && s?.buses.find((b) => b.id === under);

  // the card open, and the point it stands over
  let body = null;
  anchor.current = null;
  if (s && !placing) {
    const bus = selected != null ? s.buses.find((b) => b.id === selected) : null;
    const o = sel?.id != null ? s.picks.find((x) => x.id === sel.id) : null;
    if (bus) {
      anchor.current = [bus.x, (bus.height || 3.1) + 0.6, bus.z];
      body = html`<${BusCard} bus=${bus} ws=${wsBuses.find((b) => b.id === bus.id)} onClose=${close} />`;
    } else if (o) {
      anchor.current = [(o.x0 + o.x1) / 2, o.h, (o.z0 + o.z1) / 2];
      body = html`<${ModuleCard} o=${o} rows=${rows} sure=${sure} setSure=${setSure} onClose=${close} onMove=${() => setUi('depot', { placing: { area: o.area, move: o.id, rot: o.rot } })} onBuild=${() => setSel(null)} onGone=${() => setSel(null)} />`;
    } else if (sel?.land && s.land) {
      anchor.current = [(s.land.x0 + Math.min(s.land.x1, s.land.x0 + 40)) / 2, 2.5, (s.land.z0 + s.land.z1) / 2];
      body = html`<${LandCard} land=${p.layout?.land} onClose=${close} />`;
    }
  }

  const hint = placing ? tr('Click to put it down, R to turn it, Esc to stop') : flat ? tr('Drag to move, scroll to zoom') : tr('Drag to turn, right-drag to move, scroll to zoom');
  return html`<div class="cm-plan cm-plan3d" data-buses=${s ? s.buses.length : 0} style=${`width:${w}px;height:${h}px;background:${skyAt(minute)}`}>
    <canvas key=${level} ref=${canvas} class=${under ? 'on-bus' : ''} style=${`width:${w}px;height:${h}px`} onPointerDown=${down} onPointerMove=${move} onPointerUp=${up} onPointerLeave=${() => !drag.current && under !== null && setUnder(null)} onContextMenu=${(e) => e.preventDefault()}></canvas>
    <div class="dp-tags" ref=${tags}>${s?.labels.map(tagFor)}</div>
    ${tip && !placing && html`<div class="dp-tip">${tip.tip}</div>`}
    ${placing && html`<div class="dp-place" style=${`right:${right + 12}px`}>
      <span class=${'dp-why' + (ghost?.why ? ' no' : '')}>${ghost?.why ? tr(WHY[ghost.why]) : placing.move ? tr('Move it where it fits: it costs nothing.') : tr('Put it down where it fits, its lane kept free.')}</span>
      <${Btn} kind="ghost" icon="refresh" label=${tr('Turn')} onClick=${() => setRot((k) => (k + 1) % 4)} />
      <${Btn} kind="ghost" icon="close" label=${tr('Stop')} onClick=${() => onPlaced?.()} />
    </div>`}
    ${body && html`<div class="dp-card" ref=${card} style="visibility:hidden" onPointerDown=${(e) => e.stopPropagation()}>${body}</div>`}
    <div class="dp-tools" style=${`right:${right + 10}px;top:${top + 10}px`}>
      <${IconBtn} icon="zoom_in" size=${17} title=${tr('Zoom in')} onClick=${() => aim({ zoom: (r.current?.goal.zoom || 1) / 1.35 })} />
      <${IconBtn} icon="zoom_out" size=${17} title=${tr('Zoom out')} onClick=${() => aim({ zoom: (r.current?.goal.zoom || 1) * 1.35 })} />
      <${IconBtn} icon="north" size=${17} title=${tr('From above')} onClick=${() => aim({ yaw: 0, pitch: 1.48 })} />
      <${IconBtn} icon="restart_alt" size=${17} title=${tr('Reset the view')} onClick=${() => aim({ ...VIEW })} />
      <${IconBtn} icon="fullscreen" size=${17} title=${tr('Full screen')} onClick=${fullscreen} />
      <${IconBtn} icon="tune" size=${17} title=${tr('Graphics')} onClick=${() => setMenu(!menu)} />
    </div>
    ${menu && html`<div class="dp-menu" role="menu" style=${`right:${right + 54}px;top:${top + 10}px`}>
      <div class="co-caps">${tr('Graphics')}</div>
      ${QUALITY.map(([q, label, under]) => html`<button type="button" role="menuitemradio" aria-checked=${q === quality} class=${q === quality ? 'on' : ''} onClick=${() => {
        chooseQuality(q);
        setQuality(q);
        setLevel(levelOf(q));
        setMenu(false);
      }}><span>${tr(label)}<small>${q === 'auto' ? tr(NAMES[level]) : tr(under)}</small></span>${q === quality && html`<${Icon} name="check" size=${16} />`}</button>`)}
    </div>`}
    ${!placing && html`<div class="dp-hint" style=${`right:${right + 12}px`}>${hint}</div>`}
  </div>`;
}

/** The card's head: its icon, its title, what it says under that, and its close. */
function Head({ icon, title, sub, tone, onClose }) {
  return html`<div class="dp-head">
    ${icon && html`<span class="dp-icon"><${Icon} name=${icon} size=${18} /></span>`}
    <span class="dp-title"><b>${title}</b>${sub && html`<span class=${tone ? 't-' + tone : ''}>${sub}</span>`}</span>
    <${IconBtn} icon="close" size=${16} title=${tr('Close')} onClick=${onClose} />
  </div>`;
}

/** An area's next level: its level as pips, and its price to build it (put down where the player
 * wants it, for a module more), or the work under way, or that it is complete. */
function NextLevel({ a, onBuild }) {
  const build = () => {
    if (a.place && !a.locked) {
      onBuild?.();
      return setUi('depot', { placing: { area: a.key }, selected: null });
    }
    return actMore('depot_build', { area: a.key });
  };
  const pips = a.max > 0 && a.max <= 10;
  return html`<div class="dp-level">
    <span class="dp-lv">
      ${pips ? html`<span class="cm-pips" title=${a.level}>${Array.from({ length: a.max }, (_, i) => html`<i class=${i < a.lv ? 'on' : i === a.lv && a.works ? 'works' : ''}></i>`)}</span>` : html`<small>${a.level}</small>`}
      <small>${a.effect}</small>
    </span>
    ${a.works
      ? html`<span class="dp-said t-warn">${a.works}</span>`
      : a.cost
        ? html`<${Btn} icon=${a.locked ? 'lock' : a.place ? 'open_with' : 'add'} label=${a.cost} title=${a.tip} onClick=${build} />`
        : html`<span class="dp-said t-ok">${a.complete}</span>`}
  </div>`;
}

/** What can be added to an area: each with what it does, and its price, the work under way,
 * what it needs first, or that it is there. */
function Upgrades({ list }) {
  if (!list?.length) return null;
  return html`<div class="dp-ups">
    <div class="co-caps">${tr('What can be added')}</div>
    ${list.map((u) => html`<div class="dp-up" title=${u.what}>
      <span class=${u.built ? 't-ok' : 't-soft'}><${Icon} name=${u.built ? 'check_circle' : u.icon} size=${16} /></span>
      <span class="dp-uptext"><b>${u.label}</b><small>${u.what}</small></span>
      ${u.built
        ? html`<span class="dp-said t-ok">${tr('Added')}</span>`
        : u.works
          ? html`<span class="dp-said t-warn">${u.works}</span>`
          : u.needs
            ? html`<span class="dp-said t-faint">${u.needs}</span>`
            : u.cost && html`<${Btn} icon="add" label=${u.cost} title=${u.busy ? tr('Wait for the building work on this area') : u.tip} disabled=${u.busy} onClick=${() => actMore('depot_upgrade', { key: u.key })} />`}
    </div>`)}
  </div>`;
}

/** A module's card: its area's level and the next one, what can be added (the charging points
 * on a block of spaces too), and to move it or pull it down. */
function ModuleCard({ o, rows, sure, setSure, onClose, onMove, onBuild, onGone }) {
  const a = rows.find((x) => x.key === o.area);
  const charging = o.area === 'parking' ? rows.find((x) => x.key === 'charging') : null;
  return html`
    <${Head} icon=${a?.icon || ICONS[o.area]} title=${a?.label || o.title} sub=${o.works || o.what} tone=${o.works ? 'warn' : ''} onClose=${onClose} />
    ${!o.building && a && html`<${NextLevel} a=${a} onBuild=${onBuild} />`}
    ${!o.building && charging && html`<div class="dp-sub"><span class="dp-icon"><${Icon} name=${charging.icon} size=${16} /></span><b>${charging.label}</b></div><${NextLevel} a=${charging} />`}
    ${!o.building && html`<${Upgrades} list=${[...(a?.upgrades || []), ...(charging?.upgrades || [])]} />`}
    ${!o.building && html`<div class="dp-foot">
      <${Btn} kind="ghost" icon="open_with" label=${tr('Move')} onClick=${onMove} />
      <${Btn} kind=${sure ? 'danger' : 'ghost'} icon="close" label=${sure ? tr('Sure? Pull it down') : tr('Pull down')} onClick=${async () => {
        if (!sure) return setSure(true);
        const done = await actMore('depot_demolish', { id: o.id });
        if (done?.ok) onGone?.();
        setSure(false);
      }} />
    </div>`}`;
}

/** The land to buy beside the yard: what it is, its price. */
function LandCard({ land, onClose }) {
  return html`
    <${Head} icon="add" title=${tr('More land')} sub=${land ? land.tip : tr('There is no more land to buy here.')} onClose=${onClose} />
    ${land && html`<div class="dp-foot">
      <${Btn} kind="primary" icon="add" label=${`${tr('Buy it')} · ${land.cost}`} onClick=${async () => {
        const done = await actMore('depot_land');
        if (done?.ok) onClose();
      }} />
    </div>`}`;
}

/** A bus picked on the yard: its card, and the jobs the workshop can do on it (`wsBus`, from
 * the workshop's list; none for a bus in the workshop already). */
function BusCard({ bus, ws, onClose }) {
  return html`
    <${Head} icon="directions_bus" title=${ws ? `${ws.number}  ·  ${ws.name}` : bus.number} sub=${ws ? ws.sub : bus.tip} tone=${ws?.tone === 'warn' ? 'warn' : ''} onClose=${onClose} />
    ${ws && html`<div class="dp-cond" title=${ws.cond.tip}><small>${ws.cond.tip}</small><${Meter} frac=${ws.cond.frac} tone=${ws.cond.tone} h=${5} /></div>`}
    ${ws?.jobs?.length > 0 && html`<div class="dp-foot">${ws.jobs.map((k) => html`<${Btn} kind="ghost" label=${k.label} title=${k.tip} onClick=${() => actMore('depot_order', { id: bus.id, kind: k.kind })} />`)}</div>`}`;
}
