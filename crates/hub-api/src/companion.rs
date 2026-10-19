//! The phone and tablet companion (`hub_companion_`): `hub-companion`'s server, run by the app
//! while the player has it on (Settings › General › Phone & tablet; off by default), and what
//! it needs of the rest of Omsi-Hub - the plugin's folder, the assignment last given, and the
//! map's routes and roads as the route map reads them (`content::mapview`).
//!
//! * `hub_companion_state` - on or off, the port, and while on: the address, the pairing code,
//!   the QR code of the pairing address (`{size, bits}`, as the duty card draws one), the paired
//!   devices and whether the game is heard;
//! * `hub_companion_set {on, port?}` - switched on or off (kept in the settings: `companion`,
//!   `companion_port`), answered with the state;
//! * `hub_companion_code` - another pairing code;
//! * `hub_companion_forget {id}` - a paired device removed.
//!
//! [`start_if_on`] starts it with the app.

use crate::{arg, Answer, Platform};
use hub_companion::{Companion, Config, Source};
use hub_core::settings::Settings;
use hub_game::bridge::Assignment;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The settings' keys.
const ON: &str = "companion";
const PORT: &str = "companion_port";

/// The server while it runs, and the last error starting it.
static RUNNING: Mutex<Option<Companion>> = Mutex::new(None);
static FAILED: Mutex<String> = Mutex::new(String::new());
/// The assignment the plugin was last given (`addon::give`).
static ASSIGNED: Mutex<Option<Assignment>> = Mutex::new(None);

/// The assignment just given to the plugin: the phone's duty card.
pub fn assigned(a: &Assignment) {
    *ASSIGNED.lock().unwrap_or_else(|e| e.into_inner()) = Some(a.clone());
}

/// What the server needs of Omsi-Hub.
struct Hub;

/// The map as the launcher names it (`maps/<name>/global.cfg`), from the game's path of it.
fn map_name(game: &str) -> String {
    let m = game.replace('\\', "/");
    match m.to_lowercase().rfind("maps/") {
        Some(i) => m[i..].to_string(),
        None => m,
    }
}

fn global_of(map: &str) -> Option<PathBuf> {
    hub_core::paths::resolve_content(&map_name(map)).filter(|p| p.is_file())
}

impl Source for Hub {
    fn plugin_dir(&self) -> Option<PathBuf> {
        crate::addon::game_plugin_dir().filter(|d| d.is_dir())
    }

    fn assignment(&self) -> Option<Assignment> {
        ASSIGNED.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn route(&self, map: &str, date: &str, trip: &str) -> Option<Vec<[f64; 2]>> {
        let global = global_of(map)?;
        Some(crate::content::mapview::route_line(&map_name(map), &global, date, trip)).filter(|r| r.len() >= 2)
    }

    fn roads(&self, map: &str, date: &str, at: [f64; 2], r: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
        global_of(map).map(|g| crate::content::mapview::roads_near(&map_name(map), &g, date, at, r)).unwrap_or_default()
    }

    fn world_of(&self, map: &str, tile: [i64; 2], local: [f64; 2]) -> Option<[f64; 2]> {
        let global = global_of(map)?;
        Some(crate::content::mapview::world_of(&map_name(map), &global, "", tile, local))
    }

    fn bus_model(&self, bus_file: &str) -> Option<(PathBuf, PathBuf)> {
        let bus = hub_core::paths::resolve_content(bus_file).filter(|p| p.is_file())?;
        let cfg = hub_companion::devices::model_of_bus(&bus)?;
        // (the game's folder the bus is read in: OMSI 2's own when it lies there - its Fonts and
        // Texture are there - else the content folder it came from)
        let omsi = hub_core::paths::root().filter(|r| bus.starts_with(r)).or_else(|| hub_core::paths::bases().into_iter().find(|b| bus.starts_with(b)))?;
        Some((cfg, omsi))
    }
}

/// This computer's name, as the app shows it.
fn computer_name() -> String {
    let env = ["COMPUTERNAME", "HOSTNAME"].iter().find_map(|k| std::env::var(k).ok());
    let file = || std::fs::read_to_string("/etc/hostname").ok();
    env.or_else(file).map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| "Omsi-Hub".into())
}

fn setting_on(s: &Settings) -> bool {
    s.other.get(ON).and_then(Value::as_bool).unwrap_or(false)
}

fn setting_port(s: &Settings) -> u16 {
    s.other.get(PORT).and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok()).filter(|p| *p >= 1024).unwrap_or(hub_companion::DEFAULT_PORT)
}

/// Start the server (`port`), stopping one that runs.
fn start(port: u16) -> Result<(), String> {
    let mut running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    // (the old one lets its ports go first)
    *running = None;
    let cfg =
        Config { port, udp_port: hub_companion::UDP_PORT, name: computer_name(), version: env!("CARGO_PKG_VERSION").into(), devices_file: hub_core::data_dir().join(hub_companion::DEVICES_FILE) };
    match Companion::start(cfg, Arc::new(Hub)) {
        Ok(c) => {
            hub_core::log_line(&format!("companion: listening on port {}", c.port()));
            *running = Some(c);
            FAILED.lock().unwrap_or_else(|e| e.into_inner()).clear();
            Ok(())
        }
        Err(e) => {
            let text = format!("The port {port} is taken or not allowed: {e}");
            hub_core::log_line(&format!("companion: {text}"));
            *FAILED.lock().unwrap_or_else(|e| e.into_inner()) = text.clone();
            Err(text)
        }
    }
}

/// With the app: the server started when the player has it on.
pub fn start_if_on() {
    let s = Settings::load();
    if setting_on(&s) {
        let _ = start(setting_port(&s));
    }
}

fn state() -> Value {
    let s = Settings::load();
    let running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    let Some(c) = running.as_ref() else {
        return json!({ "on": false, "wanted": setting_on(&s), "port": setting_port(&s), "error": FAILED.lock().unwrap_or_else(|e| e.into_inner()).clone() });
    };
    let mut v = c.status();
    let url = v["pair_url"].as_str().unwrap_or("").to_string();
    v["qr"] = hub_core::qr::encode(&url, hub_core::qr::Ecc::M).or_else(|| hub_core::qr::encode(&url, hub_core::qr::Ecc::L)).map(|q| json!({ "size": q.size, "bits": q.bits() })).unwrap_or(Value::Null);
    v["wanted"] = json!(true);
    v
}

pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    match cmd {
        "hub_companion_state" => Ok(state()),
        "hub_companion_set" => {
            let on = arg::<bool>(args, "on")?;
            let mut s = Settings::load();
            let port = arg::<Option<u16>>(args, "port")?.filter(|p| *p >= 1024).unwrap_or_else(|| setting_port(&s));
            s.other.insert(ON.into(), json!(on));
            s.other.insert(PORT.into(), json!(port));
            s.save().map_err(|e| e.to_string())?;
            if on {
                // (a port that will not open says so in the state)
                let _ = start(port);
            } else {
                *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = None;
            }
            Ok(state())
        }
        "hub_companion_code" => {
            if let Some(c) = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                c.new_code();
            }
            Ok(state())
        }
        "hub_companion_forget" => {
            let id = arg::<String>(args, "id")?;
            if let Some(c) = RUNNING.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                c.forget(&id);
            }
            Ok(state())
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_games_map_path_is_the_launchers_name_of_it() {
        assert_eq!(map_name("C:\\OMSI 2\\maps\\Grundorf\\global.cfg"), "maps/Grundorf/global.cfg");
        assert_eq!(map_name("maps/Grundorf/global.cfg"), "maps/Grundorf/global.cfg");
        assert_eq!(map_name("/home/luc/.steam/steamapps/common/OMSI 2/Maps/Ruhrau/global.cfg"), "Maps/Ruhrau/global.cfg");
    }

    #[test]
    fn off_by_default_and_a_port_below_1024_is_not_taken() {
        let s = Settings::default();
        assert!(!setting_on(&s));
        assert_eq!(setting_port(&s), hub_companion::DEFAULT_PORT);
        let mut s = Settings::default();
        s.other.insert(PORT.into(), json!(80));
        assert_eq!(setting_port(&s), hub_companion::DEFAULT_PORT);
        s.other.insert(PORT.into(), json!(50000));
        assert_eq!(setting_port(&s), 50000);
    }
}
