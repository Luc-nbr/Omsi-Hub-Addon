//! The game's text files as lines: Windows-1252 (not Latin-1: 0x80-0x9F differ, and add-ons use
//! them for quotes and the euro), UTF-16LE after `FF FE`, UTF-8 after its BOM. CR, LF and CRLF
//! all break a line, as `TStringList` breaks them.

/// The lines of a file's bytes.
pub fn lines_of(bytes: &[u8]) -> Vec<String> {
    let text = if bytes.starts_with(&[0xff, 0xfe]) {
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else if let Some(rest) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        String::from_utf8_lossy(rest).into_owned()
    } else {
        encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes).0.into_owned()
    };
    let mut out = Vec::new();
    let mut line = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push(std::mem::take(&mut line));
            }
            '\n' => out.push(std::mem::take(&mut line)),
            _ => line.push(c),
        }
    }
    out.push(line);
    out
}

/// The lines of a file, or none.
pub fn read_lines(path: &std::path::Path) -> Option<Vec<String>> {
    std::fs::read(path).ok().map(|b| lines_of(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_break_breaks_and_the_codepage_is_windows_1252() {
        assert_eq!(lines_of(b"a\r\nb\rc\nd"), ["a", "b", "c", "d"]);
        assert_eq!(lines_of(b"\x80 5")[0], "€ 5");
        assert_eq!(lines_of(&[0xff, 0xfe, b'h', 0, b'i', 0])[0], "hi");
        assert_eq!(lines_of(b"x\r\n"), ["x", ""]);
    }
}
