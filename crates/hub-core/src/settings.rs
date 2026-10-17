//! What the first-run setup and the settings page keep: the player's folders and the app's own
//! choices, in `settings.json` of the data folder. A field a newer version adds reads as its
//! default from an older file; one this version does not know is kept as it is.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::PathBuf;

/// The file in the data folder.
pub const FILE: &str = "settings.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The openOMSI folder the player chose (its program is in it).
    pub openomsi_folder: String,
    /// The original OMSI 2 folder (`Omsi.exe`, `maps`, `Vehicles`).
    pub omsi2_folder: String,
    /// The app's language (`en`, `nl`, `de` …; empty: the system's).
    pub language: String,
    /// The first-run setup was finished.
    pub setup_done: bool,
    /// The old Omsi-Hub's saves were imported (or the player said no): asked once.
    pub migrated: bool,
    /// Whatever else is in the file (a newer version's fields), kept.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl Settings {
    pub fn path() -> PathBuf {
        crate::paths::data_dir().join(FILE)
    }

    /// The settings as saved (the defaults when there is no file, or it does not read).
    pub fn load() -> Settings {
        std::fs::read_to_string(Self::path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    /// Saved, through a temporary file so a crash never leaves half a file.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).unwrap_or_default())?;
        std::fs::rename(&tmp, &path)
    }

    /// Both folders are chosen.
    pub fn has_folders(&self) -> bool {
        !self.openomsi_folder.is_empty() && !self.omsi2_folder.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_field_is_kept() {
        let s: Settings = serde_json::from_str(r#"{"omsi2_folder":"C:/OMSI 2","future":7}"#).unwrap();
        assert_eq!(s.omsi2_folder, "C:/OMSI 2");
        assert!(!s.has_folders());
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["future"], 7);
    }
}
