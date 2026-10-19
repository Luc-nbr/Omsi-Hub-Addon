//! The map as the line editor reads it, from its tiles alone (the old app's game program's
//! `navigation_map_of` and the navigator's `city_roads`, taken from openOMSI's `omsi-scene` and
//! `omsi-app` in Luc's fork - MIT, THIRD_PARTY_NOTICES.md): every spline's and object's traffic
//! paths as lanes, linked into the network the line editor routes over (`lineroute`), the
//! roads the page draws, and the map's bus stops.

use glam::{DVec2, DVec3, Vec3};
use hashbrown::HashMap;
use omsi_geometry::SplineCurve;
use omsi_map::{tile_size, Terrain};
use omsi_scenery::{SceneryObject, Spline};
use omsi_sim::traffic::{Lane, LaneBuilder, LaneKey, LaneKind, Network};
use omsi_world::map_index::tile_companion;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The whole map for the navigator (see [`World::navigation_map`]).
pub struct NavigationMap {
    pub lanes: Vec<Lane>,
    /// Asphalt footprints used to corroborate editor-only driving paths. They are evidence
    /// for roads, not streets to draw: a paved yard or median has no road centre line.
    pub road_surfaces: Vec<(Vec<DVec3>, f32)>,
    /// Every placed object's position by id (bus stops beyond the loaded tiles).
    pub places: HashMap<i64, DVec3>,
    /// Street name signs: where, the object's heading and the name on it.
    // (the navigator's street names: read as the game reads them, not drawn by the pages)
    #[allow(dead_code)]
    pub signs: Vec<(DVec3, f64, String)>,
    /// Every bus stop of the map (an object whose `.sco` says `[busstop]`): placed, hung on
    /// another object (`[attachObj]`, at its parent) or put on a spline.
    pub stops: Vec<MapStop>,
}

/// A bus stop of the map as the line editor offers it.
#[derive(Clone, Debug, PartialEq)]
pub struct MapStop {
    /// The tile it is placed in, and its id there (what a timetable's station names).
    pub tile: (i32, i32),
    pub id: i64,
    pub at: DVec3,
    /// The object's heading (degrees).
    pub heading: f64,
    /// Its first string: the stop's name as the map gives it (empty when it has none).
    pub name: String,
}

/// The same read with no `World` holding the caches, so that anything else that wants to
/// know what a map has (the launcher's map picture) reads it the way the navigator does.
/// `root` is the installation the map belongs to: what its tiles name - a `.sli`, a `.sco`
/// and the model beside it - is resolved against it, so a mod's own copy wins over the
/// original's and a map that lacks one borrows the other installation's (`omsi_cfg::
/// resolve_path`).
pub fn navigation_map_of(root: &Path, tiles: &[(i32, i32, PathBuf)], chrono_dirs: &[PathBuf]) -> NavigationMap {
    use rayon::prelude::*;
    let t0 = std::time::Instant::now();
    // (each file read once: a tile that wants one being read waits for it, rather than
    // reading it too - see `OnceByKey`)
    type Once<T> = Arc<std::sync::OnceLock<Option<Arc<T>>>>;
    let scos: Mutex<HashMap<String, Once<SceneryObject>>> = Mutex::new(HashMap::new());
    let sco_of = |file: &str| -> Option<Arc<SceneryObject>> {
        let key = file.trim().to_ascii_lowercase().replace('\\', "/");
        let cell = scos.lock().entry(key).or_default().clone();
        cell.get_or_init(|| SceneryObject::load(&omsi_cfg::resolve_path(root, file)).ok().map(Arc::new)).clone()
    };
    let slis: Mutex<HashMap<String, Once<Spline>>> = Mutex::new(HashMap::new());
    let sli_of = |file: &str| -> Option<Arc<Spline>> {
        // (the navigator loads `.sli` through `World::spline_type` with the same two calls)
        let key = file.trim().to_ascii_lowercase().replace('\\', "/");
        let cell = slis.lock().entry(key).or_default().clone();
        cell.get_or_init(|| {
            Spline::load(&omsi_cfg::resolve_path(root, file)).ok().map(|def| {
                omsi_geometry::register_half_cant_width(file, &def);
                Arc::new(def)
            })
        })
        .clone()
    };
    #[allow(clippy::type_complexity)]
    let parts: Vec<(Vec<Lane>, Vec<(i64, DVec3)>, Vec<(DVec3, f64, String)>, Vec<(Vec<DVec3>, f32)>, Vec<MapStop>)> = tiles
        .par_iter()
        .map(|(tx, ty, path)| {
            let (tx, ty) = (*tx, *ty);
            let mut lanes = Vec::new();
            let mut positions = Vec::new();
            let mut signs = Vec::new();
            let mut roads = Vec::new();
            let mut stops = Vec::new();
            let Some(tile) = omsi_world::map_index::read_tile(path, chrono_dirs) else {
                return (lanes, positions, signs, roads, stops);
            };
            let origin2 = DVec2::new(tx as f64 * tile_size(), ty as f64 * tile_size());
            let terrain = Terrain::load(&tile_companion(&path, ".terrain")).unwrap_or_else(|_| Terrain::flat());
            for sp in tile.splines.iter().filter(|s| !s.deleted && !s.file.trim().is_empty()) {
                let Some(def) = sli_of(&sp.file) else { continue };
                if !def.paths.iter().any(|p| p.kind == 0) {
                    // Short surfaces also corroborate editor-only traffic paths at
                    // junctions; the navigator filters decorative patches for display.
                    if sp.length >= 2.0 {
                        let curve = SplineCurve::from_map(sp, origin2).with_sli(&def);
                        let n = ((curve.length / 4.0).ceil() as usize).clamp(1, 400);
                        let side = if sp.mirror { -1.0 } else { 1.0 };
                        for (lo, hi, z) in road_sections(&sp.file, &def) {
                            let offset = side * ((lo + hi) * 0.5) as f64;
                            let pts: Vec<DVec3> = (0..=n).map(|k| curve.offset_point(curve.length * k as f64 / n as f64, offset, z as f64)).collect();
                            roads.push((pts, hi - lo));
                        }
                    }
                }
                if def.paths.is_empty() {
                    continue;
                }
                let curve = SplineCurve::from_map(sp, origin2);
                let mut new_lanes = spline_lanes(&def, sp, &curve, (tx, ty));
                for l in new_lanes.iter_mut() {
                    l.invisible = def.only_editor;
                }
                lanes.extend(new_lanes);
            }
            for o in &tile.objects {
                if o.file.trim().is_empty() {
                    continue;
                }
                let (x, y) = (origin2.x + o.pos[0], origin2.y + o.pos[1]);
                let ground = || {
                    let (lx, ly) = ((x - origin2.x).clamp(0.0, tile_size()) as f32, (y - origin2.y).clamp(0.0, tile_size()) as f32);
                    terrain.sample(lx, ly) as f64
                };
                // street name signs carry the street's name as their text
                if is_street_sign(&o.file) {
                    if let Some(name) = o.extra.first().map(|t| t.trim()).filter(|t| t.chars().filter(|c| c.is_alphabetic()).count() >= 3) {
                        signs.push((DVec3::new(x, y, o.pos[2] + ground()), o.rot[0], name.to_string()));
                    }
                }
                let Some(sco) = sco_of(&o.file) else {
                    positions.push((o.id, DVec3::new(x, y, o.pos[2] + ground())));
                    continue;
                };
                let absolute = sco.absolute_height();
                let pos = DVec3::new(x, y, if absolute { o.pos[2] } else { o.pos[2] + ground() });
                positions.push((o.id, pos));
                if sco.is_bus_stop {
                    stops.push(MapStop { tile: (tx, ty), id: o.id, at: pos, heading: o.rot[0], name: o.extra.first().map(|s| s.trim().to_string()).unwrap_or_default() });
                }
                if !sco.paths.is_empty() {
                    lanes.extend(object_lanes(&sco, pos, [o.rot[0], 0.0, 0.0], None, (tx, ty), o.id, &o.rules));
                }
            }
            // the stops hung on another object (Ahlheim's on their shelters): at their parent,
            // a few metres off at most; and the stops put on a spline, at the row's first
            for o in &tile.attach_objects {
                if o.file.trim().is_empty() || !sco_of(&o.file).is_some_and(|s| s.is_bus_stop) {
                    continue;
                }
                let Some(parent) = o.parent_id.and_then(|p| positions.iter().find(|q| q.0 == p).map(|q| q.1)) else { continue };
                let heading = tile.objects.iter().find(|q| Some(q.id) == o.parent_id).map(|q| q.rot[0]).unwrap_or(0.0) + o.rot[0];
                if !positions.iter().any(|q| q.0 == o.id) {
                    positions.push((o.id, parent));
                }
                stops.push(MapStop { tile: (tx, ty), id: o.id, at: parent, heading, name: o.extra.first().map(|s| s.trim().to_string()).unwrap_or_default() });
            }
            for a in tile.spline_attachments.iter().filter(|a| a.repeater.is_none()) {
                if a.file.trim().is_empty() || !sco_of(&a.file).is_some_and(|s| s.is_bus_stop) {
                    continue;
                }
                let Some((_, first)) = omsi_world::map_index::tile_row_objects(a, &tile.splines, origin2, None).into_iter().next() else { continue };
                if !positions.iter().any(|q| q.0 == a.id) {
                    positions.push((a.id, first.pose.pos));
                }
                stops.push(MapStop { tile: (tx, ty), id: a.id, at: first.pose.pos, heading: first.pose.heading(), name: a.strings.first().map(|s| s.trim().to_string()).unwrap_or_default() });
            }
            (lanes, positions, signs, roads, stops)
        })
        .collect();
    let mut lanes = Vec::new();
    let mut positions = HashMap::new();
    let mut signs = Vec::new();
    let mut roads = Vec::new();
    let mut stops = Vec::new();
    for (l, p, s, r, b) in parts {
        lanes.extend(l);
        positions.extend(p);
        signs.extend(s);
        roads.extend(r);
        stops.extend(b);
    }
    // (an id objects of several tiles share - a map joined from others - is the bus stop's
    // where one of them is a stop: these places are the stops' beyond the loaded tiles)
    for s in &stops {
        positions.insert(s.id, s.at);
    }
    log::info!("navigation map: {} roads without a path for cars", roads.len());
    log::info!(
        "navigation map: {} tiles, {} lanes, {} objects placed, {} street name signs, {} object types, {} spline types, {:.1} s",
        tiles.len(),
        lanes.len(),
        positions.len(),
        signs.len(),
        scos.lock().len(),
        slis.lock().len(),
        t0.elapsed().as_secs_f64()
    );
    NavigationMap { lanes, road_surfaces: roads, places: positions, signs, stops }
}

/// Whether an asset name describes a road surface. Match whole filename tokens so objects
/// such as `StreetLight.sli` do not become roads just because their name contains "street".
pub(crate) fn road_surface_name(file: &str) -> bool {
    let base = file.replace('\\', "/").rsplit('/').next().unwrap_or("").to_ascii_lowercase();
    let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(&base);
    let tokens: Vec<&str> = stem.split(|c: char| !c.is_ascii_alphanumeric() && c != 'ß').filter(|t| !t.is_empty()).collect();
    let excludes = [
        "light",
        "lamp",
        "sign",
        "schild",
        "rail",
        "track",
        "gleis",
        "tram",
        "strab",
        "wire",
        "mast",
        "wall",
        "fence",
        "leitplanke",
        "gehweg",
        "side",
        "bord",
        "pavement",
        "fahrrad",
        "radweg",
        "cycle",
        "parking",
        "parkplatz",
        "gruen",
        "gras",
    ];
    if tokens.iter().any(|t| excludes.iter().any(|x| t.contains(x))) {
        return false;
    }
    stem.starts_with("str_")
        || tokens.iter().any(|t| ["str", "strasse", "straße", "road", "roads", "street", "streets", "fahrbahn", "pflaster", "kopfstein", "cobble"].contains(t) || t.starts_with("asph"))
}

/// The horizontal road surfaces actually drawn by a pathless spline: lateral bounds and
/// height. A texture merely listed in the file is not evidence of a road, and the origin
/// need not be in the middle of the surface. Keep medians and pavements out of its width.
pub(crate) fn road_sections(file: &str, def: &omsi_scenery::sli::Spline) -> Vec<(f32, f32, f32)> {
    let name = file.to_ascii_lowercase();
    if def.only_editor || ["gehweg", "radweg", "fahrrad", "tram", "strab", "gleis", "rail", "parking"].iter().any(|s| name.contains(s)) || def.paths.iter().any(|p| p.kind == 2) {
        return Vec::new();
    }
    let mut sections = Vec::new();
    for profile in &def.profiles {
        let road = def.textures.get(profile.texture).map(|t| road_surface_name(&t.file)).unwrap_or_else(|| def.textures.is_empty() && road_surface_name(file));
        if !road {
            continue;
        }
        for pair in profile.points.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            let (lo, hi) = (a.x.min(b.x), a.x.max(b.x));
            if lo.is_finite() && hi.is_finite() && a.z.is_finite() && b.z.is_finite() && hi - lo > 0.1 && (a.z - b.z).abs() <= (hi - lo) * 0.15 {
                sections.push((lo, hi, (a.z + b.z) * 0.5));
            }
        }
    }
    sections.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f32, f32, f32)> = Vec::new();
    for (lo, hi, z) in sections {
        if let Some(last) = merged.last_mut().filter(|s| lo <= s.1 + 0.2 && (z - s.2).abs() < 0.2) {
            last.1 = last.1.max(hi);
        } else {
            merged.push((lo, hi, z));
        }
    }
    merged.retain(|(lo, hi, _)| hi - lo >= 4.0);
    merged
}

/// A street name sign object (the stock Verkehrszeichen_MC `StreetSign_*`, and the
/// German `Strassenschild`/`StrSchild` names add-on maps use).
pub(crate) fn is_street_sign(file: &str) -> bool {
    let f = file.to_ascii_lowercase().replace(['\\', '_', ' ', '-'], "");
    let name = f.rsplit('/').next().unwrap_or(&f);
    ["streetsign", "streetname", "strschild", "strassenschild", "straßenschild", "strassenname", "roadsignname", "roadname"].iter().any(|k| name.contains(k))
}

/// A path's `[rule] trafficdensity`s: how much random traffic of any group it carries,
/// and the last value per group (the rule's fourth line: the group's place in the map's
/// `unsched_vehgroups.txt`). Without a rule for the first group the path has its medium
/// density (1); the lane carries traffic as long as any group drives on it - on
/// Berlin-Spandau 462 Falkensee paths set only the GDR cars' density.
pub(crate) fn path_densities(rules: &[omsi_map::MapRule], path: usize) -> (f32, Vec<(u16, f32)>) {
    let mut per: Vec<(u16, f32)> = Vec::new();
    for r in rules.iter().filter(|r| r.path_index == path as i32 && r.kind.eq_ignore_ascii_case("trafficdensity") && !r.kill) {
        let g = r.extra.max(0.0) as u16;
        let v = (r.value as f32).max(0.0);
        match per.iter_mut().find(|(k, _)| *k == g) {
            Some(e) => e.1 = v,
            None => per.push((g, v)),
        }
    }
    let first = per.iter().find(|(g, _)| *g == 0).map(|e| e.1).unwrap_or(1.0);
    let density = per.iter().map(|e| e.1).fold(first, f32::max);
    (density, per)
}

/// Lanes of one map spline: every `[path]` of the spline type runs along the curve at its
/// lateral offset; `direction` 1 runs backwards, 2 both ways (two lanes).
///
/// A spline placed with `mirror` has its cross-section turned over: each path lies on the
/// other side of the centre line and runs the other way, as its carriageway does (a right
/// lane that ran forward is a left lane running backward - traffic still keeps right).
/// Ignoring the flag put every lane of Spandau's 40-odd mirrored road pieces 5 to 20 m
/// beside its road and the wrong way round; a timetable route through one had its bus turn
/// into the oncoming lanes and jump back where the next piece began.
pub(crate) fn spline_lanes(def: &Spline, s: &omsi_map::MapSpline, curve: &SplineCurve, tile: (i32, i32)) -> Vec<Lane> {
    let mut out = Vec::new();
    let side = if s.mirror { -1.0 } else { 1.0 };
    let curve = &curve.with_sli(def);
    for (pi, p) in def.paths.iter().enumerate() {
        let n = ((curve.length / 3.0).ceil() as usize).clamp(1, 300);
        let pts: Vec<DVec3> = (0..=n).map(|i| curve.offset_point(curve.length * i as f64 / n as f64, side * p.start[0] as f64, p.start[2] as f64)).collect();
        let kind = LaneKind::from_code(p.kind);
        let limit = s.rules.iter().filter(|r| r.path_index == pi as i32 && r.kind.eq_ignore_ascii_case("speedlimit") && r.value > 0.0).map(|r| r.value as f32).last();
        // The other rules of this path: how much traffic the mapper wants here at all.
        // Berlin-Spandau alone carries 8516 [rule] trafficdensity and 68 no_cars, and with
        // them ignored cars appeared in pedestrian streets, depot yards and back lanes the
        // original keeps empty.
        let rule_of = |name: &str| s.rules.iter().filter(|r| r.path_index == pi as i32 && r.kind.eq_ignore_ascii_case(name) && !r.kill).map(|r| r.value as f32).last();
        let (density, group_density) = path_densities(&s.rules, pi);
        let no_cars = s.rules.iter().any(|r| r.path_index == pi as i32 && r.kind.eq_ignore_ascii_case("no_cars") && !r.kill);
        // (`bus` and `trucks` are switches that open the path to those AI vehicles, see
        // `Lane::allows`; `bus` does not close it to cars)
        let rule_bus = rule_of("bus").is_some();
        let rule_trucks = rule_of("trucks").is_some();
        let no_overtaking = rule_of("overtaking_prohib").is_some();
        let priority = rule_of("priority");
        let mut push = |pts: Vec<DVec3>, reversed: bool| {
            let mut l = LaneBuilder::polyline(pts, kind, p.width);
            if let Some(v) = priority {
                l.priority = v;
            }
            if let Some(v) = limit {
                l.speed_limit_kmh = v;
            }
            l.density = density;
            l.group_density = group_density.clone();
            l.no_cars = no_cars;
            l.rule_bus = rule_bus;
            l.rule_trucks = rule_trucks;
            l.no_overtaking = no_overtaking;
            l.source = 1;
            l.key = Some(LaneKey { tile, id: s.id, path: pi as u16 });
            l.reversed = reversed;
            l.offset = side as f32 * p.start[0];
            l.name = def.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            out.push(l);
        };
        // (a mirrored spline's forward path runs backwards and the other way round)
        match (p.direction, s.mirror) {
            (2, _) => {
                push(pts.clone(), false);
                push(pts.into_iter().rev().collect(), true);
            }
            (1, false) | (0, true) => push(pts.iter().rev().copied().collect(), true),
            _ => push(pts, false),
        }
    }
    out
}

/// Lanes of one placed scenery object: `[path]` arcs in the object frame (x right,
/// y forward, z up; heading clockwise, radius > 0 right turn) rotated by the object heading.
pub(crate) fn object_lanes(sco: &SceneryObject, pos: DVec3, rot: [f64; 3], controller: Option<usize>, tile: (i32, i32), id: i64, rules: &[omsi_map::MapRule]) -> Vec<Lane> {
    let mut out = Vec::new();
    let heading = rot[0];
    let h = heading.to_radians();
    let (sh, ch) = (h.sin(), h.cos());
    // local (x east, y north) → world for an object turned clockwise by `heading`
    let to_world = |x: f64, y: f64| DVec2::new(x * ch + y * sh, -x * sh + y * ch);
    for (pi, p) in sco.paths.iter().enumerate() {
        let v = &p.params;
        if v.len() < 11 {
            continue;
        }
        let start_local = to_world(v[0] as f64, v[1] as f64);
        let start = DVec3::new(pos.x + start_local.x, pos.y + start_local.y, pos.z + v[2] as f64);
        let (path_heading, radius, length) = (v[3] as f64 + heading, v[4] as f64, v[5] as f64);
        if length <= 0.01 {
            continue;
        }
        let (grad_start, grad_end) = (v[6] as f64, v[7] as f64);
        let kind = LaneKind::from_code(p.kind);
        let turn = match v.get(11).map(|t| *t as i32) {
            Some(2) => 1,
            Some(3) => 2,
            _ => 0,
        };
        // The `[rule]`s the map put on this object's path. Most of a map's rules sit on the
        // junctions, not on the splines - Berlin-Spandau has 5402 trafficdensity, 693
        // speedlimit, 576 trucks and 64 no_cars on objects against 3114/810/284/4 on splines
        // - so ignoring them left cars turning into every yard and pedestrian street.
        let rule_of = |name: &str| rules.iter().filter(|r| r.path_index == pi as i32 && r.kind.eq_ignore_ascii_case(name) && !r.kill).map(|r| r.value as f32).last();
        let limit = rules.iter().filter(|r| r.path_index == pi as i32 && r.kind.eq_ignore_ascii_case("speedlimit") && r.value > 0.0 && !r.kill).map(|r| r.value as f32).last();
        let (density, group_density) = path_densities(rules, pi);
        let no_cars = rules.iter().any(|r| r.path_index == pi as i32 && r.kind.eq_ignore_ascii_case("no_cars") && !r.kill);
        let rule_bus = rule_of("bus").is_some();
        let rule_trucks = rule_of("trucks").is_some();
        let no_overtaking = rule_of("overtaking_prohib").is_some();
        // who goes first where this path meets another (`Network::must_yield`)
        let priority = rule_of("priority");
        let mut push = |reverse: bool| {
            let mut l = LaneBuilder::arc_with_gradients(start, path_heading, length, radius, grad_start, grad_end, kind, p.width);
            if reverse {
                let pts: Vec<DVec3> = l.points.iter().rev().copied().collect();
                l = LaneBuilder::polyline(pts, kind, p.width);
            }
            if let Some(v) = limit {
                l.speed_limit_kmh = v;
            }
            l.density = density;
            l.group_density = group_density.clone();
            l.no_cars = no_cars;
            l.rule_bus = rule_bus;
            l.rule_trucks = rule_trucks;
            l.no_overtaking = no_overtaking;
            l.turn = turn;
            if let Some(v) = priority {
                l.priority = v;
            }
            l.source = 2;
            l.key = Some(LaneKey { tile, id, path: pi as u16 });
            l.blocks = sco.path_blocks.get(pi).map(|b| b.iter().filter(|(n, _)| *n >= 0).map(|(n, _)| *n as u16).collect()).unwrap_or_default();
            l.crossing_problem = sco.path_crossing_problem.get(pi).copied().unwrap_or(false);
            l.reversed = reverse;
            l.traffic_light = controller.and_then(|c| sco.path_traffic_light.get(pi).copied().filter(|t| *t >= 0).map(|t| (c, t as usize)));
            out.push(l);
        };
        match p.direction {
            1 => push(true),
            2 => {
                push(false);
                push(true);
            }
            _ => push(false),
        }
    }
    out
}

/// Street lanes that stand for drawn roads, including editor-only paths corroborated by
/// road surfaces. Uncorroborated helpers must not make a phantom road on the GPS.
pub(crate) fn visible_road_lanes(net: &Network) -> Vec<(usize, &omsi_sim::traffic::Lane)> {
    let mut seen = hashbrown::HashSet::<(LaneKey, u32)>::new();
    net.lanes
        .iter()
        .enumerate()
        .filter(|(_, l)| l.kind == LaneKind::Street && !l.invisible && l.points.len() >= 2)
        .filter(|(_, l)| {
            if let Some(key) = l.key {
                // A two-way [path] has the same geometry in both directions. Keep one
                // casing and surface; retain separate [path]s, which may be real lanes.
                return seen.insert((key, l.source));
            }
            true
        })
        .collect()
}

/// The roads a city map draws, from a whole map's lanes as they were read off the tiles:
/// every path linked to its neighbours, the editor-only ones confirmed by the map's own
/// asphalt, then grouped into carriageways with their real widths. The navigator's city map
/// and the launcher's map picture draw the same roads this way, so the two cannot drift
/// apart; the second value is the linked network the caller may walk (a trip's own lanes).
pub(crate) fn city_roads(lanes: Vec<omsi_sim::traffic::Lane>, surfaces: &[(Vec<DVec3>, f32)]) -> (Vec<MapRoad>, Network) {
    let mut net = Network { lanes, ..Default::default() };
    net.link(1.5);
    confirm_road_surfaces(&mut net, surfaces);
    let roads = road_geometry(&net);
    (roads, net)
}

/// A road of the city map: the centre line of a carriageway, as wide as the map makes it.
pub struct MapRoad {
    pub points: Vec<DVec3>,
    pub width: f32,
    /// A road cars drive far along (the map's own speed limits say so): drawn brighter.
    pub main: bool,
}

/// Some maps separate the asphalt mesh from their editor-only traffic splines. Use the
/// actual road footprint to distinguish those paths from invisible scenery helpers. This
/// changes only the navigator's copy of the network; driving still uses the original data.
pub(crate) fn confirm_road_surfaces(net: &mut Network, surfaces: &[(Vec<DVec3>, f32)]) {
    let mut segments = Vec::new();
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (pts, width) in surfaces {
        for ab in pts.windows(2) {
            let (a, b) = (ab[0], ab[1]);
            let margin = *width as f64 * 0.5 + 0.5;
            let lo = a.truncate().min(b.truncate()) - DVec2::splat(margin);
            let hi = a.truncate().max(b.truncate()) + DVec2::splat(margin);
            let (x0, y0) = Network::grid_cell(lo.extend(0.0));
            let (x1, y1) = Network::grid_cell(hi.extend(0.0));
            let i = segments.len();
            segments.push((a, b, margin));
            for x in x0..=x1 {
                for y in y0..=y1 {
                    grid.entry((x, y)).or_default().push(i);
                }
            }
        }
    }
    for lane in net.lanes.iter_mut().filter(|l| l.invisible && l.kind == LaneKind::Street) {
        let n = (lane.length() / 8.0).ceil().max(1.0) as usize;
        let mut covered = 0;
        for k in 0..n {
            let (p, _) = lane.at(lane.length() * (k as f32 + 0.5) / n as f32);
            let on_surface = grid
                .get(&Network::grid_cell(p))
                .map(|ids| {
                    ids.iter().any(|&i| {
                        let (a, b, margin) = segments[i];
                        let ab = (b - a).truncate();
                        let t = ((p - a).truncate().dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                        let q = a.lerp(b, t);
                        (p - q).truncate().length() <= margin && (p.z - q.z).abs() <= 2.0
                    })
                })
                .unwrap_or(false);
            if on_surface {
                covered += 1;
            }
        }
        if covered * 4 >= n * 3 {
            lane.invisible = false;
        }
    }
    // Crossings made of objects can have no asphalt spline. Keep helpers joining two
    // corroborated roads; an isolated invisible helper still does not become a street.
    let mut prev = vec![Vec::new(); net.lanes.len()];
    for (i, lane) in net.lanes.iter().enumerate().filter(|(_, l)| l.kind == LaneKind::Street) {
        for &j in &lane.next {
            if net.lanes.get(j).map(|l| l.kind == LaneKind::Street).unwrap_or(false) {
                prev[j].push(i);
            }
        }
    }
    let mut seen = vec![false; net.lanes.len()];
    let mut keep = Vec::new();
    for seed in 0..net.lanes.len() {
        if seen[seed] || !net.lanes[seed].invisible || net.lanes[seed].kind != LaneKind::Street {
            continue;
        }
        let mut queue = vec![seed];
        let mut component = Vec::new();
        let mut anchors = hashbrown::HashSet::new();
        while let Some(i) = queue.pop() {
            if seen[i] {
                continue;
            }
            seen[i] = true;
            component.push(i);
            for &j in net.lanes[i].next.iter().chain(prev[i].iter()) {
                let Some(l) = net.lanes.get(j).filter(|l| l.kind == LaneKind::Street) else { continue };
                if l.invisible {
                    if !seen[j] {
                        queue.push(j);
                    }
                } else {
                    anchors.insert((l.key, l.source, if l.key.is_none() { j } else { 0 }));
                }
            }
        }
        if anchors.len() >= 2 {
            keep.extend(component);
        }
    }
    for i in keep {
        net.lanes[i].invisible = false;
    }
}

/// Two paths of one spline whose middles lie at most this far apart are neighbouring lanes
/// of one carriageway, however narrow the map makes them: a path is as wide as the vehicle
/// on it (2.5 m paths on a 4 m pitch are common), so the strip between their edges is road
/// too - drawn apart, a two-lane street was two strips with a dark line between them. A
/// median or a tram track in the middle puts the paths further apart than any lane is wide.
pub(crate) const LANE_PITCH_MAX: f32 = 4.6;

/// Traffic paths are lane centres, not separate streets. Adjacent paths on one spline
/// describe one carriageway; combine their footprints without filling a median. Align
/// opposing directions before averaging so bends, mirrors and ramps retain their shape.
pub(crate) fn road_geometry(net: &Network) -> Vec<MapRoad> {
    let mut roads = Vec::new();
    let mut splines = std::collections::BTreeMap::<((i32, i32), i64), Vec<&omsi_sim::traffic::Lane>>::new();
    for (_, lane) in visible_road_lanes(net) {
        if let Some(key) = lane.key.filter(|_| lane.source == 1) {
            splines.entry((key.tile, key.id)).or_default().push(lane);
        } else {
            roads.push(MapRoad { points: lane.points.clone(), width: lane.width.max(2.6), main: lane.speed_limit_kmh >= 55.0 });
        }
    }
    for mut lanes in splines.into_values() {
        lanes.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        let mut start = 0;
        while start < lanes.len() {
            let a = lanes[start];
            let mut lo = a.offset - a.width.max(2.6) * 0.5;
            let mut hi = a.offset + a.width.max(2.6) * 0.5;
            let mut end = start + 1;
            while end < lanes.len() {
                let b = lanes[end];
                let touching = b.offset - b.width.max(2.6) * 0.5 <= hi + 0.65;
                let neighbour = b.offset - lanes[end - 1].offset <= LANE_PITCH_MAX;
                if b.points.len() != a.points.len() || !(touching || neighbour) {
                    break;
                }
                let mid = a.points.len() / 2;
                let za = a.points[if a.reversed { a.points.len() - 1 - mid } else { mid }].z;
                let zb = b.points[if b.reversed { b.points.len() - 1 - mid } else { mid }].z;
                if (za - zb).abs() > 1.5 {
                    break;
                }
                hi = hi.max(b.offset + b.width.max(2.6) * 0.5);
                end += 1;
            }
            let b = lanes[end - 1];
            // (lanes side by side are each a pitch wide, the outer ones too: their road reaches
            // half a pitch past their middles, not only their narrower paths' edges)
            if end - start >= 2 {
                let first = lanes[start + 1].offset - a.offset;
                let last = b.offset - lanes[end - 2].offset;
                if first <= LANE_PITCH_MAX {
                    lo = lo.min(a.offset - first * 0.5);
                }
                if last <= LANE_PITCH_MAX {
                    hi = hi.max(b.offset + last * 0.5);
                }
            }
            let span = b.offset - a.offset;
            let t = if span.abs() > 0.01 { (((lo + hi) * 0.5 - a.offset) / span) as f64 } else { 0.0 };
            let point = |l: &omsi_sim::traffic::Lane, i: usize| l.points[if l.reversed { l.points.len() - 1 - i } else { i }];
            let points = (0..a.points.len()).map(|i| point(a, i).lerp(point(b, i), t)).collect();
            roads.push(MapRoad { points, width: hi - lo, main: lanes[start..end].iter().any(|l| l.speed_limit_kmh >= 55.0) });
            start = end;
        }
    }
    // Bridge only explicit graph connections within the map's placement tolerance.
    for lane in net.lanes.iter().filter(|l| l.kind == LaneKind::Street && !l.invisible && l.points.len() >= 2) {
        for &j in &lane.next {
            let Some(next) = net.lanes.get(j).filter(|l| l.kind == LaneKind::Street && !l.invisible && l.points.len() >= 2) else { continue };
            let distance = lane.end().distance(next.start());
            if distance > 0.05 && distance <= 2.0 {
                roads.push(MapRoad { points: vec![lane.end(), next.start()], width: lane.width.min(next.width).max(2.6), main: lane.speed_limit_kmh >= 55.0 && next.speed_limit_kmh >= 55.0 });
            }
        }
    }
    roads
}

/// `pts` with the points dropped that lie within `tol` metres of the line through their
/// neighbours kept (Douglas-Peucker): a lane is sampled every metre or two, a ribbon needs
/// only its bends. The launcher's map picture draws its roads with this too.
pub(crate) fn simplify(pts: &[Vec3], tol: f32) -> Vec<Vec3> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let mut stack = vec![(0usize, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (pts[a].truncate(), pts[b].truncate());
        let ab = pb - pa;
        let len = ab.length().max(1e-6);
        let mut worst = (0.0f32, 0usize);
        for k in a + 1..b {
            let d = (pts[k].truncate() - pa).perp_dot(ab).abs() / len;
            if d > worst.0 {
                worst = (d, k);
            }
        }
        if worst.0 > tol {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    pts.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// The map as the game's city map has it (the game program's `Host::city_map`): its roads, the
/// linked lane network behind them, every placed object's place and the bus stops.
pub struct CityMap {
    pub roads: Vec<MapRoad>,
    pub net: Network,
    pub places: HashMap<i64, DVec3>,
    pub stops: Vec<MapStop>,
}

/// Read the map's tiles `tiles` (of the installation `root`, the chrono folders `chrono`) as the
/// city map reads them.
pub fn city_map(root: &Path, tiles: &[(i32, i32, PathBuf)], chrono: &[PathBuf]) -> CityMap {
    let NavigationMap { lanes, road_surfaces, places, signs: _, stops } = navigation_map_of(root, tiles, chrono);
    let (roads, net) = city_roads(lanes, &road_surfaces);
    CityMap { roads, net, places, stops }
}
