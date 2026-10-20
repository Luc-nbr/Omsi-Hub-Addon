//! Omsi-Hub and the player's own openOMSI: everything the app needs to find the game, start it
//! and talk to the plugin that runs inside it. Nothing here uses openOMSI's code: the game is a
//! program in a folder, its command line is the official one, and the plugin talks through
//! plain files.
//!
//! * [`detect`] - an openOMSI folder the player chose: its program, its version, its plugins
//!   folder (ported from the old `omsihub-plugin`);
//! * [`version`] - openOMSI's version numbers (ported);
//! * [`compat`] - which openOMSI the add-on works with and how well, the minimum being 0.2.22
//!   (ported);
//! * [`install`] - the plugin (`plugin/*.lua`) put into the game's plugins folder, switched on
//!   and off (ported);
//! * [`bridge`] - the files between Omsi-Hub and the plugin: the assignment, the handshake, the
//!   reports, with a fallback through the game's log (ported);
//! * [`controllers`] - the player's wheel, pedals, joysticks and button boxes
//!   (`Inputs/gamectrler.cfg`), and the key actions their buttons can press;
//! * [`find`] - folders to offer in the first-run setup (Steam's OMSI 2, unpacked releases);
//! * [`gamesettings`] - openOMSI's own settings (`~/.openomsi/settings.cfg`), changed from
//!   Omsi-Hub's Settings page;
//! * [`launch`] - the game's command line, built from openOMSI's official flags only;
//! * [`process`] - the game started, watched while it runs, and its end told; `--version` asked
//!   when nothing else said the version.

pub mod bridge;
pub mod compat;
pub mod controllers;
pub mod detect;
pub mod find;
pub mod gamesettings;
pub mod install;
pub mod launch;
pub mod process;
pub mod version;

pub use version::GameVersion;

/// What a folder holds, with its compatibility: the first-run setup's and the settings' check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Checked {
    pub install: detect::Install,
    pub compat: compat::Compat,
}

/// The openOMSI in `folder` checked: its program and version (`detect`), then how well Omsi-Hub
/// works with it (`compat`, from the plugin's handshake when there is one of this version).
/// `home` is the user's home folder, where openOMSI keeps its own content folder when its
/// program folder may not be written to.
pub fn check_folder(folder: &std::path::Path, home: &std::path::Path) -> Result<Checked, detect::Problem> {
    let install = detect::inspect(folder, home)?;
    let hello = bridge::read_hello(&install::plugin_dir(&install.plugins_dir));
    let compat = compat::check(install.version.as_ref(), hello.as_ref());
    Ok(Checked { install, compat })
}

/// [`check_folder`], with the version asked of the program itself (`openomsi --version`) when
/// no file says it - the usual case on Linux, whose program has no version resource and whose
/// folder may have any name. The answer is kept in `cache` (a JSON file of Omsi-Hub's data
/// folder) per program, with the program's size and change time, so it is asked once per
/// openOMSI version.
pub fn check_folder_probing(folder: &std::path::Path, home: &std::path::Path, cache: &std::path::Path) -> Result<Checked, detect::Problem> {
    let mut checked = check_folder(folder, home)?;
    if checked.install.version.is_none() {
        if let Some(v) = probed_version(&checked.install.exe, cache) {
            let hello = bridge::read_hello(&install::plugin_dir(&checked.install.plugins_dir));
            checked.compat = compat::check(Some(&v), hello.as_ref());
            checked.install.version = Some(v);
            checked.install.version_from = Some(detect::VersionFrom::Program);
        }
    }
    Ok(checked)
}

/// The program's version as it said it, from `cache` or asked now.
fn probed_version(exe: &std::path::Path, cache: &std::path::Path) -> Option<GameVersion> {
    use std::collections::BTreeMap;
    let meta = std::fs::metadata(exe).ok()?;
    let changed = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let stamp = format!("{}:{changed}", meta.len());
    let key = exe.to_string_lossy().into_owned();
    let mut kept: BTreeMap<String, (String, String)> = std::fs::read_to_string(cache).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    if let Some((s, v)) = kept.get(&key) {
        if *s == stamp {
            return GameVersion::find(v);
        }
    }
    let v = process::ask_version(exe, std::time::Duration::from_secs(5));
    kept.insert(key, (stamp, v.as_ref().map(|v| v.to_string()).unwrap_or_default()));
    let _ = std::fs::write(cache, serde_json::to_string_pretty(&kept).unwrap_or_default());
    v
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_version_no_file_says_is_asked_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("hub-game-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let game = dir.join("games/oo");
        std::fs::create_dir_all(&game).unwrap();
        // (a script in openOMSI's place, unpacked without its executable bit)
        let exe = game.join("openomsi");
        std::fs::write(&exe, "#!/bin/sh\necho 'openomsi 0.2.31'\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o644)).unwrap();
        let cache = dir.join("openomsi-versions.json");
        assert_eq!(check_folder(&game, &dir).unwrap().install.version, None);
        let c = check_folder_probing(&game, &dir, &cache).unwrap();
        assert_eq!((c.install.version, c.install.version_from), (Some(GameVersion::new(0, 2, 31)), Some(detect::VersionFrom::Program)));
        assert_eq!(c.compat.level, compat::Level::Supported);
        assert!(std::fs::read_to_string(&cache).unwrap().contains("0.2.31"));
        // (kept: the program is not asked again while it is the same file)
        let mut kept: std::collections::BTreeMap<String, (String, String)> = serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        kept.values_mut().for_each(|v| v.1 = "0.2.40".into());
        std::fs::write(&cache, serde_json::to_string(&kept).unwrap()).unwrap();
        assert_eq!(check_folder_probing(&game, &dir, &cache).unwrap().install.version, Some(GameVersion::new(0, 2, 40)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
