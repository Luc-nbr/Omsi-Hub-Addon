//! The devices of the bus the player drives - its IBIS, its ticket printer or ALMEX, any other
//! screen with keys or touch fields - as the app draws them (docs/COMPANION.md, "The bus's
//! devices, drawn as the game draws them"). `hub-devices` reads them from the bus's own files;
//! this puts them together for the state (`devices`), keeps their forms and pictures by id,
//! and makes each screen's stand from what the plugin reads in the game ten times a second.
//!
//! A form is built once per bus and device: first the number variables its place and its
//! pages hang on are asked of the plugin ([`form::needed_numbers`]), at most
//! [`WAIT_FOR_NUMBERS`] is waited for them, then it is built - off the request threads, and
//! kept on disk keyed by the model's files and those values, so that the next drive of the
//! same bus has it at once.

use hub_devices::fonts::FontBook;
use hub_devices::form::{self, Built, Input, Models};
use hub_devices::modules::{self, Key, Module};
use hub_devices::protocol::{Form, Stand};
use hub_devices::textures::{self, Finder, Flags};
use hub_devices::{cfg, layout, scripts};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// How long a build waits for the plugin's numbers before it goes on with what it has.
pub const WAIT_FOR_NUMBERS: Duration = Duration::from_secs(3);
/// How long a screen's stand waits for news.
pub const SCREEN_WAIT: Duration = Duration::from_secs(2);
/// The folder of the kept forms, beside the paired devices' file; and how many are kept.
pub const CACHE_DIR: &str = "companion-forms";
const CACHE_KEPT: usize = 40;
/// Up when what is built changes, so that an older add-on's forms are not taken.
const CACHE_FORMAT: &str = "devices-1";
/// Devices of one bus at most, and keys of one device.
const MAX_DEVICES: usize = 24;
const MAX_KEYS: usize = 160;

/// A key of a device off its screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceKey {
    pub trigger: String,
    pub label: String,
    pub digit: Option<u8>,
    /// Its row on the device, from the top (the rows as the keys lie on the device).
    pub row: usize,
    /// Of a touch device: a key that sits apart from it (the ALMEX's change, its latch).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub loose: bool,
}

/// A device as `state.devices` lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// "ibis" | "printer" | "other"
    pub role: String,
    /// "touch" | "screen_keys" | "keys" | "screen"
    pub kind: String,
    pub form: Option<String>,
    pub keys: Vec<DeviceKey>,
}

/// What was made of one bus.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Made {
    pub devices: Vec<Device>,
    pub forms: Vec<Form>,
    /// (id, path, flags) of every picture of the forms.
    pub textures: Vec<(String, PathBuf, Flags)>,
}

impl Made {
    /// The triggers the app may press: the taps and the keys of these devices, nothing else.
    pub fn allowed(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let all = self.forms.iter().flat_map(|f| f.taps.iter().map(|t| &t.trigger)).chain(self.devices.iter().flat_map(|d| d.keys.iter().map(|k| &k.trigger)));
        for t in all {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
        out
    }

    /// What the plugin is to read for the stands: `{nums, strs}`.
    pub fn want(&self) -> Value {
        let mut nums: Vec<&String> = Vec::new();
        let mut strs: Vec<&String> = Vec::new();
        for f in &self.forms {
            for n in &f.nums {
                if !nums.contains(&n) {
                    nums.push(n);
                }
            }
            for s in &f.strs {
                if !strs.contains(&s) {
                    strs.push(s);
                }
            }
        }
        json!({ "nums": nums, "strs": strs })
    }
}

/// The bus's devices as the server keeps them.
#[derive(Default)]
pub struct Board {
    /// The bus's file as the plugin names it ("": on foot).
    pub bus: String,
    /// One up per bus: a build of an older bus is dropped.
    pub generation: u64,
    pub building: bool,
    pub model_cfg: Option<PathBuf>,
    pub omsi_dir: Option<PathBuf>,
    pub made: Made,
    pub forms: HashMap<String, Arc<Form>>,
    /// Every picture the app may ask for, by id: the forms' and the freetex values met since.
    pub textures: HashMap<String, (PathBuf, Flags)>,
    pub pngs: HashMap<String, Arc<Vec<u8>>>,
}

impl Board {
    /// What was made, taken.
    pub fn take(&mut self, made: Made) {
        self.forms = made.forms.iter().map(|f| (f.id.clone(), Arc::new(f.clone()))).collect();
        self.textures = made.textures.iter().map(|(id, p, f)| (id.clone(), (p.clone(), f.clone()))).collect();
        self.pngs.clear();
        self.made = made;
        self.building = false;
    }

    /// The state's `devices`.
    pub fn json(&self) -> Value {
        serde_json::to_value(&self.made.devices).unwrap_or_else(|_| json!([]))
    }

    /// A picture's PNG kept, or `None` (not registered, or not readable).
    pub fn picture(&self, id: &str) -> Option<(PathBuf, Flags)> {
        self.textures.get(id).cloned()
    }

    /// A PNG made, kept (a few dozen).
    pub fn keep_png(&mut self, id: &str, png: Arc<Vec<u8>>) {
        if self.pngs.len() >= 48 {
            self.pngs.clear();
        }
        self.pngs.insert(id.to_string(), png);
    }
}

/// What the plugin read for the screens (its `scr` datagrams' items: `["n", name, value]`,
/// `["o", name]` for a number the bus lacks, `["s", name, text]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Screen {
    pub nums: HashMap<String, Option<f64>>,
    /// Lower case.
    pub unknown: HashSet<String>,
    pub strs: HashMap<String, String>,
}

impl Screen {
    pub fn of(items: &[Value]) -> Screen {
        let mut s = Screen::default();
        for it in items {
            let Some(a) = it.as_array() else { continue };
            let name = a.get(1).and_then(Value::as_str).unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            match a.first().and_then(Value::as_str) {
                Some("n") => {
                    s.nums.insert(name, a.get(2).and_then(Value::as_f64).filter(|v| v.is_finite()));
                }
                Some("o") => {
                    s.unknown.insert(name.to_lowercase());
                }
                Some("s") => {
                    s.strs.insert(name, a.get(2).and_then(Value::as_str).unwrap_or("").to_string());
                }
                _ => {}
            }
        }
        s
    }

    /// Whether the plugin answered for this number (with a value or "the bus lacks it").
    pub fn answered(&self, name: &str) -> bool {
        self.nums.contains_key(name) || self.unknown.contains(&name.to_lowercase())
    }

    /// The numbers that are numbers, for the build.
    pub fn numbers(&self) -> HashMap<String, f64> {
        self.nums.iter().filter_map(|(k, v)| v.map(|v| (k.clone(), v))).collect()
    }
}

/// A screen's stand now. `freetex(name, alpha)`: a picture's id for a `[matl_freetex]` value.
pub fn stand(form: &Form, screen: &Screen, v: u64, mut freetex: impl FnMut(&str, u8) -> Option<String>) -> Stand {
    // (as the bus has it: the game trims nothing, a line such as '    LEE' stands so)
    let t: Vec<String> = form.strs.iter().map(|s| screen.strs.get(s).cloned().unwrap_or_default()).collect();
    let mut g = Vec::with_capacity(form.nums.len());
    let mut o = Vec::new();
    for (i, n) in form.nums.iter().enumerate() {
        if screen.unknown.contains(&n.to_lowercase()) {
            o.push(i);
            g.push(Some(0.0));
        } else {
            g.push(screen.nums.get(n).copied().flatten());
        }
    }
    let mut f = None;
    if form.parts.iter().any(|p| p.freetex.is_some()) {
        let mut ids: Vec<Option<String>> = vec![None; form.strs.len()];
        for p in &form.parts {
            let Some(i) = p.freetex else { continue };
            if ids.get(i).is_none_or(|x| x.is_some()) {
                continue;
            }
            let name = t[i].trim();
            if !name.is_empty() {
                ids[i] = freetex(name, p.alpha);
            }
        }
        f = Some(ids);
    }
    Stand { v, form: form.id.clone(), t, g, o, f }
}

/// A device of the bus, as the state lists it; `None` when there is nothing to show of it.
pub fn device_of(m: &Module, built: Option<&Built>) -> Option<Device> {
    let on_screen: HashSet<String> = built.map(|b| b.taps_on_screen.iter().map(|t| t.to_lowercase()).collect()).unwrap_or_default();
    let touch = built.is_some_and(|b| !b.form.taps.is_empty());
    let on_device = built.map(Built::keys_on_device).unwrap_or_default();
    let rest: Vec<&Key> = m.keys.iter().filter(|k| !on_screen.contains(&k.action.to_lowercase())).take(MAX_KEYS).collect();
    // A touch screen is that screen: what keys are left and do not sit on the device (the flap,
    // the latch, the change of an ALMEX) come after its own (the keypad of an RG box).
    let (near, far): (Vec<&Key>, Vec<&Key>) = if touch { rest.iter().partition(|k| on_device.contains(&k.action.to_lowercase())) } else { (rest.clone(), Vec::new()) };
    let mut keys = Vec::new();
    let mut row = 0;
    for (group, loose) in [(near, false), (far, true)] {
        let rows = layout::rows(&group, |k| k.at);
        for r in rows {
            for i in r {
                let k = group[i];
                keys.push(DeviceKey { trigger: k.action.clone(), label: k.label.clone(), digit: k.digit, row, loose });
            }
            row += 1;
        }
    }
    let kind = match (built, touch, keys.is_empty()) {
        (Some(_), true, _) => "touch",
        (Some(_), false, false) => "screen_keys",
        (Some(_), false, true) => "screen",
        (None, _, false) => "keys",
        (None, _, true) => return None,
    };
    Some(Device { id: m.id.clone(), name: m.name.clone(), role: m.role().name().into(), kind: kind.into(), form: built.map(|b| b.form.id.clone()), keys })
}

/// The bus's devices (at most [`MAX_DEVICES`]).
pub fn modules_of(model_cfg: &Path) -> Vec<Module> {
    let mut m = modules::modules_of(model_cfg);
    m.truncate(MAX_DEVICES);
    m
}

/// The number variables all devices of the bus need for their forms.
pub fn needed(models: &Models, model_cfg: &Path, modules: &[Module]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in modules {
        for n in form::needed_numbers(models, model_cfg, m) {
            if !out.iter().any(|x| x.eq_ignore_ascii_case(&n)) {
                out.push(n);
            }
        }
    }
    out
}

fn stamp(p: &Path) -> String {
    match std::fs::metadata(p) {
        Ok(m) => format!("{}|{}|{:?}", scripts_norm(p), m.len(), m.modified().ok()),
        Err(_) => format!("{}|-", scripts_norm(p)),
    }
}

fn scripts_norm(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").to_lowercase()
}

/// The key a bus's forms are kept under: its model's files (the cfg, every mesh, the scripts)
/// with their sizes and times, the values the forms were built with, and the fonts' folder.
pub fn cache_key(model_cfg: &Path, omsi_dir: &Path, needed: &[String], screen: &Screen) -> Option<String> {
    let c = cfg::read(model_cfg)?;
    let mut text = format!("{CACHE_FORMAT}\n{}\n{}\n", stamp(model_cfg), scripts_norm(omsi_dir));
    for m in c.meshes.iter().filter(|m| m.exists) {
        text.push_str(&stamp(&m.file));
        text.push('\n');
    }
    for s in scripts::scripts_of(model_cfg) {
        text.push_str(&stamp(&s));
        text.push('\n');
    }
    if let Ok(rd) = std::fs::read_dir(omsi_dir.join("Fonts")) {
        let mut fonts: Vec<String> = rd.flatten().map(|e| stamp(&e.path())).collect();
        fonts.sort();
        text.push_str(&fonts.join("\n"));
    }
    let mut names: Vec<&String> = needed.iter().collect();
    names.sort();
    for n in names {
        let v = if screen.unknown.contains(&n.to_lowercase()) { "unknown".to_string() } else { format!("{:?}", screen.nums.get(n.as_str()).copied().flatten()) };
        text.push_str(&format!("\n{n}={v}"));
    }
    Some(textures::hex20(text.as_bytes()))
}

/// The bus's devices made, or taken from the disk when this bus was made with these values
/// before (and its pictures are still the same files).
#[allow(clippy::too_many_arguments)]
pub fn make(models: &Models, model_cfg: &Path, omsi_dir: &Path, modules: &[Module], needed: &[String], screen: &Screen, finder: &Finder, cache_dir: Option<&Path>) -> Made {
    let key = cache_key(model_cfg, omsi_dir, needed, screen);
    let file = cache_dir.zip(key.as_ref()).map(|(d, k)| d.join(format!("{k}.json")));
    if let Some(f) = &file {
        if let Some(made) = std::fs::read(f).ok().and_then(|b| serde_json::from_slice::<Made>(&b).ok()) {
            if made.textures.iter().all(|(id, p, fl)| textures::id_of(p, fl) == *id) {
                return made;
            }
        }
    }
    let book = FontBook::default();
    let fonts = std::cell::RefCell::new(HashMap::new());
    let font = |name: &str, full: bool| book.font(omsi_dir, name, full, &mut fonts.borrow_mut());
    let find = |name: &str| finder.find(model_cfg, omsi_dir, name);
    let numbers = screen.numbers();
    let unknown: Vec<String> = needed.iter().filter(|n| screen.unknown.contains(&n.to_lowercase())).cloned().collect();
    let mut made = Made::default();
    for m in modules {
        let input = Input { model_cfg, omsi_dir, module: m, numbers: &numbers, unknown: &unknown, font: &font, find_texture: &find };
        let built = form::build(models, &input);
        let Some(d) = device_of(m, built.as_ref()) else { continue };
        if let Some(b) = built {
            for t in b.textures {
                if !made.textures.iter().any(|x| x.0 == t.0) {
                    made.textures.push(t);
                }
            }
            if !made.forms.iter().any(|f| f.id == b.form.id) {
                made.forms.push(b.form);
            }
        }
        made.devices.push(d);
    }
    if let (Some(f), Some(dir)) = (&file, cache_dir) {
        if std::fs::create_dir_all(dir).is_ok() {
            let tmp = f.with_extension("new");
            if std::fs::write(&tmp, serde_json::to_vec(&made).unwrap_or_default()).is_ok() {
                let _ = std::fs::rename(&tmp, f);
            }
            prune(dir);
        }
    }
    made
}

/// The oldest kept forms away, beyond [`CACHE_KEPT`].
fn prune(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> =
        rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "json")).filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
    if files.len() <= CACHE_KEPT {
        return;
    }
    files.sort();
    for (_, p) in &files[..files.len() - CACHE_KEPT] {
        let _ = std::fs::remove_file(p);
    }
}

/// The `model.cfg` a `.bus` file names.
pub fn model_of_bus(bus_file: &Path) -> Option<PathBuf> {
    scripts::model_of_bus(bus_file)
}

/// Whether an id is one of ours: 20 hex characters (a form's or a picture's).
pub fn id_ok(id: &str) -> bool {
    id.len() == 20 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_devices::testbus;

    fn screen(items: Value) -> Screen {
        Screen::of(items.as_array().unwrap())
    }

    #[test]
    fn the_test_bus_makes_a_touch_ibis_and_a_printer_of_keys_and_keeps_them() {
        let root = std::env::temp_dir().join(format!("hub-companion-made-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cfg = testbus::write(&root).unwrap();
        let models = Models::default();
        let mods = modules_of(&cfg);
        let need = needed(&models, &cfg, &mods);
        assert!(need.contains(&"IBIS_mode".to_string()) && need.contains(&"trans_dauer".to_string()), "{need:?}");
        let scr = screen(json!([["n", "IBIS_mode", 1], ["n", "IBIS_stop", 0], ["n", "trans_dauer", 1]]));
        let finder = Finder::default();
        let cache = root.join("cache");
        let made = make(&models, &cfg, &root, &mods, &need, &scr, &finder, Some(&cache));
        let ibis = made.devices.iter().find(|d| d.role == "ibis").unwrap();
        let printer = made.devices.iter().find(|d| d.role == "printer").unwrap();
        assert_eq!((ibis.kind.as_str(), printer.kind.as_str()), ("touch", "keys"));
        assert!(ibis.form.is_some() && printer.form.is_none());
        // the keypad, off the screen, in its two rows; the touch fields are no keys
        assert_eq!(ibis.keys.len(), 12);
        assert_eq!(ibis.keys.iter().map(|k| k.row).max(), Some(1));
        assert_eq!(ibis.keys[0].label, "1");
        assert!(ibis.keys.iter().all(|k| !k.loose));
        assert_eq!(printer.keys.len(), 6);
        // the whitelist: the taps and the keys of these devices
        let allowed = made.allowed();
        assert!(allowed.contains(&"IBIS_menu".to_string()) && allowed.contains(&"IBIS_7".to_string()) && allowed.contains(&"ticketprinter_button_ticket_6".to_string()));
        assert_eq!(allowed.len(), 2 + 12 + 6);
        assert_eq!(made.want()["strs"], json!(["IBIS_line_text", "IBIS_dest_text"]));
        // kept on disk, and taken from there the next time
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
        let again = make(&Models::default(), &cfg, &root, &mods, &need, &scr, &finder, Some(&cache));
        assert_eq!(again, made);
        // other values, another form
        let scr0 = screen(json!([["n", "IBIS_mode", 0], ["n", "IBIS_stop", 0], ["o", "trans_dauer"]]));
        let other = make(&Models::default(), &cfg, &root, &mods, &need, &scr0, &finder, Some(&cache));
        assert_ne!(other.forms[0].id, made.forms[0].id);
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 2);

        // the stand: the texts as they are, the numbers, the ones the bus lacks
        let form = &made.forms[0];
        let s = stand(form, &screen(json!([["s", "IBIS_line_text", "  29/03"], ["n", "IBIS_mode", 1], ["o", "IBIS_stop"]])), 7, |_, _| None);
        assert_eq!(s.t, ["  29/03", ""]);
        let mode = form.nums.iter().position(|n| n == "IBIS_mode").unwrap();
        let stop = form.nums.iter().position(|n| n == "IBIS_stop").unwrap();
        assert_eq!(s.g[mode], Some(1.0));
        assert_eq!(s.o, [stop]);
        assert_eq!((s.v, s.form.as_str()), (7, form.id.as_str()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ids_are_twenty_hex_characters() {
        assert!(id_ok("0123456789abcdef0123"));
        assert!(!id_ok("0123456789ABCDEF0123") && !id_ok("../../etc/passwd") && !id_ok("0123"));
    }
}
