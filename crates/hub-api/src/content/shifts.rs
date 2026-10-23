//! Work shifts and what a duty of ours adds to the drive.
//!
//! * `hub_shifts {map, date, minutes, daypart, seed, legs}` - work shifts put together from the
//!   day's timetable (`hub_core::compose`), as the duty step lists them;
//! * `hub_duty_extras {bus, date, minutes, free}` - the start moved `bus_lead` minutes (a
//!   setting) ahead of the first departure, and the display font chosen for the bus.

use crate::{arg, Answer};
use hub_core::compose::{Block, ComposedDuty, Leg, Wish};
use hub_core::LineInfo;
use serde_json::{json, Value};

/// The parts of the day a shift may begin in: name, and from-to in minutes of the day (the
/// night runs past midnight).
pub const DAYPARTS: [(&str, Option<(i32, i32)>); 5] =
    [("All day", None), ("Morning", Some((4 * 60, 11 * 60))), ("Midday", Some((11 * 60, 16 * 60))), ("Evening", Some((16 * 60, 22 * 60))), ("Night", Some((22 * 60, 30 * 60)))];

/// How many shifts are drawn for one wish.
const SHIFTS: usize = 8;

pub fn shifts(args: &Value) -> Answer {
    let legs = arg::<Option<Vec<String>>>(args, "legs")?.unwrap_or_default();
    let minutes = arg::<Option<f64>>(args, "minutes")?.unwrap_or(120.0).round() as i64;
    let daypart = arg::<Option<f64>>(args, "daypart")?.unwrap_or(0.0).max(0.0) as usize;
    let seed = arg::<Option<f64>>(args, "seed")?.unwrap_or(0.0).max(0.0) as u64;
    let lines = hub_core::list_lines(&arg::<String>(args, "map")?, &super::date_arg(args)?)?;
    Ok(shifts_of(&lines, minutes, daypart, seed, &legs))
}

/// The shifts of `minutes` length beginning in the part of the day `daypart`, from the day's
/// `lines`; `seed` draws another set ("Find other shifts"). The shift `legs` (the one chosen
/// last) comes first when it still holds and the draw left it out, so the choice stays in the
/// list.
pub fn shifts_of(lines: &[LineInfo], minutes: i64, daypart: usize, seed: u64, legs: &[String]) -> Value {
    let window = DAYPARTS.get(daypart).and_then(|d| d.1);
    let wish = Wish { seconds: minutes.clamp(30, 480) as f64 * 60.0, from: window.map(|w| w.0 as f64 * 60.0), to: window.map(|w| w.1 as f64 * 60.0), lines: Vec::new(), seed };
    let mut duties = hub_core::compose::compose(lines, &wish, SHIFTS);
    if !legs.is_empty() && !duties.iter().any(|d| d.blocks().iter().map(Block::arg).collect::<Vec<_>>() == legs) {
        if let Some(d) = shift_of_legs(legs, lines) {
            duties.insert(0, d);
        }
    }
    json!(duties.iter().map(|d| shift_json(d, lines)).collect::<Vec<_>>())
}

/// The shift `legs` stands for, read back from the day's timetable (None when one of its
/// tours or trips is not in it).
pub fn shift_of_legs(legs: &[String], lines: &[LineInfo]) -> Option<ComposedDuty> {
    let mut out = ComposedDuty::default();
    for a in legs {
        let (line, tour, first, count) = Block::parse(a)?;
        let t = lines.iter().find(|x| x.name == line)?.tours.iter().find(|t| t.number == tour)?;
        let trips: Vec<_> = t.trips.iter().filter(|x| x.index >= first && x.index < first + count).collect();
        if trips.len() != count {
            return None;
        }
        for x in trips {
            let shown = if x.line.is_empty() { line.clone() } else { x.line.clone() };
            out.legs.push(Leg {
                line: line.clone(),
                tour: tour.clone(),
                index: x.index,
                trip: x.name.clone(),
                shown,
                from: x.from.clone(),
                terminus: x.terminus.clone(),
                departure: x.departure,
                arrival: x.arrival,
            });
        }
    }
    (!out.legs.is_empty()).then_some(out)
}

/// A shift as the page lists it: its parts (`key`), when it begins and ends, its lines, and
/// its trips - each with how many stops it has, the break before it and, at the first trip of
/// another tour, the tour the bus goes on with (`takes_over`).
pub fn shift_json(d: &ComposedDuty, lines: &[LineInfo]) -> Value {
    let trips: Vec<Value> = d
        .legs
        .iter()
        .enumerate()
        .map(|(k, leg)| {
            let prev = k.checked_sub(1).map(|j| &d.legs[j]);
            let same_tour = prev.is_some_and(|p| p.line == leg.line && p.tour == leg.tour);
            let stops = lines
                .iter()
                .find(|x| x.name == leg.line)
                .and_then(|x| x.tours.iter().find(|t| t.number == leg.tour))
                .and_then(|t| t.trips.iter().find(|x| x.index == leg.index))
                .map_or(0, |t| t.stops.len());
            json!({
                "departure": leg.departure, "arrival": leg.arrival, "line": leg.shown, "from": leg.from, "terminus": leg.terminus,
                "stops": stops, "pause": prev.map_or(0.0, |p| (leg.departure - p.arrival).max(0.0)),
                "takes_over": (prev.is_some() && !same_tour).then(|| json!([leg.line, leg.tour])),
                "tour_line": leg.line, "tour": leg.tour, "index": leg.index, "trip": leg.trip,
            })
        })
        .collect();
    let blocks = d.blocks();
    let first = blocks.first().map(|b| json!({ "line": b.line, "tour": b.tour, "index": b.first, "departure": b.departure }));
    json!({ "key": blocks.iter().map(Block::arg).collect::<Vec<_>>(), "start": d.start(), "end": d.end(), "seconds": d.seconds(), "lines": d.lines(), "changes": d.changes(), "first": first, "trips": trips })
}

/// What a duty of ours adds: a duty of the timetable starts `bus_lead` minutes (a setting, five
/// by default) before its first departure at `minutes` - the bus stands ready, the player is
/// not late at once - and a free drive when it was asked to; and the display font chosen for
/// `bus`. `{ date, time: "HH:MM", lead, display_font }`.
pub fn duty_extras(args: &Value) -> Answer {
    let bus = super::str_arg(args, "bus");
    let date = super::date_arg(args)?;
    let minutes = arg::<Option<f64>>(args, "minutes")?.unwrap_or(0.0).round() as i32;
    let free = arg::<Option<bool>>(args, "free")?.unwrap_or(false);
    let settings = hub_core::settings::Settings::load();
    let lead = if free { 0 } else { hub_core::bus_lead(settings.other.get("bus_lead").and_then(Value::as_i64)) };
    Ok(extras(&bus, &date, minutes, lead, hub_core::busfonts::BusFonts::load().font_for(&bus)))
}

fn extras(_bus: &str, date: &str, minutes: i32, lead: i64, font: Option<String>) -> Value {
    // (a night tour's departure past midnight - 24:30 - starts on the next day: wrapped to the
    // day's 00:30 it started a day early)
    let (date, time) = hub_core::lead_start(date, minutes, lead);
    json!({ "date": date, "time": format!("{:02}:{:02}", time / 60, time % 60), "lead": lead, "display_font": font })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_core::{TourInfo, TripInfo};

    fn lines() -> Vec<LineInfo> {
        let trip = |index: usize, dep: f64, arr: f64, from: &str, to: &str| TripInfo {
            name: format!("t{index}"),
            index,
            line: "5".into(),
            from: from.into(),
            terminus: to.into(),
            departure: dep,
            arrival: arr,
            stops: Vec::new(),
            km: 0.0,
        };
        let tour = TourInfo {
            number: "2".into(),
            first: 3600.0,
            last: 9000.0,
            days: "daily".into(),
            runs: true,
            trips: vec![trip(1, 3600.0, 5400.0, "A", "B"), trip(2, 5700.0, 7500.0, "B", "A"), trip(3, 7800.0, 9000.0, "A", "B")],
            ..Default::default()
        };
        vec![LineInfo { name: "Linie5".into(), user_allowed: true, termini: Vec::new(), tours: vec![tour] }]
    }

    #[test]
    fn a_shift_read_back_from_its_legs() {
        let lines = lines();
        let d = shift_of_legs(&["Linie5|2|1|2".to_string()], &lines).unwrap();
        assert_eq!(d.legs.len(), 2);
        assert_eq!((d.start(), d.end()), (3600.0, 7500.0));
        let v = shift_json(&d, &lines);
        assert_eq!(v["key"], json!(["Linie5|2|1|2"]));
        assert_eq!(v["trips"][1]["pause"], json!(300.0));
        // (a later trip of the same tour is no change of tour)
        assert_eq!(v["trips"][1]["takes_over"], Value::Null);
        assert_eq!(v["first"]["index"], 1);
        // a trip that is not in the timetable: no shift
        assert!(shift_of_legs(&["Linie5|2|2|5".to_string()], &lines).is_none());
    }

    #[test]
    fn the_shifts_of_a_day_keep_the_one_chosen() {
        let lines = lines();
        let v = shifts_of(&lines, 120, 0, 7, &["Linie5|2|2|2".to_string()]);
        let list = v.as_array().unwrap();
        assert!(!list.is_empty());
        assert!(list.iter().any(|s| s["key"] == json!(["Linie5|2|2|2"])), "{v}");
        for s in list {
            assert!(s["trips"].as_array().is_some_and(|t| !t.is_empty()));
            assert!(s["start"].as_f64() <= s["end"].as_f64());
        }
        assert_eq!(DAYPARTS[4].1, Some((22 * 60, 30 * 60)));
    }

    #[test]
    fn the_bus_stands_ready_before_the_first_departure() {
        assert_eq!(extras("b", "1989-05-30", 8 * 60 + 15, 5, None), json!({ "date": "1989-05-30", "time": "08:10", "lead": 5, "display_font": null }));
        let v = extras("b", "1989-05-30", 3, 10, Some("Matrix".into()));
        assert_eq!((v["date"].as_str(), v["time"].as_str(), v["display_font"].as_str()), (Some("1989-05-29"), Some("23:53"), Some("Matrix")));
        // (a departure past midnight of the service day: the next day's early hours)
        let v = extras("b", "1989-05-30", 24 * 60 + 30, 5, None);
        assert_eq!((v["date"].as_str(), v["time"].as_str()), (Some("1989-05-31"), Some("00:25")));
    }
}
