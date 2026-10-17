//! The old Omsi-Hub's saves, from `~/.openomsi` (the fork kept its data in openOMSI's folder),
//! into Omsi-Hub's own data folder - once, on the first start, with the player's yes.
//!
//! [`scan`] only reads: it lists what there is to take (INVENTORY §1, "Saves to migrate").
//! [`import`] copies it, 1:1 and under the same names: the company code reads the old formats
//! as they are (`companies/<id>.json` VERSION 3 with its `.live.jsonl`, `.orders.jsonl`,
//! `.inbox.json`, `.liveplan.json` and `.taken` beside it, `careers/<driver>.json`,
//! `trips/<driver>.jsonl`, `lines/<map>.json` REGISTRY_VERSION 2, `depots/*.hof`, the bus fonts,
//! options and classes). Two files are not copied but read:
//!
//! * `launcher.json` (the old launcher's): its `profile`, the driver, becomes the settings'
//!   `driver`;
//! * `settings.cfg` (the old game's, `key=value`): its Omsi-Hub keys ([`OWN_KEYS`]: the bus's
//!   lead before a duty, the page's look) go into the settings; openOMSI's own keys stay there.
//!
//! A file the data folder has already is kept (an import twice, or after playing on, changes
//! nothing). Nothing is ever written to, moved out of or deleted from `~/.openomsi`.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// The old saves, relative to `~/.openomsi`: folders whose files are taken, and single files.
pub const FOLDERS: &[&str] = &["companies", "careers", "trips", "lines", "depots", "cache/busphotos"];
pub const FILES: &[&str] = &[
    "hidden-drivers.txt",
    "bus-fonts.json",
    "bus-options.json",
    "vehicle-classes.json",
    "personnel.json",
    "omsihub-addon.json",
    "depots-added.txt",
    "company-tour.json",
    "rosterflow-tour.json",
    "launcher.json",
    "settings.cfg",
];

/// The files of [`FILES`] that are read into the settings, not copied.
pub const READ_ONLY: &[&str] = &["launcher.json", "settings.cfg"];

/// Omsi-Hub's own keys of the old game's `settings.cfg` that the new app still uses (the rest
/// are openOMSI's, or of what the new app has no more), taken into the settings with their
/// values made tidy as the old launcher read them ([`own_setting`]). `language` and `theme` are
/// read too: the language into the settings' own field, `theme` as `dark_mode`.
pub const OWN_KEYS: &[(&str, Kind)] = &[
    ("bus_lead", Kind::Minutes),
    ("animations", Kind::Truth),
    ("page_bus", Kind::Truth),
    ("dark_mode", Kind::Truth),
    ("welcome_done", Kind::Truth),
    ("accent", Kind::Colour),
    ("launcher_scale", Kind::Scale),
    ("company_layout", Kind::Layout),
    ("company_sidebar", Kind::Sidebar),
    ("company_pins", Kind::Pins),
    ("nav_signon", Kind::Truth),
    ("nav_board", Kind::Truth),
    ("nav_ontime", Kind::Truth),
    ("nav_strip", Kind::Truth),
    ("nav_scale", Kind::Scale),
    ("nav_rect", Kind::Text),
    ("stop_style", Kind::Text),
    ("ibis_auto", Kind::Truth),
    ("update_check", Kind::Truth),
    ("update_auto", Kind::Truth),
    ("update_notify", Kind::Truth),
];

/// How a key of [`OWN_KEYS`] is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `1`/`true`/`on`/`yes`, else false.
    Truth,
    /// Minutes, 0 to 60 (the bus's lead before a duty).
    Minutes,
    /// `#rrggbb` (`#rgb` written out); anything else left out.
    Colour,
    /// 0.6 to 2.0 in steps of 0.05.
    Scale,
    /// `sidebar` unless `palette`.
    Layout,
    /// `open` unless `rail` or `hidden`.
    Sidebar,
    /// Page numbers, comma-separated, each once, at most twelve.
    Pins,
    /// As it is.
    Text,
}

/// One thing to take.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    /// Relative to the old folder, with `/`.
    pub path: String,
    pub bytes: u64,
}

/// What the old folder holds of Omsi-Hub's.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Found {
    pub folder: PathBuf,
    pub items: Vec<Item>,
    /// The companies found (their files' names without `.json`).
    pub companies: Vec<String>,
    /// The drivers with a career.
    pub careers: Vec<String>,
}

impl Found {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// The environment variable that points the old data folder elsewhere (the tests and the
/// selftest, with a fixture: `fixtures/old-openomsi`).
pub const OLD_FOLDER_ENV: &str = "OMSIHUB_OLD_DATA_DIR";

/// The old data folder: `~/.openomsi` - in the home folder on Windows, macOS and Linux alike
/// (the old Omsi-Hub kept it there on every system); [`OLD_FOLDER_ENV`] when it is set.
pub fn old_folder() -> PathBuf {
    match std::env::var_os(OLD_FOLDER_ENV).filter(|v| !v.is_empty()) {
        Some(d) => PathBuf::from(d),
        None => crate::paths::home_dir().join(".openomsi"),
    }
}

/// What `old` (normally [`old_folder`]) holds to import. Reads only.
pub fn scan(old: &Path) -> Found {
    let mut found = Found { folder: old.to_path_buf(), ..Found::default() };
    for f in FILES {
        if let Some(m) = std::fs::metadata(old.join(f)).ok().filter(|m| m.is_file()) {
            found.items.push(Item { path: f.to_string(), bytes: m.len() });
        }
    }
    for dir in FOLDERS {
        let Ok(entries) = std::fs::read_dir(old.join(dir)) else { continue };
        let mut names: Vec<(String, u64)> = entries.flatten().filter_map(|e| Some((e.file_name().to_string_lossy().into_owned(), e.metadata().ok().filter(|m| m.is_file())?.len()))).collect();
        names.sort();
        for (name, bytes) in names {
            if *dir == "companies" && name.ends_with(".json") && !name.contains(".live") && !name.contains(".inbox") && !name.contains(".liveplan") {
                found.companies.push(name.trim_end_matches(".json").to_string());
            }
            if *dir == "careers" && name.ends_with(".json") {
                found.careers.push(name.trim_end_matches(".json").to_string());
            }
            found.items.push(Item { path: format!("{dir}/{name}"), bytes });
        }
    }
    found
}

/// What the setup shows of the old saves before they are copied ([`preview`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Preview {
    /// The companies: their files' names and the names they go by.
    pub companies: Vec<(String, String)>,
    /// The drivers with a career, the maps with own lines, the depot files.
    pub careers: Vec<String>,
    pub lines: Vec<String>,
    pub depots: Vec<String>,
    /// The settings that are taken over (their keys), the language and the driver.
    pub settings: Vec<String>,
    pub language: Option<String>,
    pub driver: Option<String>,
    /// All the files, and their size.
    pub files: usize,
    pub bytes: u64,
}

/// What [`scan`] found, as the setup shows it. Reads only.
pub fn preview(found: &Found) -> Preview {
    let under = |dir: &str, ext: &str| -> Vec<String> {
        found.items.iter().filter_map(|i| i.path.strip_prefix(&format!("{dir}/"))).filter(|n| n.to_ascii_lowercase().ends_with(ext)).map(|n| n[..n.len() - ext.len()].to_string()).collect()
    };
    let name_of = |id: &str| -> String {
        let text = std::fs::read_to_string(found.folder.join("companies").join(format!("{id}.json"))).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string)).unwrap_or_else(|| id.to_string())
    };
    let cfg = found.items.iter().any(|i| i.path == "settings.cfg").then(|| std::fs::read_to_string(found.folder.join("settings.cfg")).unwrap_or_default());
    Preview {
        companies: found.companies.iter().map(|id| (id.clone(), name_of(id))).collect(),
        careers: found.careers.clone(),
        lines: under("lines", ".json"),
        depots: under("depots", ".hof"),
        settings: cfg.as_deref().map(|t| own_settings(t).into_iter().map(|(k, _)| k).collect()).unwrap_or_default(),
        language: cfg.as_deref().and_then(language_of),
        driver: found.items.iter().any(|i| i.path == "launcher.json").then(|| driver_of(&found.folder.join("launcher.json"))).flatten(),
        files: found.items.iter().filter(|i| !READ_ONLY.contains(&i.path.as_str())).count(),
        bytes: found.items.iter().map(|i| i.bytes).sum(),
    }
}

/// What [`import_into`] did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Imported {
    /// The files copied (relative, with `/`).
    pub copied: Vec<String>,
    /// The files the data folder had already: kept as they are.
    pub kept: Vec<String>,
    /// The driver taken from the old launcher (`launcher.json`'s `profile`).
    pub driver: Option<String>,
    /// The settings taken from the old game's `settings.cfg` (key, value).
    pub settings: Vec<(String, serde_json::Value)>,
    /// The language the old game was in (`en`, `de` …), from its `settings.cfg`.
    pub language: Option<String>,
}

/// Copy what [`scan`] found into Omsi-Hub's data folder, and take the driver and Omsi-Hub's
/// settings over (see the module's text). The number of files copied. The settings are marked
/// `migrated`: the setup does not ask again.
pub fn import(found: &Found) -> Result<usize, String> {
    let done = import_into(found, &crate::paths::data_dir())?;
    let mut s = crate::settings::Settings::load();
    if let Some(d) = &done.driver {
        s.other.entry("driver").or_insert_with(|| serde_json::Value::String(d.clone()));
    }
    for (k, v) in &done.settings {
        s.other.entry(k.clone()).or_insert_with(|| v.clone());
    }
    if let Some(l) = done.language.as_ref().filter(|_| s.language.is_empty()) {
        s.language = l.clone();
    }
    s.migrated = true;
    s.save().map_err(|e| format!("{}: {e}", crate::settings::Settings::path().display()))?;
    crate::log_line(&format!("migrate: {} files of {} copied, {} kept", done.copied.len(), found.folder.display(), done.kept.len()));
    Ok(done.copied.len())
}

/// [`import`] into the data folder `data` (the settings it would take are said, not written).
pub fn import_into(found: &Found, data: &Path) -> Result<Imported, String> {
    let mut out = Imported::default();
    for item in &found.items {
        let rel = item.path.as_str();
        // (only what `scan` lists: a path of ours, relative, no way out of the folders)
        if rel.split('/').any(|p| p.is_empty() || p == "." || p == "..") || rel.contains('\\') {
            continue;
        }
        let from = found.folder.join(rel);
        match rel {
            "launcher.json" => out.driver = driver_of(&from),
            "settings.cfg" => {
                let text = std::fs::read_to_string(&from).unwrap_or_default();
                out.settings = own_settings(&text);
                out.language = language_of(&text);
            }
            _ => {
                let to = data.join(rel);
                if to.exists() {
                    out.kept.push(rel.to_string());
                    continue;
                }
                copy_file(&from, &to).map_err(|e| format!("{rel}: {e}"))?;
                out.copied.push(rel.to_string());
            }
        }
    }
    Ok(out)
}

/// `from` copied to `to`, through a file beside it (a copy cut off is no half save).
fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(d) = to.parent() {
        std::fs::create_dir_all(d)?;
    }
    let part = to.with_file_name(format!("{}.part", to.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::copy(from, &part)?;
    std::fs::rename(&part, to).inspect_err(|_| {
        let _ = std::fs::remove_file(&part);
    })
}

/// The driver of the old launcher's `launcher.json` (its `profile`).
pub fn driver_of(path: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    v.get("profile").and_then(|p| p.as_str()).map(str::trim).filter(|p| !p.is_empty()).map(str::to_string)
}

/// The `key=value` lines of a `settings.cfg` text (comments and blank lines left out; the first
/// of a key counts).
fn cfg_lines(text: &str) -> Vec<(&str, &str)> {
    let mut out: Vec<(&str, &str)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if !out.iter().any(|(x, _)| *x == k) {
            out.push((k, v));
        }
    }
    out
}

/// A value of `kind` as the settings keep it (None: not a value of its kind, left out).
pub fn own_setting(kind: Kind, v: &str) -> Option<serde_json::Value> {
    use serde_json::Value as J;
    let truth = |x: &str| matches!(x.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
    Some(match kind {
        Kind::Truth => J::Bool(truth(v)),
        Kind::Minutes => J::from(v.parse::<f64>().ok().filter(|x| x.is_finite())?.round().clamp(0.0, 60.0) as i64),
        Kind::Colour => {
            let h = v.trim().trim_start_matches('#');
            let full: String = if h.len() == 3 { h.chars().flat_map(|c| [c, c]).collect() } else { h.to_string() };
            if full.len() != 6 || !full.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            J::String(format!("#{}", full.to_ascii_lowercase()))
        }
        Kind::Scale => {
            let x = v.parse::<f64>().ok().filter(|x| x.is_finite())?.clamp(0.6, 2.0);
            J::from((x * 20.0).round() / 20.0)
        }
        Kind::Layout => J::String(if v.eq_ignore_ascii_case("palette") { "palette" } else { "sidebar" }.into()),
        Kind::Sidebar => J::String(
            match v.to_ascii_lowercase().as_str() {
                "rail" => "rail",
                "hidden" => "hidden",
                _ => "open",
            }
            .into(),
        ),
        Kind::Pins => {
            let mut pins: Vec<u32> = Vec::new();
            for n in v.split(',').filter_map(|x| x.trim().parse::<u32>().ok()) {
                if n < 200 && !pins.contains(&n) && pins.len() < 12 {
                    pins.push(n);
                }
            }
            J::String(pins.iter().map(u32::to_string).collect::<Vec<_>>().join(","))
        }
        Kind::Text => J::String(v.to_string()),
    })
}

/// Omsi-Hub's keys of a `settings.cfg` text ([`OWN_KEYS`]), as the settings keep them; `theme`
/// (`dark`/`light`) as `dark_mode` where the file has no `dark_mode`.
pub fn own_settings(text: &str) -> Vec<(String, serde_json::Value)> {
    let lines = cfg_lines(text);
    let mut out: Vec<(String, serde_json::Value)> = Vec::new();
    for (k, v) in &lines {
        let Some((_, kind)) = OWN_KEYS.iter().find(|(key, _)| key == k) else { continue };
        if let Some(value) = own_setting(*kind, v) {
            out.push((k.to_string(), value));
        }
    }
    if !out.iter().any(|(k, _)| k == "dark_mode") {
        if let Some((_, theme)) = lines.iter().find(|(k, _)| *k == "theme") {
            match theme.to_ascii_lowercase().as_str() {
                "dark" | "night" => out.push(("dark_mode".into(), serde_json::Value::Bool(true))),
                "light" | "day" => out.push(("dark_mode".into(), serde_json::Value::Bool(false))),
                _ => {}
            }
        }
    }
    out
}

/// The language of a `settings.cfg` text (`language`, or the older `lang`), as the app's texts
/// name it: OMSI's three letters (`ENG`, `DEU` …) or a two-letter code. None: none, or one the
/// app does not have.
pub fn language_of(text: &str) -> Option<String> {
    let lines = cfg_lines(text);
    let v = lines.iter().find(|(k, _)| *k == "language").or_else(|| lines.iter().find(|(k, _)| *k == "lang"))?.1;
    let code = match v.trim().to_lowercase().as_str() {
        "eng" | "en" | "english" => "en",
        "deu" | "ger" | "de" | "deutsch" | "german" => "de",
        "nld" | "dut" | "nl" | "nederlands" | "dutch" => "nl",
        "fra" | "fre" | "fr" | "fran\u{e7}ais" | "french" => "fr",
        "pol" | "pl" | "polski" | "polish" => "pl",
        "rus" | "ru" | "russian" => "ru",
        "ukr" | "uk" | "ukrainian" => "uk",
        "ces" | "cze" | "cs" | "czech" => "cs",
        "ita" | "it" | "italian" => "it",
        "esp" | "spa" | "es" | "spanish" => "es",
        _ => return None,
    };
    Some(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_folder_is_in_the_home_folder_on_every_system() {
        if std::env::var_os(OLD_FOLDER_ENV).is_some() {
            return;
        }
        let f = old_folder();
        assert_eq!(f.file_name().unwrap(), ".openomsi");
        assert_eq!(f.parent().unwrap(), crate::paths::home_dir());
    }

    #[test]
    fn the_old_saves_are_found_and_nothing_is_written() {
        let old = std::env::temp_dir().join(format!("omsihub-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&old);
        std::fs::create_dir_all(old.join("companies")).unwrap();
        std::fs::create_dir_all(old.join("careers")).unwrap();
        std::fs::write(old.join("companies/abc.json"), "{}").unwrap();
        std::fs::write(old.join("companies/abc.live.jsonl"), "").unwrap();
        std::fs::write(old.join("careers/Luc.json"), "{}").unwrap();
        std::fs::write(old.join("bus-fonts.json"), "[]").unwrap();
        std::fs::write(old.join("game.log"), "not ours").unwrap();
        let before: Vec<_> = walk(&old);
        let f = scan(&old);
        assert_eq!(f.companies, ["abc"]);
        assert_eq!(f.careers, ["Luc"]);
        let paths: Vec<&str> = f.items.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, ["bus-fonts.json", "companies/abc.json", "companies/abc.live.jsonl", "careers/Luc.json"]);
        assert_eq!(walk(&old), before);
        assert!(scan(&old.join("none")).is_empty());
        let _ = std::fs::remove_dir_all(&old);
    }

    /// The saves are copied 1:1 under their names, the driver and Omsi-Hub's settings read, a
    /// file the data folder has kept - and the old folder is as it was.
    #[test]
    fn the_old_saves_are_copied_and_never_moved() {
        let base = std::env::temp_dir().join(format!("omsihub-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (old, data) = (base.join("old"), base.join("data"));
        for d in ["companies", "careers", "trips", "lines", "depots", "cache/busphotos"] {
            std::fs::create_dir_all(old.join(d)).unwrap();
        }
        // (a company as the old launcher saved it: the same store, the same format)
        let mut c = crate::company::found(&crate::company::Founding { name: "Stadtbus".into(), date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.id = "abc".into();
        crate::company::store::save(&old, &c).unwrap();
        let company = std::fs::read_to_string(old.join("companies/abc.json")).unwrap();
        std::fs::write(old.join("companies/abc.live.jsonl"), "{}\n").unwrap();
        std::fs::write(old.join("companies/abc.inbox.json"), "{}").unwrap();
        std::fs::write(old.join("careers/Luc.json"), "{}").unwrap();
        std::fs::write(old.join("trips/luc.jsonl"), "{\"time\":5}\n").unwrap();
        std::fs::write(old.join("lines/Grundorf.json"), r#"{"version":2}"#).unwrap();
        std::fs::write(old.join("depots/oo_Grundorf.hof"), "[name]\r\nGrundorf\r\n").unwrap();
        std::fs::write(old.join("cache/busphotos/0123456789abcdef.png"), b"\x89PNG").unwrap();
        std::fs::write(old.join("bus-fonts.json"), r#"{"buses":{}}"#).unwrap();
        std::fs::write(old.join("launcher.json"), r#"{"root":"C:/OMSI 2","game":"","profile":" Luc "}"#).unwrap();
        std::fs::write(old.join("settings.cfg"), "msaa=4\nbus_lead=12\ndark_mode=true\nlanguage=ENG\n").unwrap();
        // (the data folder has a career of its own already)
        std::fs::create_dir_all(data.join("careers")).unwrap();
        std::fs::write(data.join("careers/Luc.json"), "mine").unwrap();
        let before: Vec<(PathBuf, Vec<u8>)> = walk(&old).into_iter().map(|p| (p.clone(), std::fs::read(&p).unwrap())).collect();
        let f = scan(&old);
        let done = import_into(&f, &data).unwrap();
        assert_eq!(done.driver.as_deref(), Some("Luc"));
        assert_eq!(done.settings, vec![("bus_lead".to_string(), serde_json::json!(12)), ("dark_mode".to_string(), serde_json::json!(true))]);
        assert_eq!(done.language.as_deref(), Some("en"));
        assert_eq!(done.kept, ["careers/Luc.json"]);
        assert_eq!(std::fs::read_to_string(data.join("careers/Luc.json")).unwrap(), "mine");
        assert_eq!(std::fs::read_to_string(data.join("companies/abc.json")).unwrap(), company);
        for rel in
            ["companies/abc.live.jsonl", "companies/abc.inbox.json", "trips/luc.jsonl", "lines/Grundorf.json", "depots/oo_Grundorf.hof", "cache/busphotos/0123456789abcdef.png", "bus-fonts.json"]
        {
            assert!(done.copied.iter().any(|c| c == rel), "{rel}");
            assert_eq!(std::fs::read(old.join(rel)).unwrap(), std::fs::read(data.join(rel)).unwrap(), "{rel}");
        }
        assert!(!data.join("launcher.json").exists() && !data.join("settings.cfg").exists());
        // the old folder: every file as it was, none moved
        let after: Vec<(PathBuf, Vec<u8>)> = walk(&old).into_iter().map(|p| (p.clone(), std::fs::read(&p).unwrap())).collect();
        assert_eq!(after, before);
        // (again: nothing more copied, nothing written over)
        let again = import_into(&f, &data).unwrap();
        assert!(again.copied.is_empty() && again.kept.len() == done.copied.len() + 1);
        // the old company reads in the new folder as it did
        let back = crate::company::store::load(&data, "abc").unwrap();
        assert_eq!((back.name.as_str(), back.date.as_str()), ("Stadtbus", "2024-03-04"));
        assert_eq!(crate::company::store::list(&data, "Luc").len(), 1);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The repository's fixture of an old `~/.openomsi` (`fixtures/old-openomsi`).
    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/old-openomsi")
    }

    /// The fixture's company, as the old launcher saved it (written again with
    /// `cargo test -p hub-core make_the_old_folder_s_company -- --ignored`).
    #[test]
    #[ignore = "writes the fixture's company file"]
    fn make_the_old_folder_s_company() {
        let mut c =
            crate::company::found(&crate::company::Founding { name: "Stadtbus Grundorf".into(), date: "2024-03-04".into(), map: "maps/Grundorf/global.cfg".into(), ..Default::default() }, "Luc");
        c.id = "stadtbus".into();
        crate::company::store::save(&fixture(), &c).unwrap();
    }

    /// A whole old folder (the fixture: companies with their side files, a career, trips, own
    /// lines, depots, the bus fonts, options and classes, the old launcher's driver, the old
    /// game's settings mixed with openOMSI's, and openOMSI's own log and screenshots) copied into
    /// a new data folder: what is Omsi-Hub's is there and reads, openOMSI's is not taken, the
    /// settings we still use are taken over - and the fixture is as it was, byte for byte.
    #[test]
    fn a_whole_old_folder_is_copied_and_left_as_it_was() {
        let old = fixture();
        let data = std::env::temp_dir().join(format!("omsihub-import-fixture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let before: Vec<(PathBuf, Vec<u8>)> = walk(&old).into_iter().map(|p| (p.clone(), std::fs::read(&p).unwrap())).collect();
        let found = scan(&old);
        assert_eq!(found.companies, ["stadtbus"]);
        assert_eq!(found.careers, ["luc"]);
        let p = preview(&found);
        assert_eq!(p.companies, [("stadtbus".to_string(), "Stadtbus Grundorf".to_string())]);
        assert_eq!((p.lines.as_slice(), p.depots.as_slice()), (["Grundorf".to_string()].as_slice(), ["oo_Grundorf".to_string()].as_slice()));
        assert_eq!((p.language.as_deref(), p.driver.as_deref(), p.files), (Some("de"), Some("Luc"), 11));
        assert!(p.settings.contains(&"accent".to_string()) && p.settings.contains(&"company_layout".to_string()), "{:?}", p.settings);
        let done = import_into(&found, &data).unwrap();
        for rel in [
            "companies/stadtbus.json",
            "companies/stadtbus.live.jsonl",
            "companies/stadtbus.inbox.json",
            "careers/luc.json",
            "trips/luc.jsonl",
            "lines/Grundorf.json",
            "depots/oo_Grundorf.hof",
            "bus-fonts.json",
            "bus-options.json",
            "vehicle-classes.json",
            "hidden-drivers.txt",
        ] {
            assert!(done.copied.iter().any(|c| c == rel), "{rel} not copied: {:?}", done.copied);
            assert_eq!(std::fs::read(old.join(rel)).unwrap(), std::fs::read(data.join(rel)).unwrap(), "{rel}");
        }
        // openOMSI's own files and the two read ones are not copied
        for rel in ["game.log", "screenshots/readme.txt", "settings.cfg", "launcher.json"] {
            assert!(!data.join(rel).exists(), "{rel} was copied");
        }
        // the driver, the language and the settings we still use, made tidy as the old launcher read them
        assert_eq!((done.driver.as_deref(), done.language.as_deref()), (Some("Luc"), Some("de")));
        let get = |k: &str| done.settings.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
        assert_eq!(get("accent"), Some(serde_json::json!("#2a75f7")));
        assert_eq!(get("dark_mode"), Some(serde_json::json!(true)));
        assert_eq!(get("company_layout"), Some(serde_json::json!("palette")));
        assert_eq!(get("company_sidebar"), Some(serde_json::json!("rail")));
        assert_eq!(get("company_pins"), Some(serde_json::json!("3,1,15")));
        assert_eq!((get("nav_signon"), get("nav_board"), get("ibis_auto")), (Some(serde_json::json!(true)), Some(serde_json::json!(false)), Some(serde_json::json!(false))));
        assert_eq!((get("bus_lead"), get("launcher_scale"), get("update_auto")), (Some(serde_json::json!(8)), Some(serde_json::json!(1.25)), Some(serde_json::json!(false))));
        assert_eq!((get("msaa"), get("fullscreen"), get("language")), (None, None, None), "openOMSI's keys stay in its file");
        // the company, its career and its trips read in the new folder
        let c = crate::company::store::load(&data, "stadtbus").unwrap();
        assert_eq!((c.name.as_str(), c.date.as_str()), ("Stadtbus Grundorf", "2024-03-04"));
        assert_eq!(crate::company::store::list(&data, "Luc").len(), 1);
        assert_eq!(crate::trips_of(&data, "Luc").len(), 1);
        assert_eq!(crate::company::career::load(&data, "Luc").driver, "Luc");
        // the fixture: nothing written, moved or added
        let after: Vec<(PathBuf, Vec<u8>)> = walk(&old).into_iter().map(|p| (p.clone(), std::fs::read(&p).unwrap())).collect();
        assert_eq!(after, before);
        let _ = std::fs::remove_dir_all(&data);
    }

    fn walk(d: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(d).unwrap().flatten() {
            if e.path().is_dir() {
                out.extend(walk(&e.path()));
            } else {
                out.push(e.path());
            }
        }
        out.sort();
        out
    }
}
