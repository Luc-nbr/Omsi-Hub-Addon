//! Folders to offer in the first-run setup: where OMSI 2 and openOMSI usually are on this
//! computer. Only looked at, never written to; the player still picks and the app checks
//! (`detect`, `hub_omsi::vfs::missing_omsi2_essentials`).

use std::path::{Path, PathBuf};

/// Where Steam may be installed: Windows' Program Files; macOS's Application Support; on Linux
/// `~/.steam/steam` (`~/.steam/root`), `~/.local/share/Steam` and the Flatpak's
/// `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`. OMSI 2 runs there through Proton.
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(p) = std::env::var_os(var) {
            roots.push(PathBuf::from(p).join("Steam"));
        }
    }
    for rel in ["Library/Application Support/Steam", ".steam/steam", ".steam/root", ".local/share/Steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam"] {
        roots.push(home.join(rel));
    }
    roots
}

/// Steam's library folders under `home`: each Steam's own, and those its
/// `libraryfolders.vdf` lists (`steamapps/`, or `config/` on newer installs).
pub fn steam_libraries(home: &Path) -> Vec<PathBuf> {
    let roots = steam_roots(home);
    let mut libs = roots.clone();
    for s in &roots {
        for vdf in ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"] {
            if let Ok(text) = std::fs::read_to_string(s.join(vdf)) {
                libs.extend(library_paths(&text));
            }
        }
    }
    unique(libs)
}

/// The list without repeats, in its order.
fn unique(list: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in list {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// The user's home folder, from the environment (`HOME`, else `USERPROFILE`).
fn home() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default()
}

/// A Linux desktop's own name for one of the user's folders (`DOWNLOAD`, `DESKTOP`), from
/// `~/.config/user-dirs.dirs` (`XDG_DOWNLOAD_DIR="$HOME/Downloads"`): it is translated on
/// many desktops (`~/Téléchargements`).
pub fn xdg_user_dir(home: &Path, name: &str) -> Option<PathBuf> {
    let text = std::fs::read_to_string(home.join(".config/user-dirs.dirs")).ok()?;
    let key = format!("XDG_{name}_DIR=");
    let value = text.lines().find_map(|l| l.trim().strip_prefix(&key))?.trim().trim_matches('"');
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(value),
    };
    Some(path).filter(|p| p.is_absolute())
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
    omsi2_candidates_in(&home())
}

/// The same for the user whose home folder is `home`.
pub fn omsi2_candidates_in(home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = steam_libraries(home).into_iter().map(|l| l.join("steamapps/common/OMSI 2")).collect();
    if cfg!(windows) {
        out.push(PathBuf::from("C:/OMSI 2"));
    }
    out.retain(|p| p.is_dir());
    unique(out)
}

/// Folders that may hold openOMSI (each one with its program in it): the OMSI 2 folder and
/// its `openOMSI` folder, and the usual places the release archive is unpacked to: the home
/// folder itself, Downloads (the desktop's own name for it on Linux), Desktop, Games,
/// `~/Applications`, `~/.local/share`, `/Applications` and `/opt` - each one's `openOMSI`
/// and every `openOMSI*` folder in it (`openOMSI-0.2.30-linux-x64`).
pub fn openomsi_candidates(home: &Path, omsi2: Option<&Path>) -> Vec<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(r) = omsi2 {
        cands.push(r.to_path_buf());
        cands.push(r.join("openOMSI"));
    }
    let mut dirs = vec![home.to_path_buf()];
    dirs.extend(xdg_user_dir(home, "DOWNLOAD"));
    dirs.extend(xdg_user_dir(home, "DESKTOP"));
    for rel in ["Downloads", "Desktop", "Games", "Applications", ".local/share"] {
        dirs.push(home.join(rel));
    }
    dirs.push(PathBuf::from("/Applications"));
    dirs.push(PathBuf::from("/opt"));
    for dir in unique(dirs) {
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
        // (first: what the computer itself has in /opt or /Applications comes after the home's)
        assert_eq!(openomsi_candidates(&home, None).first(), Some(&rel));
        let _ = std::fs::remove_dir_all(&home);
    }

    fn fake_home(tag: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("hub-game-find-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        home
    }

    #[test]
    fn linux_releases_are_found_where_players_unpack_them() {
        let home = fake_home("linux");
        // a translated Downloads folder (user-dirs.dirs), ~/.local/share, ~/openOMSI-…
        std::fs::create_dir_all(home.join(".config")).unwrap();
        std::fs::write(home.join(".config/user-dirs.dirs"), "# written by xdg-user-dirs-update\nXDG_DOWNLOAD_DIR=\"$HOME/Téléchargements\"\nXDG_DESKTOP_DIR=\"$HOME/Bureau\"\n").unwrap();
        assert_eq!(xdg_user_dir(&home, "DOWNLOAD"), Some(home.join("Téléchargements")));
        let unpacked = home.join("Téléchargements/openOMSI-0.2.30-linux-x64");
        let shared = home.join(".local/share/openOMSI");
        let own = home.join("openOMSI-0.2.31-linux-x64");
        for d in [&unpacked, &shared, &own] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("openomsi"), b"\x7fELF").unwrap();
        }
        let found = openomsi_candidates(&home, None);
        for d in [&unpacked, &shared, &own] {
            assert!(found.contains(d), "{} not in {found:?}", d.display());
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn omsi2_is_found_in_linux_s_steam_and_its_flatpak_and_extra_libraries() {
        let home = fake_home("steam");
        let native = home.join(".local/share/Steam/steamapps/common/OMSI 2");
        let flatpak = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/OMSI 2");
        let extra_lib = home.join("games/SteamLibrary");
        let extra = extra_lib.join("steamapps/common/OMSI 2");
        for d in [&native, &flatpak, &extra] {
            std::fs::create_dir_all(d).unwrap();
        }
        // (a Windows path's backslashes are doubled in the file)
        let vdf = format!("\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n", extra_lib.to_string_lossy().replace('\\', "\\\\"));
        std::fs::write(home.join(".local/share/Steam/steamapps/libraryfolders.vdf"), vdf).unwrap();
        let found = omsi2_candidates_in(&home);
        for d in [&native, &flatpak, &extra] {
            assert!(found.contains(d), "{} not in {found:?}", d.display());
        }
        let _ = std::fs::remove_dir_all(&home);
    }
}
