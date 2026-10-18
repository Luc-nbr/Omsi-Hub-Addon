//! The drivers on this computer and their service records, as the old launcher kept them
//! (`omsi-launcher-core`'s profiles, which the old Omsi-Hub's "Who drives" step and service
//! record showed).
//!
//! A driver is OMSI's own personnel file, `Drivers/<name>.odr`: the original OMSI 2 folder's
//! (only ever read) or openOMSI's content folder's, where a new one is made and where the game
//! writes its figures once it drives with `--driver` ([`launch_driver`]). Every run the game
//! drives is written by openOMSI into its own `~/.openomsi/sessions` (`omsi-career`'s
//! `write_session`), which Omsi-Hub reads and never changes; the trips the plugin reported are
//! Omsi-Hub's own (`hub_core::trips`).
//!
//! A driver deleted that has no file of the content folder's to delete (one of the original
//! installation, or known only from its sessions) is hidden from the list instead, in the data
//! folder's `drivers.json` (the old launcher's `hidden-drivers.txt` in `~/.openomsi` is read
//! too), which also keeps when a driver was deleted: the sessions from before are not theirs
//! any more when a new driver of the same name is made.
//!
//! The account (Omsi-Hub online, `online.rs`) belongs to one driver: the first sign-in on this
//! computer makes or chooses the driver of the account's display name and keeps it as the
//! account's (`account_driver` in the settings). The account's features - the friends and the
//! online state, the leaderboard, the cloud saves - are that driver's; every other driver on this
//! computer plays locally, without them ([`plays_online`]). Signed out, the link goes.
//!
//! Commands: `hub_profiles`, `hub_profile`, `hub_profile_create`, `hub_profile_delete`,
//! `hub_profile_select`, `hub_profile_personnel` (the personnel number and code a driver signs
//! on with, `make`: made now when there are none).

use crate::{arg, to_json, Answer};
use hub_core::settings::Settings;
use hub_core::trips::{trip_totals, trips_file, trips_of, utc_offset, TripRun, TripTotals};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A command of the drivers, or None when `cmd` is not one.
pub fn call(cmd: &str, args: &Value) -> Option<Answer> {
    Some(match cmd {
        "hub_profiles" => Ok(json!({ "names": list(), "current": chosen().unwrap_or_default(), "account": account_driver() })),
        "hub_profile" => arg::<String>(args, "name").and_then(|n| profile(&n)).and_then(to_json),
        "hub_profile_create" => arg::<String>(args, "name").and_then(|n| create(&n, &arg::<Option<String>>(args, "sex")?.unwrap_or_default())).and_then(to_json),
        "hub_profile_delete" => arg::<String>(args, "name").and_then(|n| delete(&n)).map(|_| json!(true)),
        "hub_profile_select" => arg::<String>(args, "name").and_then(|n| select(&n)),
        "hub_profile_personnel" => personnel(args),
        _ => return None,
    })
}

/// A driver's personnel number and code (`make`: made now when there are none; else None).
fn personnel(args: &Value) -> Answer {
    let name: String = arg(args, "name")?;
    let make = arg::<Option<bool>>(args, "make")?.unwrap_or(false);
    Ok(json!(crate::addon::personnel_of(&name, make).map(|p| json!({ "number": p.number, "code": p.code }))))
}

/// One run of the game as openOMSI writes it when the game ends (`omsi-career`'s
/// `write_session`).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Session {
    pub time: u64,
    pub driver: String,
    pub map: String,
    pub bus: String,
    pub line: Option<String>,
    pub tour: Option<String>,
    pub seconds: f64,
    pub metres: f64,
    pub stops: i32,
    pub early: i32,
    pub late: i32,
    pub tickets: i32,
    pub cash: f64,
    pub crashes: i32,
    pub hurt: i32,
    pub jolts: i32,
    pub driving: f64,
    pub comfort: f64,
    pub ticketing: f64,
}

/// A driver's service record.
#[derive(Serialize, Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub file: String,
    pub hours: f64,
    pub km: f64,
    pub xp: i64,
    pub level: i64,
    pub next_level_xp: i64,
    pub stops: i32,
    pub early: i32,
    pub late: i32,
    pub tickets: f64,
    pub cash: f64,
    pub crashes: i32,
    pub hurt: i32,
    pub rating_driving: f64,
    pub rating_comfort: f64,
    pub rating_tickets: f64,
    pub sessions: Vec<Session>,
    pub exists: bool,
    /// What every run comes to (`sessions` holds only the last forty).
    pub record: Record,
    /// The trips the plugin reported, the newest first: the last twenty, and what all of them
    /// come to.
    pub trips: Vec<TripRun>,
    pub trip_totals: TripTotals,
}

/// A driver's runs beyond the totals, for the service record: how many there are, since
/// when, the records, where and with which bus they drive most, and at which hour their runs
/// end.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Record {
    pub runs: usize,
    /// When the first run ended (Unix time; 0 without one).
    pub first: u64,
    pub longest: Option<Session>,
    pub furthest: Option<Session>,
    pub busiest: Option<Session>,
    /// The run with the most of its stops served on time (of those with five stops or more).
    pub most_punctual: Option<Session>,
    /// The three maps and buses driven most: (the session's map or bus, runs).
    pub maps: Vec<(String, usize)>,
    pub buses: Vec<(String, usize)>,
    /// Runs by the hour of the day (local time) they ended at.
    pub hours: [usize; 24],
    pub jolts: i64,
    /// Kilometres of the runs whose distance can be believed (see `believable_km`).
    pub km: f64,
}

// --- where things are ----------------------------------------------------------------------

/// openOMSI's own folder in the home folder (its sessions; only read).
fn openomsi_home() -> PathBuf {
    hub_core::paths::home_dir().join(".openomsi")
}

/// The OMSI 2 folder, when one is chosen.
fn root() -> Option<PathBuf> {
    Some(PathBuf::from(Settings::load().omsi2_folder)).filter(|p| !p.as_os_str().is_empty())
}

/// Where a driver's personnel file is written: openOMSI's content folder's `Drivers` (the
/// original installation is only ever read).
fn write_path(name: &str) -> Option<PathBuf> {
    hub_core::paths::content_dir().map(|c| c.join("Drivers").join(format!("{name}.odr")))
}

/// Where a driver's personnel file is read: the content folder's once there is one (the game
/// writes it there), else the original installation's (the stock `OMSI-Fan.odr`).
fn read_path(name: &str) -> Option<PathBuf> {
    let own = write_path(name).filter(|p| p.exists());
    own.or_else(|| root().map(|r| r.join("Drivers").join(format!("{name}.odr"))))
}

/// A plain name for a file (no folder in it).
fn plain(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.contains(['/', '\\', ':']) || name == "." || name == ".." {
        return Err("the name must be a plain file name".into());
    }
    Ok(name)
}

// --- the sessions ---------------------------------------------------------------------------

/// Every session openOMSI wrote in `dir`, the newest first.
fn sessions_in(dir: &Path) -> Vec<Session> {
    let mut out: Vec<Session> =
        std::fs::read_dir(dir).map(|rd| rd.flatten().filter_map(|e| std::fs::read_to_string(e.path()).ok()).filter_map(|t| serde_json::from_str::<Session>(&t).ok()).collect()).unwrap_or_default();
    out.sort_by_key(|s| std::cmp::Reverse(s.time));
    out
}

/// The sessions still a driver's: those from after the driver of the name was last deleted.
fn current_sessions(all: Vec<Session>, book: &Book) -> Vec<Session> {
    all.into_iter().filter(|s| s.time >= book.since(&s.driver)).collect()
}

fn sessions() -> Vec<Session> {
    current_sessions(sessions_in(&openomsi_home().join("sessions")), &Book::load())
}

/// Experience: a point per hundred metres, five per stop served on time, two per ticket, one
/// a minute, minus twenty per crash, fifty per pedestrian and one per jolt.
fn xp_of(s: &Session) -> i64 {
    let on_time = (s.stops - s.early - s.late).max(0) as i64;
    let xp = (s.metres / 100.0) as i64 + on_time * 5 + s.tickets as i64 * 2 + (s.seconds / 60.0) as i64 - s.crashes as i64 * 20 - s.hurt as i64 * 50 - s.jolts as i64;
    xp.max(0)
}

/// The level of `xp` (it grows with the square root) and the experience the next one needs.
fn level_of(xp: i64) -> (i64, i64) {
    let level = ((xp as f64 / 250.0).sqrt().floor() as i64) + 1;
    (level, level * level * 250)
}

/// Whether a run's distance can have been driven in its time: a broken odometer reading
/// once gave an hour's run two million kilometres (Omsi-Hub's rule: at most 100 km/h, and
/// any distance up to 10 km).
fn believable_km(s: &Session) -> bool {
    let km = s.metres / 1000.0;
    km.is_finite() && km > 0.0 && km <= (s.seconds / 3600.0 * 100.0).max(10.0)
}

/// The record of `runs`, their hours shifted by `utc_offset` seconds into local time.
pub fn record_of(runs: &[Session], utc_offset: i64) -> Record {
    let best = |key: &dyn Fn(&Session) -> Option<f64>| runs.iter().filter_map(|s| key(s).filter(|v| v.is_finite()).map(|v| (v, s))).max_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, s)| s.clone());
    let top = |name: &dyn Fn(&Session) -> &str| {
        let mut count: Vec<(String, usize)> = Vec::new();
        for s in runs {
            let n = name(s);
            if n.is_empty() {
                continue;
            }
            match count.iter_mut().find(|c| c.0 == n) {
                Some(c) => c.1 += 1,
                None => count.push((n.to_string(), 1)),
            }
        }
        // (the most first; equal counts in the order first driven)
        count.sort_by_key(|c| std::cmp::Reverse(c.1));
        count.truncate(3);
        count
    };
    let mut hours = [0usize; 24];
    for s in runs {
        hours[((s.time as i64 + utc_offset).rem_euclid(86400) / 3600) as usize] += 1;
    }
    Record {
        runs: runs.len(),
        first: runs.iter().map(|s| s.time).min().unwrap_or(0),
        longest: best(&|s| (s.seconds > 0.0).then_some(s.seconds)),
        furthest: best(&|s| believable_km(s).then_some(s.metres)),
        busiest: best(&|s| (s.tickets > 0).then_some(s.tickets as f64)),
        most_punctual: best(&|s| (s.stops >= 5).then(|| (s.stops - s.early - s.late).max(0) as f64 / s.stops as f64)),
        maps: top(&|s| s.map.as_str()),
        buses: top(&|s| s.bus.as_str()),
        hours,
        jolts: runs.iter().map(|s| s.jolts as i64).sum(),
        km: runs.iter().filter(|s| believable_km(s)).map(|s| s.metres / 1000.0).sum(),
    }
}

// --- the hidden drivers and when each was deleted -------------------------------------------

/// The data folder's `drivers.json`: the drivers hidden from the list, those of the old
/// launcher's list shown again, and when each name was last deleted (lower case -> Unix time).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
struct Book {
    hidden: Vec<String>,
    shown: Vec<String>,
    deleted: BTreeMap<String, u64>,
}

impl Book {
    fn path() -> PathBuf {
        hub_core::data_dir().join("drivers.json")
    }

    fn load() -> Book {
        let mut b: Book = std::fs::read_to_string(Self::path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        // (the old launcher's list, in openOMSI's folder: read, never written)
        let old = std::fs::read_to_string(openomsi_home().join("hidden-drivers.txt")).unwrap_or_default();
        for n in old.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !b.hidden.iter().any(|h| h.eq_ignore_ascii_case(n)) && !b.shown.iter().any(|h| h.eq_ignore_ascii_case(n)) {
                b.hidden.push(n.to_string());
            }
        }
        b
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let body = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn is_hidden(&self, name: &str) -> bool {
        self.hidden.iter().any(|h| h.eq_ignore_ascii_case(name.trim()))
    }

    /// Hide `name` (`hide`) or show it again (a driver made anew under that name; one the old
    /// launcher's list hides is marked shown, as that list is not ours to change).
    fn set_hidden(&mut self, name: &str, hide: bool) {
        let name = name.trim();
        self.hidden.retain(|h| !h.eq_ignore_ascii_case(name));
        self.shown.retain(|h| !h.eq_ignore_ascii_case(name));
        if hide {
            self.hidden.push(name.to_string());
        } else {
            self.shown.push(name.to_string());
        }
    }

    /// From when on the sessions under `name` are its present driver's.
    fn since(&self, name: &str) -> u64 {
        self.deleted.get(&name.trim().to_lowercase()).copied().unwrap_or(0)
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// --- the commands ---------------------------------------------------------------------------

/// The `.odr` files' names in `dir`.
fn odr_names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("odr"))).filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string())).collect()
        })
        .unwrap_or_default()
}

/// The drivers on this computer: OMSI 2's personnel files, the content folder's, those known
/// from openOMSI's sessions, and the driver chosen; the hidden ones left out.
pub fn list() -> Vec<String> {
    let book = Book::load();
    let mut names: Vec<String> = Vec::new();
    let mut add = |n: &str| {
        let n = n.trim();
        if !n.is_empty() && !names.iter().any(|x| x.eq_ignore_ascii_case(n)) {
            names.push(n.to_string());
        }
    };
    if let Some(r) = root() {
        odr_names(&r.join("Drivers")).iter().for_each(|n| add(n));
    }
    if let Some(c) = hub_core::paths::content_dir() {
        odr_names(&c.join("Drivers")).iter().for_each(|n| add(n));
    }
    for s in current_sessions(sessions_in(&openomsi_home().join("sessions")), &book) {
        add(&s.driver);
    }
    if let Some(d) = chosen() {
        add(&d);
    }
    names.retain(|n| !book.is_hidden(n));
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// The service record of `name`: its personnel file's figures where it has one (the game
/// keeps them), else what its runs come to.
pub fn profile(name: &str) -> Result<Profile, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a profile needs a name".into());
    }
    let driver = read_path(name).and_then(|f| omsi_content::driver::Driver::load(&f).ok());
    let mine: Vec<Session> = sessions().into_iter().filter(|s| s.driver.eq_ignore_ascii_case(name)).collect();
    let xp: i64 = mine.iter().map(xp_of).sum();
    let (level, next) = level_of(xp);
    let hours = mine.iter().map(|s| s.seconds).sum::<f64>() / 3600.0;
    let (km, stops, early, late, tickets, cash, crashes, hurt, rating) = match &driver {
        Some(d) => (
            d.hektom / 10.0,
            d.bus_stops[0],
            d.bus_stops[1],
            d.bus_stops[2],
            d.tickets[0],
            d.tickets[1],
            d.crashes[0],
            d.crashes[1],
            [d.driving_percent(), d.comfort_percent().unwrap_or(100.0), d.ticket_percent().unwrap_or(100.0)],
        ),
        None => {
            let km = mine.iter().map(|s| s.metres).sum::<f64>() / 1000.0;
            let n = mine.len().max(1) as f64;
            (
                km,
                mine.iter().map(|s| s.stops).sum(),
                mine.iter().map(|s| s.early).sum(),
                mine.iter().map(|s| s.late).sum(),
                mine.iter().map(|s| s.tickets as f64).sum(),
                mine.iter().map(|s| s.cash).sum(),
                mine.iter().map(|s| s.crashes).sum(),
                mine.iter().map(|s| s.hurt).sum(),
                [mine.iter().map(|s| s.driving).sum::<f64>() / n, mine.iter().map(|s| s.comfort).sum::<f64>() / n, mine.iter().map(|s| s.ticketing).sum::<f64>() / n],
            )
        }
    };
    let record = record_of(&mine, utc_offset());
    let trips = trips_of(&hub_core::data_dir(), name);
    let totals = trip_totals(&trips);
    Ok(Profile {
        name: name.to_string(),
        file: format!("Drivers/{name}.odr"),
        hours,
        km,
        xp,
        level,
        next_level_xp: next,
        stops,
        early,
        late,
        tickets,
        cash,
        crashes,
        hurt,
        rating_driving: rating[0],
        rating_comfort: rating[1],
        rating_tickets: rating[2],
        sessions: mine.into_iter().take(40).collect(),
        exists: driver.is_some(),
        record,
        trips: trips.into_iter().take(20).collect(),
        trip_totals: totals,
    })
}

/// Make the personnel file of a new driver (OMSI's own `.odr`) in the content folder, so the
/// game can add every run to it, and choose them.
pub fn create(name: &str, sex: &str) -> Result<Profile, String> {
    let name = plain(name)?;
    let file = write_path(name).ok_or("openOMSI's content folder is not known: choose the openOMSI folder in the settings")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if !file.exists() && !read_path(name).is_some_and(|p| p.exists()) {
        let sex = if sex.trim().is_empty() { "M" } else { sex.trim() };
        let d = omsi_content::driver::Driver { path: file.clone(), name: name.to_string(), sex: sex.to_string(), ..Default::default() };
        d.save(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    // (a driver deleted and hidden before is back under this name)
    let mut book = Book::load();
    book.set_hidden(name, false);
    book.save()?;
    select(name)?;
    profile(name)
}

/// Delete a driver: its personnel file of the content folder and its trips; one of the
/// original installation or known only from openOMSI's sessions is hidden instead.
pub fn delete(name: &str) -> Result<(), String> {
    let name = plain(name)?;
    // (the account's driver is the account's while it is signed in)
    if account_driver().is_some_and(|a| a.eq_ignore_ascii_case(name)) {
        return Err(crate::online::tr("This driver has your account: sign out first."));
    }
    if let Some(file) = write_path(name).filter(|f| f.exists()) {
        std::fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    let mut book = Book::load();
    let original = root().is_some_and(|r| r.join("Drivers").join(format!("{name}.odr")).exists());
    let known = sessions().iter().any(|s| s.driver.eq_ignore_ascii_case(name));
    if original || known {
        book.set_hidden(name, true);
    }
    // (the sessions so far are not a new driver's of the same name)
    book.deleted.insert(name.to_lowercase(), now());
    book.save()?;
    match std::fs::remove_file(trips_file(&hub_core::data_dir(), name)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
        _ => {}
    }
    // (the driver chosen is gone: the next one on the list, as the step chooses it)
    if crate::launcher::driver().eq_ignore_ascii_case(name) {
        let mut s = Settings::load();
        match list().into_iter().next() {
            Some(n) => s.other.insert("driver".into(), Value::String(n)),
            None => s.other.remove("driver"),
        };
        s.save().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The driver the next duty is driven by.
pub fn select(name: &str) -> Answer {
    let name = name.trim();
    if name.is_empty() {
        return Err("a profile needs a name".into());
    }
    let mut s = Settings::load();
    s.other.insert("driver".into(), Value::String(name.to_string()));
    s.save().map_err(|e| e.to_string())?;
    Ok(json!({ "profile": name }))
}

// --- the account's driver ----------------------------------------------------------------------

/// The settings' key of the driver the account belongs to.
const ACCOUNT_KEY: &str = "account_driver";

/// The driver the account on this computer belongs to, when one is signed in.
pub fn account_driver() -> Option<String> {
    Settings::load().other.get(ACCOUNT_KEY).and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty()).map(str::to_string)
}

/// The account's features are the driver's who drives now: the account's own driver (or any
/// driver while no account is linked).
pub fn plays_online() -> bool {
    online_for(account_driver().as_deref(), chosen().as_deref())
}

/// Whether the driver `chosen` has the account's features, the account being `account`'s.
fn online_for(account: Option<&str>, chosen: Option<&str>) -> bool {
    match (account, chosen) {
        (Some(a), Some(d)) => d.trim().eq_ignore_ascii_case(a.trim()),
        _ => true,
    }
}

/// Signed in with the display name `name`: the first time, the driver of that name is the
/// account's - made (a personnel file of OMSI's in the content folder) when there is none, and
/// chosen. Answers whether the link was made now.
pub fn link_account(name: &str) -> bool {
    let Ok(name) = plain(name) else { return false };
    if account_driver().is_some() {
        return false;
    }
    let known = list().iter().any(|n| n.eq_ignore_ascii_case(name));
    if !known && create(name, "").is_err() {
        // (no content folder yet: the name is the driver's all the same, its file made later)
        let _ = select(name);
    }
    let real = list().into_iter().find(|n| n.eq_ignore_ascii_case(name)).unwrap_or_else(|| name.to_string());
    let mut s = Settings::load();
    s.other.insert(ACCOUNT_KEY.into(), Value::String(real.clone()));
    s.other.insert("driver".into(), Value::String(real));
    s.save().is_ok()
}

/// Signed out (or the account deleted): no driver is the account's any more.
pub fn unlink_account() {
    let mut s = Settings::load();
    if s.other.remove(ACCOUNT_KEY).is_some() {
        let _ = s.save();
    }
}

/// The driver chosen in the settings (`driver`), when one is.
fn chosen() -> Option<String> {
    Settings::load().other.get("driver").and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty()).map(str::to_string)
}

/// The driver the game is started with (`--driver Drivers/<name>.odr`, the name the others
/// see in a LAN game): the duty's driver, else the one chosen; none when no driver was ever
/// chosen (the game's own then).
pub fn launch_driver(duty_driver: Option<&str>) -> Option<String> {
    let name = duty_driver.map(str::trim).filter(|d| !d.is_empty()).map(str::to_string).or_else(chosen)?;
    plain(&name).ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(time: u64, driver: &str, metres: f64, seconds: f64) -> Session {
        Session {
            time,
            driver: driver.into(),
            map: "maps/Grundorf/global.cfg".into(),
            bus: "Vehicles/MAN_SD200/SD202.bus".into(),
            metres,
            seconds,
            stops: 10,
            early: 1,
            late: 2,
            tickets: 4,
            ..Session::default()
        }
    }

    #[test]
    fn experience_and_levels_as_the_old_launcher_counted_them() {
        // (5 km, 7 stops on time, 4 tickets, 30 minutes: 50 + 35 + 8 + 30)
        assert_eq!(xp_of(&run(1, "Luc", 5000.0, 1800.0)), 123);
        assert_eq!(xp_of(&Session { crashes: 9, ..run(1, "Luc", 100.0, 60.0) }), 0);
        assert_eq!(level_of(0), (1, 250));
        assert_eq!(level_of(249), (1, 250));
        assert_eq!(level_of(250), (2, 1000));
        assert_eq!(level_of(1000), (3, 2250));
    }

    #[test]
    fn the_record_of_the_runs() {
        let runs = vec![run(3600, "Luc", 12_000.0, 3600.0), run(7200, "Luc", 2_000_000.0, 3600.0), run(10_800, "Luc", 3000.0, 600.0)];
        let r = record_of(&runs, 0);
        assert_eq!(r.runs, 3);
        assert_eq!(r.first, 3600);
        // (the broken odometer's run is not the furthest, and its kilometres do not count)
        assert_eq!(r.furthest.as_ref().map(|s| s.time), Some(3600));
        assert!((r.km - 15.0).abs() < 1e-9);
        assert_eq!((r.hours[1], r.hours[2], r.hours[3]), (1, 1, 1));
        assert_eq!(r.maps, vec![("maps/Grundorf/global.cfg".to_string(), 3)]);
    }

    #[test]
    fn sessions_from_before_a_deletion_are_not_the_new_drivers() {
        let mut book = Book::default();
        book.deleted.insert("luc".into(), 5000);
        let left = current_sessions(vec![run(4000, "Luc", 1.0, 1.0), run(6000, "luc", 1.0, 1.0), run(100, "Anna", 1.0, 1.0)], &book);
        assert_eq!(left.iter().map(|s| s.time).collect::<Vec<_>>(), vec![6000, 100]);
    }

    #[test]
    fn hiding_and_showing_again() {
        let mut book = Book::default();
        book.set_hidden("OMSI-Fan", true);
        assert!(book.is_hidden("omsi-fan"));
        book.set_hidden("omsi-fan", false);
        assert!(!book.is_hidden("OMSI-Fan"));
        assert_eq!(book.shown, vec!["omsi-fan".to_string()]);
    }

    #[test]
    fn a_session_as_openomsi_writes_it() {
        let text = r#"{"time":1760000000,"driver":"Luc","driver_file":"C:/x/Drivers/Luc.odr","map":"maps/Grundorf/global.cfg","bus":"Vehicles/MAN_SD200/SD202.bus","line":"5","tour":null,"seconds":1800.5,"metres":9000.0,"stops":12,"early":0,"late":1,"tickets":6,"cash":9.6,"crashes":0,"hurt":0,"jolts":2,"boarded":20,"served":18,"driving":96.0,"comfort":88.0,"ticketing":100.0}"#;
        let s: Session = serde_json::from_str(text).unwrap();
        assert_eq!((s.driver.as_str(), s.line.as_deref(), s.tickets, s.jolts), ("Luc", Some("5"), 6, 2));
    }

    #[test]
    fn the_account_is_its_drivers_alone() {
        assert!(online_for(None, Some("Anna")));
        assert!(online_for(Some("Luc"), Some("luc")));
        assert!(online_for(Some("Luc"), None));
        assert!(!online_for(Some("Luc"), Some("Anna")));
    }

    #[test]
    fn names_are_plain_file_names() {
        assert!(plain("Luc").is_ok());
        for bad in ["", "  ", "a/b", "a\\b", "C:x", ".."] {
            assert!(plain(bad).is_err(), "{bad}");
        }
    }
}
