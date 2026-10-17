//! What the map and day steps say of a map beyond its `global.cfg`.
//!
//! * `hub_map_facts {maps}` - each map's tours the player can drive and the year it plays in:
//!   `{ file: { tours, year } }`;
//! * `hub_weather_extras {map}` - the airport whose METAR report is the map's current weather
//!   (the one nearest the map's place, `timezone.txt`) and OMSI's airports (`Weather/ICAO.txt`);
//! * `hub_own_lines {map}` - the player's own lines of the map (the line editor's), which the
//!   line lists show apart from the map's.

use crate::Answer;
use hub_omsi::Vfs;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// A map's tours and year.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    pub tours: usize,
    pub year: Option<i32>,
}

pub fn map_facts(files: &[String]) -> Answer {
    let Some(vfs) = super::vfs() else { return Ok(json!({})) };
    Ok(facts_of(&vfs, files))
}

/// The facts of the maps `files` (`maps/Grundorf/global.cfg` …), read once while the map's
/// folder and timetable stay as they are.
pub fn facts_of(vfs: &Vfs, files: &[String]) -> Value {
    type Kept = HashMap<PathBuf, (Vec<Option<SystemTime>>, Facts)>;
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    let mut shipped: Option<HashMap<String, i32>> = None;
    let mut out = serde_json::Map::new();
    for file in files {
        let Some(dir) = super::map_dir(vfs, file) else {
            out.insert(file.clone(), json!(Facts::default()));
            continue;
        };
        let tt = hub_omsi::vfs::find_child(&dir, "TTData");
        let stamp: Vec<Option<SystemTime>> = [Some(dir.clone()), tt.clone()].iter().map(|p| p.as_ref().and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())).collect();
        let hit = KEPT.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).get(&dir).filter(|(s, _)| *s == stamp).map(|(_, f)| *f);
        let f = hit.unwrap_or_else(|| {
            let shipped = shipped.get_or_insert_with(|| situation_years(vfs));
            let f = Facts { tours: tt.as_deref().map_or(0, count_tours), year: year_of(&dir, shipped) };
            KEPT.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(dir.clone(), (stamp, f));
            f
        });
        out.insert(file.clone(), json!(f));
    }
    Value::Object(out)
}

/// The tours a player can drive (`TTData/*.ttl`): those with trips on a line the player may
/// drive (`[userallowed]`); of a map that allows none, all with trips.
pub fn count_tours(tt: &Path) -> usize {
    let lines: Vec<hub_omsi::timetable::Line> = hub_core::paths::read_dir_paths(tt)
        .into_iter()
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ttl")))
        .filter_map(|p| hub_omsi::CfgFile::read(&p).ok())
        .map(|c| hub_omsi::timetable::Line::parse(&c))
        .collect();
    let with_trips = |l: &hub_omsi::timetable::Line| l.tours.iter().filter(|t| !t.trips.is_empty()).count();
    let allowed: usize = lines.iter().filter(|l| l.user_allowed).map(with_trips).sum();
    if allowed > 0 {
        allowed
    } else {
        lines.iter().map(with_trips).sum()
    }
}

/// The year a map plays in: where the game left it last (`laststn.osn`, and the copies other
/// tools keep of it), else a situation OMSI ships for it, else a year in its folder's name.
fn year_of(dir: &Path, shipped: &HashMap<String, i32>) -> Option<i32> {
    let folder = dir.file_name()?.to_string_lossy().to_string();
    for name in ["laststn.osn", "laststn.osn.voor-omsi-enhancer", "laststn.osn.voor-omsi-career"] {
        let Some(p) = hub_omsi::vfs::find_child(dir, name).filter(|p| p.is_file()) else { continue };
        if let Some((_, y)) = hub_omsi::CfgFile::read(&p).ok().as_ref().and_then(situation_place) {
            return Some(y);
        }
    }
    shipped.get(&folder.to_lowercase()).copied().or_else(|| year_in_name(&folder))
}

/// The years of the situations in OMSI's `Situations` folder, by their map's folder (lower
/// case); the first one by name counts.
fn situation_years(vfs: &Vfs) -> HashMap<String, i32> {
    let mut files: Vec<PathBuf> =
        vfs.resolve("Situations").map(|d| hub_core::paths::read_dir_paths(&d)).unwrap_or_default().into_iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osn"))).collect();
    files.sort();
    let mut out = HashMap::new();
    for f in files {
        if let Some((folder, y)) = hub_omsi::CfgFile::read(&f).ok().as_ref().and_then(situation_place) {
            out.entry(folder.to_lowercase()).or_insert(y);
        }
    }
    out
}

/// Where and when a situation plays: its map's folder and the year (`[map]`, `[time]`'s first
/// value). Not one a tool of ours wrote (it carries the date of the last drive, not the map's).
fn situation_place(f: &hub_omsi::CfgFile) -> Option<(String, i32)> {
    let (mut map, mut year, mut own) = (None, None, false);
    let mut r = f.reader();
    while let Some(k) = r.next_keyword() {
        match k {
            "name" => {
                let n = r.str().trim().to_lowercase();
                own = n.starts_with("omsi enhancer") || n.starts_with("omsi career") || n.starts_with("omsi-hub");
            }
            "description" => {
                r.until("end");
            }
            "map" => map = map_folder(r.str()),
            "time" => year = Some(r.i32()).filter(|y| (1900..=2100).contains(y)),
            _ => {}
        }
    }
    if own {
        return None;
    }
    Some((map?, year?))
}

/// The map's folder in a situation's `[map]` line (`maps\Grundorf\global.cfg`).
fn map_folder(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split(['\\', '/']).map(str::trim).filter(|p| !p.is_empty()).collect();
    match parts.iter().position(|p| p.eq_ignore_ascii_case("maps")) {
        Some(k) => parts.get(k + 1).map(|s| s.to_string()),
        None if parts.len() >= 2 => Some(parts[parts.len() - 2].to_string()),
        None => None,
    }
}

/// A year standing in a name on its own: four digits from 1900 to 2099, no more digits either
/// side ("Hamburg109" has none, "Bad_Huegelsdorf_2020" has 2020).
fn year_in_name(name: &str) -> Option<i32> {
    let b = name.as_bytes();
    let mut k = 0;
    while k < b.len() {
        if !b[k].is_ascii_digit() {
            k += 1;
            continue;
        }
        let start = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k - start == 4 {
            if let Ok(y) = name[start..k].parse::<i32>() {
                if (1900..=2099).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    None
}

// --- the weather ---------------------------------------------------------------------------------

/// The airports round Europe whose METAR reports the weather can follow: code, latitude,
/// longitude.
const AIRPORTS: &[(&str, f64, f64)] = &[
    ("BKPR", 42.57, 21.04),
    ("EBBR", 50.90, 4.48),
    ("EDDH", 53.63, 9.99),
    ("EDDM", 48.35, 11.79),
    ("EDDN", 49.50, 11.08),
    ("EDDP", 51.42, 12.24),
    ("EDDS", 48.69, 9.22),
    ("EDDB", 52.37, 13.52),
    ("EDOP", 53.43, 11.78),
    ("EETN", 59.41, 24.83),
    ("EFHF", 60.25, 25.04),
    ("EGAA", 54.66, -6.22),
    ("EGCC", 53.35, -2.27),
    ("EGLL", 51.47, -0.45),
    ("EGPH", 55.95, -3.37),
    ("EHAM", 52.31, 4.76),
    ("EIDW", 53.42, -6.27),
    ("EKBI", 55.74, 9.15),
    ("ELLX", 49.63, 6.21),
    ("ENBR", 60.29, 5.22),
    ("ENGM", 60.19, 11.10),
    ("ESKN", 58.79, 16.91),
    ("EPKK", 50.08, 19.78),
    ("EPWA", 52.17, 20.97),
    ("EVRA", 56.92, 23.97),
    ("EYVI", 54.63, 25.29),
    ("LBSF", 42.70, 23.41),
    ("LDZA", 45.74, 16.07),
    ("LEAL", 38.28, -0.56),
    ("LEBB", 43.30, -2.91),
    ("LEMD", 40.47, -3.56),
    ("LEPA", 39.55, 2.74),
    ("LFBD", 44.83, -0.72),
    ("LFLY", 45.73, 5.08),
    ("LFMD", 43.54, 6.95),
    ("LFPG", 49.01, 2.55),
    ("LGAV", 37.94, 23.94),
    ("LHBP", 47.44, 19.26),
    ("LICJ", 38.18, 13.10),
    ("LIMC", 45.63, 8.72),
    ("LIRA", 41.80, 12.59),
    ("LOWI", 47.26, 11.34),
    ("LOWL", 48.23, 14.19),
    ("LOWW", 48.11, 16.57),
    ("LPPT", 38.78, -9.14),
    ("LQSA", 43.82, 18.33),
    ("LSGG", 46.24, 6.11),
    ("LSZH", 47.46, 8.55),
    ("LTAC", 40.13, 32.99),
    ("LWSK", 41.96, 21.62),
    ("LYBE", 44.82, 20.31),
    ("LYTV", 42.40, 18.72),
    ("LZIB", 48.17, 17.21),
    ("UKKK", 50.40, 30.45),
    ("ULLI", 59.80, 30.26),
    ("UMKK", 54.89, 20.59),
    ("UMMM", 53.88, 28.03),
    ("UUEE", 55.97, 37.41),
    ("USSS", 56.74, 60.80),
];

/// The airport of a map without a place.
const DEFAULT_AIRPORT: &str = "EDDB";

pub fn weather_extras(map: &str) -> Answer {
    let Some(vfs) = super::vfs() else { return Ok(json!({ "airport": DEFAULT_AIRPORT, "airports": [] })) };
    Ok(weather_extras_of(&vfs, map))
}

/// `{ airport, airports: [[code, name]] }` for the map `map`.
pub fn weather_extras_of(vfs: &Vfs, map: &str) -> Value {
    json!({ "airport": nearest_airport(vfs, map), "airports": metar_airports(vfs) })
}

/// OMSI's METAR airports: `Weather/ICAO.txt`, a line each (`EDDH - Hamburg`).
pub fn metar_airports(vfs: &Vfs) -> Vec<(String, String)> {
    let Ok(cfg) = vfs.read_cfg("Weather/ICAO.txt") else { return Vec::new() };
    cfg.lines
        .iter()
        .filter_map(|l| {
            let (code, name) = l.split_once('-').map_or((l.trim(), ""), |(c, n)| (c.trim(), n.trim()));
            (code.len() == 4 && code.chars().all(|c| c.is_ascii_alphanumeric())).then(|| (code.to_uppercase(), if name.is_empty() { code.to_uppercase() } else { name.to_string() }))
        })
        .collect()
}

/// The airport nearest the map's place (`timezone.txt`'s `[location]`), the default one where
/// the map says none.
pub fn nearest_airport(vfs: &Vfs, map: &str) -> String {
    let place = super::map_dir(vfs, map).and_then(|d| hub_omsi::vfs::find_child(&d, "timezone.txt")).and_then(|p| hub_omsi::map::TimeZone::load(&p).ok()).and_then(|t| t.lat_lon());
    place
        .and_then(|(lat, lon)| AIRPORTS.iter().map(|(c, a, o)| (c, (a - lat).powi(2) + ((o - lon) * lat.to_radians().cos()).powi(2))).min_by(|x, y| x.1.total_cmp(&y.1)).map(|x| x.0.to_string()))
        .unwrap_or_else(|| DEFAULT_AIRPORT.into())
}

// --- the player's own lines ----------------------------------------------------------------------

/// The player's lines of `map`: the timetable line each was written as (`file`, a
/// `LineInfo::name`), its number, name, colour, where it goes and its caption.
pub fn own_lines(map: &str) -> Value {
    let own = hub_core::lines::own_lines_of_map(map);
    json!(own.iter().map(|o| json!({ "file": o.file, "number": o.number, "name": o.name, "colour": o.colour, "destinations": o.destinations, "caption": o.caption() })).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::{fixture_vfs, temp};

    #[test]
    fn a_year_in_a_name_and_a_map_in_a_situation() {
        assert_eq!(year_in_name("Bad_Huegelsdorf_2020"), Some(2020));
        assert_eq!(year_in_name("Hamburg109"), None);
        assert_eq!(year_in_name("Vienna_2005_Line_24A"), Some(2005));
        assert_eq!(map_folder("maps\\Grundorf\\global.cfg"), Some("Grundorf".into()));
        assert_eq!(serde_json::to_value(Facts { tours: 2, year: Some(1991) }).unwrap(), json!({ "tours": 2, "year": 1991 }));
        let s = hub_omsi::CfgFile::from_str("x.osn", "[name]\nMy drive\n\n[map]\nmaps\\Grundorf\\global.cfg\n\n[time]\n1991\n5\n");
        assert_eq!(situation_place(&s), Some(("Grundorf".into(), 1991)));
        let own = hub_omsi::CfgFile::from_str("x.osn", "[name]\nOmsi-Hub drive\n\n[map]\nmaps\\Grundorf\\global.cfg\n\n[time]\n1991\n");
        assert_eq!(situation_place(&own), None);
    }

    #[test]
    fn the_test_map_s_tours_and_its_airport() {
        let v = fixture_vfs();
        let f = facts_of(&v, &["maps/Testdorf/global.cfg".to_string(), "maps/Gone/global.cfg".to_string()]);
        assert!(f["maps/Testdorf/global.cfg"]["tours"].as_u64().is_some_and(|n| n > 0), "{f}");
        assert_eq!(f["maps/Gone/global.cfg"]["tours"], 0);
        // (Testdorf lies at 51.25 N, 7.5 E: of the list, Luxembourg is nearest)
        assert_eq!(nearest_airport(&v, "maps/Testdorf/global.cfg"), "ELLX");
        assert_eq!(nearest_airport(&v, "maps/Gone/global.cfg"), DEFAULT_AIRPORT);
    }

    #[test]
    fn omsi_s_airports_are_read_from_its_list() {
        let root = temp("icao");
        std::fs::create_dir_all(root.join("weather")).unwrap();
        std::fs::write(root.join("weather/icao.TXT"), b"EDDH - Hamburg\r\nEDDM - M\xfcnchen\r\n\r\nnot one\r\n").unwrap();
        let v = Vfs::new(root.clone(), None);
        assert_eq!(metar_airports(&v), vec![("EDDH".to_string(), "Hamburg".to_string()), ("EDDM".to_string(), "München".to_string())]);
        assert_eq!(weather_extras_of(&v, "maps/x/global.cfg")["airport"], DEFAULT_AIRPORT);
        let _ = std::fs::remove_dir_all(root);
    }
}
