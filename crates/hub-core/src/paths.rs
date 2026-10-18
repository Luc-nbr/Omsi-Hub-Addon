//! Where things are: Omsi-Hub's own data folder, and the player's folders.
//!
//! **The data folder** is Omsi-Hub's alone, under the system's app data folder:
//! `%APPDATA%\Omsi-Hub` on Windows, `~/Library/Application Support/Omsi-Hub` on macOS,
//! `$XDG_DATA_HOME/Omsi-Hub` (`~/.local/share/Omsi-Hub`) on Linux. Never `~/.openomsi`, which is
//! openOMSI's (the old Omsi-Hub's saves there are imported once, `migrate`). The environment
//! variable `OMSIHUB_DATA_DIR` puts it elsewhere (tests, a portable install).
//!
//! **The player's folders** - the openOMSI folder and the OMSI 2 folder - come from the
//! settings (`settings.json`, chosen in the first-run setup). Omsi-Hub reads the OMSI 2 content
//! through them ([`vfs`]: the content folder first, then the OMSI 2 folder) and never writes
//! there.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The environment variable that moves the data folder.
pub const DATA_DIR_ENV: &str = "OMSIHUB_DATA_DIR";
/// The data folder's name under the system's app data folder.
pub const APP_FOLDER: &str = "Omsi-Hub";

/// The folder the data folder would be in, without the override.
pub fn system_data_dir() -> PathBuf {
    // (Roaming AppData on Windows, Application Support on macOS, XDG_DATA_HOME on Linux)
    #[cfg(feature = "native")]
    return dirs::data_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join(APP_FOLDER);
    // (the website: the folder the page lays out in the browser, the cloud save's files in it)
    #[cfg(not(feature = "native"))]
    PathBuf::from("/data")
}

/// Omsi-Hub's data folder, made when it is first asked for.
pub fn data_dir() -> PathBuf {
    static MADE: Mutex<Option<PathBuf>> = Mutex::new(None);
    let d = std::env::var_os(DATA_DIR_ENV).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(system_data_dir);
    let mut made = MADE.lock().unwrap_or_else(|e| e.into_inner());
    if made.as_ref() != Some(&d) && std::fs::create_dir_all(&d).is_ok() {
        *made = Some(d.clone());
    }
    d
}

/// The user's home folder (where openOMSI keeps `~/.openomsi`).
pub fn home_dir() -> PathBuf {
    #[cfg(feature = "native")]
    return dirs::home_dir().unwrap_or_default();
    #[cfg(not(feature = "native"))]
    PathBuf::from("/home")
}

/// The OMSI 2 folder from the settings, when it is set and there.
pub fn root() -> Option<PathBuf> {
    let s = crate::settings::Settings::load();
    Some(PathBuf::from(s.omsi2_folder)).filter(|p| !p.as_os_str().is_empty() && p.is_dir())
}

/// openOMSI's content folder for the chosen openOMSI (where it reads mods and its plugins
/// from), when the openOMSI folder is set. See [`content_dir_of`].
pub fn content_dir() -> Option<PathBuf> {
    let s = crate::settings::Settings::load();
    if s.openomsi_folder.is_empty() {
        return None;
    }
    content_dir_of(Path::new(&s.openomsi_folder), &home_dir())
}

/// The content folder of the openOMSI in `folder`, by openOMSI's own rule, the same on
/// Windows, macOS and Linux: the folder of its program (beside the `.app` bundle on macOS), or
/// its `openOMSI` folder when it was unpacked into the OMSI 2 folder - unless that may not be
/// written to (Program Files, `/opt`, `/Applications`), when it is `~/.openomsi/content`.
///
/// Omsi-Hub writes nothing to find it: the folder beside the program counts when openOMSI has
/// used it (its `Plugins` or `Mods` is there) or when there is no `~/.openomsi/content`.
/// (hub-game's `detect` makes the same choice by a write test when it installs the plugin.)
pub fn content_dir_of(folder: &Path, home: &Path) -> Option<PathBuf> {
    use hub_omsi::vfs::find_child;
    let bundle = std::fs::read_dir(folder).ok()?.flatten().map(|e| e.path()).any(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("app")) && p.join("Contents/MacOS").is_dir());
    let program = ["openomsi.exe", "openomsi"].iter().any(|n| find_child(folder, n).is_some_and(|p| p.is_file()));
    if !program && !bundle {
        return None;
    }
    let beside = if find_child(folder, "Omsi.exe").is_some() { folder.join("openOMSI") } else { folder.to_path_buf() };
    let fallback = home.join(".openomsi").join("content");
    let used = ["Plugins", "Mods"].iter().any(|n| find_child(&beside, n).is_some_and(|p| p.is_dir()));
    if used || (beside.is_dir() && !fallback.is_dir()) {
        Some(beside)
    } else {
        Some(fallback).filter(|f| f.is_dir())
    }
}

/// `rel` inside `base` (either slash), each part found in any case as far as it is there, the
/// rest as `rel` names it: the path of a file or folder that is there, or of one to be made
/// beside what is there (so that on a case-sensitive file system a `Vehicles` folder is not
/// made again beside a `vehicles` one). (The old `omsi_cfg::resolve_path`.)
pub fn resolve_path(base: &Path, rel: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    let mut found = true;
    for part in rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            p.pop();
            continue;
        }
        let next = if found { hub_omsi::vfs::find_child(&p, part) } else { None };
        found = next.is_some();
        p = next.unwrap_or_else(|| p.join(part));
    }
    p
}

/// The names in the folder `dir` with whether each is a folder (None: no folder that reads).
pub fn list_dir(dir: &Path) -> Option<Vec<(std::ffi::OsString, bool)>> {
    let entries = std::fs::read_dir(dir).ok()?;
    Some(entries.flatten().map(|e| (e.file_name(), e.path().is_dir())).collect())
}

/// The paths of what is in the folder `dir` (empty: none, or no folder).
pub fn read_dir_paths(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default()
}

/// The content folders, the content folder first, then the OMSI 2 folder (empty when the
/// OMSI 2 folder is not set).
pub fn bases() -> Vec<PathBuf> {
    vfs().map(|v| v.bases().to_vec()).unwrap_or_default()
}

/// Every folder of `rel` (`Vehicles`) across the content folders, by its name (in any case,
/// the first folder's spelling), with each content folder's copy of it: the content folder's
/// first. Sorted by name.
pub fn merged_folders(rel: &str) -> Vec<(String, Vec<PathBuf>)> {
    let bases = bases();
    let mut out: Vec<(String, Vec<PathBuf>)> = Vec::new();
    for b in &bases {
        let Some(dir) = hub_omsi::vfs::resolve_in(b, rel).filter(|d| d.is_dir()) else { continue };
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for e in entries.flatten().filter(|e| e.path().is_dir()) {
            let name = e.file_name().to_string_lossy().into_owned();
            match out.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&name)) {
                Some((_, copies)) => copies.push(e.path()),
                None => out.push((name, vec![e.path()])),
            }
        }
    }
    out.sort_by_key(|(n, _)| n.to_lowercase());
    out
}

/// The OMSI 2 content as openOMSI sees it: the content folder first, then the OMSI 2 folder.
pub fn vfs() -> Option<hub_omsi::vfs::Vfs> {
    Some(hub_omsi::vfs::Vfs::new(root()?, content_dir()))
}

/// A content file by its path relative to the OMSI 2 folder (`maps/Grundorf/global.cfg`,
/// either slash, any case), found in the content folder first, then in the OMSI 2 folder.
pub fn resolve_content(rel: &str) -> Option<PathBuf> {
    vfs()?.resolve(rel)
}

/// The OMSI 2 folder of this computer, for the tests that read real content (only read; they
/// pass over what is not there): `OMSI2_DIR`, else Steam's own place - on Windows, and on Linux
/// where Steam (native or Flatpak) runs OMSI 2 through Proton.
#[cfg(test)]
pub(crate) fn installed_omsi2() -> Option<PathBuf> {
    let home = home_dir();
    let steam = |base: PathBuf| base.join("steamapps").join("common").join("OMSI 2");
    let mut tries: Vec<PathBuf> = std::env::var_os("OMSI2_DIR").map(PathBuf::from).into_iter().collect();
    if cfg!(windows) {
        tries.push(steam(PathBuf::from("C:/Program Files (x86)/Steam")));
    } else {
        tries.push(steam(home.join(".steam").join("steam")));
        tries.push(steam(home.join(".local").join("share").join("Steam")));
        tries.push(steam(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")));
    }
    tries.into_iter().find(|p| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hub-core-paths-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn openomsi_s_content_folder_follows_its_rule_on_every_system() {
        let home = dir("home");
        // a Linux release unpacked in the home folder: its own folder
        let linux = home.join("openOMSI-0.2.30-linux-x64");
        std::fs::create_dir_all(&linux).unwrap();
        std::fs::write(linux.join("openomsi"), b"").unwrap();
        assert_eq!(content_dir_of(&linux, &home), Some(linux.clone()));
        // once openOMSI went to ~/.openomsi/content (its folder could not be written), that one
        std::fs::create_dir_all(home.join(".openomsi/content/Plugins")).unwrap();
        assert_eq!(content_dir_of(&linux, &home), Some(home.join(".openomsi/content")));
        // ... unless it used its own folder
        std::fs::create_dir_all(linux.join("Plugins")).unwrap();
        assert_eq!(content_dir_of(&linux, &home), Some(linux.clone()));
        // inside the OMSI 2 folder: its openOMSI folder
        let omsi2 = dir("omsi2");
        std::fs::write(omsi2.join("Omsi.exe"), b"").unwrap();
        std::fs::write(omsi2.join("openomsi.exe"), b"").unwrap();
        std::fs::create_dir_all(omsi2.join("openOMSI/Mods")).unwrap();
        assert_eq!(content_dir_of(&omsi2, &home), Some(omsi2.join("openOMSI")));
        // no openOMSI at all
        assert_eq!(content_dir_of(&home, &home), None);
    }

    #[test]
    fn the_data_folder_is_omsi_hub_s_own() {
        let d = system_data_dir();
        assert_eq!(d.file_name().unwrap(), APP_FOLDER);
        assert!(!d.to_string_lossy().contains(".openomsi"));
        if cfg!(target_os = "linux") {
            // ($XDG_DATA_HOME, else ~/.local/share)
            let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home_dir().join(".local/share"));
            assert_eq!(d, xdg.join(APP_FOLDER));
        }
    }

    /// A path is found in any case as far as it is there; the rest is as asked, one folder a
    /// part whichever slash it was written with (no `\` in a name on Linux and macOS).
    #[test]
    fn a_path_keeps_the_case_of_what_is_there() {
        let base = std::env::temp_dir().join(format!("omsihub-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("vehicles").join("MAN_SD202")).unwrap();
        std::fs::write(base.join("vehicles").join("MAN_SD202").join("SD202.BUS"), "").unwrap();
        assert_eq!(resolve_path(&base, "Vehicles\\man_sd202\\sd202.bus"), base.join("vehicles").join("MAN_SD202").join("SD202.BUS"));
        assert_eq!(resolve_path(&base, "VEHICLES/New/Folder"), base.join("vehicles").join("New").join("Folder"));
        assert_eq!(resolve_path(&base, "maps\\Grundorf/TTData"), base.join("maps").join("Grundorf").join("TTData"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
