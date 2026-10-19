// The 3D depot's renderer: one WebGL2 context on the page's own canvas, light enough for a
// yard of a hundred buses and more. The yard is one buffer drawn in one call, the buses one
// model drawn once for all of them (instanced); a shadow map of the sun, made again only when
// the yard or its buses change, not when the camera moves. It draws only when something
// changes (the camera, the buses, the look), as the 3D view does (`hub/bus3d/engine.js`).
//
// A bus whose own depot model is there (`skins.js`) is drawn as that: each model in its livery
// one buffer, one texture and one draw for all the buses wearing it.

import { inverse, lookAt, mul, ortho, perspective, project, unit } from '../../hub/bus3d/math.js';
import { STRIDE } from './mesh.js';
import { INSTANCE, SKINNED } from './bus.js';

const FOV = (30 * Math.PI) / 180;

const LIGHT_FS = `
uniform highp sampler2DShadow uShadow;
uniform vec3 uSun, uSunCol, uSky, uGround;
uniform float uNight, uTexel, uShadows;
float shade(vec4 s) {
  if (uShadows < 0.5) return 1.0;
  vec3 p = s.xyz / s.w * 0.5 + 0.5;
  if (p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0 || p.z > 1.0) return 1.0;
  float sum = 0.0;
  for (int i = -1; i <= 1; i++)
    for (int j = -1; j <= 1; j++) sum += texture(uShadow, vec3(p.xy + vec2(float(i), float(j)) * uTexel, p.z - 0.0008));
  return sum / 9.0;
}
vec3 lit(vec3 albedo, vec3 n, vec4 s, float glow) {
  vec3 a = pow(albedo, vec3(2.2));
  float d = max(dot(n, uSun), 0.0);
  float sh = d > 0.0 ? shade(s) : 0.0;
  vec3 amb = mix(uGround, uSky, n.y * 0.5 + 0.5);
  vec3 c = a * (amb + uSunCol * d * sh) + a * glow * uNight * 1.6;
  c = c / (1.0 + c * 0.12);
  return pow(c, vec3(1.0 / 2.2));
}`;

const YARD_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in vec3 aCol;
layout(location=3) in float aArea;
layout(location=4) in float aGlow;
uniform mat4 uVP, uLight;
out vec3 vCol, vNrm;
out vec4 vSh;
out float vArea, vGlow;
void main() {
  vCol = aCol; vNrm = aNrm; vArea = aArea; vGlow = aGlow;
  vSh = uLight * vec4(aPos, 1.0);
  gl_Position = uVP * vec4(aPos, 1.0);
}`;

const YARD_FS = `#version 300 es
precision highp float;
${LIGHT_FS}
uniform vec3 uAccent;
uniform float uHi;
uniform vec4 uTint;
uniform float uAlpha;
in vec3 vCol, vNrm;
in vec4 vSh;
in float vArea, vGlow;
out vec4 o;
void main() {
  vec3 c = lit(vCol, normalize(vNrm), vSh, vGlow);
  if (uHi > 0.5 && abs(vArea - uHi) < 0.25) c = mix(c, uAccent, 0.42);
  c = mix(c, uTint.rgb, uTint.a);
  o = vec4(c * uAlpha, uAlpha);
}`;

const BUS_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in vec3 aCol;
layout(location=3) in float aPaint;
layout(location=4) in float aPart;
layout(location=5) in vec4 aI0;
layout(location=6) in vec4 aI1;
layout(location=7) in vec3 aI2;
uniform mat4 uVP, uLight;
uniform float uShadowPass;
out vec3 vCol, vNrm;
out vec4 vSh;
flat out float vPart, vState;
void main() {
  float state = aI1.w;
  bool ring = aPart > 1.5 && aPart < 2.5;
  bool hide = (aPart > 0.5 && aPart < 1.5 && aI2.y < 0.5) || (ring && (state < 0.5 || uShadowPass > 0.5)) || (!ring && aI2.z > 0.5);
  vec3 p = aPos;
  if (!ring) p.y *= aI2.x / 3.1;
  vec2 l = vec2(p.x, p.z * aI0.w);
  float c = cos(aI0.z), s = sin(aI0.z);
  vec3 w = vec3(aI0.x + c * l.x + s * l.y, p.y, aI0.y - s * l.x + c * l.y);
  vNrm = vec3(c * aNrm.x + s * aNrm.z, aNrm.y, -s * aNrm.x + c * aNrm.z);
  vCol = mix(aCol, aI1.rgb, aPaint);
  vPart = aPart; vState = state;
  vSh = uLight * vec4(w, 1.0);
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : uVP * vec4(w, 1.0);
}`;

const BUS_FS = `#version 300 es
precision highp float;
${LIGHT_FS}
uniform vec3 uAccent, uWarn;
in vec3 vCol, vNrm;
in vec4 vSh;
flat in float vPart, vState;
out vec4 o;
void main() {
  if (vPart > 1.5 && vPart < 2.5) {
    o = vec4(vState > 2.5 ? uWarn : vState > 1.5 ? mix(uAccent, vec3(1.0), 0.45) : uAccent, 1.0);
    return;
  }
  vec3 c = lit(vCol, normalize(vNrm), vSh, vPart > 2.5 ? 1.0 : 0.0);
  if (vState > 0.5 && vState < 2.5) c = mix(c, vec3(1.0), vState > 1.5 ? 0.12 : 0.2);
  o = vec4(c, 1.0);
}`;

const SKIN_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in vec2 aUv;
layout(location=3) in vec4 aI;
uniform mat4 uVP, uLight;
out vec3 vNrm;
out vec2 vUv;
out vec4 vSh;
flat out float vState;
void main() {
  float c = cos(aI.z), s = sin(aI.z);
  vec3 w = vec3(aI.x + c * aPos.x + s * aPos.z, aPos.y, aI.y - s * aPos.x + c * aPos.z);
  vNrm = vec3(c * aNrm.x + s * aNrm.z, aNrm.y, -s * aNrm.x + c * aNrm.z);
  vUv = aUv;
  vState = aI.w;
  vSh = uLight * vec4(w, 1.0);
  gl_Position = uVP * vec4(w, 1.0);
}`;

const SKIN_FS = `#version 300 es
precision highp float;
${LIGHT_FS}
uniform sampler2D uTex;
in vec3 vNrm;
in vec2 vUv;
in vec4 vSh;
flat in float vState;
out vec4 o;
void main() {
  vec3 c = lit(texture(uTex, vUv).rgb, normalize(vNrm), vSh, 0.0);
  if (vState > 0.5 && vState < 2.5) c = mix(c, vec3(1.0), vState > 1.5 ? 0.12 : 0.2);
  o = vec4(c, 1.0);
}`;

const DEPTH_FS = `#version 300 es
precision mediump float;
out vec4 o;
void main() { o = vec4(1.0); }`;

function compile(gl, vs, fs) {
  const make = (type, src) => {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
    return s;
  };
  const p = gl.createProgram();
  gl.attachShader(p, make(gl.VERTEX_SHADER, vs));
  gl.attachShader(p, make(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {};
  const n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let k = 0; k < n; k++) {
    const name = gl.getActiveUniform(p, k).name;
    u[name] = gl.getUniformLocation(p, name);
  }
  return { p, u };
}

/** The camera round the yard: turned (`yaw`), tilted (`pitch`), how far, looking at (tx, tz). */
export const VIEW = { yaw: -0.5, pitch: 0.78, zoom: 0.74, tx: 0, tz: 4 };
const PITCH = [0.42, 1.48];
const ZOOM = [0.15, 1.45];

export class DepotRenderer {
  /** `level` is how finely to draw (`quality.js`); `onFrame(viewProj, w, h)` after every
   * picture (the labels follow it); `onSlow` when the pictures come too slowly while the
   * camera moves; `onLost` when the context goes. False from `ok` when there is no WebGL2. */
  constructor(canvas, level, { onFrame, onSlow, onLost } = {}) {
    this.canvas = canvas;
    this.level = level;
    this.onFrame = onFrame;
    this.onSlow = onSlow;
    this.times = [];
    this.gl = canvas.getContext('webgl2', { antialias: level.msaa, alpha: true, premultipliedAlpha: true, powerPreference: 'default' });
    this.ok = !!this.gl;
    this.cam = { ...VIEW };
    this.goal = { ...VIEW };
    this.size = { w: 1, h: 1, dpr: 1 };
    this.dirtyShadow = true;
    this.hi = 0;
    if (!this.ok) return;
    canvas.addEventListener('webglcontextlost', (e) => {
      e.preventDefault();
      this.ok = false;
      // (not when we let it go ourselves: another level, the page left)
      if (!this.gone) onLost?.();
    });
    const gl = this.gl;
    this.yard = compile(gl, YARD_VS, YARD_FS);
    this.yardDepth = compile(gl, YARD_VS, DEPTH_FS);
    this.bus = compile(gl, BUS_VS, BUS_FS);
    this.busDepth = compile(gl, BUS_VS, DEPTH_FS);
    this.skin = compile(gl, SKIN_VS, SKIN_FS);
    this.skinDepth = compile(gl, SKIN_VS, DEPTH_FS);
    /** The buses' own models: `key -> { vao, count, tex, inst, instances }`. */
    this.skins = new Map();
    const SHADOW = Math.max(1, level.shadow);
    this.shadowSize = SHADOW;
    this.shadowTex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.shadowTex);
    gl.texStorage2D(gl.TEXTURE_2D, 1, gl.DEPTH_COMPONENT24, SHADOW, SHADOW);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_COMPARE_MODE, gl.COMPARE_REF_TO_TEXTURE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_COMPARE_FUNC, gl.LEQUAL);
    this.shadowFb = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.shadowFb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, this.shadowTex, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  /** The yard to draw (`scene.js`'s mesh, bounds) in the palette's light. */
  setYard(mesh, bounds, pal) {
    if (!this.ok) return;
    const gl = this.gl;
    this.pal = pal;
    this.bounds = bounds;
    this.yardVao = this.yardVao || gl.createVertexArray();
    this.yardBuf = this.yardBuf || gl.createBuffer();
    gl.bindVertexArray(this.yardVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.yardBuf);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.floats(), gl.STATIC_DRAW);
    this.vertexLayout();
    gl.bindVertexArray(null);
    this.yardCount = mesh.count;
    this.dirtyShadow = true;
    this.invalidate();
  }

  /** The bus model (`bus.js`'s mesh), once. */
  setBusModel(mesh) {
    if (!this.ok) return;
    const gl = this.gl;
    this.busVao = this.busVao || gl.createVertexArray();
    this.busBuf = this.busBuf || gl.createBuffer();
    this.instBuf = this.instBuf || gl.createBuffer();
    gl.bindVertexArray(this.busVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.busBuf);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.floats(), gl.STATIC_DRAW);
    this.vertexLayout();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instBuf);
    const b = INSTANCE * 4;
    for (const [loc, size, off] of [
      [5, 4, 0],
      [6, 4, 16],
      [7, 3, 32],
    ]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, b, off);
      gl.vertexAttribDivisor(loc, 1);
    }
    gl.bindVertexArray(null);
    this.busCount = mesh.count;
    this.invalidate();
  }

  /** The buses' instances (`bus.js`); `moved` when where they stand changed (a new shadow). */
  setBuses(data, moved) {
    if (!this.ok || !this.instBuf) return;
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
    this.instances = data.length / INSTANCE;
    if (moved) this.dirtyShadow = true;
    this.invalidate();
  }

  /** A bus's own model (`skins.js`: its hull and its atlas), once a model. */
  setSkin(key, model, image) {
    if (!this.ok || this.skins.has(key)) return;
    const gl = this.gl;
    const vao = gl.createVertexArray();
    const buf = gl.createBuffer();
    const inst = gl.createBuffer();
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, model.verts, gl.STATIC_DRAW);
    for (const [loc, size, off] of [
      [0, 3, 0],
      [1, 3, 12],
      [2, 2, 24],
    ]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 32, off);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, inst);
    gl.enableVertexAttribArray(3);
    gl.vertexAttribPointer(3, 4, gl.FLOAT, false, SKINNED * 4, 0);
    gl.vertexAttribDivisor(3, 1);
    gl.bindVertexArray(null);
    // (the atlas smaller at the lowest level)
    let pic = image;
    const side = this.level.atlas || image.width;
    if (image.width > side) {
      pic = typeof OffscreenCanvas !== 'undefined' ? new OffscreenCanvas(side, side) : Object.assign(document.createElement('canvas'), { width: side, height: side });
      pic.getContext('2d').drawImage(image, 0, 0, side, side);
    }
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, pic);
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const aniso = gl.getExtension('EXT_texture_filter_anisotropic');
    if (aniso && this.level.msaa) gl.texParameterf(gl.TEXTURE_2D, aniso.TEXTURE_MAX_ANISOTROPY_EXT, 4);
    this.skins.set(key, { vao, count: model.verts.length / 8, tex, inst, instances: 0 });
  }

  /** Which buses are drawn as their own models (`bus.js`'s `skinned`); `moved` as for
   * `setBuses`. */
  setSkinned(groups, moved) {
    if (!this.ok) return;
    const gl = this.gl;
    for (const [key, sk] of this.skins) {
      const data = groups.get(key);
      sk.instances = data ? data.length / SKINNED : 0;
      if (!data) continue;
      gl.bindBuffer(gl.ARRAY_BUFFER, sk.inst);
      gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
    }
    if (moved) this.dirtyShadow = true;
    this.invalidate();
  }

  /** A see-through layer over the yard (the build mode's grid, the ghost of the module being
   * put down): its mesh tinted `tint` ([r, g, b, how much]) at `alpha`; null takes it away. */
  setOverlay(name, mesh, tint = [1, 1, 1, 0], alpha = 0.5) {
    if (!this.ok) return;
    const gl = this.gl;
    this.overlays ??= new Map();
    let o = this.overlays.get(name);
    if (!mesh) {
      if (o) o.count = 0;
      this.invalidate();
      return;
    }
    if (!o) {
      o = { vao: gl.createVertexArray(), buf: gl.createBuffer() };
      gl.bindVertexArray(o.vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, o.buf);
      this.vertexLayout();
      gl.bindVertexArray(null);
      this.overlays.set(name, o);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, o.buf);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.floats(), gl.DYNAMIC_DRAW);
    Object.assign(o, { count: mesh.count, tint, alpha });
    this.invalidate();
  }

  /** Light up an area of the yard (`scene.js`'s AREAS), 0 for none. */
  highlight(area) {
    if (area === this.hi) return;
    this.hi = area;
    this.invalidate();
  }

  vertexLayout() {
    const gl = this.gl;
    const b = STRIDE * 4;
    for (const [loc, size, off] of [
      [0, 3, 0],
      [1, 3, 12],
      [2, 3, 24],
      [3, 1, 36],
      [4, 1, 40],
    ]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, b, off);
    }
  }

  resize(w, h) {
    const dpr = Math.min(this.level.dpr, window.devicePixelRatio || 1);
    if (w === this.size.w && h === this.size.h && dpr === this.size.dpr) return;
    this.size = { w, h, dpr };
    this.canvas.width = Math.max(1, Math.round(w * dpr));
    this.canvas.height = Math.max(1, Math.round(h * dpr));
    this.invalidate();
  }

  // --- the camera ---------------------------------------------------------------------------

  /** Where the camera should go; it glides there (at once when `still`). */
  aim(patch, still = false) {
    const g = { ...this.goal, ...patch };
    g.pitch = Math.max(PITCH[0], Math.min(PITCH[1], g.pitch));
    g.zoom = Math.max(ZOOM[0], Math.min(ZOOM[1], g.zoom));
    if (this.bounds) {
      const b = this.bounds;
      const cx = (b.x0 + b.x1) / 2;
      const cz = (b.z0 + b.z1) / 2;
      g.tx = Math.max(b.x0 - cx, Math.min(b.x1 - cx, g.tx));
      g.tz = Math.max(b.z0 - cz, Math.min(b.z1 - cz, g.tz));
    }
    this.goal = g;
    if (still) this.cam = { ...g };
    this.invalidate();
  }

  /** The camera's eye, target and matrices now. */
  matrices() {
    const b = this.bounds || { x0: -50, x1: 50, z0: -50, z1: 50 };
    const cx = (b.x0 + b.x1) / 2;
    const cz = (b.z0 + b.z1) / 2;
    const R = Math.hypot(b.x1 - b.x0, b.z1 - b.z0) / 2;
    const aspect = this.size.w / Math.max(1, this.size.h);
    const half = Math.min(FOV / 2, Math.atan(Math.tan(FOV / 2) * aspect));
    const fit = (R / Math.sin(half)) * 0.8;
    const c = this.cam;
    const dist = fit * c.zoom;
    const target = [cx + c.tx, 0, cz + c.tz];
    const eye = [target[0] + dist * Math.cos(c.pitch) * Math.sin(c.yaw), dist * Math.sin(c.pitch), target[2] + dist * Math.cos(c.pitch) * Math.cos(c.yaw)];
    const view = lookAt(eye, target);
    const proj = perspective(FOV, aspect, Math.max(1, dist - R * 1.6), dist + R * 1.6);
    return { eye, target, vp: mul(proj, view), dist };
  }

  /** The ray under a point of the canvas (CSS pixels): its origin and direction. */
  ray(x, y) {
    const { vp } = this.matrices();
    const inv = inverse(vp);
    const nx = (x / this.size.w) * 2 - 1;
    const ny = 1 - (y / this.size.h) * 2;
    const a = project(inv, [nx, ny, -1]);
    const b = project(inv, [nx, ny, 1]);
    return { o: a.slice(0, 3), d: unit([b[0] - a[0], b[1] - a[1], b[2] - a[2]]) };
  }

  /** Where the ground is under a point of the canvas, as the camera's (tx, tz); null above
   * the horizon. */
  ground(x, y) {
    const { o, d } = this.ray(x, y);
    if (d[1] > -1e-4) return null;
    const t = -o[1] / d[1];
    const b = this.bounds || { x0: 0, x1: 0, z0: 0, z1: 0 };
    return [o[0] + d[0] * t - (b.x0 + b.x1) / 2, o[2] + d[2] * t - (b.z0 + b.z1) / 2];
  }

  // --- drawing ------------------------------------------------------------------------------

  invalidate() {
    if (this.planned || !this.ok) return;
    this.planned = true;
    requestAnimationFrame(() => {
      this.planned = false;
      this.frame();
    });
  }

  frame() {
    if (!this.ok || !this.yardVao) return;
    // glide towards the goal, an eighth of the way a frame
    let moving = false;
    for (const k of Object.keys(this.goal)) {
      const d = this.goal[k] - this.cam[k];
      if (Math.abs(d) > 1e-4) {
        this.cam[k] += d * 0.22;
        moving = true;
      } else this.cam[k] = this.goal[k];
    }
    const gl = this.gl;
    const pal = this.pal;
    const L = pal.light;
    // the sun from the west, a little behind: the sides the camera first sees are lit, the
    // shadows fall towards the street and to the right
    const sun = unit([-0.78, 0.82, 0.22]);
    const b = this.bounds;
    const cx = (b.x0 + b.x1) / 2;
    const cz = (b.z0 + b.z1) / 2;
    const R = Math.hypot(b.x1 - b.x0, b.z1 - b.z0) / 2 + 4;
    const lightVP = mul(ortho(-R, R, -R, R, 1, R * 4), lookAt([cx + sun[0] * R * 2, sun[1] * R * 2, cz + sun[2] * R * 2], [cx, 0, cz]));
    if (this.dirtyShadow && this.level.shadow) {
      this.dirtyShadow = false;
      const SHADOW = this.shadowSize;
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.shadowFb);
      gl.viewport(0, 0, SHADOW, SHADOW);
      gl.enable(gl.DEPTH_TEST);
      gl.clear(gl.DEPTH_BUFFER_BIT);
      gl.colorMask(false, false, false, false);
      gl.enable(gl.POLYGON_OFFSET_FILL);
      gl.polygonOffset(2, 4);
      this.drawAll(this.yardDepth, this.busDepth, lightVP, lightVP, 1, this.skinDepth);
      gl.disable(gl.POLYGON_OFFSET_FILL);
      gl.colorMask(true, true, true, true);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }
    const { vp } = this.matrices();
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.shadowTex);
    for (const prog of [this.yard, this.bus, this.skin]) {
      gl.useProgram(prog.p);
      const u = prog.u;
      gl.uniform1i(u.uShadow, 0);
      if (u.uTex) gl.uniform1i(u.uTex, 1);
      gl.uniform3fv(u.uSun, sun);
      gl.uniform3fv(u.uSunCol, L.sun);
      gl.uniform3fv(u.uSky, L.sky.map((v) => v * v));
      gl.uniform3fv(u.uGround, L.ground.map((v) => v * v));
      gl.uniform1f(u.uNight, L.night);
      gl.uniform1f(u.uTexel, 1 / this.shadowSize);
      gl.uniform1f(u.uShadows, this.level.shadow ? 1 : 0);
      gl.uniform3fv(u.uAccent, pal.accent);
      if (u.uWarn) gl.uniform3fv(u.uWarn, pal.warn);
      if (u.uHi) gl.uniform1f(u.uHi, this.hi);
    }
    this.drawAll(this.yard, this.bus, vp, lightVP, 0, this.skin);
    this.drawOverlays(vp, lightVP);
    this.onFrame?.(vp, this.size.w, this.size.h);
    this.timeFrame();
    if (moving) this.invalidate();
  }

  /** Pictures one after the other (the camera moving, the yard dragged round): sixty of them
   * with half slower than 30 a second is too slow (`onSlow`, once). */
  timeFrame() {
    const t = performance.now();
    if (this.lastFrame && t - this.lastFrame < 250) this.times.push(t - this.lastFrame);
    this.lastFrame = t;
    if (this.times.length < 60) return;
    const s = [...this.times].sort((a, b) => a - b);
    this.times = [];
    if (s[s.length >> 1] > 33 && this.onSlow) {
      const slow = this.onSlow;
      this.onSlow = null;
      slow();
    }
  }

  drawOverlays(vp, lightVP) {
    const list = [...(this.overlays?.values() || [])].filter((o) => o.count);
    if (!list.length) return;
    const gl = this.gl;
    const u = this.yard.u;
    gl.useProgram(this.yard.p);
    gl.uniformMatrix4fv(u.uVP, false, vp);
    gl.uniformMatrix4fv(u.uLight, false, lightVP);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.depthMask(false);
    for (const o of list) {
      gl.uniform4fv(u.uTint, o.tint);
      gl.uniform1f(u.uAlpha, o.alpha);
      gl.bindVertexArray(o.vao);
      gl.drawArrays(gl.TRIANGLES, 0, o.count);
    }
    gl.depthMask(true);
    gl.disable(gl.BLEND);
    gl.uniform4fv(u.uTint, [0, 0, 0, 0]);
    gl.uniform1f(u.uAlpha, 1);
    gl.bindVertexArray(null);
  }

  drawAll(yard, bus, vp, lightVP, shadowPass, skin) {
    const gl = this.gl;
    gl.useProgram(yard.p);
    if (yard.u.uAlpha) {
      gl.uniform4fv(yard.u.uTint, [0, 0, 0, 0]);
      gl.uniform1f(yard.u.uAlpha, 1);
    }
    gl.uniformMatrix4fv(yard.u.uVP, false, vp);
    gl.uniformMatrix4fv(yard.u.uLight, false, lightVP);
    gl.bindVertexArray(this.yardVao);
    gl.drawArrays(gl.TRIANGLES, 0, this.yardCount);
    if (this.busVao && this.instances) {
      gl.useProgram(bus.p);
      gl.uniformMatrix4fv(bus.u.uVP, false, vp);
      gl.uniformMatrix4fv(bus.u.uLight, false, lightVP);
      gl.uniform1f(bus.u.uShadowPass, shadowPass);
      gl.bindVertexArray(this.busVao);
      gl.drawArraysInstanced(gl.TRIANGLES, 0, this.busCount, this.instances);
    }
    if ([...this.skins.values()].some((k) => k.instances)) {
      gl.useProgram(skin.p);
      gl.uniformMatrix4fv(skin.u.uVP, false, vp);
      gl.uniformMatrix4fv(skin.u.uLight, false, lightVP);
      gl.activeTexture(gl.TEXTURE1);
      for (const k of this.skins.values()) {
        if (!k.instances) continue;
        gl.bindTexture(gl.TEXTURE_2D, k.tex);
        gl.bindVertexArray(k.vao);
        gl.drawArraysInstanced(gl.TRIANGLES, 0, k.count, k.instances);
      }
      gl.activeTexture(gl.TEXTURE0);
    }
    gl.bindVertexArray(null);
  }

  /** Let the context go (the page left). */
  destroy() {
    this.ok = false;
    this.gone = true;
    this.gl?.getExtension('WEBGL_lose_context')?.loseContext();
  }
}
