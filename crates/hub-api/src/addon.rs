//! The plugin in the player's openOMSI (`addon_`), on `hub-game`:
//!
//! * `addon_state` - the chosen openOMSI, how well Omsi-Hub works with it, and the plugin there
//!   (missing, on, off, out of date);
//! * `addon_enable {on}` - the plugin installed (or brought up to date) and on, or parked off.
//!
//! **The plugin port** adds the rest of the old `omsihub/addon.rs`: the assignment written for a
//! drive (`addon_assign`), the plugin's texts in the player's language, and the reports booked
//! into the driver's record (`addon_reports`). Those answer [`crate::not_ported`] until then.

use crate::{arg, not_ported, to_json, Answer, Platform};
use hub_game::install;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The plugins folder of the chosen openOMSI.
fn plugins_dir() -> Result<PathBuf, String> {
    let s = hub_core::settings::Settings::load();
    if s.openomsi_folder.is_empty() {
        return Err("Choose your openOMSI folder first".into());
    }
    let c = hub_game::check_folder(Path::new(&s.openomsi_folder), &hub_core::paths::home_dir()).map_err(|p| p.text().to_string())?;
    Ok(c.install.plugins_dir)
}

/// An `addon_` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    match cmd {
        "addon_state" => {
            let s = hub_core::settings::Settings::load();
            let game = (!s.openomsi_folder.is_empty()).then(|| crate::hub::check_openomsi(&s.openomsi_folder));
            let plugin = plugins_dir().ok().map(|d| install::status(&d));
            Ok(json!({ "folder": s.openomsi_folder, "game": game, "plugin": plugin, "plugin_version": install::VERSION }))
        }
        "addon_enable" => {
            let dir = plugins_dir()?;
            let st = if arg::<bool>(args, "on")? { install::install(&dir) } else { install::disable(&dir) };
            let st = st.map_err(|e| format!("the plugin could not be changed: {e}"))?;
            hub_core::log_line(&format!("plugin: {:?} in {}", st.state, dir.display()));
            to_json(st)
        }
        _ => Err(not_ported(cmd)),
    }
}
