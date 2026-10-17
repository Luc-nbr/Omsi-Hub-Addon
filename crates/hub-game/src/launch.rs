//! The game's command line: what Omsi-Hub asks of the player's openOMSI when it starts it.
//!
//! Only openOMSI's official flags are used (`openomsi --help`, the same from 0.2.22 on). What
//! only the old fork's own game knew (`--company`, `--duty-leg`, `--free-line`) is not a flag
//! here: the plugin does it in the game instead, through the assignment (`bridge`) and the
//! plugin API's `duty.start`.
//!
//! [`LaunchSpec`] is the drive as Omsi-Hub plans it; [`LaunchSpec::args`] gives its flags in a
//! fixed order, so the same drive always gives the same command line (a test compares them).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Every flag Omsi-Hub may pass: openOMSI's own, as its `--help` lists them. A test keeps
/// [`LaunchSpec::args`] to these.
pub const OFFICIAL_FLAGS: &[&str] = &[
    "--root",
    "--map",
    "--bus",
    "--entry",
    "--spawn",
    "--auto-entry",
    "--paint",
    "--plate",
    "--number",
    "--hof",
    "--setvar",
    "--setstr",
    "--date",
    "--time",
    "--keep-time",
    "--season",
    "--weather",
    "--traffic",
    "--passengers",
    "--riders",
    "--schedule",
    "--no-timetable-buses",
    "--line",
    "--tour",
    "--trip",
    "--whole-tour",
    "--autostart",
    "--on-foot",
    "--driver",
    "--view",
    "--drive-keys",
    "--lan-host",
    "--lan-join",
    "--lan-name",
    "--situation",
    "--tutorial",
    "--refuel",
    "--wash",
    "--repair",
    "--dirt",
    "--no-menu",
    "--menu",
    "--content-zip",
    "--export-glb",
    "--version",
];

/// LAN play: host a session, or join one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lan {
    /// Host on this UDP port (0: openOMSI's default port).
    Host(u16),
    /// Join `ip[:port]`, or `auto` to find a host on the local network.
    Join(String),
}

/// A drive as Omsi-Hub starts it. Empty fields are left to the game (its own defaults).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchSpec {
    /// The OMSI 2 folder.
    pub root: Option<PathBuf>,
    /// The map's `global.cfg`, relative to the OMSI 2 folder (`maps/Grundorf/global.cfg`).
    pub map: Option<String>,
    /// The bus's `.bus` file, relative to the OMSI 2 folder.
    pub bus: Option<String>,
    /// The entry point (`global.cfg`'s list) the bus is put at.
    pub entry: Option<usize>,
    /// Or a place of its own: `x,y,heading[,z]`.
    pub spawn: Option<String>,
    /// The entry point picked by the game from the duty's first stop.
    pub auto_entry: bool,
    /// The livery (`.cti` item name or index), the number plate and the fleet number.
    pub paint: Option<String>,
    pub plate: Option<String>,
    pub number: Option<String>,
    /// The depot (`.hof`) file's name.
    pub hof: Option<String>,
    /// The bus's script variables and string variables set after it is put down.
    pub set_vars: Vec<(String, f32)>,
    pub set_strings: Vec<(String, String)>,
    /// `YYYY-MM-DD` and `HH:MM`.
    pub date: Option<String>,
    pub time: Option<String>,
    /// The clock stays at `time` however late the duty's first trip leaves.
    pub keep_time: bool,
    /// `spring`/`summer`/`autumn`/`winter`, with `-early`/`-mid`/`-late`.
    pub season: Option<String>,
    /// A weather file relative to the OMSI 2 folder, or `natural`/`cycle`/`tomorrow`.
    pub weather: Option<String>,
    /// AI traffic vehicles around the camera.
    pub traffic: Option<u32>,
    /// Passengers at the stops, and those seated at the start.
    pub passengers: bool,
    pub riders: Option<u32>,
    /// The timetable runs (scheduled AI buses); with `no_timetable_buses` only the player's duty.
    pub schedule: bool,
    pub no_timetable_buses: bool,
    /// The player's line, tour and trip of the timetable (the trip: `HH:MM` or its place, 1 =
    /// the first), and the rest of the tour after that trip.
    pub line: Option<String>,
    pub tour: Option<String>,
    pub trip: Option<String>,
    pub whole_tour: bool,
    /// The bus put into service at the start (Shift+U).
    pub autostart: bool,
    /// Start as a pedestrian beside the bus.
    pub on_foot: bool,
    /// The driver's personnel file (`Drivers/<name>.odr`).
    pub driver: Option<String>,
    /// `driver`, `pax`, `outside` or `free`.
    pub view: Option<String>,
    /// `simple`, `wasd`, `arrows` or `omsi`.
    pub drive_keys: Option<String>,
    pub lan: Option<Lan>,
    pub lan_name: Option<String>,
    /// A situation (`.osn`) to continue, or one of OMSI's tutorials (1..4).
    pub situation: Option<String>,
    pub tutorial: Option<usize>,
    /// The depot's services before the drive.
    pub refuel: bool,
    pub wash: bool,
    pub repair: bool,
    /// How dirty the bus starts (0..1).
    pub dirt: Option<f32>,
    /// Straight into the drive (no menu), or the game's menu first.
    pub no_menu: bool,
    pub menu: bool,
    /// Extra content archives mounted for this drive.
    pub content_zips: Vec<PathBuf>,
    /// The bus written as glTF into this file, and the game ended (`--export-glb`: what
    /// openOMSI's launcher draws its bus preview from); nothing is driven.
    pub export_glb: Option<PathBuf>,
}

/// Why a [`LaunchSpec`] cannot be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecError {
    BadDate(String),
    BadTime(String),
    /// A tour or a trip without its line.
    TourWithoutLine,
    /// A trip without its tour.
    TripWithoutTour,
    /// A value with a line break or a NUL (it would not reach the game as one argument).
    BadValue(&'static str),
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecError::BadDate(d) => write!(f, "the date {d:?} is not YYYY-MM-DD"),
            SpecError::BadTime(t) => write!(f, "the time {t:?} is not HH:MM"),
            SpecError::TourWithoutLine => write!(f, "a tour needs its line"),
            SpecError::TripWithoutTour => write!(f, "a trip needs its tour"),
            SpecError::BadValue(flag) => write!(f, "the value of {flag} has a line break"),
        }
    }
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&s[5..7].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&s[8..10].parse::<u32>().unwrap_or(0))
}

fn is_time(s: &str) -> bool {
    let Some((h, m)) = s.split_once(':') else { return false };
    h.len() <= 2 && m.len() == 2 && h.parse::<u32>().is_ok_and(|h| h < 24) && m.parse::<u32>().is_ok_and(|m| m < 60)
}

/// `name=value,name=value`, as `--setvar`/`--setstr` take them.
fn pairs<V: std::fmt::Display>(list: &[(String, V)]) -> String {
    list.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(",")
}

impl LaunchSpec {
    /// The spec checked: dates and times in their forms, a tour with its line.
    pub fn validate(&self) -> Result<(), SpecError> {
        if let Some(d) = self.date.as_deref().filter(|d| !is_date(d)) {
            return Err(SpecError::BadDate(d.into()));
        }
        if let Some(t) = self.time.as_deref().filter(|t| !is_time(t)) {
            return Err(SpecError::BadTime(t.into()));
        }
        if self.tour.is_some() && self.line.is_none() {
            return Err(SpecError::TourWithoutLine);
        }
        if self.trip.is_some() && self.tour.is_none() {
            return Err(SpecError::TripWithoutTour);
        }
        Ok(())
    }

    /// The flags, in a fixed order. A line implies the timetable (`--schedule`), as the game
    /// needs it for the duty.
    pub fn args(&self) -> Result<Vec<String>, SpecError> {
        self.validate()?;
        let mut a: Vec<String> = Vec::new();
        let val = |a: &mut Vec<String>, flag: &'static str, v: String| -> Result<(), SpecError> {
            if v.contains(['\n', '\r', '\0']) {
                return Err(SpecError::BadValue(flag));
            }
            a.push(flag.to_string());
            a.push(v);
            Ok(())
        };
        let flag = |a: &mut Vec<String>, flag: &'static str, on: bool| {
            if on {
                a.push(flag.to_string());
            }
        };
        if let Some(r) = &self.root {
            val(&mut a, "--root", r.to_string_lossy().into_owned())?;
        }
        if let Some(t) = self.tutorial {
            val(&mut a, "--tutorial", t.to_string())?;
        }
        if let Some(s) = &self.situation {
            val(&mut a, "--situation", s.clone())?;
        }
        for (f, v) in [("--map", &self.map), ("--bus", &self.bus)] {
            if let Some(v) = v {
                val(&mut a, f, v.replace('\\', "/"))?;
            }
        }
        if let Some(e) = self.entry {
            val(&mut a, "--entry", e.to_string())?;
        }
        if let Some(s) = &self.spawn {
            val(&mut a, "--spawn", s.clone())?;
        }
        flag(&mut a, "--auto-entry", self.auto_entry);
        for (f, v) in [("--paint", &self.paint), ("--plate", &self.plate), ("--number", &self.number), ("--hof", &self.hof)] {
            if let Some(v) = v {
                val(&mut a, f, v.clone())?;
            }
        }
        if !self.set_vars.is_empty() {
            val(&mut a, "--setvar", pairs(&self.set_vars))?;
        }
        if !self.set_strings.is_empty() {
            val(&mut a, "--setstr", pairs(&self.set_strings))?;
        }
        for (f, v) in [("--date", &self.date), ("--time", &self.time)] {
            if let Some(v) = v {
                val(&mut a, f, v.clone())?;
            }
        }
        flag(&mut a, "--keep-time", self.keep_time);
        for (f, v) in [("--season", &self.season), ("--weather", &self.weather)] {
            if let Some(v) = v {
                val(&mut a, f, v.replace('\\', "/"))?;
            }
        }
        if let Some(t) = self.traffic {
            val(&mut a, "--traffic", t.to_string())?;
        }
        flag(&mut a, "--passengers", self.passengers);
        if let Some(r) = self.riders {
            val(&mut a, "--riders", r.to_string())?;
        }
        flag(&mut a, "--schedule", self.schedule || self.line.is_some());
        flag(&mut a, "--no-timetable-buses", self.no_timetable_buses);
        for (f, v) in [("--line", &self.line), ("--tour", &self.tour), ("--trip", &self.trip)] {
            if let Some(v) = v {
                val(&mut a, f, v.clone())?;
            }
        }
        flag(&mut a, "--whole-tour", self.whole_tour);
        flag(&mut a, "--autostart", self.autostart);
        flag(&mut a, "--on-foot", self.on_foot);
        for (f, v) in [("--driver", &self.driver), ("--view", &self.view), ("--drive-keys", &self.drive_keys)] {
            if let Some(v) = v {
                val(&mut a, f, v.clone())?;
            }
        }
        match &self.lan {
            Some(Lan::Host(port)) => val(&mut a, "--lan-host", port.to_string())?,
            Some(Lan::Join(to)) => val(&mut a, "--lan-join", to.clone())?,
            None => {}
        }
        if let Some(n) = &self.lan_name {
            val(&mut a, "--lan-name", n.clone())?;
        }
        flag(&mut a, "--refuel", self.refuel);
        flag(&mut a, "--wash", self.wash);
        flag(&mut a, "--repair", self.repair);
        if let Some(d) = self.dirt {
            val(&mut a, "--dirt", format!("{}", d.clamp(0.0, 1.0)))?;
        }
        flag(&mut a, "--no-menu", self.no_menu);
        flag(&mut a, "--menu", self.menu && !self.no_menu);
        for z in &self.content_zips {
            val(&mut a, "--content-zip", z.to_string_lossy().into_owned())?;
        }
        if let Some(out) = &self.export_glb {
            val(&mut a, "--export-glb", out.to_string_lossy().into_owned())?;
        }
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duty() -> LaunchSpec {
        LaunchSpec {
            root: Some(PathBuf::from("C:/OMSI 2")),
            map: Some("maps\\Grundorf\\global.cfg".into()),
            bus: Some("Vehicles/MAN_SD202/SD202.bus".into()),
            paint: Some("BVG".into()),
            set_vars: vec![("door_mode".into(), 1.0)],
            date: Some("1989-05-30".into()),
            time: Some("08:15".into()),
            line: Some("10".into()),
            tour: Some("2".into()),
            autostart: true,
            no_menu: true,
            ..LaunchSpec::default()
        }
    }

    #[test]
    fn a_duty_gives_its_official_flags_in_order() {
        let a = duty().args().unwrap();
        assert_eq!(
            a,
            [
                "--root",
                "C:/OMSI 2",
                "--map",
                "maps/Grundorf/global.cfg",
                "--bus",
                "Vehicles/MAN_SD202/SD202.bus",
                "--paint",
                "BVG",
                "--setvar",
                "door_mode=1",
                "--date",
                "1989-05-30",
                "--time",
                "08:15",
                "--schedule",
                "--line",
                "10",
                "--tour",
                "2",
                "--autostart",
                "--no-menu"
            ]
        );
    }

    #[test]
    fn only_official_flags_are_ever_passed() {
        let all = LaunchSpec {
            entry: Some(1),
            spawn: Some("1,2,90".into()),
            auto_entry: true,
            plate: Some("B-AB 1".into()),
            number: Some("4711".into()),
            hof: Some("Berlin.hof".into()),
            set_strings: vec![("ident".into(), "X".into())],
            keep_time: true,
            season: Some("autumn-late".into()),
            weather: Some("Weather\\Rain.owt".into()),
            traffic: Some(20),
            passengers: true,
            riders: Some(3),
            schedule: true,
            no_timetable_buses: true,
            trip: Some("08:30".into()),
            whole_tour: true,
            on_foot: true,
            driver: Some("Drivers/Me.odr".into()),
            view: Some("driver".into()),
            drive_keys: Some("wasd".into()),
            lan: Some(Lan::Join("auto".into())),
            lan_name: Some("Luc".into()),
            situation: Some("maps/x/laststn.osn".into()),
            tutorial: Some(1),
            refuel: true,
            wash: true,
            repair: true,
            dirt: Some(2.0),
            menu: true,
            no_menu: false,
            content_zips: vec![PathBuf::from("a.zip")],
            ..duty()
        };
        let a = all.args().unwrap();
        for f in a.iter().filter(|x| x.starts_with("--")) {
            assert!(OFFICIAL_FLAGS.contains(&f.as_str()), "{f} is not an official flag");
        }
        // (the old fork's own flags never)
        assert!(!a.iter().any(|x| x == "--company" || x == "--duty-leg" || x == "--free-line"));
        assert!(a.windows(2).any(|w| w == ["--dirt", "1"]));
        assert!(a.windows(2).any(|w| w == ["--weather", "Weather/Rain.owt"]));
    }

    #[test]
    fn a_wrong_spec_is_refused() {
        assert_eq!(LaunchSpec { date: Some("30-05-1989".into()), ..duty() }.args(), Err(SpecError::BadDate("30-05-1989".into())));
        assert_eq!(LaunchSpec { time: Some("25:00".into()), ..duty() }.args(), Err(SpecError::BadTime("25:00".into())));
        assert_eq!(LaunchSpec { line: None, ..duty() }.args(), Err(SpecError::TourWithoutLine));
        assert_eq!(LaunchSpec { tour: None, trip: Some("1".into()), ..duty() }.args(), Err(SpecError::TripWithoutTour));
        assert_eq!(LaunchSpec { plate: Some("a\nb".into()), ..duty() }.args(), Err(SpecError::BadValue("--plate")));
        assert!(is_time("8:05") && !is_time("08:5") && is_date("2026-10-17") && !is_date("2026-13-01"));
    }
}
