//! A trip of a map's timetable as the bus runs it, for the duty card and the planning: when it
//! is at each of its stops ([`trip_times`]) and what its driver types into the IBIS
//! ([`ibis_target`]). Our own, on `hub_omsi`'s timetable and depot file types (the old launcher
//! used the game's `timetable_run` for both).
//!
//! **The times:** the profile's whole time is the trip's; the stops the profile fixes (its
//! departures, `[profile_man_dep]`, and arrivals) keep their minutes, and the stops between two
//! fixed ones share the time between them by the lengths of the links (`StnLinks.cfg`; a link
//! the map lacks counts as one metre). A stop with a fixed departure is a hold (a timing point:
//! the bus waits for its minute). A stop the profile marks as passed (`[profile_otherstopping]`
//! with 2) is driven through.
//!
//! **The IBIS:** the line's number, the route the depot file has for the trip's stops (its
//! `[infosystem_trip]` whose stop list is the trip's, else the line's route to the same last
//! stop), and the destination that route goes to (else the one named like the trip's terminus).

use hub_omsi::hof::Hof;
use hub_omsi::timetable::{Trip, TripProfile};

/// How a profile marks a stop it passes without stopping (`[profile_otherstopping]`).
pub const PASSES: i32 = 2;

/// A trip's stops as the bus runs it: each stop's arrival and departure (seconds from the
/// trip's start), whether it stops there, and whether it waits for its minute.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripTimes {
    pub stations: Vec<(f64, f64)>,
    pub stops: Vec<bool>,
    pub holds: Vec<bool>,
}

/// The stop ids of a trip, in order.
pub fn trip_stations(trip: &Trip) -> Vec<i64> {
    trip.stops.iter().map(|s| s.id).collect()
}

/// The times of a trip of the stops `stations` with `profile` (none: one minute a link); `link`
/// gives a link's length (m) where the map has it.
pub fn trip_times(stations: &[i64], profile: Option<&TripProfile>, link: &dyn Fn(i64, i64) -> Option<f64>) -> TripTimes {
    let n = stations.len();
    if n == 0 {
        return TripTimes::default();
    }
    let lengths: Vec<f64> = stations.windows(2).map(|w| link(w[0], w[1]).filter(|l| l.is_finite() && *l > 0.0).unwrap_or(1.0)).collect();
    let total = profile.map(|p| p.minutes as f64).filter(|m| *m > 0.0).unwrap_or((n.saturating_sub(1)) as f64);
    // the minutes each stop is fixed at: departures and arrivals the profile sets, the first
    // stop at 0 and the last at the whole time
    let mut dep: Vec<Option<f64>> = vec![None; n];
    let mut arr: Vec<Option<f64>> = vec![None; n];
    let mut holds = vec![false; n];
    if let Some(p) = profile {
        for &(i, m) in &p.fixed_departures {
            if let Some(k) = usize::try_from(i).ok().filter(|k| *k < n) {
                dep[k] = Some(m as f64);
                holds[k] = true;
            }
        }
        for &(i, m) in &p.fixed_arrivals {
            if let Some(k) = usize::try_from(i).ok().filter(|k| *k < n) {
                arr[k] = Some(m as f64);
            }
        }
    }
    dep[0] = dep[0].or(Some(0.0));
    arr[n - 1] = arr[n - 1].or(Some(total));
    // a fixed point is where the bus must be by then: its arrival where fixed, else its departure
    let fixed: Vec<Option<f64>> = (0..n).map(|k| if k == 0 { dep[0] } else { arr[k].or(dep[k]) }).collect();
    let mut along = vec![0.0f64; n];
    for k in 1..n {
        along[k] = along[k - 1] + lengths[k - 1];
    }
    let mut at = vec![0.0f64; n];
    let mut j = 0;
    for k in 1..n {
        let Some(tk) = fixed[k] else { continue };
        // (from the last fixed stop's departure: a hold there leaves at its minute)
        let tj = dep[j].or(fixed[j]).unwrap_or(0.0);
        for (i, a) in at.iter_mut().enumerate().take(k + 1).skip(j + 1) {
            *a = if i == k { tk } else { tj + (tk - tj) * (along[i] - along[j]) / (along[k] - along[j]).max(1e-6) };
        }
        j = k;
    }
    let stations_out: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let a = if k == 0 { dep[0].unwrap_or(0.0) } else { at[k] };
            let d = dep[k].unwrap_or(a).max(a);
            (a * 60.0, d * 60.0)
        })
        .collect();
    let passes = |k: usize| profile.is_some_and(|p| p.stopping.iter().any(|&(i, how)| i as usize == k && how == PASSES));
    let stops = (0..n).map(|k| k == 0 || k + 1 == n || !passes(k)).collect();
    TripTimes { stations: stations_out, stops, holds }
}

/// What the driver types into the IBIS for a trip: the line's number (and its suffix), the
/// route's two digits, and the destination's place in the depot file's termini.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IbisTarget {
    pub line: i32,
    pub suffix: i32,
    pub route: Option<i32>,
    pub terminus_index: i32,
}

/// The IBIS codes of a trip of `line` to `to` with the stops `stops` (their names), from the
/// depot file `hof`. None: the depot file knows neither the route nor the destination.
pub fn ibis_target(hof: &Hof, line: &str, to: &str, stops: &[&str], _kurs: Option<&str>) -> Option<IbisTarget> {
    let want = crate::linehof::line_number(line) as i32;
    let same_name = |a: &str, b: &str| a.split('#').next().unwrap_or("").trim().eq_ignore_ascii_case(b.trim());
    let of_line = |k: usize| want > 0 && hof.info_trips[k].code.trim().parse::<i32>().is_ok_and(|c| c / 100 == want);
    let list = |k: usize| hof.info_busstop_lists.get(k).map(Vec::as_slice).unwrap_or(&[]);
    let exact = |k: usize| list(k).len() == stops.len() && list(k).iter().zip(stops).all(|(a, b)| same_name(a, b));
    let ends = |k: usize| list(k).last().zip(stops.last()).is_some_and(|(a, b)| same_name(a, b));
    let ks = 0..hof.info_trips.len();
    let route = ks
        .clone()
        .find(|&k| exact(k) && of_line(k))
        .or_else(|| ks.clone().find(|&k| exact(k)))
        .or_else(|| ks.clone().find(|&k| of_line(k) && ends(k)))
        .or_else(|| ks.clone().find(|&k| of_line(k) && same_name(&hof.info_trips[k].name, to)));
    let terminus_of_code = |code: i32| hof.termini.iter().position(|t| t.code == code);
    let terminus_named = || {
        hof.termini
            .iter()
            .position(|t| same_name(&t.texture_id, to) || t.strings.iter().any(|s| same_name(s, to)))
            .or_else(|| stops.last().and_then(|last| hof.termini.iter().position(|t| t.terminus_stop.as_deref().is_some_and(|s| same_name(s, last)))))
    };
    let terminus = route.and_then(|k| hof.info_trips[k].route.trim().parse::<i32>().ok()).and_then(terminus_of_code).or_else(terminus_named);
    if route.is_none() && terminus.is_none() {
        return None;
    }
    let code = route.and_then(|k| hof.info_trips[k].code.trim().parse::<i32>().ok());
    let line_no = if want > 0 { want } else { code.map(|c| c / 100).unwrap_or(0) };
    Some(IbisTarget { line: line_no, suffix: 0, route: code.map(|c| c % 100), terminus_index: terminus.map(|t| t as i32).unwrap_or(-1) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_omsi::hof::{InfoTrip, Terminus};

    #[test]
    fn a_trip_is_timed_by_its_profile_and_its_links() {
        // (a 30-minute profile: the third stop held at minute 20, the fourth passed)
        let p = TripProfile { minutes: 30.0, fixed_departures: vec![(2, 20.0)], stopping: vec![(3, 2)], ..Default::default() };
        let lengths = [((1, 2), 1000.0), ((2, 3), 1000.0), ((3, 5), 500.0), ((5, 4), 500.0)];
        let link = |a: i64, b: i64| lengths.iter().find(|x| x.0 == (a, b)).map(|x| x.1);
        let t = trip_times(&[1, 2, 3, 5, 4], Some(&p), &link);
        let minutes: Vec<f64> = t.stations.iter().map(|s| s.1 / 60.0).collect();
        assert_eq!(minutes, [0.0, 10.0, 20.0, 25.0, 30.0]);
        assert_eq!(t.stops, [true, true, true, false, true]);
        assert_eq!(t.holds, [false, false, true, false, false]);
        // without a profile: a minute a link
        let t = trip_times(&[1, 2, 3], None, &|_, _| None);
        assert_eq!(t.stations.last().map(|s| s.0), Some(120.0));
    }

    #[test]
    fn the_ibis_codes_come_from_the_depot_file_s_route() {
        let hof = Hof {
            termini: vec![Terminus { code: 41, texture_id: "Markt".into(), ..Default::default() }, Terminus { code: 42, texture_id: "Zentrum".into(), ..Default::default() }],
            info_trips: vec![InfoTrip { code: "13001".into(), name: "A-B".into(), route: "42".into(), line: "130".into(), extra: vec![] }],
            info_busstop_lists: vec![vec!["Station".into(), "Markt".into(), "Zentrum".into()]],
            ..Default::default()
        };
        let t = ibis_target(&hof, "130", "Zentrum", &["Station", "Markt", "Zentrum"], None).unwrap();
        assert_eq!(t, IbisTarget { line: 130, suffix: 0, route: Some(1), terminus_index: 1 });
        // a trip the depot file has no route of: its destination by name
        let t = ibis_target(&hof, "7", "Markt", &["X", "Markt"], None).unwrap();
        assert_eq!((t.line, t.route, t.terminus_index), (7, None, 0));
        assert!(ibis_target(&hof, "7", "Nowhere", &["X"], None).is_none());
    }
}
