//! A game for trying the Omsi-Hub Companion app without openOMSI: Omsi-Hub's plugin, run on the
//! harness's mock of the plugin API, drives a duty of two trips, and the add-on's companion
//! server (`hub-companion`) serves it to the app as the launcher would. Pair the app with the
//! address and the code this prints.
//!
//! The bus is `hub-devices`' synthetic test bus, written into a game folder of its own: an IBIS
//! with a touch screen (its menu key shows the menu page, OK goes back) and a keypad, and a
//! ticket printer of six keys - drawn in the app as the add-on reads them from the bus's files.
//!
//! ```text
//! cargo run -p hub-plugin-harness --example fakegame
//! ```

use hub_companion::{Companion, Config, Source};
use hub_game::bridge::Assignment;
use hub_game::install;
use hub_plugin_harness::{Duty, GameState, Harness, Stop, Trip};
use serde_json::json;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The route: north, east, north again (map metres).
const ROUTE: [[f64; 2]; 4] = [[0.0, 0.0], [0.0, 900.0], [700.0, 900.0], [700.0, 1800.0]];

struct Game {
    dir: PathBuf,
    omsi: PathBuf,
}

impl Source for Game {
    fn plugin_dir(&self) -> Option<PathBuf> {
        Some(self.dir.clone())
    }
    fn assignment(&self) -> Option<Assignment> {
        None
    }
    fn route(&self, _map: &str, _date: &str, _trip: &str) -> Option<Vec<[f64; 2]>> {
        Some(ROUTE.to_vec())
    }
    fn roads(&self, _map: &str, _date: &str, at: [f64; 2], r: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
        // a grid of streets every 300 m
        let mut out = Vec::new();
        let (x0, x1, y0, y1) = (at[0] - r, at[0] + r, at[1] - r, at[1] + r);
        let mut v = (x0 / 300.0).floor() * 300.0;
        while v <= x1 {
            out.push((10.0, vec![[v, y0], [v, y1]]));
            v += 300.0;
        }
        let mut v = (y0 / 300.0).floor() * 300.0;
        while v <= y1 {
            out.push((10.0, vec![[x0, v], [x1, v]]));
            v += 300.0;
        }
        out
    }
    fn world_of(&self, _map: &str, tile: [i64; 2], local: [f64; 2]) -> Option<[f64; 2]> {
        Some([tile[0] as f64 * 300.0 + local[0], tile[1] as f64 * 300.0 + local[1]])
    }
    fn bus_model(&self, bus_file: &str) -> Option<(PathBuf, PathBuf)> {
        let bus = bus_file.split(['\\', '/']).fold(self.omsi.clone(), |d, p| d.join(p));
        Some((hub_companion::devices::model_of_bus(&bus)?, self.omsi.clone()))
    }
}

/// Where the bus is `d` metres along the route, and its heading.
fn along(d: f64) -> (f64, f64, f64) {
    let mut left = d;
    for w in ROUTE.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let heading = (b[0] - a[0]).atan2(b[1] - a[1]).to_degrees().rem_euclid(360.0);
        if left <= len {
            let t = left / len;
            return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, heading);
        }
        left -= len;
    }
    let last = ROUTE[ROUTE.len() - 1];
    (last[0], last[1], 0.0)
}

fn main() {
    let dir = std::env::temp_dir().join(format!("hub-fakegame-{}", std::process::id()));
    let plugins = dir.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    let plugin = install::plugin_dir(&plugins);
    let omsi = dir.join("OMSI 2");
    hub_devices::testbus::write(&omsi).expect("the test bus");

    let cfg =
        Config { port: hub_companion::DEFAULT_PORT, udp_port: hub_companion::UDP_PORT, name: "Fake game".into(), version: env!("CARGO_PKG_VERSION").into(), devices_file: dir.join("devices.json") };
    let companion = Companion::start(cfg, Arc::new(Game { dir: plugin.clone(), omsi: omsi.clone() })).expect("the companion's ports are free");
    let status = companion.status();
    println!("companion on port {}, pairing code {}", companion.port(), status["code"].as_str().unwrap_or("?"));
    println!("{}", status["pair_url"].as_str().unwrap_or(""));

    let stops = |names: &[&str], from: f64| -> Vec<Stop> {
        names.iter().enumerate().map(|(i, n)| Stop { id: i as i64 + 1, name: (*n).into(), arr: from + i as f64 * 120.0, dep: from + i as f64 * 120.0 }).collect()
    };
    let out = ["Bismarckplatz", "Hauptbahnhof", "Kunsthalle", "Universitätsplatz", "Rathaus", "Markgraf-Berthold-Platz"];
    let mut back = out;
    back.reverse();
    let clock0 = 8.0 * 3600.0 + 20.0 * 60.0;
    let trip = |name: &str, terminus: &str, from: f64, s: Vec<Stop>| Trip { name: name.into(), line: "29".into(), terminus: terminus.into(), departure: from, arrival: from + 600.0, stops: s };
    let mut game = GameState {
        trips: vec![
            trip("Bismarckplatz - Markgraf-Berthold-Platz", "Markgraf-Berthold-Platz", clock0 + 60.0, stops(&out, clock0 + 60.0)),
            trip("Markgraf-Berthold-Platz - Bismarckplatz", "Bismarckplatz", clock0 + 1200.0, stops(&back, clock0 + 1200.0)),
        ],
        position: Some((0.0, 0.0, 0.0)),
        clock: clock0,
        vehicle: Some("Test bus".into()),
        bus_file: hub_devices::testbus::BUS.into(),
        ..GameState::default()
    };
    let mut triggers: Vec<String> = (0..10).map(|d| format!("IBIS_{d}")).collect();
    triggers.extend(["IBIS_eingabe", "IBIS_loeschen", "IBIS_setmode_linie_kurs", "IBIS_setmode_ziel", "IBIS_vor", "IBIS_menu", "IBIS_ok"].map(String::from));
    triggers.extend((1..=6).map(|k| format!("ticketprinter_button_ticket_{k}")));
    game.triggers = triggers;
    game.vars.insert("IBIS_LinieKurs".into(), 29.0);
    game.vars.insert("IBIS_TerminusCode".into(), 145.0);
    game.strs.insert("IBIS_terminus_name".into(), "Markgraf-Berthold-Platz".into());
    // the test bus's IBIS: its pages, its icon, and what its script set at the start
    game.vars.insert("IBIS_mode".into(), 0.0);
    game.vars.insert("IBIS_stop".into(), 0.0);
    game.vars.insert("trans_dauer".into(), 1.0);
    game.strs.insert("IBIS_line_text".into(), "29/03  145".into());
    game.strs.insert("IBIS_dest_text".into(), "MARKGRAF-BERTHOLD-PL.".into());
    game.tickets = vec![("Einzelfahrschein".into(), 2.9), ("Tageskarte".into(), 7.0)];
    game.ticket_request = Some(("Einzelfahrschein".into(), 2.9));
    game.lines = json!([{ "name": "29", "user_allowed": true, "tours": [{ "number": 3, "today": true }] }]);
    game.duty = Some(Duty { line: "29".into(), tour: "3".into(), trip: 1, trips: 2, delay: 90.0 });

    let mut h = Harness::load(&plugin, game).expect("the plugin loads");
    let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut d = 0.0;
    loop {
        {
            let mut s = h.state.borrow_mut();
            s.clock += 0.5;
            d += 4.0;
            let (x, y, heading) = along(d % 2400.0);
            s.position = Some((x, y, heading));
            s.speed = 29.0;
        }
        h.frame().expect("a frame");
        // what the plugin sent its companion server goes over UDP, as `omsi.send` does
        let sent: Vec<(i64, String)> = std::mem::take(&mut h.state.borrow_mut().sent);
        for (port, text) in sent {
            let _ = udp.send_to(text.as_bytes(), ("127.0.0.1", port as u16));
        }
        let pressed: Vec<String> = std::mem::take(&mut h.state.borrow_mut().pressed);
        let held: Vec<(String, bool)> = std::mem::take(&mut h.state.borrow_mut().held);
        for p in pressed.iter().cloned().chain(held.iter().filter(|h| h.1).map(|h| h.0.clone())) {
            println!("the bus's key: {p}");
            // (the test bus's script, as far as the demo goes: the menu page and back)
            let mut s = h.state.borrow_mut();
            match p.as_str() {
                "IBIS_menu" => {
                    s.vars.insert("IBIS_mode".into(), 1.0);
                }
                "IBIS_ok" => {
                    s.vars.insert("IBIS_mode".into(), 0.0);
                }
                _ => {}
            }
        }
        {
            let mut s = h.state.borrow_mut();
            let stop = if (s.clock as i64 / 20) % 2 == 0 { 1.0 } else { 0.0 };
            s.vars.insert("IBIS_stop".into(), stop);
            let t = s.clock as i64;
            s.strs.insert("IBIS_line_text".into(), format!("29/03 {:02}:{:02}", t / 3600 % 24, t / 60 % 60));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
