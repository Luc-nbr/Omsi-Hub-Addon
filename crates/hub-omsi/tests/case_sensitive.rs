//! OMSI 2's content names its files carelessly (`Vehicles/MAN_SD202/sd202.bus` named as
//! `vehicles/man_sd202/SD202.bus` in a `.bus` or a map), and Windows does not mind. Linux's file
//! systems do: there the content folders must be searched in any case by `hub_omsi::vfs`.
//!
//! This test only proves that on a case-sensitive file system (Linux, and macOS with a
//! case-sensitive volume). Elsewhere it says so and passes; with
//! `OMSIHUB_REQUIRE_CASE_SENSITIVE=1` (the Linux CI job) a case-insensitive temp folder fails it
//! instead, so the proof cannot be skipped there unseen.

use hub_omsi::vfs::{find_child, resolve_in, Vfs};
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-omsi-case-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Whether the file system of `d` tells names apart by their case.
fn case_sensitive(d: &std::path::Path) -> bool {
    std::fs::write(d.join("probe"), b"").unwrap();
    let sensitive = !d.join("PROBE").exists();
    std::fs::remove_file(d.join("probe")).unwrap();
    sensitive
}

/// The folder is case-sensitive, so a test in it proves something; else it says so (and fails
/// where `OMSIHUB_REQUIRE_CASE_SENSITIVE` asks for the proof).
fn provable(d: &std::path::Path) -> bool {
    if case_sensitive(d) {
        return true;
    }
    assert!(std::env::var_os("OMSIHUB_REQUIRE_CASE_SENSITIVE").filter(|v| !v.is_empty() && v != "0").is_none(), "the temp folder is not case-sensitive: this test proves nothing here");
    eprintln!("(the temp folder is case-insensitive: nothing to prove here)");
    false
}

fn write(p: PathBuf, text: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

#[test]
fn content_is_found_in_any_case_on_a_case_sensitive_file_system() {
    let root = dir("root");
    let content = dir("content");
    if !provable(&root) {
        return;
    }
    write(root.join("Maps/Grundorf/Global.CFG"), "[name]\nroot");
    write(root.join("Maps/Grundorf/TTData/Linie_10.ttl"), "[userallowed]\n1");
    write(root.join("Vehicles/MAN_SD202/sd202.bus"), "[friendlyname]\nMAN\nSD202\n");
    write(root.join("Vehicles/MAN_SD202/Texture/front.dds"), "");
    write(content.join("maps/grundorf/global.cfg"), "[name]\ncontent");
    write(content.join("vehicles/man_sd202/texture/back.dds"), "");

    // every part of a path in any case, either slash
    assert!(resolve_in(&root, "vehicles\\man_sd202\\SD202.BUS").is_some_and(|p| p.ends_with("Vehicles/MAN_SD202/sd202.bus")));
    assert_eq!(find_child(&root.join("Maps"), "grundorf"), Some(root.join("Maps/Grundorf")));
    assert_eq!(resolve_in(&root, "maps/grundorf/missing.cfg"), None);

    let vfs = Vfs::new(root.clone(), Some(content.clone()));
    // the content folder first, whatever case each folder has it in
    assert_eq!(vfs.read_cfg("MAPS/GRUNDORF/GLOBAL.CFG").unwrap().value("name"), Some("content"));
    assert!(vfs.is_file("maps/grundorf/ttdata/LINIE_10.TTL"));
    assert!(vfs.is_dir("VEHICLES/man_sd202"));
    // a folder listed from both, each name once
    let names: Vec<String> = vfs.list_dir("vehicles/MAN_SD202/TEXTURE").into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["back.dds", "front.dds"]);

    // two names that differ only in case: the exact one wins
    write(root.join("Maps/Grundorf/a.cfg"), "[name]\nlower");
    write(root.join("Maps/Grundorf/A.cfg"), "[name]\nupper");
    let only_root = Vfs::new(root.clone(), None);
    assert_eq!(only_root.read_cfg("Maps/Grundorf/A.cfg").unwrap().value("name"), Some("upper"));
    assert_eq!(only_root.read_cfg("Maps/Grundorf/a.cfg").unwrap().value("name"), Some("lower"));

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&content);
}

/// The readers find what OMSI's files name in another case than the disk has it: a map's
/// folders and files, a line's trips, a trip's track, Chrono's copies of the map's files, a
/// bus's model, cabin, rear section, paints and number lists; non-ASCII names too.
#[test]
fn the_readers_find_the_files_omsi_names_in_any_case() {
    use hub_omsi::hof::depot_files;
    use hub_omsi::map::{active_chrono_dirs, depot_hof_on, AiLists, Calendar, GlobalCfg};
    use hub_omsi::timetable::{lines_for, Timetable};
    use hub_omsi::vehicle::{capacity, Bus};
    use hub_omsi::weather::Weather;

    let root = dir("readers");
    if !provable(&root) {
        return;
    }
    let date = chrono::NaiveDate::from_ymd_opt(1989, 5, 30).unwrap();
    let m = root.join("MAPS/Grünau");
    write(m.join("GLOBAL.CFG"), "[name]\nGrünau\n\n[standarddepot]\nBusses\n\n[map]\n0\n0\ntile_0_0.map\n");
    write(m.join("TimeZone.TXT"), "[timezone]\n1\n\n[location]\n52.5\n13.25\n");
    write(m.join("holidays.TXT"), "[holiday]\n19890530\nA holiday\n");
    write(m.join("AILISTS.cfg"), "[aigroup_depot]\nBusses\nGrünau\n\n[aigroup_depot_typgroup_2]\nvehicles\\foo\\bar.BUS\n1\t\tRed\n[end]\n");
    // the line names its trip, the trip its track, each in another case than its file
    write(m.join("ttdata/Linie_10.TTL"), "[userallowed]\n\n[newtour]\n1\nBusses\n799\n\n[addtrip]\nlinie_10_HIN\n0\n360\n");
    write(m.join("ttdata/Linie_10_Hin.TTP"), "[trip]\nLINIE_10_hin_track\nMarkt\n10\n\n[station]\n1\n0\nBahnhof\n0\n0\n0\n0\n0\n\n[station]\n2\n1\nMarkt\n0\n0\n0\n0\n0\n\n[profile]\nstandard\n5\n");
    write(m.join("ttdata/linie_10_Hin_Track.ttr"), "[track_entry]\n7\n0\n0\n0\n100\n0\n\n[track_entry]\n8\n0\n0\n0\n200\n0\n");
    write(m.join("ttdata/BUSSTOPS.CFG"), "[busstop]\nBahnhof\n0\n1\n0\n0\n0\n");
    write(m.join("ttdata/stnlinks.Cfg"), "[StnLink]\n400\n1\n2\n0\n0\n0\n0\n0\n0\n");
    // a Chrono folder whose copy of the trip is named in yet another case
    write(m.join("CHRONO/0100_Works/chrono.CFG"), "[startdate]\n19890501\n");
    write(
        m.join("CHRONO/0100_Works/TTDATA/LINIE_10_HIN.ttp"),
        "[trip]\nLINIE_10_hin_track\nUmleitung\n10\n\n[station]\n1\n0\nBahnhof\n0\n0\n0\n0\n0\n\n[station]\n2\n1\nMarkt\n0\n0\n0\n0\n0\n\n[profile]\nstandard\n8\n",
    );
    write(m.join("CHRONO/0100_Works/AILISTS_#UPD.CFG"), "[aigroup_depot]\nBusses\nGrünau 1989\n");

    let vfs = Vfs::new(root.clone(), Some(dir("readers-content")));
    let g = GlobalCfg::load(&vfs.resolve("maps/GRÜNAU/global.cfg").unwrap()).unwrap();
    assert_eq!((g.name.as_str(), g.tiles.len(), g.timezone.and_then(|t| t.lat_lon())), ("Grünau", 1, Some((52.5, 13.25))));
    assert_eq!(Calendar::load(&m).unwrap().holidays.len(), 1);
    let chrono = active_chrono_dirs(&m, date);
    assert_eq!(chrono.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["0100_Works"]);
    assert_eq!(AiLists::with_chrono(&m, &[]).groups[0].typgroups[0].file, "vehicles/foo/bar.BUS");
    assert_eq!(depot_hof_on(&m, date).as_deref(), Some("Grünau 1989"));
    let tt = Timetable::load_with_chrono(&m.join("ttdata"), &chrono, &[]);
    let trip = tt.trip("LINIE_10_HIN").unwrap();
    assert_eq!((trip.terminus.as_str(), tt.trips.len()), ("Umleitung", 1));
    assert_eq!(tt.track_of(trip).map(|t| t.entries.len()), Some(2));
    assert_eq!((tt.bus_stops.len(), tt.stn_links.len()), (1, 1));
    let lines = lines_for(&vfs, "maps\\grünau\\Global.cfg", date).unwrap();
    assert_eq!(lines.len(), 1);
    let t = &lines[0].tours[0].trips[0];
    assert_eq!((t.name.as_str(), t.stops.len(), t.km, t.arrival - t.departure), ("linie_10_HIN", 2, 0.4, 480.0));
    assert!(!lines[0].tours[0].runs, "799 does not run on a public holiday");

    // a bus named `vehicles\foo\bar.BUS`, its files in other cases again
    let b = root.join("Vehicles/Foo");
    write(b.join("BAR.bus"), "[friendlyname]\nTestwerk\nBar\nRed\n\n[model]\nMODEL\\Model.CFG\n\n[passengercabin]\nModel\\PassengerCabin.cfg\n\n[number]\nREGS.org\n\n[registration_list]\nplates.ORG\n\n[couple_back]\nbar_TRAIL.bus\nfalse\n");
    write(b.join("Bar_trail.bus"), "[coupling_front]\n0\n0\n0\n\n[passengercabin]\nmodel\\PASSENGERCABIN.CFG\n");
    write(b.join("model/model.cfg"), "[CTC]\nColorscheme\nTEXTURE\\paints\n0\n");
    write(b.join("model/passengercabin.CFG"), "[passpos]\n0\n0\n0\n0.5\n0\n\n[passpos]\n0\n1\n0\n0\n0\n");
    write(b.join("Texture/Paints/red.CTI"), "[item]\nRed\nfarbschema\nred.dds\n");
    write(b.join("regs.ORG"), "1\n2\n");
    write(b.join("Plates.org"), "GR-V 1\nGR-V 2\n");
    write(b.join("Grünau.HOF"), "[name]\nGrünau\n");
    let bus = Bus::load(&vfs.resolve("vehicles\\foo\\bar.BUS").unwrap()).unwrap();
    assert_eq!((bus.name(), bus.paints.clone()), ("Testwerk Bar".to_string(), vec!["Red".to_string()]));
    assert_eq!(bus.numbers, [("1".to_string(), "GR-V 1".to_string()), ("2".to_string(), "GR-V 2".to_string())]);
    assert!(bus.couple_back_path().is_some_and(|p| p.ends_with("Bar_trail.bus")));
    // (both sections share the cabin file, each counted)
    assert_eq!(capacity(&bus), Some((2, 2)));
    assert_eq!(depot_files(&b).len(), 1);
    assert!(vfs.resolve("VEHICLES/foo/grÜnau.hof").is_some());

    write(root.join("Weather/grey.OWT"), "[name]\nGrey\n\n[fog]\n500\n1\n");
    assert_eq!(Weather::load(&vfs.resolve("weather/GREY.owt").unwrap()).unwrap().fog_m, 500.0);

    let _ = std::fs::remove_dir_all(&root);
}
