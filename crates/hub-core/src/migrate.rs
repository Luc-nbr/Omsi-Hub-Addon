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

/// Omsi-Hub's own keys of the old game's `settings.cfg` (the rest are openOMSI's), taken into
/// the settings with their values.
pub const OWN_KEYS: &[&str] = &["bus_lead", "animations", "page_bus", "dark_mode", "welcome_done"];

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

/// The old data folder: `~/.openomsi` - in the home folder on Windows, macOS and Linux alike
/// (the old Omsi-Hub kept it there on every system).
pub fn old_folder() -> PathBuf {
    crate::paths::home_dir().join(".openomsi")
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
            "settings.cfg" => out.settings = own_settings(&std::fs::read_to_string(&from).unwrap_or_default()),
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
fn driver_of(path: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    v.get("profile").and_then(|p| p.as_str()).map(str::trim).filter(|p| !p.is_empty()).map(str::to_string)
}

/// Omsi-Hub's keys of a `settings.cfg` text (`key=value` lines), as JSON values: a number or a
/// truth where the value is one.
pub fn own_settings(text: &str) -> Vec<(String, serde_json::Value)> {
    let mut out: Vec<(String, serde_json::Value)> = Vec::new();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if !OWN_KEYS.contains(&k) || out.iter().any(|(x, _)| x == k) {
            continue;
        }
        let value = match v {
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            _ => v.parse::<i64>().map(serde_json::Value::from).or_else(|_| v.parse::<f64>().map(serde_json::Value::from)).unwrap_or_else(|_| serde_json::Value::String(v.to_string())),
        };
        out.push((k.to_string(), value));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_folder_is_in_the_home_folder_on_every_system() {
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
