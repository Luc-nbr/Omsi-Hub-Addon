// The depot's geometry in metres (x east, y up, z south: the street is at the front): boxes,
// prisms and flat marks, each vertex with its normal, its colour, the area it belongs to (the
// Build list's row it lights up) and how much it glows at night. One buffer for the whole yard,
// drawn in one call; the buses are instances of their own (`bus.js`).

/** Floats a vertex: position 3, normal 3, colour 3, area 1, glow 1. */
export const STRIDE = 11;

export class Mesh {
  constructor() {
    this.data = [];
  }

  get count() {
    return this.data.length / STRIDE;
  }

  vertex(p, n, c, area, glow) {
    this.data.push(p[0], p[1], p[2], n[0], n[1], n[2], c[0], c[1], c[2], area, glow);
  }

  /** A quad a, b, c, d (counter-clockwise seen from where `n` points). */
  quad(a, b, c, d, n, col, area = 0, glow = 0) {
    for (const p of [a, b, c, a, c, d]) this.vertex(p, n, col, area, glow);
  }

  /** A box from (x0, y0, z0) to (x1, y1, z1); `bottom` drawn only when asked (it is mostly on
   * the ground). `sides` may darken or colour the walls apart from the top. */
  box(x0, y0, z0, x1, y1, z1, col, { area = 0, glow = 0, top = col, bottom = false } = {}) {
    if (x1 - x0 <= 0 || y1 - y0 <= 0 || z1 - z0 <= 0) return;
    this.quad([x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [0, 1, 0], top, area, glow);
    this.quad([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1], [0, 0, 1], col, area, glow);
    this.quad([x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [0, 0, -1], col, area, glow);
    this.quad([x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [1, 0, 0], col, area, glow);
    this.quad([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [-1, 0, 0], col, area, glow);
    if (bottom) this.quad([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1], [0, -1, 0], col, area, glow);
  }

  /** A mark on the ground (paint, a plot's outline): a thin flat rectangle just above `y`. */
  flat(x0, z0, x1, z1, y, col, area = 0, glow = 0) {
    if (x1 - x0 <= 0 || z1 - z0 <= 0) return;
    this.quad([x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0], [0, 1, 0], col, area, glow);
  }

  /** A plot that could be built on: a dashed outline on the ground. */
  dashed(x0, z0, x1, z1, col, area = 0, { dash = 1.6, gap = 1.2, w = 0.28, y = 0.05 } = {}) {
    const run = (ax, az, bx, bz) => {
      const len = Math.hypot(bx - ax, bz - az);
      const dx = (bx - ax) / len;
      const dz = (bz - az) / len;
      for (let s = 0; s < len; s += dash + gap) {
        const e = Math.min(len, s + dash);
        const px = ax + dx * s;
        const pz = az + dz * s;
        const qx = ax + dx * e;
        const qz = az + dz * e;
        this.flat(Math.min(px, qx) - (dz ? w / 2 : 0), Math.min(pz, qz) - (dx ? w / 2 : 0), Math.max(px, qx) + (dz ? w / 2 : 0), Math.max(pz, qz) + (dx ? w / 2 : 0), y, col, area);
      }
    };
    run(x0, z0, x1, z0);
    run(x1, z0, x1, z1);
    run(x1, z1, x0, z1);
    run(x0, z1, x0, z0);
  }

  /** A roof's ridge along x: a triangular prism from y0 up to the ridge at y1. */
  gable(x0, z0, x1, z1, y0, y1, col, area = 0) {
    const zm = (z0 + z1) / 2;
    const slope = Math.hypot(y1 - y0, zm - z0);
    const nf = [0, (zm - z0) / slope, (y1 - y0) / slope];
    const nb = [0, nf[1], -nf[2]];
    this.quad([x0, y0, z1], [x1, y0, z1], [x1, y1, zm], [x0, y1, zm], nf, col, area);
    this.quad([x1, y0, z0], [x0, y0, z0], [x0, y1, zm], [x1, y1, zm], nb, col, area);
    for (const [x, n] of [
      [x0, [-1, 0, 0]],
      [x1, [1, 0, 0]],
    ]) {
      const tri = n[0] < 0 ? [[x, y0, z0], [x, y0, z1], [x, y1, zm]] : [[x, y0, z1], [x, y0, z0], [x, y1, zm]];
      for (const p of tri) this.vertex(p, n, col, area, 0);
    }
  }

  /** An upright prism of `n` sides round (cx, cz) from y0 to y1, narrowing to `r1` at the top
   * (a tree's crown, a brush of the washing bay, a column). */
  prism(cx, cz, r0, r1, y0, y1, col, { n = 6, area = 0, glow = 0, cap = true, turn = 0 } = {}) {
    const at = (k, r, y) => [cx + Math.cos(turn + (k / n) * Math.PI * 2) * r, y, cz + Math.sin(turn + (k / n) * Math.PI * 2) * r];
    const lean = (r0 - r1) / Math.max(1e-3, y1 - y0);
    for (let k = 0; k < n; k++) {
      const m = turn + ((k + 0.5) / n) * Math.PI * 2;
      const len = Math.hypot(1, lean);
      const nrm = [Math.cos(m) / len, lean / len, Math.sin(m) / len];
      const a = at(k, r0, y0);
      const b = at(k + 1, r0, y0);
      const c = at(k + 1, r1, y1);
      const d = at(k, r1, y1);
      for (const p of [a, d, c, a, c, b]) this.vertex(p, nrm, col, area, glow);
    }
    if (cap && r1 > 0) for (let k = 0; k < n; k++) for (const p of [[cx, y1, cz], at(k + 1, r1, y1), at(k, r1, y1)]) this.vertex(p, [0, 1, 0], col, area, glow);
  }

  floats() {
    return new Float32Array(this.data);
  }
}
