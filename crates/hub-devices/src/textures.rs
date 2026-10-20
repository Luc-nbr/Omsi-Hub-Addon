//! The pictures of a device's screen: found in the OMSI folder and turned into PNGs the app can
//! draw (the career add-on's `core/schermtextuur.ts`).
//!
//! Never a path outward. A texture's name comes from an add-on's cfg or, with `[matl_freetex]`,
//! from a string variable the bus's script may fill with anything. [`name_parts`] refuses every
//! name that could leave the folder (absolute, `..`, a drive letter or a `:` stream, a device name
//! such as `CON`) and every extension that is no picture, and what is found is checked against
//! the folder once more. Outward goes only an id ([`id_of`]); the server answers only ids it
//! registered itself.
//!
//! As OMSI draws it: OMSI looks for a texture in `<model.cfg's folder>\..\Texture\<name>`, with
//! the subfolder when the name has one, and takes another format with the same stem when the
//! name does not exist (a cfg names `AFR200.tga`, the disk has `AFR200.dds`); then OMSI's own
//! `Texture`; last the index of the whole vehicle folder. And what the material does to the
//! picture is done here before sending, for the app knows nothing of `[matl_alpha]`: alpha 0
//! opaque even when the file has alpha, alpha 1 an alpha test at 128 (after scaling), alpha 2 as
//! it is; a transmap's alpha replaces the picture's; a nightmap switched on is added to the
//! picture's colour, as the game lights it; at most 2048 pixels the longest side,
//! scaled down by the average so that text in the picture stays readable.

use crate::ctc;
use crate::scripts::norm;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What the material does with the picture, as far as the server applies it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Flags {
    /// `[matl_alpha]`: 0 opaque, 1 alpha test, 2 blended.
    pub alpha: u8,
    /// The `[matl_transmap]`'s path, found as the texture was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trans: Option<PathBuf>,
    /// Cut to x, y, width, height in pixels of the ORIGINAL picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<[i64; 4]>,
    /// A `[matl_nightmap]` lit, added to the picture's colour (an item of a `[matl_change]`
    /// that switches only the light: the MAN SD80's IBIS display at `elec_busbar_main` 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub night: Option<PathBuf>,
}

impl Flags {
    pub fn alpha(alpha: u8) -> Flags {
        Flags { alpha, trans: None, crop: None, night: None }
    }

    /// In one fixed form, so that the same intent gives the same id.
    pub fn clean(&self) -> Flags {
        let alpha = if self.alpha <= 1 { self.alpha } else { 2 };
        // (at alpha 0 the game draws opaque: a transmap does nothing then)
        let trans = self.trans.clone().filter(|_| alpha != 0);
        let crop = self.crop.filter(|c| c[2] > 0 && c[3] > 0);
        Flags { alpha, trans, crop, night: self.night.clone() }
    }
}

/// The longest side that goes to the phone.
pub const LIMIT: u32 = 2048;

const PICTURES: [&str; 6] = ["jpg", "jpeg", "png", "bmp", "dds", "tga"];
/// The other extensions tried when the cfg's name does not exist; DDS first, for that is what
/// add-ons convert to.
const OTHER_EXTENSIONS: [&str; 6] = ["dds", "tga", "bmp", "jpg", "png", "jpeg"];

fn extension(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

/// Windows device names, with or without an extension (`CON.jpg` is the console).
fn device_name(stem: &str) -> bool {
    let s = stem.trim().to_lowercase();
    matches!(s.as_str(), "con" | "prn" | "aux" | "nul" | "conin$" | "conout$" | "clock$")
        || ((s.starts_with("com") || s.starts_with("lpt")) && {
            let rest: Vec<char> = s.chars().skip(3).collect();
            rest.len() == 1 && (rest[0].is_ascii_digit() || "¹²³".contains(rest[0]))
        })
}

/// A texture's name as folder names and a file name, or `None` when the name is not safe.
pub fn name_parts(name: &str) -> Option<Vec<String>> {
    let clean = name.trim();
    if clean.is_empty() || clean.chars().count() > 260 {
        return None;
    }
    if clean.chars().any(|c| (c as u32) < 0x20 || ":<>\"|?*".contains(c)) {
        return None;
    }
    if clean.starts_with(['\\', '/']) {
        return None;
    }
    let mut parts = Vec::new();
    for part in clean.split(['\\', '/']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part.trim_end_matches(['.', ' ']).is_empty() {
            return None;
        }
        if device_name(part.split('.').next().unwrap_or("")) {
            return None;
        }
        parts.push(part.to_string());
    }
    let last = parts.last()?;
    PICTURES.contains(&extension(last).as_str()).then_some(parts)
}

/// Whether `path` lies inside `dir` (not `dir` itself), case-insensitively.
pub fn inside(path: &Path, dir: &Path) -> bool {
    let (p, d) = (norm(path), norm(dir));
    let d = if d.ends_with('/') { d } else { format!("{d}/") };
    p.starts_with(&d)
}

/// A file in a folder by a case-insensitive name, as Windows finds it (exact first).
fn file_ci(dir: &Path, parts: &[String]) -> Option<PathBuf> {
    let exact = parts.iter().fold(dir.to_path_buf(), |d, p| d.join(p));
    if exact.is_file() {
        return Some(exact);
    }
    let mut here = dir.to_path_buf();
    for (k, part) in parts.iter().enumerate() {
        let want = part.to_lowercase();
        let found = std::fs::read_dir(&here).ok()?.flatten().find(|e| e.file_name().to_string_lossy().to_lowercase() == want)?;
        here = found.path();
        if k + 1 < parts.len() && !here.is_dir() {
            return None;
        }
    }
    here.is_file().then_some(here)
}

/// The file in `dir` under this name, or with the same stem and another picture format.
fn find_in(dir: &Path, parts: &[String]) -> Option<PathBuf> {
    let exact = parts.iter().fold(dir.to_path_buf(), |d, p| d.join(p));
    if !inside(&exact, dir) {
        return None;
    }
    if let Some(p) = file_ci(dir, parts) {
        return Some(p);
    }
    let last = parts.last()?;
    let ext = extension(last);
    let stem = &last[..last.len() - ext.len() - usize::from(!ext.is_empty())];
    for other in OTHER_EXTENSIONS {
        if other == ext {
            continue;
        }
        let mut alt = parts.to_vec();
        *alt.last_mut()? = format!("{stem}.{other}");
        if let Some(p) = file_ci(dir, &alt) {
            return Some(p);
        }
    }
    None
}

/// Finds a screen's textures, with what it found the last seconds kept: a freetex value comes
/// with every frame, ten times a second.
#[derive(Default)]
pub struct Finder {
    recent: Mutex<HashMap<String, (Option<PathBuf>, Instant)>>,
    indexes: Mutex<HashMap<PathBuf, Arc<HashMap<String, PathBuf>>>>,
}

const REMEMBER: Duration = Duration::from_secs(5);

impl Finder {
    /// A texture's absolute path as the game finds it for this bus, or `None` (not safe, or
    /// nowhere). What comes out lies inside the vehicle folder (the one above `Model`) or OMSI's
    /// `Texture`.
    pub fn find(&self, model_cfg: &Path, omsi_dir: &Path, name: &str) -> Option<PathBuf> {
        let key = format!("{}|{}|{name}", model_cfg.display(), omsi_dir.display()).to_lowercase();
        if let Some((p, until)) = self.recent.lock().ok()?.get(&key) {
            if *until > Instant::now() {
                return p.clone();
            }
        }
        let found = self.find_now(model_cfg, omsi_dir, name);
        if let Ok(mut recent) = self.recent.lock() {
            if recent.len() >= 1024 {
                recent.clear();
            }
            recent.insert(key, (found.clone(), Instant::now() + REMEMBER));
        }
        found
    }

    fn find_now(&self, model_cfg: &Path, omsi_dir: &Path, name: &str) -> Option<PathBuf> {
        let parts = name_parts(name)?;
        // (without two full paths nothing can be locked in)
        if !model_cfg.is_absolute() || !omsi_dir.is_absolute() {
            return None;
        }
        let vehicle = model_cfg.parent()?.parent()?.to_path_buf();
        let own = vehicle.join("Texture");
        let shared = omsi_dir.join("Texture");
        if let Some(p) = find_in(&own, &parts).or_else(|| find_in(&shared, &parts)) {
            return Some(p);
        }
        // the index four folders deep, only for a folder of its own inside the installation
        let wide = [norm(omsi_dir), norm(&omsi_dir.join("Vehicles"))];
        if wide.contains(&norm(&vehicle)) || !inside(&vehicle, omsi_dir) {
            return None;
        }
        let flat = parts.last()?.to_lowercase();
        let index = self.index_of(&vehicle);
        let shared_index = self.index_of(&shared);
        let hit = index.get(&flat).or_else(|| shared_index.get(&flat)).cloned().or_else(|| {
            let ext = extension(&flat);
            let stem = &flat[..flat.len() - ext.len() - usize::from(!ext.is_empty())];
            ["dds", "tga", "bmp", "png", "jpg", "jpeg"].iter().find_map(|e| {
                let k = format!("{stem}.{e}");
                index.get(&k).or_else(|| shared_index.get(&k)).cloned()
            })
        })?;
        ((inside(&hit, &vehicle) || inside(&hit, &shared)) && hit.is_file()).then_some(hit)
    }

    fn index_of(&self, dir: &Path) -> Arc<HashMap<String, PathBuf>> {
        if let Some(i) = self.indexes.lock().ok().and_then(|m| m.get(dir).cloned()) {
            return i;
        }
        let i = Arc::new(ctc::file_index(dir));
        if let Ok(mut m) = self.indexes.lock() {
            m.insert(dir.to_path_buf(), i.clone());
        }
        i
    }
}

/// Size and modification time, or a sign the file is not there.
fn stamp(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(m) => {
            let ms = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis()).unwrap_or(0);
            format!("{}|{ms}", m.len())
        }
        Err(_) => "-1|0".to_string(),
    }
}

/// The id a picture with its flags goes out under: the first 20 hex characters of a hash of
/// its path, size, modification time and flags. It follows the CONTENT, so the app may keep it
/// for good; the same file with other flags is another picture.
pub fn id_of(path: &Path, flags: &Flags) -> String {
    let f = flags.clean();
    let trans = f.trans.as_ref().map(|t| format!("{}|{}", norm(t), stamp(t))).unwrap_or_default();
    let mut key = format!("{}|{}|{}|{trans}|{:?}|v1", norm(path), stamp(path), f.alpha, f.crop);
    if let Some(n) = &f.night {
        key.push_str(&format!("|night {}|{}", norm(n), stamp(n)));
    }
    hex20(key.as_bytes())
}

/// The first 20 hex characters of a SHA-256.
pub fn hex20(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().take(10).map(|b| format!("{b:02x}")).collect()
}

/// RGBA pixels.
struct Pixels {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

fn decode(path: &Path) -> Option<Pixels> {
    if !PICTURES.contains(&extension(&path.to_string_lossy()).as_str()) {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let img = omsi_texture::decode_bytes(&bytes, path).ok()?;
    Some(Pixels { w: img.width, h: img.height, rgba: img.rgba })
}

/// A picture as a PNG for the app, with the flags applied, or `None` when it cannot be read.
///
/// The order counts: the transmap and the nightmap on the full size; the crop; opaque (alpha 0)
/// BEFORE scaling,
/// or the average still weighs the colour by an alpha the game does not use; scaling; the alpha
/// test (alpha 1) AFTER it, as Direct3D tests what it samples.
pub fn png_of(path: &Path, flags: &Flags, limit: u32) -> Option<Vec<u8>> {
    let f = flags.clean();
    let mut t = decode(path)?;
    if let Some(trans) = &f.trans {
        set_transmap(&mut t, trans);
    }
    if let Some(night) = &f.night {
        add_nightmap(&mut t, night);
    }
    if let Some(c) = f.crop {
        if !(c[0] <= 0 && c[1] <= 0 && c[0] + c[2] >= t.w as i64 && c[1] + c[3] >= t.h as i64) {
            t = cut(&t, c)?;
        }
    }
    if f.alpha == 0 {
        for a in t.rgba.iter_mut().skip(3).step_by(4) {
            *a = 255;
        }
    }
    let mut t = scale_down(t, limit);
    if f.alpha == 1 {
        for a in t.rgba.iter_mut().skip(3).step_by(4) {
            *a = if *a >= 128 { 255 } else { 0 };
        }
    }
    encode_png(t.w, t.h, &t.rgba)
}

/// RGBA pixels as a PNG.
pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out).write_image(rgba, w, h, image::ExtendedColorType::Rgba8).ok()?;
    Some(out)
}

/// The alpha of the transmap, not its red: over all transmaps of a fleet the alpha differs
/// where the red is flat, never the other way round. One without alpha gives 255 everywhere.
/// Another size is sampled to the texture's (nearest pixel), as Direct3D would with the same uvs.
fn set_transmap(t: &mut Pixels, path: &Path) -> bool {
    let Some(map) = decode(path) else { return false };
    if (map.w, map.h) == (t.w, t.h) {
        for i in (3..t.rgba.len()).step_by(4) {
            t.rgba[i] = map.rgba[i];
        }
        return true;
    }
    if map.w == 0 || map.h == 0 {
        return false;
    }
    let cols: Vec<u32> = (0..t.w).map(|x| ((((x as f64 + 0.5) * map.w as f64) / t.w as f64).floor() as u32).min(map.w - 1)).collect();
    for y in 0..t.h {
        let row = ((((y as f64 + 0.5) * map.h as f64) / t.h as f64).floor() as u32).min(map.h - 1) * map.w;
        for x in 0..t.w {
            t.rgba[((y * t.w + x) * 4 + 3) as usize] = map.rgba[((row + cols[x as usize]) * 4 + 3) as usize];
        }
    }
    true
}

/// The nightmap's colour added to the picture's (its alpha stays), sampled to the picture's size
/// as [`set_transmap`] does: lit, a black nightmap changes nothing and a red one shows red.
fn add_nightmap(t: &mut Pixels, path: &Path) -> bool {
    let Some(map) = decode(path) else { return false };
    if map.w == 0 || map.h == 0 {
        return false;
    }
    let cols: Vec<u32> = (0..t.w).map(|x| ((((x as f64 + 0.5) * map.w as f64) / t.w as f64).floor() as u32).min(map.w - 1)).collect();
    for y in 0..t.h {
        let row = ((((y as f64 + 0.5) * map.h as f64) / t.h as f64).floor() as u32).min(map.h - 1) * map.w;
        for x in 0..t.w {
            let at = ((y * t.w + x) * 4) as usize;
            let from = ((row + cols[x as usize]) * 4) as usize;
            for k in 0..3 {
                t.rgba[at + k] = t.rgba[at + k].saturating_add(map.rgba[from + k]);
            }
        }
    }
    true
}

fn cut(t: &Pixels, c: [i64; 4]) -> Option<Pixels> {
    let x0 = c[0].max(0);
    let y0 = c[1].max(0);
    let x1 = (c[0] + c[2]).min(t.w as i64);
    let y1 = (c[1] + c[3]).min(t.h as i64);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as i64 {
        let from = (((y0 + y) * t.w as i64 + x0) * 4) as usize;
        rgba.extend_from_slice(&t.rgba[from..from + (w * 4) as usize]);
    }
    Some(Pixels { w, h, rgba })
}

/// JavaScript's `Math.round`: halves up.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// At most `limit` the longest side, by the average over the whole area a new pixel covers,
/// the colour weighted by its alpha (a transparent pixel adds no colour).
fn scale_down(t: Pixels, limit: u32) -> Pixels {
    let longest = t.w.max(t.h);
    if longest <= limit || limit < 1 {
        return t;
    }
    let scale = limit as f64 / longest as f64;
    let nw = (js_round(t.w as f64 * scale) as u32).clamp(1, limit);
    let nh = (js_round(t.h as f64 * scale) as u32).clamp(1, limit);
    let (w, h) = (t.w as usize, t.h as usize);
    let fx = w as f64 / nw as f64;
    let mut src: Vec<Vec<(usize, f64)>> = Vec::with_capacity(nw as usize);
    for x in 0..nw as usize {
        let (from, to) = (x as f64 * fx, (x + 1) as f64 * fx);
        let mut cols = Vec::new();
        for i in from.floor() as usize..(to.ceil() as usize).min(w) {
            let part = ((i + 1) as f64).min(to) - (i as f64).max(from);
            if part > 1e-9 {
                cols.push((i, part));
            }
        }
        src.push(cols);
    }
    let mut out = vec![0u8; nw as usize * nh as usize * 4];
    let mut sum = vec![0f64; nw as usize * 8];
    let fy = h as f64 / nh as f64;
    for y in 0..nh as usize {
        sum.iter_mut().for_each(|s| *s = 0.0);
        let (from, to) = (y as f64 * fy, (y + 1) as f64 * fy);
        for j in from.floor() as usize..(to.ceil() as usize).min(h) {
            let wy = ((j + 1) as f64).min(to) - (j as f64).max(from);
            if wy <= 1e-9 {
                continue;
            }
            let row = j * w * 4;
            for (x, cols) in src.iter().enumerate() {
                let s = x * 8;
                for &(i, wx) in cols {
                    let wt = wy * wx;
                    let p = row + i * 4;
                    let px = &t.rgba[p..p + 4];
                    let wa = wt * px[3] as f64;
                    sum[s] += px[0] as f64 * wa;
                    sum[s + 1] += px[1] as f64 * wa;
                    sum[s + 2] += px[2] as f64 * wa;
                    sum[s + 3] += wa;
                    sum[s + 4] += px[0] as f64 * wt;
                    sum[s + 5] += px[1] as f64 * wt;
                    sum[s + 6] += px[2] as f64 * wt;
                    sum[s + 7] += wt;
                }
            }
        }
        for x in 0..nw as usize {
            let s = x * 8;
            let o = (y * nw as usize + x) * 4;
            let total = sum[s + 7];
            if total <= 0.0 {
                continue;
            }
            for k in 0..3 {
                out[o + k] = if sum[s + 3] > 0.0 { js_round(sum[s + k] / sum[s + 3]) } else { js_round(sum[s + 4 + k] / total) } as u8;
            }
            out[o + 3] = js_round(sum[s + 3] / total) as u8;
        }
    }
    Pixels { w: nw, h: nh, rgba: out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_names_are_refused() {
        assert_eq!(name_parts("Sub\\a.bmp"), Some(vec!["Sub".to_string(), "a.bmp".to_string()]));
        assert_eq!(name_parts(".\\a.dds"), Some(vec!["a.dds".to_string()]));
        for bad in ["..\\..\\x.bmp", "C:\\x.bmp", "\\x.bmp", "/etc/x.png", "x.bmp:stream", "CON.jpg", "lpt1.bmp", "a.exe", "... \\x.bmp", "", "a\u{1}.bmp"] {
            assert_eq!(name_parts(bad), None, "{bad}");
        }
    }

    #[test]
    fn found_with_another_extension_and_never_outside() {
        let root = std::env::temp_dir().join(format!("hub-devices-tex-{}", std::process::id()));
        let bus = root.join("Vehicles").join("Bus");
        std::fs::create_dir_all(bus.join("Model")).unwrap();
        std::fs::create_dir_all(bus.join("Texture").join("Sub")).unwrap();
        std::fs::create_dir_all(root.join("Texture")).unwrap();
        std::fs::write(bus.join("Texture").join("AFR200.dds"), b"x").unwrap();
        std::fs::write(bus.join("Texture").join("Sub").join("deep.png"), b"x").unwrap();
        std::fs::write(root.join("Texture").join("shared.bmp"), b"x").unwrap();
        std::fs::write(root.join("secret.bmp"), b"x").unwrap();
        let cfg = bus.join("Model").join("model.cfg");
        let f = Finder::default();
        assert!(f.find(&cfg, &root, "AFR200.tga").unwrap().ends_with("AFR200.dds"));
        assert!(f.find(&cfg, &root, "afr200.dds").is_some(), "case-insensitive as Windows");
        assert!(f.find(&cfg, &root, "Sub\\deep.png").is_some());
        assert!(f.find(&cfg, &root, "deep.png").is_some(), "the vehicle folder's index");
        assert!(f.find(&cfg, &root, "shared.bmp").unwrap().starts_with(root.join("Texture")));
        assert_eq!(f.find(&cfg, &root, "..\\..\\..\\secret.bmp"), None);
        assert_eq!(f.find(&cfg, &root, "secret.bmp"), None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn flags_apply_in_order_and_the_id_follows_the_flags() {
        let root = std::env::temp_dir().join(format!("hub-devices-png-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let p = root.join("a.png");
        // 4 x 1: alpha 0, 100, 200, 255
        let rgba: Vec<u8> = [0u8, 100, 200, 255].iter().flat_map(|a| [10, 20, 30, *a]).collect();
        std::fs::write(&p, encode_png(4, 1, &rgba).unwrap()).unwrap();
        let read = |png: Vec<u8>| image::load_from_memory(&png).unwrap().into_rgba8().into_raw();
        let opaque = read(png_of(&p, &Flags::alpha(0), LIMIT).unwrap());
        assert!(opaque.iter().skip(3).step_by(4).all(|a| *a == 255));
        let tested = read(png_of(&p, &Flags::alpha(1), LIMIT).unwrap());
        assert_eq!(tested.iter().skip(3).step_by(4).copied().collect::<Vec<_>>(), [0, 0, 255, 255]);
        let blended = read(png_of(&p, &Flags::alpha(2), 2).unwrap());
        assert_eq!(blended.len(), 2 * 4, "scaled to the limit");
        assert_eq!(blended[3], 50, "the average alpha");
        assert_ne!(id_of(&p, &Flags::alpha(0)), id_of(&p, &Flags::alpha(2)));
        assert_eq!(id_of(&p, &Flags::alpha(0)).len(), 20);
        // (a transmap at alpha 0 does nothing, and so does not change the id)
        let with = Flags { alpha: 0, trans: Some(p.clone()), crop: None, night: None };
        assert_eq!(id_of(&p, &with), id_of(&p, &Flags::alpha(0)));
        // a nightmap lit: its colour added, the alpha the picture's; another picture
        let n = root.join("n.png");
        let night: Vec<u8> = [[0u8, 0, 0, 255], [250, 0, 0, 255], [0, 0, 0, 0], [0, 100, 0, 255]].concat();
        std::fs::write(&n, encode_png(4, 1, &night).unwrap()).unwrap();
        let lit = Flags { alpha: 2, trans: None, crop: None, night: Some(n.clone()) };
        let px = read(png_of(&p, &lit, LIMIT).unwrap());
        assert_eq!(px, [10, 20, 30, 0, 255, 20, 30, 100, 10, 20, 30, 200, 10, 120, 30, 255]);
        assert_ne!(id_of(&p, &lit), id_of(&p, &Flags::alpha(2)));
        let _ = std::fs::remove_dir_all(root);
    }
}
