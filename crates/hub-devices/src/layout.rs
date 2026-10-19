//! Where the parts of a device lie (the career add-on's `core/busvorm.ts`): a `model.cfg` says
//! which part carries a text field and which is a key, but not where on the device; the `.o3d`
//! beside it does. The keys that are not on the screen are laid out in the rows they have on
//! the device from that: measured, not guessed. On the ALMEX of the Hamburg bus the stop list
//! begins at the BOTTOM and the keypad has the 1 at the TOP, unlike a phone.
//!
//! A device is seldom one plane: the ALMEX's screen stands up, a row of function keys lies
//! under it and the keypad lies almost flat further on. So it is cut into small planes first
//! (parts close together and facing the same way), each with its own up and right, and the
//! planes come under one another by their height.

use crate::o3d;
use std::collections::HashMap;
use std::path::Path;

pub type V3 = [f64; 3];

/// The box a part fits in, with the way it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxOf {
    pub lo: V3,
    pub hi: V3,
    /// The middle.
    pub m: V3,
    /// The direction its vertices spread least in: across the flat part.
    pub n: V3,
}

/// A part on a plane: 0 is left and top, 1 right and bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

fn length(v: V3) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}
fn unit(v: V3) -> V3 {
    let l = length(v);
    if l > 1e-12 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0; 3]
    }
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        values[(n - 1) / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

/// The direction a cloud of points spreads least in: Jacobi on the scatter matrix.
fn smallest_axis(vertices: &[f32], mid: V3) -> V3 {
    let mut c = [[0f64; 3]; 3];
    for p in vertices.as_chunks::<3>().0 {
        let d = [p[0] as f64 - mid[0], p[1] as f64 - mid[1], p[2] as f64 - mid[2]];
        for a in 0..3 {
            for b in 0..3 {
                c[a][b] += d[a] * d[b];
            }
        }
    }
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..12 {
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            if c[p][q].abs() < 1e-16 {
                continue;
            }
            let angle = 0.5 * (2.0 * c[p][q]).atan2(c[p][p] - c[q][q]);
            let (co, si) = (angle.cos(), angle.sin());
            for row in c.iter_mut() {
                let (a, b) = (row[p], row[q]);
                row[p] = co * a + si * b;
                row[q] = -si * a + co * b;
            }
            let (rp, rq) = (c[p], c[q]);
            for k in 0..3 {
                c[p][k] = co * rp[k] + si * rq[k];
                c[q][k] = -si * rp[k] + co * rq[k];
            }
            for row in v.iter_mut() {
                let (a, b) = (row[p], row[q]);
                row[p] = co * a + si * b;
                row[q] = -si * a + co * b;
            }
        }
    }
    let mut least = 0;
    for axis in [1, 2] {
        if c[axis][axis] < c[least][least] {
            least = axis;
        }
    }
    let n = unit([v[0][least], v[1][least], v[2][least]]);
    if length(n) > 0.5 {
        n
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// The box of one part. The vertices are already in the bus's measures; the 0x79 matrix is
/// not applied again.
pub fn box_of_o3d(path: &Path) -> Option<BoxOf> {
    let m = o3d::read_file(path)?;
    if m.vertices.len() < 3 || m.encrypted {
        return None;
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in m.vertices.as_chunks::<3>().0 {
        if p.iter().any(|x| !x.is_finite()) {
            continue;
        }
        for j in 0..3 {
            lo[j] = lo[j].min(p[j] as f64);
            hi[j] = hi[j].max(p[j] as f64);
        }
    }
    if !lo[0].is_finite() {
        return None;
    }
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
    Some(BoxOf { lo, hi, m: mid, n: smallest_axis(&m.vertices, mid) })
}

/// The boxes of a list of parts' paths (as the cfg has them, relative to its folder).
pub fn boxes_of_parts(model_dir: &Path, parts: &[String]) -> HashMap<String, BoxOf> {
    let mut out = HashMap::new();
    for part in parts {
        if out.contains_key(part) {
            continue;
        }
        let pieces: Vec<&str> = part.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
        if pieces.is_empty() {
            continue;
        }
        let file = pieces.iter().fold(model_dir.to_path_buf(), |d, p| d.join(p));
        if let Some(b) = box_of_o3d(&file) {
            out.insert(part.clone(), b);
        }
    }
    out
}

/// How big a device is at most: further away is not part of it (an inner display named after
/// the ALMEX three metres away, parts parked outside the bus that an animation puts in).
const RADIUS_M: f64 = 1.2;
/// How far apart two parts of one small plane are at most.
const PLANE_M: f64 = 0.25;
/// How far the faces of one small plane may differ (a little over 25 degrees).
const PLANE_ANGLE: f64 = 0.9;

/// What belongs to the device: the rest lies too far away.
fn together(boxes: &[(usize, BoxOf)]) -> Vec<(usize, BoxOf)> {
    if boxes.is_empty() {
        return Vec::new();
    }
    let c = [0, 1, 2].map(|j| median(&mut boxes.iter().map(|(_, b)| b.m[j]).collect::<Vec<_>>()));
    let near: Vec<(usize, BoxOf)> = boxes.iter().filter(|(_, b)| length([b.m[0] - c[0], b.m[1] - c[1], b.m[2] - c[2]]) <= RADIUS_M).copied().collect();
    if near.is_empty() {
        boxes.to_vec()
    } else {
        near
    }
}

/// The parts in small planes: close together and facing the same way, chained.
fn small_planes(boxes: &[(usize, BoxOf)]) -> Vec<Vec<usize>> {
    let n = boxes.len();
    let mut up: Vec<usize> = (0..n).collect();
    fn find(up: &mut [usize], mut i: usize) -> usize {
        while up[i] != i {
            i = up[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (boxes[i].1, boxes[j].1);
            if length([a.m[0] - b.m[0], a.m[1] - b.m[1], a.m[2] - b.m[2]]) > PLANE_M || dot(a.n, b.n).abs() < PLANE_ANGLE {
                continue;
            }
            let (wi, wj) = (find(&mut up, i), find(&mut up, j));
            if wi != wj {
                up[wj] = wi;
            }
        }
    }
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        let w = find(&mut up, i);
        match groups.iter_mut().find(|g| g.0 == w) {
            Some(g) => g.1.push(i),
            None => groups.push((w, vec![i])),
        }
    }
    groups.into_iter().map(|g| g.1).collect()
}

/// The angle a flat keypad was laid down at: most keys point to a neighbour along a row or a
/// column, at right angles; their angles taken four-fold give the grid's.
fn grid_angle(points: &[[f64; 2]]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let (mut co, mut si) = (0.0, 0.0);
    for (i, p) in points.iter().enumerate() {
        let mut nearest = f64::INFINITY;
        let (mut dx, mut dz) = (0.0, 0.0);
        for (j, q) in points.iter().enumerate() {
            if i == j {
                continue;
            }
            let (ax, az) = (q[0] - p[0], q[1] - p[1]);
            let d = ax.hypot(az);
            if d < 1e-6 || d >= nearest {
                continue;
            }
            nearest = d;
            dx = ax;
            dz = az;
        }
        if !nearest.is_finite() {
            continue;
        }
        let a = dz.atan2(dx);
        co += (4.0 * a).cos();
        si += (4.0 * a).sin();
    }
    if co.hypot(si) < 1e-9 {
        0.0
    } else {
        si.atan2(co) / 4.0
    }
}

/// The two axes of a flat device from its grid's angle; up is the one pointing forward AND
/// right (the driver sits left at the front and reads a console to his right away from him).
fn flat_axes(points: &[[f64; 2]]) -> (V3, V3) {
    let a = grid_angle(points);
    let cands = [[a.cos(), a.sin()], [-a.sin(), a.cos()]];
    let mut up: Option<[f64; 2]> = None;
    for c in cands {
        for k in [1.0, -1.0] {
            let (kx, kz) = (c[0] * k, c[1] * k);
            if kx >= -1e-9 && kz >= -1e-9 {
                up = Some([kx, kz]);
            }
        }
    }
    let up = up.unwrap_or_else(|| {
        let best = if cands[0][0].abs() >= cands[1][0].abs() { cands[0] } else { cands[1] };
        if best[0] >= 0.0 {
            best
        } else {
            [-best[0], -best[1]]
        }
    });
    ([up[1], 0.0, -up[0]], [up[0], 0.0, up[1]])
}

/// The two axes of a plane: right and up.
fn axes_of(n: V3, points: &[[f64; 2]]) -> (V3, V3) {
    if dot([0.0, 1.0, 0.0], n).abs() > 0.93 && points.len() >= 3 {
        return flat_axes(points);
    }
    let upright = dot([0.0, 1.0, 0.0], n);
    let mut up = if upright.abs() > 0.93 { unit([-n[0] * n[2], -n[1] * n[2], 1.0 - n[2] * n[2]]) } else { unit([-n[0] * upright, 1.0 - n[1] * upright, -n[2] * upright]) };
    if length(up) < 0.5 {
        up = [0.0, 1.0, 0.0];
    }
    let mut right = unit(cross(up, n));
    if length(right) < 0.5 {
        right = [1.0, 0.0, 0.0];
    }
    if right[0] < 0.0 || (right[0].abs() < 0.2 && right[2] > 0.0) {
        right = [-right[0], -right[1], -right[2]];
    }
    let mut top = unit(cross(n, right));
    if length(top) < 0.5 {
        top = [0.0, 1.0, 0.0];
    }
    if top[1] < -0.2 || (top[1].abs() <= 0.2 && top[2] < 0.0) {
        top = [-top[0], -top[1], -top[2]];
    }
    (right, top)
}

/// The furthest points of a box along an axis.
fn along(b: &BoxOf, axis: V3) -> (f64, f64) {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for x in [b.lo[0], b.hi[0]] {
        for y in [b.lo[1], b.hi[1]] {
            for z in [b.lo[2], b.hi[2]] {
                let w = x * axis[0] + y * axis[1] + z * axis[2];
                lo = lo.min(w);
                hi = hi.max(w);
            }
        }
    }
    (lo, hi)
}

/// The mean direction of a group, all normals turned the same way.
fn direction_of(boxes: &[BoxOf]) -> V3 {
    let Some(first) = boxes.first().map(|b| b.n) else { return [0.0, 0.0, 1.0] };
    let mut s = [0.0; 3];
    for b in boxes {
        let k = if dot(b.n, first) < 0.0 { -1.0 } else { 1.0 };
        for (sj, nj) in s.iter_mut().zip(b.n) {
            *sj += nj * k;
        }
    }
    let n = unit(s);
    if length(n) > 0.5 {
        n
    } else {
        first
    }
}

/// Each part's place on the plane they lie on together, measured in the plane itself.
fn places_on_plane(boxes: &[(usize, BoxOf)]) -> HashMap<usize, Place> {
    let mut out = HashMap::new();
    if boxes.is_empty() {
        return out;
    }
    let n = direction_of(&boxes.iter().map(|b| b.1).collect::<Vec<_>>());
    let points: Vec<[f64; 2]> = boxes.iter().map(|(_, b)| [b.m[0], b.m[2]]).collect();
    let (right, up) = axes_of(n, &points);
    let across: Vec<(f64, f64)> = boxes.iter().map(|(_, b)| along(b, right)).collect();
    let high: Vec<(f64, f64)> = boxes.iter().map(|(_, b)| along(b, up)).collect();
    let b0 = across.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
    let b1 = across.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
    let h0 = high.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
    let h1 = high.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
    let (bb, hh) = (b1 - b0, h1 - h0);
    for (k, (i, _)) in boxes.iter().enumerate() {
        out.insert(
            *i,
            Place {
                left: if bb > 1e-6 { (across[k].0 - b0) / bb } else { 0.0 },
                right: if bb > 1e-6 { (across[k].1 - b0) / bb } else { 1.0 },
                // (the up axis points up, the screen counts from the top down)
                top: if hh > 1e-6 { 1.0 - (high[k].1 - h0) / hh } else { 0.0 },
                bottom: if hh > 1e-6 { 1.0 - (high[k].0 - h0) / hh } else { 1.0 },
            },
        );
    }
    out
}

/// Keys at the same place do not belong in one row: a touch screen puts other keys at the same
/// place per menu (the ALMEX's `almex_click_linie` lies right over its key 2). So in layers
/// first, each key in the first layer it covers nothing in; the biggest layer first.
fn layers_of(items: &[usize], place: &HashMap<usize, Place>) -> Vec<Vec<usize>> {
    let mut layers: Vec<(Vec<usize>, Vec<Place>)> = Vec::new();
    for &i in items {
        let Some(mine) = place.get(&i).copied() else {
            if layers.is_empty() {
                layers.push((Vec::new(), Vec::new()));
            }
            layers[0].0.push(i);
            continue;
        };
        let own = ((mine.right - mine.left) * (mine.bottom - mine.top)).max(1e-9);
        let free = layers.iter().position(|(_, places)| {
            !places.iter().any(|o| {
                let w = o.right.min(mine.right) - o.left.max(mine.left);
                let h = o.bottom.min(mine.bottom) - o.top.max(mine.top);
                w > 0.0 && h > 0.0 && w * h > own * 0.3
            })
        });
        let k = free.unwrap_or_else(|| {
            layers.push((Vec::new(), Vec::new()));
            layers.len() - 1
        });
        layers[k].0.push(i);
        layers[k].1.push(mine);
    }
    layers.sort_by_key(|l| std::cmp::Reverse(l.0.len()));
    layers.into_iter().map(|l| l.0).collect()
}

/// Things in rows as they lie on a plane: one row when they overlap in height (compared with
/// the FIRST of the row, else a big panel chains itself into one row), left to right, rows top
/// to bottom; what has no place at the end, three a row.
fn rows_of(items: &[usize], place: &HashMap<usize, Place>) -> Vec<Vec<usize>> {
    let mut with: Vec<(usize, Place)> = items.iter().filter_map(|i| place.get(i).map(|p| (*i, *p))).collect();
    let without: Vec<usize> = items.iter().copied().filter(|i| !place.contains_key(i)).collect();
    with.sort_by(|a, b| a.1.top.total_cmp(&b.1.top).then(a.1.left.total_cmp(&b.1.left)));
    let mid = |p: &Place| (p.top + p.bottom) / 2.0;
    let mut rows: Vec<Vec<(usize, Place)>> = Vec::new();
    for r in with {
        let joins = rows.last().and_then(|row| row.first()).is_some_and(|first| {
            let h = (first.1.bottom - first.1.top).min(r.1.bottom - r.1.top);
            (mid(&r.1) - mid(&first.1)).abs() < h.max(1e-6) * 0.5
        });
        if joins {
            rows.last_mut().unwrap().push(r);
        } else {
            rows.push(vec![r]);
        }
    }
    let mut out: Vec<Vec<usize>> = rows
        .into_iter()
        .map(|mut row| {
            row.sort_by(|a, b| a.1.left.total_cmp(&b.1.left));
            row.into_iter().map(|r| r.0).collect()
        })
        .collect();
    for chunk in without.chunks(3) {
        out.push(chunk.to_vec());
    }
    out
}

/// The layout of a device's things: rows from top to bottom, each left to right. First apart
/// in small planes (the screen, the row of function keys, the keypad), each with its own up;
/// the planes under one another by height. What could not be measured comes last, three a row.
pub fn rows<T>(things: &[T], box_of: impl Fn(&T) -> Option<BoxOf>) -> Vec<Vec<usize>> {
    let mut boxes: Vec<(usize, BoxOf)> = Vec::new();
    let mut without: Vec<usize> = Vec::new();
    for (i, t) in things.iter().enumerate() {
        match box_of(t) {
            Some(b) => boxes.push((i, b)),
            None => without.push(i),
        }
    }
    let near = together(&boxes);
    let near_set: Vec<usize> = near.iter().map(|b| b.0).collect();
    let mut groups: Vec<(Vec<(usize, BoxOf)>, f64)> = small_planes(&near)
        .into_iter()
        .map(|g| {
            let members: Vec<(usize, BoxOf)> = g.iter().map(|&k| near[k]).collect();
            let height = members.iter().map(|m| m.1.hi[1]).fold(f64::NEG_INFINITY, f64::max);
            (members, height)
        })
        .collect();
    groups.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out = Vec::new();
    for (members, _) in groups {
        let places = places_on_plane(&members);
        let ids: Vec<usize> = members.iter().map(|m| m.0).collect();
        for layer in layers_of(&ids, &places) {
            out.extend(rows_of(&layer, &places));
        }
    }
    let rest: Vec<usize> = boxes.iter().map(|b| b.0).filter(|i| !near_set.contains(i)).chain(without).collect();
    for chunk in rest.chunks(3) {
        out.push(chunk.to_vec());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(x: f64, y: f64) -> BoxOf {
        BoxOf { lo: [x - 0.005, y - 0.005, 1.0], hi: [x + 0.005, y + 0.005, 1.0], m: [x, y, 1.0], n: [0.0, 0.0, 1.0] }
    }

    #[test]
    fn a_keypad_upright_reads_in_rows_from_the_top_left() {
        // 1 2 3 on top, 4 5 6 under them (given shuffled)
        let keys = [key(0.02, 1.0), key(0.0, 1.02), key(0.04, 1.0), key(0.02, 1.02), key(0.0, 1.0), key(0.04, 1.02)];
        let r = rows(&keys, |b| Some(*b));
        assert_eq!(r, vec![vec![1, 3, 5], vec![4, 0, 2]]);
    }

    #[test]
    fn what_cannot_be_measured_comes_last() {
        let things = [Some(key(0.0, 1.0)), None, Some(key(0.02, 1.0)), None];
        let r = rows(&things, |b| *b);
        assert_eq!(r, vec![vec![0, 2], vec![1, 3]]);
    }
}
