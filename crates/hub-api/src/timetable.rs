//! The timetable editor (the old launcher's Timetable page): a map's lines, their tours and the
//! trips each tour runs, read from the map's `TTData` as the game reads it (the content folder's
//! copy first, `hub_omsi::vfs`) with `hub_omsi::timetable`'s readers, and written back where
//! `hub_core::ttstore` says - the content folder's copy of the `TTData`, or the map's own folder
//! when the map lies unpacked in the content folder - never into the OMSI 2 folder.
//!
//! * `tt_load {map}` - `{ dir, folder, lines, trips }`: the folder the map's `TTData` lies in,
//!   the map's folder name, every line (`.ttl`) with its tours and their trips, and every trip
//!   (`.ttp`) a tour can run (`{ name, line, terminus, profiles }`);
//! * `tt_edit {line, tour, op, first, offset, until}` - a change of one line's tours, made here
//!   and not saved: `{ line, tour, made }` (`op`: `new_tour`, `copy`, `repeat`, `delete_tour`,
//!   `add_trip`, `shift`);
//! * `tt_save {map, lines}` - the lines written: `{ saved, error, lines: [{ name, path, line }] }`
//!   (the first error; the lines saved with where they lie now);
//! * `tt_reset {map}` - the map's own timetable back (`ttstore::reset_timetable`), the player's
//!   own lines of the map written into it again: `{ done, own, own_error }`.
//!
//! Departures are minutes of the day, as the `.ttl` has them (`6:15` is 375.0).

use crate::content::{map_dir_rel, need_vfs, str_arg};
use crate::{arg, to_json, Answer, Platform};
use hub_core::{lines, ttstore};
use hub_omsi::timetable::{DayMask, Line, Timetable, Tour, TourTrip, Trip};
use hub_omsi::{Codepage, Vfs};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A day of minutes: a departure shifted past midnight starts the day again.
const DAY: f64 = 1440.0;
/// How many tours `repeat` makes at most.
const MAX_REPEAT: usize = 500;

/// A line as the page has it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TtLine {
    /// Where it was read (a new line: where it is to go, `<dir>/TTData/<name>.ttl`).
    pub path: String,
    pub name: String,
    pub user_allowed: bool,
    pub priority: i32,
    /// One of the player's own lines (`oo_…`, written by the line editor).
    pub own: bool,
    pub tours: Vec<TtTour>,
}

/// A tour of a line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TtTour {
    pub number: String,
    pub ai_group: String,
    /// Its day mask (`DayMask`'s bits).
    pub days: u32,
    pub trips: Vec<TtTrip>,
}

impl Default for TtTour {
    fn default() -> TtTour {
        TtTour { number: String::new(), ai_group: String::new(), days: DayMask::ALWAYS.0, trips: Vec::new() }
    }
}

/// A trip of a tour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TtTrip {
    pub trip: String,
    pub profile: i32,
    /// Minutes of the day.
    pub departure: f64,
}

/// A trip a tour can run, as the page's lists show it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TripRow {
    pub name: String,
    pub line: String,
    pub terminus: String,
    /// Its profiles' names, by number.
    pub profiles: Vec<String>,
}

impl TtLine {
    fn of(l: &Line) -> TtLine {
        TtLine {
            path: l.path.to_string_lossy().into_owned(),
            name: l.name.clone(),
            user_allowed: l.user_allowed,
            priority: l.priority,
            own: lines::is_own_file(&l.name),
            tours: l
                .tours
                .iter()
                .map(|t| TtTour {
                    number: t.number.clone(),
                    ai_group: t.ai_group.clone(),
                    days: t.days.0,
                    trips: t.trips.iter().map(|x| TtTrip { trip: x.trip.clone(), profile: x.profile, departure: x.departure as f64 }).collect(),
                })
                .collect(),
        }
    }

    /// The line as `hub_omsi` writes it, at `path`, in `codepage`.
    fn to_line(&self, path: PathBuf, codepage: Codepage) -> Line {
        Line {
            path,
            name: self.name.clone(),
            user_allowed: self.user_allowed,
            priority: self.priority,
            tours: self
                .tours
                .iter()
                .map(|t| Tour {
                    number: t.number.trim().to_string(),
                    ai_group: t.ai_group.trim().to_string(),
                    days: DayMask(t.days),
                    trips: t.trips.iter().map(|x| TourTrip { trip: x.trip.trim().to_string(), profile: x.profile.max(0), departure: x.departure as f32 }).collect(),
                })
                .collect(),
            codepage: Some(codepage),
        }
    }
}

/// A `tt_` command.
pub fn call(cmd: &str, args: &Value, _p: &dyn Platform) -> Answer {
    match cmd {
        "tt_load" => load(&need_vfs()?, &arg::<String>(args, "map")?),
        "tt_edit" => {
            let line: TtLine = arg(args, "line")?;
            let tour = arg::<Option<usize>>(args, "tour")?.unwrap_or(0);
            let (line, tour, made) = edit(line, tour, &str_arg(args, "op"), args)?;
            Ok(json!({ "line": line, "tour": tour, "made": made }))
        }
        "tt_save" => {
            let content = hub_core::paths::content_dir().ok_or("openOMSI's content folder is not there: choose the openOMSI folder in the Settings")?;
            save(&content, &need_vfs()?, &arg::<String>(args, "map")?, &arg::<Vec<TtLine>>(args, "lines")?)
        }
        "tt_reset" => {
            let content = hub_core::paths::content_dir().ok_or("openOMSI's content folder is not there: choose the openOMSI folder in the Settings")?;
            reset(&content, &need_vfs()?, &arg::<String>(args, "map")?)
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// The map's folder (`maps/<folder>`, relative) and its name.
fn map_parts(map: &str) -> (String, String) {
    let rel = map_dir_rel(map);
    let folder = rel.rsplit('/').next().unwrap_or_default().to_string();
    (rel, folder)
}

/// The folder the map's `global.cfg` lies in.
fn global_dir(vfs: &Vfs, map: &str) -> Result<PathBuf, String> {
    vfs.resolve(map).filter(|p| p.is_file()).and_then(|p| p.parent().map(Path::to_path_buf)).ok_or_else(|| format!("The map {map} is not there"))
}

/// `tt_load`: the map's timetable as the page edits it.
pub fn load(vfs: &Vfs, map: &str) -> Answer {
    let (rel, folder) = map_parts(map);
    let map_dir = global_dir(vfs, map)?;
    let ttdata = vfs.resolve(&format!("{rel}/TTData")).filter(|p| p.is_dir());
    let dir = ttdata.as_ref().and_then(|t| t.parent().map(Path::to_path_buf)).unwrap_or(map_dir);
    let tt = ttdata.map(|t| Timetable::load(&t)).unwrap_or_default();
    let by_name = |a: &str, b: &str| a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b));
    let mut lines: Vec<TtLine> = tt.lines.iter().map(TtLine::of).collect();
    lines.sort_by(|a, b| by_name(&a.name, &b.name));
    let mut trips: Vec<TripRow> = tt
        .trips
        .iter()
        .map(|t| TripRow {
            name: t.name.clone(),
            line: t.line.trim().to_string(),
            terminus: t.terminus.trim().to_string(),
            profiles: t.profiles.iter().enumerate().map(|(k, p)| if p.name.trim().is_empty() { k.to_string() } else { p.name.trim().to_string() }).collect(),
        })
        .collect();
    trips.sort_by(|a, b| by_name(&a.name, &b.name));
    to_json(json!({ "dir": dir.to_string_lossy(), "folder": folder, "lines": lines, "trips": trips }))
}

// --- the tours' changes ----------------------------------------------------------------------

/// A minutes argument (a number, or a number in a text).
fn minutes(args: &Value, name: &str) -> Option<f64> {
    match args.get(name)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().replace(',', ".").parse().ok(),
        _ => None,
    }
    .filter(|m: &f64| m.is_finite())
}

/// A departure in the day again (`24:10` is `0:10`), to the second.
fn in_day(m: f64) -> f64 {
    ((m * 60.0).round() / 60.0).rem_euclid(DAY)
}

/// A tour number none of the line's tours has: one above the highest.
fn free_number(line: &TtLine) -> String {
    let top = line.tours.iter().filter_map(|t| t.number.trim().parse::<u64>().ok()).max().unwrap_or(0);
    (top + 1..).map(|n| n.to_string()).find(|n| !line.tours.iter().any(|t| t.number.trim() == n)).unwrap_or_default()
}

/// `tour` copied `by` minutes later, as a tour of its own.
fn shifted(tour: &TtTour, by: f64, number: String) -> TtTour {
    TtTour { number, trips: tour.trips.iter().map(|x| TtTrip { departure: in_day(x.departure + by), ..x.clone() }).collect(), ..tour.clone() }
}

/// How long the trip `name` takes with `profile`, from its `.ttp` beside the line's file
/// (`line_path`).
fn trip_minutes(line_path: &str, name: &str, profile: i32) -> Option<f64> {
    let dir = Path::new(line_path).parent()?;
    let file = hub_omsi::vfs::find_child(dir, &format!("{}.ttp", name.trim()))?;
    let trip = Trip::load(&file).ok()?;
    let p = trip.profiles.get(profile.max(0) as usize).or(trip.profiles.first())?;
    Some(p.minutes as f64).filter(|m| *m > 0.0)
}

/// The AI group of the line's tours, else of another line beside it: what a new tour gets.
fn ai_group_near(line: &TtLine) -> String {
    if let Some(t) = line.tours.iter().find(|t| !t.ai_group.trim().is_empty()) {
        return t.ai_group.clone();
    }
    let Some(dir) = Path::new(&line.path).parent() else { return String::new() };
    hub_core::paths::read_dir_paths(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ttl")))
        .filter_map(|p| Line::load(&p).ok())
        .find_map(|l| l.tours.into_iter().map(|t| t.ai_group).find(|g| !g.trim().is_empty()))
        .unwrap_or_default()
}

/// A change `op` of the tours of `line`, its tour `tour` chosen: the line as it is then, the
/// tour to show, and how many tours were made.
pub fn edit(mut line: TtLine, tour: usize, op: &str, args: &Value) -> Result<(TtLine, usize, usize), String> {
    let n = line.tours.len();
    let at = tour.min(n.saturating_sub(1));
    let first = str_arg(args, "first");
    let offset = minutes(args, "offset");
    let need = |t: Option<&TtTour>| t.cloned().ok_or_else(|| "The line has no tour yet: make one with New tour".to_string());
    match op {
        "new_tour" => {
            let start = line.tours.iter().filter_map(|t| t.trips.first()).map(|x| x.departure).fold(None, |a: Option<f64>, d| Some(a.map_or(d, |a| a.max(d))));
            let days = line.tours.get(at).map_or(DayMask::ALWAYS.0, |t| t.days);
            let trips = if first.trim().is_empty() { Vec::new() } else { vec![TtTrip { trip: first.trim().to_string(), profile: 0, departure: start.map_or(360.0, |s| in_day(s + 60.0)) }] };
            let number = free_number(&line);
            line.tours.push(TtTour { number, ai_group: ai_group_near(&line), days, trips });
            Ok((line.clone(), line.tours.len() - 1, 1))
        }
        "copy" => {
            let t = need(line.tours.get(at))?;
            let by = offset.ok_or("Type the minutes the copy runs later (negative for earlier).")?;
            let number = free_number(&line);
            line.tours.push(shifted(&t, by, number));
            Ok((line.clone(), line.tours.len() - 1, 1))
        }
        "repeat" => {
            let t = need(line.tours.get(at))?;
            let every = offset.filter(|o| *o >= 1.0).ok_or("Type the minutes between the tours (1 or more) in the field above")?;
            let until = minutes(args, "until").ok_or("Type the last departure as h:mm")?;
            let start = t.trips.first().map_or(0.0, |x| x.departure);
            let mut made = 0;
            while made < MAX_REPEAT && start + every * (made + 1) as f64 <= until + 1e-6 {
                made += 1;
                let number = free_number(&line);
                line.tours.push(shifted(&t, every * made as f64, number));
            }
            Ok((line, at, made))
        }
        "delete_tour" => {
            if at < n {
                line.tours.remove(at);
            }
            Ok((line.clone(), at.min(line.tours.len().saturating_sub(1)), 0))
        }
        "add_trip" => {
            let t = line.tours.get(at).ok_or("The line has no tour yet: make one with New tour")?;
            let k = t.trips.len();
            // (A, B, A, B …: the trip before the last again; a tour of one trip gets the one asked)
            let name = if k >= 2 {
                t.trips[k - 2].trip.clone()
            } else if !first.trim().is_empty() {
                first.trim().to_string()
            } else {
                t.trips.last().map(|x| x.trip.clone()).unwrap_or_default()
            };
            // (after the last trip has arrived: its profile's time, else half an hour)
            let departure = t.trips.last().map_or(360.0, |x| in_day(x.departure + trip_minutes(&line.path, &x.trip, x.profile).unwrap_or(30.0)));
            line.tours[at].trips.push(TtTrip { trip: name, profile: 0, departure });
            Ok((line, at, 0))
        }
        "shift" => {
            let by = offset.ok_or("Type the minutes to shift by")?;
            let t = line.tours.get_mut(at).ok_or("The line has no tour yet: make one with New tour")?;
            for x in &mut t.trips {
                x.departure = in_day(x.departure + by);
            }
            Ok((line, at, 0))
        }
        _ => Err(format!("unknown change of the timetable: {op}")),
    }
}

// --- saving ----------------------------------------------------------------------------------

/// A line's name that can be a file name.
fn name_ok(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty() && n != "." && n != ".." && !n.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control())
}

/// `tt_save`: each line written where `ttstore` says (the content folder `content`), in the
/// code page of the file it replaces.
pub fn save(content: &Path, vfs: &Vfs, map: &str, lines: &[TtLine]) -> Answer {
    let (_, folder) = map_parts(map);
    let original = hub_core::paths::resolve_path(&global_dir(vfs, map)?, "TTData");
    let mut saved = Vec::new();
    let mut error: Option<String> = None;
    for l in lines {
        match save_line(content, &folder, &original, l) {
            Ok(path) => {
                let line = TtLine { path: path.to_string_lossy().into_owned(), ..l.clone() };
                saved.push(json!({ "name": l.name, "path": line.path, "line": line }));
            }
            Err(e) => {
                error.get_or_insert(format!("{}: {e}", l.name));
            }
        }
    }
    Ok(json!({ "saved": saved.len(), "error": error, "lines": saved }))
}

fn save_line(content: &Path, folder: &str, original: &Path, l: &TtLine) -> Result<PathBuf, String> {
    if !name_ok(&l.name) {
        return Err("a line's name becomes its file name: no / \\ : * ? \" < > |".to_string());
    }
    let read_at = PathBuf::from(&l.path);
    let file = read_at.file_name().map(|f| f.to_string_lossy().into_owned()).filter(|f| f.to_lowercase().ends_with(".ttl")).unwrap_or_else(|| format!("{}.ttl", l.name.trim()));
    let target = ttstore::save_target(content, &read_at, &file, folder, original)?;
    // (a map unpacked in the content folder without a timetable yet: its TTData made)
    if let Some(d) = target.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let codepage = Line::load(&target).ok().and_then(|x| x.codepage).unwrap_or(Codepage::Windows1252);
    let text = l.to_line(target.clone(), codepage).to_text();
    hub_omsi::timetable::write_text(&target, &text, codepage).map_err(|e| format!("{}: {e}", target.display()))?;
    Ok(target)
}

/// `tt_reset`: the map's own timetable back, and the player's own lines of the map (the line
/// editor's registry) written into it again.
pub fn reset(content: &Path, vfs: &Vfs, map: &str) -> Answer {
    let (_, folder) = map_parts(map);
    let map_dir = global_dir(vfs, map)?;
    let done = ttstore::reset_timetable(content, &map_dir, &folder)?;
    let reg = lines::load_registry(&lines::registry_path(&folder));
    let (mut own, mut own_error) = (0, None);
    if !reg.lines.is_empty() {
        match lines::export_to_map(content, &map_dir, &reg) {
            Ok(_) => own = reg.lines.len(),
            Err(e) => own_error = Some(e),
        }
    }
    Ok(json!({ "done": done, "own": own, "own_error": own_error }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> TtLine {
        let trip = |t: &str, d: f64| TtTrip { trip: t.into(), profile: 0, departure: d };
        TtLine {
            path: String::new(),
            name: "10".into(),
            user_allowed: true,
            priority: 1,
            own: false,
            tours: vec![TtTour { number: "1".into(), ai_group: "Busses".into(), days: 31, trips: vec![trip("10_out", 360.0), trip("10_back", 375.0)] }],
        }
    }

    #[test]
    fn a_tour_is_copied_later_and_repeated_up_to_a_last_departure() {
        let (l, at, made) = edit(line(), 0, "copy", &json!({ "offset": 20 })).unwrap();
        assert_eq!((at, made, l.tours.len()), (1, 1, 2));
        assert_eq!((l.tours[1].number.as_str(), l.tours[1].trips[0].departure, l.tours[1].trips[1].departure), ("2", 380.0, 395.0));
        assert_eq!((l.tours[1].ai_group.as_str(), l.tours[1].days), ("Busses", 31), "the copy keeps the AI group and the days");

        // (every 30 minutes from 6:00 up to 8:00: 6:30, 7:00, 7:30, 8:00)
        let (l, at, made) = edit(line(), 0, "repeat", &json!({ "offset": "30", "until": 480 })).unwrap();
        assert_eq!((at, made), (0, 4));
        let firsts: Vec<f64> = l.tours.iter().map(|t| t.trips[0].departure).collect();
        assert_eq!(firsts, [360.0, 390.0, 420.0, 450.0, 480.0]);
        let numbers: Vec<&str> = l.tours.iter().map(|t| t.number.as_str()).collect();
        assert_eq!(numbers, ["1", "2", "3", "4", "5"]);

        assert!(edit(line(), 0, "repeat", &json!({ "offset": 0, "until": 480 })).unwrap_err().contains("1 or more"));
        assert!(edit(line(), 0, "copy", &json!({})).is_err());
    }

    #[test]
    fn a_shift_past_midnight_starts_the_day_again() {
        let mut l = line();
        l.tours[0].trips[1].departure = 1435.0;
        let (l, _, _) = edit(l, 0, "shift", &json!({ "offset": 10 })).unwrap();
        assert_eq!((l.tours[0].trips[0].departure, l.tours[0].trips[1].departure), (370.0, 5.0));
        let (l, _, _) = edit(l, 0, "shift", &json!({ "offset": -10 })).unwrap();
        assert_eq!((l.tours[0].trips[0].departure, l.tours[0].trips[1].departure), (360.0, 1435.0));
    }

    #[test]
    fn tours_are_made_deleted_and_given_trips() {
        let (l, at, _) = edit(line(), 0, "new_tour", &json!({ "first": "10_out" })).unwrap();
        assert_eq!((at, l.tours[1].number.as_str(), l.tours[1].ai_group.as_str()), (1, "2", "Busses"));
        assert_eq!(l.tours[1].trips, vec![TtTrip { trip: "10_out".into(), profile: 0, departure: 420.0 }], "an hour after the last tour's start");

        // (A, B, then A again; no .ttp to read: half an hour after the last)
        let (l, _, _) = edit(l, 0, "add_trip", &json!({ "first": "10_back" })).unwrap();
        assert_eq!((l.tours[0].trips[2].trip.as_str(), l.tours[0].trips[2].departure), ("10_out", 405.0));

        let (l, at, _) = edit(l, 1, "delete_tour", &json!({})).unwrap();
        assert_eq!((at, l.tours.len()), (0, 1));
        let (l, at, _) = edit(l, 0, "delete_tour", &json!({})).unwrap();
        assert_eq!((at, l.tours.len()), (0, 0));
        assert!(edit(l.clone(), 0, "add_trip", &json!({})).is_err());
        assert!(edit(l, 0, "nope", &json!({})).unwrap_err().contains("nope"));
    }

    #[test]
    fn a_line_s_name_must_be_a_file_name() {
        assert!(name_ok("10") && name_ok("N 5"));
        assert!(!name_ok("") && !name_ok("a/b") && !name_ok("x:") && !name_ok(".."));
    }

    #[test]
    fn the_json_reads_back_with_days_kept() {
        let v = serde_json::to_value(line()).unwrap();
        assert_eq!(v["tours"][0]["days"], 31);
        assert_eq!(serde_json::from_value::<TtLine>(v).unwrap(), line());
        // (a tour the page made without days runs every day)
        let t: TtTour = serde_json::from_value(json!({ "number": "3" })).unwrap();
        assert_eq!(t.days, DayMask::ALWAYS.0);
    }
}
