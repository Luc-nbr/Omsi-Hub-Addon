//! A note on a sheet: text written on a small paper that lies in the same clip as a larger sheet,
//! each shown on its own (the HH20's note of the driver's numbers, `19_zettel.o3d`, on its
//! timetable sheet `19_schedule.o3d`). The text's surface is the paper shown with it.

use hub_devices::fonts::FontBook;
use hub_devices::form::{self, Input, Models};
use hub_devices::textures::Finder;
use hub_devices::{modules, o3d, testbus};
use std::collections::HashMap;

/// A rectangle facing the driver (-z), its top edge `tilt` metres further back than its bottom.
fn sheet(x0: f32, x1: f32, y0: f32, y1: f32, z: f32, tilt: f32, texture: &str) -> Vec<u8> {
    let v = [[x0, y1, z + tilt], [x1, y1, z + tilt], [x1, y0, z], [x0, y0, z]];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    o3d::write(&v, &uv, &[([0, 1, 2], 0), ([0, 2, 3], 0)], &[([1.0, 1.0, 1.0, 1.0], texture)])
}

#[test]
fn text_on_a_note_lies_on_the_note_not_on_the_hidden_sheet_beside_it() {
    let root = std::env::temp_dir().join(format!("hub-devices-note-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let model = root.join("Vehicles").join("NoteBus").join("Model");
    let texture = model.parent().unwrap().join("Texture");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::create_dir_all(&texture).unwrap();
    testbus::write_test_font(&root).unwrap();
    std::fs::write(texture.join("schedule.bmp"), testbus::bmp24(64, 64, |_, _| [240, 240, 255])).unwrap();
    std::fs::write(texture.join("note.bmp"), testbus::bmp24(64, 48, |_, _| [255, 250, 220])).unwrap();
    // the sheet 13 x 10 cm; the note 13 x 7 cm on it, its top edge 0.8 mm behind the sheet
    let files = [
        ("NB_body.o3d", sheet(-1.2, 1.2, 0.0, 2.8, 6.0, 0.0, "body.bmp")),
        ("NB_schedule.o3d", sheet(-0.33, -0.20, 1.58, 1.68, 0.5, 0.0, "schedule.bmp")),
        ("NB_zettel.o3d", sheet(-0.33, -0.20, 1.60, 1.67, 0.5, 0.0008, "note.bmp")),
        ("NB_zettel_s1.o3d", sheet(-0.30, -0.23, 1.62, 1.635, 0.4996, 0.0, "hand.bmp")),
    ];
    for (name, bytes) in &files {
        std::fs::write(model.join(name), bytes).unwrap();
    }
    let cfg = "[mesh]\r\nNB_body.o3d\r\n\r\n\
               [mesh]\r\nNB_schedule.o3d\r\n[visible]\r\nschedule_vis\r\n1\r\n\r\n\
               [mesh]\r\nNB_zettel.o3d\r\n[visible]\r\nnote_vis\r\n1\r\n\r\n\
               [mesh]\r\nNB_zettel_s1.o3d\r\n[visible]\r\nnote_vis\r\n1\r\n[matl]\r\nhand.bmp\r\n0\r\n[matl_alpha]\r\n2\r\n[useTextTexture]\r\n0\r\n\r\n\
               [texttexture]\r\nnote_pin\r\nTestFont\r\n128\r\n16\r\n0\r\n20\r\n20\r\n80\r\n\r\n";
    let cfg_path = model.join("model.cfg");
    std::fs::write(&cfg_path, cfg).unwrap();

    let devices = modules::modules_of(&cfg_path);
    let note = devices.iter().find(|m| m.fields.iter().any(|f| f.field.variable == "note_pin")).expect("the note");
    let finder = Finder::default();
    let book = FontBook::default();
    let fonts = std::cell::RefCell::new(HashMap::new());
    let font = |name: &str, full: bool| book.font(&root, name, full, &mut fonts.borrow_mut());
    let find = |name: &str| finder.find(&cfg_path, &root, name);
    let numbers: HashMap<String, f64> = [("schedule_vis".to_string(), 0.0), ("note_vis".to_string(), 1.0)].into();
    let input = Input { model_cfg: &cfg_path, omsi_dir: &root, module: note, numbers: &numbers, unknown: &[], font: &font, find_texture: &find };
    let f = form::build(&Models::default(), &input).expect("a screen").form;
    // the paper is the background, shown with the text; the sheet (earlier in the cfg) is not
    let image = f.parts.iter().find(|p| p.texture.is_some()).expect("a picture");
    assert_eq!(image.visible.len(), 1, "{:#?}", f.parts);
    assert_eq!(f.nums[image.visible[0].num], "note_vis");
    assert!((f.aspect - 13.0 / 7.0).abs() < 0.05, "the note's 13 x 7 cm: {}", f.aspect);
    let _ = std::fs::remove_dir_all(root);
}
