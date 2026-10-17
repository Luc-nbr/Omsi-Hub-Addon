//! The OMSI text format and its codepages.
//!
//! OMSI's configuration files are lines: a keyword in brackets (`[name]`) and, on the lines
//! after it, its values, as many as the keyword has; anything else between keywords is a
//! comment. The files are in the codepage of the author's Windows (Windows-1252 for German and
//! English, -1250 for Czech and Polish, -1251 for Russian …), some in UTF-8 with or without a
//! BOM. [`CfgFile`] keeps the codepage it was read in, so a file written back stays readable
//! to OMSI and to its author's tools.
//!
//! Line ends are read as `\r\n`, `\n` or `\r`, and written as `\r\n` (as OMSI writes them).
//!
//! A keyword counts only at the very start of its line (`[name]`, blanks after it allowed), as
//! OMSI reads it: an indented `\t[infosystem_trip]` is a comment - files use that to show an
//! example, or to switch a keyword off. Maps' `global.cfg` and some weather files are UTF-16
//! with a BOM; they are read, and written back, as such.

use std::path::{Path, PathBuf};

/// A codepage of OMSI's files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Codepage {
    Utf8,
    /// Central European (Czech, Polish, Slovak, Hungarian …).
    Windows1250,
    /// Cyrillic.
    Windows1251,
    /// Western European: OMSI's own files, and most add-ons.
    Windows1252,
    /// Greek.
    Windows1253,
    /// Turkish.
    Windows1254,
    /// Baltic.
    Windows1257,
    /// UTF-16, little-endian (maps' `global.cfg`, some weather files), with its BOM.
    Utf16Le,
    /// UTF-16, big-endian, with its BOM.
    Utf16Be,
}

impl Codepage {
    pub fn encoding(self) -> &'static encoding_rs::Encoding {
        match self {
            Codepage::Utf8 => encoding_rs::UTF_8,
            Codepage::Windows1250 => encoding_rs::WINDOWS_1250,
            Codepage::Windows1251 => encoding_rs::WINDOWS_1251,
            Codepage::Windows1252 => encoding_rs::WINDOWS_1252,
            Codepage::Windows1253 => encoding_rs::WINDOWS_1253,
            Codepage::Windows1254 => encoding_rs::WINDOWS_1254,
            Codepage::Windows1257 => encoding_rs::WINDOWS_1257,
            Codepage::Utf16Le => encoding_rs::UTF_16LE,
            Codepage::Utf16Be => encoding_rs::UTF_16BE,
        }
    }

    /// The BOM a file in this codepage starts with when it has one.
    pub fn bom(self) -> &'static [u8] {
        match self {
            Codepage::Utf8 => UTF8_BOM,
            Codepage::Utf16Le => b"\xFF\xFE",
            Codepage::Utf16Be => b"\xFE\xFF",
            _ => b"",
        }
    }

    /// UTF-16: written with its BOM always.
    pub fn is_utf16(self) -> bool {
        matches!(self, Codepage::Utf16Le | Codepage::Utf16Be)
    }

    /// The codepage a language's Windows uses (`de`, `cs`, `ru` …), for a file that does not
    /// tell by itself.
    pub fn of_language(lang: &str) -> Codepage {
        match lang.get(..2).unwrap_or("").to_ascii_lowercase().as_str() {
            "cs" | "pl" | "sk" | "hu" | "sl" | "hr" | "ro" => Codepage::Windows1250,
            "ru" | "uk" | "be" | "bg" | "sr" | "kk" => Codepage::Windows1251,
            "el" => Codepage::Windows1253,
            "tr" => Codepage::Windows1254,
            "lt" | "lv" | "et" => Codepage::Windows1257,
            _ => Codepage::Windows1252,
        }
    }
}

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// The codepage `bytes` are most likely in. UTF-8 when they are valid UTF-8 (with or without a
/// BOM); else a guess from the letters above 127: mostly 0xC0..=0xFF in Cyrillic text, a few
/// letters only Central European codepages place where 1252 has rare signs; else
/// Windows-1252.
pub fn detect(bytes: &[u8]) -> Codepage {
    if bytes.starts_with(b"\xFF\xFE") {
        return Codepage::Utf16Le;
    }
    if bytes.starts_with(b"\xFE\xFF") {
        return Codepage::Utf16Be;
    }
    if bytes.starts_with(UTF8_BOM) {
        return Codepage::Utf8;
    }
    if !bytes.is_ascii() && std::str::from_utf8(bytes).is_ok() {
        return Codepage::Utf8;
    }
    let high: Vec<u8> = bytes.iter().copied().filter(|b| *b >= 0x80).collect();
    if high.is_empty() {
        return Codepage::Windows1252;
    }
    let cyrillic = high.iter().filter(|b| **b >= 0xC0).count();
    if high.len() >= 4 && cyrillic * 10 >= high.len() * 9 && cyrillic * 2 > high.len() {
        // (Cyrillic letters are all at 0xC0..=0xFF; German text has some, among other bytes)
        let umlauts = high.iter().filter(|b| matches!(b, 0xE4 | 0xF6 | 0xFC | 0xDF | 0xC4 | 0xD6 | 0xDC)).count();
        if umlauts * 3 < high.len() {
            return Codepage::Windows1251;
        }
    }
    // ą ł ś ź ż ř ě č ő ű ľ ť ň in 1250 are signs or rare letters in 1252
    let central = high.iter().filter(|b| matches!(b, 0xB9 | 0xB3 | 0x9C | 0x9F | 0xBF | 0xF8 | 0xEC | 0xE8 | 0xF5 | 0xFB | 0xBE | 0x9D | 0xF2 | 0x8A | 0x9A | 0x8E | 0x9E)).count();
    if central * 4 >= high.len() && central >= 2 {
        return Codepage::Windows1250;
    }
    Codepage::Windows1252
}

/// `bytes` as text in the codepage they are in, without a BOM.
pub fn decode(bytes: &[u8]) -> (String, Codepage) {
    let cp = detect(bytes);
    let body = bytes.strip_prefix(cp.bom()).unwrap_or(bytes);
    (cp.encoding().decode_without_bom_handling(body).0.into_owned(), cp)
}

/// Any text file: UTF-16 (with its BOM) as well, else as [`decode`].
pub fn decode_text(bytes: &[u8]) -> String {
    decode(bytes).0
}

/// `text` in `cp` (without a BOM); `true` with it when a character had no place in the
/// codepage (written as `?` then).
pub fn encode(text: &str, cp: Codepage) -> (Vec<u8>, bool) {
    match cp {
        Codepage::Utf8 => return (text.as_bytes().to_vec(), false),
        Codepage::Utf16Le => return (text.encode_utf16().flat_map(u16::to_le_bytes).collect(), false),
        Codepage::Utf16Be => return (text.encode_utf16().flat_map(u16::to_be_bytes).collect(), false),
        _ => {}
    }
    let (bytes, _, lossy) = cp.encoding().encode(text);
    if !lossy {
        return (bytes.into_owned(), false);
    }
    // (encoding_rs writes an HTML entity for a character it has no byte for: a `?` instead)
    let mut out = Vec::with_capacity(text.len());
    let mut buf = [0u8; 4];
    for c in text.chars() {
        let (b, _, bad) = cp.encoding().encode(c.encode_utf8(&mut buf));
        if bad {
            out.push(b'?');
        } else {
            out.extend_from_slice(&b);
        }
    }
    (out, true)
}

/// A whole number as OMSI writes it, leniently: spaces around it, a fraction cut off, junk
/// after it ignored; 0 when there is none.
pub fn parse_i32(s: &str) -> i32 {
    let s = s.trim();
    let end = s.char_indices().find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+')))).map_or(s.len(), |(i, _)| i);
    s[..end].parse().unwrap_or(0)
}

/// A decimal number, `.` or `,` as its point; 0 when there is none.
pub fn parse_f64(s: &str) -> f64 {
    let s = s.trim().replace(',', ".");
    let end = s
        .char_indices()
        .find(|(i, c)| !(c.is_ascii_digit() || *c == '.' || *c == 'e' || *c == 'E' || ((*i == 0 || s[..*i].ends_with(['e', 'E'])) && (*c == '-' || *c == '+'))))
        .map_or(s.len(), |(i, _)| i);
    s[..end].parse().unwrap_or(0.0)
}

pub fn parse_f32(s: &str) -> f32 {
    parse_f64(s) as f32
}

/// The keyword of a line (`[name]` → `name`), if it is one: at the start of the line (an
/// indented one is a comment, as OMSI reads it), blanks after it allowed.
pub fn keyword_of(line: &str) -> Option<&str> {
    let t = line.trim_end();
    t.strip_prefix('[').and_then(|r| r.strip_suffix(']')).filter(|k| !k.is_empty() && !k.contains(['[', ']']))
}

/// A configuration file read: its lines (without line ends) and the codepage it was in.
#[derive(Debug, Clone, PartialEq)]
pub struct CfgFile {
    /// Where it came from (for errors; empty for a text made in memory).
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub codepage: Codepage,
    /// It began with a BOM (written back with one; UTF-16 always has one).
    pub bom: bool,
}

fn split_lines(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

impl CfgFile {
    pub fn from_bytes(path: impl AsRef<Path>, bytes: &[u8]) -> CfgFile {
        let (text, codepage) = decode(bytes);
        let bom = !codepage.bom().is_empty() && bytes.starts_with(codepage.bom());
        CfgFile { path: path.as_ref().to_path_buf(), lines: split_lines(&text), codepage, bom }
    }

    /// A text already decoded (written back as Windows-1252, OMSI's own).
    pub fn from_str(path: impl AsRef<Path>, text: &str) -> CfgFile {
        CfgFile { path: path.as_ref().to_path_buf(), lines: split_lines(text), codepage: Codepage::Windows1252, bom: false }
    }

    pub fn read(path: impl AsRef<Path>) -> Result<CfgFile, crate::ReadError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| crate::ReadError::io(path, e))?;
        Ok(CfgFile::from_bytes(path, &bytes))
    }

    /// The text with `\r\n` line ends.
    pub fn to_text(&self) -> String {
        let mut s = self.lines.join("\r\n");
        s.push_str("\r\n");
        s
    }

    /// The file's bytes again, in its codepage.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = if self.bom || self.codepage.is_utf16() { self.codepage.bom().to_vec() } else { Vec::new() };
        out.extend(encode(&self.to_text(), self.codepage).0);
        out
    }

    /// A reader from the first line.
    pub fn reader(&self) -> Reader<'_> {
        Reader { lines: &self.lines, pos: 0 }
    }

    /// The values of every `[keyword]` (each block: the lines up to the next keyword).
    pub fn blocks(&self, keyword: &str) -> Vec<&[String]> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, l)| keyword_of(l) == Some(keyword))
            .map(|(i, _)| {
                let end = self.lines[i + 1..].iter().position(|l| keyword_of(l).is_some()).map_or(self.lines.len(), |k| i + 1 + k);
                &self.lines[i + 1..end]
            })
            .collect()
    }

    /// The first value of the first `[keyword]`.
    pub fn value(&self, keyword: &str) -> Option<&str> {
        self.blocks(keyword).first().and_then(|b| b.first()).map(|s| s.trim())
    }
}

/// Reads a [`CfgFile`] keyword by keyword: [`Reader::next_keyword`], then the keyword's values
/// with [`Reader::str`], [`Reader::i32`], [`Reader::f64`] … as many as it has.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// The next keyword (without its brackets), the lines before it skipped.
    pub fn next_keyword(&mut self) -> Option<&'a str> {
        while self.pos < self.lines.len() {
            let l = &self.lines[self.pos];
            self.pos += 1;
            if let Some(k) = keyword_of(l) {
                return Some(k);
            }
        }
        None
    }

    /// The next line as it is ("" past the end).
    pub fn str(&mut self) -> &'a str {
        let l = self.lines.get(self.pos).map_or("", String::as_str);
        self.pos = (self.pos + 1).min(self.lines.len());
        l
    }

    pub fn i32(&mut self) -> i32 {
        parse_i32(self.str())
    }

    pub fn f32(&mut self) -> f32 {
        parse_f32(self.str())
    }

    pub fn f64(&mut self) -> f64 {
        parse_f64(self.str())
    }

    /// The next line, unless it is a keyword (then "" and the reader stays: a keyword with
    /// fewer values than it should have does not swallow the next one).
    pub fn value(&mut self) -> &'a str {
        match self.lines.get(self.pos) {
            Some(l) if keyword_of(l).is_none() => {
                self.pos += 1;
                l
            }
            _ => "",
        }
    }

    /// The next `n` lines as [`Reader::value`] reads them, stopping at a keyword (fewer then).
    pub fn values(&mut self, n: usize) -> Vec<&'a str> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n && self.lines.get(self.pos).is_some_and(|l| keyword_of(l).is_none()) {
            out.push(self.lines[self.pos].as_str());
            self.pos += 1;
        }
        out
    }

    /// The lines up to the first blank line or keyword (what a keyword with any number of
    /// values has).
    pub fn run(&mut self) -> Vec<&'a str> {
        let mut out = Vec::new();
        while let Some(l) = self.lines.get(self.pos).filter(|l| !l.trim().is_empty() && keyword_of(l).is_none()) {
            out.push(l.as_str());
            self.pos += 1;
        }
        out
    }

    /// The lines up to the keyword `end` (`[end]`), which is passed over.
    pub fn until(&mut self, end: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        while self.pos < self.lines.len() {
            let l = &self.lines[self.pos];
            self.pos += 1;
            if keyword_of(l) == Some(end) {
                break;
            }
            out.push(l.as_str());
        }
        out
    }

    /// The line the reader is at (0 = the first): for errors.
    pub fn line(&self) -> usize {
        self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codepages_are_told_apart() {
        assert_eq!(detect(b"plain"), Codepage::Windows1252);
        assert_eq!(detect("Straße".as_bytes()), Codepage::Utf8);
        assert_eq!(detect(b"\xEF\xBB\xBFx"), Codepage::Utf8);
        // "Straße, Grünau" in 1252
        assert_eq!(detect(b"Stra\xDFe, Gr\xFCnau"), Codepage::Windows1252);
        // "Москва Центр" in 1251
        assert_eq!(detect(b"\xCC\xEE\xF1\xEA\xE2\xE0 \xD6\xE5\xED\xF2\xF0"), Codepage::Windows1251);
        // "Łódź, Wrocław, Kraków" in 1250
        assert_eq!(detect(b"\xA3\xF3d\x9F, Wroc\xB3aw, Krak\xF3w"), Codepage::Windows1250);
        assert_eq!(decode(b"Gr\xFCnau").0, "Grünau");
        assert_eq!(decode_text(b"\xFF\xFEa\0b\0"), "ab");
        assert_eq!(Codepage::of_language("pl-PL"), Codepage::Windows1250);
    }

    #[test]
    fn a_file_is_written_back_in_its_codepage() {
        let f = CfgFile::from_bytes("x.cfg", b"[name]\r\nGr\xFCnau\nend");
        assert_eq!(f.codepage, Codepage::Windows1252);
        assert_eq!(f.lines, ["[name]", "Grünau", "end"]);
        assert_eq!(f.to_bytes(), b"[name]\r\nGr\xFCnau\r\nend\r\n");
        assert_eq!(encode("a€ł", Codepage::Windows1252), (b"a\x80?".to_vec(), true));
        let u = CfgFile::from_bytes("u.cfg", "\u{feff}Grünau\n".as_bytes());
        assert!(u.bom && u.lines == ["Grünau"]);
        assert_eq!(u.to_bytes(), "\u{feff}Grünau\r\n".as_bytes());
    }

    #[test]
    fn keywords_and_their_values_are_read() {
        let f = CfgFile::from_str("", "comment\n[name]\nZoo\n[count]\n 12 \n[pos]\n1,5\n[list]\na\nb\n[end]\n[name]\nHbf");
        let mut r = f.reader();
        assert_eq!(r.next_keyword(), Some("name"));
        assert_eq!(r.str(), "Zoo");
        assert_eq!(r.next_keyword(), Some("count"));
        assert_eq!(r.i32(), 12);
        assert_eq!(r.next_keyword(), Some("pos"));
        assert_eq!(r.f64(), 1.5);
        assert_eq!(r.next_keyword(), Some("list"));
        assert_eq!(r.until("end"), ["a", "b"]);
        assert_eq!(r.next_keyword(), Some("name"));
        assert_eq!((r.str(), r.str()), ("Hbf", ""));
        assert_eq!(r.next_keyword(), None);
        assert_eq!(f.value("name"), Some("Zoo"));
        assert_eq!(f.blocks("name").len(), 2);
        assert_eq!((parse_i32("-4.7x"), parse_f64("2,5e1"), parse_f64("x")), (-4, 25.0, 0.0));
    }

    #[test]
    fn utf16_files_are_read_and_written_back() {
        let mut bytes = b"\xFF\xFE".to_vec();
        bytes.extend("[name]\r\nGrünau\r\n".encode_utf16().flat_map(u16::to_le_bytes));
        let f = CfgFile::from_bytes("global.cfg", &bytes);
        assert_eq!((f.codepage, f.bom), (Codepage::Utf16Le, true));
        assert_eq!(f.value("name"), Some("Grünau"));
        assert_eq!(f.to_bytes(), bytes);
        assert_eq!(decode_text(&bytes), "[name]\r\nGrünau\r\n");
        let be: Vec<u8> = b"\xFE\xFF".iter().copied().chain("ab".encode_utf16().flat_map(u16::to_be_bytes)).collect();
        assert_eq!(decode(&be), ("ab".to_string(), Codepage::Utf16Be));
    }

    #[test]
    fn an_indented_keyword_is_a_comment_and_values_stop_at_keywords() {
        assert_eq!(keyword_of("[name]\t\t"), Some("name"));
        assert_eq!(keyword_of("\t[infosystem_trip]"), None);
        let f = CfgFile::from_str("", "[station]\n1\n[station]\n2\nx\n\n[track]\na\nb\n\nc");
        let mut r = f.reader();
        r.next_keyword();
        assert_eq!(r.values(3), ["1"]);
        assert_eq!(r.next_keyword(), Some("station"));
        assert_eq!((r.value(), r.run()), ("2", vec!["x"]));
        assert_eq!(r.next_keyword(), Some("track"));
        assert_eq!(r.run(), ["a", "b"]);
        assert_eq!(r.value(), "");
    }
}
