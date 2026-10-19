//! The content the desktop sends beside a cloud save, for the website's bus company (docs/WEB.md):
//! what the company's pages read of the computer - the map's timetable, the buses' files, the data
//! folder's own files and the bus photos - in parts (`map`, `buses`, `data`), each one archive in
//! the cloud save's own layout under a magic of its own. The desktop packs them
//! (`hub-api` `cloud_content`); the website lays them out in the browser's folders (`hub-web`):
//! a name `omsi/<path>` under the OMSI 2 folder, `data/<path>` under the data folder.

use anyhow::{bail, Context, Result};

pub const MAGIC: &[u8] = b"OMSIHUB-CONTENT 1\n";
/// The most files one part may hold.
pub const MAX_FILES: usize = 200_000;

/// A name the website may lay out: under `omsi/` or `data/`, relative, no `..`, no backslash.
pub fn safe_name(name: &str) -> bool {
    (name.starts_with("omsi/") || name.starts_with("data/")) && !name.contains('\\') && !name.contains('\0') && name.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

/// Files packed (sorted by name, so the same files pack the same).
pub fn pack(files: &mut [(String, Vec<u8>)]) -> Vec<u8> {
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = MAGIC.to_vec();
    for (name, bytes) in files.iter() {
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name.as_bytes());
        out.extend((bytes.len() as u64).to_le_bytes());
        out.extend(bytes);
    }
    out
}

/// A part read back: its files, each name checked.
pub fn unpack(archive: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    let mut rest = archive.strip_prefix(MAGIC).context("not Omsi-Hub's content")?;
    let mut files = Vec::new();
    let take = |rest: &mut &[u8], n: usize| -> Result<Vec<u8>> {
        if rest.len() < n {
            bail!("the content is cut short");
        }
        let (a, b) = rest.split_at(n);
        *rest = b;
        Ok(a.to_vec())
    };
    while !rest.is_empty() {
        if files.len() >= MAX_FILES {
            bail!("too many files in the content");
        }
        let n = u16::from_le_bytes(take(&mut rest, 2)?.try_into().unwrap_or_default()) as usize;
        let name = String::from_utf8(take(&mut rest, n)?).context("a file's name is not text")?;
        let len = u64::from_le_bytes(take(&mut rest, 8)?.try_into().unwrap_or_default());
        let bytes = take(&mut rest, usize::try_from(len).unwrap_or(usize::MAX))?;
        if !safe_name(&name) {
            bail!("the content holds a file it may not: {name:?}");
        }
        files.push((name, bytes));
    }
    Ok(files)
}

/// FNV-1a.
fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Where a bus's photo (`bus` its file as the company names it, `paint` its livery) lies in the
/// data folder on the website: `web/photos/<id>.png`.
pub fn photo_name(bus: &str, paint: &str) -> String {
    format!("web/photos/{:016x}.png", fnv(&format!("{}|{}", bus.replace('\\', "/").trim().to_lowercase(), paint.trim().to_lowercase())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_packs_the_same_and_comes_back() {
        let mut a = vec![("omsi/maps/X/global.cfg".to_string(), b"[name]".to_vec()), ("data/lines/X.json".to_string(), b"{}".to_vec())];
        let mut b = vec![a[1].clone(), a[0].clone()];
        let (pa, pb) = (pack(&mut a), pack(&mut b));
        assert_eq!(pa, pb);
        let back = unpack(&pa).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].0, "data/lines/X.json");
    }

    #[test]
    fn only_names_under_the_two_folders_come_in() {
        assert!(safe_name("omsi/Vehicles/MAN/a.bus"));
        assert!(!safe_name("omsi/../etc/passwd"));
        assert!(!safe_name("/etc/passwd"));
        assert!(!safe_name("other/x"));
        assert!(!safe_name("data//x"));
        let mut bad = vec![("data/../x".to_string(), vec![1])];
        assert!(unpack(&pack(&mut bad)).is_err());
        assert_eq!(photo_name("Vehicles\\MAN\\A.bus", ""), photo_name("vehicles/man/a.bus", " "));
    }
}
