//! A device's screen from `model.cfg` and the `.o3d`s: which pictures, text fields and touch
//! fields lie on it, exactly where, and under which condition the game shows them (the career
//! add-on's `core/schermvorm.ts`, ported rule by rule; its measurements are quoted where a rule
//! came from one).
//!
//! The way, in short:
//!
//! 1. Every `[mesh]` line goes through its chain of `[animparent]` and `[newanim]`, with the
//!    live values of the variables. Without that chain the six lines of ALMEX menu 26 lie 1.3 m
//!    beside the screen (`trans_dauer`); with it 0.4-0.7 mm above it.
//! 2. The device's ANCHORS - its text materials and click fields, as [`crate::modules`] groups
//!    them - vote on the surface under them: first only the text fields, the click fields only
//!    when those lie on nothing. Candidates come from ALL meshes of the cfg. The surface chosen
//!    most is the background (the ALMEX: 29 screen quads at one place, one per menu), and of it
//!    only the flat connected PIECE under the anchors: the AFR 200 has its display, keys and
//!    case on one texture of 2517 triangles, of which 5 are the display.
//! 3. That piece is the plane. Everything is put in (s, t) on it, with the physical aspect.
//! 4. What is drawn faces the driver (the winding normal (B-A)x(C-A), never the vertex normals).
//! 5. Keys on the plane become touch fields; what lies further off stays a key of the panel.
//!
//! Axes: those of the `.o3d` files, x wide, y high, z long - a mirrored frame against the real
//! bus, which is why "right, seen from the front" is n x up here.

use crate::cfg::{self, Anim, Axis, Matl, Mesh, ModelCfg, P3};
use crate::ctc;
use crate::modules::Module;
use crate::o3d::{self, O3d};
use crate::protocol::{Choice, Font, Form, Kind, Part, Tap, TextSpec, Visible};
use crate::scripts;
use crate::textures::{self, Flags};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/* ---------------------------------------------------------------- arithmetic */

type Mat = [f64; 9];

#[derive(Debug, Clone, Copy)]
struct Affine {
    m: Mat,
    t: P3,
}

const IDENTITY: Mat = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
const STILL: Affine = Affine { m: IDENTITY, t: [0.0; 3] };

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: P3, f: f64) -> P3 {
    [a[0] * f, a[1] * f, a[2] * f]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn length(a: P3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P3) -> P3 {
    let l = length(a);
    if l > 1e-12 {
        mul(a, 1.0 / l)
    } else {
        [0.0; 3]
    }
}
fn mat_mul(a: Mat, b: Mat) -> Mat {
    let mut c = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            c[i * 3 + j] = a[i * 3] * b[j] + a[i * 3 + 1] * b[3 + j] + a[i * 3 + 2] * b[6 + j];
        }
    }
    c
}
fn mat_vec(a: Mat, p: P3) -> P3 {
    [a[0] * p[0] + a[1] * p[1] + a[2] * p[2], a[3] * p[0] + a[4] * p[1] + a[5] * p[2], a[6] * p[0] + a[7] * p[1] + a[8] * p[2]]
}
fn transpose(a: Mat) -> Mat {
    [a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8]]
}
/// Right-handed about an o3d axis, by `degrees`.
fn rot(axis: usize, degrees: f64) -> Mat {
    let h = degrees.to_radians();
    let (s, c) = h.sin_cos();
    match axis {
        0 => [1.0, 0.0, 0.0, 0.0, c, -s, 0.0, s, c],
        1 => [c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c],
        _ => [c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0],
    }
}
/// First `b`, then `a`.
fn after(a: Affine, b: Affine) -> Affine {
    Affine { m: mat_mul(a.m, b.m), t: add(mat_vec(a.m, b.t), a.t) }
}
fn apply(a: &Affine, p: P3) -> P3 {
    add(mat_vec(a.m, p), a.t)
}
/// JavaScript's `Math.round`: halves up.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}
/// Five decimals, as the form carries its numbers.
fn round5(x: f64) -> f64 {
    js_round(x * 1e5) / 1e5
}

/* ---------------------------------------------------------------- the models */

/// What an `.o3d` says of WHETHER it matters: the box of its raw vertices, its pivot, its
/// materials. Small enough to keep of every mesh of the bus; the geometry itself comes from
/// [`Models::model`] only when the mesh lies near the device.
pub struct Info {
    pub file: PathBuf,
    /// Has vertices and triangles.
    pub valid: bool,
    pub encrypted: bool,
    pub lo: P3,
    pub hi: P3,
    pub transform: Option<[f32; 16]>,
    pub textures: Vec<String>,
    pub diffuse: Vec<[f32; 4]>,
}

/// The models, cfgs and script values read during one bus's builds. Kept per build thread.
#[derive(Default)]
pub struct Models {
    infos: RefCell<HashMap<PathBuf, Rc<Info>>>,
    models: RefCell<HashMap<PathBuf, Rc<O3d>>>,
    order: RefCell<VecDeque<(PathBuf, usize)>>,
    bytes: Cell<usize>,
    cfgs: RefCell<HashMap<PathBuf, Option<Rc<ModelCfg>>>>,
    starts: RefCell<HashMap<PathBuf, Rc<HashMap<String, f64>>>>,
}

/// The models kept at most: a big bus has 206 MB of `.o3d`s.
const MODEL_BUDGET: usize = 48 * 1024 * 1024;

impl Models {
    fn cfg(&self, model_cfg: &Path) -> Option<Rc<ModelCfg>> {
        if let Some(c) = self.cfgs.borrow().get(model_cfg) {
            return c.clone();
        }
        let c = cfg::read(model_cfg).map(Rc::new);
        self.cfgs.borrow_mut().insert(model_cfg.to_path_buf(), c.clone());
        c
    }

    fn start_values(&self, model_cfg: &Path) -> Rc<HashMap<String, f64>> {
        if let Some(s) = self.starts.borrow().get(model_cfg) {
            return s.clone();
        }
        let s = Rc::new(scripts::start_values(model_cfg));
        self.starts.borrow_mut().insert(model_cfg.to_path_buf(), s.clone());
        s
    }

    fn keep(&self, file: &Path, model: Rc<O3d>) {
        let size = (model.vertices.len() + model.normals.len() + model.uvs.len() + model.triangles.len()) * 4 + model.material_of.len() * 2;
        self.models.borrow_mut().insert(file.to_path_buf(), model);
        self.order.borrow_mut().push_back((file.to_path_buf(), size));
        self.bytes.set(self.bytes.get() + size);
        while self.bytes.get() > MODEL_BUDGET {
            let Some((old, n)) = self.order.borrow_mut().pop_front() else { break };
            if old == file {
                self.order.borrow_mut().push_back((old, n));
                break;
            }
            self.models.borrow_mut().remove(&old);
            self.bytes.set(self.bytes.get() - n);
        }
    }

    fn info(&self, file: &Path) -> Rc<Info> {
        if let Some(i) = self.infos.borrow().get(file) {
            return i.clone();
        }
        let model = o3d::read_file(file);
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        if let Some(m) = &model {
            for v in m.vertices.as_chunks::<3>().0 {
                for j in 0..3 {
                    lo[j] = lo[j].min(v[j] as f64);
                    hi[j] = hi[j].max(v[j] as f64);
                }
            }
        }
        let info = Rc::new(Info {
            file: file.to_path_buf(),
            valid: model.as_ref().is_some_and(|m| m.vertices.len() >= 3 && m.triangles.len() >= 3),
            encrypted: model.as_ref().is_some_and(|m| m.encrypted),
            lo,
            hi,
            transform: model.as_ref().and_then(|m| m.transform),
            textures: model.as_ref().map(|m| m.materials.iter().map(|x| x.texture.clone()).collect()).unwrap_or_default(),
            diffuse: model.as_ref().map(|m| m.materials.iter().map(|x| x.diffuse).collect()).unwrap_or_default(),
        });
        self.infos.borrow_mut().insert(file.to_path_buf(), info.clone());
        if let Some(m) = model {
            self.keep(file, Rc::new(m));
        }
        info
    }

    /// The geometry of an `.o3d`; from memory while it is there.
    fn model(&self, info: &Info) -> Option<Rc<O3d>> {
        if !info.valid {
            return None;
        }
        if let Some(m) = self.models.borrow().get(&info.file) {
            return Some(m.clone());
        }
        let m = Rc::new(o3d::read_file(&info.file)?);
        self.keep(&info.file, m.clone());
        Some(m)
    }
}

/* ---------------------------------------------------------------- chains */

/// The movement of one `[newanim]`, in o3d axes. `anim_trans` shifts along the local x AFTER
/// the `origin_rot`s (measured on the Hamburg bus); `anim_rot` turns about the same local x.
/// `origin_from_mesh` takes only the POINT of the 0x79 matrix, not its turn.
fn anim_affine(anim: &Anim, transform: Option<[f32; 16]>, value: &dyn Fn(&str) -> f64) -> Affine {
    let o: P3 = match anim.origin {
        Some(p) => p,
        None => transform.map(|t| [t[12] as f64, t[13] as f64, t[14] as f64]).unwrap_or([0.0; 3]),
    };
    let mut r = IDENTITY;
    for (axis, angle) in &anim.rotations {
        // (rot_x about o3d x, rot_y about o3d z - OMSI's y is forward - and rot_z about o3d y)
        r = mat_mul(
            r,
            rot(
                match axis {
                    Axis::X => 0,
                    Axis::Y => 2,
                    Axis::Z => 1,
                },
                *angle,
            ),
        );
    }
    let mut out = STILL;
    if let Some((var, f)) = &anim.turn {
        let angle = f * value(var);
        if *f != 0.0 && angle != 0.0 {
            let m = mat_mul(mat_mul(r, rot(0, angle)), transpose(r));
            out = Affine { m, t: sub(o, mat_vec(m, o)) };
        }
    }
    if let Some((var, f)) = &anim.shift {
        let step = f * value(var);
        if *f != 0.0 && step != 0.0 {
            out.t = add(out.t, mul(mat_vec(r, [1.0, 0.0, 0.0]), step));
        }
    }
    out
}

/* ---------------------------------------------------------------- triangles */

#[derive(Debug, Clone)]
struct Tri {
    /// Vertex numbers in the `.o3d`.
    i: [u32; 3],
    a: P3,
    b: P3,
    c: P3,
    /// The unit normal of the winding: (B-A)x(C-A).
    n: P3,
    area: f64,
    e0: P3,
    e1: P3,
    d00: f64,
    d01: f64,
    d11: f64,
    inv: f64,
}

fn make_tri(i: [u32; 3], a: P3, b: P3, c: P3) -> Option<Tri> {
    let e0 = sub(b, a);
    let e1 = sub(c, a);
    let k = cross(e0, e1);
    let l = length(k);
    let area = l / 2.0;
    // (smaller than 1e-8 m2: ignored everywhere)
    if area.is_nan() || area < 1e-8 {
        return None;
    }
    let (d00, d01, d11) = (dot(e0, e0), dot(e0, e1), dot(e1, e1));
    let den = d00 * d11 - d01 * d01;
    if den.is_nan() || den.abs() <= 1e-30 {
        return None;
    }
    Some(Tri { i, a, b, c, n: mul(k, 1.0 / l), area, e0, e1, d00, d01, d11, inv: 1.0 / den })
}

fn corners(t: &Tri) -> [P3; 3] {
    [t.a, t.b, t.c]
}

fn points_of(tris: &[Tri]) -> Vec<P3> {
    tris.iter().flat_map(corners).collect()
}

/// A grid over triangles, so that [`hit`] need not walk all triangles of the whole interior.
/// Cells of 2 cm; a triangle over too many cells goes in a list of its own.
struct Grid {
    map: HashMap<i64, Vec<u32>>,
    big: Vec<u32>,
    cell: f64,
}

fn cell_key(x: i64, y: i64, z: i64) -> i64 {
    ((x + 5000) * 10001 + (y + 5000)) * 10001 + (z + 5000)
}

impl Grid {
    /// `area`: only triangles (and cells) within this box.
    fn new(tris: &[Tri], margin: f64, area: Option<(P3, P3)>) -> Grid {
        let cell = 0.02;
        let half_diagonal = cell * 0.8661;
        let mut map: HashMap<i64, Vec<u32>> = HashMap::new();
        let mut big = Vec::new();
        let g = area.map(|(lo, hi)| (lo.map(|x| (x / cell).floor() as i64), hi.map(|x| (x / cell).floor() as i64)));
        'tris: for (k, d) in tris.iter().enumerate() {
            let mut lo = [0i64; 3];
            let mut hi = [0i64; 3];
            for j in 0..3 {
                lo[j] = ((d.a[j].min(d.b[j]).min(d.c[j]) - margin) / cell).floor() as i64;
                hi[j] = ((d.a[j].max(d.b[j]).max(d.c[j]) + margin) / cell).floor() as i64;
                if let Some((g0, g1)) = g {
                    lo[j] = lo[j].max(g0[j]);
                    hi[j] = hi[j].min(g1[j]);
                    if lo[j] > hi[j] {
                        continue 'tris;
                    }
                }
            }
            let count = (hi[0] - lo[0] + 1) * (hi[1] - lo[1] + 1) * (hi[2] - lo[2] + 1);
            if count > 20000 {
                big.push(k as u32);
                continue;
            }
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        // (a big slanted triangle: only the cells its plane touches)
                        if count > 8 {
                            let m = [(x as f64 + 0.5) * cell - d.a[0], (y as f64 + 0.5) * cell - d.a[1], (z as f64 + 0.5) * cell - d.a[2]];
                            if dot(m, d.n).abs() > margin + half_diagonal {
                                continue;
                            }
                        }
                        map.entry(cell_key(x, y, z)).or_default().push(k as u32);
                    }
                }
            }
        }
        Grid { map, big, cell }
    }

    fn near(&self, q: P3) -> impl Iterator<Item = usize> + '_ {
        let key = cell_key((q[0] / self.cell).floor() as i64, (q[1] / self.cell).floor() as i64, (q[2] / self.cell).floor() as i64);
        self.map.get(&key).map(|v| v.as_slice()).unwrap_or(&[]).iter().chain(self.big.iter()).map(|k| *k as usize)
    }
}

#[derive(Debug, Clone, Copy)]
struct Hit {
    /// Signed: positive = in front (the side of the winding normal).
    d: f64,
    tri: usize,
}

/// The triangle right under (or over) `q`: the smallest |d|, with the projection inside the
/// triangle (barycentric >= -0.03), and |d| <= `dmax`.
fn hit(tris: &[Tri], near: impl Iterator<Item = usize>, q: P3, dmax: f64) -> Option<Hit> {
    let mut best: Option<Hit> = None;
    for k in near {
        let t = &tris[k];
        let w = sub(q, t.a);
        let d = dot(w, t.n);
        if d.abs() > dmax {
            continue;
        }
        if best.is_some_and(|b| d.abs() >= b.d.abs()) {
            continue;
        }
        let v2 = sub(w, mul(t.n, d));
        let (d20, d21) = (dot(v2, t.e0), dot(v2, t.e1));
        let b1 = (t.d11 * d20 - t.d01 * d21) * t.inv;
        let b2 = (t.d00 * d21 - t.d01 * d20) * t.inv;
        if b1 < -0.03 || b2 < -0.03 || 1.0 - b1 - b2 < -0.03 {
            continue;
        }
        best = Some(Hit { d, tri: k });
    }
    best
}

fn hit_grid(grid: &Grid, tris: &[Tri], q: P3, dmax: f64) -> Option<Hit> {
    hit(tris, grid.near(q), q, dmax)
}

fn hit_all(tris: &[Tri], q: P3, dmax: f64) -> Option<Hit> {
    hit(tris, 0..tris.len(), q, dmax)
}

/* ---------------------------------------------------------------- meshes */

struct Geo {
    /// The mesh's place among the cfg's `[mesh]` lines: its identity here.
    k: usize,
    mesh: Mesh,
    info: Rc<Info>,
    world: Affine,
    /// The `[newanim]`s of the whole chain, parent first.
    chain: Vec<Anim>,
    pos: OnceCell<Rc<Vec<P3>>>,
    /// The box around the mesh after the chain (roomy: the `.o3d`'s box, turned along).
    lo: P3,
    hi: P3,
    contexts: Vec<Option<Matl>>,
    per_mat: RefCell<HashMap<i64, Rc<Vec<Tri>>>>,
}

fn box_of(points: &[P3]) -> (P3, P3) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in points {
        for j in 0..3 {
            lo[j] = lo[j].min(p[j]);
            hi[j] = hi[j].max(p[j]);
        }
    }
    (lo, hi)
}

fn in_box(lo: &P3, hi: &P3, q: P3, margin: f64) -> bool {
    (0..3).all(|j| q[j] >= lo[j] - margin && q[j] <= hi[j] + margin)
}

fn overlaps(g: &Geo, lo: P3, hi: P3, margin: f64) -> bool {
    (0..3).all(|j| !(g.hi[j] < lo[j] - margin || g.lo[j] > hi[j] + margin))
}

fn middle_of(points: &[P3]) -> P3 {
    let mut s = [0.0; 3];
    for p in points {
        s = add(s, *p);
    }
    mul(s, 1.0 / points.len().max(1) as f64)
}

/// The vertices after the chain; empty for an encrypted mesh (that has to be unscrambled first).
fn world_pos(models: &Models, g: &Geo) -> Rc<Vec<P3>> {
    g.pos
        .get_or_init(|| {
            let mut pos = Vec::new();
            if !g.info.encrypted {
                if let Some(m) = models.model(&g.info) {
                    for v in m.vertices.as_chunks::<3>().0 {
                        pos.push(apply(&g.world, [v[0] as f64, v[1] as f64, v[2] as f64]));
                    }
                }
            }
            Rc::new(pos)
        })
        .clone()
}

/// The triangles of material `mat` (-1: all), with the world's vertices or `own` ones.
fn tris_of(models: &Models, g: &Geo, mat: i64, own: Option<&[Option<P3>]>) -> Rc<Vec<Tri>> {
    if own.is_none() {
        if let Some(t) = g.per_mat.borrow().get(&mat) {
            return t.clone();
        }
    }
    let world = if own.is_none() { Some(world_pos(models, g)) } else { None };
    let at = |i: u32| -> Option<P3> {
        match own {
            Some(o) => o.get(i as usize).copied().flatten(),
            None => world.as_ref().and_then(|w| w.get(i as usize).copied()),
        }
    };
    let mut out = Vec::new();
    if let Some(m) = models.model(&g.info) {
        for (t, material) in m.material_of.iter().enumerate() {
            if mat >= 0 && *material as i64 != mat {
                continue;
            }
            let Some(tri) = m.triangles.get(t * 3..t * 3 + 3) else { break };
            let i = [tri[0], tri[1], tri[2]];
            let (Some(a), Some(b), Some(c)) = (at(i[0]), at(i[1]), at(i[2])) else { continue };
            if let Some(d) = make_tri(i, a, b, c) {
                out.push(d);
            }
        }
    }
    let out = Rc::new(out);
    if own.is_none() {
        g.per_mat.borrow_mut().insert(mat, out.clone());
    }
    out
}

/// The name of the texture a material shows without live values: the default of
/// `[matl_freetex]`, else the `.o3d`'s.
fn texture_name_of(g: &Geo, mat: usize) -> String {
    let ctx = g.contexts.get(mat).and_then(Option::as_ref);
    match ctx.and_then(|c| c.state.freetex.as_ref()).map(|f| f.0.clone()).filter(|d| !d.is_empty()) {
        Some(d) => d,
        None => g.info.textures.get(mat).map(|t| t.trim().to_string()).unwrap_or_default(),
    }
}

/// The diffuse colour: from `[matl_allcolor]` when there is one, else from the `.o3d`.
fn colour_of(g: &Geo, mat: usize) -> [f64; 4] {
    let ctx = g.contexts.get(mat).and_then(Option::as_ref);
    if let Some(c) = ctx.and_then(|c| c.state.allcolor.as_ref()) {
        return [c[0], c[1], c[2], c[3]];
    }
    g.info.diffuse.get(mat).map(|d| d.map(|x| x as f64)).unwrap_or([1.0; 4])
}

fn ctx_of(g: &Geo, mat: usize) -> Option<&Matl> {
    g.contexts.get(mat).and_then(Option::as_ref)
}

fn alpha_of(ctx: Option<&Matl>) -> u8 {
    ctx.and_then(|c| c.state.alpha).unwrap_or(0)
}

fn script_tex(s: &str) -> bool {
    s.starts_with("\\S:")
}

/// `^(invisible|unsichtbar|transparent)`, case-insensitively.
fn invisible(name: &str) -> bool {
    let l = name.to_lowercase();
    ["invisible", "unsichtbar", "transparent"].iter().any(|p| l.starts_with(p))
}

/// What was worked out once (or found not to be there).
type Memo<K, V> = RefCell<HashMap<K, Option<Rc<V>>>>;

const PERMS: [[usize; 3]; 6] = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];

/* ---------------------------------------------------------------- the analysis */

/// A surface that can be background or layer: one material of one `[mesh]` line.
struct Cand {
    geo: Rc<Geo>,
    mat: usize,
    name: String,
    tris: Rc<Vec<Tri>>,
    lo: P3,
    hi: P3,
    grid: OnceCell<Grid>,
}

struct Anchor {
    geo: Rc<Geo>,
    text: bool,
    /// The material of a text; -1 for a click (all materials).
    mat: i64,
    points: Vec<P3>,
    middle: P3,
}

/// Whether two parts are shown under the same `[visible]` conditions.
fn same_visible(a: &[(String, i64)], b: &[(String, i64)]) -> bool {
    a.len() == b.len() && a.iter().all(|(n, v)| b.iter().any(|(m, w)| m.eq_ignore_ascii_case(n) && v == w))
}

/// The plane of the screen.
#[derive(Debug, Clone, Copy)]
struct Plane {
    o: P3,
    n: P3,
    right: P3,
    up: P3,
    s0: f64,
    s1: f64,
    t0: f64,
    t1: f64,
}

struct Layer {
    geo: Rc<Geo>,
    mat: usize,
    tris: Vec<Tri>,
}

struct TextOn {
    geo: Rc<Geo>,
    mat: usize,
    ctx: Matl,
    tris: Vec<Tri>,
    /// Encrypted: its uvs are not to be trusted.
    approx: bool,
}

struct ClickOn {
    geo: Rc<Geo>,
    points: Vec<P3>,
}

/// A key of the device that did not become a touch field, with how far it lies from the screen:
/// square to the plane (`plane_mm`) and beside it, in the plane, to the screen's edge
/// (`edge_mm`, 0 within it).
#[derive(Debug, Clone, PartialEq)]
pub struct LooseKey {
    pub actions: Vec<String>,
    pub plane_mm: f64,
    pub edge_mm: f64,
}

/// How far from the screen a key still sits ON the device. Measured over a fleet as the
/// distance to the screen's edge: the keypads of the Kajosoft Citybuses' RG boxes 21 to 91 mm
/// (they belong), the ALMEX's TICKET TOGGLE 124 mm, its RIEGEL 274 mm, the change around 330 mm
/// (they sit apart). Nothing lies between 91 and 124, so 100.
pub const OFF_THE_SCREEN_MM: f64 = 100.0;

struct Analysis {
    cfg: Rc<ModelCfg>,
    geos: Vec<Rc<Geo>>,
    /// Backgrounds, the base first, then cfg order.
    backgrounds: Vec<Layer>,
    layers: Vec<Layer>,
    texts: Vec<TextOn>,
    clicks: Vec<ClickOn>,
    loose: Vec<LooseKey>,
    plane: Option<Plane>,
    base: Option<Rc<Geo>>,
    incomplete: Vec<String>,
}

/// Case-insensitive lookup in numbers (the exact name first).
#[derive(Default)]
struct Values {
    exact: HashMap<String, f64>,
    lower: HashMap<String, f64>,
}

impl Values {
    fn new(exact: HashMap<String, f64>) -> Values {
        let mut lower = HashMap::new();
        for (n, v) in &exact {
            lower.entry(n.to_lowercase()).or_insert(*v);
        }
        Values { exact, lower }
    }
    fn get(&self, name: &str) -> Option<f64> {
        self.exact.get(name).or_else(|| self.lower.get(&name.to_lowercase())).copied()
    }
    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
    fn value(&self, name: &str) -> f64 {
        self.get(name).filter(|v| v.is_finite()).unwrap_or(0.0)
    }
}

fn norm_path(p: &str) -> String {
    let t = p.trim();
    let mut out = String::with_capacity(t.len());
    let mut sep = false;
    for c in t.chars() {
        if c == '\\' || c == '/' {
            if !sep {
                out.push('/');
            }
            sep = true;
        } else {
            out.extend(c.to_lowercase());
            sep = false;
        }
    }
    out
}

fn text_mats(g: &Geo) -> Vec<usize> {
    g.contexts.iter().enumerate().filter(|(_, c)| c.as_ref().is_some_and(|c| c.text.is_some())).map(|(k, _)| k).collect()
}

/// What is in chain `a` and not in chain `b`, by content.
fn chain_difference(a: &[Anim], b: &[Anim]) -> Vec<Anim> {
    let sign = |x: &Anim| format!("{:?}", (if x.turn.is_some() { x.origin } else { Some([0.0; 3]) }, &x.rotations, &x.shift, &x.turn));
    let mut left: HashMap<String, usize> = HashMap::new();
    for x in b {
        *left.entry(sign(x)).or_default() += 1;
    }
    a.iter()
        .filter(|x| {
            let n = left.entry(sign(x)).or_default();
            if *n == 0 {
                return true;
            }
            *n -= 1;
            false
        })
        .cloned()
        .collect()
}

/// An insertion-ordered map of triangle -> how many anchors lie on it.
#[derive(Default, Clone)]
struct Seeds {
    order: Vec<(usize, usize)>,
    at: HashMap<usize, usize>,
}

impl Seeds {
    fn add(&mut self, tri: usize, n: usize) {
        match self.at.get(&tri) {
            Some(&k) => self.order[k].1 += n,
            None => {
                self.at.insert(tri, self.order.len());
                self.order.push((tri, n));
            }
        }
    }
}

/// Whether a material can be background or layer.
fn make_cand(models: &Models, g: &Rc<Geo>, mat: usize, found: &dyn Fn(&str) -> bool) -> Option<Cand> {
    if !g.info.valid || g.info.encrypted || g.mesh.shadow {
        return None;
    }
    let ctx = ctx_of(g, mat);
    if ctx.is_some_and(|c| c.text.is_some()) || ctx.and_then(|c| c.state.transmap.as_deref()).is_some_and(script_tex) {
        return None;
    }
    let name = texture_name_of(g, mat);
    if name.is_empty() || invisible(&name) || script_tex(&name) {
        return None;
    }
    // (the alpha rule only at [matl_alpha] >= 1: the NLC's keys have diffuse alpha 0)
    if alpha_of(ctx) >= 1 && colour_of(g, mat)[3] <= 0.01 {
        return None;
    }
    if ctx.and_then(|c| c.state.freetex.as_ref()).is_none() && !found(&name) {
        return None;
    }
    let tris = tris_of(models, g, mat as i64, None);
    if tris.is_empty() {
        return None;
    }
    let (lo, hi) = box_of(&points_of(&tris));
    Some(Cand { geo: g.clone(), mat, name, tris, lo, hi, grid: OnceCell::new() })
}

enum Extra {
    Colour,
    Script,
    Missing(String),
}

/// What is no candidate but is drawn on the screen: a surface without a texture (in its colour)
/// and a script texture (not drawable).
fn extra_of(g: &Geo, mat: usize, found: &dyn Fn(&str) -> bool) -> Option<Extra> {
    if !g.info.valid || g.info.encrypted {
        return None;
    }
    let ctx = ctx_of(g, mat);
    if ctx.is_some_and(|c| c.text.is_some()) {
        return None;
    }
    let name = texture_name_of(g, mat);
    if ctx.and_then(|c| c.state.transmap.as_deref()).is_some_and(script_tex) || script_tex(&name) {
        return Some(Extra::Script);
    }
    if alpha_of(ctx) >= 1 && colour_of(g, mat)[3] <= 0.01 {
        return None;
    }
    if invisible(&name) {
        return None;
    }
    if name.is_empty() {
        return Some(Extra::Colour);
    }
    if ctx.and_then(|c| c.state.freetex.as_ref()).is_none() && !found(&name) {
        return Some(Extra::Missing(name));
    }
    None
}

/// The flat connected piece of a surface around the triangles hit: over shared vertices, the
/// normal within 3 degrees and every vertex within 1 mm of the seed's plane. AFR 200: 5 of 2517.
fn piece_of(tris: &[Tri], seeds: &Seeds) -> Vec<Tri> {
    if seeds.order.is_empty() {
        return Vec::new();
    }
    let cos3 = 3f64.to_radians().cos();
    let same_plane = |a: &Tri, b: &Tri| dot(a.n, b.n) >= cos3 && corners(b).iter().all(|p| dot(sub(*p, a.a), a.n).abs() < 0.001);
    // (the anchors may lie on more than one plane of the material: the one with most anchors wins)
    let mut best = usize::MAX;
    let mut best_weight: i64 = -1;
    for &(k, _) in &seeds.order {
        let sum: usize = seeds.order.iter().filter(|(k2, _)| same_plane(&tris[k], &tris[*k2])).map(|(_, w)| *w).sum();
        if sum as i64 > best_weight {
            best = k;
            best_weight = sum as i64;
        }
    }
    let seed: Vec<usize> = seeds.order.iter().map(|(k, _)| *k).filter(|k| same_plane(&tris[best], &tris[*k])).collect();
    let mut nz = [0.0; 3];
    for &k in &seed {
        nz = add(nz, mul(tris[k].n, tris[k].area));
    }
    let n = unit(nz);
    let a0 = tris[best].a;
    let key = |p: &P3| (js_round(p[0] * 2000.0) as i64, js_round(p[1] * 2000.0) as i64, js_round(p[2] * 2000.0) as i64);
    let mut per_point: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
    for (k, d) in tris.iter().enumerate() {
        for p in corners(d) {
            per_point.entry(key(&p)).or_default().push(k);
        }
    }
    let fits = |d: &Tri| dot(d.n, n) >= cos3 && corners(d).iter().all(|p| dot(sub(*p, a0), n).abs() < 0.001);
    let mut inside: HashSet<usize> = HashSet::new();
    let mut queue: Vec<usize> = Vec::new();
    for k in seed {
        if fits(&tris[k]) && inside.insert(k) {
            queue.push(k);
        }
    }
    while let Some(k) = queue.pop() {
        for p in corners(&tris[k]) {
            for &k2 in per_point.get(&key(&p)).map(|v| v.as_slice()).unwrap_or(&[]) {
                if inside.contains(&k2) || !fits(&tris[k2]) {
                    continue;
                }
                inside.insert(k2);
                queue.push(k2);
            }
        }
    }
    let mut ks: Vec<usize> = inside.into_iter().collect();
    ks.sort_unstable();
    ks.into_iter().map(|k| tris[k].clone()).collect()
}

/// The plane of a piece: the area-weighted winding normal, "up" = o3d +y projected on the plane
/// (o3d +z for a flat one), "right" seen from the front, and the rectangle around it.
fn plane_of(tris: &[Tri]) -> Option<Plane> {
    if tris.is_empty() {
        return None;
    }
    let mut nz = [0.0; 3];
    let mut o = [0.0; 3];
    let mut area = 0.0;
    for d in tris {
        nz = add(nz, mul(d.n, d.area));
        o = add(o, mul(middle_of(&corners(d)), d.area));
        area += d.area;
    }
    let n = unit(nz);
    if length(n) < 0.5 || area <= 0.0 {
        return None;
    }
    let o = mul(o, 1.0 / area);
    let mut up = sub([0.0, 1.0, 0.0], mul(n, n[1]));
    if length(up) < 0.3 {
        up = sub([0.0, 0.0, 1.0], mul(n, n[2]));
    }
    let up = unit(up);
    let right = unit(cross(n, up));
    let (mut s0, mut s1, mut t0, mut t1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for d in tris {
        for p in corners(d) {
            let w = sub(p, o);
            let (s, t) = (dot(w, right), dot(w, up));
            s0 = s0.min(s);
            s1 = s1.max(s);
            t0 = t0.min(t);
            t1 = t1.max(t);
        }
    }
    if (s1 - s0).is_nan() || s1 - s0 <= 1e-4 || (t1 - t0).is_nan() || t1 - t0 <= 1e-4 {
        return None;
    }
    Some(Plane { o, n, right, up, s0, s1, t0, t1 })
}

impl Plane {
    fn distance(&self, p: P3) -> f64 {
        dot(sub(p, self.o), self.n)
    }
    fn st(&self, p: P3) -> (f64, f64) {
        let w = sub(p, self.o);
        (dot(w, self.right), dot(w, self.up))
    }
    fn within(&self, p: P3, slack: f64) -> bool {
        let (s, t) = self.st(p);
        let bs = (self.s1 - self.s0) * slack;
        let bt = (self.t1 - self.t0) * slack;
        s >= self.s0 - bs && s <= self.s1 + bs && t >= self.t0 - bt && t <= self.t1 + bt
    }
    fn faces(&self, d: &Tri) -> bool {
        dot(d.n, self.n) > 0.0
    }
}

struct Ctx<'a> {
    models: &'a Models,
    cfg: Rc<ModelCfg>,
    geos: Vec<Option<Rc<Geo>>>,
    busy: HashSet<usize>,
    per_ident: HashMap<String, usize>,
    value: &'a dyn Fn(&str) -> f64,
}

fn geo_of(k: usize, cx: &mut Ctx) -> Option<Rc<Geo>> {
    if let Some(g) = &cx.geos[k] {
        return Some(g.clone());
    }
    let cfg = cx.cfg.clone();
    let mesh = &cfg.meshes[k];
    if !mesh.exists || cx.busy.contains(&k) {
        return None;
    }
    cx.busy.insert(k);
    let info = cx.models.info(&mesh.file);
    let mut world = STILL;
    let mut chain: Vec<Anim> = Vec::new();
    if let Some(parent) = mesh.parent.as_ref().filter(|p| !p.is_empty()).and_then(|p| cx.per_ident.get(&p.to_lowercase())).copied() {
        if parent != k {
            if let Some(og) = geo_of(parent, cx) {
                world = og.world;
                chain = og.chain.clone();
            }
        }
    }
    for anim in &mesh.anims {
        world = after(world, anim_affine(anim, info.transform, cx.value));
        chain.push(anim.clone());
    }
    // (the box without turning every vertex: the eight corners of the `.o3d`'s box)
    let mut cs = Vec::new();
    if info.valid && !info.encrypted {
        for x in [info.lo[0], info.hi[0]] {
            for y in [info.lo[1], info.hi[1]] {
                for z in [info.lo[2], info.hi[2]] {
                    cs.push(apply(&world, [x, y, z]));
                }
            }
        }
    }
    let (lo, hi) = box_of(&cs);
    let contexts = if info.valid { cfg::material_contexts(mesh, &info.textures) } else { Vec::new() };
    let g = Rc::new(Geo { k, mesh: mesh.clone(), info, world, chain, pos: OnceCell::new(), lo, hi, contexts, per_mat: RefCell::new(HashMap::new()) });
    cx.geos[k] = Some(g.clone());
    cx.busy.remove(&k);
    Some(g)
}

fn list_of(meshes: &HashSet<String>) -> String {
    let mut l: Vec<&String> = meshes.iter().collect();
    l.sort();
    let first: Vec<&str> = l.iter().take(3).map(|s| s.as_str()).collect();
    if l.len() > 3 {
        format!("{} and {} more", first.join(", "), l.len() - 3)
    } else {
        first.join(", ")
    }
}

fn analyze(models: &Models, model_cfg: &Path, module: &Module, numbers: &HashMap<String, f64>, unknown: &[String], found: &dyn Fn(&str) -> bool) -> Option<Analysis> {
    let cfg = models.cfg(model_cfg)?;
    let live = Values::new(numbers.clone());
    // The place of a part hangs on its animation variables. When the plugin does not give one,
    // the start value from the bus's scripts; only when the script says nothing either: 0, and
    // "place uncertain". A name the bus does not know gets 0 (the script may be another variant's).
    let unknown: HashSet<String> = unknown.iter().map(|n| n.to_lowercase()).collect();
    let mut anim_names: Vec<String> = Vec::new();
    for m in &cfg.meshes {
        for a in &m.anims {
            for (n, _) in [&a.shift, &a.turn].into_iter().flatten() {
                if !n.is_empty() && !live.has(n) && !unknown.contains(&n.to_lowercase()) && !anim_names.contains(n) {
                    anim_names.push(n.clone());
                }
            }
        }
    }
    let script = if anim_names.is_empty() {
        Values::default()
    } else {
        let all = models.start_values(model_cfg);
        Values::new(anim_names.iter().filter_map(|n| all.get(&n.to_lowercase()).map(|v| (n.clone(), *v))).collect())
    };
    let value = |n: &str| -> f64 {
        if live.has(n) {
            live.value(n)
        } else if script.has(n) {
            script.value(n)
        } else {
            0.0
        }
    };
    let mut incomplete: Vec<String> = Vec::new();

    /* ---- 1. every existing mesh through its chain ---- */
    let mut per_ident = HashMap::new();
    for (k, m) in cfg.meshes.iter().enumerate() {
        if let Some(id) = m.ident.as_ref().filter(|i| m.exists && !i.is_empty()) {
            per_ident.entry(id.to_lowercase()).or_insert(k);
        }
    }
    let mut cx = Ctx { models, cfg: cfg.clone(), geos: vec![None; cfg.meshes.len()], busy: HashSet::new(), per_ident, value: &value };
    let mut all: Vec<Rc<Geo>> = Vec::new();
    for (k, m) in cfg.meshes.iter().enumerate() {
        if !m.exists || !cfg::inside_view(&cfg, m) {
            continue;
        }
        if let Some(g) = geo_of(k, &mut cx).filter(|g| g.info.valid) {
            all.push(g);
        }
    }

    /* ---- 2. the device's anchors ---- */
    let own: HashSet<String> = module.fields.iter().map(|f| norm_path(&f.part)).chain(module.keys.iter().map(|k| norm_path(&k.part))).collect();
    let actions: HashSet<String> = module.keys.iter().map(|k| k.action.to_lowercase()).collect();
    let is_own = |g: &Geo| own.contains(&norm_path(&g.mesh.path));
    let mut anchors: Vec<Anchor> = Vec::new();
    let mut encrypted_anchors: Vec<Rc<Geo>> = Vec::new();
    for g in &all {
        if !is_own(g) {
            continue;
        }
        let click = g.mesh.clicks.iter().any(|a| actions.contains(&a.to_lowercase()));
        let tm = text_mats(g);
        if !click && tm.is_empty() {
            continue;
        }
        if g.info.encrypted {
            encrypted_anchors.push(g.clone());
            continue;
        }
        for mat in tm {
            let points = points_of(&tris_of(models, g, mat as i64, None));
            if !points.is_empty() {
                anchors.push(Anchor { geo: g.clone(), text: true, mat: mat as i64, middle: middle_of(&points), points });
            }
        }
        if click {
            let points = points_of(&tris_of(models, g, -1, None));
            if !points.is_empty() {
                anchors.push(Anchor { geo: g.clone(), text: false, mat: -1, middle: middle_of(&points), points });
            }
        }
    }

    /* ---- 3. candidates and the vote ---- */
    let cands: Memo<(usize, usize), Cand> = RefCell::default();
    let cand = |g: &Rc<Geo>, mat: usize| -> Option<Rc<Cand>> {
        if let Some(c) = cands.borrow().get(&(g.k, mat)) {
            return c.clone();
        }
        let c = make_cand(models, g, mat, found).map(Rc::new);
        cands.borrow_mut().insert((g.k, mat), c.clone());
        c
    };
    let cands_near = |q: P3, margin: f64| -> Vec<Rc<Cand>> {
        let mut out = Vec::new();
        for g in &all {
            if g.info.encrypted || g.mesh.shadow || !in_box(&g.lo, &g.hi, q, margin) {
                continue;
            }
            for mat in 0..g.info.textures.len() {
                if let Some(k) = cand(g, mat).filter(|k| in_box(&k.lo, &k.hi, q, margin)) {
                    out.push(k);
                }
            }
        }
        out
    };
    // (the box around all anchors, with 3 cm to spare: nothing outside it needs hitting)
    let area = (!anchors.is_empty()).then(|| {
        let (lo, hi) = box_of(&anchors.iter().flat_map(|a| a.points.iter().copied()).collect::<Vec<_>>());
        (sub(lo, [0.03; 3]), add(hi, [0.03; 3]))
    });

    // First only the text fields vote: they make the screen (the AFR 200's 25 keys lie on the
    // display's texture, on another plane). A click field (every mesh with a [mouseevent]) is
    // never another anchor's background, unless nothing is left then.
    let click_geos: HashSet<usize> = all.iter().filter(|g| !g.mesh.clicks.is_empty()).map(|g| g.k).collect();
    // (per surface: the anchors that chose it, and of those the ones shown under the same
    // [visible] as it - the HH20's note of the driver's numbers lies on its own paper and on
    // the larger timetable sheet held in the same clip, and only its paper shows with it)
    let mut votes: Vec<(Rc<Cand>, usize, Seeds, usize)> = Vec::new();
    let mut vote_at: HashMap<(usize, usize), usize> = HashMap::new();
    let rounds: [Vec<&Anchor>; 2] = [anchors.iter().filter(|a| a.text).collect(), anchors.iter().filter(|a| !a.text).collect()];
    for round in &rounds {
        for free in [false, true] {
            if !votes.is_empty() {
                break;
            }
            let no_clicks = !free;
            for a in round {
                let mut under: Vec<(Rc<Cand>, f64, Vec<usize>)> = Vec::new();
                for k in cands_near(a.middle, 0.02) {
                    if !a.text && k.geo.k == a.geo.k {
                        continue;
                    }
                    if no_clicks && click_geos.contains(&k.geo.k) {
                        continue;
                    }
                    if a.text && k.geo.k == a.geo.k && k.mat as i64 == a.mat {
                        continue;
                    }
                    let grid = k.grid.get_or_init(|| Grid::new(&k.tris, 0.016, area));
                    let mut seed: Vec<usize> = Vec::new();
                    let mut ds = Vec::new();
                    for p in &a.points {
                        if let Some(r) = hit_grid(grid, &k.tris, *p, 0.015).filter(|r| r.d >= -0.0005) {
                            ds.push(r.d);
                            if !seed.contains(&r.tri) {
                                seed.push(r.tri);
                            }
                        }
                    }
                    let m = hit_grid(grid, &k.tris, a.middle, 0.015).filter(|m| m.d >= -0.0005);
                    if let Some(m) = m {
                        if !seed.contains(&m.tri) {
                            seed.push(m.tri);
                        }
                    }
                    if m.is_none() && (ds.len() as f64) < a.points.len() as f64 * 0.5 {
                        continue;
                    }
                    let d = m.map(|m| m.d).unwrap_or_else(|| ds.iter().sum::<f64>() / ds.len() as f64);
                    under.push((k, d, seed));
                }
                if under.is_empty() {
                    continue;
                }
                let dmin = under.iter().map(|u| u.1).fold(f64::INFINITY, f64::min);
                for (k, d, seed) in under {
                    if d > dmin + 0.0005 {
                        continue;
                    }
                    let key = (k.geo.k, k.mat);
                    let at = *vote_at.entry(key).or_insert_with(|| {
                        votes.push((k.clone(), 0, Seeds::default(), 0));
                        votes.len() - 1
                    });
                    votes[at].1 += 1;
                    if !a.geo.mesh.visible.is_empty() && same_visible(&a.geo.mesh.visible, &k.geo.mesh.visible) {
                        votes[at].3 += 1;
                    }
                    // (per triangle how many anchors lie on it: so `piece_of` chooses its plane)
                    for z in seed {
                        votes[at].2.add(z, 1);
                    }
                }
            }
        }
    }
    // the most votes; of as many, the surface shown with more of its anchors, then the first
    let mut chosen: Option<(Rc<Cand>, Seeds)> = None;
    let mut best = (0, 0);
    for (k, count, seeds, kin) in &votes {
        let better = match &chosen {
            None => true,
            Some((c, _)) => (*count, *kin) > best || ((*count, *kin) == best && (k.geo.mesh.cfg_index < c.geo.mesh.cfg_index || (k.geo.k == c.geo.k && k.mat < c.mat))),
        };
        if better {
            chosen = Some((k.clone(), seeds.clone()));
            best = (*count, *kin);
        }
    }

    let mut analysis = Analysis {
        cfg: cfg.clone(),
        geos: all.clone(),
        backgrounds: Vec::new(),
        layers: Vec::new(),
        texts: Vec::new(),
        clicks: Vec::new(),
        loose: Vec::new(),
        plane: None,
        base: None,
        incomplete: Vec::new(),
    };

    /* ---- 4. the piece, the plane and the background set ---- */
    let mut plane = None;
    let mut piece: Vec<Tri> = Vec::new();
    if let Some((k, seeds)) = &chosen {
        piece = piece_of(&k.tris, seeds);
        plane = plane_of(&piece);
    }
    if plane.is_none() {
        // no background: the plane of the text fields themselves, as with a text LCD
        let text_tris: Vec<Tri> = anchors.iter().filter(|a| a.text).flat_map(|a| tris_of(models, &a.geo, a.mat, None).as_ref().clone()).collect();
        plane = plane_of(&text_tris);
        if plane.is_some() {
            incomplete.push("no background: the plane comes from the text fields".into());
        }
    }
    let Some(v) = plane else {
        analysis.incomplete = incomplete;
        return Some(analysis);
    };
    analysis.plane = Some(v);

    if let Some((kk, _)) = &chosen {
        analysis.base = Some(kk.geo.clone());
        let piece_grid = Grid::new(&piece, 0.016, None);
        let piece_points = points_of(&piece);
        let (plo, phi) = box_of(&piece_points);
        let mut members: Vec<(Rc<Geo>, usize, Vec<Tri>)> = vec![(kk.geo.clone(), kk.mat, piece.iter().filter(|d| v.faces(d)).cloned().collect())];
        let mut layers: Vec<Layer> = Vec::new();
        let k_name = kk.name.to_lowercase();
        let on_piece = |p: &P3| hit_grid(&piece_grid, &piece, *p, 0.015).is_some_and(|r| r.d >= -0.0005);
        for g in &all {
            if g.info.encrypted || g.mesh.shadow || !overlaps(g, plo, phi, 0.02) {
                continue;
            }
            for mat in 0..g.info.textures.len() {
                if g.k == kk.geo.k && mat == kk.mat {
                    continue;
                }
                let Some(k) = cand(g, mat) else { continue };
                // the same footprint, both ways within 1 mm: another menu of the same screen
                let same = k.tris.iter().all(|d| corners(d).iter().all(|p| hit_grid(&piece_grid, &piece, *p, 0.001).map(|r| r.d).unwrap_or(1.0).abs() < 0.001))
                    && piece_points.iter().all(|p| hit_all(&k.tris, *p, 0.001).map(|r| r.d).unwrap_or(1.0).abs() < 0.001);
                if same {
                    members.push((g.clone(), mat, k.tris.iter().filter(|d| v.faces(d)).cloned().collect()));
                    continue;
                }
                // the same texture in the same plane (the LAWO: Body_01 and Body_02)
                if k.name.to_lowercase() == k_name {
                    let flat: Vec<Tri> = k
                        .tris
                        .iter()
                        .filter(|d| dot(d.n, v.n) > 3f64.to_radians().cos() && corners(d).iter().all(|p| v.distance(*p).abs() < 0.001) && v.within(middle_of(&corners(d)), 0.0))
                        .cloned()
                        .collect();
                    if !flat.is_empty() {
                        members.push((g.clone(), mat, flat));
                        continue;
                    }
                }
                // a layer: all vertices on the piece, from 0.5 mm behind to 15 mm in front
                if k.tris.iter().all(|d| corners(d).iter().all(&on_piece)) {
                    let tris: Vec<Tri> = k.tris.iter().filter(|d| v.faces(d)).cloned().collect();
                    if !tris.is_empty() {
                        layers.push(Layer { geo: g.clone(), mat, tris });
                    }
                }
            }
        }
        // also surfaces without a texture and script layers, which cannot be candidates
        for g in &all {
            if g.info.encrypted || g.mesh.shadow || !overlaps(g, plo, phi, 0.02) {
                continue;
            }
            for mat in 0..g.info.textures.len() {
                if cand(g, mat).is_some() {
                    continue;
                }
                let Some(extra) = extra_of(g, mat, found) else { continue };
                let tris = tris_of(models, g, mat as i64, None);
                if tris.is_empty() {
                    continue;
                }
                let front: Vec<Tri> = tris.iter().filter(|d| v.faces(d)).cloned().collect();
                if !tris.iter().all(|d| corners(d).iter().all(&on_piece)) || front.is_empty() {
                    continue;
                }
                if let Extra::Missing(name) = extra {
                    incomplete.push(format!("texture missing: {name} (layer {})", g.mesh.path));
                    continue;
                }
                layers.push(Layer { geo: g.clone(), mat, tris: front });
            }
        }
        // What lies clearly IN FRONT of the piece with the same footprint is a layer, not a
        // background: the NLC display's glass (0.7 mm in front) else became the base.
        let mut kept = Vec::new();
        for m in members {
            let is_k = m.0.k == kk.geo.k && m.1 == kk.mat;
            if !is_k && !m.2.is_empty() {
                let d = points_of(&m.2).iter().map(|p| v.distance(*p)).sum::<f64>() / (m.2.len() * 3) as f64;
                if d > 0.0002 {
                    layers.push(Layer { geo: m.0, mat: m.1, tris: m.2 });
                    continue;
                }
            }
            kept.push(m);
        }
        let mut members = kept;
        // the base: the member without [visible], else the first in the cfg
        members.sort_by(|a, b| a.0.mesh.cfg_index.cmp(&b.0.mesh.cfg_index).then(a.1.cmp(&b.1)));
        if let Some(i) = members.iter().position(|m| m.0.mesh.visible.is_empty()) {
            let m = members.remove(i);
            members.insert(0, m);
        }
        analysis.base = Some(members[0].0.clone());
        analysis.backgrounds = members.into_iter().filter(|m| !m.2.is_empty()).map(|(geo, mat, tris)| Layer { geo, mat, tris }).collect();
        analysis.layers = layers;
    }

    /* ---- 5. unscrambling: encrypted text and click meshes laid on the plane ---- */
    let background_grid_tris: Vec<Tri> = analysis.backgrounds.iter().flat_map(|l| l.tris.iter().cloned()).chain(piece.iter().cloned()).collect();
    let background_grid = chosen.as_ref().map(|_| Grid::new(&background_grid_tris, 0.006, None));
    let unscramble = |g: &Geo| -> Option<Vec<Option<P3>>> {
        let model = models.model(&g.info)?;
        let count = model.vertices.len() / 3;
        let mut pos = Vec::with_capacity(count);
        let mut hits = 0;
        let pivot: Option<P3> = model.transform.map(|t| [t[12] as f64, t[13] as f64, t[14] as f64]);
        let pivot_usable = pivot.is_some_and(|p| length(p) > 0.001 && length(sub(p, [0.0, 0.1, 0.0])) > 0.001);
        for i in 0..count {
            let raw = [model.vertices[i * 3] as f64, model.vertices[i * 3 + 1] as f64, model.vertices[i * 3 + 2] as f64];
            let mut best: Option<(P3, f64)> = None;
            for perm in PERMS {
                let q = apply(&g.world, [raw[perm[0]], raw[perm[1]], raw[perm[2]]]);
                let d = match &background_grid {
                    Some(grid) => hit_grid(grid, &background_grid_tris, q, 0.005).map(|r| r.d.abs()),
                    // (no readable piece: with the plane itself)
                    None => Some(v.distance(q).abs()).filter(|d| *d <= 0.005 && v.within(q, 0.02)),
                };
                if let Some(d) = d {
                    if best.is_none_or(|b| d < b.1) {
                        best = Some((q, d));
                    }
                }
            }
            if best.is_none() && pivot_usable {
                let p = pivot.unwrap_or_default();
                let mut ds: Vec<([usize; 3], f64)> = PERMS.iter().map(|perm| (*perm, length(sub([raw[perm[0]], raw[perm[1]], raw[perm[2]]], p)))).collect();
                ds.sort_by(|a, b| a.1.total_cmp(&b.1));
                if ds[1].1 - ds[0].1 > 0.05 {
                    let perm = ds[0].0;
                    best = Some((apply(&g.world, [raw[perm[0]], raw[perm[1]], raw[perm[2]]]), 0.0));
                }
            }
            if best.is_some() {
                hits += 1;
            }
            pos.push(best.map(|b| b.0));
        }
        // (a mesh less than three quarters on the plane does not belong)
        (hits as f64 >= count as f64 * 0.75).then_some(pos)
    };

    /* ---- 6. texts: every text material of the cfg that lies on the plane ---- */
    let near_plane = |p: &P3| {
        let d = v.distance(*p);
        (-0.001..=0.015).contains(&d)
    };
    let unscrambled: Memo<usize, Vec<Option<P3>>> = RefCell::default();
    let pos_of = |g: &Geo| -> Option<Rc<Vec<Option<P3>>>> {
        if !g.info.encrypted {
            return Some(Rc::new(world_pos(models, g).iter().map(|p| Some(*p)).collect()));
        }
        if let Some(p) = unscrambled.borrow().get(&g.k) {
            return p.clone();
        }
        let p = unscramble(g).map(Rc::new);
        unscrambled.borrow_mut().insert(g.k, p.clone());
        p
    };
    for g in &all {
        let tm = text_mats(g);
        if tm.is_empty() || (!g.info.encrypted && !in_box(&g.lo, &g.hi, v.o, 1.0)) {
            continue;
        }
        let Some(pos) = pos_of(g) else {
            if g.info.encrypted && is_own(g) {
                incomplete.push(format!("encrypted, not to be laid on the plane: {}", g.mesh.path));
            }
            continue;
        };
        for mat in tm {
            let Some(ctx) = g.contexts[mat].clone() else { continue };
            let all3 = tris_of(models, g, mat as i64, Some(&pos));
            if all3.is_empty() {
                continue;
            }
            let points = points_of(&all3);
            let m = middle_of(&points);
            if !points.iter().all(near_plane) || !v.within(m, 0.02) {
                if is_own(g) && !g.info.encrypted {
                    incomplete.push(format!("text field beside the plane: {} ({:.1} mm)", g.mesh.path, v.distance(m) * 1000.0));
                }
                continue;
            }
            let front: Vec<Tri> = all3.iter().filter(|d| v.faces(d)).cloned().collect();
            if front.is_empty() && !g.info.encrypted {
                continue;
            }
            analysis.texts.push(TextOn {
                geo: g.clone(),
                mat,
                ctx,
                // (with an encrypted mesh the winding says nothing: everything counts)
                tris: if g.info.encrypted { all3.as_ref().clone() } else { front },
                approx: g.info.encrypted,
            });
        }
    }

    /* ---- 7. clicks: every [mouseevent] line on the plane, whichever way it faces ---- */
    for g in &all {
        if g.mesh.clicks.is_empty() || (!g.info.encrypted && !in_box(&g.lo, &g.hi, v.o, 1.0)) {
            continue;
        }
        let Some(pos) = pos_of(g) else {
            if g.info.encrypted && is_own(g) {
                incomplete.push(format!("encrypted, not to be laid on the plane: {}", g.mesh.path));
            }
            continue;
        };
        let points = points_of(&tris_of(models, g, -1, Some(&pos)));
        if points.is_empty() {
            continue;
        }
        let ds: Vec<f64> = points.iter().map(|p| v.distance(*p)).collect();
        let mean = ds.iter().sum::<f64>() / ds.len() as f64;
        let far = ds.iter().fold(0.0f64, |m, d| m.max(d.abs()));
        if mean.abs() > 0.005 || far > 0.01 || !v.within(middle_of(&points), 0.02) {
            // not on the screen but with a place: remember how far off (a key BESIDE the screen
            // or something apart from it)
            let (s, t) = v.st(middle_of(&points));
            let out_s = (v.s0 - s).max(0.0).max(s - v.s1);
            let out_t = (v.t0 - t).max(0.0).max(t - v.t1);
            let mut acts: Vec<String> = Vec::new();
            for a in &g.mesh.clicks {
                if !acts.contains(a) {
                    acts.push(a.clone());
                }
            }
            analysis.loose.push(LooseKey { actions: acts, plane_mm: js_round(mean.abs() * 1000.0), edge_mm: js_round(out_s.hypot(out_t) * 1000.0) });
            continue;
        }
        analysis.clicks.push(ClickOn { geo: g.clone(), points });
    }
    let unscrambled_clicks: Vec<String> = analysis.clicks.iter().filter(|k| k.geo.info.encrypted).map(|k| k.geo.mesh.path.trim().to_string()).collect();
    if !unscrambled_clicks.is_empty() {
        incomplete.push(format!(
            "encrypted, unscrambled on the plane: {} click fields ({}{})",
            unscrambled_clicks.len(),
            unscrambled_clicks.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
            if unscrambled_clicks.len() > 3 { ", ..." } else { "" }
        ));
    }

    /* ---- 8. where a missing value decided the place ---- */
    // Only the animations NOT shared with the background count; what moves all together (the
    // driver's door the ALMEX hangs on) falls away.
    let base_chain = analysis.base.as_ref().map(|b| b.chain.clone()).unwrap_or_default();
    let mut per_var: Vec<(String, HashSet<String>)> = Vec::new();
    let mut from_script: Vec<(String, HashSet<String>)> = Vec::new();
    let mut note = |anims: Vec<Anim>, g: &Geo| {
        for a in anims {
            for (n, _) in [&a.shift, &a.turn].into_iter().flatten() {
                if n.is_empty() || live.has(n) {
                    continue;
                }
                let table = if script.has(n) { &mut from_script } else { &mut per_var };
                let at = match table.iter().position(|(x, _)| x == n) {
                    Some(i) => i,
                    None => {
                        table.push((n.clone(), HashSet::new()));
                        table.len() - 1
                    }
                };
                table[at].1.insert(g.mesh.path.trim().to_string());
            }
        }
    };
    let mut placed: Vec<Rc<Geo>> = Vec::new();
    for g in analysis.texts.iter().map(|t| &t.geo).chain(analysis.clicks.iter().map(|k| &k.geo)).chain(analysis.layers.iter().map(|l| &l.geo)) {
        if !placed.iter().any(|p| p.k == g.k) {
            placed.push(g.clone());
        }
    }
    for g in &placed {
        note(chain_difference(&g.chain, &base_chain), g);
        note(chain_difference(&base_chain, &g.chain), g);
    }
    for g in anchors.iter().map(|a| &a.geo).chain(encrypted_anchors.iter()) {
        if !placed.iter().any(|p| p.k == g.k) {
            note(chain_difference(&base_chain, &g.chain), g);
        }
    }
    for (n, meshes) in &per_var {
        incomplete.push(format!("place uncertain, {n} missing (counted as 0): {}", list_of(meshes)));
    }
    for (n, meshes) in &from_script {
        incomplete.push(format!("place from the script's start value, {n} = {}: {}", script.value(n), list_of(meshes)));
    }
    analysis.incomplete = incomplete;
    Some(analysis)
}

/* ---------------------------------------------------------------- what comes out */

/// The `[mesh]` lines whose `.o3d` exists, as file names, in the game's order.
pub fn mesh_list(model_cfg: &Path) -> Vec<String> {
    let Some(cfg) = cfg::read(model_cfg) else { return Vec::new() };
    cfg.meshes.iter().filter(|m| m.exists).map(|m| m.path.trim().rsplit(['\\', '/']).next().unwrap_or(m.path.trim()).to_string()).collect()
}

/// All number variables the form needs; they are asked of the plugin first, and only then is
/// the form built. The `[visible]`s, `[matl_change]`s, `[matl_lightmap]`s and `[texcoordtrans]`s
/// of everything that can lie on the screen, the animation variables of their chains, and the
/// repaint's variable. The analysis runs here with all animations at 0 and without looking for
/// textures: rather too many than too few.
pub fn needed_numbers(models: &Models, model_cfg: &Path, module: &Module) -> Vec<String> {
    let Some(a) = analyze(models, model_cfg, module, &HashMap::new(), &[], &|_| true) else { return Vec::new() };
    let spell = cfg::spellings(&a.cfg);
    let mut names: Vec<String> = Vec::new();
    let mut put = |n: &str| {
        if n.is_empty() {
            return;
        }
        let fixed = spell.get(&n.to_lowercase()).cloned().unwrap_or_else(|| n.to_string());
        if !names.iter().any(|x| x.eq_ignore_ascii_case(&fixed)) {
            names.push(fixed);
        }
    };
    let own: HashSet<String> = module.fields.iter().map(|f| norm_path(&f.part)).chain(module.keys.iter().map(|k| norm_path(&k.part))).collect();
    let mut set: Vec<Rc<Geo>> = Vec::new();
    let mut add_geo = |g: &Rc<Geo>| {
        if !set.iter().any(|m| m.k == g.k) {
            set.push(g.clone());
        }
    };
    for g in a.geos.iter().filter(|g| own.contains(&norm_path(&g.mesh.path))) {
        add_geo(g);
    }
    for g in a.backgrounds.iter().chain(a.layers.iter()).map(|l| &l.geo).chain(a.texts.iter().map(|t| &t.geo)).chain(a.clicks.iter().map(|k| &k.geo)) {
        add_geo(g);
    }
    for g in &set {
        for (n, _) in &g.mesh.visible {
            put(n);
        }
        for m in &g.mesh.materials {
            if let Some(n) = &m.variable {
                put(n);
            }
            for s in std::iter::once(&m.state).chain(m.items.iter()) {
                if let Some((_, Some(n))) = &s.lightmap {
                    put(n);
                }
                for n in [&s.texcoord_x, &s.texcoord_y].into_iter().flatten() {
                    put(n);
                }
            }
        }
        for anim in &g.chain {
            for (n, _) in [&anim.shift, &anim.turn].into_iter().flatten() {
                put(n);
            }
        }
    }
    if let Some(r) = ctc::read(model_cfg) {
        put(&r.variable);
    }
    names
}

/// What the form is built from.
pub struct Input<'a> {
    pub model_cfg: &'a Path,
    /// The installation; a repaint's texture has to lie inside it.
    pub omsi_dir: &'a Path,
    pub module: &'a Module,
    /// Live values. A missing animation variable takes the start value from the bus's scripts,
    /// else 0 - marked where it touches the place.
    pub numbers: &'a HashMap<String, f64>,
    /// The names the plugin says the bus does not know: 0, never a script's value.
    pub unknown: &'a [String],
    pub font: &'a dyn Fn(&str, bool) -> Option<Font>,
    /// A texture name's absolute path, or `None`.
    pub find_texture: &'a dyn Fn(&str) -> Option<PathBuf>,
}

/// A form built, with the pictures it registered and which keys lie on it.
#[derive(Debug, Clone)]
pub struct Built {
    pub form: Form,
    /// (id, path, flags) of every picture the form refers to.
    pub textures: Vec<(String, PathBuf, Flags)>,
    pub taps_on_screen: Vec<String>,
    /// The device's click parts that do NOT lie on the screen.
    pub loose: Vec<LooseKey>,
}

impl Built {
    /// The actions (lower case) of the keys that lie not on the screen but ON the device.
    pub fn keys_on_device(&self) -> HashSet<String> {
        self.loose.iter().filter(|k| k.plane_mm.hypot(k.edge_mm) <= OFF_THE_SCREEN_MM).flat_map(|k| k.actions.iter().map(|a| a.to_lowercase())).collect()
    }
}

/// Whether a repaint's path may be used: a picture inside the installation.
fn safe_picture(path: &Path, omsi_dir: &Path) -> bool {
    if !omsi_dir.is_absolute() || !path.is_absolute() || !textures::inside(path, omsi_dir) {
        return false;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    ["dds", "tga", "bmp", "jpg", "jpeg", "png"].contains(&ext.as_str())
}

/// The form of one device's screen; see the head of this file. `None` when no screen can be
/// made of it: the device is then its keys.
pub fn build(models: &Models, input: &Input) -> Option<Built> {
    /* ---- textures: the repaint, then the finder, then the registry ---- */
    let live = Values::new(input.numbers.clone());
    let repaints = ctc::read(input.model_cfg);
    let swap: Option<HashMap<String, PathBuf>> = repaints.as_ref().and_then(|r| {
        if !live.has(&r.variable) {
            return None;
        }
        let i = js_round(live.value(&r.variable));
        let pick = (i >= 0.0).then(|| r.list.get(i as usize)).flatten()?;
        Some(ctc::replacements(r, pick))
    });
    let looked: RefCell<HashMap<String, Option<PathBuf>>> = RefCell::new(HashMap::new());
    let repaint_faults: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let path_of = |name: &str| -> Option<PathBuf> {
        if name.is_empty() || script_tex(name) {
            return None;
        }
        let key = name.to_lowercase();
        if let Some(p) = looked.borrow().get(&key) {
            return p.clone();
        }
        let mut p = swap.as_ref().and_then(|s| s.get(&ctc::texture_key(name)).cloned());
        if p.as_ref().is_some_and(|p| !safe_picture(p, input.omsi_dir)) {
            repaint_faults.borrow_mut().push(format!("repaint points outside the installation, ignored: {name}"));
            p = None;
        }
        let p = p.or_else(|| (input.find_texture)(name));
        looked.borrow_mut().insert(key, p.clone());
        p
    };

    let a = analyze(models, input.model_cfg, input.module, input.numbers, input.unknown, &|n| path_of(n).is_some())?;
    let v = a.plane?;
    if a.backgrounds.is_empty() && a.texts.is_empty() {
        return None;
    }
    let mut incomplete = a.incomplete.clone();
    incomplete.extend(repaint_faults.borrow().iter().cloned());
    let spell = cfg::spellings(&a.cfg);
    let nums: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let strs: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let num_index = |n: &str| -> usize {
        let fixed = spell.get(&n.to_lowercase()).cloned().unwrap_or_else(|| n.to_string());
        let mut nums = nums.borrow_mut();
        if let Some(k) = nums.iter().position(|x| x.eq_ignore_ascii_case(&fixed)) {
            return k;
        }
        nums.push(fixed);
        nums.len() - 1
    };
    let str_index = |n: &str| -> usize {
        let mut strs = strs.borrow_mut();
        if let Some(k) = strs.iter().position(|x| x == n) {
            return k;
        }
        strs.push(n.to_string());
        strs.len() - 1
    };
    let visible_of = |m: &Mesh| -> Vec<Visible> { m.visible.iter().map(|(n, value)| Visible { num: num_index(n), value: *value }).collect() };
    let registered: RefCell<Vec<(String, PathBuf, Flags)>> = RefCell::new(Vec::new());
    let register = |path: &Path, flags: Flags| -> String {
        let id = textures::id_of(path, &flags);
        let mut r = registered.borrow_mut();
        if !r.iter().any(|x| x.0 == id) {
            r.push((id.clone(), path.to_path_buf(), flags.clean()));
        }
        id
    };

    /* ---- coordinates ---- */
    let width = v.s1 - v.s0;
    let height = v.t1 - v.t0;
    let to_st = |p: P3| -> (f64, f64) {
        let w = sub(p, v.o);
        ((dot(w, v.right) - v.s0) / width, (v.t1 - dot(w, v.up)) / height)
    };
    let depth_mm = |points: &[P3]| -> f64 { points.iter().map(|p| v.distance(*p)).sum::<f64>() / points.len().max(1) as f64 * 1000.0 };

    // The triangles as s t u v per point. u and v per Direct3D (v = 0 at the top); per part
    // u' = u - floor(u_middle), v' = v - floor(v_middle): the ALMEX's screen (v -0.88..-0.12)
    // comes out on rows 122.9..901.2 of its picture.
    let tris_out = |tris: &[Tri], uvs: &[f32]| -> Vec<f64> {
        let uv = |i: u32, j: usize| uvs.get(i as usize * 2 + j).copied().unwrap_or(0.0) as f64;
        let (mut u0, mut u1, mut v0, mut v1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for d in tris {
            for i in d.i {
                u0 = u0.min(uv(i, 0));
                u1 = u1.max(uv(i, 0));
                v0 = v0.min(uv(i, 1));
                v1 = v1.max(uv(i, 1));
            }
        }
        let du = ((u0 + u1) / 2.0).floor();
        let dv = ((v0 + v1) / 2.0).floor();
        let mut out = Vec::with_capacity(tris.len() * 12);
        for d in tris {
            for (p, i) in [(d.a, d.i[0]), (d.b, d.i[1]), (d.c, d.i[2])] {
                let (s, t) = to_st(p);
                out.extend([round5(s), round5(t), round5(uv(i, 0) - du), round5(uv(i, 1) - dv)]);
            }
        }
        out
    };
    let notes: RefCell<Vec<String>> = RefCell::new(Vec::new());

    /* ---- an image or colour part ---- */
    let image_part = |g: &Geo, mat: usize, tris: &[Tri], role: &str| -> Option<Part> {
        let ctx = ctx_of(g, mat);
        let model = models.model(&g.info)?;
        let alpha = alpha_of(ctx);
        let address = ctx.and_then(|c| c.address).unwrap_or(cfg::Address::Wrap).name().to_string();
        let name = texture_name_of(g, mat);
        let colour = colour_of(g, mat);
        let mut part = Part {
            kind: Kind::Image,
            mesh: g.mesh.index,
            visible: visible_of(&g.mesh),
            tris: tris_out(tris, &model.uvs),
            depth: round5(depth_mm(&points_of(tris))),
            alpha,
            address,
            texture: None,
            freetex: None,
            choice: None,
            color: None,
            text: None,
        };
        let transmap = ctx.and_then(|c| c.state.transmap.clone()).filter(|t| !t.is_empty());
        if transmap.as_deref().is_some_and(script_tex) || script_tex(&name) {
            notes.borrow_mut().push(format!("script texture, not drawable: {} on {}", transmap.unwrap_or(name), g.mesh.path));
            part.kind = Kind::Script;
            return Some(part);
        }
        if name.is_empty() {
            part.kind = Kind::Color;
            part.color = Some(colour);
            return Some(part);
        }
        if ctx.is_some_and(|c| c.state.texcoord_x.is_some() || c.state.texcoord_y.is_some()) {
            notes.borrow_mut().push(format!("texcoordtrans not drawn: {}", g.mesh.path));
        }
        let mut trans = None;
        if let Some(t) = &transmap {
            trans = path_of(t);
            if trans.is_none() {
                notes.borrow_mut().push(format!("transmap missing: {t} ({})", g.mesh.path));
            }
        }
        let path = path_of(&name);
        if let Some(p) = &path {
            part.texture = Some(register(p, Flags { alpha, trans, crop: None }));
        }
        if alpha == 2 {
            part.color = Some(colour);
        }
        if let Some((_, var)) = ctx.and_then(|c| c.state.freetex.as_ref()).filter(|f| !f.1.is_empty()) {
            part.freetex = Some(str_index(var));
        }
        if path.is_none() && part.freetex.is_none() {
            notes.borrow_mut().push(format!("texture missing: {name} ({role} {})", g.mesh.path));
            return None;
        }
        // [matl_change] <tex> <n> <var>: at value k item k's picture; an item with a
        // [matl_nightmap] shows that (the ALMEX's red delay box); one without keeps the own
        if let Some(c) = ctx.filter(|c| c.change && !c.items.is_empty()) {
            if let Some(var) = &c.variable {
                let items = c
                    .items
                    .iter()
                    .map(|item| {
                        let item_name = item.freetex.as_ref().map(|f| f.0.clone()).filter(|d| !d.is_empty()).or_else(|| item.nightmap.clone()).unwrap_or_default();
                        let item_path = if item_name.is_empty() { None } else { path_of(&item_name) };
                        if !item_name.is_empty() && item_path.is_none() {
                            notes.borrow_mut().push(format!("texture missing: {item_name} (item of {})", g.mesh.path));
                        }
                        match item_path {
                            Some(p) => Some(register(&p, Flags::alpha(item.alpha.unwrap_or(alpha)))),
                            None => part.texture.clone(),
                        }
                    })
                    .collect();
                part.choice = Some(Choice { num: num_index(var), items });
            }
        }
        Some(part)
    };

    /* ---- text parts ---- */
    let fonts_needed: RefCell<Vec<(String, bool)>> = RefCell::new(Vec::new());
    let text_part = |t: &TextOn| -> Option<Part> {
        let k = t.ctx.text?;
        let Some(block) = a.cfg.text.get(k) else {
            notes.borrow_mut().push(format!("[useTextTexture] {k} does not exist: {}", t.geo.mesh.path));
            return None;
        };
        if !block.valid {
            notes.borrow_mut().push(format!("the text block on line {} cannot be read", block.line));
            return None;
        }
        let full = block.fc & 1 == 1;
        {
            let mut f = fonts_needed.borrow_mut();
            match f.iter_mut().find(|x| x.0 == block.font) {
                Some(x) => x.1 |= full,
                None => f.push((block.font.clone(), full)),
            }
        }
        let text = TextSpec { str: str_index(&block.source), w: block.w, h: block.h, font: block.font.clone(), fc: block.fc, color: block.color, align: block.align, grid: block.grid };
        let model = models.model(&t.geo.info)?;
        let tris = if t.approx {
            // Encrypted: the uvs are not to be trusted. One line of the font, in the middle and
            // over the full width - so the game sets a text of one line (y0 = (H - height) / 2).
            let points: Vec<(f64, f64)> = points_of(&t.tris).into_iter().map(to_st).collect();
            let s0 = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let s1 = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let t0 = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let t1 = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            let fh = (input.font)(&block.font, full).map(|f| f.height).unwrap_or(block.h);
            let y0 = (block.h - fh) / 2;
            let (vt, vb) = (y0 as f64 / block.h as f64, (y0 + fh) as f64 / block.h as f64);
            notes.borrow_mut().push(format!("approximated (encrypted): {}", t.geo.mesh.path));
            [s0, t0, 0.0, vt, s1, t0, 1.0, vt, s1, t1, 1.0, vb, s0, t0, 0.0, vt, s1, t1, 1.0, vb, s0, t1, 0.0, vb].map(round5).to_vec()
        } else {
            tris_out(&t.tris, &model.uvs)
        };
        Some(Part {
            kind: Kind::Text,
            mesh: t.geo.mesh.index,
            visible: visible_of(&t.geo.mesh),
            tris,
            depth: round5(depth_mm(&points_of(&t.tris))),
            alpha: t.ctx.state.alpha.unwrap_or(0),
            address: t.ctx.address.unwrap_or(cfg::Address::Wrap).name().to_string(),
            texture: None,
            freetex: None,
            choice: None,
            color: None,
            text: Some(text),
        })
    };

    /* ---- in drawing order ---- */
    let mut parts: Vec<Part> = Vec::new();
    for l in &a.backgrounds {
        if let Some(p) = image_part(&l.geo, l.mat, &l.tris, "background") {
            parts.push(p);
        }
    }
    if parts.is_empty() && !a.backgrounds.is_empty() {
        notes.borrow_mut().push("no background at all to show".into());
    }
    let mut over: Vec<(Part, u8, usize)> = Vec::new();
    for l in &a.layers {
        if let Some(p) = image_part(&l.geo, l.mat, &l.tris, "layer") {
            over.push((p, 0, l.geo.mesh.cfg_index * 1000 + l.mat));
        }
    }
    for t in &a.texts {
        if let Some(p) = text_part(t) {
            over.push((p, 1, t.geo.mesh.cfg_index * 1000 + t.mat));
        }
    }
    // Layers and texts by depth; within 0.2 mm of each other as deep, and then the layers
    // first and the texts after, each in cfg order.
    over.sort_by(|x, y| x.0.depth.total_cmp(&y.0.depth));
    let mut groups: Vec<Vec<(Part, u8, usize)>> = Vec::new();
    for x in over {
        match groups.last_mut() {
            Some(g) if x.0.depth - g[0].0.depth < 0.2 => g.push(x),
            _ => groups.push(vec![x]),
        }
    }
    for mut g in groups {
        g.sort_by(|x, y| x.1.cmp(&y.1).then(x.2.cmp(&y.2)));
        parts.extend(g.into_iter().map(|x| x.0));
    }

    /* ---- touch fields ---- */
    let labels: HashMap<String, String> = input.module.keys.iter().map(|k| (k.action.to_lowercase(), k.label.clone())).collect();
    let mut taps: Vec<(Tap, f64)> = Vec::new();
    for k in &a.clicks {
        let st: Vec<(f64, f64)> = k.points.iter().map(|p| to_st(*p)).collect();
        let x0 = st.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x1 = st.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y0 = st.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y1 = st.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        // one tap per action: 84 [mesh] lines of a fleet have the same [mouseevent] twice
        let mut seen: Vec<&String> = Vec::new();
        for action in &k.geo.mesh.clicks {
            if seen.contains(&action) {
                continue;
            }
            seen.push(action);
            taps.push((
                Tap {
                    trigger: action.clone(),
                    label: labels.get(&action.to_lowercase()).cloned().unwrap_or_else(|| action.clone()),
                    mesh: k.geo.mesh.index,
                    visible: visible_of(&k.geo.mesh),
                    x: round5(x0),
                    y: round5(y0),
                    w: round5(x1 - x0),
                    h: round5(y1 - y0),
                    depth: round5(depth_mm(&k.points)),
                },
                (x1 - x0) * (y1 - y0),
            ));
        }
    }
    // The front one last; what lies within 0.2 mm as deep, the smallest on top (uncertain:
    // the game probably takes the first hit along the mouse ray; not measured).
    taps.sort_by(|x, y| x.0.depth.total_cmp(&y.0.depth));
    let mut tap_groups: Vec<Vec<(Tap, f64)>> = Vec::new();
    for t in taps {
        match tap_groups.last_mut() {
            Some(g) if t.0.depth - g[0].0.depth < 0.2 => g.push(t),
            _ => tap_groups.push(vec![t]),
        }
    }
    let taps: Vec<Tap> = tap_groups
        .into_iter()
        .flat_map(|mut g| {
            g.sort_by(|x, y| y.1.total_cmp(&x.1));
            g.into_iter().map(|t| t.0)
        })
        .collect();

    /* ---- fonts ---- */
    let mut fonts = BTreeMap::new();
    for (name, full) in fonts_needed.borrow().iter() {
        match (input.font)(name, *full) {
            Some(f) => {
                fonts.insert(name.clone(), f);
            }
            None => notes.borrow_mut().push(format!("font missing: {name}")),
        }
    }

    // A picture without text and without one touch field is no screen: then they are keys in
    // the room (a change machine's coins) and a surface under them by chance.
    if !parts.iter().any(|p| p.kind == Kind::Text) && taps.is_empty() {
        return None;
    }
    if !parts.iter().any(|p| p.kind == Kind::Image || p.kind == Kind::Text) {
        return None;
    }
    incomplete.extend(notes.into_inner());
    let mut seen = HashSet::new();
    incomplete.retain(|n| seen.insert(n.clone()));
    let mut form = Form {
        version: 1,
        id: String::new(),
        device: input.module.id.clone(),
        aspect: round5(width / height),
        blank: "#000000".into(),
        parts,
        taps,
        fonts,
        strs: strs.into_inner(),
        nums: nums.into_inner(),
        incomplete,
    };
    // the id follows from the content: the same form, the same id
    form.id = textures::hex20(serde_json::to_string(&form).unwrap_or_default().as_bytes());
    let mut on_screen: Vec<String> = Vec::new();
    for t in &form.taps {
        if !on_screen.contains(&t.trigger) {
            on_screen.push(t.trigger.clone());
        }
    }
    Some(Built { form, textures: registered.into_inner(), taps_on_screen: on_screen, loose: a.loose })
}
