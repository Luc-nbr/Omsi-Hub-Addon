//! A check against a real OMSI 2 install - ignored, so CI (which has none) skips it. It only
//! reads; what it writes back stays in memory. Run it with the OMSI 2 folder, and a map in it
//! if only one should be checked:
//!
//! ```text
//! set HUB_OMSI_REAL_ROOT=C:\Program Files (x86)\Steam\steamapps\common\OMSI 2
//! set HUB_OMSI_REAL_MAP=maps/Grundorf/global.cfg      (optional: else every map)
//! cargo test -p hub-omsi --test real_data -- --ignored --nocapture
//! ```

use chrono::NaiveDate;
use hub_omsi::hof::{depot_files, Comments, Hof};
use hub_omsi::map::{active_chrono_dirs, AiLists, Calendar, GlobalCfg};
use hub_omsi::text::CfgFile;
use hub_omsi::timetable::{lines_for, parse_busstops, parse_stnlinks, Line, Timetable, Track, Trip};
use hub_omsi::vehicle::{capacity, Bus};
use hub_omsi::weather::Weather;
use hub_omsi::{font, Vfs};
use std::path::{Path, PathBuf};

fn root() -> Option<PathBuf> {
    std::env::var_os("HUB_OMSI_REAL_ROOT").map(PathBuf::from).filter(|p| p.is_dir())
}

/// The maps to check: `HUB_OMSI_REAL_MAP`, else every map folder with a `global.cfg`.
fn maps(root: &Path) -> Vec<String> {
    if let Ok(m) = std::env::var("HUB_OMSI_REAL_MAP") {
        return vec![m];
    }
    let vfs = Vfs::new(root.to_path_buf(), None);
    vfs.list_dir("maps").into_iter().filter(|(_, dir)| *dir).map(|(n, _)| format!("maps/{n}/global.cfg")).filter(|m| vfs.is_file(m)).collect()
}

fn files_with(dir: &Path, ext: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for p in entries.flatten().map(|e| e.path()) {
        if p.is_dir() {
            files_with(&p, ext, out);
        } else if p.extension().is_some_and(|x| ext.iter().any(|e| x.eq_ignore_ascii_case(e))) {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "needs a real OMSI 2 install: HUB_OMSI_REAL_ROOT"]
fn a_real_map_reads_and_its_timetable_writes_back_the_same() {
    let Some(root) = root() else {
        eprintln!("HUB_OMSI_REAL_ROOT is not set: nothing checked");
        return;
    };
    let vfs = Vfs::new(root.clone(), None);
    let date = NaiveDate::from_ymd_opt(1989, 5, 30).unwrap();
    let mut failures = Vec::new();
    for map in maps(&root) {
        let path = vfs.resolve(&map).expect("the map's global.cfg");
        let dir = path.parent().unwrap().to_path_buf();
        let g = GlobalCfg::load(&path).expect("global.cfg");
        assert!(!g.tiles.is_empty(), "{map}: no [map] tiles");
        assert!(g.entry_points.iter().all(|e| !e.name.is_empty()), "{map}: an entry point without a name");
        let cal = Calendar::load(&dir).unwrap();
        let ai = AiLists::with_chrono(&dir, &active_chrono_dirs(&dir, date));
        let depot = hub_omsi::map::depot_hof_on(&dir, date);
        let lines = lines_for(&vfs, &map, date).expect("lines_for");
        let tours: usize = lines.iter().map(|l| l.tours.len()).sum();
        let running = lines.iter().flat_map(|l| &l.tours).filter(|t| t.runs).count();
        let timed = lines.iter().flat_map(|l| &l.tours).flat_map(|t| &t.trips).filter(|t| !t.stops.is_empty()).count();
        // (every stop time in order)
        for t in lines.iter().flat_map(|l| &l.tours).flat_map(|t| &t.trips) {
            assert!(t.stops.windows(2).all(|w| w[0].dep <= w[1].arr && w[1].arr <= w[1].dep), "{map}: {} stop times out of order", t.name);
        }
        println!(
            "{map}: {} ({} tiles, {} entry points, tz {:?}, LHT {}), {} holidays, {} ranges, {} AI groups ({} depot), depot file {:?}; {} lines, {tours} tours ({running} run on {date}), {timed} trips timed",
            g.name,
            g.tiles.len(),
            g.entry_points.len(),
            g.timezone.as_ref().map(|t| (t.offset_hours, t.lat_lon())),
            g.left_hand_traffic,
            cal.holidays.len(),
            cal.ranges.len(),
            ai.groups.len(),
            ai.groups.iter().filter(|g| g.is_depot).count(),
            depot,
            lines.len()
        );
        // the round trip: every TTData file read, written, read again - the same
        let mut tt_files = Vec::new();
        files_with(&dir, &["ttl", "ttp", "ttr", "cfg"], &mut tt_files);
        let mut checked = 0;
        for f in tt_files.iter().filter(|f| f.components().any(|c| c.as_os_str().eq_ignore_ascii_case("TTData"))) {
            let cfg = CfgFile::read(f).unwrap();
            let again = |text: String| CfgFile::from_str(f, &text);
            let name = f.file_name().unwrap().to_string_lossy().to_lowercase();
            let same = match f.extension().unwrap().to_string_lossy().to_lowercase().as_str() {
                "ttl" => {
                    let l = Line::parse(&cfg);
                    l == Line { codepage: l.codepage, ..Line::parse(&again(l.to_text())) }
                }
                "ttp" => Trip::parse(&cfg) == Trip::parse(&again(Trip::parse(&cfg).to_text())),
                "ttr" => Track::parse(&cfg) == Track::parse(&again(Track::parse(&cfg).to_text())),
                _ if name == "stnlinks.cfg" => {
                    let l = parse_stnlinks(&cfg);
                    l == parse_stnlinks(&again(l.iter().map(|x| x.to_text("a", "b")).collect()))
                }
                _ if name == "busstops.cfg" => {
                    let b = parse_busstops(&cfg);
                    b == parse_busstops(&again(b.iter().map(|x| x.to_text()).collect()))
                }
                _ => continue,
            };
            checked += 1;
            if !same {
                failures.push(f.display().to_string());
            }
        }
        println!("  {checked} TTData files written back");
        let hms = |s: f64| format!("{:02}:{:02}:{:02}", (s / 3600.0) as i64, (s / 60.0) as i64 % 60, s as i64 % 60);
        if let Some((l, t)) = lines.iter().find_map(|l| l.tours.iter().find(|t| t.runs && t.trips.iter().any(|x| x.stops.len() > 1)).map(|t| (l, t))) {
            let x = t.trips.iter().find(|x| x.stops.len() > 1).unwrap();
            let stops: Vec<String> = x.stops.iter().map(|s| format!("{} {}/{}", s.name, hms(s.arr), hms(s.dep))).collect();
            println!("  line {} tour {} ({}), trip {} #{} {} -> {} ({:.1} km): {}", l.name, t.number, t.days.words(), x.name, x.index, x.from, x.terminus, x.km, stops.join(", "));
        }
        let tt = Timetable::load(&dir.join("TTData"));
        let named = tt.trips.iter().flat_map(|t| &t.stops).filter(|s| !s.name.is_empty()).count();
        let stops: usize = tt.trips.iter().map(|t| t.stops.len()).sum();
        println!("  {} trips, {stops} stops ({named} named), {} links, {} bus stops", tt.trips.len(), tt.stn_links.len(), tt.bus_stops.len());
    }
    assert!(failures.is_empty(), "not the same after writing back: {failures:#?}");
}

#[test]
#[ignore = "needs a real OMSI 2 install: HUB_OMSI_REAL_ROOT"]
fn real_depot_files_buses_fonts_and_weather_read() {
    let Some(root) = root() else {
        eprintln!("HUB_OMSI_REAL_ROOT is not set: nothing checked");
        return;
    };
    let vfs = Vfs::new(root.clone(), None);
    let vehicles = vfs.resolve("Vehicles").unwrap();
    let (mut hofs, mut termini, mut trips, mut failures) = (0, 0, 0, Vec::new());
    for (folder, _) in vfs.list_dir("Vehicles").into_iter().filter(|(_, d)| *d) {
        for f in depot_files(&vehicles.join(&folder)) {
            let cfg = CfgFile::read(&f).unwrap();
            let h = Hof::parse(&cfg);
            let c = Comments::read(&cfg);
            let back = Hof::parse(&CfgFile::from_str(&f, &h.to_text(&c)));
            let back = Hof { codepage: h.codepage, ..back };
            if back != h || Comments::read(&CfgFile::from_str(&f, &h.to_text(&c))) != c {
                failures.push(f.display().to_string());
            }
            assert_eq!(h.info_trips.len(), h.info_busstop_lists.len());
            hofs += 1;
            termini += h.termini.len();
            trips += h.info_trips.len();
        }
    }
    println!("{hofs} depot files: {termini} destinations, {trips} routes; {} not the same written back", failures.len());
    assert!(failures.is_empty(), "depot files not the same after writing back: {failures:#?}");

    let mut buses = Vec::new();
    files_with(&vehicles, &["bus"], &mut buses);
    let (mut offered, mut with_cabin, mut paints, mut numbers, mut left) = (0, 0, 0, 0, 0);
    for f in &buses {
        let b = Bus::load(f).unwrap();
        if !b.has_friendly_name {
            continue;
        }
        offered += 1;
        paints += b.paints.len();
        numbers += b.numbers.len();
        left += usize::from(b.left_hand_drive() == Some(true));
        if let Some((seats, standing)) = capacity(&b) {
            with_cabin += 1;
            assert!(seats + standing > 0 || b.ai_only, "{}: an empty cabin", f.display());
        }
    }
    println!("{} .bus files, {offered} offered ({left} driven from the left), {with_cabin} with a cabin, {paints} paints, {numbers} fleet numbers", buses.len());

    let mut fonts = Vec::new();
    files_with(&root.join("Fonts"), &["oft"], &mut fonts);
    let names: usize = fonts.iter().map(|f| font::load_names(f).unwrap().len()).sum();
    println!("{} font files, {names} fonts", fonts.len());

    let mut owt = Vec::new();
    files_with(&root.join("Weather"), &["owt"], &mut owt);
    for f in &owt {
        let w = Weather::load(f).unwrap();
        assert!(!w.name.is_empty(), "{}: no name", f.display());
        let back = Weather { path: w.path.clone(), codepage: w.codepage, ..Weather::parse(&CfgFile::from_str(f, &w.to_text())) };
        assert_eq!(back, w, "{}: not the same written back", f.display());
        println!("  {}: {} - fog {} m, {} °C, {:?} {:.2}, wet {:.2}", f.file_name().unwrap().to_string_lossy(), w.name, w.fog_m, w.temperature, w.precipitation, w.intensity, w.ground_wet);
    }
}
