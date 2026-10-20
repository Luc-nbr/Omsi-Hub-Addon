//! A synthetic bus with two devices, for the tests and the fake game: everything a real bus's
//! files give the pipeline, written at runtime (no binary fixture in the repository).
//!
//! * an IBIS with a screen: an LCD picture as the background, a menu page over it shown at
//!   `[visible] IBIS_mode 1`, two text fields in the font `TestFont` (a 5 x 7 bitmap font
//!   written to `Fonts\`), a `[matl_change]` icon switched by `IBIS_stop`, two touch fields on
//!   the screen (`IBIS_menu`, and `IBIS_ok` only on the menu page) and a keypad of twelve keys
//!   3 cm below it (`IBIS_0`..`IBIS_9`, `IBIS_eingabe`, `IBIS_loeschen`); `IBIS_menu` lies on the
//!   screen only through its `[newanim]` with `trans_dauer` = 1, as the bus's script sets it at
//!   the start (the ALMEX's menu 26 of the Hamburg bus);
//! * a ticket printer of six keys and no screen (`ticketprinter_button_ticket_1`..`6`).
//!
//! The screen faces the driver along -z (the `.o3d`'s axes: x wide, y high, z long), the texts
//! lie 0.5 mm in front of it, the touch fields 0.3 mm in front facing away (as the ALMEX's do).

use crate::fonts;
use crate::o3d;
use std::io;
use std::path::{Path, PathBuf};

/// The bus file, relative to the OMSI folder, as `bus.file()` names it.
pub const BUS: &str = "Vehicles\\TestBus\\TestBus.bus";

/// A 24-bit BMP (bottom-up), `px(x, y)` from the top left.
pub fn bmp24(w: u32, h: u32, px: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let row = (w as usize * 3).div_ceil(4) * 4;
    let mut data = vec![0u8; row * h as usize];
    for y in 0..h {
        for x in 0..w {
            let [r, g, b] = px(x, y);
            let at = (h - 1 - y) as usize * row + x as usize * 3;
            data[at..at + 3].copy_from_slice(&[b, g, r]);
        }
    }
    let mut out = Vec::with_capacity(54 + data.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    out.extend_from_slice(&data);
    out
}

/// A classic 5 x 7 font: rows top to bottom, bit 4 the left column.
const GLYPHS: [(char, [u8; 7]); 41] = [
    (' ', [0, 0, 0, 0, 0, 0, 0]),
    ('0', [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E]),
    ('1', [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    ('2', [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F]),
    ('3', [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E]),
    ('4', [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02]),
    ('5', [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E]),
    ('6', [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E]),
    ('7', [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08]),
    ('8', [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E]),
    ('9', [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C]),
    ('A', [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
    ('B', [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E]),
    ('C', [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E]),
    ('D', [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C]),
    ('E', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F]),
    ('F', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10]),
    ('G', [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F]),
    ('H', [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
    ('I', [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    ('J', [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C]),
    ('K', [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11]),
    ('L', [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F]),
    ('M', [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11]),
    ('N', [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11]),
    ('O', [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    ('P', [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10]),
    ('Q', [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D]),
    ('R', [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11]),
    ('S', [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E]),
    ('T', [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
    ('U', [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    ('V', [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04]),
    ('W', [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A]),
    ('X', [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11]),
    ('Y', [0x11, 0x11, 0x11, 0x0A, 0x04, 0x04, 0x04]),
    ('Z', [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F]),
    ('-', [0, 0, 0, 0x1F, 0, 0, 0]),
    (':', [0, 0x0C, 0x0C, 0, 0x0C, 0x0C, 0]),
    ('/', [0, 0x01, 0x02, 0x04, 0x08, 0x10, 0]),
    ('.', [0, 0, 0, 0, 0, 0x0C, 0x0C]),
];

/// The font `TestFont` in `<omsi>\Fonts\testfont.oft`.
pub fn write_test_font(omsi_dir: &Path) -> io::Result<()> {
    let chars: String = GLYPHS.iter().map(|g| g.0).collect();
    fonts::write_font(&omsi_dir.join("Fonts"), "TestFont", "testfont.oft", &chars, 5, 7, |c, x, y| {
        let rows = GLYPHS.iter().find(|g| g.0 == c).map(|g| g.1).unwrap_or([0; 7]);
        if rows[y as usize] & (0x10 >> x) != 0 {
            255
        } else {
            0
        }
    })
}

/// A rectangle at depth `z`: two triangles facing the driver (-z) or, with `away`, the other way.
fn quad(x0: f32, x1: f32, y0: f32, y1: f32, z: f32, away: bool) -> Vec<u8> {
    // top left, top right, bottom right, bottom left; v = 0 at the top
    let v = [[x0, y1, z], [x1, y1, z], [x1, y0, z], [x0, y0, z]];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    // (0, 1, 2): (B-A)x(C-A) = (w,0,0)x(w,-h,0) = (0,0,-wh): facing -z
    let tris = if away { [([0, 2, 1], 0), ([0, 3, 2], 0)] } else { [([0, 1, 2], 0), ([0, 2, 3], 0)] };
    o3d::write(&v, &uv, &tris, &[([1.0, 1.0, 1.0, 1.0], "")])
}

fn textured(x0: f32, x1: f32, y0: f32, y1: f32, z: f32, texture: &str) -> Vec<u8> {
    let v = [[x0, y1, z], [x1, y1, z], [x1, y0, z], [x0, y0, z]];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    o3d::write(&v, &uv, &[([0, 1, 2], 0), ([0, 2, 3], 0)], &[([1.0, 1.0, 1.0, 1.0], texture)])
}

/// The screen: x -0.06..0.06, y 1.19..1.25, z 0.5.
const SCREEN: (f32, f32, f32, f32, f32) = (-0.06, 0.06, 1.19, 1.25, 0.5);

/// Writes the bus under `omsi_dir` (and the font); gives its `model.cfg`.
pub fn write(omsi_dir: &Path) -> io::Result<PathBuf> {
    let bus = omsi_dir.join("Vehicles").join("TestBus");
    let model = bus.join("Model");
    let texture = bus.join("Texture");
    std::fs::create_dir_all(&model)?;
    std::fs::create_dir_all(&texture)?;
    std::fs::create_dir_all(bus.join("Script"))?;
    write_test_font(omsi_dir)?;

    // the pictures: a pale green LCD with a dark frame; the menu page with two buttons
    std::fs::write(texture.join("ibis_lcd.bmp"), bmp24(240, 120, |x, y| if x < 4 || y < 4 || x >= 236 || y >= 116 { [30, 40, 30] } else { [150, 190, 120] }))?;
    std::fs::write(
        texture.join("ibis_menu.bmp"),
        bmp24(240, 120, |x, y| {
            let button = (86..116).contains(&y) && ((4..80).contains(&x) || (160..236).contains(&x));
            if button {
                [60, 90, 60]
            } else {
                [150, 190, 120]
            }
        }),
    )?;
    std::fs::write(texture.join("ibis_icon_off.bmp"), bmp24(16, 16, |_, _| [150, 190, 120]))?;
    std::fs::write(texture.join("ibis_icon_on.bmp"), bmp24(16, 16, |x, y| if (x as i32 - 8).pow(2) + (y as i32 - 8).pow(2) < 40 { [200, 30, 30] } else { [150, 190, 120] }))?;

    let (x0, x1, y0, y1, z) = SCREEN;
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("TB_body.o3d", textured(-1.25, 1.25, 0.0, 2.8, 6.0, "body.bmp")),
        ("TB_floor.o3d", textured(-1.25, 1.25, 0.3, 0.3, 0.0, "floor.bmp")),
        ("TB_dash.o3d", textured(-0.6, 0.6, 0.8, 1.0, 0.7, "dash.bmp")),
        ("TB_ibis_screen.o3d", textured(x0, x1, y0, y1, z, "ibis_lcd.bmp")),
        ("TB_ibis_menu.o3d", textured(x0, x1, y0, y1, z, "ibis_menu.bmp")),
        ("TB_ibis_line.o3d", textured(-0.055, 0.045, 1.23, 1.245, z - 0.0005, "ibis_text.bmp")),
        ("TB_ibis_dest.o3d", textured(-0.055, 0.055, 1.212, 1.227, z - 0.0005, "ibis_text.bmp")),
        ("TB_ibis_icon.o3d", textured(0.048, 0.058, 1.236, 1.246, z - 0.0003, "ibis_icon_off.bmp")),
        // (written 1.3 m to the side: its [newanim] brings it back once the script set trans_dauer)
        ("TB_ibis_touch_menu.o3d", quad(-0.058 + 1.3, -0.02 + 1.3, 1.192, 1.208, z - 0.0003, true)),
        ("TB_ibis_touch_ok.o3d", quad(0.02, 0.058, 1.192, 1.208, z - 0.0003, true)),
    ];
    for (name, bytes) in &files {
        std::fs::write(model.join(name), bytes)?;
    }
    let mut cfg = String::from("[friendlyname]\r\nTest bus\r\n\r\n");
    let mesh = |cfg: &mut String, file: &str, extra: &str| cfg.push_str(&format!("[mesh]\r\n{file}\r\n{extra}\r\n"));
    mesh(&mut cfg, "TB_body.o3d", "");
    mesh(&mut cfg, "TB_floor.o3d", "");
    mesh(&mut cfg, "TB_dash.o3d", "");
    mesh(&mut cfg, "TB_ibis_screen.o3d", "");
    mesh(&mut cfg, "TB_ibis_menu.o3d", "[visible]\r\nIBIS_mode\r\n1\r\n");
    mesh(&mut cfg, "TB_ibis_line.o3d", "[matl]\r\nibis_text.bmp\r\n0\r\n[useTextTexture]\r\n0\r\n[matl_alpha]\r\n1\r\n");
    mesh(&mut cfg, "TB_ibis_dest.o3d", "[matl]\r\nibis_text.bmp\r\n0\r\n[useTextTexture]\r\n1\r\n[matl_alpha]\r\n1\r\n");
    mesh(&mut cfg, "TB_ibis_icon.o3d", "[matl_change]\r\nibis_icon_off.bmp\r\n0\r\nIBIS_stop\r\n[matl_item]\r\n[matl_nightmap]\r\nibis_icon_on.bmp\r\n");
    mesh(&mut cfg, "TB_ibis_touch_menu.o3d", "[mouseevent]\r\nIBIS_menu\r\n[newanim]\r\norigin_trans\r\n0\r\n0\r\n0\r\n\r\nanim_trans\r\ntrans_dauer\r\n-1.3\r\n");
    mesh(&mut cfg, "TB_ibis_touch_ok.o3d", "[mouseevent]\r\nIBIS_ok\r\n[visible]\r\nIBIS_mode\r\n1\r\n");
    // the keypad, two rows of six, 3 cm below the screen
    let keys = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "eingabe", "loeschen"];
    for (k, key) in keys.iter().enumerate() {
        let (col, row) = ((k % 6) as f32, (k / 6) as f32);
        let kx = x0 + col * 0.02;
        let ky = y0 - 0.03 - row * 0.016;
        let file = format!("TB_ibis_key_{key}.o3d");
        std::fs::write(model.join(&file), quad(kx, kx + 0.016, ky - 0.012, ky, z, false))?;
        mesh(&mut cfg, &file, &format!("[mouseevent]\r\nIBIS_{key}\r\n"));
    }
    // the ticket printer: six keys in a row, 40 cm to the right
    for k in 1..=6 {
        let kx = 0.4 + (k - 1) as f32 * 0.025;
        let file = format!("TB_printer_key_{k}.o3d");
        std::fs::write(model.join(&file), quad(kx, kx + 0.02, 1.0, 1.015, 0.6, false))?;
        mesh(&mut cfg, &file, &format!("[mouseevent]\r\nticketprinter_button_ticket_{k}\r\n"));
    }
    // the text blocks: [useTextTexture] 0 and 1
    cfg.push_str("[texttexture]\r\nIBIS_line_text\r\nTestFont\r\n128\r\n16\r\n0\r\n255\r\n160\r\n0\r\n\r\n");
    cfg.push_str("[texttexture_enh]\r\nIBIS_dest_text\r\nTestFont\r\n128\r\n16\r\n0\r\n20\r\n30\r\n20\r\n1\r\n1\r\n\r\n");
    std::fs::write(model.join("model.cfg"), cfg)?;
    std::fs::write(bus.join("TestBus.bus"), "[friendlyname]\r\nTest bus\r\n\r\n[model]\r\nModel\\model.cfg\r\n\r\n[script]\r\n1\r\nScript\\main.osc\r\n")?;
    std::fs::write(bus.join("Script").join("main.osc"), "{init}\r\n  0 (S.L.IBIS_mode)\r\n  (M.L.cockpit_init)\r\n{end}\r\n\r\n{macro:cockpit_init}\r\n  1 (S.L.trans_dauer)\r\n{end}\r\n")?;
    Ok(model.join("model.cfg"))
}
