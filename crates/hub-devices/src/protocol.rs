//! A device's screen as the app receives it: the form (what is on the screen, where, and when
//! the game shows it) and the stand (what the screen shows now). The career add-on's
//! `shared/scherm.ts`, in English; `docs/COMPANION.md` and the app's `docs/PROTOCOL.md` hold the
//! contract ("The bus's devices, drawn as the game draws them").
//!
//! Coordinates: the screen is a rectangle with `aspect` = width / height in the real world. `s`
//! runs from 0 (left) to 1 (right) and `t` from 0 (top) to 1 (bottom) as the driver sees it;
//! `u` and `v` point into a picture, v = 0 at the top (Direct3D's way), wrapping outside 0..1 per
//! `address`. A text part has a canvas of `w` x `h` pixels instead of a picture: u = x / w.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A condition from `[visible] <var> <value>`: shown when Delphi-round(value of `nums[num]`)
/// equals `value`. A variable the bus lacks counts as true, a value that is no number as false.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Visible {
    pub num: usize,
    pub value: i64,
}

/// What a part draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A picture of the bus (the background, a layer over it).
    Image,
    /// A surface without a (findable) picture, in its material's colour.
    Color,
    /// A `[texttexture]`: a string variable's text in one of OMSI's fonts.
    Text,
    /// A `[scripttexture]` (`\S:n`) the bus's script paints itself: not drawable, left blank.
    Script,
}

/// The layout of one `[texttexture]` or `[texttexture_enh]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpec {
    /// Index into `Form::strs`: where the text comes from.
    pub str: usize,
    /// The texture the game writes the text into, in pixels.
    pub w: i64,
    pub h: i64,
    /// Key into `Form::fonts`; a missing font leaves the texture blank.
    pub font: String,
    /// Field 5 as a bit field: bit 0 the colour from the bitmap, (fc & 3) == 2 opaque only.
    pub fc: u8,
    pub color: [u8; 3],
    /// `_enh` field 9 (0 for a plain block): 0 centre, 1 left, 2 right, 3/4/5 on a gap.
    pub align: u8,
    /// `_enh` field 10 (1 for a plain block): x rounded down to a multiple of this.
    pub grid: u16,
}

/// `[matl_change] <tex> <n> <var>`: at value 0 the part's own picture, at value k item k's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub num: usize,
    pub items: Vec<Option<String>>,
}

/// One part of the screen: a `[mesh]` with one material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub kind: Kind,
    /// The mesh's place in the game's mesh list, or -1.
    pub mesh: i64,
    /// All `[visible]`s of the mesh; all true = shown, none = always.
    pub visible: Vec<Visible>,
    /// s t u v per point, three points a triangle: only triangles facing the driver.
    pub tris: Vec<f64>,
    /// Millimetres towards the driver; drawn from small to large.
    pub depth: f64,
    /// `[matl_alpha]`: 0 opaque, 1 alpha test, 2 blended.
    pub alpha: u8,
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    /// `[matl_freetex]`: index into `strs` whose VALUE is a file; the stand's `f` gives its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freetex: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<Choice>,
    /// A colour part's colour, and an alpha-2 image's opacity (a): 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextSpec>,
}

/// A touch field: a `[mesh]` with a `[mouseevent]` that lies on the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tap {
    /// What the bus's script listens to.
    pub trigger: String,
    /// For screen readers and the list of keys; not drawn.
    pub label: String,
    pub mesh: i64,
    pub visible: Vec<Visible>,
    /// A rectangle in shares of the screen.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Millimetres towards the driver: where they overlap, the front one wins.
    pub depth: f64,
}

/// One character of a font, in the `.oft`'s order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Glyph {
    /// One UTF-16 unit.
    pub ch: String,
    /// x2 - x1; may be negative (the pen goes back and nothing is drawn).
    pub width: i64,
    /// The coverage, max(0, width) x height bytes row by row, base64; missing when the game
    /// could not read the character from its bitmap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<String>,
    /// For a font coloured from its bitmap: r g b per pixel, base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rgb: Option<String>,
}

/// One of OMSI's fonts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub name: String,
    pub height: i64,
    /// The space between two characters.
    pub sep: i64,
    /// File order, doubles kept; glyph 0 is the fallback.
    pub glyphs: Vec<Glyph>,
}

/// Everything that can be on one device's screen. It does not change while the bus is the same.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Form {
    pub version: u32,
    /// Follows from the content (20 hex characters); the app keeps forms by it.
    pub id: String,
    /// The device ([`crate::modules::Module::id`]).
    pub device: String,
    pub aspect: f64,
    pub blank: String,
    /// In drawing order.
    pub parts: Vec<Part>,
    /// The front one last.
    pub taps: Vec<Tap>,
    pub fonts: BTreeMap<String, Font>,
    /// The string variables the stand sends (texts and freetex).
    pub strs: Vec<String>,
    /// The number variables the conditions refer to.
    pub nums: Vec<String>,
    /// What could not be drawn, for the log and the diagnosis.
    pub incomplete: Vec<String>,
}

/// What a screen shows now.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Stand {
    pub v: u64,
    pub form: String,
    /// Per `strs`, as the bus has it (not trimmed).
    pub t: Vec<String>,
    /// Per `nums`; `None`: the bus has it but it is no number.
    pub g: Vec<Option<f64>>,
    /// Indexes into `nums` the bus does not have (always true).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub o: Vec<usize>,
    /// Per `strs`: the picture id of a freetex value, or `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f: Option<Vec<Option<String>>>,
}

/// Rounding as Delphi's `Round` does on a single: to the nearest integer, halves to even.
pub fn round_omsi(x: f64) -> i64 {
    let f = x as f32 as f64;
    let below = f.floor();
    let rest = f - below;
    let below = below as i64;
    if rest > 0.5 || (rest == 0.5 && below % 2 != 0) {
        below + 1
    } else {
        below
    }
}

/// Base64 (standard, padded) of some bytes.
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(ABC[(n >> 18) as usize & 63] as char);
        out.push(ABC[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { ABC[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { ABC[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delphi_rounds_halves_to_even() {
        assert_eq!([0.5, 1.5, 2.5, -0.5, 2.4, 2.6].map(round_omsi), [0, 2, 2, 0, 2, 3]);
    }

    #[test]
    fn base64_pads() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
    }
}
