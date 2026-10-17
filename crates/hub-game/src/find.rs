//! Folders to offer in the first-run setup: where OMSI 2 and openOMSI usually are on this
//! computer. Only looked at, never written to; the player still picks and the app checks
//! (`detect`, `hub_omsi::vfs::missing_omsi2_essentials`).

use std::path::{Path, PathBuf};

/// Steam's library folders: its own, and those its `libraryfolders.vdf` lists.
fn steam_libraries() -> Vec<PathBuf> {
    let mut steams: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(p) = std::env::var_os(var) {
            steams.push(PathBuf::from(p).join("Steam"));
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        steams.push(PathBuf::from(&home).join("Library/Application Support/Steam"));
        steams.push(PathBuf::from(&home).join(".steam/steam"));
    }
    let mut libs = steams.clone();
    for s in &steams {
        if let Ok(text) = std::fs::read_to_string(s.join("steamapps/libraryfolders.vdf")) {
            libs.extend(library_paths(&text));
        }
    }
    libs.dedup();
    libs
}

/// The `"path"` values of a `libraryfolders.vdf`.
pub fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("\"path\"")?.trim();
            Some(PathBuf::from(rest.trim_matches('"').replace("\\\\", "\\")))
        })
        .collect()
}

/// Folders that may be OMSI 2's (each one exists).
pub fn omsi2_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = steam_libraries().into_iter().map(|l| l.join("steamapps/common/OMSI 2")).collect();
    out.push(PathBuf::from("C:/OMSI 2"));
    out.retain(|p| p.is_dir());
    out.dedup();
    out
}

/// Folders that may hold openOMSI (each one with its program in it): the OMSI 2 folder and
/// its `openOMSI` folder, and the usual places the release archive is unpacked to.
pub fn openomsi_candidates(home: &Path, omsi2: Option<&Path>) -> Vec<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(r) = omsi2 {
        cands.push(r.to_path_buf());
        cands.push(r.join("openOMSI"));
    }
    for dir in [home.join("Downloads"), home.join("Desktop"), home.join("Games"), home.join("Applications"), PathBuf::from("/Applications")] {
        cands.push(dir.join("openOMSI"));
        // (a release archive unpacked: `openOMSI-0.2.30-windows-x64`)
        if let Ok(entries) = std::fs::read_dir(&dir) {
            let mut named: Vec<PathBuf> =
                entries.flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("openomsi"))).collect();
            named.sort();
            cands.extend(named.into_iter().rev());
        }
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for c in cands {
        if crate::detect::find_exe(&c).is_some() && !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_s_libraries_are_read() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}";
        assert_eq!(library_paths(vdf), [PathBuf::from("C:\\Program Files (x86)\\Steam"), PathBuf::from("D:\\SteamLibrary")]);
    }

    #[test]
    fn an_unpacked_release_is_offered() {
        let home = std::env::temp_dir().join(format!("hub-game-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let rel = home.join("Downloads/openOMSI-0.2.30-windows-x64");
        std::fs::create_dir_all(&rel).unwrap();
        std::fs::write(rel.join("openomsi.exe"), b"MZ").unwrap();
        std::fs::create_dir_all(home.join("Downloads/other")).unwrap();
        assert_eq!(openomsi_candidates(&home, None), [rel]);
        let _ = std::fs::remove_dir_all(&home);
    }
}
