//! The maps on the website (`hub-api` without "native"): the desktop app cannot be asked, and the
//! map's tiles are not in the browser, so the roads and the routes come as the desktop app exported
//! them into the cloud save's content (`content::mapview::fleet_export` there, written into the
//! data folder's `web/map.json`: `{origin, lo, hi, roads, colors, tracks: {trip: {p, stops}}}`).
//! A trip the export lacks has no route; without the export the map is empty.

use serde_json::{json, Value};
use std::path::Path;

fn exported() -> Value {
    let p = hub_core::paths::data_dir().join("web").join("map.json");
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({}))
}

fn base(m: &Value) -> Value {
    json!({
        "origin": m.get("origin").cloned().unwrap_or(json!([0.0, 0.0])),
        "lo": m.get("lo").cloned().unwrap_or(json!([0.0, 0.0])),
        "hi": m.get("hi").cloned().unwrap_or(json!([0.0, 0.0])),
        "roads": m.get("roads").cloned().unwrap_or(json!([])),
        "colors": m.get("colors").cloned().unwrap_or(json!(["rgba(30,30,30,0.9)", "rgba(92,92,92,1)", "rgba(112,112,112,1)", "rgba(214,48,40,1)"])),
    })
}

fn track(m: &Value, name: &str) -> Value {
    m.get("tracks").and_then(|t| t.get(name)).cloned().unwrap_or_else(|| json!({ "p": [], "stops": [] }))
}

/// The route map of one trip (`export` on the desktop): its route and its stops.
pub fn export(_map: &str, _global: &Path, _date: &str, trip: &str, _tolerance: f32) -> Value {
    let m = exported();
    let mut v = base(&m);
    let t = track(&m, trip);
    let p = t.get("p").cloned().unwrap_or(json!([]));
    v["route"] = if p.as_array().is_some_and(|a| !a.is_empty()) { json!([p]) } else { json!([]) };
    v["stops"] = json!(t
        .get("stops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter(|(_, s)| s.is_array())
        .map(|(k, at)| json!({ "k": k, "at": at }))
        .collect::<Vec<_>>());
    v["entries"] = json!([]);
    v
}

/// The map with the routes of `trips` (`fleet_export` on the desktop).
pub fn fleet_export(_map: &str, _global: &Path, _date: &str, trips: &[String], _tolerance: f32) -> Value {
    let m = exported();
    let mut v = base(&m);
    v["tracks"] = json!(trips
        .iter()
        .filter(|n| !n.trim().is_empty())
        .map(|n| {
            let mut t = track(&m, n);
            t["name"] = json!(n);
            t
        })
        .collect::<Vec<_>>());
    v
}

/// The duty map: the routes of `trips` each as one route, and their stops (`trips_page`).
pub fn trips_page(map: &str, global: &Path, date: &str, trips: &[String]) -> Value {
    let mut v = fleet_export(map, global, date, trips, 0.6);
    let tracks = v.get("tracks").and_then(Value::as_array).cloned().unwrap_or_default();
    let route: Vec<Value> = tracks.iter().filter_map(|t| t.get("p").filter(|p| p.as_array().is_some_and(|a| !a.is_empty())).cloned()).collect();
    let stops: Vec<Value> = tracks.iter().flat_map(|t| t.get("stops").and_then(Value::as_array).cloned().unwrap_or_default()).filter(|s| s.is_array()).map(|at| json!({ "at": at, "k": -1 })).collect();
    v["route"] = json!(route);
    v["stops"] = json!(stops);
    v["entries"] = json!([]);
    if let Some(o) = v.as_object_mut() {
        o.remove("tracks");
    }
    v
}
