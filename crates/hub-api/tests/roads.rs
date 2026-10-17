//! The maps' roads end to end: a synthetic OMSI 2 folder with a map of two tiles (written as
//! OMSI writes them, UTF-16 with its BOM), a spline file with two street lanes, and a timetable
//! whose trip drives three splines - the route map (`hub_route_map`) and the duty map
//! (`co_duty_map`) draw its roads and the trip's way over them.

use hub_api::{call, Quiet};
use hub_omsi::timetable::{write_text, BusStopEntry, Line, Tour, TourTrip, Track, TrackEntry, Trip, TripProfile, TripStop};
use hub_omsi::Codepage;
use serde_json::{json, Value};
use std::path::Path;

/// A text as OMSI writes a map file: UTF-16 little-endian, its BOM first, `\r\n`.
fn utf16(path: &Path, text: &str) {
    let mut bytes = vec![0xFF, 0xFE];
    for u in text.replace('\n', "\r\n").encode_utf16() {
        bytes.extend(u.to_le_bytes());
    }
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A `[spline]` block: ID, its neighbours, its start, heading, length and radius.
fn spline(id: i64, prev: i64, next: i64, at: [f64; 2], heading: f64, length: f64, radius: f64) -> String {
    format!("[spline]\n0\nSplines\\Test\\road.sli\n{id}\n{prev}\n{next}\n{}\n0\n{}\n{heading}\n{length}\n{radius}\n0\n0\n0\n0\n0\n0\n0\n\n", at[0], at[1])
}

fn omsi2(base: &Path) -> std::path::PathBuf {
    let root = base.join("OMSI 2");
    let map = root.join("maps/Roadtown");
    for d in ["Vehicles", "Sceneryobjects", "Splines"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("Omsi.exe"), b"").unwrap();
    utf16(&map.join("global.cfg"), "[name]\nRoadtown\n\n[version]\n14\n\n[map]\n0\n0\ntile_0_0.map\n\n[map]\n1\n0\ntile_1_0.map\n");
    // tile 0: 250 m east from (20, 100), then a right half circle of radius 25 back west; tile 1:
    // a road 300 m north, and a stop beside the first road
    let half = 25.0 * std::f64::consts::PI;
    utf16(
        &map.join("tile_0_0.map"),
        &format!(
            "[version]\n14\n\n{}{}[object]\n0\nSceneryobjects\\stop.sco\n900\n120\n96\n0\n0\n0\n0\n\n",
            spline(1, -1, 2, [20.0, 100.0], 90.0, 250.0, 0.0),
            spline(2, 1, -1, [270.0, 100.0], 90.0, half, 25.0)
        ),
    );
    utf16(&map.join("tile_1_0.map"), &format!("[version]\n14\n\n{}", spline(7, -1, -1, [50.0, 0.0], 0.0, 300.0, 0.0)));
    std::fs::create_dir_all(root.join("Splines/Test")).unwrap();
    std::fs::write(root.join("Splines/Test/road.sli"), "[path]\r\n0\r\n-1.75\r\n0\r\n3.5\r\n1\r\n\r\n[path]\r\n0\r\n1.75\r\n0\r\n3.5\r\n0\r\n").unwrap();
    // the timetable: line 1, a trip east along the road and round the bend, then north in tile 1
    let tt = map.join("TTData");
    std::fs::create_dir_all(&tt).unwrap();
    let trip = Trip {
        name: "1_out".into(),
        terminus: "Bend".into(),
        line: "1".into(),
        stops: vec![TripStop { id: 900, name: "Market".into(), ..TripStop::default() }],
        profiles: vec![TripProfile { name: "standard".into(), minutes: 5.0, ..TripProfile::default() }],
        ..Trip::default()
    };
    write_text(&tt.join("1_out.ttp"), &trip.to_text(), Codepage::Windows1252).unwrap();
    let entry = |spline: i64, tile: usize| TrackEntry { spline, path: 1, tile, length: 10.0, ..TrackEntry::default() };
    let track = Track { name: "1_out".into(), entries: vec![entry(1, 0), entry(2, 0), entry(7, 1)], ..Track::default() };
    write_text(&tt.join("1_out.ttr"), &track.to_text(), Codepage::Windows1252).unwrap();
    let line = Line {
        name: "1".into(),
        user_allowed: true,
        tours: vec![Tour { number: "1".into(), ai_group: "Busses".into(), trips: vec![TourTrip { trip: "1_out".into(), profile: 0, departure: 360.0 }], ..Tour::default() }],
        ..Line::default()
    };
    write_text(&tt.join("1.ttl"), &line.to_text(), Codepage::Windows1252).unwrap();
    write_text(&tt.join("Busstops.cfg"), &BusStopEntry { name: "Market".into(), tile: 0, object_id: 900, ..BusStopEntry::default() }.to_text(), Codepage::Windows1252).unwrap();
    root
}

fn points(v: &Value) -> Vec<[f64; 2]> {
    v.as_array().unwrap().chunks(2).map(|c| [c[0].as_f64().unwrap(), c[1].as_f64().unwrap()]).collect()
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() < 0.11
}

#[test]
fn the_maps_draw_the_roads_and_the_trip_s_way_over_them() {
    let base = std::env::temp_dir().join(format!("hub-api-roads-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = omsi2(&base);
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, base.join("data"));
    hub_core::settings::Settings { omsi2_folder: root.to_string_lossy().into_owned(), ..Default::default() }.save().unwrap();
    const MAP: &str = "maps/Roadtown/global.cfg";

    let m = call("hub_route_map", &json!({ "map": MAP, "trip": "1_out" }), &Quiet).unwrap();
    let roads = m["roads"].as_array().unwrap();
    assert_eq!(roads.len(), 3, "{m}");
    let first = points(&roads[0]["p"]);
    assert!(near(first[0], [20.0, 100.0]) && near(*first.last().unwrap(), [270.0, 100.0]), "{first:?}");
    assert_eq!((roads[0]["w"].as_f64(), roads[0]["main"].as_bool()), (Some(7.0), Some(false)));
    // (the bend: a half circle to the right, from heading east to heading west, 50 m south)
    let bend = points(&roads[1]["p"]);
    assert!(near(*bend.last().unwrap(), [270.0, 50.0]), "{:?}", bend.last());
    // (tile 1 begins 300 m east)
    assert!(near(points(&roads[2]["p"])[0], [350.0, 0.0]));
    assert_eq!((m["lo"].clone(), m["hi"].clone()), (json!([0.0, 0.0]), json!([600.0, 300.0])));

    // the trip on its right-hand lane: east 1.75 m south of the road's line, round the bend on the
    // inside, then from tile 0 over to tile 1's road (an object between them: bridged straight)
    let route = points(&m["route"][0]);
    assert!(near(route[0], [20.0, 98.25]) && near(route[1], [270.0, 98.25]), "{:?}", &route[..2]);
    let turned = route.iter().position(|p| near(*p, [270.0, 51.75])).expect("the bend's end on the inside");
    assert!(route[1..=turned].iter().all(|p| p[0] >= 270.0 - 0.1), "the bend turns right, east of where it begins");
    assert!(near(route[turned + 1], [351.75, 0.0]) && near(*route.last().unwrap(), [351.75, 300.0]), "{:?}", &route[turned..]);

    // the duty map: the same roads, a route for each trip of the duty
    let d = call("co_duty_map", &json!({ "map": MAP, "date": "1989-05-30", "trips": ["1_out", "none"] }), &Quiet).unwrap();
    assert_eq!((d["roads"].as_array().unwrap().len(), d["route"].as_array().unwrap().len()), (3, 1), "a trip without a track draws nothing");
    assert!(call("hub_route_map", &json!({ "map": "maps/Nowhere/global.cfg" }), &Quiet).is_err());

    // what the reading says of the map in the log: every spline meets its next one
    let log = std::fs::read_to_string(base.join("data").join(hub_core::log::FILE)).unwrap_or_default();
    assert!(log.contains("roads: maps/Roadtown/global.cfg: 2 tiles, 3 splines, 3 roads; 1 of 1 splines meet their next one"), "{log}");
    let _ = std::fs::remove_dir_all(&base);
}
