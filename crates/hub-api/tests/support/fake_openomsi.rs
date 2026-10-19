//! A fake openOMSI for the drive's end-to-end test (`tests/drive.rs`): a tiny program that the
//! test copies into a folder as `openomsi(.exe)` and that Omsi-Hub then starts as the game. It
//! runs no game and none of openOMSI's code. It does what the Omsi-Hub plugin inside a real
//! openOMSI would do on a short duty, through the same files (`hub_game::bridge`):
//!
//! * `--version`: says `openomsi 0.2.30` and ends (what the setup asks where no file says it);
//! * otherwise: reads the assignment Omsi-Hub wrote (`Plugins/omsihub/assignment.lua`, as
//!   text), writes the handshake, takes the duty its command line names (`--line`, `--tour`,
//!   `--trip`), reports the duty taken, a trip, a defect of the walk-around and a live event,
//!   writes the status and where the bus is, waits a few seconds (the app's bridge looks while
//!   the game runs), reports a second trip and the duty's end, and ends.
//!
//! What it was started with goes to `Plugins/omsihub/data/fake-args.txt`, for the test.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The value of a flag of the command line.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

/// The first number after `key` in a Lua text, from `from` on (`["issued"] = 1760000000,`).
fn number_after(text: &str, key: &str, from: usize) -> Option<u64> {
    let at = text[from..].find(key)? + from + key.len();
    let rest = text[at..].trim_start_matches([' ', '=', '\t']);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// The string after `key` in a Lua text (`["duty"] = "5 2 1",`).
fn string_after(text: &str, key: &str) -> String {
    let Some(at) = text.find(key) else { return String::new() };
    let rest = text[at + key.len()..].trim_start_matches([' ', '=', '\t']);
    let Some(rest) = rest.strip_prefix('"') else { return String::new() };
    rest.split('"').next().unwrap_or_default().to_string()
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

fn append(path: &Path, line: &str) {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).expect("reports file");
    writeln!(f, "{line}").expect("a report");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("openomsi 0.2.30");
        return;
    }
    let exe = std::env::current_exe().expect("the program's path");
    let plugin: PathBuf = exe.parent().expect("its folder").join("Plugins").join("omsihub");
    let data = plugin.join("data");
    std::fs::create_dir_all(&data).expect("the plugin's data folder");
    std::fs::write(data.join("fake-args.txt"), args.join("\n")).expect("the arguments written");

    // the assignment Omsi-Hub wrote beside the plugin
    let lua = std::fs::read_to_string(plugin.join("assignment.lua")).unwrap_or_default();
    let issued = number_after(&lua, "[\"issued\"]", 0).unwrap_or(0);
    let duty = string_after(&lua, "[\"duty\"]");
    let bus_id = lua.find("[\"crew\"] = {").and_then(|k| number_after(&lua, "[\"bus\"]", k)).unwrap_or(0);

    // the handshake: everything the add-on uses is there
    let functions = [
        "duty.get",
        "duty.trips",
        "files.append",
        "files.write",
        "duty.start",
        "ui.set",
        "ui.toast",
        "ui.remove",
        "files.read",
        "json.decode",
        "send",
        "duty.set_auto_ibis",
        "duty.type_ibis",
        "bus.set_display_font",
        "game.action",
        "duty.skip_to",
        "duty.finish",
        "duty.trip_stops",
        "audio.play",
        "duty.set_closed_stops",
        "traffic.list",
        "timetable.buses",
    ];
    let funcs: Vec<String> = functions.iter().map(|f| format!("\"{f}\":true")).collect();
    let hello = format!(
        "{{\"bridge\":1,\"plugin\":\"fake\",\"game\":\"0.2.30\",\"api\":1,\"platform\":\"{}\",\"language\":\"en\",\"functions\":{{{}}},\"issued\":{issued}}}",
        std::env::consts::OS,
        funcs.join(",")
    );
    std::fs::write(data.join("hello.json"), hello).expect("the handshake");

    let line = flag(&args, "--line").unwrap_or_default();
    let tour = flag(&args, "--tour").unwrap_or_default();
    let first: u32 = flag(&args, "--trip").and_then(|t| t.parse().ok()).unwrap_or(1);
    let bus = flag(&args, "--bus").unwrap_or_default();
    let map = flag(&args, "--map").unwrap_or_default();
    let reports = data.join("reports.jsonl");
    let report = |kind: &str, more: &str| {
        let sep = if more.is_empty() { "" } else { "," };
        append(&reports, &format!("{{\"kind\":\"{kind}\",\"issued\":{issued},\"duty\":\"{duty}\",\"at\":{:.0},\"clock\":28800{sep}{more}}}", now()));
    };
    let trip = |n: u32| {
        format!(
            "\"trip\":{n},\"trip_name\":\"{line} {n}\",\"trips\":{},\"number\":\"{line}\",\"terminus\":\"{line} Endstation\",\"departure\":{},\"arrival\":{},\"bus\":\"{bus}\",\"map\":\"{map}\",\"how\":\"arrived\",\"driving\":92,\"comfort\":88,\"tickets\":100,\"delay\":45,\"line\":\"{line}\",\"tour\":\"{tour}\"",
            first + 1,
            28_800 + (n - first) * 1_200,
            29_700 + (n - first) * 1_200
        )
    };
    report("duty_start", &format!("\"line\":\"{line}\",\"tour\":\"{tour}\""));
    report("trip", &trip(first));
    if bus_id > 0 {
        report("defect", &format!("\"defect\":\"tyres\",\"text\":\"front left\",\"bus_id\":{bus_id}"));
        report("live", &format!("\"event\":{{\"kind\":\"breakdown\",\"vehicle\":{bus_id}}}"));
    }
    std::fs::write(
        data.join("status.json"),
        format!(
            "{{\"issued\":{issued},\"line\":\"{line}\",\"tour\":\"{tour}\",\"trip\":{},\"trips\":{},\"delay\":45,\"stops\":6,\"passed\":2,\"trip_done\":false,\"at\":{:.0},\"clock\":30000}}",
            first + 1,
            first + 1,
            now()
        ),
    )
    .expect("the status");
    std::fs::write(
        data.join("live.json"),
        format!("{{\"issued\":{issued},\"at\":{:.0},\"clock\":30000,\"line\":\"{line}\",\"tour\":\"{tour}\",\"trip\":{},\"next\":1,\"delay\":45,\"stops\":[]}}", now(), first + 1),
    )
    .expect("where the bus is");

    // the game runs a while: the app's bridge looks in the meantime
    let secs: u64 = std::env::var("FAKE_OPENOMSI_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    std::thread::sleep(Duration::from_secs(secs));

    report("trip", &trip(first + 1));
    report("duty_end", &format!("\"line\":\"{line}\",\"tour\":\"{tour}\""));
}
