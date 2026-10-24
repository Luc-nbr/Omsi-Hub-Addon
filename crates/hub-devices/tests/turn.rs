//! A device mounted a little askew: its face is drawn level, as its picture lies on it (the MAN
//! SD80's Krüger IBIS sits a few degrees turned on the dashboard).

use hub_devices::fonts::FontBook;
use hub_devices::form::{self, Input, Models};
use hub_devices::textures::Finder;
use hub_devices::{modules, o3d, testbus};
use std::collections::HashMap;

/// A rectangle facing the driver (-z), turned by `degrees` about its middle in its own plane.
fn turned(x0: f32, x1: f32, y0: f32, y1: f32, z: f32, degrees: f32, texture: &str) -> Vec<u8> {
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (s, c) = degrees.to_radians().sin_cos();
    let at = |x: f32, y: f32| [mx + (x - mx) * c - (y - my) * s, my + (x - mx) * s + (y - my) * c, z];
    let v = [at(x0, y1), at(x1, y1), at(x1, y0), at(x0, y0)];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    o3d::write(&v, &uv, &[([0, 1, 2], 0), ([0, 2, 3], 0)], &[([1.0, 1.0, 1.0, 1.0], texture)])
}

#[test]
fn a_face_turned_a_little_is_drawn_level() {
    let root = std::env::temp_dir().join(format!("hub-devices-turn-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let model = root.join("Vehicles").join("TurnBus").join("Model");
    let texture = model.parent().unwrap().join("Texture");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::create_dir_all(&texture).unwrap();
    testbus::write_test_font(&root).unwrap();
    std::fs::write(texture.join("face.bmp"), testbus::bmp24(64, 32, |_, _| [40, 40, 40])).unwrap();
    // the face 16 x 8 cm turned by 4 degrees, its text field on it, turned with it
    let files = [
        ("TU_body.o3d", turned(-1.2, 1.2, 0.0, 2.8, 6.0, 0.0, "body.bmp")),
        ("TU_ibis_face.o3d", turned(-0.08, 0.08, 1.16, 1.24, 0.5, 4.0, "face.bmp")),
        ("TU_ibis_text.o3d", turned(-0.06, 0.02, 1.215, 1.23, 0.4995, 4.0, "text.bmp")),
    ];
    for (name, bytes) in &files {
        std::fs::write(model.join(name), bytes).unwrap();
    }
    let cfg = "[mesh]\r\nTU_body.o3d\r\n\r\n[mesh]\r\nTU_ibis_face.o3d\r\n\r\n\
               [mesh]\r\nTU_ibis_text.o3d\r\n[matl]\r\ntext.bmp\r\n0\r\n[matl_alpha]\r\n1\r\n[useTextTexture]\r\n0\r\n\r\n\
               [texttexture]\r\nIBIS_line_text\r\nTestFont\r\n128\r\n16\r\n0\r\n255\r\n160\r\n0\r\n\r\n";
    let cfg_path = model.join("model.cfg");
    std::fs::write(&cfg_path, cfg).unwrap();

    let devices = modules::modules_of(&cfg_path);
    let ibis = devices.iter().find(|m| !m.fields.is_empty()).expect("the IBIS");
    let finder = Finder::default();
    let book = FontBook::default();
    let fonts = std::cell::RefCell::new(HashMap::new());
    let font = |name: &str, full: bool| book.font(&root, name, full, &mut fonts.borrow_mut());
    let find = |name: &str| finder.find(&cfg_path, &root, name);
    let input = Input { model_cfg: &cfg_path, omsi_dir: &root, module: ibis, numbers: &HashMap::new(), unknown: &[], font: &font, find_texture: &find };
    let f = form::build(&Models::default(), &input).expect("a screen").form;
    // the face fills the screen, corner to corner, and has its own 2:1
    let face = f.parts.iter().find(|p| p.texture.is_some()).expect("the face");
    for p in face.tris.chunks(4) {
        for x in &p[..2] {
            assert!(x.abs() < 0.005 || (x - 1.0).abs() < 0.005, "level: {:?}", face.tris);
        }
    }
    assert!((f.aspect - 2.0).abs() < 0.01, "{}", f.aspect);
    let _ = std::fs::remove_dir_all(root);
}

/// A rectangle in the roof at height `y`, facing down, its picture's top towards -z.
fn in_the_roof(x0: f32, x1: f32, z0: f32, z1: f32, y: f32, texture: &str) -> Vec<u8> {
    let v = [[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    o3d::write(&v, &uv, &[([0, 1, 2], 0), ([0, 2, 3], 0)], &[([1.0, 1.0, 1.0, 1.0], texture)])
}

#[test]
fn a_face_in_the_roof_is_drawn_as_its_picture_stands() {
    let root = std::env::temp_dir().join(format!("hub-devices-roof-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let model = root.join("Vehicles").join("RoofBus").join("Model");
    let texture = model.parent().unwrap().join("Texture");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::create_dir_all(&texture).unwrap();
    testbus::write_test_font(&root).unwrap();
    std::fs::write(texture.join("panel.bmp"), testbus::bmp24(64, 32, |_, _| [150, 190, 120])).unwrap();
    // (the Setra's matrix panel: 12 x 6 cm in the roof, the text field 0.5 mm below it)
    let files = [
        ("RB_body.o3d", turned(-1.2, 1.2, 0.0, 2.8, 6.0, 0.0, "body.bmp")),
        ("RB_ibis_panel.o3d", in_the_roof(-0.06, 0.06, 5.30, 5.36, 2.73, "panel.bmp")),
        ("RB_ibis_text.o3d", in_the_roof(-0.05, 0.03, 5.31, 5.32, 2.7295, "text.bmp")),
    ];
    for (name, bytes) in &files {
        std::fs::write(model.join(name), bytes).unwrap();
    }
    let cfg = "[mesh]\r\nRB_body.o3d\r\n\r\n[mesh]\r\nRB_ibis_panel.o3d\r\n\r\n\
               [mesh]\r\nRB_ibis_text.o3d\r\n[matl]\r\ntext.bmp\r\n0\r\n[matl_alpha]\r\n1\r\n[useTextTexture]\r\n0\r\n\r\n\
               [texttexture]\r\nIBIS_line_text\r\nTestFont\r\n128\r\n16\r\n0\r\n255\r\n160\r\n0\r\n\r\n";
    let cfg_path = model.join("model.cfg");
    std::fs::write(&cfg_path, cfg).unwrap();

    let devices = modules::modules_of(&cfg_path);
    let ibis = devices.iter().find(|m| !m.fields.is_empty()).expect("the IBIS");
    let finder = Finder::default();
    let book = FontBook::default();
    let fonts = std::cell::RefCell::new(HashMap::new());
    let font = |name: &str, full: bool| book.font(&root, name, full, &mut fonts.borrow_mut());
    let find = |name: &str| finder.find(&cfg_path, &root, name);
    let input = Input { model_cfg: &cfg_path, omsi_dir: &root, module: ibis, numbers: &HashMap::new(), unknown: &[], font: &font, find_texture: &find };
    let f = form::build(&Models::default(), &input).expect("a screen").form;
    // upright: the picture's top left (u, v = 0, 0) at the screen's top left
    let face = f.parts.iter().find(|p| p.texture.is_some()).expect("the panel");
    for p in face.tris.chunks(4) {
        assert!((p[0] - p[2]).abs() < 0.01 && (p[1] - p[3]).abs() < 0.01, "as the picture stands: {:?}", face.tris);
    }
    // and the text in its upper part, where it lies on the panel
    let text = f.parts.iter().find(|p| p.text.is_some()).expect("the text");
    assert!(text.tris.chunks(4).all(|p| p[1] < 0.4), "{:?}", text.tris);
    let _ = std::fs::remove_dir_all(root);
}
