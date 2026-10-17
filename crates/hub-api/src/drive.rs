//! A drive started in the player's openOMSI: the page's [`Duty`] made into the official command
//! line ([`launch_spec`]), the game started from the openOMSI folder of the settings, watched,
//! and its end told to the page (`game-ended`).
//!
//! What the command line cannot say - a company's duty, its legs, a free line, the display
//! font, the object editor - goes to the plugin as its assignment (`hub_game::bridge`): **the
//! plugin port** writes it before the start ([`start`] marks the place).

use crate::{to_json, Answer, Platform};
use hub_core::Duty;
use hub_game::launch::{Lan, LaunchSpec};
use hub_game::process::{self, Game};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

/// The game Omsi-Hub started (one at a time).
static GAME: Mutex<Option<Game>> = Mutex::new(None);

fn some(s: &Option<String>) -> Option<String> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// The command line of a drive: official flags only.
pub fn launch_spec(d: &Duty, root: &Path) -> LaunchSpec {
    let mut s = LaunchSpec { root: Some(root.to_path_buf()), ..LaunchSpec::default() };
    // (a tutorial or a situation is all the game needs)
    if let Some(t) = d.tutorial {
        s.tutorial = Some(t);
        return s;
    }
    if let Some(sit) = some(&d.situation) {
        s.situation = Some(sit);
        return s;
    }
    s.map = Some(d.map.clone()).filter(|m| !m.trim().is_empty());
    s.bus = Some(d.bus.clone()).filter(|b| !b.trim().is_empty());
    s.paint = some(&d.paint);
    s.plate = some(&d.plate);
    s.number = some(&d.number);
    s.hof = some(&d.hof);
    s.set_vars = d.set_vars.clone();
    s.entry = d.entry.and_then(|e| usize::try_from(e).ok());
    s.date = some(&d.date);
    s.time = Some(d.time.trim().to_string()).filter(|t| !t.is_empty());
    s.weather = some(&d.weather);
    s.season = some(&d.season);
    s.traffic = d.traffic;
    s.passengers = d.passengers == Some(true);
    s.schedule = d.schedule == Some(true);
    s.autostart = d.autostart == Some(true);
    s.on_foot = d.on_foot == Some(true);
    // (a free drive along a line has no tour: the plugin follows the route)
    if d.free_line.is_none() {
        s.line = some(&d.line);
        s.tour = some(&d.tour).filter(|_| s.line.is_some());
        s.trip = some(&d.trip).filter(|_| s.tour.is_some());
        s.whole_tour = d.whole_tour && s.trip.is_some();
    }
    s.lan = d.lan.as_deref().map(str::trim).and_then(|l| {
        if l == "host" {
            Some(Lan::Host(0))
        } else {
            l.strip_prefix("join").map(|r| Lan::Join(Some(r.trim_start_matches(':').trim()).filter(|r| !r.is_empty()).unwrap_or("auto").to_string()))
        }
    });
    s.lan_name = some(&d.lan_name);
    s.no_menu = s.map.is_some() && s.bus.is_some();
    s
}

/// The command line `hub_drive` would start (the page shows it; the tests check it).
pub fn args(d: &Duty) -> Answer {
    let s = hub_core::settings::Settings::load();
    let spec = launch_spec(d, Path::new(&s.omsi2_folder));
    let a = spec.args().map_err(|e| e.to_string())?;
    Ok(json!({ "args": a }))
}

/// The drive started.
pub fn start(d: &Duty, p: &dyn Platform) -> Answer {
    let s = hub_core::settings::Settings::load();
    if !s.has_folders() {
        return Err("Choose your openOMSI and OMSI 2 folders first".into());
    }
    let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(g) = game.as_mut() {
        if g.poll().is_none() {
            return Err("openOMSI is running already".into());
        }
    }
    let checked = hub_game::check_folder(Path::new(&s.openomsi_folder), &hub_core::paths::home_dir()).map_err(|e| e.text().to_string())?;
    if checked.compat.level == hub_game::compat::Level::Unsupported {
        return Err(checked.compat.notes.first().cloned().unwrap_or_else(|| "This openOMSI is not supported".into()));
    }
    // (the plugin port: the assignment for the plugin is written here, before the start)
    let spec = launch_spec(d, Path::new(&s.omsi2_folder));
    let g = process::start(&checked.install.exe, &spec).map_err(|e| e.to_string())?;
    let pid = g.pid();
    hub_core::log_line(&format!("drive: openOMSI started (pid {pid})"));
    *game = Some(g);
    drop(game);
    watch(p.emitter());
    Ok(json!({ "started": true, "pid": pid }))
}

/// Watch the game on a thread of its own until it ends, then tell the page.
fn watch(emit: Option<crate::Emitter>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
        let Some(g) = game.as_mut() else { return };
        if let Some(ended) = g.poll() {
            *game = None;
            drop(game);
            hub_core::log_line(&format!("drive: openOMSI ended ({:?}) after {:.0} s", ended.code, ended.seconds));
            if let Some(e) = &emit {
                e("game-ended", to_json(ended).unwrap_or(Value::Null));
            }
            return;
        }
    });
}

/// Whether a game Omsi-Hub started runs.
pub fn state() -> Value {
    let mut game = GAME.lock().unwrap_or_else(|e| e.into_inner());
    let running = match game.as_mut() {
        Some(g) => g.poll().is_none().then(|| g.pid()),
        None => None,
    };
    match running {
        Some(pid) => json!({ "running": true, "pid": pid }),
        None => {
            *game = None;
            json!({ "running": false })
        }
    }
}

/// The game ended by the app.
pub fn stop(_p: &dyn Platform) -> Value {
    let taken = GAME.lock().unwrap_or_else(|e| e.into_inner()).take();
    match taken {
        Some(mut g) => to_json(g.stop()).unwrap_or(Value::Null),
        None => json!({ "running": false }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duty() -> Duty {
        Duty { map: "maps/Grundorf/global.cfg".into(), bus: "Vehicles/X/a.bus".into(), time: "08:15".into(), line: Some("10".into()), tour: Some("2".into()), autostart: Some(true), ..Duty::default() }
    }

    #[test]
    fn a_duty_becomes_official_flags() {
        let s = launch_spec(&duty(), Path::new("C:/OMSI 2"));
        let a = s.args().unwrap();
        assert_eq!(
            a,
            ["--root", "C:/OMSI 2", "--map", "maps/Grundorf/global.cfg", "--bus", "Vehicles/X/a.bus", "--time", "08:15", "--schedule", "--line", "10", "--tour", "2", "--autostart", "--no-menu"]
        );
    }

    #[test]
    fn what_only_the_plugin_can_do_stays_off_the_command_line() {
        let free = Duty { free_line: Some("10_a".into()), company: Some("c1".into()), legs: vec!["11/3".into()], editor: true, display_font: Some("x".into()), ..duty() };
        let a = launch_spec(&free, Path::new("C:/OMSI 2")).args().unwrap();
        assert!(!a.iter().any(|x| x == "--line" || x == "--tour" || x == "--company" || x == "--free-line" || x == "--duty-leg"), "{a:?}");
        let host = Duty { lan: Some("host".into()), ..duty() };
        assert!(launch_spec(&host, Path::new("r")).args().unwrap().windows(2).any(|w| w == ["--lan-host", "0"]));
        let join = Duty { lan: Some("join:".into()), ..duty() };
        assert!(launch_spec(&join, Path::new("r")).args().unwrap().windows(2).any(|w| w == ["--lan-join", "auto"]));
        let tut = Duty { tutorial: Some(2), ..duty() };
        assert_eq!(launch_spec(&tut, Path::new("r")).args().unwrap(), ["--root", "r", "--tutorial", "2"]);
    }
}
