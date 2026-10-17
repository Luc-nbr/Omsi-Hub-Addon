//! The version a release is named after is the same everywhere it is written: the workspace's
//! (`Cargo.toml`, what `hub_version` answers), `app/tauri.conf.json`'s (the installers' and the
//! updater's), and a section of `CHANGELOG.md` (the release links there). The release workflow
//! refuses a difference; this says so before a release is built.

use std::path::Path;

fn repo(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn the_version_is_the_same_in_cargo_tauri_and_the_changelog() {
    let version = env!("CARGO_PKG_VERSION");
    let conf: serde_json::Value = serde_json::from_str(&repo("app/tauri.conf.json")).unwrap();
    assert_eq!(conf["version"], version, "app/tauri.conf.json's version is not the workspace's (Cargo.toml)");
    let changelog = repo("CHANGELOG.md");
    let heading = format!("## {version}");
    assert!(changelog.lines().any(|l| l == heading || l.starts_with(&format!("{heading} "))), "CHANGELOG.md has no section for {version}: write what is in it under \"{heading}\"");
}
