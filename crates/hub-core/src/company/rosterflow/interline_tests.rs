//! Interlining (a tester, 2026-10-19: "all routes can interline together"): Optimise chains trips
//! of different lines onto one bus where they interline, a trip moves by hand onto a bus of
//! another line, the day and the game's plan keep each trip's own line, and an old save loads as
//! it was - every bus keeping to its line.

use super::super::studio::{self as st, Target};
use super::super::{found, Founding};
use super::*;
use crate::company::model::CompanyLine;
use crate::{StopInfo, TourInfo, TripInfo};

/// A trip of line number `num` from `a` to `b`, `dep` to `arr` (minutes), with passengers.
fn tt(num: &str, name: &str, a: &str, b: &str, dep: i32, arr: i32) -> TripInfo {
    TripInfo {
        name: name.into(),
        index: 0,
        line: num.into(),
        from: a.into(),
        terminus: b.into(),
        departure: dep as f64 * 60.0,
        arrival: arr as f64 * 60.0,
        stops: (0..6)
            .map(|s| StopInfo {
                name: match s {
                    0 => a.to_string(),
                    5 => b.to_string(),
                    _ => format!("{num} S{s}"),
                },
                id: s,
                arr: 0.0,
                dep: 0.0,
            })
            .collect(),
        km: (arr - dep) as f64 * 0.4,
    }
}

fn tour(no: &str, trips: Vec<TripInfo>) -> TourInfo {
    TourInfo { number: no.into(), ai_group: String::new(), first: 0.0, last: 0.0, days: "daily".into(), runs: true, next_run: None, trips }
}

/// Line 10 out from A to B and back in the morning (06:00 to 07:30), line 20 from A to C and
/// back after it (`y_from` its first stop, from 07:40): by the timetable a bus each.
fn lines_from(y_from: &str) -> Vec<LineInfo> {
    let x = LineInfo {
        name: "LinieX".into(),
        user_allowed: true,
        termini: vec!["A".into(), "B".into()],
        tours: vec![tour("1", vec![tt("10", "10 1-0", "A", "B", 360, 400), tt("10", "10 1-1", "B", "A", 410, 450)])],
    };
    let y = LineInfo {
        name: "LinieY".into(),
        user_allowed: true,
        termini: vec![y_from.into(), "C".into()],
        tours: vec![tour("1", vec![tt("20", "20 1-0", y_from, "C", 460, 500), tt("20", "20 1-1", "C", y_from, 510, 550)])],
    };
    vec![x, y]
}

fn lines() -> Vec<LineInfo> {
    lines_from("A")
}

fn company(interline: bool) -> Company {
    let mut c = found(&Founding { name: "Flow".into(), date: "2024-03-04".into(), ..Default::default() }, "Luc");
    c.game_mode = super::super::GameMode::Normal;
    for (name, number) in [("LinieX", "10"), ("LinieY", "20")] {
        c.lines.push(CompanyLine { name: name.into(), number: number.into(), numbers: vec![number.into()], added: c.date.clone(), service_from: Some(0), ..Default::default() });
    }
    c.planning.flow.prefs.interline = interline;
    c
}

/// The company with what Optimise made of its day kept.
fn optimised(c: &Company, l: &[LineInfo]) -> Company {
    let (layouts, joined) = optimise_with(c, l, &c.date, true, true);
    let mut v = c.clone();
    let wd = super::super::plandays::slot_of(c, &c.date);
    apply(&mut v, wd, layouts, joined);
    v
}

fn day(c: &Company, l: &[LineInfo]) -> Vec<TourOfDay> {
    network::tours_of_day(c, l, &c.date)
}

/// The day's buses: each one's lines.
fn buses(c: &Company, l: &[LineInfo]) -> Vec<Vec<String>> {
    day(c, l).iter().map(|t| t.lines().iter().map(|x| x.to_string()).collect()).collect()
}

#[test]
fn optimise_chains_trips_of_two_lines_onto_one_bus_where_they_interline() {
    let l = lines();
    // off (as every company was): a bus a line
    let off = optimised(&company(false), &l);
    assert_eq!(buses(&off, &l), vec![vec!["LinieX".to_string()], vec!["LinieY".to_string()]]);
    // on: line 20 leaves A ten minutes after line 10 is back there - one bus runs both
    let c = company(true);
    let on = optimised(&c, &l);
    assert_eq!(buses(&on, &l), vec![vec!["LinieX".to_string(), "LinieY".to_string()]]);
    let t = &day(&on, &l)[0];
    assert!(t.interlined());
    assert_eq!(t.trips.iter().filter(|x| x.counts()).map(|x| t.line_of(x)).collect::<Vec<_>>(), ["LinieX", "LinieX", "LinieY", "LinieY"]);
    // kept with line 10's layout, line 20's trips naming their line; line 20's own has no bus
    let wd = super::super::plandays::slot_of(&on, &on.date);
    let lay = |line: &str| on.planning.layouts.iter().find(|x| x.weekday == wd && x.line == line).cloned().unwrap();
    assert_eq!(lay("LinieX").blocks.len(), 1);
    assert_eq!(lay("LinieX").blocks[0].trips.iter().map(|r| r.line.as_str()).collect::<Vec<_>>(), ["", "", "LinieY", "LinieY"]);
    assert!(lay("LinieY").blocks.is_empty());
    let b = board(&on, &l, &on.date);
    assert_eq!((b.figures.blocks, b.figures.pvr, b.figures.assigned), (1, 1, 4));
    assert!(!b.issues.iter().any(|i| i.text == I_INTERLINE));
    assert!(st::stale(&on, &l, &on.date).is_empty());
    // and Optimise weighs it the cheaper day: one bus fewer
    let o = optimise_day(&c, &l, &c.date);
    assert!(o.better && o.board.figures.blocks == 1, "{}", o.board.figures.blocks);
}

#[test]
fn a_line_kept_to_its_own_buses_does_not_interline() {
    let l = lines();
    let mut c = company(true);
    c.planning.flow.prefs.interline_not = vec!["linieY".into()];
    assert!(!interlines(&c, "LinieX", "LinieY") && interlines(&c, "LinieY", "LinieY"));
    assert_eq!(optimised(&c, &l).planning.layouts.iter().map(|x| x.blocks.len()).sum::<usize>(), 2);
    // a bus that interlined before is said to, once its lines may not
    let on = optimised(&company(true), &l);
    let mut now = on.clone();
    now.planning.flow.prefs.interline = false;
    let b = board(&now, &l, &now.date);
    let i = b.issues.iter().find(|i| i.text == I_INTERLINE).expect("said");
    assert!(!i.hard && i.args[0].1 == "10, 20", "{:?}", i.args);
}

#[test]
fn interlining_runs_empty_only_as_far_as_the_preferences_allow() {
    // line 20 leaves from B: the bus runs empty from A, as long as line 10's trip there (40')
    let l = lines_from("B");
    let mut c = company(true);
    assert_eq!(c.planning.flow.prefs.interline_deadhead, 15);
    assert_eq!(optimised(&c, &l).planning.layouts.iter().map(|x| x.blocks.len()).sum::<usize>(), 2, "too far");
    c.planning.flow.prefs.interline_deadhead = 0;
    assert_eq!(optimised(&c, &l).planning.layouts.iter().map(|x| x.blocks.len()).sum::<usize>(), 2);
    // (the 07:30 arrival at A, 40 minutes to B: there at 08:10, line 20 leaves B at 07:40 - no;
    // half an hour later it fits)
    let late = {
        let mut l = lines_from("B");
        for t in l[1].tours[0].trips.iter_mut() {
            t.departure += 60.0 * 60.0;
            t.arrival += 60.0 * 60.0;
        }
        l
    };
    c.planning.flow.prefs.interline_deadhead = 45;
    assert_eq!(optimised(&c, &late).planning.layouts.iter().map(|x| x.blocks.len()).sum::<usize>(), 1);
    // a relief point the two stops are one of: no run empty at all (a stop's names for arriving
    // and leaving)
    let mut c = company(true);
    c.planning.flow.prefs.interline_deadhead = 0;
    assert_eq!(interline_gap(&c, &day(&c, &l), &[], "A", "a "), Some(0));
    assert_eq!(interline_gap(&c, &day(&c, &l), &[], "A", "Z"), None);
}

#[test]
fn a_trip_moves_by_hand_onto_a_bus_of_another_line_only_where_they_interline() {
    let l = lines();
    let wd = |c: &Company| super::super::plandays::slot_of(c, &c.date);
    let off = company(false);
    let tours = day(&off, &l);
    let (x, y) = (tours.iter().position(|t| t.line == "LinieX").unwrap(), tours.iter().position(|t| t.line == "LinieY").unwrap());
    let first_y = tours[y].trips.iter().position(|t| t.counts()).unwrap();
    assert_eq!(st::plan_move(&off, wd(&off), &tours, y, first_y, Target::Block(x)), Err(st::OTHER_LINE));
    // on: line 20's first trip onto line 10's bus, both lines' layouts kept
    let mut c = company(true);
    let w = wd(&c);
    let lays = st::plan_move(&c, w, &tours, y, first_y, Target::Block(x)).unwrap();
    assert_eq!(lays.len(), 2);
    st::move_trip(&mut c, w, &tours, y, first_y, Target::Block(x)).unwrap();
    let now = day(&c, &l);
    let bx = now.iter().find(|t| t.line == "LinieX").unwrap();
    assert_eq!(bx.lines(), ["LinieX", "LinieY"]);
    assert_eq!(bx.trips.iter().filter(|t| t.counts()).count(), 3);
    assert_eq!(now.iter().find(|t| t.line == "LinieY").unwrap().trips.iter().filter(|t| t.counts()).count(), 1);
    assert!(st::stale(&c, &l, &c.date).is_empty());
    // and back again: line 10's bus its own line's alone
    let i = now.iter().position(|t| t.line == "LinieX").unwrap();
    let k = now[i].trips.iter().position(|t| t.counts() && t.of_line == "LinieY").unwrap();
    let j = now.iter().position(|t| t.line == "LinieY").unwrap();
    st::move_trip(&mut c, w, &now, i, k, Target::Block(j)).unwrap();
    let back = day(&c, &l);
    assert!(back.iter().all(|t| !t.interlined()), "{:?}", back.iter().map(|t| t.lines()).collect::<Vec<_>>());
    assert!(c.planning.layouts.iter().flat_map(|x| x.blocks.iter()).flat_map(|b| b.trips.iter()).all(|r| r.line.is_empty()));
    // taken off the bus that interlines, a trip waits on its own line's stack
    st::move_trip(&mut c, w, &back, j, back[j].trips.iter().position(|t| t.counts()).unwrap(), Target::Block(i)).unwrap();
    let mixed = day(&c, &l);
    let i = mixed.iter().position(|t| t.interlined()).unwrap();
    let k = mixed[i].trips.iter().position(|t| t.counts() && t.of_line == "LinieY").unwrap();
    st::unassign(&mut c, w, &mixed, i, &[k]).unwrap();
    let lay = |line: &str| c.planning.layouts.iter().find(|x| x.weekday == w && x.line == line).cloned().unwrap();
    assert_eq!(lay("LinieY").stack.len(), 1);
    assert!(lay("LinieX").stack.is_empty());
    assert_eq!(stack(&c, &l, w).iter().map(|(line, _)| line.as_str()).collect::<Vec<_>>(), ["LinieY"]);
}

#[test]
fn a_swap_between_buses_of_two_lines_keeps_each_trip_with_its_line() {
    let l = lines();
    let mut c = company(true);
    let w = super::super::plandays::slot_of(&c, &c.date);
    let tours = day(&c, &l);
    let (x, y) = (tours.iter().position(|t| t.line == "LinieX").unwrap(), tours.iter().position(|t| t.line == "LinieY").unwrap());
    let k = tours[x].trips.iter().position(|t| t.counts()).unwrap();
    st::swap(&mut c, w, &tours, x, &[k], y, st::Swap::FromHere).unwrap();
    // (every trip of line 10 from its first on to line 20's bus, and line 20's from then on back)
    let now = day(&c, &l);
    let by = |line: &str| now.iter().find(|t| t.line == line).unwrap();
    assert_eq!(by("LinieY").lines(), ["LinieY", "LinieX"]);
    assert_eq!(by("LinieX").lines(), ["LinieX", "LinieY"]);
    assert!(st::stale(&c, &l, &c.date).is_empty());
}

#[test]
fn a_layout_that_ran_another_lines_trips_goes_with_it_once_it_no_longer_fits() {
    let l = lines();
    let on = optimised(&company(true), &l);
    // line 10's timetable changed: its layout does not fit, so line 20's - which left its trips
    // to line 10's bus - does not either, and both run as the timetable has them
    let mut changed = l.clone();
    changed[0].tours[0].trips[1].departure += 60.0;
    changed[0].tours[0].trips[1].arrival += 60.0;
    let mut gone = st::stale(&on, &changed, &on.date);
    gone.sort();
    assert_eq!(gone, ["LinieX", "LinieY"]);
    let t = day(&on, &changed);
    assert_eq!(t.len(), 2);
    assert!(t.iter().all(|t| !t.interlined()));
}

#[test]
fn the_game_runs_each_tour_of_a_bus_that_interlines_as_its_own_lines() {
    let l = lines();
    let on = optimised(&company(true), &l);
    let p = super::super::plan::day_plan(&on, &on.date, day(&on, &l), &[], &[], false);
    let lp = super::super::plan::live_plan(&on, &p);
    let mut runs: Vec<(String, String, String)> = lp.tours.iter().map(|t| (t.line.clone(), t.number.clone(), t.tour.clone())).collect();
    runs.sort();
    assert_eq!(runs, [("LinieX".into(), "10".into(), "1".into()), ("LinieY".into(), "20".into(), "1".into())]);
    // a report of line 20's tour 1 is that bus's, and line 10's tour 1 too - not a tour 2
    let t = &day(&on, &l)[0];
    assert!(t.runs_tour_of("20", "1") && t.runs_tour_of("LinieX", "1") && !t.runs_tour_of("20", "2") && !t.runs_tour_of("30", "1"));
}

#[test]
fn an_old_save_loads_with_every_bus_keeping_to_its_line() {
    let p: Prefs = serde_json::from_str("{}").unwrap();
    assert!(!p.interline && p.interline_not.is_empty() && p.interline_deadhead == 15);
    let r: st::TripRef = serde_json::from_str(r#"{"tour":"1","name":"10 1-0","dep":360}"#).unwrap();
    assert!(r.line.is_empty() && r.line_in("LinieX") == "LinieX");
    // (written as before where it is of its layout's line)
    assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"tour":"1","name":"10 1-0","dep":360}"#);
    let x: network::PlannedTrip = serde_json::from_str(r#"{"name":"t","line":"10","from":"A","to":"B","dep":1,"arr":2,"km":1.0,"stops":4}"#).unwrap();
    assert!(x.of_line.is_empty());
    // (a ref names its line only where it is another; equal whatever line it names)
    let mut other = r.clone();
    other.line = "LinieY".into();
    assert_eq!(r, other);
    // an old layout of each line: as it was
    let l = lines();
    let mut c = company(false);
    let w = super::super::plandays::slot_of(&c, &c.date);
    let tours = day(&c, &l);
    for line in ["LinieX", "LinieY"] {
        let blocks = tours.iter().filter(|t| t.line == line).map(st::layout_of_block).collect();
        c.planning.layouts.push(st::LineLayout { weekday: w, line: line.into(), blocks, stack: Vec::new() });
    }
    // (its blocks' trips written as before: no line of their own)
    let json = serde_json::to_string(&c.planning.layouts.iter().map(|x| &x.blocks).collect::<Vec<_>>()).unwrap();
    assert!(!json.contains("\"line\""), "{json}");
    assert_eq!(buses(&c, &l), vec![vec!["LinieX".to_string()], vec!["LinieY".to_string()]]);
    assert!(st::stale(&c, &l, &c.date).is_empty());
}
