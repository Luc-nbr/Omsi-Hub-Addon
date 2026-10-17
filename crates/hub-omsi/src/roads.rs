//! A map's roads, read from its tiles: what the route map, the duty map and the fleet map draw,
//! and the way a timetable trip drives over them. Written from the file formats (openOMSI's
//! `docs/FORMATS.md` describes them), not from any game's code.
//!
//! The files:
//! * `global.cfg`'s `[map]` - a tile's x, y and file (`map::GlobalCfg::tiles`, `tile_files`); a
//!   tile's edge is 300 m (371.9 m with `[worldcoordinates]`), its places run from its corner;
//! * `tile_x_y.map` - its splines (`[spline]`, `[spline_h]`: `0`, the `.sli` file, the ID, the
//!   previous and the next spline's IDs - one line before version 11 -, x, height, y, heading,
//!   length, radius - 0 straight, above 0 turning right -, then gradients, cant and skew, and an
//!   optional `mirror`), its objects (`[object]`: `0`, the file, the ID, x, y, z, …) and the
//!   objects along its splines (`[splineAttachement]`: `0`, the file, the ID, the spline's place in
//!   the tile, the lateral offset, the height, the distance along it, …);
//! * a spline's `.sli` - its lanes (`[path]`: kind - 0 street, 1 pavement, 2 rail -, lateral
//!   offset with right positive, height, width, direction - 0 along, 1 against, 2 both);
//! * a trip's `.ttr` (`timetable::Track`) - the lanes it drives: a spline's ID, its lane's place
//!   in the `.sli`, the tile's place in `[map]`.
//!
//! A spline runs from its place at its heading (degrees clockwise from north; x east, y north): a
//! straight line, or an arc of its radius that turns right (left for a radius below 0). A
//! `mirror`ed spline has its lanes on the other side, running the other way. What lies on an
//! object (a junction's own lanes, `.sco`) is not read: a trip's route bridges it with a straight
//! line.

use crate::map::GlobalCfg;
use crate::text::{parse_f64, parse_i32, CfgFile};
use crate::timetable::{parse_i64, Track};
use crate::vfs::Vfs;
use std::collections::HashMap;
use std::path::Path;

/// A place in metres: x east, y north.
pub type P = [f64; 2];

/// A lane of a spline's cross-section (`[path]` of its `.sli`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Lane {
    /// 0 street, 1 pavement, 2 rail, 3 flight path.
    pub kind: i32,
    /// From the spline's line, right positive.
    pub offset: f64,
    pub width: f64,
    /// 0 along the spline, 1 against it, 2 both ways.
    pub direction: i32,
}

/// A spline file's lanes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SplineFile {
    pub lanes: Vec<Lane>,
}

impl SplineFile {
    pub fn parse(cfg: &CfgFile) -> SplineFile {
        let mut lanes = Vec::new();
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            if k.eq_ignore_ascii_case("path") {
                let v = r.values(5);
                if v.len() == 5 {
                    lanes.push(Lane { kind: parse_i32(v[0]), offset: parse_f64(v[1]), width: parse_f64(v[3]), direction: parse_i32(v[4]) });
                }
            }
        }
        SplineFile { lanes }
    }

    /// The street lanes' extent across: (the middle's offset, the width), None without one.
    pub fn street(&self) -> Option<(f64, f64)> {
        let streets = self.lanes.iter().filter(|l| l.kind == 0);
        let (lo, hi) = streets.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), l| (lo.min(l.offset - l.width / 2.0), hi.max(l.offset + l.width / 2.0)));
        (lo.is_finite() && hi > lo).then(|| ((lo + hi) / 2.0, hi - lo))
    }
}

/// A spline of a tile (`[spline]`, `[spline_h]`), in the tile's metres.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spline {
    /// The `.sli` file, as the tile names it.
    pub file: String,
    pub id: i64,
    pub prev: i64,
    pub next: i64,
    /// Its start: x, y (north) and height.
    pub at: P,
    pub height: f64,
    /// Degrees clockwise from north.
    pub heading: f64,
    pub length: f64,
    /// 0 straight; above 0 it turns right, below 0 left.
    pub radius: f64,
    pub mirror: bool,
}

impl Spline {
    /// The place `s` metres along it and `side` metres right of its line, in the tile's metres.
    pub fn point(&self, s: f64, side: f64) -> P {
        let h = self.heading.to_radians();
        if self.radius.abs() < 1e-6 {
            let (sin, cos) = h.sin_cos();
            return [self.at[0] + s * sin + side * cos, self.at[1] + s * cos - side * sin];
        }
        // (the arc's centre lies on the side it turns to: right of the start for a radius above 0)
        let r = self.radius;
        let centre = [self.at[0] + r * h.cos(), self.at[1] - r * h.sin()];
        let hs = h + s / r;
        // (a lane right of the line lies nearer the centre on a right turn)
        let reach = r - side;
        [centre[0] - reach * hs.cos(), centre[1] + reach * hs.sin()]
    }

    /// Its heading `s` metres along it (degrees clockwise from north).
    pub fn heading_at(&self, s: f64) -> f64 {
        if self.radius.abs() < 1e-6 {
            self.heading
        } else {
            self.heading + (s / self.radius).to_degrees()
        }
    }

    /// Its line `side` metres right of it, from its start to its end: a straight spline's two
    /// ends, an arc every few metres.
    pub fn points(&self, side: f64) -> Vec<P> {
        let len = self.length.max(0.0);
        let n = if self.radius.abs() < 1e-6 { 1 } else { ((len / STEP).ceil() as usize).clamp(2, 400) };
        (0..=n).map(|k| self.point(len * k as f64 / n as f64, side)).collect()
    }

    pub fn end(&self) -> P {
        self.point(self.length, 0.0)
    }
}

/// How far apart an arc's points are drawn, in metres.
const STEP: f64 = 4.0;

/// An object of a tile that a stop can be: its ID and place in the tile's metres.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Placed {
    pub id: i64,
    pub at: P,
}

/// A tile's splines and placed objects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tile {
    pub splines: Vec<Spline>,
    pub objects: Vec<Placed>,
}

/// One line as the block's values read it ("" past the end).
fn at<'a>(v: &[&'a str], k: usize) -> &'a str {
    v.get(k).map_or("", |s| s.trim())
}

impl Tile {
    /// A tile file's text, of a map of `version` (its `[version]`; the tile's own when it has one).
    pub fn parse(cfg: &CfgFile, version: i32) -> Tile {
        let version = cfg.value("version").map(parse_i32).filter(|v| *v > 0).unwrap_or(version);
        let mut t = Tile::default();
        let mut attachments: Vec<(i64, usize, f64, f64)> = Vec::new();
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            let k = k.to_ascii_lowercase();
            if k == "spline" || k == "spline_h" {
                let v = r.values(40);
                // (`0`, file, ID, then the neighbours: two lines from version 11 on, one before)
                let (prev, next, p) = if version >= 11 { (parse_i64(at(&v, 3)), parse_i64(at(&v, 4)), 5) } else { (parse_i64(at(&v, 3)), -1, 4) };
                if v.len() < p + 6 {
                    continue;
                }
                t.splines.push(Spline {
                    file: at(&v, 1).to_string(),
                    id: parse_i64(at(&v, 2)),
                    prev,
                    next,
                    at: [parse_f64(at(&v, p)), parse_f64(at(&v, p + 2))],
                    height: parse_f64(at(&v, p + 1)),
                    heading: parse_f64(at(&v, p + 3)),
                    length: parse_f64(at(&v, p + 4)),
                    radius: parse_f64(at(&v, p + 5)),
                    mirror: v.iter().any(|l| l.trim().eq_ignore_ascii_case("mirror")),
                });
            } else if k == "object" {
                // (the detail level's `0` first from version 9 on)
                let v = r.values(6);
                let o = usize::from(version >= 9);
                if v.len() >= o + 5 {
                    t.objects.push(Placed { id: parse_i64(at(&v, o + 1)), at: [parse_f64(at(&v, o + 2)), parse_f64(at(&v, o + 3))] });
                }
            } else if k == "splineattachement" {
                let v = r.values(7);
                if v.len() == 7 {
                    let index = usize::try_from(parse_i32(at(&v, 3))).unwrap_or(usize::MAX);
                    attachments.push((parse_i64(at(&v, 2)), index, parse_f64(at(&v, 4)), parse_f64(at(&v, 6))));
                }
            }
        }
        // (an object along a spline: at its distance on the spline, where the spline has it)
        for (id, index, side, along) in attachments {
            if let Some(s) = t.splines.get(index) {
                t.objects.push(Placed { id, at: s.point(along.clamp(0.0, s.length.max(0.0)), side) });
            }
        }
        t
    }
}

/// A road as the maps draw it: its line in the map's metres, its width, a main road or not.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Road {
    pub points: Vec<P>,
    pub width: f64,
    pub main: bool,
}

/// A map's tiles with their splines, read once, and the spline files they name.
#[derive(Debug, Clone, Default)]
pub struct MapRoads {
    /// Each tile's corner in the map's metres, beside `tiles` (a tile not read: empty).
    pub origins: Vec<P>,
    pub tiles: Vec<Tile>,
    /// The spline files by the name the tiles give them (in lower case).
    pub files: HashMap<String, SplineFile>,
    /// The map's frame: its tiles' lowest and highest corners.
    pub lo: P,
    pub hi: P,
}

/// How the splines' ends meet their neighbours' (`prev`, `next`): how many neighbours were found,
/// and how many begin or end within a metre of the spline's end. A check of the reading on a real
/// map: OMSI's maps join their splines end to end.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Joins {
    pub checked: usize,
    pub met: usize,
}

impl MapRoads {
    /// The roads of the map `g` (its `global.cfg` read), its tiles in the folder `map_dir`, the
    /// spline files found through `vfs`.
    pub fn read(vfs: &Vfs, g: &GlobalCfg, map_dir: &Path) -> MapRoads {
        let edge = g.tile_edge();
        let version = g.version.unwrap_or(14);
        let mut m = MapRoads { lo: [f64::INFINITY; 2], hi: [f64::NEG_INFINITY; 2], ..MapRoads::default() };
        for (k, (x, y)) in g.tiles.iter().enumerate() {
            let origin = [*x as f64 * edge, *y as f64 * edge];
            m.lo = [m.lo[0].min(origin[0]), m.lo[1].min(origin[1])];
            m.hi = [m.hi[0].max(origin[0] + edge), m.hi[1].max(origin[1] + edge)];
            let file = g.tile_files.get(k).cloned().unwrap_or_else(|| format!("tile_{x}_{y}.map"));
            let tile = crate::vfs::find_child(map_dir, &file).and_then(|p| CfgFile::read(p).ok()).map(|c| Tile::parse(&c, version)).unwrap_or_default();
            m.origins.push(origin);
            m.tiles.push(tile);
        }
        if g.tiles.is_empty() {
            (m.lo, m.hi) = ([0.0; 2], [0.0; 2]);
        }
        for s in m.tiles.iter().flat_map(|t| t.splines.iter()) {
            m.files.entry(s.file.to_lowercase()).or_insert_with(|| vfs.resolve(&s.file).and_then(|p| CfgFile::read(p).ok()).map(|c| SplineFile::parse(&c)).unwrap_or_default());
        }
        m
    }

    fn file_of(&self, s: &Spline) -> Option<&SplineFile> {
        self.files.get(&s.file.to_lowercase())
    }

    fn world(&self, tile: usize, p: P) -> P {
        let o = self.origins.get(tile).copied().unwrap_or([0.0; 2]);
        [o[0] + p[0], o[1] + p[1]]
    }

    /// Every spline with a street lane as a road (its street lanes' middle line and width); a road
    /// of three street lanes or more, or 10 m wide, is a main road.
    pub fn roads(&self) -> Vec<Road> {
        let mut out = Vec::new();
        for (k, t) in self.tiles.iter().enumerate() {
            for s in &t.splines {
                let Some(file) = self.file_of(s) else { continue };
                let Some((mid, width)) = file.street() else { continue };
                let side = if s.mirror { -mid } else { mid };
                let lanes = file.lanes.iter().filter(|l| l.kind == 0).count();
                out.push(Road { points: s.points(side).into_iter().map(|p| self.world(k, p)).collect(), width, main: lanes >= 3 || width >= 10.0 });
            }
        }
        out
    }

    /// The spline `id` of tile `tile`.
    pub fn spline(&self, tile: usize, id: i64) -> Option<&Spline> {
        self.tiles.get(tile)?.splines.iter().find(|s| s.id == id)
    }

    /// One lane of a spline in the map's metres, in the way it runs (a `mirror`ed spline's on the
    /// other side, the other way), and whether it runs both ways.
    fn lane_points(&self, tile: usize, s: &Spline, lane: usize) -> Option<(Vec<P>, bool)> {
        let l = *self.file_of(s)?.lanes.get(lane)?;
        let side = if s.mirror { -l.offset } else { l.offset };
        let mut pts: Vec<P> = s.points(side).into_iter().map(|p| self.world(tile, p)).collect();
        let against = (l.direction == 1) != s.mirror && l.direction != 2;
        if against {
            pts.reverse();
        }
        Some((pts, l.direction == 2))
    }

    /// The way a trip's track (`.ttr`) drives, in the map's metres: each lane on a spline in its
    /// own direction (a lane both ways turned to go on from where the route is), what lies on an
    /// object bridged by the line between. Empty when none of its lanes is on a spline found.
    pub fn route(&self, track: &Track) -> Vec<P> {
        let mut out: Vec<P> = Vec::new();
        for e in &track.entries {
            let Some(s) = self.spline(e.tile, e.spline) else { continue };
            let Some((mut pts, both)) = self.lane_points(e.tile, s, e.path.max(0) as usize) else { continue };
            if let (true, Some(last)) = (both, out.last()) {
                if dist(*last, pts[pts.len() - 1]) < dist(*last, pts[0]) {
                    pts.reverse();
                }
            }
            if out.last().is_some_and(|l| dist(*l, pts[0]) < 0.05) {
                pts.remove(0);
            }
            out.extend(pts);
        }
        out
    }

    /// The place of the object `id` in tile `tile` (a stop), in the map's metres.
    pub fn object(&self, tile: usize, id: i64) -> Option<P> {
        let t = self.tiles.get(tile)?;
        t.objects.iter().find(|o| o.id == id).map(|o| self.world(tile, o.at))
    }

    /// How the splines' ends meet the neighbours they name ([`Joins`]).
    pub fn joins(&self) -> Joins {
        let mut j = Joins::default();
        for t in &self.tiles {
            for s in &t.splines {
                let Some(n) = (s.next >= 0).then(|| t.splines.iter().find(|x| x.id == s.next)).flatten() else { continue };
                j.checked += 1;
                let end = s.end();
                if dist(end, n.at) < 1.0 || dist(end, n.end()) < 1.0 {
                    j.met += 1;
                }
            }
        }
        j
    }
}

fn dist(a: P, b: P) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timetable::TrackEntry;

    fn close(a: P, b: P) -> bool {
        dist(a, b) < 1e-6
    }

    fn spline(heading: f64, length: f64, radius: f64) -> Spline {
        Spline { id: 1, at: [100.0, 100.0], heading, length, radius, ..Spline::default() }
    }

    #[test]
    fn a_straight_spline_runs_its_heading_and_a_lane_lies_to_its_right() {
        let north = spline(0.0, 50.0, 0.0);
        assert!(close(north.end(), [100.0, 150.0]));
        assert!(close(north.point(0.0, 3.5), [103.5, 100.0]), "right of a road north is east");
        let east = spline(90.0, 20.0, 0.0);
        assert!(close(east.end(), [120.0, 100.0]));
        assert!(close(east.point(10.0, 2.0), [110.0, 98.0]), "right of a road east is south");
    }

    #[test]
    fn an_arc_turns_right_for_a_radius_above_0_and_left_below() {
        let quarter = std::f64::consts::FRAC_PI_2;
        // (north, a quarter circle of radius 10 to the right: it ends 10 east, 10 north, heading east)
        let right = spline(0.0, 10.0 * quarter, 10.0);
        assert!(close(right.end(), [110.0, 110.0]), "{:?}", right.end());
        assert!((right.heading_at(right.length) - 90.0).abs() < 1e-9);
        let left = spline(0.0, 10.0 * quarter, -10.0);
        assert!(close(left.end(), [90.0, 110.0]), "{:?}", left.end());
        assert!((left.heading_at(left.length) + 90.0).abs() < 1e-9);
        // (a lane 2 m right of a right turn runs on the inside: radius 8)
        assert!(close(right.point(right.length, 2.0), [110.0, 108.0]));
        let pts = right.points(0.0);
        assert!(pts.len() > 3 && close(pts[0], [100.0, 100.0]) && close(*pts.last().unwrap(), [110.0, 110.0]));
        assert!(pts.iter().all(|p| (dist(*p, [110.0, 100.0]) - 10.0).abs() < 1e-9), "every point on the circle");
    }

    fn tile_text() -> String {
        // (two splines end to end: 40 m north, then a right quarter turn of radius 20; a stop on
        // the first, and one placed on its own)
        let quarter = 20.0 * std::f64::consts::FRAC_PI_2;
        format!(
            "[version]\r\n14\r\n\r\n[spline]\r\n0\r\nSplines\\road.sli\r\n11\r\n-1\r\n12\r\n10\r\n0\r\n20\r\n0\r\n40\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n\r\n\
             [spline_h]\r\n0\r\nSplines\\road.sli\r\n12\r\n11\r\n-1\r\n10\r\n0\r\n60\r\n0\r\n{quarter}\r\n20\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n\r\n\
             [spline]\r\n0\r\nSplines\\road.sli\r\n13\r\n-1\r\n-1\r\n200\r\n0\r\n200\r\n90\r\n30\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\n0\r\nmirror\r\n\r\n\
             [object]\r\n0\r\nSceneryobjects\\stop.sco\r\n501\r\n5\r\n30\r\n0\r\n0\r\n0\r\n0\r\n\r\n\
             [splineAttachement]\r\n0\r\nSceneryobjects\\stop.sco\r\n502\r\n0\r\n-4\r\n0\r\n25\r\n0\r\n0\r\n0\r\n\r\n"
        )
    }

    const SLI: &str = "[path]\r\n0\r\n-1.75\r\n0\r\n3.5\r\n1\r\n\r\n[path]\r\n0\r\n1.75\r\n0\r\n3.5\r\n0\r\n\r\n[path]\r\n1\r\n4.5\r\n0.2\r\n2\r\n2\r\n";

    fn map() -> MapRoads {
        let tile = Tile::parse(&CfgFile::from_str("tile_1_0.map", &tile_text()), 14);
        let mut files = HashMap::new();
        files.insert("splines\\road.sli".to_string(), SplineFile::parse(&CfgFile::from_str("road.sli", SLI)));
        MapRoads { origins: vec![[0.0, 0.0], [300.0, 0.0]], tiles: vec![Tile::default(), tile], files, lo: [0.0; 2], hi: [600.0, 300.0] }
    }

    #[test]
    fn a_tile_s_splines_objects_and_lanes_are_read() {
        let m = map();
        let t = &m.tiles[1];
        assert_eq!(t.splines.len(), 3);
        let a = &t.splines[0];
        assert_eq!((a.id, a.prev, a.next, a.at, a.length, a.mirror), (11, -1, 12, [10.0, 20.0], 40.0, false));
        assert!(t.splines[2].mirror);
        let f = &m.files["splines\\road.sli"];
        assert_eq!(f.lanes.len(), 3);
        assert_eq!(f.street(), Some((0.0, 7.0)), "two lanes of 3.5 m about the line");
        assert_eq!(m.object(1, 501), Some([305.0, 30.0]));
        // (the attachment 25 m along the first spline, 4 m left of it)
        assert_eq!(m.object(1, 502), Some([306.0, 45.0]));
        assert_eq!(m.object(1, 999), None);
        assert_eq!(m.joins(), Joins { checked: 1, met: 1 }, "the first spline's end is the second's start");
    }

    #[test]
    fn the_roads_are_in_the_map_s_metres() {
        let roads = map().roads();
        assert_eq!(roads.len(), 3);
        assert!(close(roads[0].points[0], [310.0, 20.0]) && close(*roads[0].points.last().unwrap(), [310.0, 60.0]));
        assert_eq!((roads[0].width, roads[0].main), (7.0, false));
        // (the turn ends 20 east and 20 north of where it began, heading east)
        assert!(close(*roads[1].points.last().unwrap(), [330.0, 80.0]), "{:?}", roads[1].points.last());
    }

    #[test]
    fn a_trip_drives_its_lanes_in_their_direction() {
        let m = map();
        let entry = |spline: i64, path: i32| TrackEntry { spline, path, tile: 1, ..TrackEntry::default() };
        // the right-hand lane (1) north, on into the turn
        let out = m.route(&Track { entries: vec![entry(11, 1), entry(12, 1)], ..Track::default() });
        assert!(close(out[0], [311.75, 20.0]) && close(*out.last().unwrap(), [330.0, 78.25]), "{:?} … {:?}", out[0], out.last());
        // (the straight's end is the turn's start, once: no jump where the splines meet)
        assert!(close(out[1], [311.75, 60.0]) && dist(out[1], out[2]) < STEP && dist(out[1], out[2]) > 0.1, "{out:?}");
        // the left-hand lane (0) runs against the spline: back out of the turn, then south
        let back = m.route(&Track { entries: vec![entry(12, 0), entry(11, 0)], ..Track::default() });
        assert!(close(back[0], [330.0, 81.75]) && close(*back.last().unwrap(), [308.25, 20.0]), "{:?} … {:?}", back[0], back.last());
        // a mirrored spline's lane 1 lies left of its line and runs against it
        let mirrored = m.route(&Track { entries: vec![entry(13, 1)], ..Track::default() });
        assert!(close(mirrored[0], [530.0, 201.75]) && close(mirrored[1], [500.0, 201.75]), "{mirrored:?}");
        // (a lane on an object, a spline not there: left out)
        assert!(m.route(&Track { entries: vec![entry(77, 0)], ..Track::default() }).is_empty());
    }

    #[test]
    fn a_spline_before_version_11_has_one_neighbour_line() {
        let old = "[spline]\r\n0\r\nSplines\\road.sli\r\n5\r\n4\r\n1\r\n0\r\n2\r\n90\r\n10\r\n0\r\n0\r\n0\r\n";
        let t = Tile::parse(&CfgFile::from_str("old.map", old), 8);
        let s = &t.splines[0];
        assert_eq!((s.id, s.prev, s.next, s.at, s.heading, s.length), (5, 4, -1, [1.0, 2.0], 90.0, 10.0));
        // (and an object of before version 9 has no detail level first)
        let o = Tile::parse(&CfgFile::from_str("old.map", "[object]\r\nSceneryobjects\\a.sco\r\n7\r\n3\r\n4\r\n0\r\n"), 8);
        assert_eq!(o.objects, [Placed { id: 7, at: [3.0, 4.0] }]);
    }
}
