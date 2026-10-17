//! A map's own files: `global.cfg` ([`GlobalCfg`]), `ailists.cfg` ([`AiLists`]),
//! `Holidays.txt` ([`Calendar`]) and its `Chrono` folders ([`ChronoDir`]).
//!
//! Dates are [`chrono::NaiveDate`]; [`date_code`] gives the `YYYYMMDD` number OMSI's files
//! compare.
//!
//! **Status: the contract.** The types are fixed; the readers are `todo!()` until the hub-omsi
//! port.
#![allow(unused_variables)]

use crate::text::CfgFile;
use crate::ReadError;
use chrono::{Datelike, NaiveDate};
use std::path::{Path, PathBuf};

/// `YYYY-MM-DD` as the number `YYYYMMDD`.
pub fn date_code(date: &str) -> Option<i32> {
    let d = NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok()?;
    Some(code_of(d))
}

/// A date as `YYYYMMDD`.
pub fn code_of(d: NaiveDate) -> i32 {
    d.year() * 10_000 + d.month() as i32 * 100 + d.day() as i32
}

/// An entry point of the map (`[entrypoints]`): where a bus can be put.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryPoint {
    pub index: i32,
    pub name: String,
}

/// A map's `global.cfg`: what Omsi-Hub needs of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlobalCfg {
    pub path: PathBuf,
    pub name: String,
    pub friendly_name: String,
    pub description: String,
    pub entry_points: Vec<EntryPoint>,
    /// The tiles (`[map]` x, y), in the file's order: a track's tile index points here.
    pub tiles: Vec<(i32, i32)>,
    /// The map drives on the left.
    pub left_hand_traffic: bool,
    /// The depot file the map gives the player's bus (`[standarddepot]`).
    pub standard_depot: Option<String>,
}

impl GlobalCfg {
    pub fn parse(cfg: &CfgFile) -> GlobalCfg {
        todo!("hub-omsi port: global.cfg")
    }

    pub fn load(path: &Path) -> Result<GlobalCfg, ReadError> {
        Ok(GlobalCfg::parse(&CfgFile::read(path)?))
    }
}

/// A holiday of the calendar.
#[derive(Debug, Clone, PartialEq)]
pub struct Holiday {
    pub date: NaiveDate,
    pub name: String,
}

/// A stretch of school holidays.
#[derive(Debug, Clone, PartialEq)]
pub struct HolidayRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub name: String,
}

/// A map's calendar (`Holidays.txt`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Calendar {
    pub holidays: Vec<Holiday>,
    pub ranges: Vec<HolidayRange>,
}

impl Calendar {
    pub fn parse(cfg: &CfgFile) -> Calendar {
        todo!("hub-omsi port: Holidays.txt")
    }

    /// The map folder's `Holidays.txt` (none: an empty calendar).
    pub fn load(map_dir: &Path) -> Result<Calendar, ReadError> {
        match crate::vfs::find_child(map_dir, "Holidays.txt") {
            Some(p) => Ok(Calendar::parse(&CfgFile::read(p)?)),
            None => Ok(Calendar::default()),
        }
    }

    pub fn is_holiday(&self, d: NaiveDate) -> bool {
        self.holidays.iter().any(|h| h.date == d)
    }

    pub fn in_holiday_range(&self, d: NaiveDate) -> bool {
        self.ranges.iter().any(|r| r.start <= d && d <= r.end)
    }
}

/// A folder of the map's `Chrono`: from when (and until when) its files take the place of the
/// map's own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChronoDir {
    pub path: PathBuf,
    pub name: String,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// The map folder's Chrono folders active on `date`, in the order they apply.
pub fn active_chrono_dirs(map_dir: &Path, date: NaiveDate) -> Vec<ChronoDir> {
    todo!("hub-omsi port: Chrono folders")
}

/// The lines the active Chrono folders switch off.
pub fn chrono_deactivated_lines(chrono: &[ChronoDir]) -> Vec<String> {
    todo!("hub-omsi port: Chrono's switched-off lines")
}

/// A livery and number rule of a type group.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypGroupEntry {
    /// From and until which date (`YYYYMMDD`) it holds.
    pub from: Option<i32>,
    pub to: Option<i32>,
    pub paint: String,
    pub number: String,
}

impl TypGroupEntry {
    /// It holds on the day `code` (`YYYYMMDD`).
    pub fn valid(&self, code: i32) -> bool {
        self.from.is_none_or(|f| f <= code) && self.to.is_none_or(|t| code <= t)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypGroup {
    pub file: String,
    pub entries: Vec<TypGroupEntry>,
}

/// A bus of an AI group, and how often it is taken.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiVehicle {
    pub file: String,
    pub weight: f32,
    pub number: Option<String>,
}

/// An AI group of `ailists.cfg`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiGroup {
    pub name: String,
    /// A depot group (the timetable's buses), with its depot file.
    pub is_depot: bool,
    pub hof: Option<String>,
    pub typgroups: Vec<TypGroup>,
    pub vehicles: Vec<AiVehicle>,
}

/// A map's `ailists.cfg`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiLists {
    pub groups: Vec<AiGroup>,
}

impl AiLists {
    pub fn parse(cfg: &CfgFile) -> AiLists {
        todo!("hub-omsi port: ailists.cfg")
    }

    pub fn load(path: &Path) -> Result<AiLists, ReadError> {
        Ok(AiLists::parse(&CfgFile::read(path)?))
    }

    /// The map folder's `ailists.cfg`, a Chrono folder's copy winning.
    pub fn with_chrono(map_dir: &Path, chrono: &[ChronoDir]) -> AiLists {
        todo!("hub-omsi port: ailists.cfg with Chrono")
    }

    pub fn group(&self, name: &str) -> Option<&AiGroup> {
        self.groups.iter().find(|g| g.name.eq_ignore_ascii_case(name.trim()))
    }
}

/// The depot file the map's depot group gives buses on `date`.
pub fn depot_hof_on(map_dir: &Path, date: NaiveDate) -> Option<String> {
    todo!("hub-omsi port: the day's depot file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_codes_and_ranges() {
        assert_eq!(date_code("1989-05-30"), Some(19_890_530));
        assert_eq!(date_code("30.05.1989"), None);
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let cal =
            Calendar { holidays: vec![Holiday { date: d("1989-12-25"), name: "Xmas".into() }], ranges: vec![HolidayRange { start: d("1989-07-01"), end: d("1989-08-15"), name: "Summer".into() }] };
        assert!(cal.is_holiday(d("1989-12-25")) && !cal.is_holiday(d("1989-12-24")));
        assert!(cal.in_holiday_range(d("1989-08-15")) && !cal.in_holiday_range(d("1989-08-16")));
        let e = TypGroupEntry { from: Some(19_900_101), to: None, ..TypGroupEntry::default() };
        assert!(!e.valid(19_891_231) && e.valid(19_900_101));
    }
}
