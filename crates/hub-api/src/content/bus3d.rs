//! The bus in 3D on the bus step (`Bus3D` of the page): the player's own openOMSI writes the bus
//! as glTF (`--export-glb`, what its own launcher draws its preview from) in the livery chosen
//! and with the bus options chosen (`busoptions::vars_for`), and the page draws the model
//! itself. Omsi-Hub reads no model of its own: every bus openOMSI can drive is shown.
//!
//! * `hub_bus3d {bus, paint, retry}` - the model of that look (`retry`: a look that failed tried
//!   again): `{ state: "ready", key }`, or
//!   `{ state: "making" }` while openOMSI writes it (asked again by the page), or
//!   `{ state: "failed", error }`;
//! * `hub_bus3d_glb {key}` (bytes) - the model's file.
//!
//! The models are kept in the data folder's `bus3d` (one per bus, livery, options, bus file and
//! openOMSI program), so a look is written once; one is written at a time.

use super::busoptions::vars_for;
use super::str_arg;
use crate::Answer;
use hub_core::settings::Settings;
use hub_game::launch::LaunchSpec;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// How long openOMSI may take to write a bus.
const WAIT: Duration = Duration::from_secs(180);

/// What is known of a look being made: still being written, or why it failed.
#[derive(Clone, Debug, PartialEq)]
enum Making {
    Busy,
    Failed(String),
}

static MAKING: Mutex<Option<HashMap<String, Making>>> = Mutex::new(None);
/// Held while openOMSI writes a model: one at a time.
static WRITING: Mutex<()> = Mutex::new(());

fn making() -> std::sync::MutexGuard<'static, Option<HashMap<String, Making>>> {
    MAKING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where the models are kept.
pub fn dir() -> PathBuf {
    hub_core::paths::data_dir().join("bus3d")
}

fn stamp(p: &Path) -> u64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// The key of a look: the bus, the livery, the options, and the bus file's and the program's
/// last change (a bus or an openOMSI updated is written again).
pub fn key(bus: &str, paint: &str, vars: &[(String, f32)], bus_file: Option<&Path>, exe: &Path) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bus.replace('\\', "/").to_lowercase().hash(&mut h);
    paint.hash(&mut h);
    for (n, v) in vars {
        n.hash(&mut h);
        v.to_bits().hash(&mut h);
    }
    bus_file.map(stamp).hash(&mut h);
    exe.hash(&mut h);
    stamp(exe).hash(&mut h);
    format!("{:016x}", h.finish())
}

/// A key as `key` makes them (so that a file name is all it can name).
fn is_key(k: &str) -> bool {
    k.len() == 16 && k.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `hub_bus3d`: the model of the look, or the start of its writing.
pub fn model(args: &Value) -> Answer {
    let (bus, paint) = (str_arg(args, "bus"), str_arg(args, "paint"));
    if bus.trim().is_empty() {
        return Err("No bus chosen".into());
    }
    let s = Settings::load();
    if s.openomsi_folder.trim().is_empty() || s.omsi2_folder.trim().is_empty() {
        return Ok(json!({ "state": "failed", "error": "Choose your openOMSI and OMSI 2 folders first" }));
    }
    let checked = match hub_game::check_folder(Path::new(&s.openomsi_folder), &hub_core::paths::home_dir()) {
        Ok(c) => c,
        Err(e) => return Ok(json!({ "state": "failed", "error": e.text() })),
    };
    let exe = checked.install.exe.clone();
    let vars = vars_for(&bus, &paint);
    let k = key(&bus, &paint, &vars, hub_core::paths::resolve_content(&bus).as_deref(), &exe);
    let file = dir().join(format!("{k}.glb"));
    if file.is_file() {
        return Ok(json!({ "state": "ready", "key": k }));
    }
    let mut m = making();
    let all = m.get_or_insert_with(HashMap::new);
    // (tried again: what failed is forgotten)
    if args.get("retry").and_then(Value::as_bool).unwrap_or(false) && matches!(all.get(&k), Some(Making::Failed(_))) {
        all.remove(&k);
    }
    match all.get(&k) {
        Some(Making::Busy) => return Ok(json!({ "state": "making" })),
        Some(Making::Failed(e)) => return Ok(json!({ "state": "failed", "error": e })),
        None => {}
    }
    all.insert(k.clone(), Making::Busy);
    drop(m);
    let spec = LaunchSpec {
        root: Some(PathBuf::from(&s.omsi2_folder)),
        bus: Some(bus.replace('\\', "/")),
        paint: Some(paint.clone()).filter(|p| !p.trim().is_empty()),
        set_vars: vars,
        export_glb: Some(file.clone()),
        ..LaunchSpec::default()
    };
    std::thread::spawn(move || {
        let _one = WRITING.lock().unwrap_or_else(|e| e.into_inner());
        let done = hub_game::process::export_glb(&exe, &spec, WAIT);
        if let Err(e) = &done {
            hub_core::log_line(&format!("bus3d: {bus} ({paint}) not written: {e}"));
        }
        let mut m = making();
        let all = m.get_or_insert_with(HashMap::new);
        match done {
            Ok(_) => {
                all.remove(&k);
            }
            Err(e) => {
                all.insert(k, Making::Failed(e));
            }
        }
    });
    Ok(json!({ "state": "making" }))
}

/// `hub_bus3d_glb`: the model's file.
pub fn glb(args: &Value) -> Result<Vec<u8>, String> {
    let k = str_arg(args, "key");
    if !is_key(&k) {
        return Err("no such model".into());
    }
    std::fs::read(dir().join(format!("{k}.glb"))).map_err(|_| "no such model".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_has_a_key_of_its_own() {
        let exe = Path::new("/nowhere/openomsi");
        let a = key("Vehicles/X/a.bus", "Red", &[], None, exe);
        assert!(is_key(&a));
        assert_eq!(a, key("vehicles\\x\\A.bus", "Red", &[], None, exe), "the bus in any case, either slash");
        assert_ne!(a, key("Vehicles/X/a.bus", "Blue", &[], None, exe));
        assert_ne!(a, key("Vehicles/X/a.bus", "Red", &[("vis_mirrors".into(), 1.0)], None, exe));
        assert_ne!(a, key("Vehicles/X/a.bus", "Red", &[], None, Path::new("/elsewhere/openomsi")));
        assert!(!is_key("../../etc") && !is_key("") && glb(&json!({ "key": "../x" })).is_err());
    }
}
