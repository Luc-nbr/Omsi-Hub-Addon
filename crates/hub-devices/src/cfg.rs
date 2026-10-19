//! A `model.cfg` read as Omsi.exe reads it (the career add-on's `core/schermcfg.ts`, checked
//! against Omsi.exe 2.3.004 there): per `[mesh]` line all its `[visible]`s, the whole chain of
//! `[animparent]` and `[newanim]`, its LOD group and viewpoint, and per material what the cfg
//! says of it. It has to be read exactly as the game does, for the numbering counts: the mesh
//! index and the number after `[useTextTexture]`.
//!
//! How the game reads:
//!
//! * the file goes through `TStringList.LoadFromFile`: CR, LF and CRLF all break a line;
//! * a heading counts only when the WHOLE line equals it, untrimmed and case-sensitive. An
//!   installation has 129 indented `\t[mesh]` and five `[Mesh_ident]`: comments to the game,
//!   and so here. Whoever trims gets parts the game does not have and `[useTextTexture]`
//!   numbers that shift;
//! * after a heading the game reads a fixed number of lines, whatever they say.
//!
//! The game skips a `[mesh]` whose `.o3d` does not exist (relative to the cfg's folder); the
//! mesh index counts only the ones that do. What follows a missing mesh stays with that line.

use crate::text;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A point or direction in the axes of the `.o3d` files: x wide, y high, z long.
pub type P3 = [f64; 3];

/// Which axis a rotation turns about, as the cfg names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// One `[newanim]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Anim {
    /// The point turned about, in o3d axes (`origin_trans` is given in OMSI's: x right, y
    /// forward, z up, and is turned here), or `None`: `origin_from_mesh`, the 0x79 matrix's.
    pub origin: Option<P3>,
    /// `origin_rot_x|y|z <angle>`, in the cfg's order; degrees.
    pub rotations: Vec<(Axis, f64)>,
    /// `anim_trans <var> <factor>`: a shift along the local x.
    pub shift: Option<(String, f64)>,
    /// `anim_rot <var> <factor>`: a turn about the local x, degrees.
    pub turn: Option<(String, f64)>,
    /// The heading's line (from 1).
    pub line: usize,
}

/// What the cfg says of a material, or of one `[matl_item]` of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatlState {
    /// `[matl_alpha]`: 0 opaque, 1 alpha test, 2 blended.
    pub alpha: Option<u8>,
    /// `[matl_freetex] <default> <string variable>`.
    pub freetex: Option<(String, String)>,
    /// `[matl_transmap]`: a texture's name, `\S:n` (a script texture) or ''.
    pub transmap: Option<String>,
    /// `[matl_allcolor]`: 14 numbers, diffuse rgba first; over the `.o3d`'s colours.
    pub allcolor: Option<Vec<f64>>,
    /// `[matl_lightmap] <tex> [var]`.
    pub lightmap: Option<(String, Option<String>)>,
    /// `[matl_nightmap] <tex>`.
    pub nightmap: Option<String>,
    /// `[texcoordtransX|Y] <var>`.
    pub texcoord_x: Option<String>,
    pub texcoord_y: Option<String>,
}

/// Texture addressing outside 0..1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address {
    Wrap,
    Clamp,
    Border,
}

impl Address {
    pub fn name(self) -> &'static str {
        match self {
            Address::Wrap => "wrap",
            Address::Clamp => "clamp",
            Address::Border => "border",
        }
    }
}

/// One `[matl]` or `[matl_change]` with everything that belongs to it up to the next.
#[derive(Debug, Clone, PartialEq)]
pub struct Matl {
    pub change: bool,
    /// The texture's name in the cfg; with `nth` it picks the `.o3d`'s material.
    pub texture: String,
    /// The how-manyth `.o3d` material WITH THIS NAME (from 0); see [`material_of`].
    pub nth: i64,
    /// `[matl_change]`: the number variable that picks the item.
    pub variable: Option<String>,
    /// `[useTextTexture] k`: the k-th text block of this file (from 0).
    pub text: Option<usize>,
    pub address: Option<Address>,
    pub state: MatlState,
    /// The `[matl_item]`s of a `[matl_change]`: item 1, 2, ...
    pub items: Vec<MatlState>,
    pub line: usize,
}

/// One `[mesh]` line with everything that belongs to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// The path as the cfg has it (untrimmed, as the game opens it).
    pub path: String,
    pub file: PathBuf,
    /// Whether the `.o3d` exists; if not, the game skips the line.
    pub exists: bool,
    /// Its place in the game's mesh list (existing ones only), or -1.
    pub index: i64,
    /// Its place among the cfg's `[mesh]` lines (missing ones too).
    pub cfg_index: usize,
    pub line: usize,
    /// `[viewpoint]`; 0 when missing.
    pub viewpoint: i64,
    /// Its `[LOD]` group (from 0), or -1 before the first.
    pub lod: i64,
    /// All `[visible] <var> <value>`; all true = shown.
    pub visible: Vec<(String, i64)>,
    pub clicks: Vec<String>,
    pub ident: Option<String>,
    pub parent: Option<String>,
    pub anims: Vec<Anim>,
    pub shadow: bool,
    pub materials: Vec<Matl>,
}

/// One `[texttexture]` or `[texttexture_enh]`, with its fields as the game reads them.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    /// From 0, over ALL blocks in file order: what `[useTextTexture]` counts.
    pub index: usize,
    pub line: usize,
    pub enh: bool,
    /// Field 1: the string variable (untrimmed).
    pub source: String,
    /// Field 2: the font's name, untrimmed and case-sensitive.
    pub font: String,
    pub w: i64,
    pub h: i64,
    /// Field 5, as a byte.
    pub fc: u8,
    pub color: [u8; 3],
    /// Field 9 of `_enh` (a byte), else 0.
    pub align: u8,
    /// Field 10 of `_enh` (a word), else 1.
    pub grid: u16,
    /// All its numbers could be read; if not, the game draws nothing of it.
    pub valid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCfg {
    pub path: PathBuf,
    pub dir: PathBuf,
    pub meshes: Vec<Mesh>,
    pub text: Vec<TextBlock>,
    /// The `[LOD]` groups' thresholds, in order.
    pub lods: Vec<f64>,
}

/// How many lines a heading takes. Only the headings read here.
fn length_of(h: &str) -> Option<usize> {
    Some(match h {
        "[mesh]" => 1,
        "[matl]" => 2,
        "[matl_change]" => 3,
        "[matl_item]" => 0,
        "[useTextTexture]" => 1,
        "[texttexture]" => 8,
        "[texttexture_enh]" => 10,
        "[visible]" => 2,
        "[mouseevent]" => 1,
        "[mesh_ident]" => 1,
        "[animparent]" => 1,
        "[newanim]" => 0,
        "[viewpoint]" => 1,
        "[isshadow]" => 0,
        "[LOD]" => 1,
        "[matl_alpha]" => 1,
        "[matl_freetex]" => 2,
        "[matl_transmap]" => 1,
        "[matl_texadress_clamp]" => 0,
        "[matl_texadress_border]" => 0,
        "[matl_allcolor]" => 14,
        "[matl_lightmap]" => 2,
        "[matl_nightmap]" => 1,
        "[matl_envmap]" => 2,
        "[texcoordtransX]" => 1,
        "[texcoordtransY]" => 1,
        "[matl_noZwrite]" => 0,
        _ => return None,
    })
}

/// The words of a `[newanim]`, with the lines after them. The game does not read them as one
/// block: between `origin_trans` and `origin_rot_x` there is often an empty line, and a
/// heading may follow at once. They count for the mesh's last `[newanim]`, wherever they are.
fn anim_word(w: &str) -> Option<usize> {
    Some(match w {
        "origin_trans" => 3,
        "origin_from_mesh" => 0,
        "origin_rot_x" | "origin_rot_y" | "origin_rot_z" => 1,
        "anim_trans" | "anim_rot" => 2,
        "maxspeed" | "delay" | "offset" => 1,
        _ => return None,
    })
}

/// An integer as Delphi's `_ValLong` reads it: only spaces in front, a sign, then decimal or
/// hexadecimal (`$`, `x`, `X`, `0x`) to the end. Anything else - a trailing tab or space, an
/// empty line - is an error.
pub fn str_to_int(line: &str) -> Option<i64> {
    let s = match line.find('\0') {
        Some(i) => &line[..i],
        None => line,
    };
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i] == b' ' {
        i += 1;
    }
    let mut negative = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        negative = b[i] == b'-';
        i += 1;
    }
    let mut hex = false;
    if i < b.len() && (b[i] == b'$' || b[i] == b'x' || b[i] == b'X') {
        hex = true;
        i += 1;
    } else if i + 1 < b.len() && b[i] == b'0' && (b[i + 1] == b'x' || b[i + 1] == b'X') {
        hex = true;
        i += 2;
    }
    if i >= b.len() {
        return None;
    }
    let mut v: i64 = 0;
    for &c in &b[i..] {
        let d = match c {
            b'0'..=b'9' => (c - b'0') as i64,
            b'a'..=b'f' if hex => (c - b'a' + 10) as i64,
            b'A'..=b'F' if hex => (c - b'A' + 10) as i64,
            _ => return None,
        };
        v = v.checked_mul(if hex { 16 } else { 10 })?.checked_add(d)?;
        if v > u32::MAX as i64 {
            return None;
        }
    }
    if hex {
        // (eight hex digits are read as an Integer: `$FFFFFFFF` is -1)
        let w = v as u32 as i32 as i64;
        return Some(if negative { -w } else { w });
    }
    if negative {
        (v <= 2_147_483_648).then_some(-v)
    } else {
        (v <= 2_147_483_647).then_some(v)
    }
}

/// A number of a field; the game writes a point. Unreadable: `None`.
fn number(line: &str) -> Option<f64> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// An integer of a field, with some slack (for `[visible]` and the numbers).
fn integer(line: &str) -> Option<i64> {
    str_to_int(line).or_else(|| number(line).filter(|v| v.fract() == 0.0).map(|v| v as i64))
}

/// The cfg of a model, or `None` when it cannot be read.
pub fn read(model_cfg: &Path) -> Option<ModelCfg> {
    let lines = text::lines_of(&std::fs::read(model_cfg).ok()?);
    Some(parse(model_cfg, &lines))
}

/// The work itself, apart from the disk (for a test with lines of its own). The `.o3d`s are
/// looked for beside `model_cfg`.
pub fn parse(model_cfg: &Path, lines: &[String]) -> ModelCfg {
    let dir = model_cfg.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut meshes: Vec<Mesh> = Vec::new();
    let mut texts: Vec<TextBlock> = Vec::new();
    let mut lods: Vec<f64> = Vec::new();
    let mut mesh_index: i64 = 0;
    let mut lod: i64 = -1;
    // (the material being read, and whether an item of it is open)
    let mut matl: Option<usize> = None;
    let mut item: Option<usize> = None;
    let mut anim: Option<usize> = None;
    let r = |i: usize| -> &str { lines.get(i).map(String::as_str).unwrap_or("") };

    let mut i = 0;
    while i < lines.len() {
        let head = lines[i].as_str();
        let Some(len) = length_of(head) else {
            // a word of a [newanim], or a comment
            if let (Some(n), Some(a)) = (anim_word(head), anim) {
                if let Some(m) = meshes.last_mut() {
                    let an = &mut m.anims[a];
                    match head {
                        "origin_trans" => {
                            // OMSI's axes (x right, y forward, z up) to the o3d's (x, z, y)
                            an.origin = Some([number(r(i + 1)).unwrap_or(0.0), number(r(i + 3)).unwrap_or(0.0), number(r(i + 2)).unwrap_or(0.0)])
                        }
                        "origin_from_mesh" => an.origin = None,
                        "origin_rot_x" | "origin_rot_y" | "origin_rot_z" => {
                            if let Some(angle) = number(r(i + 1)).filter(|a| *a != 0.0) {
                                let axis = match head {
                                    "origin_rot_x" => Axis::X,
                                    "origin_rot_y" => Axis::Y,
                                    _ => Axis::Z,
                                };
                                an.rotations.push((axis, angle));
                            }
                        }
                        "anim_trans" => an.shift = Some((r(i + 1).trim().to_string(), number(r(i + 2)).unwrap_or(0.0))),
                        "anim_rot" => an.turn = Some((r(i + 1).trim().to_string(), number(r(i + 2)).unwrap_or(0.0))),
                        _ => {}
                    }
                }
                i += n;
            }
            i += 1;
            continue;
        };
        let arg = |n: usize| r(i + 1 + n);
        match head {
            "[texttexture]" | "[texttexture_enh]" => {
                let enh = head == "[texttexture_enh]";
                let w = str_to_int(arg(2));
                let h = str_to_int(arg(3));
                let fc = str_to_int(arg(4));
                let rgb = [str_to_int(arg(5)), str_to_int(arg(6)), str_to_int(arg(7))];
                let align = if enh { str_to_int(arg(8)) } else { Some(0) };
                let grid = if enh { str_to_int(arg(9)) } else { Some(1) };
                let valid = [w, h, fc, rgb[0], rgb[1], rgb[2], align, grid].iter().all(Option::is_some) && w.unwrap_or(0) > 0 && h.unwrap_or(0) > 0;
                texts.push(TextBlock {
                    index: texts.len(),
                    line: i + 1,
                    enh,
                    source: arg(0).to_string(),
                    font: arg(1).to_string(),
                    w: w.unwrap_or(0),
                    h: h.unwrap_or(0),
                    fc: (fc.unwrap_or(0) & 0xff) as u8,
                    // (unclamped in the game: colour = (R << 16) | (G << 8) | B; here within 0..255)
                    color: rgb.map(|c| (c.unwrap_or(0) & 0xff) as u8),
                    align: (align.unwrap_or(0) & 0xff) as u8,
                    grid: (grid.unwrap_or(1) & 0xffff) as u16,
                    valid,
                });
            }
            "[LOD]" => {
                lods.push(number(arg(0)).unwrap_or(f64::NAN));
                lod = lods.len() as i64 - 1;
                matl = None;
                item = None;
                anim = None;
                // (what follows belongs to no mesh until the next)
                meshes.push(Mesh::none());
            }
            "[mesh]" => {
                let path = arg(0).to_string();
                let file = path.split(['\\', '/']).filter(|p| !p.is_empty()).fold(dir.clone(), |d, p| d.join(p));
                // a FILE, as Delphi's FileExists (which says no to a folder)
                let exists = !path.trim().is_empty() && file.is_file();
                let cfg_index = meshes.iter().filter(|m| m.line > 0).count();
                meshes.push(Mesh {
                    path,
                    file,
                    exists,
                    index: if exists { mesh_index } else { -1 },
                    cfg_index,
                    line: i + 1,
                    viewpoint: 0,
                    lod,
                    visible: Vec::new(),
                    clicks: Vec::new(),
                    ident: None,
                    parent: None,
                    anims: Vec::new(),
                    shadow: false,
                    materials: Vec::new(),
                });
                if exists {
                    mesh_index += 1;
                }
                matl = None;
                item = None;
                anim = None;
            }
            _ => {
                if let Some(m) = meshes.last_mut().filter(|m| m.line > 0) {
                    apply(m, head, &arg, i + 1, &mut matl, &mut item, &mut anim);
                }
            }
        }
        i += len + 1;
    }
    meshes.retain(|m| m.line > 0);
    ModelCfg { path: model_cfg.to_path_buf(), dir, meshes, text: texts, lods }
}

impl Mesh {
    /// A place holder after a `[LOD]`: what follows it belongs to no mesh.
    fn none() -> Mesh {
        Mesh {
            path: String::new(),
            file: PathBuf::new(),
            exists: false,
            index: -1,
            cfg_index: 0,
            line: 0,
            viewpoint: 0,
            lod: -1,
            visible: Vec::new(),
            clicks: Vec::new(),
            ident: None,
            parent: None,
            anims: Vec::new(),
            shadow: false,
            materials: Vec::new(),
        }
    }
}

/// A heading that belongs to the mesh being read.
fn apply<'a>(m: &mut Mesh, head: &str, arg: &dyn Fn(usize) -> &'a str, line: usize, matl: &mut Option<usize>, item: &mut Option<usize>, anim: &mut Option<usize>) {
    match head {
        "[visible]" => {
            let var = arg(0).trim();
            if let (false, Some(v)) = (var.is_empty(), integer(arg(1))) {
                m.visible.push((var.to_string(), v));
            }
            return;
        }
        "[mouseevent]" => {
            let a = arg(0).trim();
            if !a.is_empty() {
                m.clicks.push(a.to_string());
            }
            return;
        }
        "[mesh_ident]" => {
            m.ident = Some(arg(0).trim().to_string());
            return;
        }
        "[animparent]" => {
            m.parent = Some(arg(0).trim().to_string());
            return;
        }
        "[viewpoint]" => {
            m.viewpoint = integer(arg(0)).unwrap_or(0);
            return;
        }
        "[isshadow]" => {
            m.shadow = true;
            return;
        }
        "[newanim]" => {
            m.anims.push(Anim { origin: Some([0.0; 3]), rotations: Vec::new(), shift: None, turn: None, line });
            *anim = Some(m.anims.len() - 1);
            return;
        }
        "[matl]" | "[matl_change]" => {
            let change = head == "[matl_change]";
            m.materials.push(Matl {
                change,
                texture: arg(0).trim().to_string(),
                nth: integer(arg(1)).unwrap_or(0),
                variable: if change { Some(arg(2).trim().to_string()).filter(|v| !v.is_empty()) } else { None },
                text: None,
                address: None,
                state: MatlState::default(),
                items: Vec::new(),
                line,
            });
            *matl = Some(m.materials.len() - 1);
            *item = None;
            return;
        }
        _ => {}
    }
    let Some(k) = *matl else { return };
    let mt = &mut m.materials[k];
    match head {
        "[matl_item]" => {
            mt.items.push(MatlState::default());
            *item = Some(mt.items.len() - 1);
            return;
        }
        "[useTextTexture]" => {
            if let Some(v) = integer(arg(0)).filter(|v| *v >= 0) {
                mt.text = Some(v as usize);
            }
            return;
        }
        "[matl_texadress_clamp]" => {
            mt.address = Some(Address::Clamp);
            return;
        }
        "[matl_texadress_border]" => {
            mt.address = Some(Address::Border);
            return;
        }
        _ => {}
    }
    let st = match *item {
        Some(j) => &mut mt.items[j],
        None => &mut mt.state,
    };
    match head {
        "[matl_alpha]" => {
            st.alpha = Some(match integer(arg(0)) {
                Some(1) => 1,
                Some(2) => 2,
                _ => 0,
            })
        }
        "[matl_freetex]" => st.freetex = Some((arg(0).trim().to_string(), arg(1).trim().to_string())),
        "[matl_transmap]" => st.transmap = Some(arg(0).trim().to_string()),
        "[matl_allcolor]" => {
            let v: Vec<f64> = (0..14).map(|k| number(arg(k)).unwrap_or(f64::NAN)).collect();
            if v[..4].iter().all(|x| x.is_finite()) {
                st.allcolor = Some(v);
            }
        }
        "[matl_lightmap]" => {
            let var = arg(1).trim();
            // (the second line is a variable or empty; a heading or a number it is not)
            let var = (!var.is_empty() && !var.starts_with('[') && var.parse::<f64>().is_err()).then(|| var.to_string());
            st.lightmap = Some((arg(0).trim().to_string(), var));
        }
        "[matl_nightmap]" => st.nightmap = Some(arg(0).trim().to_string()),
        "[texcoordtransX]" => st.texcoord_x = Some(arg(0).trim().to_string()),
        "[texcoordtransY]" => st.texcoord_y = Some(arg(0).trim().to_string()),
        _ => {}
    }
}

/// Whether the game draws this mesh in the view from inside. `[viewpoint]` is a bit field
/// (0 always, 1 outside, 2 inside, 4 the AI bus): empty, 0, or bit 1 on. And of the `[LOD]`
/// groups only the one with the biggest threshold, or what stands before the first.
pub fn inside_view(cfg: &ModelCfg, mesh: &Mesh) -> bool {
    if mesh.viewpoint != 0 && mesh.viewpoint & 2 == 0 {
        return false;
    }
    if mesh.lod < 0 || cfg.lods.is_empty() {
        return true;
    }
    let mut best = 0;
    for k in 1..cfg.lods.len() {
        let (a, b) = (cfg.lods[k], cfg.lods[best]);
        if a.is_finite() && (!b.is_finite() || a > b) {
            best = k;
        }
    }
    mesh.lod == best as i64
}

/// Which `.o3d` material a `[matl]`/`[matl_change]` means: the `nth` material (from 0) whose
/// texture's name is the same, case-insensitively. Measured: O550_Euro3.cfg's `[matl]
/// Textfeld_AFR_1.tga 0` is material 2 of O550_AFR200_Body.o3d, not material 0; over all
/// model.cfgs "by name" is right 115,914 times and "the global number" 76,421 times.
pub fn material_of(textures: &[String], texture: &str, nth: i64) -> Option<usize> {
    let name = texture.to_lowercase();
    let mut seen = 0;
    for (k, t) in textures.iter().enumerate() {
        if t.trim().to_lowercase() != name {
            continue;
        }
        if seen == nth {
            return Some(k);
        }
        seen += 1;
    }
    None
}

/// Per `.o3d` material what the cfg says of it (`None`: nothing, the `.o3d`'s own material
/// counts, as with the ALMEX screens). Said twice of one material, the last counts.
pub fn material_contexts(mesh: &Mesh, textures: &[String]) -> Vec<Option<Matl>> {
    let mut out: Vec<Option<Matl>> = vec![None; textures.len()];
    for m in &mesh.materials {
        let Some(k) = material_of(textures, &m.texture, m.nth) else { continue };
        out[k] = Some(match out[k].take() {
            Some(before) => merge(before, m),
            None => m.clone(),
        });
    }
    out
}

/// A second word on one material: what it says goes over the first.
fn merge(mut a: Matl, b: &Matl) -> Matl {
    a.change = b.change;
    a.texture = b.texture.clone();
    a.nth = b.nth;
    a.line = b.line;
    if b.variable.is_some() {
        a.variable = b.variable.clone();
    }
    if b.text.is_some() {
        a.text = b.text;
    }
    if b.address.is_some() {
        a.address = b.address;
    }
    let (s, t) = (&mut a.state, &b.state);
    if t.alpha.is_some() {
        s.alpha = t.alpha;
    }
    if t.freetex.is_some() {
        s.freetex = t.freetex.clone();
    }
    if t.transmap.is_some() {
        s.transmap = t.transmap.clone();
    }
    if t.allcolor.is_some() {
        s.allcolor = t.allcolor.clone();
    }
    if t.lightmap.is_some() {
        s.lightmap = t.lightmap.clone();
    }
    if t.nightmap.is_some() {
        s.nightmap = t.nightmap.clone();
    }
    if t.texcoord_x.is_some() {
        s.texcoord_x = t.texcoord_x.clone();
    }
    if t.texcoord_y.is_some() {
        s.texcoord_y = t.texcoord_y.clone();
    }
    if !b.items.is_empty() {
        a.items = b.items.clone();
    }
    a
}

/// One spelling per number variable: the first in the cfg. The game looks variables up
/// case-insensitively, but the plugin gives them back under the name AS ASKED; 24 cfgs of a
/// fleet spell one variable two ways (`vis_Matrix` and `vis_matrix`), and asking one spelling
/// and looking up the other left that part unseen.
pub fn spellings(cfg: &ModelCfg) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut put = |n: &str| {
        if !n.is_empty() {
            out.entry(n.to_lowercase()).or_insert_with(|| n.to_string());
        }
    };
    for m in &cfg.meshes {
        for (v, _) in &m.visible {
            put(v);
        }
        for mt in &m.materials {
            if let Some(v) = &mt.variable {
                put(v);
            }
            for s in std::iter::once(&mt.state).chain(mt.items.iter()) {
                if let Some((_, Some(v))) = &s.lightmap {
                    put(v);
                }
                for v in [&s.texcoord_x, &s.texcoord_y].into_iter().flatten() {
                    put(v);
                }
            }
        }
        for a in &m.anims {
            for (v, _) in [&a.shift, &a.turn].into_iter().flatten() {
                put(v);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(t: &str) -> Vec<String> {
        text::lines_of(t.as_bytes())
    }

    #[test]
    fn delphis_strtoint_takes_spaces_in_front_and_nothing_after() {
        assert_eq!(str_to_int("  12"), Some(12));
        assert_eq!(str_to_int("-7"), Some(-7));
        assert_eq!(str_to_int("$FF"), Some(255));
        assert_eq!(str_to_int("0x10"), Some(16));
        assert_eq!(str_to_int("$FFFFFFFF"), Some(-1));
        assert_eq!(str_to_int("12 "), None);
        assert_eq!(str_to_int("12\t"), None);
        assert_eq!(str_to_int(""), None);
        assert_eq!(str_to_int("1.5"), None);
    }

    #[test]
    fn headings_count_only_as_whole_lines_and_missing_meshes_have_no_index() {
        let dir = std::env::temp_dir().join(format!("hub-devices-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.o3d"), b"x").unwrap();
        std::fs::write(dir.join("c.o3d"), b"x").unwrap();
        let text = "\t[mesh]\r\nindented.o3d\r\n[mesh]\r\na.o3d\r\n[visible]\r\nmenu\r\n3\r\n[mouseevent]\r\nibis_7\r\n\
[matl]\r\nscreen.bmp\r\n1\r\n[useTextTexture]\r\n1\r\n[matl_alpha]\r\n2\r\n[mesh]\r\nmissing.o3d\r\n[mesh]\r\nc.o3d\r\n\
[newanim]\r\norigin_trans\r\n1\r\n2\r\n3\r\n\r\norigin_rot_y\r\n90\r\nanim_trans\r\ntrans_dauer\r\n0.5\r\n\
[texttexture]\r\nvar_a\r\nFont A\r\n256\r\n32\r\n0\r\n255\r\n128\r\n0\r\n[texttexture_enh]\r\nvar_b\r\nFont B\r\n64\r\n16\r\n1\r\n1\r\n2\r\n3\r\n1\r\n4\r\n";
        let cfg = parse(&dir.join("model.cfg"), &lines(text));
        let names: Vec<(&str, i64, usize)> = cfg.meshes.iter().map(|m| (m.path.as_str(), m.index, m.cfg_index)).collect();
        assert_eq!(names, [("a.o3d", 0, 0), ("missing.o3d", -1, 1), ("c.o3d", 1, 2)]);
        let a = &cfg.meshes[0];
        assert_eq!((a.visible.clone(), a.clicks.clone()), (vec![("menu".to_string(), 3)], vec!["ibis_7".to_string()]));
        assert_eq!((a.materials[0].nth, a.materials[0].text, a.materials[0].state.alpha), (1, Some(1), Some(2)));
        let c = &cfg.meshes[2];
        assert_eq!(c.anims[0].origin, Some([1.0, 3.0, 2.0]), "OMSI's axes turned into the o3d's");
        assert_eq!((c.anims[0].rotations.clone(), c.anims[0].shift.clone()), (vec![(Axis::Y, 90.0)], Some(("trans_dauer".to_string(), 0.5))));
        assert_eq!(cfg.text.len(), 2);
        assert_eq!((cfg.text[0].font.as_str(), cfg.text[0].color, cfg.text[0].valid), ("Font A", [255, 128, 0], true));
        assert_eq!((cfg.text[1].enh, cfg.text[1].align, cfg.text[1].grid, cfg.text[1].fc), (true, 1, 4, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_material_is_the_nth_of_its_name_and_the_last_word_counts() {
        let tex: Vec<String> = ["a.bmp", "Textfeld.tga", "b.bmp", "textfeld.tga"].iter().map(|s| s.to_string()).collect();
        assert_eq!(material_of(&tex, "textfeld.tga", 1), Some(3));
        assert_eq!(material_of(&tex, "textfeld.tga", 2), None);
        let cfg = parse(Path::new("/x/model.cfg"), &lines("[mesh]\r\nm.o3d\r\n[matl]\r\nb.bmp\r\n0\r\n[matl_alpha]\r\n1\r\n[matl]\r\nb.bmp\r\n0\r\n[useTextTexture]\r\n0\r\n"));
        let ctx = material_contexts(&cfg.meshes[0], &tex);
        let b = ctx[2].as_ref().unwrap();
        assert_eq!((b.state.alpha, b.text), (Some(1), Some(0)));
        assert!(ctx[0].is_none());
    }

    #[test]
    fn only_the_inside_and_the_nearest_lod_are_drawn() {
        let cfg = parse(Path::new("/x/model.cfg"), &lines("[LOD]\r\n50\r\n[mesh]\r\nnear.o3d\r\n[viewpoint]\r\n1\r\n[mesh]\r\ninside.o3d\r\n[LOD]\r\n10\r\n[mesh]\r\nfar.o3d\r\n"));
        let seen: Vec<bool> = cfg.meshes.iter().map(|m| inside_view(&cfg, m)).collect();
        assert_eq!(seen, [false, true, false]);
    }
}
