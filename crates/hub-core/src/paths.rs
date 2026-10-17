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

/// The OMSI 2 content as openOMSI sees it: the content folder first, then the OMSI 2 folder.
pub fn vfs() -> Option<hub_omsi::vfs::Vfs> {
    Some(hub_omsi::vfs::Vfs::new(root()?, content_dir()))
}

/// A content file by its path relative to the OMSI 2 folder (`maps/Grundorf/global.cfg`,
/// either slash, any case), found in the content folder first, then in the OMSI 2 folder.
pub fn resolve_content(rel: &str) -> Option<PathBuf> {
    vfs()?.resolve(rel)
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
}
