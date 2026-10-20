//! OMSI's fonts (`Fonts\*.oft` and two bitmaps), read as Omsi.exe 2.3.004 reads them (the career
//! add-on's `core/oft.ts`, where every rule was traced in the machine code and counted over 698
//! `.oft` files). Where the game does something odd, so do we:
//!
//! * the characters are read byte by byte, not as Windows-1252 (0x80 is U+0080, not the euro);
//!   after `FF FE` the file is UTF-16LE. The game knows no UTF-8 BOM;
//! * a line runs to the CR, after which exactly one unit (the LF) is skipped; a lone LF belongs
//!   to the line;
//! * tags count only literally: `[newfont]` and `[char]` without anything before them;
//! * nothing is trimmed: the character line ` ` IS the space;
//! * numbers go through StrToInt; when that fails the game drops the font it is reading and the
//!   rest of the file, the fonts already done stay;
//! * double characters stay, in file order (the first wins); character 0 is the fallback;
//! * the fonts of all files go in one list in the order FindFirst gives the files, the first of
//!   a double name wins (case-sensitively).
//!
//! The bitmaps are read as raw bytes, as `DrawChar` does: byte `(x1 + j) * 3` of the row, without
//! looking at how many bytes a pixel has. The coverage is byte 0 of the three (with 24 bits that
//! is BLUE), the colour of a full-colour font red = byte 2, green = 1, blue = 0. A character the
//! game cannot read (a row outside the bitmap, a column beyond 32767 bytes) gets no `alpha`. Only
//! bitmaps inside the OMSI folder are read, of at most 128 MB.

use crate::cfg::str_to_int;
use crate::protocol::{base64, Font, Glyph};
use crate::scripts::by_name;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One character as the game keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct OftChar {
    /// One UTF-16 unit.
    pub ch: u16,
    pub x: i64,
    pub y: i64,
    /// x2 - x1; may be negative.
    pub width: i64,
}

/// One font of an `.oft`.
#[derive(Debug, Clone, PartialEq)]
pub struct OftFont {
    /// Literally the line after `[newfont]`: so the game compares it with the cfg.
    pub name: String,
    /// The colour bitmap, relative to the OMSI folder (`Fonts/` + the line).
    pub bitmap: String,
    /// The alpha bitmap, the same.
    pub alpha: String,
    pub height: i64,
    pub sep: i64,
    /// File order, doubles kept. Never empty.
    pub chars: Vec<OftChar>,
}

/// The code units of the file as the game sees them.
fn units_of(bytes: &[u8]) -> Vec<u16> {
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        bytes[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
    } else {
        bytes.iter().map(|b| *b as u16).collect()
    }
}

struct Lines {
    u: Vec<u16>,
    p: usize,
}

impl Lines {
    fn eof(&self) -> bool {
        self.p >= self.u.len()
    }

    /// ReadLine: to the CR, then one unit skipped.
    fn line(&mut self) -> String {
        let start = self.p;
        while self.p < self.u.len() {
            let c = self.u[self.p];
            self.p += 1;
            if c == 0x0d {
                let end = self.p - 1;
                if self.p < self.u.len() {
                    self.p += 1;
                }
                return String::from_utf16_lossy(&self.u[start..end]);
            }
        }
        String::from_utf16_lossy(&self.u[start..self.p])
    }

    /// The first unit of the next line, as it is (a lone surrogate too).
    fn first_unit(&mut self) -> Option<u16> {
        let first = self.u.get(self.p).copied().filter(|c| *c != 0x0d);
        self.line();
        first
    }
}

/// One `.oft` read as the game reads it: its fonts, and why it stopped halfway if it did.
pub fn parse_oft(bytes: &[u8]) -> (Vec<OftFont>, Option<String>) {
    let mut r = Lines { u: units_of(bytes), p: 0 };
    let mut fonts = Vec::new();
    let mut busy = OftFont { name: String::new(), bitmap: String::new(), alpha: String::new(), height: 0, sep: 0, chars: Vec::new() };
    while !r.eof() {
        let line = r.line();
        if line == "[newfont]" {
            // (a font without characters does not exist for the game)
            if !busy.chars.is_empty() {
                fonts.push(busy.clone());
                busy.chars.clear();
            }
            busy.name = r.line();
            busy.bitmap = format!("Fonts/{}", r.line());
            busy.alpha = format!("Fonts/{}", r.line());
            let Some(h) = str_to_int(&r.line()) else { return (fonts, Some(format!("height of '{}' is no number", busy.name))) };
            busy.height = h;
            let Some(s) = str_to_int(&r.line()) else { return (fonts, Some(format!("sep of '{}' is no number", busy.name))) };
            busy.sep = s;
        } else if line == "[char]" {
            let ch = r.first_unit();
            let (x1, x2, y) = (r.line(), r.line(), r.line());
            let Some(ch) = ch else { return (fonts, Some(format!("empty character in '{}'", busy.name))) };
            let (Some(x), Some(top), Some(right)) = (str_to_int(&x1), str_to_int(&y), str_to_int(&x2)) else {
                return (fonts, Some(format!("a coordinate in '{}' is no number", busy.name)));
            };
            let width = right - x;
            if i32::try_from(width).is_err() {
                return (fonts, Some(format!("a width in '{}' overflows", busy.name)));
            }
            busy.chars.push(OftChar { ch, x, y: top, width });
        }
    }
    if !busy.chars.is_empty() {
        fonts.push(busy);
    }
    (fonts, None)
}

/// A bitmap as TBitmap has it in memory.
struct RawBitmap {
    data: Vec<u8>,
    rows: usize,
    row: usize,
    /// Stored bottom-up (a positive height in the header).
    flipped: bool,
}

const MAX_BITMAP: u64 = 128 * 1024 * 1024;

/// A bitmap of an `.oft` (`Fonts/` + the line), only when it stays inside the OMSI folder.
fn bitmap_path(omsi_dir: &Path, line: &str) -> Option<PathBuf> {
    let mut parts: Vec<&str> = Vec::new();
    for p in line.split(['\\', '/']) {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ if p.contains(':') => return None,
            _ => parts.push(p),
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.iter().fold(omsi_dir.to_path_buf(), |d, p| d.join(p)))
}

fn read_bitmap(path: &Path) -> Option<RawBitmap> {
    if !path.to_string_lossy().to_lowercase().ends_with(".bmp") || std::fs::metadata(path).ok()?.len() > MAX_BITMAP {
        return None;
    }
    let b = std::fs::read(path).ok()?;
    if b.len() < 54 || &b[..2] != b"BM" {
        return None;
    }
    let u32at = |p: usize| u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]);
    let start = u32at(10) as usize;
    let header = u32at(14);
    let width = u32at(18) as i32;
    let height = u32at(22) as i32;
    let bpp = u16::from_le_bytes([b[28], b[29]]) as usize;
    let compression = u32at(30);
    if header < 40 || width <= 0 || height == 0 || bpp == 0 || (compression != 0 && compression != 3) {
        return None;
    }
    let row = (width as usize * bpp).div_ceil(32) * 4;
    let rows = height.unsigned_abs() as usize;
    if start + row * rows > b.len() {
        return None;
    }
    Some(RawBitmap { data: b[start..start + row * rows].to_vec(), rows, row, flipped: height > 0 })
}

/// The bytes of one character row by row, `channels` a pixel (1: byte 0; 3: bytes 2, 1, 0), or
/// `None` where the game would stop.
fn char_bytes(bmp: Option<&RawBitmap>, c: &OftChar, height: i64, channels: usize) -> Option<Vec<u8>> {
    let bmp = bmp?;
    let w = c.width.max(0) as usize;
    let h = height.max(0) as usize;
    let top = if channels == 3 { 2 } else { 0 };
    if h > 0 && (c.y < 0 || c.y as usize + h > bmp.rows) {
        return None;
    }
    for j in 0..w as i64 {
        let col = (c.x + j) * 3;
        if col < 0 || col + top > 0x7fff {
            return None;
        }
    }
    let mut out = vec![0u8; w * h * channels];
    for i in 0..h {
        let y = c.y as usize + i;
        let start = (if bmp.flipped { bmp.rows - 1 - y } else { y }) * bmp.row;
        for j in 0..w {
            let src = start + (c.x as usize + j) * 3;
            let at = |k: usize| bmp.data.get(src + k).copied().unwrap_or(0);
            let dst = (i * w + j) * channels;
            if channels == 1 {
                out[dst] = at(0);
            } else {
                out[dst] = at(2);
                out[dst + 1] = at(1);
                out[dst + 2] = at(0);
            }
        }
    }
    Some(out)
}

/// A font ready for the app: per character the coverage and, with `full_colour` (a text
/// texture with bit 0 of field 5 draws in it), the colour.
pub fn font_of(omsi_dir: &Path, f: &OftFont, full_colour: bool) -> Font {
    let read = |line: &str| bitmap_path(omsi_dir, line).and_then(|p| read_bitmap(&p));
    let alpha = read(&f.alpha);
    let own = if full_colour && f.bitmap != f.alpha { read(&f.bitmap) } else { None };
    let colour = if f.bitmap == f.alpha { alpha.as_ref() } else { own.as_ref() };
    let glyphs = f
        .chars
        .iter()
        .map(|c| Glyph {
            ch: char::from_u32(c.ch as u32).unwrap_or('\u{fffd}').to_string(),
            width: c.width,
            alpha: char_bytes(alpha.as_ref(), c, f.height, 1).map(|b| base64(&b)),
            rgb: if full_colour { char_bytes(colour, c, f.height, 3).map(|b| base64(&b)) } else { None },
        })
        .collect();
    Font { name: f.name.clone(), height: f.height, sep: f.sep, glyphs }
}

/// All fonts of an installation in the order the game knows them, kept while no `.oft` changes.
/// The folder, the print of its `.oft`s and their fonts.
type Read = (PathBuf, String, std::sync::Arc<Vec<OftFont>>);

#[derive(Default)]
pub struct FontBook {
    read: Mutex<Option<Read>>,
}

impl FontBook {
    pub fn all(&self, omsi_dir: &Path) -> std::sync::Arc<Vec<OftFont>> {
        let dir = omsi_dir.join("Fonts");
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .map(|r| r.flatten().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.to_lowercase().ends_with(".oft")).collect())
            .unwrap_or_default();
        names.sort_by(|a, b| by_name(a, b));
        let print: String = names
            .iter()
            .map(|n| match std::fs::metadata(dir.join(n)) {
                Ok(m) => format!("{n}:{}:{:?}|", m.len(), m.modified().ok()),
                Err(_) => format!("{n}:?|"),
            })
            .collect();
        let mut guard = self.read.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((d, p, fonts)) = guard.as_ref() {
            if d == omsi_dir && *p == print {
                return fonts.clone();
            }
        }
        let mut fonts = Vec::new();
        for n in &names {
            if let Ok(bytes) = std::fs::read(dir.join(n)) {
                fonts.extend(parse_oft(&bytes).0);
            }
        }
        let fonts = std::sync::Arc::new(fonts);
        *guard = Some((omsi_dir.to_path_buf(), print, fonts.clone()));
        fonts
    }

    /// A font as the game finds it (exact name, first hit), ready for the app.
    pub fn font(&self, omsi_dir: &Path, name: &str, full_colour: bool, cache: &mut HashMap<(String, bool), Option<Font>>) -> Option<Font> {
        if let Some(f) = cache.get(&(name.to_string(), full_colour)) {
            return f.clone();
        }
        let all = self.all(omsi_dir);
        let f = all.iter().find(|f| f.name == name).map(|f| font_of(omsi_dir, f, full_colour));
        cache.insert((name.to_string(), full_colour), f.clone());
        f
    }
}

/// A font's `.oft` and its bitmap (24 bits), for tests and the synthetic bus: each character
/// `width` x `height` pixels side by side, `pixels(char, x, y)` its coverage.
pub fn write_font(dir: &Path, name: &str, file: &str, chars: &str, width: i64, height: i64, pixels: impl Fn(char, i64, i64) -> u8) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let list: Vec<char> = chars.chars().collect();
    let bmp = crate::testbus::bmp24((list.len() as i64 * width).max(1) as u32, height as u32, |x, y| {
        let v = pixels(list[(x as i64 / width) as usize], x as i64 % width, y as i64);
        [v, v, v]
    });
    let stem = file.trim_end_matches(".oft");
    std::fs::write(dir.join(format!("{stem}.bmp")), bmp)?;
    let mut oft = format!("\t\t[newfont]\r\n{{name}}\r\n\r\n[newfont]\r\n{name}\r\n{stem}.bmp\r\n{stem}.bmp\r\n{height}\r\n1\r\n\r\n");
    for (k, c) in chars.chars().enumerate() {
        let x = k as i64 * width;
        oft.push_str(&format!("[char]\r\n{c}\r\n{x}\r\n{}\r\n0\r\n\r\n", x + width));
    }
    // (byte by byte, as the game reads it)
    let bytes: Vec<u8> = oft.chars().map(|c| c as u32 as u8).collect();
    std::fs::write(dir.join(file), bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indented_tags_are_comments_and_the_space_is_a_character() {
        let oft = b"\t\t[newfont]\r\n{name}\r\n[newfont]\r\nMy Font\r\na.bmp\r\nb.bmp\r\n9\r\n1\r\n[char]\r\n \r\n0\r\n3\r\n0\r\n[char]\r\n\x80\r\n3\r\n-1\r\n0\r\n";
        let (fonts, err) = parse_oft(oft);
        assert_eq!(err, None);
        assert_eq!(fonts.len(), 1);
        let f = &fonts[0];
        assert_eq!((f.name.as_str(), f.bitmap.as_str(), f.height, f.sep), ("My Font", "Fonts/a.bmp", 9, 1));
        assert_eq!(f.chars[0], OftChar { ch: 0x20, x: 0, y: 0, width: 3 });
        assert_eq!((f.chars[1].ch, f.chars[1].width), (0x80, -4));
        let (_, err) = parse_oft(b"[newfont]\r\nX\r\na\r\na\r\n9 \r\n1\r\n");
        assert!(err.is_some(), "a space after a number stops the game");
    }

    #[test]
    fn coverage_is_byte_0_of_the_raw_rows_and_bitmaps_stay_inside() {
        let root = std::env::temp_dir().join(format!("hub-devices-fonts-{}", std::process::id()));
        write_font(&root.join("Fonts"), "Test", "test.oft", "AB", 2, 2, |c, x, y| if c == 'B' && x == 1 && y == 0 { 200 } else { 0 }).unwrap();
        let book = FontBook::default();
        let mut cache = HashMap::new();
        let f = book.font(&root, "Test", true, &mut cache).unwrap();
        assert_eq!(f.glyphs.len(), 2);
        assert_eq!(f.glyphs[1].alpha.as_deref(), Some(base64(&[0, 200, 0, 0]).as_str()));
        assert_eq!(f.glyphs[1].rgb.as_deref(), Some(base64(&[0, 0, 0, 200, 200, 200, 0, 0, 0, 0, 0, 0]).as_str()));
        assert!(book.font(&root, "test", false, &mut cache).is_none(), "the name is case-sensitive");
        assert_eq!(bitmap_path(&root, "Fonts/../../x.bmp"), None);
        assert_eq!(bitmap_path(&root, "Fonts/C:\\x.bmp"), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
