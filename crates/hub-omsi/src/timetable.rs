//! A map's timetable (`<map>/TTData`): read, written, and made into the lines of a day.
//!
//! The files, in OMSI's text format ([`crate::text`]):
//! * `<line>.ttl` - a line ([`Line`]): `[userallowed]`, `[priority]`, and its tours
//!   (`[newtour]`: number, AI group, day mask; then its trips, `[addtrip]`: the trip, the
//!   profile, the departure in minutes of the day);
//! * `<trip>.ttp` - a trip ([`Trip`]): its track, its terminus and line, its stops (`[station]`
//!   or the older `[station_typ2]` lists) and its profiles (the minutes from stop to stop,
//!   `[profile]` with `[profile_man_dep]`/`[profile_man_arr]`/`[profile_stopping]`);
//! * `<track>.ttr` - the lanes a trip drives ([`Track`], `[track_entry]`);
//! * `StnLinks.cfg` - the links between stops and their lengths ([`StnLink`]);
//! * `Busstops.cfg` - the stops the map names ([`BusStopEntry`]).
//!
//! Stop times are interpolated along a trip's profile: each link's length times the profile's
//! factor, the profile's fixed departures and arrivals kept ([`stop_times`]).
//!
//! **Status: the contract.** The types are fixed; the readers and writers are `todo!()` until
//! the hub-omsi port (with tests on synthetic TTData).
#![allow(unused_variables)]

use crate::text::{CfgFile, Codepage};
use crate::vfs::Vfs;
use crate::ReadError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A tour's days: bit 0 Monday … bit 6 Sunday; the higher bits as the file has them (school
/// days, holidays: their meaning is the port's to pin down against real files).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DayMask(pub u32);

impl DayMask {
    pub const WEEKDAYS: DayMask = DayMask(0b001_1111);
    pub const DAILY: DayMask = DayMask(0b111_1111);

    /// It runs on this weekday (0 = Monday).
    pub fn on_weekday(self, day: u32) -> bool {
        day < 7 && self.0 & (1 << day) != 0
    }

    /// The days in words: `daily`, `Mon-Fri`, `Sat`, `Mon, Wed`, `Sat-Sun` …
    pub fn words(self) -> String {
        const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let days = self.0 & 0b111_1111;
        if days == 0b111_1111 {
            return "daily".into();
        }
        let mut parts: Vec<String> = Vec::new();
        let mut d = 0;
        while d < 7 {
            if days & (1 << d) == 0 {
                d += 1;
                continue;
            }
            let start = d;
            while d + 1 < 7 && days & (1 << (d + 1)) != 0 {
                d += 1;
            }
            parts.push(if d - start >= 1 { format!("{}-{}", NAMES[start], NAMES[d]) } else { NAMES[start].to_string() });
            d += 1;
        }
        parts.join(", ")
    }
}

/// A map's whole timetable, as the files say it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timetable {
    pub lines: Vec<Line>,
    pub trips: Vec<Trip>,
    pub tracks: Vec<Track>,
    pub stn_links: Vec<StnLink>,
    pub bus_stops: Vec<BusStopEntry>,
}

/// A line (`.ttl`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Line {
    pub path: PathBuf,
    /// The file's name without `.ttl`.
    pub name: String,
    pub user_allowed: bool,
    pub priority: i32,
    pub tours: Vec<Tour>,
    pub codepage: Option<Codepage>,
}

/// A tour of a line (`[newtour]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tour {
    pub number: String,
    pub ai_group: String,
    pub days: DayMask,
    pub trips: Vec<TourTrip>,
}

/// A trip of a tour (`[addtrip]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TourTrip {
    /// The trip file's name (without `.ttp`).
    pub trip: String,
    /// The profile of the trip it drives with (0 = the first).
    pub profile: i32,
    /// Its departure, minutes of the day.
    pub departure: f32,
}

/// A stop of a trip (from `[station]`, or the older lists: normalised to one form).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripStop {
    /// The stop's map object id.
    pub id: i64,
    pub name: String,
    /// Its place in the trip's track, where the file says it.
    pub track_index: Option<usize>,
}

/// A trip (`.ttp`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trip {
    pub path: PathBuf,
    /// The file's name without `.ttp`.
    pub name: String,
    /// The name it shows in OMSI's timetable dialog (empty: `name`).
    pub display_name: String,
    /// The track (`.ttr`) it drives.
    pub track: String,
    pub terminus: String,
    pub line: String,
    pub stops: Vec<TripStop>,
    pub profiles: Vec<TripProfile>,
}

/// A trip's profile: how long it takes from stop to stop.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripProfile {
    pub name: String,
    /// The trip's whole time, minutes: the links' lengths share it.
    pub minutes: f32,
    /// Fixed departures and arrivals: (stop's place in the trip, minutes from the start).
    pub fixed_departures: Vec<(i32, f32)>,
    pub fixed_arrivals: Vec<(i32, f32)>,
    /// Stops it serves otherwise than the trip says: (stop's place, how).
    pub stopping: Vec<(i32, i32)>,
}

/// A lane of a trip's track (`[track_entry]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackEntry {
    /// The spline's id, its path (lane), the tile's index in `global.cfg`, its length (m).
    pub spline: i64,
    pub path: i32,
    pub tile: usize,
    pub length: f64,
    /// Every value as the file has it (written back as they are).
    pub values: Vec<f64>,
}

/// A track (`.ttr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Track {
    pub path: PathBuf,
    /// The file's name without `.ttr`.
    pub name: String,
    pub entries: Vec<TrackEntry>,
}

/// A link of `StnLinks.cfg`: from one stop to the next, its length and lanes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StnLink {
    pub from_id: i64,
    pub to_id: i64,
    /// Its length, m.
    pub length: f64,
    /// The link's values after the ids (from lane, to lane, from s, to s, 0, last index).
    pub params: [f64; 6],
    /// Its lanes: (spline id, path, tile, length …) each.
    pub entries: Vec<[f64; 7]>,
}

/// A stop of `Busstops.cfg`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BusStopEntry {
    pub name: String,
    /// The tile's index in `global.cfg`.
    pub tile: i32,
    pub object_id: i64,
    pub params: [f64; 3],
}

impl Timetable {
    /// Every file of the TTData folder `dir` (a missing or unreadable file is left out).
    pub fn load(dir: &Path) -> Timetable {
        todo!("hub-omsi port: read the TTData folder")
    }

    /// The same with the Chrono folders active on the day: their files take the place of the
    /// TTData ones of the same name, and the lines they switch off are left out.
    pub fn load_with_chrono(dir: &Path, chrono: &[crate::map::ChronoDir], off: &[String]) -> Timetable {
        todo!("hub-omsi port: TTData with Chrono")
    }

    /// A trip by its name, in any case.
    pub fn trip(&self, name: &str) -> Option<&Trip> {
        self.trips.iter().find(|t| t.name.eq_ignore_ascii_case(name.trim()))
    }

    /// A track by its name, in any case.
    pub fn track(&self, name: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.name.eq_ignore_ascii_case(name.trim()))
    }

    /// The link from stop `from` to stop `to`.
    pub fn link(&self, from: i64, to: i64) -> Option<&StnLink> {
        self.stn_links.iter().find(|l| l.from_id == from && l.to_id == to)
    }
}

impl Line {
    pub fn parse(cfg: &CfgFile) -> Line {
        todo!("hub-omsi port: .ttl")
    }

    pub fn load(path: &Path) -> Result<Line, ReadError> {
        Ok(Line::parse(&CfgFile::read(path)?))
    }

    /// The file's text again (`\r\n` line ends).
    pub fn to_text(&self) -> String {
        todo!("hub-omsi port: .ttl writer")
    }
}

impl Trip {
    pub fn parse(cfg: &CfgFile) -> Trip {
        todo!("hub-omsi port: .ttp")
    }

    pub fn to_text(&self) -> String {
        todo!("hub-omsi port: .ttp writer")
    }
}

impl Track {
    pub fn parse(cfg: &CfgFile) -> Track {
        todo!("hub-omsi port: .ttr")
    }

    pub fn to_text(&self) -> String {
        todo!("hub-omsi port: .ttr writer")
    }
}

impl StnLink {
    /// The link's block for `StnLinks.cfg`, with the two stops' names as its comment.
    pub fn to_text(&self, from_name: &str, to_name: &str) -> String {
        todo!("hub-omsi port: StnLinks.cfg writer")
    }
}

impl BusStopEntry {
    pub fn to_text(&self) -> String {
        todo!("hub-omsi port: Busstops.cfg writer")
    }
}

pub fn parse_stnlinks(cfg: &CfgFile) -> Vec<StnLink> {
    todo!("hub-omsi port: StnLinks.cfg")
}

pub fn parse_busstops(cfg: &CfgFile) -> Vec<BusStopEntry> {
    todo!("hub-omsi port: Busstops.cfg")
}

/// `text` written to `path` in `cp`, its line ends `\r\n`, through a temporary file.
pub fn write_text(path: &Path, text: &str, cp: Codepage) -> std::io::Result<()> {
    let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
    let tmp = path.with_extension("hubtmp");
    std::fs::write(&tmp, crate::text::encode(&text, cp).0)?;
    std::fs::rename(&tmp, path)
}

/// `text` with its block that starts like `block` (the same keyword and first value) replaced
/// by `block`, or with `block` added at its end.
pub fn replace_block(text: &str, block: &str) -> String {
    todo!("hub-omsi port: block replacement for StnLinks.cfg / Busstops.cfg")
}

/// When `trip`, driven with its profile `profile` from `start` (seconds of the day), is due at
/// each of its stops: (arrival, departure), seconds of the day.
pub fn stop_times(tt: &Timetable, trip: &Trip, profile: usize, start: f64) -> Vec<(f64, f64)> {
    todo!("hub-omsi port: stop times along a profile")
}

// --- the lines of a day --------------------------------------------------------------------

/// A stop of a trip on the day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceStop {
    pub name: String,
    pub id: i64,
    pub arr: f64,
    pub dep: f64,
}

/// A trip of a tour on the day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceTrip {
    pub name: String,
    /// 1 = the tour's first.
    pub index: usize,
    pub line: String,
    pub from: String,
    pub terminus: String,
    pub departure: f64,
    pub arrival: f64,
    pub stops: Vec<ServiceStop>,
    pub km: f64,
}

/// A tour of a line, and whether it runs on the day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceTour {
    pub number: String,
    pub ai_group: String,
    pub first: f64,
    pub last: f64,
    pub days: DayMask,
    pub runs: bool,
    /// The first day from the one asked for on which it runs.
    pub next_run: Option<NaiveDate>,
    pub trips: Vec<ServiceTrip>,
}

/// A line on the day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceLine {
    pub name: String,
    pub user_allowed: bool,
    /// The first and last stop of its longest trip.
    pub termini: Vec<String>,
    pub tours: Vec<ServiceTour>,
}

/// The lines of `map` (its `global.cfg`, relative to the OMSI 2 folder) on `date`: the
/// TTData with the day's Chrono, each tour's days checked against the map's calendar
/// (`Holidays.txt`), the trips' stop times worked out.
pub fn lines_for(vfs: &Vfs, map: &str, date: NaiveDate) -> Result<Vec<ServiceLine>, ReadError> {
    todo!("hub-omsi port: the lines of a day")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_masks_read_as_words() {
        assert_eq!(DayMask::DAILY.words(), "daily");
        assert_eq!(DayMask::WEEKDAYS.words(), "Mon-Fri");
        assert_eq!(DayMask(0b110_0000).words(), "Sat-Sun");
        assert_eq!(DayMask(0b000_0101).words(), "Mon, Wed");
        assert!(DayMask::WEEKDAYS.on_weekday(4) && !DayMask::WEEKDAYS.on_weekday(5));
    }

    #[test]
    fn text_is_written_with_crlf_in_its_codepage() {
        let p = std::env::temp_dir().join(format!("hub-omsi-write-{}.ttl", std::process::id()));
        write_text(&p, "[name]\nGrünau\n", Codepage::Windows1252).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"[name]\r\nGr\xFCnau\r\n");
        let _ = std::fs::remove_file(p);
    }
}
