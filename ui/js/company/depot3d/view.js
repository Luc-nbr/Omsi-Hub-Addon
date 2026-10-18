// The depot in 3D on the depot page: the yard from `scene.js` drawn by `renderer.js`, its
// labels floating over the buildings, the buses to pick (clicked: picked for the workshop's
// list, clicked again: let go). Drag to turn the yard round, right-drag (or Shift) to move it,
// scroll to zoom; the camera is kept for when the page comes back. How finely it is drawn is
// chosen in its menu (`quality.js`). Without WebGL2 the page draws its plan from above instead
// (`onFail`).

import { html, useEffect, useRef, useState, tr, Icon, IconBtn, ui, setUi } from '../kit.js';
import { project } from '../../hub/bus3d/math.js';
import { buildScene, AREAS } from './scene.js';
import { busMesh, instances, pickBus } from './bus.js';
import { DepotRenderer, VIEW } from './renderer.js';
import { palette, rgb01 } from './palette.js';
import { LEVELS, chooseQuality, chosenQuality, levelOf, stepDown } from './quality.js';

/** The menu's levels: what is chosen, its name, and what it gives up. */
const QUALITY = [
  ['auto', 'Automatic', ''],
  ['high', 'High', 'Shadows, smooth edges, full sharpness'],
  ['medium', 'Medium', 'Softer shadows, a little less sharp'],
  ['low', 'Low', 'No shadows or trees: for slower computers'],
];
const NAMES = { high: 'Now: high', medium: 'Now: medium', low: 'Now: low' };

/** The areas' icons, as the Build list has them (`depot::Area::icon`), for an area it does not
 * list. */
const ICONS = { parking: 'local_parking', workshop: 'construction', wash: 'water_drop', fuel: 'inventory_2', charging: 'bolt', offices: 'groups' };

const still = () => document.documentElement.classList.contains('hub-still');

export function Depot3D({ p, names, w, h, selected, hover, onPick, onFail }) {
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
    view.setBuses(instances(s.buses, rgb01(p.colour), { picked: selected, hover: under }), true);
    setBuilt((n) => n + 1);
  }, [key, level]);

  // (the labels drawn again: a picture puts them where they belong)
  useEffect(() => r.current?.invalidate());

  // the picked bus and the one under the pointer
  useEffect(() => {
    const s = scene.current;
    if (r.current && s) r.current.setBuses(instances(s.buses, rgb01(p.colour), { picked: selected, hover: under }), false);
  }, [selected, under]);

  useEffect(() => r.current?.highlight(AREAS[hover] || 0), [hover]);

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
    if (g.pan) {
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
    if (g.moved < 4 && e.button === 0) {
      const b = busAt(e);
      if (b) onPick?.(b.id);
    } else keep();
  };

  const aim = (patch) => {
    r.current?.aim(patch, still());
    keep();
  };
  const s = scene.current;
  const tagFor = (t) => {
    const k = t.key === 'parking_more' ? 'parking' : t.key;
    const n = { icon: ICONS[k], label: t.head, ...names[k] };
    const lit = hover && (t.key === hover || (t.key === 'parking_more' && hover === 'parking'));
    if (t.works) return html`<div class=${'dp-tag works' + (lit ? ' lit' : '')}><${Icon} name="construction" size=${13} /><span>${n.label}<small>${t.works}</small></span></div>`;
    if (!t.built || t.key === 'parking_more') return html`<div class=${'dp-tag plot' + (lit ? ' lit' : '')}><${Icon} name="add" size=${13} /><span>${n.label}</span></div>`;
    return html`<div class=${'dp-tag' + (lit ? ' lit' : '')}><${Icon} name=${n.icon || 'info'} size=${13} /><span>${n.label}</span></div>`;
  };
  const tip = under && s?.buses.find((b) => b.id === under);
  return html`<div class="cm-plan cm-plan3d" data-buses=${s ? s.buses.length : 0} style=${`width:${w}px;height:${h}px`}>
    <canvas key=${level} ref=${canvas} class=${under ? 'on-bus' : ''} style=${`width:${w}px;height:${h}px`} onPointerDown=${down} onPointerMove=${move} onPointerUp=${up} onPointerLeave=${() => !drag.current && under !== null && setUnder(null)} onContextMenu=${(e) => e.preventDefault()}></canvas>
    <div class="dp-tags" ref=${tags}>${s?.labels.map(tagFor)}</div>
    ${tip && html`<div class="dp-tip">${tip.tip}</div>`}
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
    <div class="dp-hint">${tr('Drag to turn, right-drag to move, scroll to zoom')}</div>
  </div>`;
}
