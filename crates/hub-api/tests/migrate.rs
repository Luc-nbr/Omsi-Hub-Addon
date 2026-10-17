//! The old Omsi-Hub's saves through the first-run setup's commands, from a FIXTURE of the old
//! `~/.openomsi` (`fixtures/old-openomsi`, pointed at with `OMSIHUB_OLD_DATA_DIR`): offered once
//! with what was found, copied into a temporary data folder (`OMSIHUB_DATA_DIR`) with the
//! settings we still use taken over, and the fixture left exactly as it was.

use hub_api::{call, Quiet};
use serde_json::json;
use std::path::{Path, PathBuf};

fn files(d: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(d).unwrap().flatten() {
        if e.path().is_dir() {
            out.extend(files(&e.path()));
        } else {
            out.push((e.path(), std::fs::read(e.path()).unwrap()));
        }
    }
    out.sort();
    out
}

#[test]
fn the_old_saves_are_offered_once_and_copied() {
    let old = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/old-openomsi").canonicalize().unwrap();
    let data = std::env::temp_dir().join(format!("hub-api-migrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, &data);
    std::env::set_var(hub_core::migrate::OLD_FOLDER_ENV, &old);
    let before = files(&old);

    // offered, with what was found
    let st = call("hub_setup_state", &json!({}), &Quiet).unwrap();
    let m = &st["migrate"];
    assert_eq!(m["offer"], true, "{st}");
    assert_eq!(m["preview"]["companies"], json!([["stadtbus", "Stadtbus Grundorf"]]));
    assert_eq!((m["preview"]["careers"].clone(), m["preview"]["lines"].clone(), m["preview"]["depots"].clone()), (json!(["luc"]), json!(["Grundorf"]), json!(["oo_Grundorf"])));
    assert_eq!((m["preview"]["language"].as_str(), m["preview"]["driver"].as_str()), (Some("de"), Some("Luc")));
    let scan = call("hub_migrate_scan", &json!({}), &Quiet).unwrap();
    assert_eq!(scan["preview"], m["preview"]);

    // copied: the companies read, the settings taken over, the driver set
    let done = call("hub_migrate_import", &json!({}), &Quiet).unwrap();
    assert_eq!(done["imported"], 11, "{done}");
    let s = &done["settings"];
    assert_eq!((s["migrated"].as_bool(), s["language"].as_str(), s["driver"].as_str()), (Some(true), Some("de"), Some("Luc")), "{s}");
    assert_eq!((s["accent"].as_str(), s["dark_mode"].as_bool(), s["company_layout"].as_str(), s["bus_lead"].as_i64()), (Some("#2a75f7"), Some(true), Some("palette"), Some(8)), "{s}");
    assert!(s.get("msaa").is_none() && s.get("fullscreen").is_none(), "openOMSI's keys stay in its file: {s}");
    assert!(data.join("companies/stadtbus.json").is_file() && data.join("trips/luc.jsonl").is_file() && !data.join("game.log").exists());
    assert_eq!(hub_core::company::store::list(&data, "Luc").len(), 1);
    assert_eq!(hub_api::launcher::driver(), "Luc");

    // offered once
    assert_eq!(call("hub_setup_state", &json!({}), &Quiet).unwrap()["migrate"]["offer"], false);
    // the old folder as it was
    assert_eq!(files(&old), before);
    let _ = std::fs::remove_dir_all(&data);
}
