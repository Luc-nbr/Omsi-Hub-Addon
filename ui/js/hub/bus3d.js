// The bus in 3D on the bus step: the model the player's own openOMSI writes (`hub_bus3d`, its
// `--export-glb`) in the livery and with the bus options chosen, drawn here with three.js
// (`../../vendor/three.js`): turned with the mouse (or a finger), zoomed with the wheel, moved
// with the right button, and turning by itself until it is touched. The views from the front,
// the side and the back, and the view back to the start. While openOMSI writes a look the stage
// says so (the first time a while; then the look is kept); a look it cannot write says why.

import { html, useEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { call, bytesUrl } from '../api.js';
import { dark } from './theme.js';
import { Icon, tr } from './bits.js';

/** three.js, loaded the first time a bus is shown (it is big). */
let three = null;
const loadThree = () => (three ??= import('../../vendor/three.js'));

/** The looks shown, kept while the page lives: model key → the loaded scene. */
const kept = new Map();

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
      const renderer = new T.WebGLRenderer({ antialias: true, alpha: true });
      renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
      renderer.outputColorSpace = T.SRGBColorSpace;
      renderer.toneMapping = T.ACESFilmicToneMapping;
      renderer.shadowMap.enabled = true;
      renderer.shadowMap.type = T.PCFSoftShadowMap;
      el.appendChild(renderer.domElement);
      const scene = new T.Scene();
      const camera = new T.PerspectiveCamera(32, 1, 0.1, 500);
      scene.add(new T.HemisphereLight(0xdfe8ff, dark() ? 0x1a1d28 : 0xb8a88f, dark() ? 1.6 : 2.2));
      const sun = new T.DirectionalLight(0xffffff, 2.4);
      sun.castShadow = true;
      sun.shadow.mapSize.set(2048, 2048);
      scene.add(sun);
      const ground = new T.Mesh(new T.CircleGeometry(1, 64), new T.ShadowMaterial({ opacity: dark() ? 0.45 : 0.22 }));
      ground.rotation.x = -Math.PI / 2;
      ground.receiveShadow = true;
      scene.add(ground);
      const controls = new T.OrbitControls(camera, renderer.domElement);
      controls.enableDamping = true;
      controls.autoRotateSpeed = 1.2;
      controls.addEventListener('start', () => (controls.autoRotate = false));
      const size = () => {
        const w = el.clientWidth || 1;
        const h = el.clientHeight || 1;
        renderer.setSize(w, h, false);
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
      };
      const ro = new ResizeObserver(size);
      ro.observe(el);
      size();
      const place = (model) => {
        // (the bus on the ground, its middle at the origin, the camera at three quarters front)
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
      const show = (model) => {
        model.traverse((o) => {
          if (o.isMesh) o.castShadow = o.receiveShadow = true;
        });
        scene.add(model);
        place(model);
      };
      const have = kept.get(key);
      if (have) show(have.clone());
      else
        new T.GLTFLoader().load(
          bytesUrl('hub_bus3d_glb', { key }),
          (g) => {
            if (gone) return;
            kept.set(key, g.scene);
            show(g.scene.clone());
          },
          undefined,
          (e) => !gone && setErr(String(e?.message || e)),
        );
      const tick = () => {
        raf = requestAnimationFrame(tick);
        controls.autoRotate = controls.autoRotate && spin.current;
        controls.update();
        renderer.render(scene, camera);
      };
      controls.autoRotate = spin.current;
      tick();
      cleanup = () => {
        cancelAnimationFrame(raf);
        ro.disconnect();
        controls.dispose();
        renderer.dispose();
        renderer.domElement.remove();
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
