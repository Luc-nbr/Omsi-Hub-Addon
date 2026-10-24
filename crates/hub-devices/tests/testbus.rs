//! The whole pipeline on the synthetic bus: devices, the form, its pictures and its font.

use hub_devices::fonts::FontBook;
use hub_devices::form::{self, Input, Models};
use hub_devices::modules::{self, Role};
use hub_devices::protocol::Kind;
use hub_devices::textures::{self, Finder};
use hub_devices::{layout, testbus};
use std::collections::HashMap;

#[test]
fn the_test_bus_has_an_ibis_with_a_touch_screen_and_a_printer_of_keys() {
    let root = std::env::temp_dir().join(format!("hub-devices-testbus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = testbus::write(&root).unwrap();
    let devices = modules::modules_of(&cfg);
    let ibis = devices.iter().find(|m| m.role() == Role::Ibis).expect("an IBIS");
    let printer = devices.iter().find(|m| m.role() == Role::Printer).expect("a ticket printer");
    assert_eq!(devices.len(), 2, "{devices:#?}");
    assert_eq!(ibis.fields.len(), 2);
    assert_eq!(ibis.keys.len(), 14, "two touch fields and twelve keys");
    assert_eq!(printer.keys.len(), 6);
    assert!(printer.fields.is_empty());

    let models = Models::default();
    let needed = form::needed_numbers(&models, &cfg, ibis);
    for n in ["IBIS_mode", "IBIS_stop", "trans_dauer"] {
        assert!(needed.contains(&n.to_string()), "{n}: {needed:?}");
    }

    let finder = Finder::default();
    let book = FontBook::default();
    let fonts = std::cell::RefCell::new(HashMap::new());
    let font = |name: &str, full: bool| book.font(&root, name, full, &mut fonts.borrow_mut());
    let find = |name: &str| finder.find(&cfg, &root, name);
    let numbers: HashMap<String, f64> = [("IBIS_mode".to_string(), 0.0), ("IBIS_stop".to_string(), 0.0)].into();
    let input = Input { model_cfg: &cfg, omsi_dir: &root, module: ibis, numbers: &numbers, unknown: &[], font: &font, find_texture: &find };
    let built = form::build(&models, &input).expect("the IBIS has a screen");
    let f = &built.form;
    assert_eq!(f.id.len(), 20);
    assert!((f.aspect - 2.0).abs() < 1e-3, "120 x 60 mm: {}", f.aspect);
    let kinds: Vec<Kind> = f.parts.iter().map(|p| p.kind).collect();
    assert_eq!(kinds, [Kind::Image, Kind::Image, Kind::Image, Kind::Text, Kind::Text], "{:#?}", f.parts);
    // the LCD first (no [visible]), then its menu page shown at IBIS_mode 1
    assert!(f.parts[0].visible.is_empty());
    assert_eq!(f.parts[1].visible.len(), 1);
    assert_eq!(f.nums[f.parts[1].visible[0].num], "IBIS_mode");
    // the icon's [matl_change]
    let choice = f.parts[2].choice.as_ref().expect("a choice");
    assert_eq!(f.nums[choice.num], "IBIS_stop");
    assert!(choice.items[0].is_some() && choice.items[0] != f.parts[2].texture);
    // the texts: top line first in the cfg, its triangles at the top of the screen
    let line = f.parts[3].text.as_ref().unwrap();
    assert_eq!((f.strs[line.str].as_str(), line.font.as_str(), line.color), ("IBIS_line_text", "TestFont", [255, 160, 0]));
    assert!(f.parts[3].tris.chunks(4).all(|p| p[1] < 0.34), "{:?}", f.parts[3].tris);
    assert!((f.parts[3].depth - 0.5).abs() < 1e-3);
    assert_eq!(f.parts[4].text.as_ref().unwrap().align, 1);
    // s runs left to right: the line's text starts near the left
    assert!(f.parts[3].tris.chunks(4).any(|p| p[0] < 0.05));
    assert_eq!(f.fonts["TestFont"].glyphs.len(), 41);
    // the touch fields, and the keypad off the screen stays keys of the device
    let taps: Vec<&str> = f.taps.iter().map(|t| t.trigger.as_str()).collect();
    assert_eq!(taps.len(), 2);
    assert!(taps.contains(&"IBIS_menu") && taps.contains(&"IBIS_ok"));
    let ok = f.taps.iter().find(|t| t.trigger == "IBIS_ok").unwrap();
    assert!(ok.x > 0.6 && ok.y > 0.69 && ok.w > 0.2 && ok.visible.len() == 1, "{ok:?}");
    let on_device = built.keys_on_device();
    assert!(on_device.contains("ibis_7") && on_device.contains("ibis_loeschen"), "{:?}", built.loose);
    // (the plugin gave no trans_dauer: the script's start value put IBIS_menu on the screen)
    assert_eq!(f.incomplete, ["place from the script's start value, trans_dauer = 1: TB_ibis_touch_menu.o3d"]);
    // the pictures, as PNGs
    assert_eq!(built.textures.len(), 4, "{:?}", built.textures);
    for (id, path, flags) in &built.textures {
        assert_eq!(&textures::id_of(path, flags), id);
        let png = textures::png_of(path, flags, textures::LIMIT).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }
    // the same input, the same id
    let again = form::build(&Models::default(), &input).unwrap();
    assert_eq!(again.form.id, f.id);

    // a bus that does not know trans_dauer: 0, never a script's value, and IBIS_menu falls off
    let unknown = ["trans_dauer".to_string()];
    let off = form::build(&Models::default(), &Input { unknown: &unknown, ..input }).unwrap();
    assert_eq!(off.form.taps.len(), 1);
    // (more than a metre off the screen: not even a loose key of it)
    assert!(!off.loose.iter().any(|k| k.actions == ["IBIS_menu"]), "{:?}", off.loose);
    let live: HashMap<String, f64> = [("trans_dauer".to_string(), 1.0)].into();
    let given = form::build(&Models::default(), &Input { numbers: &live, ..input }).unwrap();
    assert!(given.form.incomplete.is_empty() && given.form.taps.len() == 2, "{:?}", given.form.incomplete);

    // the printer has no screen: its keys only, in rows
    let input = Input { module: printer, ..input };
    assert!(form::build(&models, &input).is_none());
    let rows = layout::rows(&printer.keys, |k| k.at);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let _ = std::fs::remove_dir_all(root);
}
