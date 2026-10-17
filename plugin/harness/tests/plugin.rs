//! The plugin run on the harness's mock of openOMSI's plugin API: a fake bus with a duty, a
//! few seconds of game, and what reaches Omsi-Hub through the bridge. Ported from the old
//! `omsihub-plugin/tests/plugin.rs`, which ran the plugin inside openOMSI's own runtime.

use hub_game::bridge::{self, Assignment, Notice, ReportReader};
use hub_game::{compat, install};
use hub_plugin_harness::{GameState, Harness, Trip, FUNCTIONS};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A plugins folder of its own for a test, the plugin installed in it as Omsi-Hub installs it.
fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-plugin-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

/// When the assignment was given: today (one of yesterday is not believed).
fn issued() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600
}

fn assignment() -> Assignment {
    let mut texts = BTreeMap::new();
    texts.insert("Start duty".to_string(), "Dienst starten".to_string());
    Assignment {
        issued: issued(),
        company: "Stadtbus Berlin".into(),
        duty: "D-12".into(),
        line: "10".into(),
        tour: "2".into(),
        auto_start: true,
        notices: vec![Notice { id: "n1".into(), title: "Depot".into(), text: "Bus 4711 is in the workshop".into(), kind: "info".into() }],
        texts,
        ..Assignment::default()
    }
}

/// The game: the duty's two trips without stops.
fn game() -> GameState {
    let trip = |n: &str| Trip { name: n.into(), line: "10".into(), ..Trip::default() };
    GameState { trips: vec![trip("Zoo - Hbf"), trip("Hbf - Zoo")], ..GameState::default() }
}

#[test]
fn the_duty_is_taken_its_trips_reported_and_the_notices_shown() {
    let dir = installed("full");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();

    // the handshake: everything the add-on uses is there
    let hello = bridge::read_hello(&dir).expect("data/hello.json");
    assert_eq!((hello.bridge, hello.plugin.as_str(), hello.api, hello.issued), (bridge::BRIDGE, install::VERSION, 1, issued()));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Supported, "{c:?}");

    h.frame().unwrap();
    // (auto_start: the duty is taken as soon as the player is in the bus)
    assert_eq!(h.state.borrow().started, [("10".to_string(), "2".to_string(), 0, 0)]);
    h.frame().unwrap();
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(95.0), json!(88.0), json!(100.0)]).unwrap();
    h.frame().unwrap();
    h.state.borrow_mut().duty = None;
    h.frame().unwrap();

    let reports = ReportReader::default().read_new(&dir);
    let kinds: Vec<&str> = reports.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["duty_start", "trip", "duty_end"]);
    let trip = &reports[1];
    assert_eq!((trip.issued, trip.duty.as_str(), trip.line.as_str(), trip.tour.as_str()), (issued(), "D-12", "10", "2"));
    assert_eq!((trip.trip, trip.trip_name.as_str(), trip.how.as_str()), (1, "Zoo - Hbf", "arrived"));
    assert_eq!((trip.trips, trip.number.as_str()), (2, "10"));
    assert_eq!((trip.driving, trip.comfort, trip.tickets, trip.delay), (Some(95.0), Some(88.0), Some(100.0), Some(42.0)));
    assert_eq!(trip.clock, Some(28_800.0));

    // the notice as a card, once; the company's panel
    let toasts: Vec<String> = h.state.borrow().toasts.iter().map(|t| t.text.clone()).collect();
    assert_eq!(toasts, ["Bus 4711 is in the workshop"]);
    let panel = h.state.borrow().panels.get("omsihub").cloned().expect("the company's panel");
    assert!(panel.to_string().contains("Stadtbus Berlin"), "{panel}");

    // a notice that comes while the game runs
    bridge::write_notices(&dir, &[Notice { id: "n2".into(), text: "Diversion on line 10".into(), ..Notice::default() }]).unwrap();
    for _ in 0..6 {
        h.frame().unwrap();
    }
    assert!(h.state.borrow().toasts.iter().any(|t| t.text == "Diversion on line 10"));
}

#[test]
fn an_older_openomsi_without_files_panels_or_game_functions_still_works() {
    let dir = installed("old");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, ..assignment() }).unwrap();
    // (an openOMSI before the file functions, the panels and `game.*`)
    let mut h = Harness::load_with(&dir, game(), "omsi.files = nil; omsi.ui = nil; omsi.game = nil; omsi.json = nil").unwrap();
    // the player takes the duty from the game's menu; trips are watched one by one
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 42.0 });
    for trip in [1, 1, 2, 2, 2] {
        h.state.borrow_mut().duty.as_mut().unwrap().trip = trip;
        h.frame().unwrap();
    }
    // nothing could be written: the handshake and the reports are in the log
    assert!(!dir.join(bridge::DATA_DIR).join(bridge::REPORTS_FILE).exists());
    let (hello, reports) = bridge::from_log(&h.log_text());
    let hello = hello.expect("the handshake in the log");
    assert_eq!(hello.functions.get("files.append"), Some(&false));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Limited);
    assert!(c.features.iter().any(|f| f.id == "reports" && !f.on));
    let kinds: Vec<&str> = reports.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["duty_start", "trip"]);
    assert_eq!((reports[1].trip, reports[1].trip_name.as_str()), (1, "Zoo - Hbf"));
    // the notice as a plain line of text
    let messages = h.state.borrow().messages.clone();
    assert!(messages.iter().any(|m| m == "Depot: Bus 4711 is in the workshop"), "{messages:?}");
}

#[test]
fn the_plugin_checks_every_function_the_add_on_negotiates() {
    let main = install::FILES[0].1;
    let needs = &main[main.find("local NEEDS = {").unwrap()..];
    let needs = &needs[..needs.find('}').unwrap()];
    let mut in_lua: Vec<&str> = needs.split('"').skip(1).step_by(2).collect();
    let mut in_rust: Vec<&str> = compat::FEATURES.iter().flat_map(|f| f.needs.iter().copied()).collect();
    in_lua.sort();
    in_rust.sort();
    // (a function two features need is looked up once)
    in_rust.dedup();
    assert_eq!(in_lua, in_rust);
    // (and the harness's mock of the API has each of them, so every feature is exercised)
    for n in in_rust {
        assert!(FUNCTIONS.contains(&n), "the mock lacks omsi.{n}");
    }
}

#[test]
fn an_assignment_of_an_earlier_day_is_not_taken() {
    let dir = installed("stale");
    bridge::write_assignment(&dir, &Assignment { issued: issued() - 2 * 86_400, ..assignment() }).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    h.frame().unwrap();
    // (no duty taken, no company's panel)
    assert!(h.state.borrow().started.is_empty());
    assert!(!h.state.borrow().panels.contains_key("omsihub"));
    assert_eq!(bridge::read_hello(&dir).map(|h| h.issued), Some(0));
}
