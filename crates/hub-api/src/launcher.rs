//! What the company's and RosterFlow's pages used of the old launcher around them (its
//! `omsi_launcher_lib`, which they called `core`), on Omsi-Hub's own parts: `hub_core`
//! (re-exported whole, so `core::list_lines`, `core::data_dir`, `core::busfonts` … are hub-core's)
//! and, for the drive, [`crate::drive`].
//!
//! * [`load_config`] - the OMSI 2 folder and the driver ([`driver`]: `driver` in the settings,
//!   else "Driver");
//! * [`list_maps`] - the installed maps (`global.cfg` read with `hub_omsi::map`);
//! * [`list_instances`], [`launch`], [`duty_args`] - the game Omsi-Hub started (one at a time);
//! * [`get_settings`] - the app's settings as JSON (`bus_lead` and the like);
//! * [`save_file_as`] - a file the player saves: into the data folder's `exports` (the app's
//!   own file dialog comes with the UI port);
//! * [`list_weather`] - the weather files of OMSI 2's `Weather` folder.

pub use hub_core::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The OMSI 2 folder and the driver, as the old launcher kept them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    /// The OMSI 2 folder ("" when it is not chosen).
    pub root: String,
    /// The driver's name: whose career and trips these are.
    pub profile: String,
}

/// The driver's name when none is set.
pub const DEFAULT_DRIVER: &str = "Driver";

/// The driver whose career, trips and company duties these are: `driver` in the settings
/// (taken over from the old launcher's profile by `migrate::import`), else "Driver".
pub fn driver() -> String {
    driver_of(&settings::Settings::load())
}

fn driver_of(s: &settings::Settings) -> String {
    s.other.get("driver").and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty()).unwrap_or(DEFAULT_DRIVER).to_string()
}

/// One read of the settings for both.
pub fn load_config() -> Config {
    let s = settings::Settings::load();
    Config { profile: driver_of(&s), root: s.omsi2_folder }
}

/// The OMSI 2 folder, when it is chosen and there.
pub fn root_dir() -> Option<PathBuf> {
    root()
}

/// The content folders (the content folder first), and the OMSI 2 folder: where the depot
/// files are read from.
pub fn depot_roots() -> (Vec<PathBuf>, Option<PathBuf>) {
    (paths::bases(), root())
}

/// A line into Omsi-Hub's log (the old launcher's `launcher.log`).
pub fn log_to_file(line: &str) {
    log_line(line);
}

/// The app's settings as the old launcher's settings JSON was read (`bus_lead` …): the
/// settings file's own fields and whatever else it keeps.
pub fn get_settings() -> Result<Value, String> {
    serde_json::to_value(settings::Settings::load()).map_err(|e| e.to_string())
}

/// The settings when there are none to read.
pub fn settings_from_text(_text: Option<&str>) -> Value {
    serde_json::to_value(settings::Settings::default()).unwrap_or(Value::Null)
}

// --- the maps ------------------------------------------------------------------------------------

/// An entry point of a map.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct EntryInfo {
    pub index: i32,
    pub name: String,
}

/// An installed map, as the company's chooser lists it (the old launcher's JSON).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct MapInfo {
    /// Its `[name]`, else its folder's name.
    pub name: String,
    pub friendly: String,
    /// Its `global.cfg`, relative to the OMSI 2 folder (`maps/<folder>/global.cfg`).
    pub file: String,
    pub description: String,
    pub entry_points: Vec<EntryInfo>,
    /// The depot file the map's own buses use (`[standarddepot]`).
    pub hof: String,
    /// It is in openOMSI's content folder (a mod).
    pub installed: bool,
    /// It drives on the left.
    pub left_hand: bool,
}

impl MapInfo {
    /// The map of `g`, read from `maps/<folder>/global.cfg`.
    pub fn of(g: &hub_omsi::map::GlobalCfg, folder: &str, installed: bool) -> MapInfo {
        let name = if g.name.trim().is_empty() { folder.to_string() } else { g.name.trim().to_string() };
        MapInfo {
            friendly: if g.friendly_name.trim().is_empty() { name.clone() } else { g.friendly_name.trim().to_string() },
            name,
            file: format!("maps/{folder}/global.cfg"),
            description: g.description.clone(),
            entry_points: g.entry_points.iter().map(|e| EntryInfo { index: e.index, name: e.name.clone() }).collect(),
            hof: g.standard_depot.clone().unwrap_or_default(),
            installed,
            left_hand: g.left_hand_traffic,
        }
    }
}

/// Every installed map with a `global.cfg`, sorted by name (none without the OMSI 2 folder);
/// its `hof` the depot file its `[standarddepot]` group gives (`content::maps_in`).
pub fn list_maps() -> Result<Vec<MapInfo>, String> {
    let Some(vfs) = paths::vfs() else { return Ok(Vec::new()) };
    Ok(crate::content::maps_in(&vfs))
}

/// A weather file of OMSI 2's `Weather` folder.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WeatherInfo {
    /// Relative to the OMSI 2 folder (`Weather/Sunny.owt`).
    pub file: String,
    pub name: String,
}

/// The weather files (by their file names: what the drive's weather list shows).
pub fn list_weather() -> Result<Vec<WeatherInfo>, String> {
    let Some(vfs) = paths::vfs() else { return Ok(Vec::new()) };
    Ok(vfs
        .list_dir("Weather")
        .into_iter()
        .filter(|(n, d)| !d && n.to_lowercase().ends_with(".owt"))
        .map(|(n, _)| WeatherInfo { file: format!("Weather/{n}"), name: n[..n.len() - 4].replace('_', " ") })
        .collect())
}

// --- the game ------------------------------------------------------------------------------------

/// A game Omsi-Hub started, as the company's pages look at it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Instance {
    pub pid: u32,
    pub running: bool,
    pub map: String,
    pub bus: String,
    pub line: Option<String>,
    pub tour: Option<String>,
    pub profile: String,
    /// When it was started (Unix time), and its command line (its `--time`: the game's clock).
    pub started: u64,
    pub args: Vec<String>,
}

/// The duty of the game the company started last (the drive itself knows only its process).
static STARTED: Mutex<Option<(hub_core::Duty, u64, Vec<String>)>> = Mutex::new(None);

/// The games running: the one Omsi-Hub started, if it runs.
pub fn list_instances() -> Vec<Instance> {
    let st = crate::drive::state();
    if st.get("running").and_then(Value::as_bool) != Some(true) {
        return Vec::new();
    }
    let pid = st.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32;
    let (d, started, args) = STARTED.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
    vec![Instance { pid, running: true, map: d.map, bus: d.bus, line: d.line, tour: d.tour, profile: d.profile.unwrap_or_else(driver), started, args }]
}

/// A game started.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Launched {
    pub pid: u32,
    /// Its command line, for the log.
    pub command: String,
}

/// Start the game with the duty `d` (`crate::drive::start`).
pub fn launch(d: &hub_core::Duty, p: &dyn crate::Platform) -> Result<Launched, String> {
    let args = duty_args(d).unwrap_or_default();
    let command = args.join(" ");
    let v = crate::drive::start(d, p)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |t| t.as_secs());
    *STARTED.lock().unwrap_or_else(|e| e.into_inner()) = Some((d.clone(), now, args));
    Ok(Launched { pid: v.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32, command })
}

/// The command line the duty `d` becomes (official flags only).
pub fn duty_args(d: &hub_core::Duty) -> Result<Vec<String>, String> {
    let v = crate::drive::args(d)?;
    serde_json::from_value(v.get("args").cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())
}

// --- files ---------------------------------------------------------------------------------------

/// The folder the player's saved files go to: the data folder's `exports`.
pub fn exports_dir() -> PathBuf {
    data_dir().join("exports")
}

/// Where a file the player saves goes (`name`, kept apart from what is there: `name (2)` …).
/// The app's own file dialog comes with the UI port; until then the data folder's `exports`.
pub fn save_file_as(_title: &str, name: &str, _kind: &str, _exts: &[&str]) -> Option<PathBuf> {
    let dir = exports_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let clean: String = name.chars().map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')') { c } else { '_' }).collect();
    let clean = if clean.trim().is_empty() { "file".to_string() } else { clean.trim().to_string() };
    let first = dir.join(&clean);
    if !first.exists() {
        return Some(first);
    }
    let (stem, ext) = match clean.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (clean.clone(), String::new()),
    };
    (2..1000).map(|k| dir.join(format!("{stem} ({k}){ext}"))).find(|p| !p.exists())
}

/// The texts in a language (`hub-locales`): English text → its translation.
pub mod i18n {
    pub fn table(lang: &str) -> std::collections::HashMap<String, String> {
        hub_locales::texts(lang).into_iter().collect()
    }
}

/// When a file was last changed, in nanoseconds since 1970 (0: not known).
pub mod index {
    pub fn mtime_ns(path: &std::path::Path) -> u64 {
        std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as u64)
    }
}

/// Bytes from base64 (the page's pictures; a `data:` prefix is passed over).
pub fn base64(text: &str) -> Result<Vec<u8>, String> {
    let text = text.split_once(',').filter(|(head, _)| head.starts_with("data:")).map_or(text, |(_, b)| b);
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return Err(format!("not base64: {}", c as char)),
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Bytes as base64 (a PDF for the page).
pub fn base64_encode(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A path relative to `base`, found in any case.
pub fn resolve_path(base: &Path, rel: &str) -> PathBuf {
    paths::resolve_path(base, rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_reads_the_page_s_pictures() {
        assert_eq!(base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64(&base64_encode(&[0, 255, 7, 9])).unwrap(), [0, 255, 7, 9]);
        assert_eq!(base64("data:image/png;base64,aGk=").unwrap(), b"hi");
        assert!(base64("a*b").is_err());
    }

    #[test]
    fn a_map_is_listed_by_its_names() {
        let g = hub_omsi::map::GlobalCfg { name: "Grundorf".into(), left_hand_traffic: true, standard_depot: Some("Grundorf.hof".into()), ..Default::default() };
        let m = MapInfo::of(&g, "Grundorf", false);
        assert_eq!((m.file.as_str(), m.friendly.as_str(), m.hof.as_str(), m.left_hand), ("maps/Grundorf/global.cfg", "Grundorf", "Grundorf.hof", true));
    }

    #[test]
    fn nothing_runs_until_a_game_is_started() {
        assert!(list_instances().is_empty());
    }
}
