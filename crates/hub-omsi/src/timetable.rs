//! A map's timetable (`<map>/TTData`): read, written, and made into the lines of a day.
//!
//! The files, in OMSI's text format ([`crate::text`]):
//! * `<line>.ttl` - a line ([`Line`]): `[userallowed]` (no value: the player may drive it),
//!   `[priority]`, and its tours (`[newtour]`: number, AI group, day mask; then its trips,
//!   `[addtrip]`: the trip, the profile, the departure in minutes of the day);
//! * `<trip>.ttp` - a trip ([`Trip`]): `[trip]` (the track it drives - empty: the one named
//!   like the trip -, its terminus, its line), `[trainreverse]`, its stops (`[station_typ2]`:
//!   the stop's object id; or the older `[station]`: the id, its place in the track, its name and
//!   five more values) and its profiles (`[profile]`: a name and the trip's whole time in
//!   minutes, then `[profile_man_arr_time]`/`[profile_man_dep_time]` - a stop's place and its
//!   fixed time, minutes from the start - and `[profile_otherstopping]` - a stop's place and
//!   how it is served);
//! * `<track>.ttr` - the lanes a trip drives ([`Track`], `[track_entry]`: the spline, its
//!   path, the tile, a fourth value, the length, a sixth);
//! * `StnLinks.cfg` - the links between stops ([`StnLink`]: `[StnLink]` - length, the two
//!   stops, six values - and its `[StnLink_entry]` lanes, seven values each);
//! * `Busstops.cfg` - the stops the map names ([`BusStopEntry`], `[busstop]`: name, tile,
//!   object id, three values).
//!
//! Stop times are interpolated along a trip's profile: the profile's whole time shared by the
//! links' lengths, its fixed departures and arrivals kept ([`stop_times`]).
//!
//! A tour's day mask ([`DayMask`]): bits 0-6 Monday to Sunday, bit 7 public holidays, bit 8
//! school days, bit 9 school holidays, as `Omsi.exe` reads it: a tour runs when the day's bit
//! (the public holiday's on one, else the weekday's) and its school bit (the school holidays' in
//! them, else the school days') are both set. A mask without school bits (older maps) runs
//! whatever the school calendar.

use crate::map::Calendar;
use crate::text::{keyword_of, parse_f32, parse_f64, parse_i32, CfgFile, Codepage};
use crate::vfs::{find_child, Vfs};
use crate::ReadError;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A tour's days: bit 0 Monday … bit 6 Sunday, bit 7 public holidays, bit 8 school days, bit 9
/// school holidays (see the module's text).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DayMask(pub u32);

impl DayMask {
    pub const WEEKDAYS: DayMask = DayMask(0b001_1111);
    pub const DAILY: DayMask = DayMask(0b111_1111);
    /// Public holidays.
    pub const HOLIDAY: u32 = 1 << 7;
    pub const SCHOOL_DAYS: u32 = 1 << 8;
    pub const SCHOOL_HOLIDAYS: u32 = 1 << 9;
    /// Every day of every kind: what a tour without a mask is taken for.
    pub const ALWAYS: DayMask = DayMask(0b11_1111_1111);

    /// It runs on this weekday (0 = Monday).
    pub fn on_weekday(self, day: u32) -> bool {
        day < 7 && self.0 & (1 << day) != 0
    }

    /// It runs on `date`, by the map's calendar (as `Omsi.exe` decides it).
    pub fn runs_on(self, calendar: &Calendar, date: NaiveDate) -> bool {
        let day = if calendar.is_holiday(date) { Self::HOLIDAY } else { 1 << date.weekday().num_days_from_monday() };
        let school_bits = self.0 & (Self::SCHOOL_DAYS | Self::SCHOOL_HOLIDAYS);
        let school = if calendar.in_holiday_range(date) { Self::SCHOOL_HOLIDAYS } else { Self::SCHOOL_DAYS };
        self.0 & day != 0 && (school_bits == 0 || self.0 & school != 0)
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

/// A stop of a trip (from `[station_typ2]`, or the older `[station]`: normalised to one form).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripStop {
    /// The stop's map object id.
    pub id: i64,
    /// `[station]`'s own name; else the one `Busstops.cfg` gives the id (filled in by
    /// [`Timetable::load`]).
    pub name: String,
    /// Its place in the trip's track, where the file says it (`[station]`; such a stop is
    /// written back as one).
    pub track_index: Option<usize>,
    /// `[station]`'s values after the name, as they are.
    pub extra: Vec<String>,
}

/// A trip (`.ttp`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trip {
    pub path: PathBuf,
    /// The file's name without `.ttp`.
    pub name: String,
    /// `[trip]`'s first value as the file has it: the track it names (empty: the one named
    /// like the trip).
    pub display_name: String,
    /// The track (`.ttr`) it drives: the one `[trip]` names, else the trip's own name.
    pub track: String,
    pub terminus: String,
    pub line: String,
    pub stops: Vec<TripStop>,
    pub profiles: Vec<TripProfile>,
    /// `[trainreverse]`.
    pub train_reverse: bool,
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
    /// Every value as the file has it (written back as they are; empty: from the fields).
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

// --- reading -------------------------------------------------------------------------------

fn stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A whole number of any size, leniently (as [`parse_i32`]).
pub fn parse_i64(s: &str) -> i64 {
    let t = s.trim();
    let end = t.char_indices().find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+')))).map_or(t.len(), |(i, _)| i);
    t[..end].parse().unwrap_or(0)
}

fn kw_is(k: &str, name: &str) -> bool {
    k.eq_ignore_ascii_case(name)
}

impl Timetable {
    /// Every file of the TTData folder `dir` (a missing or unreadable file is left out).
    pub fn load(dir: &Path) -> Timetable {
        Timetable::from_dirs(&[dir.to_path_buf()])
    }

    /// The same with the Chrono folders active on the day: their lines, trips and tracks take
    /// the place of the TTData ones of the same name; their `StnLinks.cfg` and `Busstops.cfg`
    /// add to the map's (a link of the same two stops, a stop of the same id, replaced); the
    /// lines they switch off are left out.
    pub fn load_with_chrono(dir: &Path, chrono: &[crate::map::ChronoDir], off: &[String]) -> Timetable {
        let mut dirs = vec![dir.to_path_buf()];
        dirs.extend(chrono.iter().filter_map(|c| find_child(&c.path, "TTData")));
        let mut tt = Timetable::from_dirs(&dirs);
        tt.lines.retain(|l| !off.iter().any(|o| o.trim().eq_ignore_ascii_case(l.name.trim())));
        tt
    }

    /// The files of `dirs`, a later folder's taking the place of an earlier one's (by their
    /// names in any case), its links and stops added.
    fn from_dirs(dirs: &[PathBuf]) -> Timetable {
        let mut by_name: Vec<(String, PathBuf)> = Vec::new();
        let mut tt = Timetable::default();
        for d in dirs {
            let Ok(entries) = std::fs::read_dir(d) else { continue };
            let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
            files.sort();
            for p in files {
                let key = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                if key == "stnlinks.cfg" || key == "busstops.cfg" {
                    let Ok(cfg) = CfgFile::read(&p) else { continue };
                    if key == "stnlinks.cfg" {
                        for l in parse_stnlinks(&cfg) {
                            tt.stn_links.retain(|o| (o.from_id, o.to_id) != (l.from_id, l.to_id));
                            tt.stn_links.push(l);
                        }
                    } else {
                        for b in parse_busstops(&cfg) {
                            tt.bus_stops.retain(|o| o.object_id != b.object_id);
                            tt.bus_stops.push(b);
                        }
                    }
                    continue;
                }
                match by_name.iter_mut().find(|(k, _)| *k == key) {
                    Some(slot) => slot.1 = p,
                    None => by_name.push((key, p)),
                }
            }
        }
        by_name.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, p) in &by_name {
            let Ok(cfg) = CfgFile::read(p) else { continue };
            match p.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
                Some("ttl") => tt.lines.push(Line::parse(&cfg)),
                Some("ttp") => tt.trips.push(Trip::parse(&cfg)),
                Some("ttr") => tt.tracks.push(Track::parse(&cfg)),
                _ => {}
            }
        }
        for i in 0..tt.trips.len() {
            for k in 0..tt.trips[i].stops.len() {
                if tt.trips[i].stops[k].name.trim().is_empty() {
                    if let Some(n) = tt.stop_name(tt.trips[i].stops[k].id) {
                        tt.trips[i].stops[k].name = n.to_string();
                    }
                }
            }
        }
        tt
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

    /// The name `Busstops.cfg` gives the stop with this object id.
    pub fn stop_name(&self, id: i64) -> Option<&str> {
        self.bus_stops.iter().find(|b| b.object_id == id).map(|b| b.name.trim())
    }

    /// The track a trip drives: the one it names, else the one named like it.
    pub fn track_of(&self, trip: &Trip) -> Option<&Track> {
        self.track(&trip.track).or_else(|| self.track(&trip.name))
    }
}

impl Line {
    pub fn parse(cfg: &CfgFile) -> Line {
        let mut line = Line { path: cfg.path.clone(), name: stem(&cfg.path), codepage: Some(cfg.codepage), ..Line::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            if kw_is(k, "userallowed") {
                line.user_allowed = true;
            } else if kw_is(k, "priority") {
                line.priority = parse_i32(r.value());
            } else if kw_is(k, "newtour") {
                let v = r.values(3);
                let at = |i: usize| v.get(i).map_or("", |s| s.trim());
                let days = if at(2).is_empty() { DayMask::ALWAYS } else { DayMask(parse_i32(at(2)).max(0) as u32) };
                line.tours.push(Tour { number: at(0).to_string(), ai_group: at(1).to_string(), days, trips: Vec::new() });
            } else if kw_is(k, "addtrip") {
                let v = r.values(3);
                let at = |i: usize| v.get(i).map_or("", |s| s.trim());
                if let Some(t) = line.tours.last_mut() {
                    t.trips.push(TourTrip { trip: at(0).to_string(), profile: parse_i32(at(1)), departure: parse_f32(at(2)) });
                }
            }
        }
        line
    }

    pub fn load(path: &Path) -> Result<Line, ReadError> {
        Ok(Line::parse(&CfgFile::read(path)?))
    }

    /// The file's text again (`\r\n` line ends).
    pub fn to_text(&self) -> String {
        let mut s = header("Line");
        if self.user_allowed {
            s.push_str("[userallowed]\r\n\r\n");
        }
        s.push_str(&format!("[priority]\r\n{}\r\n------------------------------------\r\n", self.priority));
        for t in &self.tours {
            s.push_str(&format!("\r\n[newtour]\r\n{}\r\n{}\r\n{}\r\n\r\n------------------------------------\r\n", t.number, t.ai_group, t.days.0));
            for x in &t.trips {
                let secs = (x.departure as f64 * 60.0).round().max(0.0) as i64;
                s.push_str(&format!("\r\n  Dep.: {}:{}:{}\r\n[addtrip]\r\n{}\r\n{}\r\n{}\r\n", secs / 3600, secs / 60 % 60, secs % 60, x.trip, x.profile, dec32(x.departure, 3)));
            }
        }
        s
    }
}

impl Trip {
    pub fn parse(cfg: &CfgFile) -> Trip {
        let mut trip = Trip { path: cfg.path.clone(), name: stem(&cfg.path), ..Trip::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            if kw_is(k, "trip") {
                let v = r.values(3);
                let at = |i: usize| v.get(i).map_or("", |s| s.trim());
                trip.display_name = at(0).to_string();
                trip.terminus = at(1).to_string();
                trip.line = at(2).to_string();
            } else if kw_is(k, "trainreverse") {
                trip.train_reverse = true;
            } else if kw_is(k, "station_typ2") {
                trip.stops.push(TripStop { id: parse_i64(r.value()), ..TripStop::default() });
            } else if kw_is(k, "station") {
                let v = r.values(8);
                let at = |i: usize| v.get(i).map_or("", |s| s.trim());
                let track_index = at(1).parse::<usize>().ok().or(Some(0));
                trip.stops.push(TripStop { id: parse_i64(at(0)), name: at(2).to_string(), track_index, extra: v.iter().skip(3).map(|s| s.trim().to_string()).collect() });
            } else if kw_is(k, "profile") {
                let v = r.values(2);
                trip.profiles.push(TripProfile { name: v.first().map_or("", |s| s.trim()).to_string(), minutes: v.get(1).map_or(0.0, |s| parse_f32(s)), ..TripProfile::default() });
            } else if let Some(kind) = ["profile_man_arr_time", "profile_man_dep_time", "profile_otherstopping"].iter().position(|x| kw_is(k, x)) {
                let v = r.values(2);
                let (Some(p), [i, x]) = (trip.profiles.last_mut(), v.as_slice()) else { continue };
                match kind {
                    0 => p.fixed_arrivals.push((parse_i32(i), parse_f32(x))),
                    1 => p.fixed_departures.push((parse_i32(i), parse_f32(x))),
                    _ => p.stopping.push((parse_i32(i), parse_i32(x))),
                }
            }
        }
        trip.track = if trip.display_name.is_empty() { trip.name.clone() } else { trip.display_name.clone() };
        trip
    }

    pub fn load(path: &Path) -> Result<Trip, ReadError> {
        Ok(Trip::parse(&CfgFile::read(path)?))
    }

    pub fn to_text(&self) -> String {
        let mut s = header("Trip");
        // (the track: empty when it is the trip's own, as OMSI writes it)
        let track = if !self.display_name.trim().is_empty() || !self.track.trim().eq_ignore_ascii_case(self.name.trim()) { self.track.trim() } else { "" };
        s.push_str(&format!("[trip]\r\n{track}\r\n{}\r\n{}\r\n\r\n", self.terminus, self.line));
        if self.train_reverse {
            s.push_str("[trainreverse]\r\n\r\n");
        }
        s.push_str(".........................\r\n        Stations\r\n.........................\r\n\r\n");
        for st in &self.stops {
            match st.track_index {
                None => s.push_str(&format!("[station_typ2]\r\n{}\r\n\r\n", st.id)),
                Some(i) => {
                    s.push_str(&format!("[station]\r\n{}\r\n{i}\r\n{}\r\n", st.id, st.name));
                    // (OMSI reads eight values: those the stop lacks written as 0)
                    for k in 0..st.extra.len().max(5) {
                        s.push_str(st.extra.get(k).map_or("0", String::as_str));
                        s.push_str("\r\n");
                    }
                    s.push_str("\r\n");
                }
            }
        }
        s.push_str("\r\n.........................\r\n        Profiles\r\n.........................\r\n");
        for p in &self.profiles {
            s.push_str(&format!("\r\n[profile]\r\n{}\r\n{}\r\n\r\n", p.name, dec32(p.minutes, 3)));
            // (stop by stop, as OMSI writes them: arrival, departure, then how it stops)
            let (mut a, mut d, mut o) = (0, 0, 0);
            loop {
                let next = [p.fixed_arrivals.get(a).map(|x| x.0), p.fixed_departures.get(d).map(|x| x.0), p.stopping.get(o).map(|x| x.0)];
                let Some(min) = next.iter().flatten().min().copied() else { break };
                if next[0] == Some(min) {
                    s.push_str(&format!("[profile_man_arr_time]\r\n{min}\r\n{}\r\n", dec32(p.fixed_arrivals[a].1, 3)));
                    a += 1;
                } else if next[1] == Some(min) {
                    s.push_str(&format!("[profile_man_dep_time]\r\n{min}\r\n{}\r\n", dec32(p.fixed_departures[d].1, 3)));
                    d += 1;
                } else {
                    s.push_str(&format!("[profile_otherstopping]\r\n{min}\r\n{}\r\n", p.stopping[o].1));
                    o += 1;
                }
            }
        }
        s
    }
}

impl Track {
    pub fn parse(cfg: &CfgFile) -> Track {
        let mut t = Track { path: cfg.path.clone(), name: stem(&cfg.path), entries: Vec::new() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            if kw_is(k, "track_entry") {
                let values: Vec<f64> = r.run().into_iter().map(parse_f64).collect();
                let at = |i: usize| values.get(i).copied().unwrap_or(0.0);
                t.entries.push(TrackEntry { spline: at(0) as i64, path: at(1) as i32, tile: at(2).max(0.0) as usize, length: at(4), values: values.clone() });
            }
        }
        t
    }

    pub fn load(path: &Path) -> Result<Track, ReadError> {
        Ok(Track::parse(&CfgFile::read(path)?))
    }

    pub fn to_text(&self) -> String {
        let mut s = header("Track");
        for (i, e) in self.entries.iter().enumerate() {
            let values = if e.values.is_empty() { vec![e.spline as f64, e.path as f64, e.tile as f64, 0.0, e.length, 0.0] } else { e.values.clone() };
            s.push_str(&format!("\r\n   {i}:\r\n[track_entry]\r\n"));
            for (k, v) in values.iter().enumerate() {
                s.push_str(&if k == 4 { dec(*v, 10) } else { num(*v) });
                s.push_str("\r\n");
            }
        }
        s
    }
}

impl StnLink {
    /// The link's block for `StnLinks.cfg`, with the two stops' names as its comment.
    pub fn to_text(&self, from_name: &str, to_name: &str) -> String {
        let mut s = format!("\r\n{} ==> {}\r\n\r\n[StnLink]\r\n{}\r\n{}\r\n{}\r\n", from_name.trim(), to_name.trim(), dec(self.length, 10), self.from_id, self.to_id);
        for (k, v) in self.params.iter().enumerate() {
            s.push_str(&if k < 4 { dec(*v, 10) } else { num(*v) });
            s.push_str("\r\n");
        }
        for (i, e) in self.entries.iter().enumerate() {
            s.push_str(&format!("\r\n   {i}:\r\n[StnLink_entry]\r\n"));
            for (k, v) in e.iter().enumerate() {
                s.push_str(&if k == 3 { dec(*v, 10) } else { num(*v) });
                s.push_str("\r\n");
            }
        }
        s
    }
}

impl BusStopEntry {
    pub fn to_text(&self) -> String {
        format!("\r\n[busstop]\r\n{}\r\n{}\r\n{}\r\n{}\r\n{}\r\n{}\r\n\r\n", self.name, self.tile, self.object_id, dec(self.params[0], 10), num(self.params[1]), num(self.params[2]))
    }
}

pub fn parse_stnlinks(cfg: &CfgFile) -> Vec<StnLink> {
    let mut out: Vec<StnLink> = Vec::new();
    let mut r = cfg.reader();
    while let Some(k) = r.next_keyword() {
        if kw_is(k, "StnLink") {
            let v = r.run();
            let f = |i: usize| v.get(i).map_or(0.0, |s| parse_f64(s));
            out.push(StnLink {
                length: f(0),
                from_id: v.get(1).map_or(0, |s| parse_i64(s)),
                to_id: v.get(2).map_or(0, |s| parse_i64(s)),
                params: [f(3), f(4), f(5), f(6), f(7), f(8)],
                entries: Vec::new(),
            });
        } else if kw_is(k, "StnLink_entry") {
            let v = r.run();
            let mut e = [0.0; 7];
            for (slot, s) in e.iter_mut().zip(&v) {
                *slot = parse_f64(s);
            }
            if let Some(l) = out.last_mut() {
                l.entries.push(e);
            }
        }
    }
    out
}

pub fn parse_busstops(cfg: &CfgFile) -> Vec<BusStopEntry> {
    let mut out = Vec::new();
    let mut r = cfg.reader();
    while let Some(k) = r.next_keyword() {
        if kw_is(k, "busstop") {
            let v = r.values(6);
            let f = |i: usize| v.get(i).map_or(0.0, |s| parse_f64(s));
            out.push(BusStopEntry {
                name: v.first().map_or("", |s| s.trim()).to_string(),
                tile: v.get(1).map_or(0, |s| parse_i32(s)),
                object_id: v.get(2).map_or(0, |s| parse_i64(s)),
                params: [f(3), f(4), f(5)],
            });
        }
    }
    out
}

// --- writing -------------------------------------------------------------------------------

/// The head OMSI writes above a timetable file of `kind` (`Line`, `Trip`, `Track`).
fn header(kind: &str) -> String {
    format!("-----------------------\r\nTime Table {kind} File\r\n-----------------------\r\n\r\nCreated with Omsi-Hub\r\n\r\n")
}

/// A value as OMSI writes a whole number, or as short as it reads back the same.
fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// A decimal with `places` places when that reads back the same, else as short as it does.
fn dec(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    if parse_f64(&s) == v {
        s
    } else {
        format!("{v}")
    }
}

fn dec32(v: f32, places: usize) -> String {
    let s = format!("{v:.places$}");
    if parse_f64(&s) as f32 == v {
        s
    } else {
        format!("{v}")
    }
}

/// `text` written to `path` in `cp`, its line ends `\r\n`, through a temporary file. A file
/// in UTF-16 gets its BOM; one in UTF-8 keeps the BOM the file it replaces had.
pub fn write_text(path: &Path, text: &str, cp: Codepage) -> std::io::Result<()> {
    let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
    let had_bom = cp == Codepage::Utf8 && std::fs::read(path).is_ok_and(|b| b.starts_with(cp.bom()));
    let mut bytes = if cp.is_utf16() || had_bom { cp.bom().to_vec() } else { Vec::new() };
    bytes.extend(crate::text::encode(&text, cp).0);
    let tmp = path.with_extension("hubtmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// How many values a block of `StnLinks.cfg` or `Busstops.cfg` has (None: up to a blank line).
fn value_count(kw: &str) -> Option<usize> {
    match kw.to_ascii_lowercase().as_str() {
        "stnlink" => Some(9),
        "stnlink_entry" => Some(7),
        "busstop" => Some(6),
        _ => None,
    }
}

/// What tells a block from the others of its keyword: a link's two stops, a stop's object
/// id, else its first value.
fn block_id(kw: &str, values: &[&str]) -> String {
    let at = |i: usize| values.get(i).map_or("", |s| s.trim());
    match kw.to_ascii_lowercase().as_str() {
        "stnlink" => format!("{}>{}", parse_i64(at(1)), parse_i64(at(2))),
        "busstop" => parse_i64(at(2)).to_string(),
        _ => at(0).to_string(),
    }
}

/// Past the last value of the block whose keyword is at `start` (its lanes' blocks with it),
/// before `limit`.
fn data_end(lines: &[&str], start: usize, limit: usize) -> usize {
    let (mut i, mut last) = (start, start + 1);
    while i < limit {
        let Some(k) = keyword_of(lines[i]) else {
            i += 1;
            continue;
        };
        let n = value_count(k);
        let mut j = i + 1;
        while j < limit && keyword_of(lines[j]).is_none() && n.map_or(!lines[j].trim().is_empty(), |n| j - i <= n) {
            j += 1;
        }
        last = j;
        i = j;
    }
    last
}

/// The comment line just above the keyword at `at` (`A ==> B` over a link), when it stands
/// alone.
fn caption_above(lines: &[&str], at: usize) -> Option<usize> {
    let mut j = at.checked_sub(1)?;
    while lines[j].trim().is_empty() {
        j = j.checked_sub(1)?;
    }
    let alone = j == 0 || lines[j - 1].trim().is_empty();
    (keyword_of(lines[j]).is_none() && alone).then_some(j)
}

/// `text` with its block that starts like `block` (the same keyword and first value - for a
/// `[StnLink]` its two stops, for a `[busstop]` its object id) replaced by `block`, or with
/// `block` added at its end. The comment line above a block goes with it when `block` brings
/// its own. An empty `block` leaves `text` as it is.
pub fn replace_block(text: &str, block: &str) -> String {
    let split = |t: &str| -> Vec<String> {
        let t = t.replace("\r\n", "\n").replace('\r', "\n");
        let mut v: Vec<String> = t.split('\n').map(str::to_string).collect();
        if v.last().is_some_and(String::is_empty) {
            v.pop();
        }
        v
    };
    let blines = split(block);
    let Some(bk) = blines.iter().position(|l| keyword_of(l).is_some()) else { return text.to_string() };
    let kw = keyword_of(&blines[bk]).unwrap_or_default().to_string();
    let brefs: Vec<&str> = blines.iter().map(String::as_str).collect();
    let id = block_id(&kw, &brefs[bk + 1..]);
    let has_caption = brefs[..bk].iter().any(|l| !l.trim().is_empty());
    // (the block without the blank lines around it)
    let first = brefs.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
    let last = brefs.iter().rposition(|l| !l.trim().is_empty()).map_or(0, |i| i + 1);
    let body = &brefs[first..last];

    let tlines = split(text);
    let lines: Vec<&str> = tlines.iter().map(String::as_str).collect();
    let starts: Vec<usize> = (0..lines.len()).filter(|i| keyword_of(lines[*i]).is_some_and(|k| k.eq_ignore_ascii_case(&kw))).collect();
    for (n, &s) in starts.iter().enumerate() {
        let limit = starts.get(n + 1).copied().unwrap_or(lines.len());
        if block_id(&kw, &lines[s + 1..limit]) != id {
            continue;
        }
        let end = data_end(&lines, s, limit);
        let begin = if has_caption { caption_above(&lines, s).unwrap_or(s) } else { s };
        let mut out: Vec<&str> = lines[..begin].to_vec();
        out.extend_from_slice(body);
        out.extend_from_slice(&lines[end..]);
        return out.join("\r\n") + "\r\n";
    }
    let mut out = lines.join("\r\n");
    if !out.is_empty() {
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    out.push_str(&body.join("\r\n"));
    out.push_str("\r\n");
    out
}

// --- stop times ----------------------------------------------------------------------------

/// The length of each leg of `trip` (m): its station link's, else its track's lanes between
/// the two stops' places; None where neither says.
pub fn leg_lengths(tt: &Timetable, trip: &Trip) -> Vec<Option<f64>> {
    let track = tt.track_of(trip);
    trip.stops
        .windows(2)
        .map(|w| {
            if let Some(l) = tt.link(w[0].id, w[1].id).filter(|l| l.length > 0.0) {
                return Some(l.length);
            }
            let (t, a, b) = (track?, w[0].track_index?, w[1].track_index?);
            (a < b && b <= t.entries.len()).then(|| t.entries[a..b].iter().map(|e| e.length).sum::<f64>()).filter(|l| *l > 0.0)
        })
        .collect()
}

/// The trip's length (m): its legs', a leg without one taken as long as the others' average;
/// else its track's.
pub fn trip_length(tt: &Timetable, trip: &Trip) -> f64 {
    let legs = leg_lengths(tt, trip);
    let known: Vec<f64> = legs.iter().flatten().copied().collect();
    if known.is_empty() {
        return tt.track_of(trip).map_or(0.0, |t| t.entries.iter().map(|e| e.length).sum());
    }
    let avg = known.iter().sum::<f64>() / known.len() as f64;
    legs.iter().map(|l| l.unwrap_or(avg)).sum()
}

/// When `trip`, driven with its profile `profile` from `start` (seconds of the day), is due at
/// each of its stops: (arrival, departure), seconds of the day.
///
/// The profile's fixed times hold (a stop's arrival, its departure, the first stop's departure
/// at 0 and the last's arrival at the profile's whole time when the profile does not fix them);
/// the stops between are shared out by the legs' lengths. A trip without a profile's time
/// drives at 24 km/h; a leg of unknown length counts as the others' average.
pub fn stop_times(tt: &Timetable, trip: &Trip, profile: usize, start: f64) -> Vec<(f64, f64)> {
    let n = trip.stops.len();
    if n == 0 {
        return Vec::new();
    }
    let legs = leg_lengths(tt, trip);
    let known: Vec<f64> = legs.iter().flatten().copied().collect();
    let avg = if known.is_empty() { 300.0 } else { known.iter().sum::<f64>() / known.len() as f64 };
    let legs: Vec<f64> = legs.iter().map(|l| l.unwrap_or(avg).max(1.0)).collect();
    let mut cum = vec![0.0; n];
    for i in 1..n {
        cum[i] = cum[i - 1] + legs[i - 1];
    }
    let p = trip.profiles.get(profile).or(trip.profiles.first());
    let minutes = p.map(|p| p.minutes as f64).filter(|m| *m > 0.0).unwrap_or(cum[n - 1] / 400.0);
    let fixed = |list: Option<&Vec<(i32, f32)>>, i: usize| list.and_then(|l| l.iter().rev().find(|(k, _)| *k >= 0 && *k as usize == i)).map(|x| x.1 as f64);
    let mut arr: Vec<Option<f64>> = (0..n).map(|i| fixed(p.map(|p| &p.fixed_arrivals), i)).collect();
    let mut dep: Vec<Option<f64>> = (0..n).map(|i| fixed(p.map(|p| &p.fixed_departures), i)).collect();
    dep[0] = dep[0].or(arr[0]).or(Some(0.0));
    if n > 1 {
        arr[n - 1] = arr[n - 1].or(Some(minutes));
    }
    for i in 0..n {
        arr[i] = arr[i].or(dep[i]);
        dep[i] = dep[i].or(arr[i]);
    }
    // (between two stops of fixed times, by the legs' lengths)
    let mut j = 0;
    for k in 1..n {
        let (Some(tk), Some(tj)) = (arr[k], dep[j]) else { continue };
        for i in j + 1..k {
            let t = tj + (tk - tj) * (cum[i] - cum[j]) / (cum[k] - cum[j]).max(1e-6);
            arr[i] = Some(t);
            dep[i] = Some(t);
        }
        j = k;
    }
    let mut out = Vec::with_capacity(n);
    let mut prev = f64::MIN;
    for i in 0..n {
        let a = arr[i].unwrap_or(0.0).max(prev);
        let d = dep[i].unwrap_or(a).max(a);
        prev = d;
        out.push((start + a * 60.0, start + d * 60.0));
    }
    out
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

/// A tour's trips on the day: their stops' times from their departures (a departure earlier
/// than the one before it is the next day's, above 86 400 s).
fn service_trips(tt: &Timetable, tour: &Tour) -> Vec<ServiceTrip> {
    let mut out = Vec::new();
    let mut day = 0.0;
    let mut prev = f64::MIN;
    for (i, x) in tour.trips.iter().enumerate() {
        let mut dep = x.departure as f64 * 60.0 + day;
        if dep + 43_200.0 < prev {
            day += 86_400.0;
            dep += 86_400.0;
        }
        prev = dep;
        let mut st = ServiceTrip { name: x.trip.clone(), index: i + 1, departure: dep, arrival: dep, ..ServiceTrip::default() };
        if let Some(trip) = tt.trip(&x.trip) {
            let times = stop_times(tt, trip, x.profile.max(0) as usize, dep);
            st.stops = trip
                .stops
                .iter()
                .zip(&times)
                .map(|(s, &(arr, dep))| ServiceStop { name: if s.name.trim().is_empty() { tt.stop_name(s.id).unwrap_or("").to_string() } else { s.name.trim().to_string() }, id: s.id, arr, dep })
                .collect();
            st.line = trip.line.trim().to_string();
            st.from = st.stops.first().map(|s| s.name.clone()).unwrap_or_default();
            st.terminus = st.stops.last().map(|s| s.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| trip.terminus.trim().to_string());
            st.arrival = st.stops.last().map_or(dep, |s| s.arr);
            st.km = trip_length(tt, trip) / 1000.0;
            prev = prev.max(st.arrival);
        }
        out.push(st);
    }
    out
}

/// The lines of `tt` on `date` with the calendar `cal`.
pub fn service_lines(tt: &Timetable, cal: &Calendar, date: NaiveDate) -> Vec<ServiceLine> {
    tt.lines
        .iter()
        .map(|l| {
            let tours: Vec<ServiceTour> = l
                .tours
                .iter()
                .map(|t| {
                    let trips = service_trips(tt, t);
                    let next_run = (0..=366).filter_map(|k| date.checked_add_days(chrono::Days::new(k))).find(|d| t.days.runs_on(cal, *d));
                    ServiceTour {
                        number: t.number.clone(),
                        ai_group: t.ai_group.clone(),
                        first: trips.first().map_or(0.0, |x| x.departure),
                        last: trips.last().map_or(0.0, |x| x.arrival),
                        days: t.days,
                        runs: t.days.runs_on(cal, date),
                        next_run,
                        trips,
                    }
                })
                .collect();
            let longest = tours.iter().flat_map(|t| &t.trips).filter(|t| !t.stops.is_empty()).max_by_key(|t| t.stops.len());
            let termini = longest.map(|t| vec![t.from.clone(), t.terminus.clone()]).unwrap_or_default();
            ServiceLine { name: l.name.clone(), user_allowed: l.user_allowed, termini, tours }
        })
        .collect()
}

/// The lines of `map` (its `global.cfg`, relative to the OMSI 2 folder) on `date`: the
/// TTData with the day's Chrono (its files, the lines it switches off), each tour's days
/// checked against the map's calendar (`Holidays.txt`), the trips' stop times worked out. The
/// map's folders are found in the content folder first, then in the OMSI 2 folder.
pub fn lines_for(vfs: &Vfs, map: &str, date: NaiveDate) -> Result<Vec<ServiceLine>, ReadError> {
    let global = vfs.resolve(map).filter(|p| p.is_file()).ok_or_else(|| ReadError::NotFound(map.to_string()))?;
    let rel = map.replace('\\', "/");
    let dir_rel = rel.rsplit_once('/').map_or("", |(d, _)| d);
    let in_map = |name: &str| vfs.resolve(&format!("{dir_rel}/{name}"));
    let map_dir = global.parent().map(Path::to_path_buf).unwrap_or_default();
    let chrono_base = in_map("Chrono").and_then(|c| c.parent().map(Path::to_path_buf)).unwrap_or_else(|| map_dir.clone());
    let chrono = crate::map::active_chrono_dirs(&chrono_base, date);
    let off = crate::map::chrono_deactivated_lines(&chrono);
    let Some(ttdata) = in_map("TTData").filter(|p| p.is_dir()) else { return Ok(Vec::new()) };
    let tt = Timetable::load_with_chrono(&ttdata, &chrono, &off);
    let cal = match in_map("Holidays.txt") {
        Some(p) => Calendar::parse(&CfgFile::read(p)?),
        None => Calendar::default(),
    };
    Ok(service_lines(&tt, &cal, date))
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
    fn day_masks_run_by_the_calendar() {
        use crate::map::{Holiday, HolidayRange};
        let d = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let cal = Calendar {
            holidays: vec![Holiday { date: d("1989-05-04"), name: "Ascension".into() }],
            ranges: vec![HolidayRange { start: d("1989-07-01"), end: d("1989-08-15"), name: "Summer".into() }],
        };
        // 1989-05-30 is a Tuesday, 1989-05-04 a Thursday and a public holiday, 1989-07-04 a
        // Tuesday in the school holidays
        let (tue, holiday, summer) = (d("1989-05-30"), d("1989-05-04"), d("1989-07-04"));
        let workdays = DayMask(799); // Mon-Fri, school days and holidays
        assert!(workdays.runs_on(&cal, tue) && workdays.runs_on(&cal, summer) && !workdays.runs_on(&cal, holiday));
        let school = DayMask(287); // Mon-Fri, school days only
        assert!(school.runs_on(&cal, tue) && !school.runs_on(&cal, summer));
        let sundays = DayMask(960); // Sun, holidays, both school bits
        assert!(sundays.runs_on(&cal, holiday) && !sundays.runs_on(&cal, tue));
        // (an old mask without school bits: the school calendar does not matter)
        assert!(DayMask(31).runs_on(&cal, summer));
        assert!(DayMask::ALWAYS.runs_on(&cal, holiday));
    }

    #[test]
    fn text_is_written_with_crlf_in_its_codepage() {
        let p = std::env::temp_dir().join(format!("hub-omsi-write-{}.ttl", std::process::id()));
        write_text(&p, "[name]\nGrünau\n", Codepage::Windows1252).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"[name]\r\nGr\xFCnau\r\n");
        // (a UTF-8 file with a BOM keeps it)
        std::fs::write(&p, b"\xEF\xBB\xBFold").unwrap();
        write_text(&p, "Grünau", Codepage::Utf8).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), "\u{feff}Grünau".as_bytes());
        write_text(&p, "a\n", Codepage::Utf16Le).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"\xFF\xFEa\0\r\0\n\0");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn numbers_are_written_as_omsi_writes_them() {
        assert_eq!((num(99.0), num(-1.0), num(2.5)), ("99".to_string(), "-1".to_string(), "2.5".to_string()));
        assert_eq!((dec(55.0, 10), dec(1.0 / 3.0, 10)), ("55.0000000000".to_string(), format!("{}", 1.0 / 3.0)));
        assert_eq!((dec32(247.0, 3), dec32(0.1234, 3)), ("247.000".to_string(), "0.1234".to_string()));
        assert_eq!(parse_i64(" 200075 "), 200_075);
    }
}
