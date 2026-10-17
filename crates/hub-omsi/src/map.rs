//! A map's own files: `global.cfg` ([`GlobalCfg`]), `timezone.txt` ([`TimeZone`]),
//! `ailists.cfg` ([`AiLists`]), `Holidays.txt` ([`Calendar`]) and its `Chrono` folders
//! ([`ChronoDir`]).
//!
//! Dates are [`chrono::NaiveDate`]; [`date_code`] gives the `YYYYMMDD` number OMSI's files
//! compare.
//!
//! * `global.cfg` (UTF-16 in OMSI's own maps): `[name]`, `[friendlyname]`, `[description]` …
//!   `[end]`, `[entrypoints]` (a count, then twelve lines each: two ids, the place, the
//!   rotation, the tile, the name), `[map]` (the tiles: x, y, file; a track's tile index points
//!   into this list), `[LHT]` (left-hand traffic) and `[standarddepot]` (the AI depot group the
//!   player's bus takes its depot file from).
//! * `timezone.txt` beside it: `[timezone]` (hours from UTC), `[location]` (latitude,
//!   longitude), `[DST]` (start and end date, the hours of the switch, the shift).
//! * `Holidays.txt`: `[holiday]` (a date, its name) and `[holidays]` (a stretch of school
//!   holidays: first and last day, its name).
//! * `Chrono/<folder>/Chrono.cfg`: `[startdate]`, `[enddate]` (`YYYYMMDD`), `[deactivate_lines]`
//!   (a count, then the lines' names), `[name]`. The folders active on a day apply in the order
//!   of their names; their files (`TTData/…`, `ailists.cfg`) take the place of the map's.
//! * `ailists.cfg`: `[aigroup_2]` (name, depot file, then the vehicles - `file<TAB>weight` - up to
//!   `[end]`), `[aigroup_depot]` (name, depot file) with its `[aigroup_depot_typgroup_2]` (a
//!   bus file, then its buses - `number<TAB>plate<TAB>paint<TAB>from<TAB>until` - up to
//!   `[end]`). A Chrono folder's `ailists.cfg` replaces the map's; its `ailists_#upd.cfg` replaces
//!   the groups of the same name and adds the others.

use crate::text::{parse_f64, parse_i32, CfgFile, Reader};
use crate::vfs::{find_child, normalize, same_name};
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

/// The date of a `YYYYMMDD` value as OMSI's files write it (None: not one).
pub fn date_of_code(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    if t.len() != 8 || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: i32 = t.parse().ok()?;
    NaiveDate::from_ymd_opt(n / 10_000, (n / 100 % 100) as u32, (n % 100) as u32)
}

/// A `YYYYMMDD` value as a code, when it is a date.
fn code_value(s: &str) -> Option<i32> {
    date_of_code(s).map(code_of)
}

/// An entry point of the map (`[entrypoints]`): where a bus can be put.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryPoint {
    /// Its place in the list (0 = the first).
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
    /// The map drives on the left (`[LHT]`).
    pub left_hand_traffic: bool,
    /// `[standarddepot]`: the AI depot group (`ailists.cfg`) whose depot file the player's bus
    /// is given.
    pub standard_depot: Option<String>,
    /// The map folder's `timezone.txt`, when it has one (read by [`GlobalCfg::load`]).
    pub timezone: Option<TimeZone>,
}

impl GlobalCfg {
    pub fn parse(cfg: &CfgFile) -> GlobalCfg {
        let mut g = GlobalCfg { path: cfg.path.clone(), ..GlobalCfg::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k {
                "name" => g.name = r.value().trim().to_string(),
                "friendlyname" => g.friendly_name = r.value().trim().to_string(),
                "description" => g.description = r.until("end").join("\n").trim().to_string(),
                "LHT" => g.left_hand_traffic = true,
                "standarddepot" => g.standard_depot = Some(r.value().trim().to_string()).filter(|s| !s.is_empty()),
                "map" => {
                    let v = r.values(2);
                    if v.len() == 2 {
                        g.tiles.push((parse_i32(v[0]), parse_i32(v[1])));
                    }
                }
                "entrypoints" => {
                    let n = parse_i32(r.value()).max(0) as usize;
                    for i in 0..n {
                        let v = r.values(12);
                        if v.len() < 12 {
                            break;
                        }
                        g.entry_points.push(EntryPoint { index: i as i32, name: v[11].trim().to_string() });
                    }
                }
                _ => {}
            }
        }
        g
    }

    /// The file, and the `timezone.txt` beside it.
    pub fn load(path: &Path) -> Result<GlobalCfg, ReadError> {
        let mut g = GlobalCfg::parse(&CfgFile::read(path)?);
        g.timezone = path.parent().and_then(|d| find_child(d, "timezone.txt")).and_then(|p| TimeZone::load(&p).ok());
        Ok(g)
    }
}

/// A summer time period of `timezone.txt` (`[DST]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Dst {
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    /// The hours the clocks change at, and by how many hours.
    pub start_hour: f32,
    pub end_hour: f32,
    pub shift: f32,
}

/// A map's `timezone.txt`: its time zone and place.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeZone {
    /// Hours from UTC (`[timezone]`).
    pub offset_hours: f32,
    /// `[location]`: latitude, longitude.
    pub location: Option<(f64, f64)>,
    pub dst: Vec<Dst>,
}

impl TimeZone {
    pub fn parse(cfg: &CfgFile) -> TimeZone {
        let mut t = TimeZone::default();
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                "timezone" => t.offset_hours = parse_f64(r.value()) as f32,
                "location" => {
                    let v = r.values(2);
                    if v.len() == 2 && !v[0].trim().is_empty() {
                        t.location = Some((parse_f64(v[0]), parse_f64(v[1])));
                    }
                }
                "dst" => {
                    let v = r.values(5);
                    let f = |i: usize| v.get(i).map_or(0.0, |s| parse_f64(s) as f32);
                    t.dst.push(Dst { start: v.first().and_then(|s| date_of_code(s)), end: v.get(1).and_then(|s| date_of_code(s)), start_hour: f(2), end_hour: f(3), shift: f(4) });
                }
                _ => {}
            }
        }
        t
    }

    pub fn load(path: &Path) -> Result<TimeZone, ReadError> {
        Ok(TimeZone::parse(&CfgFile::read(path)?))
    }

    /// Its place: latitude, longitude.
    pub fn lat_lon(&self) -> Option<(f64, f64)> {
        self.location
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
        let mut c = Calendar::default();
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k {
                "holiday" => {
                    let v = r.values(2);
                    if let Some(date) = v.first().and_then(|s| date_of_code(s)) {
                        c.holidays.push(Holiday { date, name: v.get(1).map_or("", |s| s.trim()).to_string() });
                    }
                }
                "holidays" => {
                    let v = r.values(3);
                    if let (Some(start), Some(end)) = (v.first().and_then(|s| date_of_code(s)), v.get(1).and_then(|s| date_of_code(s))) {
                        c.ranges.push(HolidayRange { start, end, name: v.get(2).map_or("", |s| s.trim()).to_string() });
                    }
                }
                _ => {}
            }
        }
        c
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
    /// The folder's name.
    pub name: String,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// The lines it switches off (`[deactivate_lines]`).
    pub deactivate_lines: Vec<String>,
    /// Its `[name]`, where `Chrono.cfg` gives one.
    pub title: String,
}

impl ChronoDir {
    /// The folder `dir`, from its `Chrono.cfg` (None: it has none).
    pub fn read(dir: &Path) -> Option<ChronoDir> {
        let cfg = CfgFile::read(find_child(dir, "Chrono.cfg")?).ok()?;
        let mut c = ChronoDir { path: dir.to_path_buf(), name: dir.file_name()?.to_string_lossy().into_owned(), ..ChronoDir::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                "startdate" => c.from = date_of_code(r.value()),
                "enddate" => c.to = date_of_code(r.value()),
                "name" => c.title = r.value().trim().to_string(),
                "deactivate_lines" => {
                    let n = parse_i32(r.value()).max(0) as usize;
                    c.deactivate_lines.extend(r.values(n).into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
                }
                _ => {}
            }
        }
        Some(c)
    }

    /// It applies on `date`.
    pub fn active_on(&self, date: NaiveDate) -> bool {
        self.from.is_none_or(|f| f <= date) && self.to.is_none_or(|t| date <= t)
    }
}

/// The map folder's Chrono folders active on `date`, in the order they apply (by their names).
pub fn active_chrono_dirs(map_dir: &Path, date: NaiveDate) -> Vec<ChronoDir> {
    let Some(chrono) = find_child(map_dir, "Chrono").filter(|p| p.is_dir()) else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(&chrono) else { return Vec::new() };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    dirs.iter().filter_map(|d| ChronoDir::read(d)).filter(|c| c.active_on(date)).collect()
}

/// The lines the active Chrono folders switch off (each once, as the first names it).
pub fn chrono_deactivated_lines(chrono: &[ChronoDir]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in chrono.iter().flat_map(|c| &c.deactivate_lines) {
        if !out.iter().any(|o| same_name(o, l)) {
            out.push(l.clone());
        }
    }
    out
}

/// A livery and number rule of a type group.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypGroupEntry {
    /// From and until which date (`YYYYMMDD`) it holds.
    pub from: Option<i32>,
    pub to: Option<i32>,
    pub paint: String,
    pub number: String,
    /// The plate it has, where the list gives one (else the bus's own rule makes it).
    pub plate: String,
}

impl TypGroupEntry {
    /// It holds on the day `code` (`YYYYMMDD`).
    pub fn valid(&self, code: i32) -> bool {
        self.from.is_none_or(|f| f <= code) && self.to.is_none_or(|t| code <= t)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypGroup {
    /// The bus file, relative to the OMSI 2 folder, with `/`.
    pub file: String,
    pub entries: Vec<TypGroupEntry>,
}

/// A bus of an AI group, and how often it is taken.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiVehicle {
    /// The vehicle's file, relative to the OMSI 2 folder, with `/`.
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
    /// The group's depot file's `[name]` (None: it names none).
    pub hof: Option<String>,
    pub typgroups: Vec<TypGroup>,
    pub vehicles: Vec<AiVehicle>,
}

/// A map's `ailists.cfg`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiLists {
    pub groups: Vec<AiGroup>,
}

/// The lines of a list up to `[end]`, the blank ones left out.
fn list_lines<'a>(r: &mut Reader<'a>) -> Vec<&'a str> {
    r.until("end").into_iter().filter(|l| !l.trim().is_empty()).collect()
}

impl AiLists {
    pub fn parse(cfg: &CfgFile) -> AiLists {
        let mut out = AiLists::default();
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                kw @ ("aigroup_2" | "aigroup" | "aigroup_depot") => {
                    let v = r.values(2);
                    let mut g = AiGroup {
                        name: v.first().map_or("", |s| s.trim()).to_string(),
                        is_depot: kw == "aigroup_depot",
                        hof: v.get(1).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                        ..AiGroup::default()
                    };
                    if !g.is_depot {
                        for l in list_lines(&mut r) {
                            let f: Vec<&str> = l.split('\t').map(str::trim).filter(|s| !s.is_empty()).collect();
                            let Some(file) = f.first() else { continue };
                            let weight = f.get(1).map_or(1.0, |w| parse_f64(w) as f32);
                            g.vehicles.push(AiVehicle { file: normalize(file), weight, number: f.get(2).map(|s| s.to_string()) });
                        }
                    }
                    out.groups.push(g);
                }
                "aigroup_depot_typgroup_2" | "aigroup_depot_typgroup" => {
                    let file = normalize(r.value());
                    let mut t = TypGroup { file, entries: Vec::new() };
                    for l in list_lines(&mut r) {
                        let f: Vec<&str> = l.split('\t').collect();
                        let col = |i: usize| f.get(i).map_or("", |s| s.trim());
                        if col(0).is_empty() {
                            continue;
                        }
                        t.entries.push(TypGroupEntry { number: col(0).to_string(), plate: col(1).to_string(), paint: col(2).to_string(), from: code_value(col(3)), to: code_value(col(4)) });
                    }
                    match out.groups.iter_mut().rev().find(|g| g.is_depot) {
                        Some(g) => g.typgroups.push(t),
                        None => out.groups.push(AiGroup { is_depot: true, typgroups: vec![t], ..AiGroup::default() }),
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn load(path: &Path) -> Result<AiLists, ReadError> {
        Ok(AiLists::parse(&CfgFile::read(path)?))
    }

    /// The groups of `update` in place of those of the same name, the others added.
    pub fn update(&mut self, update: AiLists) {
        for g in update.groups {
            match self.groups.iter_mut().find(|o| o.name.trim().eq_ignore_ascii_case(g.name.trim())) {
                Some(o) => *o = g,
                None => self.groups.push(g),
            }
        }
    }

    /// The map folder's `ailists.cfg`, a Chrono folder's copy winning (its `ailists.cfg`
    /// replaces it, its `ailists_#upd.cfg` updates it).
    pub fn with_chrono(map_dir: &Path, chrono: &[ChronoDir]) -> AiLists {
        let mut lists = find_child(map_dir, "ailists.cfg").and_then(|p| AiLists::load(&p).ok()).unwrap_or_default();
        for c in chrono {
            if let Some(full) = find_child(&c.path, "ailists.cfg").and_then(|p| AiLists::load(&p).ok()) {
                lists = full;
            }
            if let Some(upd) = find_child(&c.path, "ailists_#upd.cfg").and_then(|p| AiLists::load(&p).ok()) {
                lists.update(upd);
            }
        }
        lists
    }

    pub fn group(&self, name: &str) -> Option<&AiGroup> {
        self.groups.iter().find(|g| g.name.eq_ignore_ascii_case(name.trim()))
    }

    /// The depot group the player's bus takes its depot file from: the `[standarddepot]` one
    /// when it is a depot group, else the first depot group.
    pub fn depot_group(&self, standard: Option<&str>) -> Option<&AiGroup> {
        let depots = || self.groups.iter().filter(|g| g.is_depot);
        standard.and_then(|s| depots().find(|g| g.name.trim().eq_ignore_ascii_case(s.trim()))).or_else(|| depots().next())
    }
}

/// The depot file (its `[name]`) the map's depot group gives buses on `date`: the
/// `[standarddepot]` group's, else the first depot group's, with the day's Chrono applied.
pub fn depot_hof_on(map_dir: &Path, date: NaiveDate) -> Option<String> {
    let chrono = active_chrono_dirs(map_dir, date);
    let lists = AiLists::with_chrono(map_dir, &chrono);
    let standard = find_child(map_dir, "global.cfg").and_then(|p| CfgFile::read(p).ok()).and_then(|c| c.value("standarddepot").map(str::to_string));
    lists.depot_group(standard.as_deref()).and_then(|g| g.hof.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn dates_are_codes_and_ranges() {
        assert_eq!(date_code("1989-05-30"), Some(19_890_530));
        assert_eq!(date_code("30.05.1989"), None);
        assert_eq!(date_of_code(" 19890530 "), Some(d("1989-05-30")));
        assert_eq!((date_of_code("19890230"), date_of_code("0"), date_of_code("")), (None, None, None));
        let cal =
            Calendar { holidays: vec![Holiday { date: d("1989-12-25"), name: "Xmas".into() }], ranges: vec![HolidayRange { start: d("1989-07-01"), end: d("1989-08-15"), name: "Summer".into() }] };
        assert!(cal.is_holiday(d("1989-12-25")) && !cal.is_holiday(d("1989-12-24")));
        assert!(cal.in_holiday_range(d("1989-08-15")) && !cal.in_holiday_range(d("1989-08-16")));
        let e = TypGroupEntry { from: Some(19_900_101), to: None, ..TypGroupEntry::default() };
        assert!(!e.valid(19_891_231) && e.valid(19_900_101));
    }

    #[test]
    fn the_calendar_is_read() {
        let cal = Calendar::parse(&CfgFile::from_str(
            "Holidays.txt",
            "----1989----\r\n\r\n[holidays]\r\n19890701\r\n19890815\r\nSummer\r\n\r\n[holiday]\r\n19891225\r\nChristmas\r\n[holiday]\r\nbad\r\nx\r\n",
        ));
        assert_eq!(cal.ranges, [HolidayRange { start: d("1989-07-01"), end: d("1989-08-15"), name: "Summer".into() }]);
        assert_eq!(cal.holidays, [Holiday { date: d("1989-12-25"), name: "Christmas".into() }]);
    }

    #[test]
    fn global_cfg_has_its_tiles_entry_points_and_traffic_side() {
        let text = "File created with OMSI-Editor\r\n\r\n[name]\r\nTestdorf\r\n\r\n[friendlyname]\r\nTestdorf (demo)\r\n\r\n[description]\r\nA village.\r\nTwo lines.\r\n[end]\r\n\r\n[LHT]\r\n[standarddepot]\r\nBusses\r\n\r\n[entrypoints]\r\n2\r\n4\r\n103\r\n0\r\n1.0\r\n2.0\r\n3.0\r\n0\r\n0\r\n0\r\n1\r\n0\r\nDepot\r\n5\r\n104\r\n0\r\n1.0\r\n2.0\r\n3.0\r\n0\r\n0\r\n0\r\n1\r\n1\r\nMarket Square\r\n\r\n[map]\r\n0\r\n0\r\ntile_0_0.map\r\n\r\n[map]\r\n-1\r\n2\r\ntile_-1_2.map\r\n";
        let g = GlobalCfg::parse(&CfgFile::from_str("global.cfg", text));
        assert_eq!((g.name.as_str(), g.friendly_name.as_str(), g.description.as_str()), ("Testdorf", "Testdorf (demo)", "A village.\nTwo lines."));
        assert_eq!(g.tiles, [(0, 0), (-1, 2)]);
        assert!(g.left_hand_traffic);
        assert_eq!(g.standard_depot.as_deref(), Some("Busses"));
        assert_eq!(g.entry_points, [EntryPoint { index: 0, name: "Depot".into() }, EntryPoint { index: 1, name: "Market Square".into() }]);
        let tz = TimeZone::parse(&CfgFile::from_str("timezone.txt", "[timezone]\r\n1\r\n\r\n[location]\r\n52.5\r\n13.25\r\n\r\n[DST]\r\n19890326\r\n19890924\r\n2\r\n3\r\n1\r\n"));
        assert_eq!((tz.offset_hours, tz.lat_lon()), (1.0, Some((52.5, 13.25))));
        assert_eq!(tz.dst, [Dst { start: Some(d("1989-03-26")), end: Some(d("1989-09-24")), start_hour: 2.0, end_hour: 3.0, shift: 1.0 }]);
    }

    #[test]
    fn ai_lists_have_their_groups_depots_and_typgroups() {
        let text = "[aigroup_2]\r\nNormalCars\r\n\r\nvehicles\\Car\\car.bus\t70\r\nvehicles\\Van\\van.ovh\t\t10\r\nvehicles\\Lone\\lone.bus\r\n[end]\r\n\r\n****\r\n[aigroup_depot]\r\nBusses\r\nTestdorf\r\n\r\n[aigroup_depot_typgroup_2]\r\nVehicles\\Bus\\bus.bus\r\n2530\tTD-V 30\r\n2701\t\tRed\t\t19870430\r\n2708\t\tBlue\t19870201\t\r\n[end]\r\n\r\n[aigroup_2]\r\nCoaches\r\nTestdorf Coaches\r\nvehicles\\Coach\\coach.bus\t1\t17\r\n[end]\r\n";
        let a = AiLists::parse(&CfgFile::from_str("ailists.cfg", text));
        assert_eq!(a.groups.len(), 3);
        let cars = a.group("normalcars").unwrap();
        assert_eq!((cars.is_depot, cars.hof.clone()), (false, None));
        assert_eq!(cars.vehicles.iter().map(|v| (v.file.as_str(), v.weight)).collect::<Vec<_>>(), [("vehicles/Car/car.bus", 70.0), ("vehicles/Van/van.ovh", 10.0), ("vehicles/Lone/lone.bus", 1.0)]);
        let depot = a.group("Busses").unwrap();
        assert_eq!((depot.is_depot, depot.hof.as_deref()), (true, Some("Testdorf")));
        let t = &depot.typgroups[0];
        assert_eq!(t.file, "Vehicles/Bus/bus.bus");
        assert_eq!(t.entries[0], TypGroupEntry { number: "2530".into(), plate: "TD-V 30".into(), ..TypGroupEntry::default() });
        assert_eq!(t.entries[1], TypGroupEntry { number: "2701".into(), paint: "Red".into(), to: Some(19_870_430), ..TypGroupEntry::default() });
        assert_eq!(t.entries[2], TypGroupEntry { number: "2708".into(), paint: "Blue".into(), from: Some(19_870_201), ..TypGroupEntry::default() });
        let coach = a.group("Coaches").unwrap();
        assert_eq!((coach.hof.as_deref(), coach.vehicles[0].number.as_deref()), (Some("Testdorf Coaches"), Some("17")));
        assert_eq!(a.depot_group(Some("busses")).map(|g| g.name.as_str()), Some("Busses"));
        assert_eq!(a.depot_group(Some("nothing")).map(|g| g.name.as_str()), Some("Busses"));
        let mut b = a.clone();
        b.update(AiLists { groups: vec![AiGroup { name: "BUSSES".into(), is_depot: true, hof: Some("New".into()), ..AiGroup::default() }, AiGroup { name: "Extra".into(), ..AiGroup::default() }] });
        assert_eq!((b.groups.len(), b.group("Busses").unwrap().hof.as_deref()), (4, Some("New")));
    }
}
