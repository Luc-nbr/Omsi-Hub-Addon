//! How a late bus is brought back on time, worked out from the timetable's stops, the day's
//! blocks and duties and RosterFlow's deadheads (the testers, 2026-10-19: "the system should be
//! way smarter than it is now ... it should use the map data, the timetable data and the duty
//! data within RosterFlow"):
//!
//! - **a short-turn** loses a part of a trip: the bus turns back at a stop on the way that the
//!   next trip calls at too, the latest one from which it rejoins the next trip on time - the
//!   stops after it and the next trip's before it are not served (`short_turn`). Not where its
//!   driver is relieved at the terminus: the relief would wait for a bus that never comes;
//! - **running dead to the opposite terminus** loses a whole trip: the bus runs its next trip
//!   empty, in the time RosterFlow's deadheads give (Preferences › Vehicles › Deadheads; else
//!   the trip without its stops), and is there for the trip after on time (`dead_run`);
//! - **a spare bus** loses no trip but costs wages and fuel: a driver brings it out from the
//!   depot to the terminus - a spare line standing by at the depot is paid already, else one is
//!   called in - and takes the late bus back (`swap_cost`);
//! - **a shorter layover** keeps what the terminus needs: a bus stands at least the turnaround -
//!   a minute for the driver, and the time the passengers waiting take to board, more at a busy
//!   terminus several lines meet at than at a quiet one (`turnaround`) -, and a late driver
//!   keeps a couple of minutes besides (`kept_rest`), which service control can take off.

use super::model::{Cents, Company};
use super::network::{PlannedTrip, TourOfDay};
use crate::LineInfo;

/// The minute a driver needs at a terminus before the next trip (the blinds, the ticket machine).
pub const TURN_MINUTES: i32 = 1;
/// What a late driver keeps of a layover besides the turnaround (service control can take it off).
pub const DRIVER_REST: i32 = 2;
/// Seconds a passenger takes to board (a smartcard, a ticket bought).
const BOARD_SECONDS: f64 = 4.0;
/// The share of a trip's passengers who board at its first stop on a quiet terminus; each other
/// line beginning or ending there adds half of it again (an interchange).
const TERMINUS_SHARE: f64 = 0.15;
const TERMINUS_SHARE_MAX: f64 = 0.45;
/// Minutes a bus takes to turn round at a stop on the way (a loop, a turning circle).
pub const SHORT_TURN_MINUTES: i32 = 2;
/// What a stop costs a bus in service (pulling in, the doors, the passengers, pulling out): what
/// running without passengers saves.
pub const DWELL: f64 = 0.5;
/// Minutes to get a spare bus going at the depot: a spare line's driver standing by there, or
/// a driver called in.
pub const SPARE_READY: i32 = 5;
pub const CALLED_IN: i32 = 15;

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

// --- the terminus -----------------------------------------------------------------------------

/// How many of the day's lines begin or end at each stop: a terminus several lines meet at is
/// busier than one only its own line reaches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Termini(Vec<(String, Vec<String>)>);

impl Termini {
    pub fn of<'a>(tours: impl IntoIterator<Item = &'a TourOfDay>) -> Termini {
        let mut out: Vec<(String, Vec<String>)> = Vec::new();
        for t in tours {
            for x in t.trips.iter().filter(|x| x.counts()) {
                for stop in [x.from.as_str(), x.end()] {
                    let stop = stop.trim();
                    if stop.is_empty() {
                        continue;
                    }
                    let at = match out.iter().position(|e| same(&e.0, stop)) {
                        Some(i) => i,
                        None => {
                            out.push((stop.to_string(), Vec::new()));
                            out.len() - 1
                        }
                    };
                    if !out[at].1.iter().any(|l| same(l, &t.line)) {
                        out[at].1.push(t.line.clone());
                    }
                }
            }
        }
        Termini(out)
    }

    /// The lines that begin or end at `stop` (at least its own).
    pub fn lines_at(&self, stop: &str) -> usize {
        self.0.iter().find(|e| same(&e.0, stop)).map_or(1, |e| e.1.len().max(1))
    }
}

/// The passengers waiting at the first stop of trip `x` of `line` to board before it leaves
/// (`lines_at`: the lines that meet there).
pub fn waiting(c: &Company, line: &str, x: &PlannedTrip, lines_at: usize) -> f64 {
    if !x.counts() {
        return 0.0;
    }
    let l = c.lines.iter().find(|l| same(&l.name, line));
    let pax = super::income::expected(l, &c.date, x.dep, x.arr);
    let share = (TERMINUS_SHARE * (1.0 + 0.5 * (lines_at.max(1) - 1) as f64)).min(TERMINUS_SHARE_MAX);
    pax.max(0.0) * share
}

/// The minutes the passengers waiting for trip `x` take to board at its first stop.
pub fn boarding_minutes(c: &Company, line: &str, x: &PlannedTrip, lines_at: usize) -> i32 {
    (waiting(c, line, x, lines_at) * BOARD_SECONDS / 60.0).round() as i32
}

/// The least a bus stands at the first stop of trip `x` before it leaves: the driver's minute and
/// the boarding.
pub fn turnaround(c: &Company, line: &str, x: &PlannedTrip, lines_at: usize) -> i32 {
    TURN_MINUTES + boarding_minutes(c, line, x, lines_at)
}

/// What a bus coming in late keeps of its layover before trip `x`: the turnaround and the
/// driver's couple of minutes (a quiet terminus `control::MIN_LAYOVER`).
pub fn kept_rest(c: &Company, line: &str, x: &PlannedTrip, lines_at: usize) -> i32 {
    turnaround(c, line, x, lines_at) + DRIVER_REST
}

// --- the timetable's stops ----------------------------------------------------------------------

/// A trip's stops as the timetable has them: each stop's name and how far along the trip it is
/// due there (by the times: 0 the first stop, 1 the last).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Stops {
    pub names: Vec<String>,
    pub at: Vec<f64>,
}

impl Stops {
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The stops still ahead of a bus `share` of the way along.
    pub fn ahead(&self, share: f64) -> usize {
        self.at.iter().filter(|a| **a > share + 1e-6).count()
    }
}

/// Trip `x`'s stops in the timetable read (`lines`): the trip of its name leaving nearest its time
/// (the company's own timetable may have moved it; the shares stay). None: not found, or a part
/// of a trip cut at a relief point.
pub fn stops_of(lines: &[LineInfo], x: &PlannedTrip) -> Option<Stops> {
    if x.part.is_some() || x.name.trim().is_empty() {
        return None;
    }
    let found = lines
        .iter()
        .flat_map(|l| l.tours.iter())
        .flat_map(|t| t.trips.iter())
        .filter(|y| same(&y.name, &x.name) && y.stops.len() >= 2)
        .min_by_key(|y| ((y.departure / 60.0).round() as i32 - x.dep).abs())?;
    let first = found.stops.first().map(|s| if s.dep > 0.0 { s.dep } else { s.arr })?;
    let last = found.stops.last().map(|s| if s.arr > 0.0 { s.arr } else { s.dep })?;
    let span = last - first;
    if span <= 0.0 {
        return None;
    }
    let names = found.stops.iter().map(|s| s.name.trim().to_string()).collect();
    let at = found.stops.iter().map(|s| (((if s.arr > 0.0 { s.arr } else { s.dep }) - first) / span).clamp(0.0, 1.0)).collect();
    Some(Stops { names, at })
}

// --- a short-turn -------------------------------------------------------------------------------

/// Where a late bus best turns back.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    /// The stop it turns at.
    pub stop: String,
    /// The share of its trip not served (after the stop), and of the next (before it).
    pub part: f64,
    pub next: f64,
    /// The stops not served, of both trips.
    pub skipped: usize,
    /// Its minutes late when it rejoins the next trip.
    pub late: i32,
}

/// The stop a bus on trip `a` (its stops `sa`, `from` of the way along now, `start` minutes late
/// leaving and `end` at the end) best turns back at to rejoin trip `n` (`sn`; `rest` the minutes
/// it keeps at the terminus otherwise) - the latest from which it is back on time (under
/// `on_time` minutes), else the one bringing it back the most. None: no stop of both, or none
/// ahead that brings it back two minutes or more.
#[allow(clippy::too_many_arguments)]
pub fn short_turn(sa: &Stops, sn: &Stops, a: &PlannedTrip, n: &PlannedTrip, from: f64, start: i32, end: i32, rest: i32, on_time: i32) -> Option<Turn> {
    if sa.len() < 3 || sn.len() < 3 {
        return None;
    }
    let (dur_a, dur_n) = ((a.arr - a.dep).max(1) as f64, (n.arr - n.dep).max(1) as f64);
    // (left as it is: the next trip's minutes late leaving)
    let now_late = (a.arr + end + rest.min((n.dep - a.arr).max(0)) - n.dep).max(0);
    let mut best: Option<Turn> = None;
    // (the stops ahead, not the terminus itself; each once on the way back, not its first stop)
    for i in 1..sa.len() - 1 {
        if sa.at[i] <= from + 0.02 {
            continue;
        }
        let Some(k) = (1..sn.len() - 1).find(|&k| same(&sn.names[k], &sa.names[i])) else { continue };
        let delay = start as f64 + (end - start) as f64 * sa.at[i];
        let there = a.dep as f64 + sa.at[i] * dur_a + delay;
        let due = n.dep as f64 + sn.at[k] * dur_n;
        let late = (there + SHORT_TURN_MINUTES as f64 - due).max(0.0).floor() as i32;
        let t = Turn { stop: sa.names[i].clone(), part: 1.0 - sa.at[i], next: sn.at[k], skipped: (sa.len() - 1 - i) + k, late };
        // (the latest stop back on time: the fewest passengers left; else the one that helps most)
        best = match best {
            None => Some(t),
            Some(b) if t.late < on_time && (b.late >= on_time || t.part < b.part) => Some(t),
            Some(b) if b.late >= on_time && t.late < b.late => Some(t),
            b => b,
        };
    }
    best.filter(|t| t.late <= now_late - 2 && t.part > 0.0)
}

/// Whether a short-turn on trip `j` of tour `t` would leave a relief waiting: a duty ends with it
/// and another driver takes the bus over at its terminus (`duties`: each duty's first and its
/// end, exclusive; `drivers`: who drives each).
pub fn relieved_after(duties: &[(usize, usize)], j: usize) -> bool {
    duties.iter().any(|&(s, e)| e == j + 1 && duties.iter().any(|&(s2, _)| s2 == j + 1 && s2 != s))
}

// --- running dead -----------------------------------------------------------------------------

/// A bus running its trip `x` empty to its terminus: the minutes and kilometres (RosterFlow's
/// deadhead between its ends where the Preferences set one; else the trip without its stops) and
/// the minutes it saves against running it in service.
pub fn dead_run(c: &Company, tours: &[TourOfDay], line: &[&PlannedTrip], x: &PlannedTrip) -> (i32, f64, i32) {
    let dur = (x.arr - x.dep).max(1);
    let dh = super::rosterflow::deadhead_between(c, tours, line, &x.from, x.end());
    let minutes = if dh.set { dh.minutes } else { (dur as f64 - x.stops.saturating_sub(2) as f64 * DWELL).round().max(dur as f64 * 0.5) as i32 };
    let km = if dh.set { dh.km } else { x.km };
    (minutes, km, (dur - minutes).max(0))
}

/// The minutes running the rest of trip `x` out of service saves (`rem` of it left, `stops`
/// still to call at): the stops not called at, at the most 40 % of the time left.
pub fn express_saves(x: &PlannedTrip, rem: f64, stops: usize) -> i32 {
    let left = rem * (x.arr - x.dep).max(1) as f64;
    (stops as f64 * DWELL).min(left * 0.4).round().max(if left >= 3.0 { 1.0 } else { 0.0 }) as i32
}

// --- a spare bus ------------------------------------------------------------------------------

/// What bringing a spare bus to `terminus` comes to: when it can be there (from `now`), and its
/// cost - its kilometres out and the late bus's back to the depot (fuel and wear), and the
/// wages of the driver who brings it out and the late bus back (none when a spare line stands
/// by at the depot: paid already).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpareRun {
    pub there: i32,
    pub km: f64,
    pub fuel: Cents,
    pub wages: Cents,
    pub spare_line: bool,
}

impl SpareRun {
    pub fn cost(&self) -> Cents {
        self.fuel + self.wages
    }
}

pub fn spare_run(c: &Company, tours: &[TourOfDay], line: &[&PlannedTrip], vehicle: u32, late_bus: u32, terminus: &str, now: i32) -> SpareRun {
    use super::rosterflow::{deadhead_between, spare_lines, DEPOT_POINT};
    let out = deadhead_between(c, tours, line, DEPOT_POINT, terminus);
    let back = deadhead_between(c, tours, line, terminus, DEPOT_POINT);
    let spare_line = spare_lines(c, &c.date).iter().any(|x| x.at_depot() && x.covers(now, now + CALLED_IN + out.minutes + back.minutes));
    let ready = if spare_line { SPARE_READY } else { CALLED_IN };
    let level = super::econ_uk::Level::of(c);
    let per_km = |id: u32| {
        c.vehicle(id).map_or(0.0, |v| {
            let age = super::dates::years_between(&v.built, &c.date);
            super::economy::energy_per_km(v.kind, &level) + super::economy::maintenance_per_km(v.kind, age, &level)
        })
    };
    let fuel = (out.km * per_km(vehicle) + back.km * per_km(late_bus)).round() as Cents;
    let wages = if spare_line { 0 } else { ((ready + out.minutes + back.minutes) as f64 / 60.0 * super::plan::agency_hour(c)).round() as Cents };
    SpareRun { there: now + ready + out.minutes, km: out.km + back.km, fuel, wages, spare_line }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StopInfo, TourInfo, TripInfo};

    fn trip(name: &str, from: &str, to: &str, dep: i32, arr: i32, stops: u32) -> PlannedTrip {
        PlannedTrip { name: name.into(), line: "5".into(), from: from.into(), to: to.into(), dep, arr, km: 15.0, stops, ..Default::default() }
    }

    /// A line out and back over six stops, A to F, ten minutes between each.
    fn lines() -> Vec<LineInfo> {
        let names = ["A", "B", "C", "D", "E", "F"];
        let info = |name: &str, order: Vec<&str>, dep: i32| TripInfo {
            name: name.into(),
            line: "5".into(),
            from: order[0].into(),
            terminus: order[5].into(),
            departure: dep as f64 * 60.0,
            arrival: (dep + 50) as f64 * 60.0,
            stops: order.iter().enumerate().map(|(i, s)| StopInfo { name: s.to_string(), id: i as i64, arr: (dep + 10 * i as i32) as f64 * 60.0, dep: (dep + 10 * i as i32) as f64 * 60.0 }).collect(),
            ..Default::default()
        };
        let mut back = names.to_vec();
        back.reverse();
        vec![LineInfo { name: "Linie5".into(), tours: vec![TourInfo { trips: vec![info("out", names.to_vec(), 360), info("back", back, 420)], ..Default::default() }], ..Default::default() }]
    }

    #[test]
    fn the_stops_come_from_the_timetable_even_moved() {
        let l = lines();
        // (the company's timetable moved it ten minutes: the shares stay)
        let s = stops_of(&l, &trip("out", "A", "F", 370, 420, 6)).unwrap();
        assert_eq!(s.names, vec!["A", "B", "C", "D", "E", "F"]);
        assert_eq!(s.at, vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
        assert_eq!(s.ahead(0.5), 3);
        assert!(stops_of(&l, &trip("nowhere", "A", "F", 370, 420, 6)).is_none());
    }

    #[test]
    fn a_short_turn_turns_at_the_latest_stop_back_on_time() {
        let l = lines();
        let (a, n) = (trip("out", "A", "F", 360, 410, 6), trip("back", "F", "A", 420, 470, 6));
        let (sa, sn) = (stops_of(&l, &a).unwrap(), stops_of(&l, &n).unwrap());
        // 20 minutes late, just leaving: at E at 07:00, turned round 07:02, E back is due at
        // 07:10 - on time, one stop of each trip lost
        let t = short_turn(&sa, &sn, &a, &n, 0.0, 20, 20, 3, 6).unwrap();
        assert_eq!((t.stop.as_str(), t.skipped, t.late), ("E", 2, 0));
        assert!((t.part - 0.2).abs() < 1e-9 && (t.next - 0.2).abs() < 1e-9);
        // 35 minutes late: E would be 7 late (07:17 against 07:10), D is on time (07:07 against
        // 07:20) - two stops of each trip lost
        let t = short_turn(&sa, &sn, &a, &n, 0.0, 35, 35, 3, 6).unwrap();
        assert_eq!((t.stop.as_str(), t.late, t.skipped), ("D", 0, 4));
        // past D: E is all that is left, 7 late against 28 left as it is
        let t = short_turn(&sa, &sn, &a, &n, 0.65, 35, 35, 3, 6).unwrap();
        assert_eq!((t.stop.as_str(), t.late), ("E", 7));
        // past E: no stop left to turn at
        assert!(short_turn(&sa, &sn, &a, &n, 0.85, 35, 35, 3, 6).is_none());
        // on time: nothing to gain
        assert!(short_turn(&sa, &sn, &a, &n, 0.0, 0, 0, 3, 6).is_none());
    }

    #[test]
    fn a_relief_at_the_terminus_rules_a_short_turn_out() {
        // two duties: trips 0-1 and 2-3; the first ends with trip 1, the second takes over at 2
        let duties = [(0, 2), (2, 4)];
        assert!(relieved_after(&duties, 1));
        assert!(!relieved_after(&duties, 0));
        assert!(!relieved_after(&[(0, 4)], 1));
    }

    #[test]
    fn a_busy_terminus_takes_longer_to_turn_round() {
        let (mut c, _, _) = super::super::lateness::tests::company();
        c.lines.push(super::super::model::CompanyLine { name: "Linie5".into(), number: "5".into(), ..Default::default() });
        let x = trip("out", "A", "F", 8 * 60, 9 * 60, 20);
        let quiet = turnaround(&c, "Linie5", &x, 1);
        let busy = turnaround(&c, "Linie5", &x, 4);
        assert!(quiet >= TURN_MINUTES && busy >= quiet, "{quiet} {busy}");
        assert!(waiting(&c, "Linie5", &x, 4) > waiting(&c, "Linie5", &x, 1));
        assert_eq!(kept_rest(&c, "Linie5", &x, 1), quiet + DRIVER_REST);
        // (a trip without passengers: nobody boards)
        assert_eq!(boarding_minutes(&c, "Linie5", &PlannedTrip { empty: true, ..x.clone() }, 4), 0);
    }

    #[test]
    fn running_dead_saves_the_stops_or_takes_the_deadhead_set() {
        let (c, _, _) = super::super::lateness::tests::company();
        let x = trip("out", "A", "F", 360, 410, 22);
        let (minutes, _, saved) = dead_run(&c, &[], &[], &x);
        assert_eq!((minutes, saved), (40, 10));
        // (running the last half out of service: its stops, at the most 40 % of the time left)
        assert_eq!(express_saves(&x, 0.5, 11), 6);
        assert_eq!(express_saves(&x, 0.5, 4), 2);
    }
}
