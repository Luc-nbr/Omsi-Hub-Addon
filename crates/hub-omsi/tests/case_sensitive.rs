//! OMSI 2's content names its files carelessly (`Vehicles/MAN_SD202/sd202.bus` named as
//! `vehicles/man_sd202/SD202.bus` in a `.bus` or a map), and Windows does not mind. Linux's file
//! systems do: there the content folders must be searched in any case by `hub_omsi::vfs`.
//!
//! This test only proves that on a case-sensitive file system (Linux, and macOS with a
//! case-sensitive volume). Elsewhere it says so and passes; with
//! `OMSIHUB_REQUIRE_CASE_SENSITIVE=1` (the Linux CI job) a case-insensitive temp folder fails it
//! instead, so the proof cannot be skipped there unseen.

use hub_omsi::vfs::{find_child, resolve_in, Vfs};
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-omsi-case-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Whether the file system of `d` tells names apart by their case.
fn case_sensitive(d: &std::path::Path) -> bool {
    std::fs::write(d.join("probe"), b"").unwrap();
    let sensitive = !d.join("PROBE").exists();
    std::fs::remove_file(d.join("probe")).unwrap();
    sensitive
}

fn write(p: PathBuf, text: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

#[test]
fn content_is_found_in_any_case_on_a_case_sensitive_file_system() {
    let root = dir("root");
    let content = dir("content");
    if !case_sensitive(&root) {
        assert!(std::env::var_os("OMSIHUB_REQUIRE_CASE_SENSITIVE").filter(|v| !v.is_empty() && v != "0").is_none(), "the temp folder is not case-sensitive: this test proves nothing here");
        eprintln!("(the temp folder is case-insensitive: nothing to prove here)");
        return;
    }
    write(root.join("Maps/Grundorf/Global.CFG"), "[name]\nroot");
    write(root.join("Maps/Grundorf/TTData/Linie_10.ttl"), "[userallowed]\n1");
    write(root.join("Vehicles/MAN_SD202/sd202.bus"), "[friendlyname]\nMAN\nSD202\n");
    write(root.join("Vehicles/MAN_SD202/Texture/front.dds"), "");
    write(content.join("maps/grundorf/global.cfg"), "[name]\ncontent");
    write(content.join("vehicles/man_sd202/texture/back.dds"), "");

    // every part of a path in any case, either slash
    assert!(resolve_in(&root, "vehicles\\man_sd202\\SD202.BUS").is_some_and(|p| p.ends_with("Vehicles/MAN_SD202/sd202.bus")));
    assert_eq!(find_child(&root.join("Maps"), "grundorf"), Some(root.join("Maps/Grundorf")));
    assert_eq!(resolve_in(&root, "maps/grundorf/missing.cfg"), None);

    let vfs = Vfs::new(root.clone(), Some(content.clone()));
    // the content folder first, whatever case each folder has it in
    assert_eq!(vfs.read_cfg("MAPS/GRUNDORF/GLOBAL.CFG").unwrap().value("name"), Some("content"));
    assert!(vfs.is_file("maps/grundorf/ttdata/LINIE_10.TTL"));
    assert!(vfs.is_dir("VEHICLES/man_sd202"));
    // a folder listed from both, each name once
    let names: Vec<String> = vfs.list_dir("vehicles/MAN_SD202/TEXTURE").into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["back.dds", "front.dds"]);

    // two names that differ only in case: the exact one wins
    write(root.join("Maps/Grundorf/a.cfg"), "[name]\nlower");
    write(root.join("Maps/Grundorf/A.cfg"), "[name]\nupper");
    let only_root = Vfs::new(root.clone(), None);
    assert_eq!(only_root.read_cfg("Maps/Grundorf/A.cfg").unwrap().value("name"), Some("upper"));
    assert_eq!(only_root.read_cfg("Maps/Grundorf/a.cfg").unwrap().value("name"), Some("lower"));

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&content);
}
