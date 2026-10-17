//! The buses' photos: one the page took (`co_photo_save`, a PNG as base64) is kept where
//! `co_photo` finds it, for that bus in that livery only; what is not a PNG is refused.

use base64::Engine as _;
use hub_api::{call, call_bytes, Quiet};
use serde_json::json;

#[test]
fn a_photo_taken_is_kept_for_its_bus_and_livery() {
    let base = std::env::temp_dir().join(format!("hub-api-photos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("OMSI 2");
    std::fs::create_dir_all(root.join("Vehicles/Bus")).unwrap();
    std::fs::write(root.join("Vehicles/Bus/a.bus"), b"[name]\n").unwrap();
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, base.join("data"));
    hub_core::settings::Settings { omsi2_folder: root.to_string_lossy().into_owned(), ..Default::default() }.save().unwrap();
    let bus = "Vehicles/Bus/a.bus";

    assert!(call_bytes("co_photo", &json!({ "bus": bus, "livery": "Red" }), &Quiet).is_err(), "none taken yet");
    let png = b"\x89PNG\r\n\x1a\n-the rest of the picture-".to_vec();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    assert_eq!(call("co_photo_save", &json!({ "bus": bus, "livery": "Red", "png": b64 }), &Quiet).unwrap(), json!({ "ok": true }));
    assert_eq!(call_bytes("co_photo", &json!({ "bus": bus, "livery": "Red" }), &Quiet).unwrap(), png);
    assert_eq!(call_bytes("co_photo", &json!({ "bus": "vehicles\\bus\\A.bus", "livery": "red" }), &Quiet).unwrap(), png, "the bus in any case, either slash");
    assert!(call_bytes("co_photo", &json!({ "bus": bus, "livery": "Blue" }), &Quiet).is_err(), "another livery's photo is not taken");

    let jpeg = base64::engine::general_purpose::STANDARD.encode(b"\xff\xd8\xff\xe0 not a png");
    for (args, why) in [
        (json!({ "bus": bus, "livery": "Blue", "png": jpeg }), "not a PNG"),
        (json!({ "bus": bus, "livery": "Blue", "png": "%%% no base64" }), "no base64"),
        (json!({ "bus": "", "livery": "Blue", "png": b64 }), "no bus"),
    ] {
        assert!(call("co_photo_save", &args, &Quiet).is_err(), "{why}");
    }
    assert!(call_bytes("co_photo", &json!({ "bus": bus, "livery": "Blue" }), &Quiet).is_err());
    let _ = std::fs::remove_dir_all(&base);
}
