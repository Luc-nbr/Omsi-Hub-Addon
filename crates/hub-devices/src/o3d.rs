//! OMSI's own 3D model format (`.o3d`), read as the career add-on read it (`core/o3d.ts`):
//! every rule below was counted over the 55,041 `.o3d` files of a full installation there.
//!
//! The header is `0x84 0x19` and a version byte (1, 3, 4, 5 or 7 occur). Version 1 starts with
//! blocks at once; from version 3 a flag byte follows, and from version 4 four more bytes. Each
//! block starts with an id byte:
//!
//! * `0x17` vertices: a count, then 8 floats each - position xyz, normal xyz, uv (32 bytes);
//! * `0x49` triangles: a count, then three vertex indices and a material index each;
//! * `0x26` materials: **always** a u16 count (also in version 7), then 11 floats each (diffuse
//!   rgba, specular rgb, emissive rgb, power) and the texture's name as a length byte and
//!   Latin-1 characters;
//! * `0x79` a 4x4 matrix of floats, row by row, the translation in elements 12, 13, 14. The
//!   vertices are already where they belong in the vehicle: the matrix only says what the part
//!   turns about (with it applied once more, the ALMEX of the Hamburg bus lay twice as far
//!   from the bus as it is);
//! * `0x54` bones: only people and bellows.
//!
//! Counts of vertices and triangles are u16 in version 1 and u32 from version 3. Bit 0 of the
//! flag byte makes the triangles' indices u32 (14 bytes a triangle instead of 8); it has to be
//! read, it does not come with big models by itself.
//!
//! From version 4 the word at byte 4 is `ffffffff` in an open file and the key a maker shuffled
//! the vertices with otherwise: such a file is [`O3d::encrypted`], its triangles and materials
//! read right but its positions are mixed up per vertex (and its uvs are not to be trusted).
//!
//! Axes: x the width, y the height, z the length, in metres.
//!
//! Nothing here panics: a file that cannot be read gives `None`, a broken one what was read of
//! it before the break.

/// One material: its colours and its texture.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub diffuse: [f32; 4],
    pub specular: [f32; 3],
    pub emissive: [f32; 3],
    pub power: f32,
    /// The texture's file name, without a folder (it is in `Texture\` beside the model); empty:
    /// a plain colour.
    pub texture: String,
}

/// A model read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct O3d {
    pub version: u8,
    /// Three floats a vertex.
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    /// Two floats a vertex.
    pub uvs: Vec<f32>,
    /// Three vertex indices a triangle.
    pub triangles: Vec<u32>,
    /// The material of each triangle; not always a valid index (a model without materials).
    pub material_of: Vec<u16>,
    pub materials: Vec<Material>,
    /// The 0x79 matrix, row by row: the point the part turns about, not its place.
    pub transform: Option<[f32; 16]>,
    /// The file was encrypted (its key cleared before reading): positions mixed up per vertex.
    pub encrypted: bool,
}

const VERTICES: u8 = 0x17;
const TRIANGLES: u8 = 0x49;
const MATERIALS: u8 = 0x26;
const MATRIX: u8 = 0x79;
const BONES: u8 = 0x54;

/// The endings a texture's name has; only for the repair of a broken name's length (below).
const TEXTURE_ENDINGS: [&str; 9] = [".dds", ".bmp", ".tga", ".png", ".jpg", ".jpeg", ".dxt", ".gif", ".tif"];

/// A model from its bytes. An encrypted one is read with its key cleared ([`O3d::encrypted`]).
pub fn parse(bytes: &[u8]) -> Option<O3d> {
    if bytes.len() < 4 || bytes[0] != 0x84 || bytes[1] != 0x19 {
        return None;
    }
    let mut owned;
    let mut b = bytes;
    let encrypted = b[2] >= 4 && b.len() >= 8 && u32le(b, 4) != 0xffff_ffff;
    if encrypted {
        owned = bytes.to_vec();
        owned[4..8].copy_from_slice(&[0xff; 4]);
        b = &owned;
        // (the borrow below must not outlive `owned`)
        let strict = read(b, false);
        let model = if strict.1 { strict.0 } else { read(b, true).0.or(strict.0) };
        return model.map(|mut m| {
            m.encrypted = true;
            m
        });
    }
    // Two readings: the strict one keeps to the texture name's length byte, the lenient one
    // cuts the name at its extension. That saves ten files whose byte was spoilt by hand
    // (`KW.dds` with length 19), but must not become the rule: some textures really are called
    // `str_asphdrk.bmp.003`. So lenient only when strict does not come out.
    let strict = read(b, false);
    if strict.1 {
        return strict.0;
    }
    let lenient = read(b, true);
    if lenient.1 {
        lenient.0
    } else {
        strict.0
    }
}

/// A model from a file.
pub fn read_file(path: &std::path::Path) -> Option<O3d> {
    parse(&std::fs::read(path).ok()?)
}

fn u16le(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn u32le(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}
fn f32le(b: &[u8], p: usize) -> f32 {
    f32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

/// The reading itself: the model so far, and whether the whole file came out.
fn read(b: &[u8], lenient: bool) -> (Option<O3d>, bool) {
    let version = b[2];
    let mut p = if version >= 4 {
        8
    } else if version >= 3 {
        4
    } else {
        3
    };
    if p > b.len() {
        return (None, false);
    }
    let wide = version >= 3;
    let long_index = version >= 3 && b[3] & 1 == 1;
    let tri_step = if long_index { 14 } else { 8 };
    let mut m = O3d { version, ..O3d::default() };
    let count = |p: usize| -> Option<usize> {
        if wide {
            (p + 4 <= b.len()).then(|| u32le(b, p) as usize)
        } else {
            (p + 2 <= b.len()).then(|| u16le(b, p) as usize)
        }
    };
    let cw = if wide { 4 } else { 2 };
    while p < b.len() {
        let tag = b[p];
        p += 1;
        match tag {
            VERTICES => {
                let Some(n) = count(p) else { return (Some(m), false) };
                p += cw;
                if n.checked_mul(32).is_none_or(|len| p + len > b.len()) {
                    return (Some(m), false);
                }
                m.vertices.reserve(n * 3);
                m.normals.reserve(n * 3);
                m.uvs.reserve(n * 2);
                for i in 0..n {
                    let o = p + i * 32;
                    for k in 0..3 {
                        m.vertices.push(f32le(b, o + k * 4));
                        m.normals.push(f32le(b, o + 12 + k * 4));
                    }
                    m.uvs.push(f32le(b, o + 24));
                    m.uvs.push(f32le(b, o + 28));
                }
                p += n * 32;
            }
            TRIANGLES => {
                let Some(n) = count(p) else { return (Some(m), false) };
                p += cw;
                if n.checked_mul(tri_step).is_none_or(|len| p + len > b.len()) {
                    return (Some(m), false);
                }
                m.triangles.reserve(n * 3);
                m.material_of.reserve(n);
                for i in 0..n {
                    let o = p + i * tri_step;
                    if long_index {
                        m.triangles.extend([u32le(b, o), u32le(b, o + 4), u32le(b, o + 8)]);
                        m.material_of.push(u16le(b, o + 12));
                    } else {
                        m.triangles.extend([u16le(b, o) as u32, u16le(b, o + 2) as u32, u16le(b, o + 4) as u32]);
                        m.material_of.push(u16le(b, o + 6));
                    }
                }
                p += n * tri_step;
            }
            MATERIALS => {
                if p + 2 > b.len() {
                    return (Some(m), false);
                }
                let n = u16le(b, p) as usize;
                p += 2;
                for _ in 0..n {
                    if p + 45 > b.len() {
                        return (Some(m), false);
                    }
                    let f = |j: usize| f32le(b, p + j * 4);
                    let mut mat = Material { diffuse: [f(0), f(1), f(2), f(3)], specular: [f(4), f(5), f(6)], emissive: [f(7), f(8), f(9)], power: f(10), texture: String::new() };
                    p += 44;
                    let promised = b[p] as usize;
                    p += 1;
                    let len = if lenient { name_length(b, p, promised) } else { promised };
                    if p + len > b.len() {
                        return (Some(m), false);
                    }
                    mat.texture = b[p..p + len].iter().map(|&c| c as char).collect();
                    p += len;
                    m.materials.push(mat);
                }
            }
            MATRIX => {
                if p + 64 > b.len() {
                    return (Some(m), false);
                }
                let mut t = [0f32; 16];
                for (j, v) in t.iter_mut().enumerate() {
                    *v = f32le(b, p + j * 4);
                }
                m.transform = Some(t);
                p += 64;
            }
            BONES => {
                if p + 2 > b.len() {
                    return (Some(m), false);
                }
                let n = u16le(b, p) as usize;
                p += 2;
                for _ in 0..n {
                    if p >= b.len() {
                        return (Some(m), false);
                    }
                    let len = b[p] as usize;
                    p += 1 + len;
                    if p + 2 > b.len() {
                        return (Some(m), false);
                    }
                    let weights = u16le(b, p) as usize;
                    p += 2 + weights * 6;
                    if p > b.len() {
                        return (Some(m), false);
                    }
                }
            }
            _ => return (Some(m), false),
        }
    }
    (Some(m), true)
}

/// The repair of a spoilt length byte: the name cut at its extension. It looks a little past
/// the promise, for it goes both ways (`Schild_Stadtwerke.dds` with 27, six too many;
/// `OVR_Betriebshof_Zaun.dds` with 21, three too few). Everything before the extension has to
/// be readable text, else the extension is chance and the byte stands.
fn name_length(b: &[u8], start: usize, promised: usize) -> usize {
    let end = b.len().min(start + promised + 24);
    let piece: String = b[start.min(end)..end].iter().map(|&c| (c as char).to_ascii_lowercase()).collect();
    let mut cut: Option<usize> = None;
    for ext in TEXTURE_ENDINGS {
        if let Some(i) = piece.find(ext) {
            let e = i + ext.len();
            if cut.is_none_or(|c| e < c) {
                cut = Some(e);
            }
        }
    }
    let Some(cut) = cut else { return promised };
    if b[start..start + cut].iter().any(|&c| !(0x20..=0x7e).contains(&c)) {
        return promised;
    }
    cut
}

/// Writes a model in version 1 (for the tests and the synthetic test bus): vertices with their
/// uvs (normals left 0), triangles with their material, materials (diffuse and texture).
pub fn write(vertices: &[[f32; 3]], uvs: &[[f32; 2]], triangles: &[([u16; 3], u16)], materials: &[([f32; 4], &str)]) -> Vec<u8> {
    let mut out = vec![0x84, 0x19, 1];
    out.push(VERTICES);
    out.extend((vertices.len() as u16).to_le_bytes());
    for (v, uv) in vertices.iter().zip(uvs) {
        for x in v {
            out.extend(x.to_le_bytes());
        }
        out.extend([0u8; 12]);
        out.extend(uv[0].to_le_bytes());
        out.extend(uv[1].to_le_bytes());
    }
    out.push(TRIANGLES);
    out.extend((triangles.len() as u16).to_le_bytes());
    for (t, mat) in triangles {
        for i in t {
            out.extend(i.to_le_bytes());
        }
        out.extend(mat.to_le_bytes());
    }
    out.push(MATERIALS);
    out.extend((materials.len() as u16).to_le_bytes());
    for (diffuse, texture) in materials {
        for x in diffuse.iter().chain([0f32; 7].iter()) {
            out.extend(x.to_le_bytes());
        }
        out.push(texture.len() as u8);
        out.extend(texture.bytes());
    }
    out.push(MATRIX);
    for x in [1f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.] {
        out.extend(x.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_model_reads_back_and_a_broken_one_gives_what_was_read() {
        let bytes = write(&[[0.0, 1.0, 2.0], [1.0, 1.0, 2.0], [0.0, 2.0, 2.0]], &[[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]], &[([0, 1, 2], 0)], &[([1.0, 1.0, 1.0, 1.0], "screen.bmp")]);
        let m = parse(&bytes).unwrap();
        assert_eq!((m.version, m.vertices.len(), m.triangles, m.material_of.clone()), (1, 9, vec![0, 1, 2], vec![0]));
        assert_eq!(m.materials[0].texture, "screen.bmp");
        assert_eq!(m.uvs[1], 1.0);
        assert!(m.transform.is_some() && !m.encrypted);
        // cut off in the materials: the geometry is still there
        let cut = parse(&bytes[..bytes.len() - 70]).unwrap();
        assert_eq!(cut.triangles.len(), 3);
        assert!(parse(b"no model").is_none());
    }

    #[test]
    fn a_name_with_a_spoilt_length_is_cut_at_its_extension() {
        let mut bytes = write(&[[0.0; 3]; 3], &[[0.0; 2]; 3], &[([0, 1, 2], 0)], &[([1.0; 4], "KW.dds")]);
        // the length byte of "KW.dds" says 19; the matrix block follows the name
        let at = bytes.iter().rposition(|&b| b == 6).unwrap();
        bytes[at] = 19;
        let m = parse(&bytes).unwrap();
        assert_eq!(m.materials[0].texture, "KW.dds");
    }

    #[test]
    fn an_encrypted_model_is_read_with_its_key_cleared_and_said_so() {
        let mut bytes = write(&[[0.0; 3]; 3], &[[0.0; 2]; 3], &[([0, 1, 2], 0)], &[([1.0; 4], "a.dds")]);
        // version 4: a flag byte and the key after the header
        bytes[2] = 4;
        bytes.splice(3..3, [0u8, 0x29, 0x3d, 0, 0]);
        // (version 4 counts are u32)
        let m = parse(&bytes);
        assert!(m.is_some_and(|m| m.encrypted));
    }
}
