// The bus in 3D on the bus step: the model the player's own openOMSI writes (`hub_bus3d`, its
// `--export-glb`) in the livery and with the bus options chosen, drawn here with three.js
// (`../../vendor/three.js`): turned with the mouse (or a finger), zoomed with the wheel, moved
// with the right button, and turning by itself until it is touched. The views from the front,
// the side and the back, and the view back to the start. While openOMSI writes a look the stage
// says so (the first time a while; then the look is kept); a look it cannot write says why.
// `photograph` takes the buses' photos for their tiles from the same models (`photos.js`).

import { html, useEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { call, callBytes } from '../api.js';
import { dark } from './theme.js';
import { Icon, tr } from './bits.js';

/** three.js, loaded the first time a bus is shown (it is big). */
let three = null;
const loadThree = () => (three ??= import('../../vendor/three.js'));

/** The looks loaded, kept while the page lives: model key → the promise of its scene (shown as
 * a clone of it). */
const kept = new Map();

/** The model of `key` (`hub_bus3d_glb`), loaded once. */
function loadModel(T, key) {
  if (!kept.has(key)) {
    const p = callBytes('hub_bus3d_glb', { key }).then((bytes) => new T.GLTFLoader().parseAsync(bytes, '')).then((g) => g.scene);
    p.catch(() => kept.delete(key));
    kept.set(key, p);
  }
  return kept.get(key);
}

/** The stage a bus stands on: the sky's and the sun's light and the ground its shadow falls on.
 * `place(model)` puts the bus on it (on the ground, its middle at the origin) and answers where
 * the camera looks from: `{ target, views: { start, front, side, back }, r }`; `dispose()` lets
 * go of what the stage holds of the graphics card (the sun's shadow, the ground). */
function stage(T, scene, night) {
  scene.add(new T.HemisphereLight(0xdfe8ff, night ? 0x1a1d28 : 0xb8a88f, night ? 1.6 : 2.2));
  const sun = new T.DirectionalLight(0xffffff, 2.4);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  scene.add(sun);
  const ground = new T.Mesh(new T.CircleGeometry(1, 64), new T.ShadowMaterial({ opacity: night ? 0.45 : 0.22 }));
  ground.rotation.x = -Math.PI / 2;
  ground.receiveShadow = true;
  scene.add(ground);
  const place = (model) => {
    model.traverse((o) => {
      if (o.isMesh) o.castShadow = o.receiveShadow = true;
    });
    scene.add(model);
    // (the camera at three quarters front to start with)
    const box = new T.Box3().setFromObject(model);
    const mid = box.getCenter(new T.Vector3());
    model.position.sub(new T.Vector3(mid.x, box.min.y, mid.z));
    const dim = box.getSize(new T.Vector3());
    const r = Math.max(dim.x, dim.z, dim.y) || 10;
    ground.scale.setScalar(r * 0.75);
    sun.position.set(r, r * 1.6, r * 0.8);
    Object.assign(sun.shadow.camera, { left: -r, right: r, top: r, bottom: -r, near: 0.1, far: r * 5 });
    sun.shadow.camera.updateProjectionMatrix();
    const target = new T.Vector3(0, dim.y * 0.42, 0);
    const views = {
      start: new T.Vector3(r * 0.9, dim.y * 0.9, r * 0.95),
      front: new T.Vector3(0, dim.y * 0.6, r * 1.3),
      side: new T.Vector3(r * 1.35, dim.y * 0.55, 0),
      back: new T.Vector3(0, dim.y * 0.6, -r * 1.3),
    };
    // (a bus long along x: its front and back are along x)
    if (dim.x > dim.z) {
      views.front = new T.Vector3(r * 1.3, dim.y * 0.6, 0);
      views.side = new T.Vector3(0, dim.y * 0.55, r * 1.35);
      views.back = new T.Vector3(-r * 1.3, dim.y * 0.6, 0);
    }
    return { target, views, r };
  };
  const dispose = () => {
    sun.shadow.dispose();
    ground.geometry.dispose();
    ground.material.dispose();
  };
  return { place, dispose };
}

/** A renderer as both draw with. */
function renderer(T, opts = {}) {
  const r = new T.WebGLRenderer({ antialias: true, alpha: true, ...opts });
  r.outputColorSpace = T.SRGBColorSpace;
  r.toneMapping = T.ACESFilmicToneMapping;
  r.shadowMap.enabled = true;
  return r;
}

/** The renderer the photos are taken with (one, kept). */
let photoGl = null;

/** A photo of the model of `key` for the buses' tiles: the bus at three quarters front on a
 * clear ground, `w` × `h` (the tiles' 560 × 350), as a PNG Blob. */
export async function photograph(key, w = 560, h = 350) {
  const T = await loadThree();
  const model = (await loadModel(T, key)).clone();
  photoGl ??= renderer(T, { preserveDrawingBuffer: true });
  const gl = photoGl;
  gl.setPixelRatio(1);
  gl.setSize(w, h, false);
  gl.setClearColor(0x000000, 0);
  const scene = new T.Scene();
  const set = stage(T, scene, false);
  const { views, target } = set.place(model);
  const camera = new T.PerspectiveCamera(32, w / h, 0.1, 500);
  // (from where the viewer starts, a little higher, as near as the whole bus fills the photo:
  // its box's corners in the camera's view, the camera moved until they are in the middle and
  // the farthest is at the edge)
  const box = new T.Box3().setFromObject(model);
  const look = box.getCenter(new T.Vector3());
  const r = box.getSize(new T.Vector3()).length();
  const dir = views.start.clone().sub(target).setY(0).normalize().setY(0.2).normalize();
  const corners = [0, 1, 2, 3, 4, 5, 6, 7].map((i) => new T.Vector3(i & 1 ? box.max.x : box.min.x, i & 2 ? box.max.y : box.min.y, i & 4 ? box.max.z : box.min.z));
  const tanY = Math.tan((camera.fov * Math.PI) / 360);
  let far = r * 2;
  for (let i = 0; i < 8; i++) {
    camera.position.copy(dir).multiplyScalar(far).add(look);
    camera.lookAt(look);
    camera.updateMatrixWorld();
    const seen = corners.map((c) => c.clone().project(camera));
    const [x0, x1] = [Math.min(...seen.map((c) => c.x)), Math.max(...seen.map((c) => c.x))];
    const [y0, y1] = [Math.min(...seen.map((c) => c.y)), Math.max(...seen.map((c) => c.y))];
    const right = new T.Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
    const up = new T.Vector3().setFromMatrixColumn(camera.matrixWorld, 1);
    look.addScaledVector(right, ((x0 + x1) / 2) * far * tanY * camera.aspect).addScaledVector(up, ((y0 + y1) / 2) * far * tanY);
    far *= Math.max((x1 - x0) / 2, (y1 - y0) / 2) / 0.9;
  }
  camera.position.copy(dir).multiplyScalar(far).add(look);
  camera.lookAt(look);
  gl.render(scene, camera);
  const blob = await new Promise((done) => gl.domElement.toBlob(done, 'image/png'));
  set.dispose();
  gl.renderLists.dispose();
  if (!blob) throw new Error('the photo could not be taken');
  return blob;
}

/** The look's state, asked for again while openOMSI writes it (`retry` above 0: a failed look
 * tried again). */
function useLook(bus, paint, optionsRev, retry) {
  const [state, setState] = useState({ state: 'making' });
  useEffect(() => {
    let live = true;
    let timer = 0;
    const ask = (again) =>
      call('hub_bus3d', { bus, paint, retry: again })
        .then((r) => {
          if (!live) return;
          setState(r);
          if (r.state === 'making') timer = setTimeout(() => ask(false), 1500);
        })
        .catch((e) => live && setState({ state: 'failed', error: String(e?.message || e) }));
    setState({ state: 'making' });
    ask(retry > 0);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [bus, paint, optionsRev, retry]);
  return state;
}

/** The scene: the renderer in `host`, the model of `key`, the camera and its controls. */
function useScene(host, key, spin) {
  const view = useRef(null);
  const [err, setErr] = useState('');
  useEffect(() => {
    if (!key || !host.current) return;
    let gone = false;
    let raf = 0;
    let cleanup = () => {};
    loadThree().then((T) => {
      if (gone) return;
      const el = host.current;
      const gl = renderer(T);
      gl.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
      el.appendChild(gl.domElement);
      const scene = new T.Scene();
      const camera = new T.PerspectiveCamera(32, 1, 0.1, 500);
      const set = stage(T, scene, dark());
      const controls = new T.OrbitControls(camera, gl.domElement);
      controls.enableDamping = true;
      controls.autoRotateSpeed = 1.2;
      controls.addEventListener('start', () => (controls.autoRotate = false));
      const size = () => {
        const w = el.clientWidth || 1;
        const h = el.clientHeight || 1;
        gl.setSize(w, h, false);
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
      };
      const ro = new ResizeObserver(size);
      ro.observe(el);
      size();
      const show = (model) => {
        const { target, views, r } = set.place(model);
        controls.minDistance = r * 0.35;
        controls.maxDistance = r * 4;
        controls.maxPolarAngle = Math.PI * 0.495;
        const go = (name) => {
          camera.position.copy(views[name]);
          controls.target.copy(target);
          controls.update();
        };
        go('start');
        view.current = { go, controls };
      };
      loadModel(T, key).then(
        (m) => !gone && show(m.clone()),
        (e) => !gone && setErr(String(e?.message || e)),
      );
      const tick = () => {
        raf = requestAnimationFrame(tick);
        controls.autoRotate = controls.autoRotate && spin.current;
        controls.update();
        gl.render(scene, camera);
      };
      controls.autoRotate = spin.current;
      tick();
      cleanup = () => {
        cancelAnimationFrame(raf);
        ro.disconnect();
        controls.dispose();
        set.dispose();
        gl.dispose();
        gl.domElement.remove();
        view.current = null;
      };
    }, (e) => !gone && setErr(String(e?.message || e)));
    return () => {
      gone = true;
      cleanup();
    };
  }, [key]);
  return [view, err];
}

/** The bus `bus` in `paint` in 3D, with the options chosen (`optionsRev` changes with them). */
export function Bus3D({ bus, paint, optionsRev }) {
  const [retry, setRetry] = useState(0);
  const look = useLook(bus, paint || '', optionsRev, retry);
  const host = useRef(null);
  const spinning = useRef(true);
  const [spin, setSpin] = useState(true);
  spinning.current = spin;
  const [view, err] = useScene(host, look.state === 'ready' ? look.key : null, spinning);
  const go = (name) => {
    setSpin(false);
    view.current?.go(name);
  };
  const toggle = () => {
    setSpin(!spin);
    if (view.current) view.current.controls.autoRotate = !spin;
  };
  const failed = look.state === 'failed' ? look.error : err;
  return html`<div class="viewer bus3d" id="bus3d">
    <div class="canvas" ref=${host} style="position:absolute;inset:0"></div>
    ${look.state === 'making' && html`<div class="state" style="position:absolute;inset:0;display:grid;place-content:center;gap:10px;justify-items:center;text-align:center;padding:24px">
      <span class="spinner"></span><span>${tr('openOMSI is writing the bus in 3D… The first time takes a while; then it is kept.')}</span></div>`}
    ${failed && html`<div class="state" style="position:absolute;inset:0;display:grid;place-content:center;gap:10px;justify-items:center;text-align:center;padding:24px">
      <${Icon} name="view_in_ar" size=${30} /><span>${tr('The bus could not be shown in 3D: %{error}', { error: tr(failed) })}</span>
      <button type="button" class="hub-btn small" onClick=${() => setRetry(retry + 1)}><${Icon} name="refresh" size=${16} />${tr('Try again')}</button></div>`}
    ${look.state === 'ready' && !err && html`<div class="hint" style="position:absolute;display:flex;gap:6px;flex-wrap:wrap">
      ${[['front', 'Front'], ['side', 'Side'], ['back', 'Back'], ['start', 'Reset the view']].map(([k, label]) => html`<button type="button" class="hub-btn small" onClick=${() => go(k)}>${tr(label)}</button>`)}
      <button type="button" class=${'hub-btn small' + (spin ? ' on' : '')} title=${tr('Turn by itself')} onClick=${toggle}><${Icon} name=${spin ? 'pause' : '360'} size=${16} />${tr('Turn by itself')}</button>
    </div>`}
  </div>`;
}
