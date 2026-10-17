//! Display fonts (`.oft`): the names of the fonts a file holds (`[newfont]`: the name, the
//! bitmap, its alpha bitmap, the height, the spacing; then its `[char]`s), for the bus's display
//! font choice. Their bitmaps are not read.

use crate::text::CfgFile;
use crate::ReadError;
use std::path::Path;

/// The font names in a `.oft` file, in its order.
pub fn font_names(cfg: &CfgFile) -> Vec<String> {
    let mut out = Vec::new();
    let mut r = cfg.reader();
    while let Some(k) = r.next_keyword() {
        if k.eq_ignore_ascii_case("newfont") {
            let name = r.value().trim();
            if !name.is_empty() {
                out.push(name.to_string());
            }
        }
    }
    out
}

pub fn load_names(path: &Path) -> Result<Vec<String>, ReadError> {
    Ok(font_names(&CfgFile::read(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_font_file_names_its_fonts() {
        let text =
            "\t[newfont]\r\n\tAn example\r\n[newfont]\r\nMatrix 8\r\nmatrix8.bmp\r\nmatrix8_alpha.bmp\r\n8\r\n1\r\n\r\n[char]\r\nA\r\n0\r\n4\r\n0\r\n\r\n[newfont]\r\nMatrix 16 bold \r\nm16.bmp\r\n";
        assert_eq!(font_names(&CfgFile::from_str("x.oft", text)), ["Matrix 8", "Matrix 16 bold"]);
    }
}
