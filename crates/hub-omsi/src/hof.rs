//! Depot files (`.hof`): the destinations a bus's IBIS and displays know (`[addterminus]`),
//! the stops (`[addbusstop]`), the routes (`[infosystem_trip]`) with their stop lists
//! (`[infosystem_busstop_list]`), and the strings each one has (`[stringcount_terminus]`,
//! `[stringcount_busstop]`). Read and written back with their comments and codepage.
//!
//! **Status: the contract.** The types are fixed; the reader and the writer are `todo!()` until
//! the hub-omsi port.
#![allow(unused_variables)]

use crate::text::{CfgFile, Codepage};
use crate::ReadError;
use std::path::{Path, PathBuf};

/// A destination (`[addterminus]`, `[addterminus_allexit]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Terminus {
    /// Its IBIS code.
    pub code: i32,
    /// Its name, which is also the texture's id the displays look up.
    pub texture_id: String,
    /// The stop it ends at, where the file names one.
    pub terminus_stop: Option<String>,
    /// Everyone gets out there (`_allexit`).
    pub all_exit: bool,
    /// Its strings (`[stringcount_terminus]` of them).
    pub strings: Vec<String>,
}

/// A stop (`[addbusstop]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BusStop {
    pub ident: String,
    /// Its strings (`[stringcount_busstop]` of them).
    pub strings: Vec<String>,
}

/// A route of the IBIS (`[infosystem_trip]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfoTrip {
    /// Its IBIS route code (a number, as text).
    pub code: String,
    pub name: String,
    /// The terminus code it goes to (a number, as text).
    pub route: String,
    pub line: String,
    /// Any further values, kept as they are.
    pub extra: Vec<String>,
}

/// The comments of a file, kept where they were.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Comments {
    pub head: Vec<String>,
    pub terminus: Vec<String>,
    pub busstop: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hof {
    pub path: PathBuf,
    /// `[name]`.
    pub name: String,
    /// `[servicetrip]`: the destination of an empty run.
    pub service_trip: String,
    /// `[global_strings]`.
    pub global_strings: Vec<String>,
    pub string_count_terminus: usize,
    pub string_count_busstop: usize,
    pub termini: Vec<Terminus>,
    pub bus_stops: Vec<BusStop>,
    pub info_trips: Vec<InfoTrip>,
    /// The stops of each route, in step with `info_trips`.
    pub info_busstop_lists: Vec<Vec<String>>,
    pub codepage: Option<Codepage>,
}

impl Hof {
    pub fn parse(cfg: &CfgFile) -> Hof {
        todo!("hub-omsi port: .hof reader")
    }

    pub fn load(path: &Path) -> Result<Hof, ReadError> {
        let mut h = Hof::parse(&CfgFile::read(path)?);
        h.path = path.to_path_buf();
        Ok(h)
    }

    /// Only the depot's `[name]`, without reading the rest.
    pub fn read_name(path: &Path) -> Option<String> {
        CfgFile::read(path).ok()?.value("name").map(str::to_string)
    }

    /// The file's text again, with `comments` where they were.
    pub fn to_text(&self, comments: &Comments) -> String {
        todo!("hub-omsi port: .hof writer")
    }

    /// It has a route for `line`.
    pub fn has_line(&self, line: &str) -> bool {
        self.info_trips.iter().any(|t| t.line.trim().eq_ignore_ascii_case(line.trim()))
    }
}

/// The `.hof` files of a folder, sorted by name.
pub fn depot_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> =
        std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("hof"))).collect()).unwrap_or_default();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_depot_s_name_and_lines() {
        let d = std::env::temp_dir().join(format!("hub-omsi-hof-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("Berlin.HOF"), b"[name]\r\nBerlin Spandau\r\n").unwrap();
        std::fs::write(d.join("x.txt"), b"").unwrap();
        let files = depot_files(&d);
        assert_eq!(files.len(), 1);
        assert_eq!(Hof::read_name(&files[0]).as_deref(), Some("Berlin Spandau"));
        let h = Hof { info_trips: vec![InfoTrip { line: "10".into(), ..InfoTrip::default() }], ..Hof::default() };
        assert!(h.has_line(" 10") && !h.has_line("11"));
    }
}
