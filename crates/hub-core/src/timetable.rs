//! Our own timetable types: a map's lines, their tours, the tours' trips and the trips' stops,
//! on one date. These are what the company, RosterFlow and the pages work with; the files they
//! come from are `hub-omsi`'s business (`hub_omsi::timetable`).
//!
//! Times are seconds of the day (`08:15` is 29 700.0); a trip past midnight goes on above
//! 86 400. Their JSON (field names, snake_case) is what the page and the saves see: keep it.

use serde::{Deserialize, Serialize};

/// The date the game starts on without `--date` (day 150 of 1989), and the app's default date.
pub const DEFAULT_DATE: &str = "1989-05-30";

/// A stop of a trip.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StopInfo {
    pub name: String,
    /// The stop's map object id: where a trip ends and another begins, the two meet at it.
    pub id: i64,
    /// When the bus is due there, and leaves (seconds of the day).
    pub arr: f64,
    pub dep: f64,
}

/// A trip of a tour, as OMSI's timetable dialog lists it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TripInfo {
    /// The trip file's name (`4 Liman-ZS`), the name the map gives the route.
    pub name: String,
    /// Its place in the tour, 1 = the first (what `--trip` takes).
    pub index: usize,
    /// The line its displays show (empty: a depot run).
    pub line: String,
    /// Its first and last stop.
    pub from: String,
    pub terminus: String,
    pub departure: f64,
    pub arrival: f64,
    pub stops: Vec<StopInfo>,
    /// Its length.
    pub km: f64,
}

/// A tour (one bus's work) of a line.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TourInfo {
    pub number: String,
    /// The AI group whose buses drive it (`ailists.cfg`).
    pub ai_group: String,
    /// Its first departure and last arrival.
    pub first: f64,
    pub last: f64,
    /// The days it runs, in words (`Mon-Fri`, `Sat`, `daily`), from its day mask.
    pub days: String,
    /// It runs on the date asked for.
    pub runs: bool,
    /// The first date from the one asked for on which it runs (`YYYY-MM-DD`).
    pub next_run: Option<String>,
    pub trips: Vec<TripInfo>,
}

/// A line of the map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LineInfo {
    /// The line file's name (`.ttl` without its extension).
    pub name: String,
    /// The player may drive it (`[userallowed]`).
    pub user_allowed: bool,
    /// Its ends: the first and last stop of its longest trip.
    pub termini: Vec<String>,
    pub tours: Vec<TourInfo>,
}

impl TourInfo {
    /// Its trips' stops in order, each once where one trip ends where the next begins.
    pub fn stops(&self) -> Vec<&StopInfo> {
        let mut out: Vec<&StopInfo> = Vec::new();
        for t in &self.trips {
            for (k, s) in t.stops.iter().enumerate() {
                if k == 0 && out.last().is_some_and(|l| l.id == s.id && s.id != 0) {
                    continue;
                }
                out.push(s);
            }
        }
        out
    }
}

/// The lines of `map` (its `global.cfg`, relative to the OMSI 2 folder) on `date`
/// (`YYYY-MM-DD`), with the map's Chrono and calendar applied: `hub-omsi`'s timetable made into
/// our types. **Port:** the rosterflow and company ports call this where the old code called
/// the launcher's `list_lines` / `lines_on`.
pub fn list_lines(map: &str, date: &str) -> Result<Vec<LineInfo>, String> {
    let date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| format!("date {date}: {e}"))?;
    let vfs = crate::paths::vfs().ok_or("the OMSI 2 folder is not set")?;
    let lines = hub_omsi::timetable::lines_for(&vfs, map, date).map_err(|e| e.to_string())?;
    Ok(lines.into_iter().map(LineInfo::from).collect())
}

impl From<hub_omsi::timetable::ServiceStop> for StopInfo {
    fn from(s: hub_omsi::timetable::ServiceStop) -> StopInfo {
        StopInfo { name: s.name, id: s.id, arr: s.arr, dep: s.dep }
    }
}

impl From<hub_omsi::timetable::ServiceTrip> for TripInfo {
    fn from(t: hub_omsi::timetable::ServiceTrip) -> TripInfo {
        TripInfo {
            name: t.name,
            index: t.index,
            line: t.line,
            from: t.from,
            terminus: t.terminus,
            departure: t.departure,
            arrival: t.arrival,
            stops: t.stops.into_iter().map(StopInfo::from).collect(),
            km: t.km,
        }
    }
}

impl From<hub_omsi::timetable::ServiceTour> for TourInfo {
    fn from(t: hub_omsi::timetable::ServiceTour) -> TourInfo {
        TourInfo {
            number: t.number,
            ai_group: t.ai_group,
            first: t.first,
            last: t.last,
            days: t.days.words(),
            runs: t.runs,
            next_run: t.next_run.map(|d| d.format("%Y-%m-%d").to_string()),
            trips: t.trips.into_iter().map(TripInfo::from).collect(),
        }
    }
}

impl From<hub_omsi::timetable::ServiceLine> for LineInfo {
    fn from(l: hub_omsi::timetable::ServiceLine) -> LineInfo {
        LineInfo { name: l.name, user_allowed: l.user_allowed, termini: l.termini, tours: l.tours.into_iter().map(TourInfo::from).collect() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(id: i64, name: &str, at: f64) -> StopInfo {
        StopInfo { name: name.into(), id, arr: at, dep: at }
    }

    #[test]
    fn a_tour_s_stops_meet_where_its_trips_do() {
        let a = TripInfo { stops: vec![stop(1, "Zoo", 0.0), stop(2, "Hbf", 600.0)], ..TripInfo::default() };
        let b = TripInfo { stops: vec![stop(2, "Hbf", 900.0), stop(1, "Zoo", 1500.0)], ..TripInfo::default() };
        let tour = TourInfo { trips: vec![a, b], ..TourInfo::default() };
        let names: Vec<&str> = tour.stops().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Zoo", "Hbf", "Zoo"]);
    }

    #[test]
    fn the_json_keeps_its_field_names() {
        let l = LineInfo { name: "10".into(), user_allowed: true, ..LineInfo::default() };
        let v = serde_json::to_value(&l).unwrap();
        assert_eq!(v["name"], "10");
        assert_eq!(v["user_allowed"], true);
        // (an older save without a field still reads)
        let t: TourInfo = serde_json::from_str(r#"{"number":"2"}"#).unwrap();
        assert_eq!((t.number.as_str(), t.next_run), ("2", None));
    }
}
