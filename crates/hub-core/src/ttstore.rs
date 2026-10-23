//! Where the launcher writes a map's timetable. The game reads one `TTData` folder per map -
//! the content folder's when it has one, else the map's own (`crate::paths::resolve_path`) - so a
//! map of the original installation gets its `TTData` copied into the content folder whole
//! before anything is written there, and the original stays as it is. A map that lies
//! unpacked in the content folder is written in place, every file it had kept as
//! `<file>.orig` the first time it is written over.
//!
//! The Timetable page saves lines here; the line editor writes its lines here too
//! (`lines::export_to_map`), and the reset puts the map's own timetable back.

use std::path::{Path, PathBuf};

/// Marks a `TTData` folder the launcher copied into the content folder: the reset deletes
/// such a copy, and only such a one.
pub const COPY_MARK: &str = ".openomsi-ttdata-copy";

/// The content folder's copy of a map's `TTData`.
pub fn copy_dir(content: &Path, map_folder: &str) -> PathBuf {
    crate::paths::resolve_path(content, &format!("maps/{map_folder}/TTData"))
}

/// The map lies unpacked in the content folder.
fn in_content(content: &Path, path: &Path) -> bool {
    path.starts_with(content)
}

/// The content folder's copy of `original_ttdata`, made whole when it is not there yet: beside
/// it first and then put in its place, so that a copy cut off half-way (a full disk, the
/// launcher closed) is never taken for the map's timetable.
fn make_copy(content: &Path, map_folder: &str, original_ttdata: &Path) -> Result<PathBuf, String> {
    let dir = copy_dir(content, map_folder);
    if !dir.is_dir() {
        let part = dir.with_file_name("TTData.openomsi-part");
        let _ = std::fs::remove_dir_all(&part);
        if let Err(e) = copy_into(&part, original_ttdata).and_then(|()| std::fs::rename(&part, &dir).map_err(|e| e.to_string())) {
            let _ = std::fs::remove_dir_all(&part);
            return Err(e);
        }
    }
    Ok(dir)
}

/// Every file of `original_ttdata` copied into the folder `part`, and the mark of a copy.
fn copy_into(part: &Path, original_ttdata: &Path) -> Result<(), String> {
    std::fs::create_dir_all(part).map_err(|e| e.to_string())?;
    let entries = std::fs::read_dir(original_ttdata).map_err(|_| format!("{} cannot be read", original_ttdata.display()))?;
    for e in entries.flatten() {
        if e.path().is_file() {
            let bytes = std::fs::read(e.path()).map_err(|e| e.to_string())?;
            std::fs::write(part.join(e.file_name()), bytes).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(part.join(COPY_MARK), b"TTData copied by the openOMSI launcher's timetable editor; its reset deletes this folder\n").map_err(|e| e.to_string())
}

/// `<file>.orig` beside a file of a map in the content folder, the first time it is written
/// over (nothing in a copy the launcher made: the reset deletes that whole).
pub fn keep_original(path: &Path) -> Result<(), String> {
    let orig = PathBuf::from(format!("{}.orig", path.display()));
    let copied = path.parent().is_some_and(|d| d.join(COPY_MARK).is_file());
    if !copied && path.is_file() && !orig.exists() {
        std::fs::copy(path, &orig).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Where a map's line is written (`line_path` where it was read, `file` its file name): in
/// place when the map lies unpacked in the content folder (backed up as `<file>.orig` the
/// first time), else in the content folder's copy of its `TTData`.
pub fn save_target(content: &Path, line_path: &Path, file: &str, map_folder: &str, original_ttdata: &Path) -> Result<PathBuf, String> {
    if in_content(content, line_path) {
        keep_original(line_path)?;
        return Ok(line_path.to_path_buf());
    }
    Ok(make_copy(content, map_folder, original_ttdata)?.join(file))
}

/// The `TTData` folder the launcher writes a map's files into: the map's own when it lies
/// unpacked in the content folder, else the content folder's copy (made whole first).
pub fn ttdata_dir(content: &Path, map_dir: &Path, map_folder: &str) -> Result<PathBuf, String> {
    if in_content(content, map_dir) {
        let own = crate::paths::resolve_path(map_dir, "TTData");
        std::fs::create_dir_all(&own).map_err(|e| e.to_string())?;
        return Ok(own);
    }
    let copy = copy_dir(content, map_folder);
    if copy.is_dir() {
        return Ok(copy);
    }
    make_copy(content, map_folder, &crate::paths::resolve_path(map_dir, "TTData"))
}

/// The map's own timetable back: the launcher's copy of its `TTData` deleted, or (a map
/// unpacked in the content folder) every file saved over put back from its `.orig`.
/// Returns what was done: the copy removed (None), or how many files were put back - the page
/// words it (its texts translated, `RESET_*`).
pub fn reset_timetable(content: &Path, map_dir: &Path, map_folder: &str) -> Result<Option<usize>, String> {
    let copy = copy_dir(content, map_folder);
    if copy.join(COPY_MARK).is_file() {
        std::fs::remove_dir_all(&copy).map_err(|e| e.to_string())?;
        return Ok(None);
    }
    let own = crate::paths::resolve_path(map_dir, "TTData");
    let mut restored = 0;
    if own.is_dir() {
        for e in std::fs::read_dir(&own).map_err(|e| e.to_string())?.flatten() {
            let p = e.path();
            if let Some(orig) = p.to_str().and_then(|s| s.strip_suffix(".orig")) {
                std::fs::rename(&p, orig).map_err(|e| e.to_string())?;
                restored += 1;
            }
        }
        // (the lines of the map's the line editor wrote over are the map's own again: its list
        // of them goes, so that it keeps each one anew when it writes it again)
        let _ = std::fs::remove_file(own.join(crate::lines::MAP_MANIFEST));
    }
    if restored > 0 {
        Ok(Some(restored))
    } else {
        Err(RESET_NONE.to_string())
    }
}

/// What resetting a map's timetable did, as the page says it (`%{map}` the map's folder, `%{n}`
/// the files put back), and why it did nothing.
pub const RESET_COPY: &str = "The timetable of %{map} is the map's own again (the edited copy was removed)";
pub const RESET_BACK: &str = "%{n} line(s) of %{map} put back as they were";
pub const RESET_NONE: &str = "The timetable of this map has not been changed here";

// --- what the line editor adds to a map's StnLinks.cfg and Busstops.cfg --------------------
//
// (the old `omsi-timetable/src/write.rs`, Omsi-Hub's own part of it: the block, and the text
// written in the code page of the file it replaces)

/// The comment lines around what the line editor adds to a map's `StnLinks.cfg` or
/// `Busstops.cfg`, so that it can be written again or taken out without touching a line of the
/// map's own (no reader takes them for a keyword). The old Omsi-Hub's words, so that a file it
/// wrote has its block found.
pub const BLOCK_BEGIN: &str = "--- openOMSI line editor: begin (written again on every save) ---";
pub const BLOCK_END: &str = "--- openOMSI line editor: end ---";

/// `text` written to `path` in the code page of the file it replaces (Windows-1252 for a new
/// one, as OMSI's own files are) - in UTF-8 when that code page lacks one of its characters
/// (a Polish or Cyrillic name in a Windows-1252 file would no longer match the name the depot
/// file and the other files give it).
pub fn write_text(path: &Path, text: &str) -> std::io::Result<()> {
    let page = std::fs::read(path).map(|b| hub_omsi::text::detect(&b)).unwrap_or(hub_omsi::Codepage::Windows1252);
    let (bytes, lossy) = hub_omsi::text::encode(text, page);
    if lossy {
        return std::fs::write(path, text.as_bytes());
    }
    std::fs::write(path, bytes)
}

/// `text` without the line editor's block (see [`BLOCK_BEGIN`]), and with `block` as the new
/// one at its end when it is not empty. Everything outside the block stays as it was.
pub fn replace_block(text: &str, block: &str) -> String {
    let mut out = String::with_capacity(text.len() + block.len());
    let mut inside = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t == BLOCK_BEGIN {
            inside = true;
            // (the empty line put before it: else every save adds one more)
            for nl in ["\r\n", "\n"] {
                if out.ends_with(&format!("{nl}{nl}")) {
                    out.truncate(out.len() - nl.len());
                    break;
                }
            }
            continue;
        }
        if t == BLOCK_END {
            inside = false;
            continue;
        }
        if !inside {
            out.push_str(line);
        }
    }
    if !block.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str("\r\n");
        }
        out.push_str(&format!("\r\n{BLOCK_BEGIN}\r\n\r\n{block}{BLOCK_END}\r\n"));
    }
    out
}

/// `path`'s block (see [`replace_block`]) written anew, in the file's own code page; a file
/// that is not there is made when there is something to put in it.
pub fn update_block(path: &Path, block: &str) -> std::io::Result<()> {
    let old = match std::fs::read(path) {
        Ok(b) => hub_omsi::text::decode(&b).0,
        Err(_) if block.trim().is_empty() => return Ok(()),
        Err(_) => String::new(),
    };
    let new = replace_block(&old, block);
    if new == old {
        return Ok(());
    }
    write_text(path, &new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_is_replaced_and_the_rest_kept() {
        let own = "[busstop]\r\nNordspitze\r\n3\r\n129\r\n0.0\r\n0\r\n0\r\n";
        let a = "[busstop]\r\nMarkt\r\n4\r\n7\r\n0.0\r\n0\r\n0\r\n";
        let b = "[busstop]\r\nKirche\r\n4\r\n8\r\n0.0\r\n0\r\n0\r\n";
        let once = replace_block(own, a);
        let twice = replace_block(&once, b);
        assert!(twice.starts_with(own) && twice.contains("Kirche") && !twice.contains("Markt"));
        // the same block again: the file as it was (no empty line more on every save)
        assert_eq!(replace_block(&twice, b), twice);
        // an empty block takes ours out and leaves the map's as it was
        assert_eq!(replace_block(&twice, ""), own);
    }

    #[test]
    fn what_the_code_page_lacks_is_written_in_utf8() {
        let dir = std::env::temp_dir().join(format!("omsi_tt_write_text_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // a German name: Windows-1252, as OMSI's own files
        let p = dir.join("a.ttp");
        write_text(&p, "Marktplatz Süd\r\n").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"Marktplatz S\xFCd\r\n");
        // a Polish and a Russian one: no HTML entity, the text as it is
        let p = dir.join("b.ttp");
        write_text(&p, "Dworzec Główny\r\nВокзал\r\n").unwrap();
        let b = std::fs::read(&p).unwrap();
        assert_eq!(hub_omsi::text::decode(&b).0, "Dworzec Główny\r\nВокзал\r\n");
        // and the block in a file that is not there yet
        let p = dir.join("Busstops.cfg");
        update_block(&p, "").unwrap();
        assert!(!p.exists());
        update_block(&p, "[busstop]\r\nMarkt\r\n").unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains(BLOCK_END));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_map_of_the_installation_is_copied_whole_and_reset() {
        let base = std::env::temp_dir().join(format!("omsi_ttstore_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (content, omsi) = (base.join("content"), base.join("omsi"));
        let map = omsi.join("maps").join("Dorf");
        std::fs::create_dir_all(map.join("TTData")).unwrap();
        std::fs::write(map.join("TTData").join("1.ttl"), "[priority]\r\n1\r\n").unwrap();
        std::fs::write(map.join("TTData").join("1_a.ttp"), "[trip]\r\n\r\nX\r\n1\r\n").unwrap();
        let dir = ttdata_dir(&content, &map, "Dorf").unwrap();
        assert_eq!(dir, copy_dir(&content, "Dorf"));
        assert!(dir.join("1.ttl").is_file() && dir.join("1_a.ttp").is_file() && dir.join(COPY_MARK).is_file());
        assert!(!dir.with_file_name("TTData.openomsi-part").exists(), "made beside it, then put in its place");
        // an original that cannot be read leaves no copy that would be taken for the map's
        let other = ttdata_dir(&content, &omsi.join("maps").join("Gone"), "Gone");
        assert!(other.is_err() && !copy_dir(&content, "Gone").exists() && !copy_dir(&content, "Gone").with_file_name("TTData.openomsi-part").exists());
        // a line goes into that copy
        assert_eq!(save_target(&content, &map.join("TTData").join("1.ttl"), "1.ttl", "Dorf", &map.join("TTData")).unwrap(), dir.join("1.ttl"));
        assert!(reset_timetable(&content, &map, "Dorf").is_ok());
        assert!(!dir.exists());
        // the original was never touched
        assert!(map.join("TTData").join("1.ttl").is_file());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_map_in_the_content_folder_is_written_in_place() {
        let base = std::env::temp_dir().join(format!("omsi_ttstore_in_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let content = base.join("content");
        let map = content.join("maps").join("Mod");
        std::fs::create_dir_all(map.join("TTData")).unwrap();
        let line = map.join("TTData").join("5.ttl");
        std::fs::write(&line, "own").unwrap();
        assert_eq!(ttdata_dir(&content, &map, "Mod").unwrap(), map.join("TTData"));
        assert_eq!(save_target(&content, &line, "5.ttl", "Mod", &map.join("TTData")).unwrap(), line);
        std::fs::write(&line, "changed").unwrap();
        assert!(reset_timetable(&content, &map, "Mod").is_ok());
        assert_eq!(std::fs::read_to_string(&line).unwrap(), "own");
        let _ = std::fs::remove_dir_all(&base);
    }
}
