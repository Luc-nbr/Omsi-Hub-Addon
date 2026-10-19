//! A bus's repaints, what OMSI calls its "Appearance" (the career add-on's
//! `core/kleurstelling.ts`).
//!
//! In `model.cfg`, `[CTC]` points at a folder and names the variable that carries the choice
//! (nearly always `Colorscheme`), and `[CTCTexture]` ties a name to the texture that lies there
//! by default. The folder holds `.cti` files; each `[item]` says: in repaint "OVPS" the place
//! `Farbschema_12C_2door` gets the file `OVPS\12C_2d_OVPS.dds`.
//!
//! The choice is a number in that variable, and it counts in the order the repaints appear in
//! the files: the files by name as Windows lists them, within a file top to bottom, each name
//! once. (OMSI's picker shows them alphabetically but does not count so; measured on the
//! `laststn.osn` the game wrote.)

use crate::scripts::{by_name, join_rel};
use crate::text;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One repaint.
#[derive(Debug, Clone, PartialEq)]
pub struct Repaint {
    pub index: usize,
    pub name: String,
    /// Per place (the name from `[CTCTexture]`) the texture's full path.
    pub textures: HashMap<String, PathBuf>,
}

/// The repaints of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Repaints {
    /// The script variable that carries the number.
    pub variable: String,
    pub dir: PathBuf,
    /// Per place the texture that lies there by default.
    pub places: HashMap<String, String>,
    pub list: Vec<Repaint>,
}

/// The repaints of a `model.cfg`, or `None` when it has none.
pub fn read(model_cfg: &Path) -> Option<Repaints> {
    let lines = text::read_lines(model_cfg)?;
    let model_dir = model_cfg.parent()?;
    let vehicle_dir = model_dir.parent()?;
    let omsi_dir = vehicle_dir.parent().and_then(Path::parent);
    let mut variable: Option<String> = None;
    let mut dir: Option<PathBuf> = None;
    let mut places = HashMap::new();
    let at = |i: usize| lines.get(i).map(|s| s.trim()).unwrap_or("");
    for (i, line) in lines.iter().enumerate() {
        match line.trim().to_lowercase().as_str() {
            "[ctc]" => {
                variable = Some(at(i + 1).to_string());
                let rel = at(i + 2);
                if !rel.is_empty() {
                    dir = Some(join_rel(vehicle_dir, rel));
                }
            }
            "[ctctexture]" => {
                let (place, default) = (at(i + 1), at(i + 2));
                if !place.is_empty() && !default.is_empty() {
                    places.insert(place.to_string(), default.to_string());
                }
            }
            _ => {}
        }
    }
    let variable = variable.filter(|v| !v.is_empty())?;
    let dir = dir.filter(|d| d.is_dir())?;
    let mut files: Vec<String> = std::fs::read_dir(&dir).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.to_lowercase().ends_with(".cti")).collect();
    files.sort_by(|a, b| by_name(a, b));

    let find = |rel: &str| -> Option<PathBuf> {
        let direct = join_rel(&dir, rel);
        if direct.exists() {
            return Some(direct);
        }
        index_lookup(vehicle_dir, omsi_dir, rel)
    };
    let mut list: Vec<Repaint> = Vec::new();
    let mut by: HashMap<String, usize> = HashMap::new();
    for file in files {
        let Some(content) = text::read_lines(&dir.join(&file)) else { continue };
        let at = |i: usize| content.get(i).map(|s| s.trim()).unwrap_or("");
        for (i, line) in content.iter().enumerate() {
            if line.trim().eq_ignore_ascii_case("[item]") {
                let (name, place, rel) = (at(i + 1), at(i + 2), at(i + 3));
                if name.is_empty() {
                    continue;
                }
                let k = *by.entry(name.to_string()).or_insert_with(|| {
                    list.push(Repaint { index: list.len(), name: name.to_string(), textures: HashMap::new() });
                    list.len() - 1
                });
                if !place.is_empty() && !rel.is_empty() {
                    if let Some(p) = find(rel) {
                        list[k].textures.insert(place.to_string(), p);
                    }
                }
            }
        }
    }
    if list.is_empty() {
        return None;
    }
    Some(Repaints { variable, dir, places, list })
}

/// The textures a repaint changes: the default's [`texture_key`] and what comes in its place.
pub fn replacements(info: &Repaints, repaint: &Repaint) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    for (place, path) in &repaint.textures {
        if let Some(default) = info.places.get(place) {
            out.insert(texture_key(default), path.clone());
        }
    }
    out
}

/// A texture's name without folder and extension, in lower case: the `.o3d` and
/// `[CTCTexture]` sometimes name the same texture with another extension (`.bmp` and `.dds`).
pub fn texture_key(name: &str) -> String {
    let flat = name.rsplit(['\\', '/']).next().unwrap_or(name).to_lowercase();
    match flat.rfind('.') {
        Some(i) if i > 0 => flat[..i].to_string(),
        _ => flat,
    }
}

/// The file index of a vehicle folder, four folders deep: by lower-case file name, the
/// shallowest wins (the career add-on's `busmodel.ts` `bestandenIn`).
pub fn file_index(dir: &Path) -> HashMap<String, PathBuf> {
    let mut index: HashMap<String, (PathBuf, usize)> = HashMap::new();
    fn walk(here: &Path, level: usize, index: &mut HashMap<String, (PathBuf, usize)>) {
        if level > 4 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(here) else { return };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by(|a, b| by_name(&a.file_name().to_string_lossy(), &b.file_name().to_string_lossy()));
        for e in entries {
            let full = e.path();
            if full.is_dir() {
                walk(&full, level + 1, index);
                continue;
            }
            let key = e.file_name().to_string_lossy().to_lowercase();
            match index.get(&key) {
                Some((_, l)) if *l <= level => {}
                _ => {
                    index.insert(key, (full, level));
                }
            }
        }
    }
    walk(dir, 0, &mut index);
    index.into_iter().map(|(k, (p, _))| (k, p)).collect()
}

/// A texture by its file name alone: the vehicle folder's index, then OMSI's `Texture`, then
/// the same stem with the other picture formats (`zoekTextuurVan`).
pub fn index_lookup(vehicle_dir: &Path, omsi_dir: Option<&Path>, name: &str) -> Option<PathBuf> {
    let flat = name.rsplit(['\\', '/']).next()?.to_lowercase();
    if flat.is_empty() {
        return None;
    }
    let own = file_index(vehicle_dir);
    let shared = omsi_dir.map(|o| file_index(&o.join("Texture"))).unwrap_or_default();
    if let Some(p) = own.get(&flat).or_else(|| shared.get(&flat)) {
        return Some(p.clone());
    }
    let dot = flat.rfind('.').filter(|i| *i > 0)?;
    let stem = &flat[..dot];
    for ext in [".dds", ".tga", ".bmp", ".png", ".jpg", ".jpeg"] {
        let key = format!("{stem}{ext}");
        if let Some(p) = own.get(&key).or_else(|| shared.get(&key)) {
            return Some(p.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repaints_count_in_file_order_and_replace_by_stem() {
        let root = std::env::temp_dir().join(format!("hub-devices-ctc-{}", std::process::id()));
        let bus = root.join("Vehicles").join("Bus");
        std::fs::create_dir_all(bus.join("Model")).unwrap();
        std::fs::create_dir_all(bus.join("Texture").join("CTC").join("red")).unwrap();
        std::fs::write(bus.join("Texture").join("CTC").join("red").join("screen_red.dds"), b"x").unwrap();
        std::fs::write(bus.join("Model").join("model.cfg"), "[CTC]\r\nColorscheme\r\nTexture\\CTC\r\n\r\n[CTCTexture]\r\nScreen\r\nscreen.bmp\r\n").unwrap();
        std::fs::write(bus.join("Texture").join("CTC").join("b.cti"), "[item]\r\nRed\r\nScreen\r\nred\\screen_red.dds\r\n").unwrap();
        std::fs::write(bus.join("Texture").join("CTC").join("A.cti"), "[item]\r\nZebra\r\nNone\r\nnothing.dds\r\n").unwrap();
        let info = read(&bus.join("Model").join("model.cfg")).unwrap();
        assert_eq!(info.variable, "Colorscheme");
        assert_eq!(info.list.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Zebra", "Red"]);
        let swap = replacements(&info, &info.list[1]);
        assert!(swap.get("screen").unwrap().ends_with("screen_red.dds"));
        let _ = std::fs::remove_dir_all(root);
    }
}
