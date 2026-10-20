//! The IBIS codes of a trip (Luc, 2026-10-20): what a driver types into the bus's IBIS for the
//! trip the timetable gives, found in the depot file (`.hof`) the bus reads - the line
//! (`Linie/Kurs`: the line × 100 and its letter's code), the route (the last two digits of the
//! depot file's `[infosystem_trip]` code) and, without a route, the destination's code (`Ziel`).
//! The live dashboard shows them so that the IBIS can be set from it.
//!
//! Nothing in the timetable ties a trip to a route of the depot file: a driver picks it by its
//! stops. So does this, as openOMSI's player IBIS does (ported from its MIT code of before
//! 2026-10-14, `omsi-sim/src/timetable_run/ibis.rs` and `duty.rs`): the destination by the
//! trip's `[trip]` target, of the line's routes to it the one whose stops follow the trip's.

use crate::linehof::Depot;
use hub_omsi::hof::{Hof, Terminus};
use hub_omsi::text::parse_i32;
use serde::Serialize;

/// A trip's IBIS as the driver sets it, and what it then shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TripIbis {
    /// What is typed: the line (`Linie/Kurs`), the route - or, without a route, the
    /// destination's code (`Ziel`).
    pub line: Option<u32>,
    pub route: Option<u32>,
    pub terminus: Option<u32>,
    /// The destination the codes lead to: its code, its name (the ident the timetable names),
    /// what the IBIS shows and the front sign's two rows.
    pub terminus_code: i32,
    pub terminus_name: String,
    pub display: String,
    pub front: Vec<String>,
    /// The depot file's route: its code, its name, its place in the file and its stops.
    pub route_code: Option<u32>,
    pub route_name: String,
    pub route_index: Option<usize>,
    pub route_stops: Vec<String>,
    /// For each of the trip's stations its stop in the route's list (what the IBIS's stop has
    /// to be at it), where the route has it.
    pub stations: Vec<Option<usize>>,
}

/// The IBIS of a trip of line `line` to `terminus` (its `[trip]` target) through `stops` (its
/// stations' names), from the depot file `depot`. None when the file has no such destination.
pub fn trip_ibis(depot: &Depot, line: &str, terminus: &str, stops: &[&str]) -> Option<TripIbis> {
    let hof = &depot.hof;
    let (ti, route) = trip_terminus(hof, line, terminus.trim(), stops)?;
    let t = &hof.termini[ti];
    let route_code = route.and_then(|i| hof.info_trips[i].code.trim().parse::<u32>().ok());
    let (typed_line, typed_route, typed_terminus) = match route_code {
        Some(r) => (line_code_from_text(line, Some(r)), Some(r % 100), None),
        None => (line_code_from_text(line, None), None, u32::try_from(t.code).ok().filter(|c| *c < 1000)),
    };
    let (side, ibis) = depot.shown(&t.strings);
    let (top, bottom) = depot.front(&t.strings);
    let display = [ibis, side, top.clone()].into_iter().find(|s| !s.trim().is_empty()).unwrap_or_else(|| t.texture_id.trim().to_string());
    Some(TripIbis {
        line: typed_line,
        route: typed_route,
        terminus: typed_terminus,
        terminus_code: t.code,
        terminus_name: t.texture_id.trim().to_string(),
        display,
        front: [top, bottom].into_iter().filter(|s| !s.is_empty()).collect(),
        route_code,
        route_name: route.map(|i| hof.info_trips[i].name.trim().to_string()).unwrap_or_default(),
        route_index: route,
        stations: stops.iter().enumerate().map(|(k, s)| route.and_then(|r| stop_index(hof, r, s, k))).collect(),
        route_stops: route.and_then(|i| hof.info_busstop_lists.get(i)).map(|l| l.iter().map(|s| s.split('#').next().unwrap_or("").trim().to_string()).collect()).unwrap_or_default(),
    })
}

/// Where timetable stop `k` (called `name`) stands in the IBIS's own stop list of route
/// `route` (an index into the depot file's routes): the stop of that name nearest `k` - what
/// the IBIS's stop has to be for it to show that stop.
pub fn stop_index(hof: &Hof, route: usize, name: &str, k: usize) -> Option<usize> {
    let stop = (name.trim().to_lowercase(), stop_words(name));
    let list = hof.info_busstop_lists.get(route)?;
    list.iter().enumerate().filter(|(_, id)| same_stop(&ident_names(hof, id), &stop)).map(|(i, _)| i).min_by_key(|i| i.abs_diff(k))
}

/// The depot file's destination a trip's target names, and the route of the line to it that
/// follows the trip's stops: of the rows of that name the one that route leads to (a file may
/// give one destination a row per route, each with a code of its own), else the first.
fn trip_terminus(hof: &Hof, line: &str, terminus: &str, stops: &[&str]) -> Option<(usize, Option<usize>)> {
    if terminus.is_empty() {
        return None;
    }
    let rows = termini_named(hof, terminus);
    let first = *rows.first()?;
    let mut codes: Vec<i32> = Vec::new();
    for &i in &rows {
        if !codes.contains(&hof.termini[i].code) {
            codes.push(hof.termini[i].code);
        }
    }
    let route = pick_route(hof, &routes_to(hof, line, &codes), stops);
    let ti = route.map(|r| parse_i32(&hof.info_trips[r].route)).and_then(|code| rows.iter().copied().find(|&i| hof.termini[i].code == code)).unwrap_or(first);
    Some((ti, route))
}

/// How well a destination answers to a name: 3 its ident, its stop or the first line of its
/// sign; 2 another of its lines; 1 one the start of the other (a depot file's shorter name of
/// the timetable's stop); 0 not at all.
fn terminus_match_score(t: &Terminus, wanted: &str) -> u8 {
    let wanted = wanted.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    if wanted.is_empty() {
        return 0;
    }
    let shown = t.strings.iter().position(|s| !s.trim().is_empty());
    let mut score = 0;
    let names = [t.texture_id.as_str(), t.terminus_stop.as_deref().unwrap_or("")].into_iter().chain(t.strings.iter().map(String::as_str));
    for (k, c) in names.enumerate() {
        let c = c.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        if c.is_empty() {
            continue;
        }
        if c == wanted {
            score = score.max(if k < 2 || Some(k - 2) == shown { 3 } else { 2 });
        } else if wanted.starts_with(&format!("{c} ")) || c.starts_with(&format!("{wanted} ")) {
            score = score.max(1);
        }
    }
    score
}

/// Every row of the depot file a trip's target names, in file order: those whose ident is the
/// name (as OMSI takes it), else those of the best looser match.
fn termini_named(hof: &Hof, wanted: &str) -> Vec<usize> {
    let exact = wanted.trim();
    let rows: Vec<usize> = (0..hof.termini.len()).filter(|&i| hof.termini[i].texture_id.trim() == exact).collect();
    if !rows.is_empty() {
        return rows;
    }
    let scores: Vec<u8> = hof.termini.iter().map(|t| terminus_match_score(t, wanted)).collect();
    let best = scores.iter().copied().max().unwrap_or(0);
    (0..scores.len()).filter(|&i| best > 0 && scores[i] == best).collect()
}

/// The depot file's routes of `line` to a destination with one of `codes`, in file order. A
/// route's code is the line's number and two digits, whatever its line column says.
fn routes_to(hof: &Hof, line: &str, codes: &[i32]) -> Vec<usize> {
    let digits: String = line.trim().chars().take_while(|c| c.is_ascii_digit()).collect();
    let number = digits.parse::<u32>().ok();
    hof.info_trips
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            codes.contains(&parse_i32(&t.route))
                && (t.line.trim().eq_ignore_ascii_case(line.trim())
                    || (!digits.is_empty() && t.line.trim() == digits)
                    || (number.is_some() && t.code.trim().parse::<u32>().ok().map(|c| c / 100) == number))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Which of `routes` a trip through `stops` runs: one starting at the trip's first stop, of
/// those the one with most of the trip's stops in its order, then the one as long as the trip.
/// The first route when no stop matches any.
fn pick_route(hof: &Hof, routes: &[usize], stops: &[&str]) -> Option<usize> {
    let trip: Vec<(String, Vec<String>)> = stops.iter().map(|s| (s.trim().to_lowercase(), stop_words(s))).filter(|(s, _)| !s.is_empty()).collect();
    let mut best: Option<(usize, (bool, usize, std::cmp::Reverse<usize>))> = None;
    for &r in routes {
        let list = hof.info_busstop_lists.get(r).map(Vec::as_slice).unwrap_or(&[]);
        let names: Vec<Vec<(String, Vec<String>)>> = list.iter().map(|id| ident_names(hof, id)).collect();
        // (the longest common subsequence: a stop missing on either side costs only itself)
        let mut row = vec![0usize; names.len() + 1];
        for t in &trip {
            let mut diag = 0;
            for (j, n) in names.iter().enumerate() {
                let up = row[j + 1];
                row[j + 1] = if same_stop(n, t) { diag + 1 } else { up.max(row[j]) };
                diag = up;
            }
        }
        let starts = matches!((trip.first(), names.first()), (Some(t), Some(n)) if same_stop(n, t));
        let score = (starts, row[names.len()], std::cmp::Reverse(names.len().abs_diff(stops.len())));
        if best.as_ref().is_none_or(|(_, b)| score > *b) {
            best = Some((r, score));
        }
    }
    match best {
        Some((r, (_, found, _))) if found > 0 => Some(r),
        _ => routes.first().copied(),
    }
}

/// The names a stop of a route's list goes by: its ident (before a `#`) and the strings of the
/// depot file's `[addbusstop]` of that ident, lowercased and as their words.
fn ident_names(hof: &Hof, ident: &str) -> Vec<(String, Vec<String>)> {
    let ident = ident.split('#').next().unwrap_or("").trim();
    let mut names = vec![ident.to_string()];
    for b in hof.bus_stops.iter().filter(|b| b.ident.trim().eq_ignore_ascii_case(ident)) {
        names.extend(b.strings.iter().map(|s| s.trim().to_string()));
    }
    names.into_iter().filter(|n| !n.is_empty()).map(|n| (n.to_lowercase(), stop_words(&n))).collect()
}

/// A route's stop and a timetable stop are the same: a name equal, one the start of the
/// other, or the same words.
fn same_stop(names: &[(String, Vec<String>)], stop: &(String, Vec<String>)) -> bool {
    names.iter().any(|(raw, words)| !raw.is_empty() && (*raw == stop.0 || raw.starts_with(&stop.0) || stop.0.starts_with(raw.as_str()) || (!words.is_empty() && *words == stop.1)))
}

/// A stop name as the set of its words, in any order and with any punctuation, without
/// one-letter prefixes ("F_Kirchweg" is "Kirchweg").
fn stop_words(name: &str) -> Vec<String> {
    let mut w: Vec<String> = name.split(|c: char| !c.is_alphanumeric()).filter(|w| w.chars().count() > 1).map(str::to_lowercase).collect();
    w.sort();
    w
}

/// The letter and digits of a line named letter first ("X10"), else None.
fn line_prefix(line: &str) -> Option<(char, &str)> {
    let line = line.trim();
    let first = line.chars().next().filter(|c| c.is_ascii_alphabetic())?;
    let digits = &line[first.len_utf8()..];
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then_some((first.to_ascii_uppercase(), digits))
}

/// The IBIS's code of a line's letter (the stock matrices' codes): a letter before the number
/// ("E5", "X10") or after it ("5E").
fn line_suffix_from_text(line: &str) -> u32 {
    if let Some((letter, _)) = line_prefix(line) {
        return match letter {
            'E' => 1,
            'S' => 5,
            'A' => 6,
            'D' => 11,
            'C' => 12,
            'B' => 13,
            'U' => 25,
            'M' => 28,
            'N' => 35,
            'X' => 36,
            _ => 0,
        };
    }
    match line.trim().chars().last().map(|c| c.to_ascii_uppercase()) {
        Some('E') => 10,
        Some('U') => 31,
        Some('N') => 4,
        Some('S') => 23,
        Some('M') => 32,
        _ => 0,
    }
}

/// What is typed as the line: its number × 100 and its letter's code (a lettered line takes
/// the depot file's number: X10 Berlin's is 510); the route's code without a number.
fn line_code_from_text(line: &str, route_code: Option<u32>) -> Option<u32> {
    if let (Some(_), Some(code)) = (line_prefix(line), route_code) {
        return Some(code / 100 * 100 + line_suffix_from_text(line));
    }
    let digits: String = match line_prefix(line) {
        Some((_, d)) => d.to_string(),
        None => line.trim().chars().take_while(|c| c.is_ascii_digit()).collect(),
    };
    match digits.parse::<u32>().ok().filter(|n| *n > 0 && *n < 100_000) {
        Some(n) => Some(n * 100 + line_suffix_from_text(line)),
        None => route_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linehof::tests::GRUNDORF;
    use std::path::Path;

    fn grundorf() -> Depot {
        Depot::from_text(Path::new("Grundorf.hof"), GRUNDORF, "maps/Grundorf/global.cfg")
    }

    #[test]
    fn a_trip_s_codes_are_its_line_the_route_through_its_stops_and_where_it_goes() {
        let d = grundorf();
        let i = trip_ibis(&d, "76", "Krankenhaus", &["Bauernhof", "Krankenhaus"]).unwrap();
        assert_eq!((i.line, i.route, i.terminus), (Some(7600), Some(1), None));
        assert_eq!((i.terminus_code, i.terminus_name.as_str(), i.route_code), (105, "Krankenhaus", Some(7601)));
        assert_eq!(i.route_stops, ["Bauernhof", "Krankenhaus"]);
        assert_eq!(i.stations, [Some(0), Some(1)]);
        assert_eq!(i.display, "KRANKENHAUS");
        assert_eq!(stop_index(&d.hof, i.route_index.unwrap(), "Krankenhaus", 1), Some(1));
        // a line without a route to it: the destination's code is typed
        let i = trip_ibis(&d, "5", "Flugplatz", &["Bauernhof", "Flugplatz"]).unwrap();
        assert_eq!((i.line, i.route, i.terminus, i.terminus_code), (Some(500), None, Some(113), 113));
        // a destination the file does not have: nothing to type
        assert!(trip_ibis(&d, "5", "Nirgendwo", &[]).is_none());
    }

    #[test]
    fn a_lettered_line_types_its_letter_s_code() {
        assert_eq!(line_code_from_text("5E", None), Some(510));
        assert_eq!(line_code_from_text("X10", Some(1003)), Some(1036));
        assert_eq!(line_code_from_text("92", None), Some(9200));
        assert_eq!(line_code_from_text("", Some(7601)), Some(7601));
    }
}
