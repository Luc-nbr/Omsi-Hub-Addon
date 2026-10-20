//! The live dashboard (Luc, 2026-10-19): what Omsi-Hub's window shows while a drive it started
//! runs - a shift, a tour, a free drive and a company's duty alike. `hub_live` answers it
//! ([`view`]); the page asks every second or two.
//!
//! * The drive is kept as it began ([`began`], from `drive::start`): the duty, the plugin's
//!   assignment (a company's sheet: signing on and off, the breaks), where the plugin lies.
//! * The plugin writes `data/dash.json` every two seconds ([`bridge::Dash`], `lua/dash.lua`):
//!   the bus, the duty under way and its trip's stations, what the drive came to so far. The
//!   bridge's loop takes a sample of it each time it looks ([`sample`]): the speed, the delay and
//!   the passengers over the drive, the way the bus went, where it stood at its stops.
//! * The duty's plan (its trips in order with their times, the lengths between their stops) is
//!   read from the map's timetable of the day once, on a thread of its own as the drive begins.
//! * The week's and month's figures come from the driver's trip records (`trips/<driver>.jsonl`).
//! * What the driver types into the IBIS for each trip (Luc, 2026-10-20): read with the plan from
//!   the depot file the bus reads (`--hof`, else the map's standard one; [`hub_core::ibis`]),
//!   and set against what the plugin says the bus's IBIS shows.
//!
//! The view says what is so; the page words it (its alerts are kinds and numbers, or the
//! company's own words).

use hub_core::ibis::TripIbis;
use hub_core::trips::TripRun;
use hub_core::{Duty, LineInfo};
use hub_game::bridge::{self, Assignment, Dash, Inbox, Notice, SheetRow};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// What the plugin wrote longer ago than this (s) is of a game not driven now.
const FRESH: f64 = 30.0;
/// The samples and the way kept for a drive: when there are more, every other one goes.
const SAMPLES: usize = 1800;
const TRAIL: usize = 2400;
/// The points of the way the view sends at the most.
const TRAIL_VIEW: usize = 600;
/// The next stops the view lists.
const NEXT_STOPS: usize = 4;
/// A gap this long between two trips of a tour or shift is a layover (s).
const LAYOVER: f64 = 300.0;

/// A trip of the duty as planned.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PlanTrip {
    /// The timetable's line and tour, the trip's place in the tour (from 1) and its file.
    pub line: String,
    pub tour: String,
    pub index: usize,
    pub name: String,
    /// The line its displays show, its first and last stop.
    pub number: String,
    pub from: String,
    pub terminus: String,
    /// Seconds of the day (a later day's above 86 400).
    pub departure: f64,
    pub arrival: f64,
    pub km: f64,
}

/// One look at the drive.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Sample {
    clock: f64,
    speed: f64,
    delay: Option<f64>,
    passengers: Option<f64>,
}

/// A notice or message the driver was told, kept with the clock it came at.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
struct Told {
    id: String,
    kind: String,
    title: String,
    text: String,
    at: Option<f64>,
}

#[derive(Default)]
struct Drive {
    duty: Duty,
    assignment: Option<Assignment>,
    plugin_dir: Option<PathBuf>,
    /// Unix seconds it began.
    began: f64,
    /// The plan and the timetable it came from (read once).
    plan: Option<Arc<Plan>>,
    planning: bool,
    samples: Vec<Sample>,
    trail: Vec<[f64; 2]>,
    /// Where the bus stood at a stop, by its name.
    stood: HashMap<String, [f64; 2]>,
    told: Vec<Told>,
    /// The first trip of the duty driven (line, tour, place in the tour).
    first: Option<(String, String, u32)>,
    last_at: f64,
    bus_name: Option<String>,
}

#[derive(Default)]
struct Plan {
    trips: Vec<PlanTrip>,
    /// The metres from a trip's first station to each of its stations, by the trip's file.
    metres: HashMap<String, Vec<f64>>,
    /// The depot file the bus reads, and each trip's IBIS from it (by the trip's file).
    depot: Depot,
    ibis: HashMap<String, TripIbis>,
}

/// The depot file the bus reads: the name it goes by and the file found for it (none: the bus
/// has no such file, and its IBIS knows no destination).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Depot {
    pub name: String,
    pub file: Option<String>,
}

static DRIVE: Mutex<Option<Drive>> = Mutex::new(None);

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

fn drive() -> std::sync::MutexGuard<'static, Option<Drive>> {
    DRIVE.lock().unwrap_or_else(|e| e.into_inner())
}

/// A drive began (`drive::start`): what it is, the plugin's assignment for it (none: the
/// plugin is off) and the plugin's folder.
pub fn began(d: &Duty, a: Option<&Assignment>, plugin_dir: Option<&Path>) {
    let told = a.map(|a| a.notices.iter().map(|n| told_of(n, None)).collect()).unwrap_or_default();
    *drive() = Some(Drive { duty: d.clone(), assignment: a.cloned(), plugin_dir: plugin_dir.map(Path::to_path_buf), began: now(), told, ..Drive::default() });
    // (the plan read while the game loads, not in the page's first `hub_live`)
    std::thread::spawn(ensure_plan);
}

/// How the drive is driven: `company`, `shift`, `tour` or `free`.
pub fn mode_of(d: &Duty, a: Option<&Assignment>) -> &'static str {
    let company = a.is_some_and(|a| !a.company.trim().is_empty()) || d.company.as_deref().is_some_and(|c| !c.trim().is_empty());
    if company {
        "company"
    } else if d.free_line.as_deref().is_some_and(|f| !f.trim().is_empty()) {
        "free"
    } else if !d.legs.is_empty() {
        "shift"
    } else if d.line.as_deref().is_some_and(|l| !l.trim().is_empty()) && d.tour.as_deref().is_some_and(|t| !t.trim().is_empty()) {
        "tour"
    } else {
        "free"
    }
}

fn told_of(n: &Notice, at: Option<f64>) -> Told {
    let kind = match n.kind.as_str() {
        "alert" => "alert",
        "warning" => "warning",
        _ => "info",
    };
    Told { id: format!("notice:{}", n.id), kind: kind.into(), title: n.title.clone(), text: n.text.clone(), at }
}

/// The plugin's dashboard, when it is of this drive and fresh.
fn fresh_dash(dir: &Path, began: f64) -> Option<Dash> {
    let path = dir.join(bridge::DATA_DIR).join(bridge::DASH_FILE);
    let written = std::fs::metadata(&path).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs_f64();
    let t = now();
    if t - written > FRESH || written + 2.0 < began {
        return None;
    }
    bridge::read_dash(dir)
}

/// Every other one of `v`, when it holds more than `most`.
fn thin<T: Copy>(v: &mut Vec<T>, most: usize) {
    if v.len() > most {
        *v = v.iter().step_by(2).copied().collect();
    }
}

/// One look at the drive (the bridge's loop, every two seconds): a sample of the plugin's
/// dashboard, the way the bus went, the notices and messages that came.
pub fn sample() {
    let (dir, began) = match drive().as_ref() {
        Some(d) => (d.plugin_dir.clone(), d.began),
        None => return,
    };
    let Some(dir) = dir else { return };
    let dash = fresh_dash(&dir, began);
    let notices = read_json::<Vec<Notice>>(&dir, bridge::NOTICES_FILE).unwrap_or_default();
    let inbox = read_json::<Inbox>(&dir, bridge::MESSAGES_FILE);
    let mut g = drive();
    let Some(d) = g.as_mut() else { return };
    let clock = dash.as_ref().and_then(|x| x.clock);
    for n in &notices {
        let t = told_of(n, clock);
        if !d.told.iter().any(|x| x.id == t.id) {
            d.told.push(t);
        }
    }
    for m in inbox.iter().flat_map(|i| &i.messages).filter(|m| !m.mine) {
        let id = format!("message:{}", m.id);
        if d.told.iter().any(|x| x.id == id) {
            continue;
        }
        let kind = match m.priority.as_str() {
            "urgent" => "alert",
            "instruction" => "warning",
            _ => "info",
        };
        let at = hhmm_secs(&m.time).or(clock);
        d.told.push(Told { id, kind: kind.into(), title: m.from.clone(), text: m.text.clone(), at });
    }
    let Some(x) = dash else { return };
    if x.at > 0.0 && x.at <= d.last_at {
        return;
    }
    d.last_at = x.at;
    take(d, &x);
}

/// A sample of `x` into the drive `d`.
fn take(d: &mut Drive, x: &Dash) {
    if let Some(clock) = x.clock {
        d.samples.push(Sample { clock, speed: x.bus.speed, delay: x.duty.as_ref().map(|u| u.delay), passengers: x.bus.passengers });
        thin(&mut d.samples, SAMPLES);
    }
    if let (Some(px), Some(py)) = (x.bus.x, x.bus.y) {
        let moved = d.trail.last().is_none_or(|l| (l[0] - px).hypot(l[1] - py) >= 4.0);
        if moved {
            d.trail.push([px, py]);
            thin(&mut d.trail, TRAIL);
        }
        if let Some(u) = x.duty.as_ref().filter(|u| u.at_stop) {
            if let Some(s) = x.stops.get(u.next) {
                d.stood.insert(s.name.clone(), [px, py]);
            }
        }
    }
    if let Some(u) = &x.duty {
        if d.first.is_none() && u.trip > 0 {
            d.first = Some((u.line.clone(), u.tour.clone(), u.trip));
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(dir: &Path, file: &str) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(dir.join(bridge::DATA_DIR).join(file)).ok()?).ok()
}

/// `"14:05"` as seconds of the day.
fn hhmm_secs(t: &str) -> Option<f64> {
    let (h, m) = t.trim().split_once(':')?;
    Some(h.trim().parse::<f64>().ok()? * 3600.0 + m.trim().parse::<f64>().ok()? * 60.0)
}

// --- the plan --------------------------------------------------------------------------------

/// The duty's trips in order: a shift's or a company's parts (`legs`), else the tour from its
/// first trip on (none on a free drive).
pub fn plan_of(d: &Duty, a: Option<&Assignment>, lines: &[LineInfo]) -> Vec<PlanTrip> {
    let mut parts: Vec<(String, String, u32, Option<u32>)> = Vec::new();
    match a.filter(|a| !a.legs.is_empty()) {
        Some(a) => parts.extend(a.legs.iter().map(|l| (l.line.clone(), l.tour.clone(), l.first.max(1), Some(l.trips.max(1))))),
        None => {
            let legs: Vec<_> = d.legs.iter().filter_map(|l| crate::addon::leg_of(l)).collect();
            if !legs.is_empty() {
                parts.extend(legs.into_iter().map(|l| (l.line, l.tour, l.first.max(1), Some(l.trips.max(1)))));
            } else if mode_of(d, a) != "free" {
                let first = a.map(|a| a.trip).filter(|t| *t > 0).or_else(|| d.trip.as_deref().and_then(|t| t.trim().parse().ok())).unwrap_or(1);
                parts.push((d.line.clone().unwrap_or_default(), d.tour.clone().unwrap_or_default(), first.max(1), None));
            }
        }
    }
    let mut out = Vec::new();
    for (line, tour, first, count) in parts {
        let Some(t) = lines.iter().find(|l| l.name == line).and_then(|l| l.tours.iter().find(|t| t.number == tour)) else { continue };
        let last = count.map_or(usize::MAX, |c| (first + c - 1) as usize);
        for x in t.trips.iter().filter(|x| x.index >= first as usize && x.index <= last) {
            out.push(PlanTrip {
                line: line.clone(),
                tour: tour.clone(),
                index: x.index,
                name: x.name.clone(),
                number: x.line.clone(),
                from: x.from.clone(),
                terminus: x.terminus.clone(),
                departure: x.departure,
                arrival: x.arrival,
                km: x.km,
            });
        }
    }
    out
}

/// The metres from the first station of `trip` to each of its stations (a leg nobody knows the
/// length of taken as long as the others' average).
fn metres_of(tt: &hub_omsi::timetable::Timetable, trip: &str) -> Option<Vec<f64>> {
    let t = tt.trip(trip)?;
    let legs = hub_omsi::timetable::leg_lengths(tt, t);
    let known: Vec<f64> = legs.iter().flatten().copied().collect();
    if known.is_empty() {
        return None;
    }
    let avg = known.iter().sum::<f64>() / known.len() as f64;
    let mut at = 0.0;
    let mut out = vec![0.0];
    for l in legs {
        at += l.unwrap_or(avg);
        out.push(at);
    }
    Some(out)
}

/// The drive's plan, read from the map's timetable of the day once (outside the lock: it
/// takes a moment on a large map).
fn ensure_plan() {
    let (duty, a) = {
        let mut g = drive();
        let Some(d) = g.as_mut() else { return };
        if d.plan.is_some() || d.planning {
            return;
        }
        d.planning = true;
        (d.duty.clone(), d.assignment.clone())
    };
    let date = duty.date.clone().filter(|x| !x.trim().is_empty()).unwrap_or_else(|| hub_core::DEFAULT_DATE.to_string());
    let lines = hub_core::list_lines(&duty.map, &date).unwrap_or_default();
    let trips = plan_of(&duty, a.as_ref(), &lines);
    let mut metres = HashMap::new();
    let mut ibis = HashMap::new();
    let day = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
    let vfs = hub_core::paths::vfs();
    let tt = vfs.as_ref().zip(day).and_then(|(v, day)| hub_omsi::timetable::timetable_for(v, &duty.map, day).ok().flatten());
    let (depot, hof) = match &vfs {
        Some(v) => depot_of(v, &duty),
        None => (Depot::default(), None),
    };
    if let Some(tt) = &tt {
        let mut names: Vec<String> = trips.iter().map(|t| t.name.clone()).collect();
        if let Some(f) = duty.free_line.as_deref().map(str::trim).filter(|f| !f.is_empty()) {
            names.push(f.to_string());
        }
        for n in names {
            if let Some(m) = metres_of(tt, &n) {
                metres.insert(n.clone(), m);
            }
            if let Some(i) = hof.as_ref().and_then(|h| ibis_of(tt, h, &n)) {
                ibis.insert(n, i);
            }
        }
    }
    let bus_name = hub_core::paths::vfs().and_then(|v| v.resolve(&duty.bus)).and_then(|p| hub_omsi::vehicle::Bus::load(&p).ok()).map(|b| hub_core::display_bus_name(&b.name()));
    let mut g = drive();
    if let Some(d) = g.as_mut() {
        d.plan = Some(Arc::new(Plan { trips, metres, depot, ibis }));
        d.planning = false;
        d.bus_name = bus_name;
    }
}

/// The depot file the bus of `d` reads: the one it was started with (`--hof`), else the map's
/// standard one - found as the game looks for it: in the bus's folder, else its pack's, else
/// beside another bus.
fn depot_of(vfs: &hub_omsi::Vfs, d: &Duty) -> (Depot, Option<hub_core::linehof::Depot>) {
    let given = d.hof.as_deref().map(str::trim).filter(|h| !h.is_empty()).map(str::to_string);
    let name = given.or_else(|| hub_core::paths::resolve_content(&d.map).and_then(|g| hub_omsi::map::GlobalCfg::load(&g).ok()).and_then(|g| g.standard_depot)).unwrap_or_default();
    let name = name.strip_suffix(".hof").or_else(|| name.strip_suffix(".HOF")).unwrap_or(&name).trim().to_string();
    let bus = d.bus.replace('\\', "/");
    let own = bus.rsplit_once('/').map(|(dir, _)| dir.to_string()).unwrap_or_default();
    let pack = bus.split('/').take(2).collect::<Vec<_>>().join("/");
    let hofs = |dir: &str| -> Vec<(String, std::path::PathBuf)> {
        vfs.list_dir(dir)
            .into_iter()
            .filter(|(n, is_dir)| !is_dir && n.to_lowercase().ends_with(".hof"))
            .filter_map(|(n, _)| vfs.resolve(&format!("{dir}/{n}")).map(|p| (format!("{dir}/{n}"), p)))
            .collect()
    };
    let pick = |list: Vec<(String, std::path::PathBuf)>| {
        if name.is_empty() {
            // (no name anywhere: the bus's only file)
            return (list.len() == 1).then(|| list.into_iter().next()).flatten();
        }
        list.into_iter().find(|(_, p)| hub_core::depot::answers_to(p, &name))
    };
    let mut found = pick(hofs(&own));
    if found.is_none() && pack != own {
        found = pick(hofs(&pack));
    }
    if found.is_none() && !name.is_empty() {
        found = vfs.list_dir("Vehicles").into_iter().filter(|(_, is_dir)| *is_dir).find_map(|(f, _)| pick(hofs(&format!("Vehicles/{f}"))));
    }
    let depot = found.as_ref().and_then(|(_, p)| hub_core::linehof::Depot::load(p, &d.map));
    let name = if name.is_empty() { depot.as_ref().map(|x| x.hof.name.trim().to_string()).unwrap_or_default() } else { name };
    (Depot { name, file: found.filter(|_| depot.is_some()).map(|(rel, _)| rel) }, depot)
}

/// The IBIS of the timetable's trip `name` from the depot file `hof`: its line, its target and
/// its stations' names, as the game types them.
fn ibis_of(tt: &hub_omsi::timetable::Timetable, hof: &hub_core::linehof::Depot, name: &str) -> Option<TripIbis> {
    let t = tt.trip(name)?;
    let names: Vec<String> = t.stops.iter().map(|s| if s.name.trim().is_empty() { tt.stop_name(s.id).unwrap_or("").to_string() } else { s.name.trim().to_string() }).collect();
    let stops: Vec<&str> = names.iter().map(String::as_str).collect();
    hub_core::ibis::trip_ibis(hof, &t.line, &t.terminus, &stops)
}

// --- the view --------------------------------------------------------------------------------

/// What the dashboard is worked out from.
pub struct Facts<'a> {
    pub duty: &'a Duty,
    pub assignment: Option<&'a Assignment>,
    pub dash: Option<&'a Dash>,
    pub plan: &'a [PlanTrip],
    pub metres: &'a HashMap<String, Vec<f64>>,
    pub first: Option<(String, String, u32)>,
    pub bus_name: Option<&'a str>,
    pub depot: &'a Depot,
    pub ibis: &'a HashMap<String, TripIbis>,
}

/// A clock of the day near `start`: one just after midnight is the next day's.
fn near(clock: f64, start: f64) -> f64 {
    if start > 43_200.0 && clock + 43_200.0 < start {
        clock + 86_400.0
    } else {
        clock
    }
}

fn sheet_row<'a>(a: Option<&'a Assignment>, kind: &str) -> Option<&'a SheetRow> {
    a?.sheet.iter().find(|r| r.kind == kind)
}

/// The duty's start and end (seconds of the day) and where it begins and ends.
fn span(f: &Facts, trips: &[PlanTrip]) -> (Option<f64>, Option<f64>, String, String) {
    let on = sheet_row(f.assignment, "sign_on");
    let off = f.assignment.and_then(|a| a.sheet.iter().rev().find(|r| r.kind == "sign_off"));
    let start = on.map(|r| r.from as f64 * 60.0).or_else(|| trips.first().map(|t| t.departure));
    let end = off.map(|r| r.to as f64 * 60.0).or_else(|| trips.last().map(|t| t.arrival));
    let from = on.map(|r| r.at.clone()).filter(|x| !x.is_empty()).or_else(|| trips.first().map(|t| t.from.clone())).unwrap_or_default();
    let to = off.map(|r| r.at.clone()).filter(|x| !x.is_empty()).or_else(|| trips.last().map(|t| t.terminus.clone())).unwrap_or_default();
    (start, end, from, to)
}

/// The next break of the duty after `clock`: a company's sheet's, else a layover between two
/// trips.
fn next_break(f: &Facts, trips: &[PlanTrip], clock: f64) -> Option<Value> {
    if let Some(a) = f.assignment.filter(|a| a.sheet.iter().any(|r| r.kind == "break")) {
        let b = a.sheet.iter().filter(|r| r.kind == "break").find(|r| near(clock, r.from as f64 * 60.0) < r.to as f64 * 60.0)?;
        return Some(json!({ "kind": "break", "from": b.from as f64 * 60.0, "to": b.to as f64 * 60.0, "at": b.at }));
    }
    trips
        .windows(2)
        .find(|w| w[1].departure - w[0].arrival >= LAYOVER && near(clock, w[0].arrival) < w[1].departure)
        .map(|w| json!({ "kind": "layover", "from": w[0].arrival, "to": w[1].departure, "at": w[0].terminus }))
}

/// The duty's trips from the first one driven on (a tour taken up at the time of day starts
/// there, not at its first trip).
fn from_first<'a>(plan: &'a [PlanTrip], first: &Option<(String, String, u32)>) -> &'a [PlanTrip] {
    match first {
        Some((l, t, n)) => plan.iter().position(|p| &p.line == l && &p.tour == t && p.index == *n as usize).map_or(plan, |k| &plan[k..]),
        None => plan,
    }
}

/// What the driver types into the IBIS, and what it is set to: the codes of the trip under way
/// (`for`: `now`), else of the one to come (`next`: between two trips, `first`: before the
/// duty), the next trip's too while one is under way, the IBIS's stop of the next stop, and
/// what the bus's IBIS shows (the plugin's) and whether that is the trip's.
fn ibis_view(f: &Facts, trips: &[PlanTrip], k: Option<usize>, trip_done: bool, clock: Option<f64>, next: usize) -> Value {
    let x = f.dash;
    let u = x.and_then(|x| x.duty.as_ref());
    let free = f.duty.free_line.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let (which, idx) = match (k, u) {
        (Some(k), Some(_)) if trip_done => ("next", Some(k + 1)),
        (Some(k), Some(_)) => ("now", Some(k)),
        _ => ("first", clock.and_then(|c| trips.iter().position(|t| t.arrival > near(c, t.departure))).or((!trips.is_empty()).then_some(0))),
    };
    let trip = idx.and_then(|i| trips.get(i));
    let name = match (u, which) {
        (Some(u), "now") if !u.trip_name.is_empty() => Some(u.trip_name.clone()),
        _ => trip.map(|t| t.name.clone()).or_else(|| free.map(str::to_string)),
    };
    let codes = name.as_ref().and_then(|n| f.ibis.get(n));
    let after = (which == "now").then(|| k.and_then(|k| trips.get(k + 1))).flatten();
    let set = x.and_then(|x| x.ibis.as_ref());
    // (the bus's IBIS against the trip's: the line, then the route, else the destination)
    let matches = set.zip(codes).map(|(s, c)| {
        let line = c.line.is_none_or(|l| s.line.is_some_and(|v| v.round() as u32 == l / 100));
        let route = c.route_index.is_none_or(|r| s.route_index.is_none_or(|v| v.round() as i64 == r as i64));
        let dest = s.terminus.is_none_or(|t| t.round() as i32 == c.terminus_code);
        line && route && dest
    });
    let stop = codes.filter(|_| which == "now").and_then(|c| c.stations.get(next).copied().flatten());
    json!({
        "hof": f.depot.name, "file": f.depot.file,
        "for": which, "trip": name, "kurs": trip.map(|t| t.tour.clone()).or_else(|| f.duty.tour.clone()),
        "number": trip.map(|t| t.number.clone()), "departure": trip.map(|t| t.departure),
        "codes": codes, "stop": stop,
        "next": after.and_then(|t| f.ibis.get(&t.name).map(|c| json!({ "trip": t.name, "number": t.number, "departure": t.departure, "codes": c }))),
        "set": set, "matches": matches,
    })
}

/// The dashboard's view of a drive.
pub fn build(f: &Facts) -> Value {
    let mode = mode_of(f.duty, f.assignment);
    let trips = from_first(f.plan, &f.first);
    let x = f.dash;
    let clock = x.and_then(|x| x.clock);
    let u = x.and_then(|x| x.duty.as_ref());
    let stops = x.map(|x| x.stops.as_slice()).unwrap_or(&[]);
    // the trip under way: its place in the plan
    let k = u.and_then(|u| trips.iter().position(|p| p.line == u.line && p.tour == u.tour && p.index == u.trip as usize));
    let cur = k.map(|k| &trips[k]);
    let (start, end, from_place, to_place) = span(f, trips);
    let delay = u.map(|u| u.delay);
    let trip_done = u.is_some_and(|u| u.next >= stops.len() && !stops.is_empty());
    let status = match (x, u) {
        (None, _) => "starting",
        (Some(_), Some(u)) if u.at_stop => "at_stop",
        (Some(_), Some(_)) if trip_done => "layover",
        (Some(_), Some(_)) => "in_service",
        (Some(_), None) if mode == "free" => "free",
        (Some(_), None) if clock.zip(end).is_some_and(|(c, e)| near(c, e) > e && f.first.is_some()) => "done",
        (Some(_), None) => "waiting",
    };
    // the next stops: when the bus is due there, when it gets there at its delay, how far
    let trip_name = u.map(|u| u.trip_name.clone()).filter(|n| !n.is_empty()).or_else(|| cur.map(|c| c.name.clone())).unwrap_or_default();
    let cum = f.metres.get(&trip_name).filter(|m| m.len() == stops.len());
    let since = x.map_or(0.0, |x| x.drive.since_stop);
    let next = u.map_or(stops.len(), |u| u.next.min(stops.len()));
    let behind = delay.unwrap_or(0.0).max(0.0);
    let stop_view = |i: usize, s: &bridge::DashStop| {
        let due = if i == 0 { s.departure } else { s.arrival };
        let eta = due + behind;
        let metres = cum.map(|m| {
            let from = m[next.saturating_sub(1).min(m.len() - 1)];
            if u.is_some_and(|u| u.at_stop) && i == next {
                0.0
            } else {
                (m[i] - from - if next > 0 { since } else { 0.0 }).max(0.0)
            }
        });
        json!({ "index": i, "name": s.name, "due": due, "eta": eta, "minutes": clock.map(|c| ((near(eta, c) - c) / 60.0).max(0.0)), "metres": metres, "passed": i < next, "stops": s.stops })
    };
    let upcoming: Vec<Value> = stops.iter().enumerate().skip(next).filter(|(_, s)| s.stops).take(NEXT_STOPS).map(|(i, s)| stop_view(i, s)).collect();
    let all_stops: Vec<Value> = stops.iter().enumerate().map(|(i, s)| stop_view(i, s)).collect();
    let number = u.map(|u| u.number.clone()).filter(|n| !n.is_empty()).or_else(|| cur.map(|c| c.number.clone())).or_else(|| f.duty.line.clone()).unwrap_or_default();
    let terminus = u.map(|u| u.terminus.clone()).filter(|n| !n.is_empty()).or_else(|| cur.map(|c| c.terminus.clone())).unwrap_or_default();
    let first_stop = stops.first().map(|s| s.name.clone()).or_else(|| cur.map(|c| c.from.clone())).unwrap_or_default();
    let elapsed = clock.zip(start).map(|(c, s)| (near(c, s) - s).max(0.0));
    let (company, duty_name) = f.assignment.map(|a| (a.company.clone(), a.duty.clone())).unwrap_or_default();
    let bus = x.map(|x| &x.bus);
    json!({
        "live": x.is_some(),
        "mode": mode,
        "status": status,
        "company": company,
        "duty": duty_name,
        "clock": clock,
        "date": f.duty.date,
        "route": {
            "trip_name": trip_name, "number": number, "line": u.map(|u| u.line.clone()).or_else(|| f.duty.line.clone()), "tour": u.map(|u| u.tour.clone()).or_else(|| f.duty.tour.clone()),
            "from": first_stop, "to": terminus, "trip": k.map(|k| k + 1), "trips": trips.len(),
            "free_route": f.duty.free_line,
        },
        "vehicle": {
            "number": f.duty.number.clone().filter(|n| !n.trim().is_empty()).or_else(|| f.duty.plate.clone().filter(|n| !n.trim().is_empty())),
            "name": f.bus_name.map(str::to_string).unwrap_or_else(|| hub_core::display_bus_name(&Path::new(&f.duty.bus).file_stem().unwrap_or_default().to_string_lossy())),
            "file": f.duty.bus,
        },
        "start": start, "end": end, "elapsed": elapsed, "start_place": from_place, "end_place": to_place,
        "next_break": clock.and_then(|c| next_break(f, trips, c)),
        "speed": bus.map(|b| b.speed), "delay": delay, "passengers": bus.and_then(|b| b.passengers), "fuel": bus.and_then(|b| b.fuel),
        "stops": upcoming,
        "trip_stops": all_stops,
        "plan": trips,
        "ibis": ibis_view(f, trips, k, trip_done, clock, next),
        "today": x.map(|x| {
            let d = &x.drive;
            json!({
                "metres": d.metres, "seconds": d.seconds, "boarded": d.boarded, "served": d.served, "late": d.late, "early": d.early,
                "on_time": (d.served > 0).then(|| d.on_time as f64 / d.served as f64 * 100.0),
                "avg_speed": (d.seconds > 60.0).then(|| d.metres / d.seconds * 3.6),
                "jolts": d.jolts, "crashes": d.crashes,
            })
        }),
    })
}

/// The driver's figures over `days` days to `now` (Unix seconds), and over the days before them.
pub fn period(runs: &[TripRun], now: u64, days: u64) -> Value {
    let span = days * 86_400;
    let sum = |from: u64, to: u64| {
        let r: Vec<&TripRun> = runs.iter().filter(|t| t.time >= from && t.time < to).collect();
        let metres: f64 = r.iter().map(|t| t.metres).sum();
        let seconds: f64 = r.iter().map(|t| t.seconds).sum();
        let stops: i32 = r.iter().filter(|t| !t.free).map(|t| t.stops).sum();
        let on_time: i32 = r.iter().filter(|t| !t.free).map(|t| t.on_time()).sum();
        let passengers: i32 = r.iter().map(|t| t.passengers).sum();
        json!({
            "trips": r.len(), "metres": metres, "boarded": passengers,
            "on_time": (stops > 0).then(|| on_time as f64 / stops as f64 * 100.0),
            "avg_speed": (seconds > 60.0).then(|| metres / seconds * 3.6),
        })
    };
    // the days of the period: the kilometres of each (the oldest first)
    let off = hub_core::trips::utc_offset();
    let today = (now as i64 + off).div_euclid(86_400);
    let mut per_day = vec![0.0; days as usize];
    for t in runs.iter().filter(|t| t.time + span >= now && t.time <= now) {
        let d = (t.time as i64 + off).div_euclid(86_400);
        let back = today - d;
        if (0..days as i64).contains(&back) {
            per_day[(days as i64 - 1 - back) as usize] += t.metres / 1000.0;
        }
    }
    json!({ "now": sum(now.saturating_sub(span), now + 1), "before": sum(now.saturating_sub(2 * span), now.saturating_sub(span)), "days": per_day })
}

/// The trip records, read again only when the file changed.
fn runs(driver: &str) -> Arc<Vec<TripRun>> {
    /// The file, its length and when it was written, and what it held.
    type Read = (PathBuf, u64, std::time::SystemTime, Arc<Vec<TripRun>>);
    static CACHE: Mutex<Option<Read>> = Mutex::new(None);
    let data = hub_core::data_dir();
    let file = hub_core::trips::trips_file(&data, driver);
    let meta = std::fs::metadata(&file).ok();
    let (len, modified) = meta.map_or((0, std::time::UNIX_EPOCH), |m| (m.len(), m.modified().unwrap_or(std::time::UNIX_EPOCH)));
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((f, l, m, r)) = c.as_ref() {
        if *f == file && *l == len && *m == modified {
            return Arc::clone(r);
        }
    }
    let r = Arc::new(hub_core::trips::trips_of(&data, driver));
    *c = Some((file, len, modified, Arc::clone(&r)));
    r
}

/// `hub_live`: the dashboard of the drive Omsi-Hub started, `{ running: false }` when none runs.
pub fn view() -> Value {
    let running = crate::drive::state().get("running").and_then(Value::as_bool).unwrap_or(false);
    if !running || drive().is_none() {
        return json!({ "running": false });
    }
    ensure_plan();
    let g = drive();
    let Some(d) = g.as_ref() else { return json!({ "running": false }) };
    let dash = d.plugin_dir.as_deref().and_then(|dir| fresh_dash(dir, d.began));
    let empty = Plan::default();
    let plan = d.plan.as_deref().unwrap_or(&empty);
    let facts = Facts {
        duty: &d.duty,
        assignment: d.assignment.as_ref(),
        dash: dash.as_ref(),
        plan: &plan.trips,
        metres: &plan.metres,
        first: d.first.clone(),
        bus_name: d.bus_name.as_deref(),
        depot: &plan.depot,
        ibis: &plan.ibis,
    };
    let mut v = build(&facts);
    v["running"] = json!(true);
    v["plugin"] = json!(d.plugin_dir.is_some());
    v["began"] = json!(d.began);
    // what was told, the newest first; the company's words as they came
    let mut told: Vec<&Told> = d.told.iter().collect();
    told.sort_by(|a, b| b.at.unwrap_or(-1.0).total_cmp(&a.at.unwrap_or(-1.0)));
    v["told"] = to_value(&told);
    v["brief"] = d.plugin_dir.as_deref().and_then(|dir| read_json::<Value>(dir, crate::company::ontime::BRIEF_FILE)).and_then(|b| b.get("alerts").cloned()).unwrap_or(json!([]));
    // the drive over time: [clock, speed, delay, passengers], at the most 240 of them
    let step = d.samples.len().div_ceil(240).max(1);
    v["series"] = Value::Array(d.samples.iter().step_by(step).map(|s| json!([s.clock, s.speed, s.delay, s.passengers])).collect());
    let tstep = d.trail.len().div_ceil(TRAIL_VIEW).max(1);
    let mut trail: Vec<[f64; 2]> = d.trail.iter().step_by(tstep).copied().collect();
    if let (Some(l), Some(t)) = (d.trail.last(), trail.last()) {
        if l != t {
            trail.push(*l);
        }
    }
    let bus = dash.as_ref().and_then(|x| x.bus.x.zip(x.bus.y)).map(|(a, b)| [a, b]);
    let heading = dash.as_ref().and_then(|x| x.bus.heading).or_else(|| {
        d.trail.iter().rev().skip(1).find(|p| d.trail.last().is_some_and(|l| (l[0] - p[0]).hypot(l[1] - p[1]) >= 8.0)).zip(d.trail.last()).map(|(p, l)| (l[0] - p[0]).atan2(l[1] - p[1]).to_degrees())
    });
    v["map"] = json!({ "map": d.duty.map, "bus": bus, "heading": heading, "trail": trail, "stood": d.stood.iter().map(|(n, at)| json!({ "name": n, "at": at })).collect::<Vec<_>>() });
    let driver = d.assignment.as_ref().map(|a| a.driver.clone()).filter(|x| !x.trim().is_empty()).unwrap_or_else(crate::launcher::driver);
    drop(g);
    let r = runs(&driver);
    let t = now() as u64;
    v["driver"] = json!(driver);
    v["week"] = period(&r, t, 7);
    v["month"] = period(&r, t, 30);
    v
}

fn to_value<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_core::{StopInfo, TourInfo, TripInfo};
    use hub_game::bridge::{DashBus, DashDrive, DashDuty, DashStop, Leg};

    fn trip(index: usize, name: &str, dep: f64, arr: f64) -> TripInfo {
        TripInfo { name: name.into(), index, line: "23".into(), from: "Riverside".into(), terminus: "Central Station".into(), departure: dep, arrival: arr, stops: Vec::<StopInfo>::new(), km: 9.4 }
    }

    fn lines() -> Vec<LineInfo> {
        let tour = |n: &str, at: f64| TourInfo {
            number: n.into(),
            trips: vec![trip(1, "23_a", at, at + 1800.0), trip(2, "23_b", at + 2400.0, at + 4200.0), trip(3, "23_a", at + 4300.0, at + 6100.0)],
            ..TourInfo::default()
        };
        vec![LineInfo { name: "23".into(), tours: vec![tour("1", 27_000.0), tour("2", 30_000.0)], ..LineInfo::default() }]
    }

    fn tour_duty() -> Duty {
        Duty { map: "maps/X/global.cfg".into(), bus: "Vehicles/MB_Citaro/Citaro_Solo.bus".into(), line: Some("23".into()), tour: Some("1".into()), number: Some("A-284".into()), ..Duty::default() }
    }

    #[test]
    fn every_way_to_drive_has_its_mode_and_plan() {
        let l = lines();
        assert_eq!(mode_of(&tour_duty(), None), "tour");
        assert_eq!(plan_of(&tour_duty(), None, &l).iter().map(|t| t.index).collect::<Vec<_>>(), [1, 2, 3]);
        // (a tour started at its second trip)
        let a = Assignment { trip: 2, ..Assignment::default() };
        assert_eq!(plan_of(&tour_duty(), Some(&a), &l).len(), 2);
        // a shift of two parts
        let a = Assignment {
            legs: vec![Leg { line: "23".into(), tour: "1".into(), first: 2, trips: 1, ..Leg::default() }, Leg { line: "23".into(), tour: "2".into(), first: 1, trips: 2, ..Leg::default() }],
            ..Assignment::default()
        };
        let shift = Duty { legs: vec!["x".into()], ..tour_duty() };
        assert_eq!(mode_of(&shift, Some(&a)), "shift");
        let p = plan_of(&shift, Some(&a), &l);
        assert_eq!(p.iter().map(|t| (t.tour.as_str(), t.index)).collect::<Vec<_>>(), [("1", 2), ("2", 1), ("2", 2)]);
        // a company's duty, a free drive
        assert_eq!(mode_of(&tour_duty(), Some(&Assignment { company: "Stadtbus".into(), ..Assignment::default() })), "company");
        let free = Duty { line: None, tour: None, ..tour_duty() };
        assert_eq!(mode_of(&free, None), "free");
        assert!(plan_of(&free, None, &l).is_empty());
    }

    fn dash() -> Dash {
        let stop = |n: &str, at: f64| DashStop { name: n.into(), arrival: at, departure: at, stops: true };
        Dash {
            at: 1.0,
            clock: Some(27_000.0 + 38.0 * 60.0),
            bus: DashBus { speed: 34.0, x: Some(10.0), y: Some(20.0), passengers: Some(31.0), ..DashBus::default() },
            duty: Some(DashDuty {
                line: "23".into(),
                tour: "1".into(),
                trip: 1,
                trips: 3,
                trip_name: "23_a".into(),
                number: "23".into(),
                terminus: "Central Station".into(),
                delay: 120.0,
                next: 2,
                ..DashDuty::default()
            }),
            stops: vec![stop("Riverside", 27_000.0), stop("City Mall", 28_000.0), stop("City Hall", 29_000.0), stop("Museum Quarter", 29_300.0), stop("Central Station", 29_800.0)],
            drive: DashDrive { seconds: 2280.0, metres: 21_660.0, boarded: 40, served: 4, on_time: 3, late: 1, since_stop: 300.0, ..DashDrive::default() },
            ..Dash::default()
        }
    }

    #[test]
    fn the_view_has_the_duty_its_next_stops_and_how_far() {
        let plan = plan_of(&tour_duty(), None, &lines());
        let x = dash();
        let mut metres = HashMap::new();
        metres.insert("23_a".to_string(), vec![0.0, 2000.0, 3500.0, 6000.0, 9400.0]);
        let f = Facts {
            duty: &tour_duty(),
            assignment: None,
            dash: Some(&x),
            plan: &plan,
            metres: &metres,
            first: Some(("23".into(), "1".into(), 1)),
            bus_name: Some("Mercedes Citaro"),
            depot: &Depot::default(),
            ibis: &HashMap::new(),
        };
        let v = build(&f);
        assert_eq!(v["status"], "in_service");
        assert_eq!(v["mode"], "tour");
        assert_eq!((v["route"]["number"].as_str(), v["route"]["trip"].as_u64(), v["route"]["trips"].as_u64()), (Some("23"), Some(1), Some(3)));
        assert_eq!((v["vehicle"]["number"].as_str(), v["vehicle"]["name"].as_str()), (Some("A-284"), Some("Mercedes Citaro")));
        // the duty from its first departure to its last arrival, 38 min in
        assert_eq!((v["start"].as_f64(), v["end"].as_f64(), v["elapsed"].as_f64()), (Some(27_000.0), Some(33_100.0), Some(38.0 * 60.0)));
        // the next stops from the next one on: due, expected at the delay, the way there
        let names: Vec<&str> = v["stops"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["City Hall", "Museum Quarter", "Central Station"]);
        let s = &v["stops"][0];
        assert_eq!((s["due"].as_f64(), s["eta"].as_f64(), s["metres"].as_f64()), (Some(29_000.0), Some(29_120.0), Some(1200.0)));
        assert_eq!(s["minutes"].as_f64(), Some((29_120.0 - 29_280.0f64).max(0.0) / 60.0));
        // the layover between the first two trips is the next break
        assert_eq!(v["next_break"]["kind"], "layover");
        assert_eq!((v["next_break"]["from"].as_f64(), v["next_break"]["to"].as_f64()), (Some(28_800.0), Some(29_400.0)));
        assert_eq!(v["today"]["on_time"].as_f64(), Some(75.0));
        assert!((v["today"]["avg_speed"].as_f64().unwrap() - 34.2).abs() < 0.01);
    }

    #[test]
    fn a_company_s_sheet_gives_its_start_end_and_breaks() {
        let row = |kind: &str, at: &str, from: i32, to: i32| SheetRow { kind: kind.into(), at: at.into(), from, to, ..SheetRow::default() };
        let a = Assignment {
            company: "Stadtbus".into(),
            duty: "23 1 4".into(),
            sheet: vec![row("sign_on", "Riverside Depot", 445, 445), row("trip", "", 450, 480), row("break", "Central Station", 630, 660), row("sign_off", "Riverside Depot", 720, 720)],
            ..Assignment::default()
        };
        let plan = plan_of(&tour_duty(), Some(&a), &lines());
        let x = dash();
        let metres = HashMap::new();
        let f = Facts { duty: &tour_duty(), assignment: Some(&a), dash: Some(&x), plan: &plan, metres: &metres, first: None, bus_name: None, depot: &Depot::default(), ibis: &HashMap::new() };
        let v = build(&f);
        assert_eq!((v["start"].as_f64(), v["end"].as_f64()), (Some(445.0 * 60.0), Some(720.0 * 60.0)));
        assert_eq!((v["start_place"].as_str(), v["end_place"].as_str()), (Some("Riverside Depot"), Some("Riverside Depot")));
        assert_eq!((v["next_break"]["kind"].as_str(), v["next_break"]["from"].as_f64()), (Some("break"), Some(630.0 * 60.0)));
        // (no lengths of the legs: no distance)
        assert!(v["stops"][0]["metres"].is_null());
        // no dashboard yet: the game is starting
        let f = Facts { dash: None, ..f };
        assert_eq!(build(&f)["status"], "starting");
    }

    #[test]
    fn the_ibis_of_the_trip_and_whether_the_bus_is_set_to_it() {
        let plan = plan_of(&tour_duty(), None, &lines());
        let codes = |route: u32, dest: i32, index: usize| TripIbis {
            line: Some(2300),
            route: Some(route),
            terminus_code: dest,
            route_index: Some(index),
            stations: vec![Some(0), Some(1), Some(2), Some(3), Some(4)],
            ..TripIbis::default()
        };
        let mut ibis = HashMap::new();
        ibis.insert("23_a".to_string(), codes(1, 105, 0));
        ibis.insert("23_b".to_string(), codes(2, 107, 1));
        let depot = Depot { name: "Riverside".into(), file: Some("Vehicles/MB_Citaro/Riverside.hof".into()) };
        let mut x = dash();
        x.ibis = Some(bridge::DashIbis { line: Some(23.0), route_index: Some(0.0), terminus: Some(105.0), terminus_name: None });
        let metres = HashMap::new();
        let f = Facts { duty: &tour_duty(), assignment: None, dash: Some(&x), plan: &plan, metres: &metres, first: Some(("23".into(), "1".into(), 1)), bus_name: None, depot: &depot, ibis: &ibis };
        let v = build(&f)["ibis"].clone();
        // the trip under way: its codes, the IBIS's stop of the next stop, the next trip's codes
        assert_eq!((v["for"].as_str(), v["trip"].as_str(), v["hof"].as_str()), (Some("now"), Some("23_a"), Some("Riverside")));
        assert_eq!((v["codes"]["route"].as_u64(), v["codes"]["terminus_code"].as_i64(), v["stop"].as_u64()), (Some(1), Some(105), Some(2)));
        assert_eq!((v["next"]["trip"].as_str(), v["next"]["codes"]["route"].as_u64()), (Some("23_b"), Some(2)));
        assert_eq!(v["matches"], true);
        // the IBIS on another destination: not the trip's
        let mut y = x.clone();
        y.ibis.as_mut().unwrap().terminus = Some(107.0);
        let f = Facts { dash: Some(&y), ..f };
        assert_eq!(build(&f)["ibis"]["matches"], false);
        // before the game is there: the first trip's codes, nothing known of the bus's IBIS
        let f = Facts { dash: None, ..f };
        let v = build(&f)["ibis"].clone();
        assert_eq!((v["for"].as_str(), v["trip"].as_str(), v["matches"].is_null()), (Some("first"), Some("23_a"), true));
    }

    #[test]
    fn the_week_and_month_come_from_the_trip_records() {
        let now = 1_800_000_000;
        let run = |days_ago: u64, metres: f64, stops: i32, late: i32| TripRun { time: now - days_ago * 86_400 - 60, metres, seconds: metres / 10.0, stops, late, passengers: 10, ..TripRun::default() };
        let runs = vec![run(1, 10_000.0, 10, 1), run(3, 5_000.0, 10, 3), run(9, 5_000.0, 10, 0)];
        let w = period(&runs, now, 7);
        assert_eq!((w["now"]["trips"].as_u64(), w["now"]["metres"].as_f64(), w["now"]["boarded"].as_u64()), (Some(2), Some(15_000.0), Some(20)));
        assert_eq!(w["now"]["on_time"].as_f64(), Some(80.0));
        assert_eq!((w["before"]["trips"].as_u64(), w["before"]["metres"].as_f64()), (Some(1), Some(5_000.0)));
        assert_eq!(w["now"]["avg_speed"].as_f64(), Some(36.0));
        let days = w["days"].as_array().unwrap();
        assert_eq!(days.len(), 7);
        assert!((days.iter().map(|d| d.as_f64().unwrap()).sum::<f64>() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn a_sample_keeps_the_way_and_where_the_bus_stood() {
        let mut d = Drive::default();
        let mut x = dash();
        take(&mut d, &x);
        x.bus.x = Some(11.0);
        take(&mut d, &x);
        assert_eq!(d.trail.len(), 1, "a metre is no way");
        x.bus.x = Some(30.0);
        x.duty.as_mut().unwrap().at_stop = true;
        take(&mut d, &x);
        assert_eq!(d.trail, [[10.0, 20.0], [30.0, 20.0]]);
        assert_eq!(d.stood.get("City Hall"), Some(&[30.0, 20.0]));
        assert_eq!(d.samples.len(), 3);
        assert_eq!(d.first, Some(("23".into(), "1".into(), 1)));
    }
}
