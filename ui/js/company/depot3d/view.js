// The depot in 3D on the depot page: the yard from `scene.js` drawn by `renderer.js`, its
// labels floating over the buildings, the buses to pick (clicked: picked for the workshop's
// list, clicked again: let go), the modules to move or pull down (clicked: their card). In the
// build mode (`placing`: a module of an area to put down, or one to move) its ghost follows the
// pointer on the yard's grid, green where it fits (`yard.js`), R turns it, a click puts it down,
// Escape or a right click stops. Drag to turn the yard round (in 2D: to move it), right-drag (or
// Shift) to move it,
// scroll to zoom; the camera is kept for when the page comes back. Each bus is drawn as its own
// model in its livery once that is there (`skins.js`), until then as the plain one. The staff at
// work walk about the yard (`people.js`, their figures `figures.js`); with the animations off
// they stand still. How finely it is drawn is chosen in its menu (`quality.js`). Without WebGL2
// the page draws its plan from above instead (`onFail`).

import { html, useEffect, useRef, useState, tr, Icon, IconBtn, Btn, ui, setUi } from '../kit.js';
import { actMore } from '../more.js';
import { project } from '../../hub/bus3d/math.js';
import { buildScene, AREAS, moduleUnder, ghostMesh, gridMesh } from './scene.js';
import { rectOf, whyNot } from './yard.js';
import { busMesh, instances, pickBus, skinned } from './bus.js';
import { skin, skinKey } from './skins.js';
import { DepotRenderer, VIEW } from './renderer.js';
import { palette, rgb01 } from './palette.js';
import { LEVELS, chooseQuality, chosenQuality, levelOf, stepDown } from './quality.js';
import { Crowd } from './people.js';
import { figure } from './figures.js';

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

/** The areas' icons, as the Build list has them (`depot::Area::icon`), for an area it does not
 * list. */
const ICONS = { parking: 'local_parking', workshop: 'construction', wash: 'water_drop', fuel: 'inventory_2', charging: 'bolt', offices: 'groups' };

const still = () => document.documentElement.classList.contains('hub-still');

export function Depot3D({ p, names, w, h, selected, hover, onPick, onFail, placing, onPlaced, flat }) {
  const canvas = useRef(null);
  const tags = useRef(null);
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
  const [sel, setSel] = useState(null);
  const [sure, setSure] = useState(false);
  const pointer = useRef(null);
  const [skins, setSkins] = useState(0);
  const crowd = useRef(null);
  const auto = useRef(quality === 'auto');
  auto.current = quality === 'auto';
  const pal = palette();
  const key = JSON.stringify([p, pal.dark, pal.accent]);

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
    view.setBusModel(busMesh(pal));
    view.resize(w, h);
    const kept = ui('depot').cam;
    view.aim(kept || VIEW, true);
    // scrolling zooms towards the point under the pointer
    const wheel = (e) => {
      e.preventDefault();
      const g = view.goal;
      const zoom = Math.max(0.15, Math.min(1.45, g.zoom * Math.exp(e.deltaY * 0.0012)));
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

  // the yard again when the depot (or the look) changed
  useEffect(() => {
    const view = r.current;
    if (!view) return;
    const s = buildScene(p, pal, LEVELS[level]);
    scene.current = s;
    view.setYard(s.mesh, s.bounds, pal);
    buses(true);
    staff(view, s);
    setBuilt((n) => n + 1);
  }, [key, level]);

  // the buses' own models, as they come
  useEffect(() => buses(true), [skins]);

  // (the labels drawn again: a picture puts them where they belong)
  useEffect(() => r.current?.invalidate());

  // the picked bus and the one under the pointer
  useEffect(() => buses(false), [selected, under]);

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

  /** The staff about the yard: their figures asked for (each drawn once it is there), walking
   * while the animations are on. */
  function staff(view, s) {
    const c = new Crowd(p.staff || [], s.routes, LEVELS[level].people);
    crowd.current = c;
    const modelOf = (role) => view.figureModel(role);
    const speedOf = (role) => view.figureModel(role)?.speed || 0.65;
    const show = () => view.setFigures(c.instances(modelOf));
    for (const role of c.roles())
      figure(role).then((m) => {
        if (!m || r.current !== view || crowd.current !== c) return;
        view.setFigureModel(role, m);
        show();
      });
    show();
    view.setTicker(c.count && !still() ? (dt) => (c.step(dt, speedOf), show()) : null);
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
    setSel(null);
    setGhost(null);
    view.setOverlay('ghost', null);
    if (!placing) {
      view.setOverlay('grid', null);
      return undefined;
    }
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

  /** The ghost where the pointer is: its cells on the grid, whether it fits. */
  function aimGhost(e) {
    const view = r.current;
    const sc = scene.current;
    const L = p.layout;
    if (!view || !sc || !placing) return;
    const [px, py] = at(e);
    const hit = view.ground(px, py);
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

  /** The module under a point of the canvas. */
  function moduleAt(e) {
    const view = r.current;
    const sc = scene.current;
    if (!view || !sc) return null;
    const [px, py] = at(e);
    const hit = view.ground(px, py);
    if (!hit) return null;
    const b = sc.bounds;
    return moduleUnder(sc.picks, hit[0] + (b.x0 + b.x1) / 2, hit[1] + (b.z0 + b.z1) / 2);
  }

  /** The labels over what they name, where the camera now shows it. */
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
  }

  const keep = () => {
    clearTimeout(keep.t);
    keep.t = setTimeout(() => r.current && setUi('depot', { cam: { ...r.current.goal } }), 400);
  };

  const at = (e) => {
    const b = canvas.current.getBoundingClientRect();
    return [e.clientX - b.left, e.clientY - b.top];
  };
  const busAt = (e) => {
    const view = r.current;
    const s = scene.current;
    if (!view || !s) return null;
    const [x, y] = at(e);
    const { o, d } = view.ray(x, y);
    const k = pickBus(s.buses, o, d);
    return k < 0 ? null : s.buses[k];
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
    if (g.moved < 4) return;
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
    if (g.moved < 4 && placing) {
      if (e.button === 2) onPlaced?.();
      else if (e.button === 0) putDown();
    } else if (g.moved < 4 && e.button === 0) {
      const b = busAt(e);
      if (b) {
        onPick?.(b.id);
        setSel(null);
      } else {
        const o = moduleAt(e);
        setSel(o && o.id !== sel ? o.id : null);
        setSure(false);
      }
    } else keep();
  };

  const aim = (patch) => {
    r.current?.aim(patch, still());
    keep();
  };
  const s = scene.current;
  const tagFor = (t) => {
    const k = t.key;
    const n = { icon: ICONS[k], label: k === 'land' ? tr('More land') : t.head, ...names[k] };
    const lit = hover && t.key === hover;
    if (t.works) return html`<div class=${'dp-tag works' + (lit ? ' lit' : '')}><${Icon} name="construction" size=${13} /><span>${n.label}<small>${t.works}</small></span></div>`;
    if (!t.built) return html`<div class=${'dp-tag plot' + (lit ? ' lit' : '')}><${Icon} name="add" size=${13} /><span>${n.label}</span></div>`;
    return html`<div class=${'dp-tag' + (lit ? ' lit' : '')}><${Icon} name=${n.icon || 'info'} size=${13} /><span>${n.label}</span></div>`;
  };
  const tip = under && s?.buses.find((b) => b.id === under);
  const card = sel && s?.picks.find((o) => o.id === sel);
  const hint = placing ? tr('Click to put it down, R to turn it, Esc to stop') : flat ? tr('Drag to move, scroll to zoom') : tr('Drag to turn, right-drag to move, scroll to zoom');
  return html`<div class="cm-plan cm-plan3d" data-buses=${s ? s.buses.length : 0} data-people=${crowd.current?.count || 0} style=${`width:${w}px;height:${h}px`}>
    <canvas key=${level} ref=${canvas} class=${under ? 'on-bus' : ''} style=${`width:${w}px;height:${h}px`} onPointerDown=${down} onPointerMove=${move} onPointerUp=${up} onPointerLeave=${() => !drag.current && under !== null && setUnder(null)} onContextMenu=${(e) => e.preventDefault()}></canvas>
    <div class="dp-tags" ref=${tags}>${s?.labels.map(tagFor)}</div>
    ${tip && !placing && html`<div class="dp-tip">${tip.tip}</div>`}
    ${placing && html`<div class="dp-place">
      <span class=${'dp-why' + (ghost?.why ? ' no' : '')}>${ghost?.why ? tr(WHY[ghost.why]) : placing.move ? tr('Move it where it fits: it costs nothing.') : tr('Put it down where it fits, its lane kept free.')}</span>
      <${Btn} kind="ghost" icon="refresh" label=${tr('Turn')} onClick=${() => setRot((k) => (k + 1) % 4)} />
      <${Btn} kind="ghost" icon="close" label=${tr('Stop')} onClick=${() => onPlaced?.()} />
    </div>`}
    ${card && !placing && html`<div class="dp-card">
      <b>${card.title}</b>
      <span>${card.works || card.what}</span>
      ${!card.building && html`<div class="row">
        <${Btn} kind="ghost" icon="open_with" label=${tr('Move')} onClick=${() => setUi('depot', { placing: { area: card.area, move: card.id, rot: card.rot } })} />
        <${Btn} kind=${sure ? 'danger' : 'ghost'} icon="close" label=${sure ? tr('Sure? Pull it down') : tr('Pull down')} onClick=${async () => {
          if (!sure) return setSure(true);
          const done = await actMore('depot_demolish', { id: card.id });
          if (done?.ok) setSel(null);
          setSure(false);
        }} />
      </div>`}
    </div>`}
    <div class="dp-tools">
      <${IconBtn} icon="zoom_in" size=${17} title=${tr('Zoom in')} onClick=${() => aim({ zoom: (r.current?.goal.zoom || 1) / 1.35 })} />
      <${IconBtn} icon="zoom_out" size=${17} title=${tr('Zoom out')} onClick=${() => aim({ zoom: (r.current?.goal.zoom || 1) * 1.35 })} />
      <${IconBtn} icon="north" size=${17} title=${tr('From above')} onClick=${() => aim({ yaw: 0, pitch: 1.48 })} />
      <${IconBtn} icon="restart_alt" size=${17} title=${tr('Reset the view')} onClick=${() => aim({ ...VIEW })} />
      <${IconBtn} icon="tune" size=${17} title=${tr('Graphics')} onClick=${() => setMenu(!menu)} />
    </div>
    ${menu && html`<div class="dp-menu" role="menu">
      <div class="co-caps">${tr('Graphics')}</div>
      ${QUALITY.map(([q, label, under]) => html`<button type="button" role="menuitemradio" aria-checked=${q === quality} class=${q === quality ? 'on' : ''} onClick=${() => {
        chooseQuality(q);
        setQuality(q);
        setLevel(levelOf(q));
        setMenu(false);
      }}><span>${tr(label)}<small>${q === 'auto' ? tr(NAMES[level]) : tr(under)}</small></span>${q === quality && html`<${Icon} name="check" size=${16} />`}</button>`)}
    </div>`}
    ${!placing && html`<div class="dp-hint">${hint}</div>`}
  </div>`;
}
