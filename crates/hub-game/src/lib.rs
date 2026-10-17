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
//! * [`find`] - folders to offer in the first-run setup (Steam's OMSI 2, unpacked releases);
//! * [`launch`] - the game's command line, built from openOMSI's official flags only;
//! * [`process`] - the game started, watched while it runs, and its end told; `--version` asked
//!   when nothing else said the version.

pub mod bridge;
pub mod compat;
pub mod detect;
pub mod find;
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
