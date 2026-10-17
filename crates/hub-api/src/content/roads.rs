//! The maps' roads for the pages that draw them (`hub_omsi::roads`): the route map of the drive's
//! steps (`hub_route_map`), the company's duty map (`co_duty_map`) and its fleet map
//! (`co_more_map`). A map's tiles are read once and kept (Omsi-Hub never changes them); its
//! timetable is read each time, as the editors change it.
//!
//! What a page gets: `{ roads: [{ p: [x, y, x, y …], w, main }], route: [[x, y …]], lo, hi }` in
//! the map's metres, y north - each road a spline's street lanes (`w` their width), `route` the
//! way each trip asked for drives.

use super::map_dir_rel;
use hub_omsi::map::GlobalCfg;
use hub_omsi::roads::{MapRoads, P};
use hub_omsi::timetable::Timetable;
use hub_omsi::Vfs;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A map's roads as read, and as the pages draw them.
pub struct Read {
    pub roads: MapRoads,
    /// `roads` as JSON (made once: a big map has thousands).
    pub drawn: Value,
}

/// The maps read, the latest last (a few are kept).
static KEPT: Mutex<Vec<(PathBuf, Arc<Read>)>> = Mutex::new(Vec::new());
const KEEP: usize = 3;

/// A number as the pages get it: to the decimetre.
fn dm(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// A line as the pages draw it: `[x, y, x, y …]`.
pub fn flat(points: &[P]) -> Vec<f64> {
    points.iter().flat_map(|p| [dm(p[0]), dm(p[1])]).collect()
}

/// The roads of `map` (its `global.cfg`, relative), read the first time it is asked for.
pub fn of(vfs: &Vfs, map: &str) -> Result<Arc<Read>, String> {
    let path = vfs.resolve(map).filter(|p| p.is_file()).ok_or_else(|| format!("The map {map} is not there"))?;
    if let Some((_, r)) = KEPT.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(p, _)| *p == path) {
        return Ok(r.clone());
    }
    let g = GlobalCfg::load(&path).map_err(|e| format!("{map}: {e}"))?;
    let dir = path.parent().map(PathBuf::from).unwrap_or_default();
    let started = std::time::Instant::now();
    let roads = MapRoads::read(vfs, &g, &dir);
    let drawn: Vec<Value> = roads.roads().iter().filter(|r| r.points.len() >= 2).map(|r| json!({ "p": flat(&r.points), "w": dm(r.width), "main": r.main })).collect();
    // (a check of the reading on the player's own maps: their splines join end to end)
    let j = roads.joins();
    let splines: usize = roads.tiles.iter().map(|t| t.splines.len()).sum();
    hub_core::log_line(&format!(
        "roads: {map}: {} tiles, {splines} splines, {} roads; {} of {} splines meet their next one; read in {:.1} s",
        roads.tiles.len(),
        drawn.len(),
        j.met,
        j.checked,
        started.elapsed().as_secs_f64()
    ));
    let read = Arc::new(Read { roads, drawn: Value::Array(drawn) });
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    kept.push((path, read.clone()));
    if kept.len() > KEEP {
        kept.remove(0);
    }
    Ok(read)
}

/// The map's timetable as the game reads it (the content folder's copy of its `TTData` first).
pub fn timetable(vfs: &Vfs, map: &str) -> Timetable {
    vfs.resolve(&format!("{}/TTData", map_dir_rel(map))).filter(|p| p.is_dir()).map(|d| Timetable::load(&d)).unwrap_or_default()
}

/// The way the trip `name` drives (its track's lanes), empty when it has none on the map's
/// splines.
pub fn route(r: &Read, tt: &Timetable, name: &str) -> Vec<P> {
    tt.trip(name).and_then(|t| tt.track_of(t)).map(|track| r.roads.route(track)).unwrap_or_default()
}

/// Where the stops with the object IDs `ids` are (`Busstops.cfg` says the tile), each where the
/// map has it.
pub fn stops(r: &Read, tt: &Timetable, ids: &[i64]) -> Vec<Option<P>> {
    ids.iter().map(|id| tt.bus_stops.iter().find(|b| b.object_id == *id).and_then(|b| usize::try_from(b.tile).ok()).and_then(|t| r.roads.object(t, *id))).collect()
}

/// A map page's drawing: the roads, the routes of `trips` (each one found), the frame, and what
/// `more` adds.
pub fn page(vfs: &Vfs, map: &str, trips: &[String], more: Value) -> Result<Value, String> {
    let r = of(vfs, map)?;
    let tt = if trips.is_empty() { Timetable::default() } else { timetable(vfs, map) };
    let routes: Vec<Value> = trips.iter().map(|t| route(&r, &tt, t)).filter(|p| p.len() >= 2).map(|p| json!(flat(&p))).collect();
    let mut v = json!({ "roads": r.drawn, "route": routes, "lo": [r.roads.lo[0], r.roads.lo[1]], "hi": [r.roads.hi[0], r.roads.hi[1]] });
    if let (Some(o), Value::Object(m)) = (v.as_object_mut(), more) {
        o.extend(m);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_flat_to_the_decimetre() {
        assert_eq!(flat(&[[1.04, 2.06], [-3.0, 0.0]]), [1.0, 2.1, -3.0, 0.0]);
    }
}
