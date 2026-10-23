//! The player's game controllers - steering wheels, pedals, joysticks, shifters and button
//! boxes - as openOMSI (and OMSI 2 before it) keeps them in `Inputs/gamectrler.cfg`, changed
//! from Omsi-Hub's Settings page (the tab "Controllers").
//!
//! The file is a `[ctrl]` block per device: its name (DirectInput's), a second line kept as
//! it is, then
//!
//! * `[axis]`: per DirectInput axis (X, Y, Z, X rotation, Y rotation, Z rotation, slider 1,
//!   slider 2) its function (-1 none, 0 steering, 1 throttle, 2 brake, 3 clutch, 4 throttle and
//!   brake on one axis, and openOMSI's own 5 and 6: looking left and right, up and down) and its
//!   flags (bit 0: it runs the other way; 2: a longer travel; 4, 8 and 0x10: the response curve);
//! * `[buttons]`: per button (from 0; the hats' directions from 128, four per hat) the key
//!   action it presses (`Inputs/keyboard.cfg`'s names) and `1` when the action is held for as
//!   long as the button is (OMSI's continuous flag);
//! * `[FFScale]`: the force feedback's steering forces, then its vibration (1: as openOMSI
//!   makes them);
//! * openOMSI's own: `[openOMSI.FFInvert]` (the wheel's motor the other way, 0 or 1),
//!   `[openOMSI.Latching]` (the buttons, from 1, that are latching switches) and
//!   `[openOMSI.MoreActions]` (more actions on one button: the button, from 1, and the action).
//!
//! **Which file**: openOMSI reads the newer of `~/.openomsi/Inputs/gamectrler.cfg` (where its
//! launcher writes) and the content folder's `Inputs/gamectrler.cfg`, and OMSI 2's own when
//! there is neither ([`Places::in_use`]). Omsi-Hub writes `~/.openomsi/Inputs` - and the content
//! folder's file as well when there is one, for an openOMSI that reads only that - as
//! openOMSI's launcher writes it (CR LF, the blocks in the same order), keeping a copy of the
//! file as it was before Omsi-Hub first changed it beside it. OMSI 2's own file is only read.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file's name in an `Inputs` folder.
pub const FILE: &str = "gamectrler.cfg";
/// The first button number of the hats' directions (up, right, down, left per hat).
pub const HAT_BUTTONS: usize = 128;
/// The functions an axis can have (the file's numbers; -1 none).
pub const FUNCTIONS: std::ops::RangeInclusive<i32> = -1..=6;
/// The most buttons a device keeps (with the hats' and room to spare, as openOMSI reads them).
pub const MAX_BUTTONS: usize = 512;

/// One device of the file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Device {
    pub name: String,
    /// The line after the name, as the file has it.
    pub second: String,
    /// Per DirectInput axis: its function (-1 none) and its flags.
    pub axes: [[i32; 2]; 8],
    /// Per button: the action (empty: none) and the number after it (`1`: continuous).
    pub buttons: Vec<(String, String)>,
    /// `[FFScale]`: steering forces, vibration (None: 1 and 1).
    pub ff_scale: Option<(f32, f32)>,
    /// `[openOMSI.FFInvert]` (None: the setting `ff_invert` holds).
    pub ff_invert: Option<bool>,
    /// `[openOMSI.Latching]`: buttons from 0.
    pub latching: Vec<usize>,
    /// `[openOMSI.MoreActions]`: (button from 0, action).
    pub more: Vec<(usize, String)>,
}

impl Device {
    /// A device the file does not have yet: nothing set up.
    pub fn new(name: &str) -> Device {
        Device { name: name.to_string(), second: "0".into(), axes: [[-1, 0]; 8], ..Default::default() }
    }
}

/// The devices of the file's text.
pub fn parse(text: &str) -> Vec<Device> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let line = |k: usize| lines.get(k).copied().unwrap_or("");
    let number = |k: usize, or: i32| line(k).parse::<i32>().unwrap_or(or);
    let mut out: Vec<Device> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let here = lines[i];
        if here == "[ctrl]" {
            out.push(Device { name: line(i + 1).to_string(), second: lines.get(i + 2).unwrap_or(&"0").to_string(), axes: [[-1, 0]; 8], ..Default::default() });
            i += 3;
            continue;
        }
        // (a block before the first `[ctrl]` belongs to no device: skipped)
        match (here, out.last_mut()) {
            ("[axis]", Some(d)) => {
                for a in 0..8 {
                    d.axes[a] = [number(i + 1 + a * 2, -1), number(i + 2 + a * 2, 0)];
                }
                i += 17;
            }
            ("[buttons]", Some(d)) => {
                let n = line(i + 1).parse::<usize>().unwrap_or(0).min(MAX_BUTTONS);
                d.buttons = (0..n).map(|b| (line(i + 2 + b * 2).to_string(), lines.get(i + 3 + b * 2).unwrap_or(&"0").to_string())).collect();
                i += 2 + n * 2;
            }
            ("[FFScale]" | "[ffscale]", Some(d)) => {
                let f = |k: usize| line(i + k).replace(',', ".").parse::<f32>().ok().filter(|v| v.is_finite()).unwrap_or(1.0);
                d.ff_scale = Some((f(1), f(2)));
                i += 3;
            }
            ("[openOMSI.FFInvert]", Some(d)) => {
                d.ff_invert = match line(i + 1) {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => None,
                };
                i += 2;
            }
            ("[openOMSI.Latching]", Some(d)) => {
                d.latching = line(i + 1).split_whitespace().filter_map(|v| v.parse::<usize>().ok()?.checked_sub(1)).collect();
                i += 2;
            }
            ("[openOMSI.MoreActions]", Some(d)) => {
                let n = line(i + 1).parse::<usize>().unwrap_or(0).min(4096);
                for k in 0..n {
                    let button = line(i + 2 + k * 2).parse::<usize>().ok().and_then(|b| b.checked_sub(1));
                    let action = line(i + 3 + k * 2);
                    if let Some(b) = button.filter(|_| !action.is_empty()) {
                        d.more.push((b, action.to_string()));
                    }
                }
                i += 2 + n * 2;
            }
            _ => i += 1,
        }
    }
    out
}

/// The file's text for `devices`, as openOMSI's launcher writes it (CR LF).
pub fn text(devices: &[Device]) -> String {
    let mut t = String::new();
    for d in devices {
        t.push_str(&format!("\r\n[ctrl]\r\n{}\r\n{}\r\n\r\n[axis]\r\n", d.name, if d.second.is_empty() { "0" } else { &d.second }));
        for [f, flags] in d.axes {
            t.push_str(&format!("{f}\r\n{flags}\r\n"));
        }
        t.push_str(&format!("\r\n[buttons]\r\n{}\r\n", d.buttons.len()));
        for (action, n) in &d.buttons {
            t.push_str(&format!("{action}\r\n{}\r\n", if n.is_empty() { "0" } else { n }));
        }
        let (a, b) = d.ff_scale.unwrap_or((1.0, 1.0));
        t.push_str(&format!("\r\n[FFScale]\r\n{a:.3}\r\n{b:.3}\r\n\r\n"));
        if let Some(invert) = d.ff_invert {
            t.push_str(&format!("[openOMSI.FFInvert]\r\n{}\r\n\r\n", invert as u8));
        }
        if !d.latching.is_empty() {
            let numbers: Vec<String> = d.latching.iter().map(|b| (b + 1).to_string()).collect();
            t.push_str(&format!("[openOMSI.Latching]\r\n{}\r\n\r\n", numbers.join(" ")));
        }
        if !d.more.is_empty() {
            t.push_str(&format!("[openOMSI.MoreActions]\r\n{}\r\n", d.more.len()));
            for (b, action) in &d.more {
                t.push_str(&format!("{}\r\n{action}\r\n", b + 1));
            }
            t.push_str("\r\n");
        }
    }
    t
}

/// Why devices are not written: a name or an action the file cannot hold (one line each), a
/// function or a number out of its range.
pub fn check(devices: &[Device]) -> Result<(), String> {
    let line = |s: &str| !s.trim().is_empty() && s.chars().count() <= 200 && !s.chars().any(char::is_control) && s == s.trim();
    for d in devices {
        if !line(&d.name) {
            return Err(format!("{:?} is no device name the file can hold", d.name));
        }
        if d.second.chars().any(char::is_control) {
            return Err(format!("{}: its second line is not one line", d.name));
        }
        if let Some([f, _]) = d.axes.iter().find(|[f, _]| !FUNCTIONS.contains(f)) {
            return Err(format!("{}: {f} is no function of an axis", d.name));
        }
        if d.buttons.len() > MAX_BUTTONS {
            return Err(format!("{}: more than {MAX_BUTTONS} buttons", d.name));
        }
        let action_ok = |a: &str| a.is_empty() || line(a);
        if let Some((a, _)) = d.buttons.iter().find(|(a, n)| !action_ok(a) || !matches!(n.as_str(), "" | "0" | "1")) {
            return Err(format!("{}: {a:?} is no action of a button", d.name));
        }
        if let Some((_, a)) = d.more.iter().find(|(b, a)| *b >= MAX_BUTTONS || !line(a)) {
            return Err(format!("{}: {a:?} is no action of a button", d.name));
        }
        if d.latching.iter().any(|b| *b >= MAX_BUTTONS) {
            return Err(format!("{}: a latching button past the last", d.name));
        }
        if let Some((a, b)) = d.ff_scale {
            if !(0.0..=4.0).contains(&a) || !(0.0..=4.0).contains(&b) {
                return Err(format!("{}: the force feedback's strength is out of its range", d.name));
            }
        }
    }
    Ok(())
}

/// Where the files of the `Inputs` folder are: `~/.openomsi/Inputs` (openOMSI's own), the
/// content folder's and OMSI 2's (the game folders; their `Inputs` found in any case when it
/// is asked for).
#[derive(Debug, Clone)]
pub struct Places {
    pub own: PathBuf,
    pub content: Option<PathBuf>,
    pub omsi2: Option<PathBuf>,
}

impl Places {
    /// For the home folder `home`, openOMSI's content folder and the OMSI 2 folder (either may
    /// be unknown).
    pub fn new(home: &Path, content: Option<&Path>, omsi2: Option<&Path>) -> Places {
        Places { own: home.join(".openomsi").join("Inputs"), content: content.map(Path::to_path_buf), omsi2: omsi2.map(Path::to_path_buf) }
    }

    /// The file `name` of the `Inputs` folder openOMSI reads: the newer of its own and the
    /// content folder's, else OMSI 2's (None: none of them is there).
    pub fn in_use(&self, name: &str) -> Option<PathBuf> {
        let own = file_in(&self.own, name);
        let content = self.content.as_deref().and_then(|d| file_in(&inputs_of(d), name));
        let newer = match (own, content) {
            (Some(a), Some(b)) => Some(if modified(&b) > modified(&a) { b } else { a }),
            (a, b) => a.or(b),
        };
        newer.or_else(|| self.omsi2.as_deref().and_then(|d| file_in(&inputs_of(d), name)))
    }

    /// Where `name` is written: openOMSI's own folder, and the content folder's file too when
    /// it is there.
    pub fn targets(&self, name: &str) -> Vec<PathBuf> {
        let mut out = vec![file_in(&self.own, name).unwrap_or_else(|| self.own.join(name))];
        out.extend(self.content.as_deref().and_then(|d| file_in(&inputs_of(d), name)));
        out
    }
}

/// The `Inputs` folder of a game folder, in whatever case it has.
fn inputs_of(dir: &Path) -> PathBuf {
    hub_omsi::vfs::find_child(dir, "Inputs").unwrap_or_else(|| dir.join("Inputs"))
}

/// The file `name` in `dir`, in any case, when it is there.
fn file_in(dir: &Path, name: &str) -> Option<PathBuf> {
    hub_omsi::vfs::find_child(dir, name).filter(|p| p.is_file())
}

fn modified(p: &Path) -> std::time::SystemTime {
    std::fs::metadata(p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH)
}

/// The text of a file of OMSI's, in the codepage it is in.
fn read_text(p: &Path) -> std::io::Result<String> {
    std::fs::read(p).map(|b| hub_omsi::text::decode_text(&b))
}

/// The devices of the file openOMSI reads, and that file (None: there is none yet).
pub fn load(places: &Places) -> Result<(Vec<Device>, Option<PathBuf>), String> {
    let Some(p) = places.in_use(FILE) else { return Ok((Vec::new(), None)) };
    let text = read_text(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut devices = parse(&text);
    // (OMSI 2's own file can say 0 and 0 for a wheel's force feedback: openOMSI takes its
    // own 1 and 1 until its file is written, and so does this)
    if places.omsi2.as_deref().is_some_and(|o| p.starts_with(o)) {
        for d in &mut devices {
            if d.ff_scale == Some((0.0, 0.0)) {
                d.ff_scale = None;
            }
        }
    }
    Ok((devices, Some(p)))
}

/// `devices` written where openOMSI reads them ([`Places::targets`]); the file before Omsi-Hub
/// first wrote it is kept beside it (`gamectrler.cfg.before-omsihub`). Returns the files
/// written, openOMSI's own first.
pub fn save(places: &Places, devices: &[Device]) -> Result<Vec<PathBuf>, String> {
    check(devices)?;
    let text = text(devices);
    let mut written = Vec::new();
    for p in places.targets(FILE) {
        let old = std::fs::read(&p).ok();
        if old.as_deref() == Some(text.as_bytes()) {
            written.push(p);
            continue;
        }
        let backup = p.with_extension("cfg.before-omsihub");
        if let Some(old) = old.as_ref().filter(|_| !backup.exists()) {
            let _ = std::fs::write(&backup, old);
        }
        write(&p, &text).map_err(|e| format!("{}: {e}", p.display()))?;
        written.push(p);
    }
    Ok(written)
}

/// `text` written to `path` through a file beside it (a write cut off leaves the old file).
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    // (a name of its own per write: two at once never write into one part file)
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let part = path.with_extension(format!("cfg.omsihub-part-{}-{n}", std::process::id()));
    std::fs::write(&part, text)?;
    std::fs::rename(&part, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&part);
    })
}

/// The key actions of `Inputs/keyboard.cfg` (the one openOMSI reads), the game's and the
/// vehicles', in the file's order and each once: what a button can press.
pub fn key_actions(places: &Places) -> Vec<String> {
    let Some(text) = places.in_use("keyboard.cfg").and_then(|p| read_text(&p).ok()) else { return Vec::new() };
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out: Vec<String> = Vec::new();
    for (k, l) in lines.iter().enumerate() {
        if l.eq_ignore_ascii_case("[entry]") {
            if let Some(a) = lines.get(k + 1).filter(|a| !a.is_empty() && !a.starts_with('[')) {
                if !out.iter().any(|o| o.eq_ignore_ascii_case(a)) {
                    out.push(a.to_string());
                }
            }
        }
    }
    out
}

/// OMSI's language of its key names for Omsi-Hub's language `lang`: German and French have
/// their own, every other language reads the English ones.
pub fn names_language(lang: &str) -> &'static str {
    match lang.get(..2).unwrap_or("").to_ascii_lowercase().as_str() {
        "de" => "DEU",
        "fr" => "FRA",
        _ => "ENG",
    }
}

/// The names OMSI 2's key assignment gives the actions, in the language `omsi_lang` (`ENG`,
/// `DEU`, `FRA`): `Languages/<lang>_key_*.olf` of the OMSI 2 folder, `KY_<action>` and its
/// text a line; the actions in lower case.
pub fn action_names(omsi2: &Path, omsi_lang: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(dir) = hub_omsi::vfs::find_child(omsi2, "Languages") else { return out };
    let prefix = format!("{}_key_", omsi_lang.to_ascii_lowercase());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).map(|it| it.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    files.retain(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".olf")));
    files.sort();
    for f in files {
        let Ok(text) = read_text(&f) else { continue };
        for l in text.lines() {
            let Some((k, v)) = l.split_once('\t') else { continue };
            let Some(action) = k.trim().strip_prefix("KY_").or_else(|| k.trim().strip_prefix("ky_")) else { continue };
            let v = v.trim();
            if !action.is_empty() && !v.is_empty() && !v.starts_with('<') {
                out.insert(action.to_ascii_lowercase(), v.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wheel as OMSI 2 writes it, a shifter of openOMSI's with its own blocks.
    const FILE_TEXT: &str = "[ctrl]\r\nLogitech G25 Racing Wheel USB\r\n1\r\n\r\n[axis]\r\n0\r\n0\r\n1\r\n5\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n2\r\n9\r\n-1\r\n0\r\n3\r\n3\r\n\r\n[buttons]\r\n3\r\nautomatic_D\r\n0\r\n\r\n0\r\nhorn\r\n1\r\n\r\n[FFScale]\r\n1.010\r\n1.000\r\n\r\n\r\n[ctrl]\r\nT500 RS Gear Shift\r\n0\r\n\r\n[axis]\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n\r\n[buttons]\r\n2\r\nblinker_warn_toggle\r\n0\r\n\r\n0\r\n\r\n[FFScale]\r\n1.000\r\n1.000\r\n\r\n[openOMSI.Latching]\r\n1\r\n\r\n[openOMSI.FFInvert]\r\n1\r\n\r\n[openOMSI.MoreActions]\r\n1\r\n1\r\nkw_fernlicht_toggle\r\n\r\n";

    #[test]
    fn the_file_is_read_as_openomsi_reads_it() {
        let d = parse(FILE_TEXT);
        assert_eq!(d.len(), 2);
        let wheel = &d[0];
        assert_eq!((wheel.name.as_str(), wheel.second.as_str()), ("Logitech G25 Racing Wheel USB", "1"));
        assert_eq!(wheel.axes[0], [0, 0], "X steers");
        assert_eq!(wheel.axes[1], [1, 5], "Y: the throttle, reversed, degressive");
        assert_eq!(wheel.axes[5], [2, 9]);
        assert_eq!(wheel.axes[7], [3, 3], "slider 2: the clutch, reversed, longer travel");
        assert_eq!(wheel.buttons, vec![("automatic_D".into(), "0".into()), (String::new(), "0".into()), ("horn".into(), "1".into())]);
        assert_eq!(wheel.ff_scale, Some((1.01, 1.0)));
        let shifter = &d[1];
        assert_eq!((shifter.latching.clone(), shifter.ff_invert), (vec![0], Some(true)));
        assert_eq!(shifter.more, vec![(0, "kw_fernlicht_toggle".to_string())]);
    }

    #[test]
    fn the_file_is_written_back_as_it_was_read() {
        let d = parse(FILE_TEXT);
        let t = text(&d);
        assert_eq!(parse(&t), d, "read again, the same");
        assert!(t.contains("[ctrl]\r\nLogitech G25 Racing Wheel USB\r\n1\r\n"));
        assert!(t.contains("[openOMSI.MoreActions]\r\n1\r\n1\r\nkw_fernlicht_toggle\r\n"));
        assert!(t.contains("[FFScale]\r\n1.010\r\n1.000\r\n"));
        assert!(!t.replace("\r\n", "").contains('\n'), "CR LF throughout");
        // (a device of Omsi-Hub's: nothing set up)
        let new = text(&[Device::new("FANATEC CSL Pedals")]);
        assert_eq!(parse(&new), vec![Device { ff_scale: Some((1.0, 1.0)), ..Device::new("FANATEC CSL Pedals") }]);
    }

    #[test]
    fn only_what_the_file_can_hold_is_written() {
        let ok = Device::new("MOZA R5 Base");
        assert!(check(&[ok.clone()]).is_ok());
        assert!(check(&[Device::new("two\nlines")]).is_err());
        assert!(check(&[Device::new(" ")]).is_err());
        let mut d = ok.clone();
        d.axes[2] = [9, 0];
        assert!(check(&[d]).is_err(), "no such function");
        let mut d = ok.clone();
        d.buttons = vec![("horn\r".into(), "0".into())];
        assert!(check(&[d]).is_err());
        let mut d = ok.clone();
        d.buttons = vec![("horn".into(), "7".into())];
        assert!(check(&[d]).is_err(), "0 or 1 after the action");
        let mut d = ok;
        d.ff_scale = Some((f32::NAN, 1.0));
        assert!(check(&[d]).is_err());
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hub-game-controllers-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_newer_file_is_the_one_in_use_and_omsi_2_s_is_only_read() {
        let root = temp("places");
        let (home, content, omsi2) = (root.join("home"), root.join("content"), root.join("OMSI 2"));
        std::fs::create_dir_all(omsi2.join("Inputs")).unwrap();
        std::fs::write(omsi2.join("Inputs").join(FILE), "[ctrl]\r\nOld Wheel\r\n0\r\n\r\n[FFScale]\r\n0\r\n0\r\n").unwrap();
        let places = Places::new(&home, Some(&content), Some(&omsi2));
        let (devices, from) = load(&places).unwrap();
        assert_eq!(from, Some(omsi2.join("Inputs").join(FILE)), "nothing of openOMSI's: OMSI 2's");
        assert_eq!(devices[0].ff_scale, None, "OMSI 2's 0 and 0: openOMSI's own strength");
        // (written: openOMSI's own folder, OMSI 2's left as it was)
        let written = save(&places, &devices).unwrap();
        assert_eq!(written, vec![home.join(".openomsi").join("Inputs").join(FILE)]);
        assert!(std::fs::read_to_string(omsi2.join("Inputs").join(FILE)).unwrap().contains("0\r\n0"));
        assert_eq!(load(&places).unwrap().1, Some(written[0].clone()));
        // (the content folder's file, newer: in use, and written as well from then on)
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::create_dir_all(content.join("inputs")).unwrap();
        std::fs::write(content.join("inputs").join(FILE), text(&[Device::new("Content Wheel")])).unwrap();
        let (devices, from) = load(&places).unwrap();
        assert_eq!((devices[0].name.as_str(), from), ("Content Wheel", Some(content.join("inputs").join(FILE))));
        let written = save(&places, &[Device::new("Both")]).unwrap();
        assert_eq!(written.len(), 2);
        for p in &written {
            assert_eq!(parse(&std::fs::read_to_string(p).unwrap())[0].name, "Both");
        }
        // (the file as it was before Omsi-Hub wrote it, kept once)
        let backup = content.join("inputs").join("gamectrler.cfg.before-omsihub");
        assert_eq!(parse(&std::fs::read_to_string(&backup).unwrap())[0].name, "Content Wheel");
        save(&places, &[Device::new("Again")]).unwrap();
        assert_eq!(parse(&std::fs::read_to_string(&backup).unwrap())[0].name, "Content Wheel");
        assert!(save(&places, &[Device::new("bad\tname")]).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_buttons_actions_and_their_names_come_from_omsi_s_files() {
        let root = temp("names");
        let omsi2 = root.join("OMSI 2");
        std::fs::create_dir_all(omsi2.join("Inputs")).unwrap();
        std::fs::create_dir_all(omsi2.join("Languages")).unwrap();
        std::fs::write(
            omsi2.join("Inputs").join("keyboard.cfg"),
            "[game]\r\n\r\n[entry]\r\nsim_pause\r\n25\r\n0\r\n\r\n[vehicles]\r\n\r\n[entry]\r\nhorn\r\n35\r\n0\r\n\r\n[entry]\r\nHORN\r\n36\r\n0\r\n",
        )
        .unwrap();
        let places = Places::new(&root.join("home"), None, Some(&omsi2));
        assert_eq!(key_actions(&places), vec!["sim_pause".to_string(), "horn".to_string()]);
        std::fs::write(
            omsi2.join("Languages").join("DEU_key_veh_gen.olf"),
            b"DEU\t\r\nKY_horn\tHupe\r\nKY_kw_fernlicht_toggle\tFernlicht an/aus\r\nKY_empty\t\r\nKY_x\t<nicht>\r\nKY_brake\tBremse \xfc\r\n",
        )
        .unwrap();
        let n = action_names(&omsi2, "DEU");
        assert_eq!(n.get("horn").map(String::as_str), Some("Hupe"));
        assert_eq!(n.get("kw_fernlicht_toggle").map(String::as_str), Some("Fernlicht an/aus"));
        assert_eq!(n.get("brake").map(String::as_str), Some("Bremse ü"), "Windows-1252");
        assert!(!n.contains_key("empty") && !n.contains_key("x"));
        assert!(action_names(&omsi2, "ENG").is_empty());
        assert_eq!((names_language("de"), names_language("fr-BE"), names_language("nl")), ("DEU", "FRA", "ENG"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
