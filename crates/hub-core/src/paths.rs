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
    dirs::data_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join(APP_FOLDER)
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
    dirs::home_dir().unwrap_or_default()
}

/// The OMSI 2 folder from the settings, when it is set and there.
pub fn root() -> Option<PathBuf> {
    let s = crate::settings::Settings::load();
    Some(PathBuf::from(s.omsi2_folder)).filter(|p| !p.as_os_str().is_empty() && p.is_dir())
}

/// openOMSI's content folder for the chosen openOMSI (where it reads mods and its plugins
/// from), when the openOMSI folder is set: its program's folder, or `openOMSI` inside the OMSI 2
/// folder, or `~/.openomsi/content` (hub-game's `detect` rule).
pub fn content_dir() -> Option<PathBuf> {
    let s = crate::settings::Settings::load();
    if s.openomsi_folder.is_empty() {
        return None;
    }
    let exe = hub_omsi::vfs::find_child(Path::new(&s.openomsi_folder), "openomsi.exe").or_else(|| hub_omsi::vfs::find_child(Path::new(&s.openomsi_folder), "openomsi"))?;
    let dir = exe.parent()?.to_path_buf();
    let inside = dir.join("openOMSI");
    Some(if hub_omsi::vfs::find_child(&dir, "Omsi.exe").is_some() { inside } else { dir }).filter(|p| p.is_dir())
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

    #[test]
    fn the_data_folder_is_omsi_hub_s_own() {
        let d = system_data_dir();
        assert_eq!(d.file_name().unwrap(), APP_FOLDER);
        assert!(!d.to_string_lossy().contains(".openomsi"));
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
