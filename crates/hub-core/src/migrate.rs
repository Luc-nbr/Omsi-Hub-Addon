//! The old Omsi-Hub's saves, from `~/.openomsi` (the fork kept its data in openOMSI's folder),
//! into Omsi-Hub's own data folder - once, on the first start, with the player's yes.
//!
//! [`scan`] only reads: it lists what there is to take (INVENTORY §1, "Saves to migrate").
//! [`import`] copies it. **Not ported yet:** it answers an error until the company port knows
//! each save's format (`companies/<id>.json` VERSION 3, `lines/<map>.json` REGISTRY_VERSION 2,
//! and `settings.cfg`, whose Omsi-Hub keys are mixed with openOMSI's).
//!
//! Nothing is ever written to, moved out of or deleted from `~/.openomsi`.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// The old saves, relative to `~/.openomsi`: folders whose files are taken, and single files.
pub const FOLDERS: &[&str] = &["companies", "careers", "trips", "lines", "depots"];
pub const FILES: &[&str] = &["hidden-drivers.txt", "bus-fonts.json", "bus-options.json", "vehicle-classes.json", "personnel.json", "omsihub-addon.json", "launcher.json", "settings.cfg"];

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

/// The old data folder: `~/.openomsi`.
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

/// Copy what [`scan`] found into the data folder. **Not ported yet.**
pub fn import(_found: &Found) -> Result<usize, String> {
    Err("not ported yet: the import of the old saves comes with the company port".into())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(import(&f).is_err());
        assert!(scan(&old.join("none")).is_empty());
        let _ = std::fs::remove_dir_all(&old);
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
