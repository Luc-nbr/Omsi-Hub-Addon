// The roads of a map drawn once for a zoom, for every canvas map of the app (the line editor's,
// the route map's, the company's): a map has thousands of roads, and drawn one by one on every
// frame a map crawled once it was zoomed out. They are kept as a few paths in metres instead
// (one a kind and width of road, its points thinned out as far as the zoom cannot show), drawn
// into a picture a good deal larger than the map's box, and that picture is what a frame puts
// down: a drag only moves it, a zoom scales it until the wheel rests and the roads are drawn
// anew for the new zoom.
//
// A road is `{ w, main, p: [x0, y0, x1, y1, ...] }` in metres, y up (`hub_le`'s `map_data`,
// `hub_route_map`).

/** The picture's size against the box's. */
const LAYER = 1.7;
/** How long the wheel rests before the roads are drawn for the new zoom (ms). */
export const SETTLE = 140;

/** The roads of a list as paths in metres, a path a kind and width of road (`[{ main, w, path,
 * ends }]`, `ends`: a main road's two ends as strokes of no length), for each thinning (metres:
 * points closer than it to the one before are left out). */
const paths = new WeakMap();

export function roadPaths(roads, tol) {
  let bands = paths.get(roads);
  if (!bands) paths.set(roads, (bands = new Map()));
  let gs = bands.get(tol);
  if (gs) return gs;
  const by = new Map();
  const t2 = tol * tol;
  for (const r of roads) {
    const p = r.p;
    if (!p || p.length < 4) continue;
    const key = (r.main ? 'm' : 's') + r.w;
    let e = by.get(key);
    if (!e) by.set(key, (e = { main: !!r.main, w: r.w, path: new Path2D(), ends: r.main ? new Path2D() : null }));
    const n = p.length - 2;
    let [lx, ly] = [p[0], p[1]];
    e.path.moveTo(lx, ly);
    for (let i = 2; i <= n; i += 2) {
      const [x, y] = [p[i], p[i + 1]];
      if (i < n && (x - lx) ** 2 + (y - ly) ** 2 < t2) continue;
      e.path.lineTo(x, y);
      [lx, ly] = [x, y];
    }
    if (e.ends) for (const i of [0, n]) (e.ends.moveTo(p[i], p[i + 1]), e.ends.lineTo(p[i] + 0.01, p[i + 1]));
  }
  gs = [...by.values()];
  bands.set(tol, gs);
  return gs;
}

/** A map's picture of its roads (one a map on the page). `put` puts the roads under camera `k`
 * (`{ cx, cy, mpp }`, metres a point) onto `g` - a box `W` x `H` points at `dpr` pixels a
 * point -: the picture moved, or scaled while the wheel turns (`zoomedAt`: its last turn;
 * `later` is called to come back once it rests), else drawn anew first by `paint(g, groups,
 * mpp)`, which strokes the groups in metres (`g` is set to metres, y up). `look` is anything
 * that changes the picture besides the roads and the camera (compared with ===). */
export function roadLayer() {
  let l = null;
  const draw = (roads, look, k, LW, LH, dpr, paint) => {
    const c = l?.c || document.createElement('canvas');
    const [cw, ch] = [Math.round(LW * dpr), Math.round(LH * dpr)];
    if (c.width !== cw || c.height !== ch) Object.assign(c, { width: cw, height: ch });
    const g = c.getContext('2d');
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.clearRect(0, 0, cw, ch);
    const s = dpr / k.mpp;
    g.setTransform(s, 0, 0, -s, dpr * (LW / 2) - k.cx * s, dpr * (LH / 2) + k.cy * s);
    // (points closer than a little over half a pixel make no difference)
    const tol = Math.max(0.5, 2 ** Math.round(Math.log2(Math.max(k.mpp * 0.6, 1e-6))));
    g.lineJoin = 'round';
    g.lineCap = 'round';
    paint(g, roadPaths(roads, tol), k.mpp);
    return { c, roads, look, dpr, cx: k.cx, cy: k.cy, mpp: k.mpp, LW, LH };
  };
  return {
    put(g, { roads, look, cam: k, W, H, dpr, zoomedAt = 0, later, paint }) {
      const same = l && l.roads === roads && l.look === look && l.dpr === dpr && l.LW >= W * LAYER - 1 && l.LH >= H * LAYER - 1;
      // (what the picture covers, in metres, against what the box shows)
      const covers = () => {
        const [hw, hh, lw, lh] = [(W / 2) * k.mpp, (H / 2) * k.mpp, (l.LW / 2) * l.mpp, (l.LH / 2) * l.mpp];
        return k.cx - hw >= l.cx - lw && k.cx + hw <= l.cx + lw && k.cy - hh >= l.cy - lh && k.cy + hh <= l.cy + lh;
      };
      if (!(same && covers() && (l.mpp === k.mpp || (performance.now() - zoomedAt < SETTLE && (later?.(), true))))) {
        l = draw(roads, look, k, W * LAYER, H * LAYER, dpr, paint);
      }
      const f = l.mpp / k.mpp;
      const x = W / 2 + (l.cx - k.cx) / k.mpp - (l.LW / 2) * f;
      const y = H / 2 - (l.cy - k.cy) / k.mpp - (l.LH / 2) * f;
      g.drawImage(l.c, x, y, l.LW * f, l.LH * f);
    },
    /** The picture let go (the map gone). */
    drop() {
      l = null;
    },
  };
}

// --- the roads as the game's city map draws them ---------------------------------------------

/** A road at least this many points wide, its casing a hair round it (`mapview.rs`); a road at
 * least a lane wide, and a little more on either side (metres). */
const ROAD_PX = 1.4;
const CASING_EDGE_FAR = 0.5;
const CASING_EDGE_NEAR = 1.5;
const SURFACE_GROW_NEAR = 1.0;
const LANE_MIN = 3.6;
const ROAD_MARGIN = 0.3;

/** The roads' casing round their surface and how far their surface grows past its metres, in
 * points, at `mpp` metres a point (`mapview::casing_edge`, its zoom bands). */
function casingEdge(mpp) {
  const tol = Math.min(24, Math.max(0.05, 2 ** Math.round(Math.log2(Math.max(mpp * 0.5, 1e-6)))));
  const t = Math.max(0, Math.min(1, Math.log2(4 / (tol * 2)) / 3));
  return [CASING_EDGE_FAR + (CASING_EDGE_NEAR - CASING_EDGE_FAR) * t, SURFACE_GROW_NEAR * t];
}

const painters = new WeakMap();

/** The `paint` of the roads as the game's city map draws them (`MapView::build`) in the colours
 * `pal` (`{ casing, road, main }`, one object a look): every casing, then the surfaces over them -
 * the side streets' first, the main roads' over them, square-ended on a side street they run
 * into, their own ends rounded. */
export function cityRoads(pal) {
  let f = painters.get(pal);
  if (f) return f;
  f = (g, gs, mpp) => {
    const [edge, grow] = casingEdge(mpp);
    const surface = (e) => Math.max(Math.max(e.w, LANE_MIN) + 2 * ROAD_MARGIN, ROAD_PX * mpp) + 2 * grow * mpp;
    g.strokeStyle = pal.casing;
    for (const e of gs) ((g.lineWidth = surface(e) + 2 * edge * mpp), g.stroke(e.path));
    g.strokeStyle = pal.main;
    for (const e of gs) if (e.ends) ((g.lineWidth = surface(e)), g.stroke(e.ends));
    g.strokeStyle = pal.road;
    for (const e of gs) if (!e.main) ((g.lineWidth = surface(e)), g.stroke(e.path));
    g.lineCap = 'butt';
    g.strokeStyle = pal.main;
    for (const e of gs) if (e.main) ((g.lineWidth = surface(e)), g.stroke(e.path));
  };
  painters.set(pal, f);
  return f;
}
