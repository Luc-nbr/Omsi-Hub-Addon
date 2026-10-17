//! Trips: every trip of a duty judged at its end (the plugin's trip report, the old game's
//! `trip_report`), a line each in `trips/<driver>.jsonl` of the data folder - written as the
//! trip ends, not with the session, so that a game that does not close cleanly keeps the trips
//! it drove. (From the old `omsi-launcher-core/src/lib.rs`, Omsi-Hub's own part of it.)
//!
//! Also the clock helpers the company uses from there: [`lead_start`], [`bus_lead`],
//! [`local_now`] and [`utc_offset`].

use crate::company;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// This machine's local date and time now: (year, month, day, hour, minute).
pub fn local_now() -> Option<(i32, i32, i32, i32, i32)> {
    use chrono::{Datelike, Timelike};
    let t = chrono::Local::now();
    Some((t.year(), t.month() as i32, t.day() as i32, t.hour() as i32, t.minute() as i32))
}

/// This machine's distance from UTC in seconds now, to the quarter hour.
pub fn utc_offset() -> i64 {
    let Some((y, mo, d, h, mi)) = local_now() else { return 0 };
    let utc = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_secs() as i64).unwrap_or(0);
    // days from the civil date (Howard Hinnant)
    let (y, m) = if mo <= 2 { (y as i64 - 1, mo as i64 + 9) } else { (y as i64, mo as i64 - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + d as i64 - 1;
    let local = (era * 146097 + doe - 719468) * 86400 + h as i64 * 3600 + mi as i64 * 60;
    ((local - utc) as f64 / 900.0).round() as i64 * 900
}

// ---------------------------------------------------------------------------------------
// trips: every trip of a duty the game judged at its end (`omsi-app`'s `trip_report`), a
// line each in the data folder's trips/<driver>.jsonl - written as the trip ends, not with the
// session, so that a game that does not close cleanly keeps the trips it drove

/// One trip as the game judged it at its end: the stops served and how many of them early
/// and late as OMSI 2 counts them (left more than two minutes before the timetable, arrived
/// more than three after), how far off the timetable it ran, and what happened on the way.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct TripRun {
    /// When it ended (Unix time).
    pub time: u64,
    pub driver: String,
    pub map: String,
    pub bus: String,
    /// The line (empty: an empty run) and tour it was driven on, and where it went.
    pub line: String,
    pub tour: String,
    pub terminus: String,
    /// Its place in the duty (1 = the first) and the duty's trips.
    pub trip: usize,
    pub trips: usize,
    /// When it was to leave its first stop and reach its last (seconds of the day).
    pub departure: f64,
    pub arrival: f64,
    /// A free drive along a line: its stops are followed, not judged.
    pub free: bool,
    /// It reached its last stop (else the duty went on from its last leg).
    pub completed: bool,
    /// The stops it had to serve, and those served (on a free drive: reached).
    pub planned: i32,
    pub stops: i32,
    pub early: i32,
    pub late: i32,
    /// The stop furthest off the timetable (s, negative early) and the average of them all.
    pub worst: Option<f64>,
    pub average: Option<f64>,
    /// Seconds and metres driven from its first stop, jolts and collisions on the way, and
    /// the passengers who got on.
    pub seconds: f64,
    pub metres: f64,
    pub jolts: i32,
    pub crashes: i32,
    pub passengers: i32,
    /// What the game's drive watch saw on the way (`omsi-app`'s `drive_watch`; a trip of an
    /// older game has none): red lights run and speed cameras that flashed, with what they
    /// cost in fines (cents), and the trip's comfort - hard braking, hard starts and stops
    /// that jerked the passengers. `watched`: the watch ran.
    pub watched: bool,
    pub red_lights: i32,
    pub speeding: i32,
    pub fines: i64,
    pub hard_brakes: i32,
    pub hard_starts: i32,
    pub rough_stops: i32,
    pub offences: Vec<company::career::Offence>,
    /// The stops of a timed trip in order, as the trip's report lists them: when it was due
    /// and when the bus came and left (a trip of an older game has none).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub visits: Vec<StopVisit>,
}

/// One stop of a trip as the bus served it (`TripRun::visits`).
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct StopVisit {
    /// Its place among the trip's stations (0 = the first) and its name.
    pub index: usize,
    pub name: String,
    /// The timetable's arrival and departure there (s of the day).
    pub arr: f64,
    pub dep: f64,
    /// When the bus came and left (s of the day); None: it did not (a stop not reached; the
    /// last stop has no departure).
    pub arrived: Option<f64>,
    pub left: Option<f64>,
}

impl TripRun {
    /// Its stops served neither early nor late.
    pub fn on_time(&self) -> i32 {
        (self.stops - self.early - self.late).max(0)
    }

    /// It was driven to a timetable and served a stop.
    pub fn timed(&self) -> bool {
        !self.free && self.stops > 0
    }
}

#[cfg(test)]
mod trip_visit_tests {
    use super::*;

    /// A trip's stops go into its record and come back; a line of an older game, without
    /// them, still reads.
    #[test]
    fn a_trip_keeps_its_stops() {
        let run = TripRun {
            line: "5".into(),
            stops: 2,
            visits: vec![
                StopVisit { index: 0, name: "Rathaus".into(), arr: 100.0, dep: 130.0, arrived: Some(150.0), left: Some(160.0) },
                StopVisit { index: 3, name: "Bahnhof".into(), arr: 400.0, dep: 400.0, arrived: Some(420.0), left: None },
            ],
            ..Default::default()
        };
        let back: TripRun = serde_json::from_str(&serde_json::to_string(&run).unwrap()).unwrap();
        assert_eq!(back, run);
        let old: TripRun = serde_json::from_str(r#"{"line":"5","stops":3}"#).unwrap();
        assert!(old.visits.is_empty());
        assert!(!serde_json::to_string(&TripRun::default()).unwrap().contains("visits"));
    }
}

/// What a driver's trips come to, for the service record.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct TripTotals {
    /// The trips, and those of them driven to a timetable (a free drive keeps none).
    pub trips: usize,
    pub timed: usize,
    /// Their stops served, and of those early and late (on time is the rest).
    pub stops: i32,
    pub early: i32,
    pub late: i32,
    /// The average delay over all those stops (s, negative early), and the stop furthest
    /// off the timetable on any trip (either way).
    pub average: Option<f64>,
    pub worst: Option<f64>,
    /// Trips of five stops or more without a stop early or late.
    pub spotless: usize,
}

/// What `trips` come to.
pub fn trip_totals(trips: &[TripRun]) -> TripTotals {
    let timed: Vec<&TripRun> = trips.iter().filter(|t| t.timed()).collect();
    // (the average of the trips' averages weighed by their stops: a trip of thirty stops
    // says more than one of three)
    let weighed: Vec<(f64, i32)> = timed.iter().filter_map(|t| t.average.filter(|a| a.is_finite()).map(|a| (a, t.stops))).collect();
    let n: i32 = weighed.iter().map(|w| w.1).sum();
    TripTotals {
        trips: trips.len(),
        timed: timed.len(),
        stops: timed.iter().map(|t| t.stops).sum(),
        early: timed.iter().map(|t| t.early).sum(),
        late: timed.iter().map(|t| t.late).sum(),
        average: (n > 0).then(|| weighed.iter().map(|(a, k)| a * *k as f64).sum::<f64>() / n as f64),
        worst: timed.iter().filter_map(|t| t.worst).filter(|w| w.is_finite()).max_by(|a, b| a.abs().total_cmp(&b.abs())),
        spotless: timed.iter().filter(|t| t.stops >= 5 && t.early == 0 && t.late == 0).count(),
    }
}

/// The file of a driver's trips in the data folder `data` (`crate::data_dir`):
/// `trips/<name>.jsonl`, the name in lower case and without what a file name cannot hold
/// ("Luc" and "luc" are one driver, as the profiles have them).
pub fn trips_file(data: &Path, driver: &str) -> PathBuf {
    let name: String = driver.trim().to_lowercase().chars().map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') { c } else { '_' }).collect();
    let name = if name.trim().is_empty() { "driver".to_string() } else { name };
    data.join("trips").join(format!("{name}.jsonl"))
}

/// Add a trip to its driver's file (`trips_file`).
pub fn append_trip(data: &Path, t: &TripRun) -> Result<PathBuf> {
    use std::io::{Read, Seek, SeekFrom, Write};
    let path = trips_file(data, &t.driver);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).read(true).append(true).open(&path)?;
    // (a line a game left half written - it was ended in the middle of writing - gets its
    // end first: this trip is not lost with it)
    let mut line = String::new();
    if f.seek(SeekFrom::End(0))? > 0 {
        f.seek(SeekFrom::End(-1))?;
        let mut last = [0u8];
        f.read_exact(&mut last)?;
        if last[0] != b'\n' {
            line.push('\n');
        }
    }
    line.push_str(&serde_json::to_string(t)?);
    line.push('\n');
    f.write_all(line.as_bytes())?;
    Ok(path)
}

/// A driver's trips in the data folder `data`, the newest first (a line that cannot be read
/// is left out).
pub fn trips_of(data: &Path, driver: &str) -> Vec<TripRun> {
    let text = std::fs::read_to_string(trips_file(data, driver)).unwrap_or_default();
    let me = driver.trim().to_lowercase();
    let mut out: Vec<TripRun> =
        text.lines().filter(|l| !l.trim().is_empty()).filter_map(|l| serde_json::from_str::<TripRun>(l).ok()).filter(|t| t.driver.trim().is_empty() || t.driver.trim().to_lowercase() == me).collect();
    // (written in the order they ended: of two in the same second the later line first)
    out.reverse();
    out.sort_by(|a, b| b.time.cmp(&a.time));
    out
}

#[cfg(test)]
mod trip_tests {
    use super::*;
    use std::io::Write;

    fn trip(time: u64, stops: i32, early: i32, late: i32, average: f64, worst: f64) -> TripRun {
        TripRun {
            time,
            driver: "Luc".into(),
            line: "5".into(),
            terminus: "Rathaus".into(),
            planned: stops,
            stops,
            early,
            late,
            average: Some(average),
            worst: Some(worst),
            completed: true,
            ..Default::default()
        }
    }

    fn folder(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("omsi-trips-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn trips_are_added_up_by_their_stops() {
        let free = TripRun { free: true, stops: 9, ..Default::default() };
        let all = vec![trip(3, 30, 1, 2, 60.0, 250.0), trip(2, 10, 0, 0, -30.0, -150.0), free, trip(1, 6, 0, 0, 10.0, 40.0)];
        let t = trip_totals(&all);
        assert_eq!((t.trips, t.timed, t.stops, t.early, t.late, t.spotless), (4, 3, 46, 1, 2, 2));
        // (30 * 60 - 10 * 30 + 6 * 10) / 46
        assert!((t.average.unwrap() - 1560.0 / 46.0).abs() < 1e-9);
        // furthest off either way: 250 s late beats 150 s early
        assert_eq!(t.worst, Some(250.0));
        assert_eq!(all[0].on_time(), 27);
        let none = trip_totals(&[]);
        assert_eq!((none.trips, none.average, none.worst), (0, None, None));
    }

    #[test]
    fn the_trips_file_is_the_drivers_whatever_the_case() {
        let data = Path::new("/data");
        assert_eq!(trips_file(data, "Luc"), trips_file(data, " luc "));
        assert_eq!(trips_file(data, "Luc").file_name().unwrap(), "luc.jsonl");
        assert_eq!(trips_file(data, "../a:b").file_name().unwrap(), "___a_b.jsonl");
        assert_eq!(trips_file(data, "").file_name().unwrap(), "driver.jsonl");
    }

    /// Written a line at a time and read back newest first; a line cut short by a game that
    /// ended while writing costs that line only, and another driver's are not read.
    #[test]
    fn trips_are_written_and_read_back() {
        let data = folder("rw");
        assert!(trips_of(&data, "Luc").is_empty());
        append_trip(&data, &trip(100, 12, 1, 0, 20.0, -130.0)).unwrap();
        let path = append_trip(&data, &trip(200, 8, 0, 3, 200.0, 400.0)).unwrap();
        // half a line, as a game killed while writing leaves it
        std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"{\"time\": 250, \"driv").unwrap();
        append_trip(&data, &trip(300, 5, 0, 0, 0.0, 30.0)).unwrap();
        let other = TripRun { driver: "Anna".into(), ..trip(400, 4, 0, 0, 0.0, 0.0) };
        append_trip(&data, &other).unwrap();
        let read = trips_of(&data, "LUC");
        assert_eq!(read.iter().map(|t| t.time).collect::<Vec<_>>(), [300, 200, 100]);
        assert_eq!(read[1], trip(200, 8, 0, 3, 200.0, 400.0));
        assert_eq!(trips_of(&data, "anna").len(), 1);
        // (an older line without the newer fields still reads)
        std::fs::write(trips_file(&data, "Old"), "{\"time\": 5, \"driver\": \"Old\", \"stops\": 3}\n").unwrap();
        assert_eq!(trips_of(&data, "Old")[0].stops, 3);
        let _ = std::fs::remove_dir_all(&data);
    }
}

/// The minutes a duty's bus stands ready before its first departure: by default five, at
/// most an hour.
pub const BUS_LEAD_DEFAULT: i64 = 5;

pub fn bus_lead(v: Option<i64>) -> i64 {
    v.unwrap_or(BUS_LEAD_DEFAULT).clamp(0, 60)
}

/// The start of a duty whose first departure is at `minutes` on `date`, `lead` minutes
/// before it: the time and the date (the day before when that is before midnight).
pub fn lead_start(date: &str, minutes: i32, lead: i64) -> (String, i32) {
    let t = minutes - lead.clamp(0, 60) as i32;
    if t >= 0 {
        (date.to_string(), t)
    } else {
        (company::dates::add(date, -1), t + 24 * 60)
    }
}
