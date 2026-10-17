//! Omsi-Hub's own OMSI 2 file readers and writers - only what Omsi-Hub needs, written from the
//! file formats (never from openOMSI's code). No OMSI 2 content is in this repository: the tests
//! use small synthetic files.
//!
//! * [`text`] - the OMSI text format (`[keyword]` lines and their values), its codepages
//!   (Windows-1252, -1250, -1251 … or UTF-8) detected, decoded and kept for writing back;
//! * [`vfs`] - the content as openOMSI sees it: its content folder first, then the OMSI 2
//!   folder; names in any case, either slash;
//! * [`timetable`] - a map's `TTData`: lines (`.ttl`), trips (`.ttp`), tracks (`.ttr`),
//!   `StnLinks.cfg`, `Busstops.cfg`, read and written; and the lines of a day
//!   ([`timetable::lines_for`]) with the map's Chrono and calendar;
//! * [`map`] - a map's `global.cfg`, `ailists.cfg`, `Holidays.txt` (the calendar) and its Chrono
//!   folders;
//! * [`hof`] - depot files (`.hof`), read and written;
//! * [`vehicle`] - a bus's `.bus` header, its `passengercabin.cfg`;
//! * [`font`] - the names in a display font (`.oft`);
//! * [`weather`] - a weather file (`.owt`).
//!
//! **Status:** [`text`] and [`vfs`] work; the other modules are the contract the ports code
//! against - their types are fixed, their readers are `todo!()` until the hub-omsi port fills
//! them in (with tests on synthetic fixtures).

pub mod error;
pub mod font;
pub mod hof;
pub mod map;
pub mod text;
pub mod timetable;
pub mod vehicle;
pub mod vfs;
pub mod weather;

pub use error::ReadError;
pub use text::{CfgFile, Codepage};
pub use vfs::Vfs;
