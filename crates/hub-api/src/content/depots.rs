//! The bus's depot files on the bus step: which of them the bus has and what each is for - the
//! map it belongs to, how many destinations it knows, whether it is the chosen map's - with
//! the one the drive takes marked and the one for the chosen map first; and the map's depot
//! file added when the bus lacks it (`hub_core::depot`, into openOMSI's content folder, never
//! the OMSI 2 folder).
//!
//! A bus that lacks the map's depot file gets, in the game, its own depot file of the same
//! place first, then the map's borrowed from another vehicle folder, then its first one. The
//! menu follows that order, so the row marked "in use" is the file the drive will have.
//!
//! * `hub_depots {bus, map: {file, name, friendly, hof}, date, chosen, line, maps}` - the menu,
//!   with what the drive takes (`used`: `{ row }`, `{ borrowed }` or `{}`), each row's fit to
//!   the map and the dialog's choices;
//! * `hub_depot_add {bus, want, source}` - the map's depot file `want` given to the bus from
//!   the copy `source`: `{ added: path }` or `{ refusal: { kind, what } }`.

use super::{bus_depot_files, closest_name, fold, keys_for, name_cmp, slashed};
use crate::{arg, Answer};
use hub_core::depot::{Refusal, Source};
use hub_omsi::hof::Hof;
use hub_omsi::Vfs;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One depot file of the bus, as its tile shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub path: PathBuf,
    pub file: String,
    /// Its `[name]`, or its file name without `.hof` when it has none.
    pub name: String,
    /// What the game is given (`--hof`) to take this one.
    pub key: String,
    pub destinations: usize,
    /// How many of the chosen map's destinations it knows (None: nothing to compare with).
    pub known: Option<usize>,
    /// The map it belongs to, as far as can be told.
    pub map: Option<String>,
    /// It is the chosen map's depot file.
    pub maps: bool,
    /// Omsi-Hub added it (`depot::added`).
    pub added: bool,
}

/// What a drive takes when the bus is given the map's depot file and lacks it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Fallback {
    Own(PathBuf),
    Borrowed(String),
    #[default]
    Nothing,
}

/// The depot file a drive with the bus has.
#[derive(Clone, Debug, PartialEq)]
pub enum Use {
    Row(usize),
    Borrowed(String),
    Nothing,
}

/// How a depot file stands to the chosen map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    Maps,
    Fits,
    Partly(usize, usize),
    Other,
    Unknown,
}

/// The bus's depot files, worked out for a bus on a map.
#[derive(Clone, Debug, Default)]
pub struct Menu {
    pub key: String,
    pub bus: String,
    pub rows: Vec<Row>,
    pub want: String,
    pub map: String,
    pub total: Option<usize>,
    pub source: Option<Source>,
    pub source_name: String,
    pub source_destinations: usize,
    pub refusal: Option<Refusal>,
    pub target: Option<PathBuf>,
    pub fallback: Fallback,
    pub recommended: Option<usize>,
    pub own: Vec<hub_core::owndepot::Entry>,
}

impl Menu {
    /// The bus lacks the map's depot file.
    pub fn lacks(&self) -> bool {
        !self.want.trim().is_empty() && !self.rows.iter().any(|r| r.maps)
    }

    pub fn used(&self, chosen: &str) -> Use {
        if let Some(i) = row_of(&self.rows, chosen) {
            return Use::Row(i);
        }
        match &self.fallback {
            Fallback::Own(p) => self.rows.iter().position(|r| &r.path == p).map_or(Use::Nothing, Use::Row),
            Fallback::Borrowed(f) => Use::Borrowed(f.clone()),
            Fallback::Nothing => Use::Nothing,
        }
    }
}

/// A depot file's destinations to compare: its termini but the "everybody out" ones, folded.
fn destinations(h: &Hof) -> HashSet<String> {
    h.termini.iter().filter(|t| !t.all_exit).map(|t| fold(&t.texture_id)).filter(|s| !s.is_empty()).collect()
}

/// The row of the depot file `chosen`: by what the game is given, its file name or its name.
pub fn row_of(rows: &[Row], chosen: &str) -> Option<usize> {
    let c = chosen.trim();
    if c.is_empty() {
        return None;
    }
    let stem = |r: &Row| Path::new(&r.file).file_stem().map(|s| s.to_string_lossy().trim().to_string()).unwrap_or_default();
    rows.iter()
        .position(|r| r.key.trim().eq_ignore_ascii_case(c))
        .or_else(|| rows.iter().position(|r| stem(r).eq_ignore_ascii_case(c)))
        .or_else(|| rows.iter().position(|r| r.name.trim().eq_ignore_ascii_case(c)))
}

pub fn fit_of(row: &Row, total: Option<usize>) -> Fit {
    if row.maps {
        return Fit::Maps;
    }
    match (row.known, total) {
        (Some(k), Some(t)) if t > 0 && k * 10 >= t * 9 => Fit::Fits,
        (Some(k), Some(t)) if k > 0 => Fit::Partly(k, t),
        (Some(_), Some(t)) if t > 0 => Fit::Other,
        _ => Fit::Unknown,
    }
}

/// The map a depot file belongs to: a map whose depot file it is, else the map whose name
/// shares the most words of a place with it. `maps`: each map's label and depot file.
pub fn map_of(name: &str, stem: &str, maps: &[(String, String)]) -> Option<String> {
    let says = |h: &str| !h.trim().is_empty() && (h.trim().eq_ignore_ascii_case(name.trim()) || h.trim().eq_ignore_ascii_case(stem.trim()));
    if let Some((label, _)) = maps.iter().find(|(_, h)| says(h)) {
        return Some(label.clone());
    }
    let labels: Vec<&str> = maps.iter().map(|(l, _)| l.as_str()).collect();
    closest_name(&labels, &[name, stem]).map(|i| maps[i].0.clone())
}

/// The row that belongs to the place `hints` name (by its file name and its name).
fn like(rows: &[Row], hints: &[&str]) -> Option<usize> {
    let labels: Vec<String> = rows.iter().map(|r| format!("{} {}", Path::new(&r.file).file_stem().unwrap_or_default().to_string_lossy(), r.name)).collect();
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    closest_name(&refs, hints)
}

pub fn fallback_of(rows: &[Row], want: &str, map_hints: &[&str], borrowed: Option<String>) -> Fallback {
    if let Some(i) = like(rows, &[want]) {
        return Fallback::Own(rows[i].path.clone());
    }
    if let Some(f) = borrowed {
        return Fallback::Borrowed(f);
    }
    if let Some(i) = like(rows, map_hints) {
        return Fallback::Own(rows[i].path.clone());
    }
    rows.first().map(|r| Fallback::Own(r.path.clone())).unwrap_or_default()
}

/// The row to recommend: the map's own, else the one that knows most of its destinations (a
/// quarter at least), else the one of the same place.
pub fn recommend(rows: &[Row], like: Option<usize>, total: Option<usize>) -> Option<usize> {
    if let Some(i) = rows.iter().position(|r| r.maps) {
        return Some(i);
    }
    let enough = |k: usize| k > 0 && total.is_none_or(|t| k * 4 >= t);
    let best = rows.iter().enumerate().filter(|(_, r)| r.known.is_some_and(enough)).max_by(|(a, x), (b, y)| x.known.cmp(&y.known).then(b.cmp(a)));
    best.map(|(i, _)| i).or(like)
}

/// The rows in the order the tiles show them: `first` first, then the map's, then by what they
/// know, then by name.
pub fn order(mut rows: Vec<Row>, first: Option<usize>) -> Vec<Row> {
    let head = first.filter(|i| *i < rows.len()).map(|i| rows.remove(i));
    rows.sort_by(|a, b| b.maps.cmp(&a.maps).then(b.known.unwrap_or(0).cmp(&a.known.unwrap_or(0))).then_with(|| name_cmp(&a.name, &b.name)));
    head.into_iter().chain(rows).collect()
}

/// The bus's own depot files the dialog offers instead of the map's: those that know a quarter
/// of its destinations at least, the best first, three at most.
pub fn choices(m: &Menu) -> Vec<usize> {
    let enough = |k: usize| k > 0 && m.total.is_none_or(|t| k * 4 >= t);
    let mut v: Vec<usize> = (0..m.rows.len()).filter(|i| m.rows[*i].known.is_some_and(enough)).collect();
    v.sort_by(|a, b| m.rows[*b].known.cmp(&m.rows[*a].known).then(a.cmp(b)));
    if v.is_empty() && m.total.is_none() {
        v.extend(m.recommended);
    }
    v.truncate(3);
    v
}

/// A place to show in a line: from the vehicle folder's parent on.
pub fn short_place(p: &Path) -> String {
    let parts: Vec<String> = p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    match parts.iter().rposition(|c| c.eq_ignore_ascii_case("Vehicles")) {
        Some(i) if i > 0 => parts[i - 1..].join(std::path::MAIN_SEPARATOR_STR),
        _ => p.display().to_string(),
    }
}

// --- working it out ------------------------------------------------------------------------------

/// What a menu is worked out of.
pub struct Job {
    pub key: String,
    pub bus: String,
    pub map: String,
    pub want: String,
    pub map_label: String,
    pub map_hints: Vec<String>,
    pub maps: Vec<(String, String)>,
}

fn same(a: &Path, b: &Path) -> bool {
    slashed(&a.to_string_lossy()).eq_ignore_ascii_case(&slashed(&b.to_string_lossy()))
}

/// The menu for `j`, its depot files read through `vfs`, the copies of the map's found by
/// `find` (`hub_core::depot::find_sources`).
pub fn analyse(vfs: &Vfs, j: Job, find: &dyn Fn(&str, &str) -> Vec<Source>) -> Menu {
    let files = bus_depot_files(vfs, &j.bus, false);
    let added = hub_core::depot::added();
    let hofs: Vec<Hof> = files.iter().map(|f| Hof::load(f).unwrap_or_else(|_| Hof { path: f.clone(), ..Default::default() })).collect();
    let stems: Vec<String> = files.iter().map(|f| f.file_stem().unwrap_or_default().to_string_lossy().trim().to_string()).collect();
    let names: Vec<String> = hofs.iter().zip(&stems).map(|(h, s)| if h.name.trim().is_empty() { s.clone() } else { h.name.trim().to_string() }).collect();
    let keys = keys_for(&names, &stems);
    let dests: Vec<HashSet<String>> = hofs.iter().map(destinations).collect();
    let maps_flags: Vec<bool> = files.iter().map(|f| hub_core::depot::answers_to(f, &j.want)).collect();
    let own = maps_flags.iter().position(|m| *m);
    let sources = if j.want.trim().is_empty() { Vec::new() } else { find(&j.want, &j.map) };
    let others: Vec<Source> = sources.iter().filter(|s| !files.iter().any(|f| same(f, &s.path))).cloned().collect();
    let source = hub_core::depot::best(&others).map(|i| others[i].clone());
    let source_hof = source.as_ref().and_then(|s| Hof::load(&s.path).ok());
    let reference: Option<HashSet<String>> = own.map(|i| dests[i].clone()).or_else(|| source_hof.as_ref().map(destinations));
    let rows: Vec<Row> = (0..files.len())
        .map(|i| Row {
            path: files[i].clone(),
            file: files[i].file_name().unwrap_or_default().to_string_lossy().into_owned(),
            name: names[i].clone(),
            key: keys[i].clone(),
            destinations: dests[i].len(),
            known: reference.as_ref().map(|r| dests[i].intersection(r).count()),
            map: if maps_flags[i] { Some(j.map_label.clone()).filter(|m| !m.is_empty()) } else { map_of(&names[i], &stems[i], &j.maps) },
            maps: maps_flags[i],
            added: added.iter().any(|a| same(a, &files[i])),
        })
        .collect();
    let lacks = !j.want.trim().is_empty() && own.is_none();
    let hints: Vec<&str> = j.map_hints.iter().map(String::as_str).filter(|h| !h.trim().is_empty()).collect();
    let borrowed = if lacks {
        let mut v: Vec<&Source> = others.iter().filter(|s| !s.from_map).collect();
        v.sort_by_key(|s| s.folder.to_lowercase());
        v.first().map(|s| s.folder.clone())
    } else {
        None
    };
    let fallback = if lacks { fallback_of(&rows, &j.want, &hints, borrowed) } else { Fallback::Nothing };
    let same_place = if j.want.trim().is_empty() { like(&rows, &hints) } else { like(&rows, &[j.want.as_str()]).or_else(|| like(&rows, &hints)) };
    let first = recommend(&rows, same_place, reference.as_ref().map(|r| r.len()));
    let rows = order(rows, first);
    let target = hub_core::depot::target_for(&j.bus);
    let existing: Vec<(String, String)> = rows.iter().map(|r| (r.file.clone(), r.name.clone())).collect();
    let refusal = if j.want.trim().is_empty() {
        None
    } else if let Some(r) = rows.iter().find(|r| r.maps) {
        Some(Refusal::Has(r.file.clone()))
    } else {
        match &source {
            None => Some(Refusal::NoSource),
            Some(_) if target.is_none() => Some(Refusal::NoFolder),
            Some(s) => hub_core::depot::check(&s.file, &j.want, &existing).err(),
        }
    };
    let source_name = source.as_ref().and_then(|s| Hof::read_name(&s.path)).unwrap_or_default().trim().to_string();
    Menu {
        key: j.key,
        bus: j.bus,
        total: reference.as_ref().map(|r| r.len()),
        source_destinations: source_hof.as_ref().map_or(0, |h| destinations(h).len()),
        source,
        source_name,
        refusal,
        target,
        fallback,
        recommended: first.map(|_| 0),
        own: hub_core::owndepot::for_map(&hub_core::owndepot::list(&hub_core::owndepot::dir()), &hub_core::lines::map_folder(&j.map)),
        want: j.want,
        map: j.map_label,
        rows,
    }
}

/// The menus worked out last (a few), by bus, map, depot file and what was added.
fn menus() -> &'static Mutex<Vec<Arc<Menu>>> {
    static M: Mutex<Vec<Arc<Menu>>> = Mutex::new(Vec::new());
    &M
}

/// The player's own depot file (its place among `m.own`) the drive gives the bus: the one
/// chosen, else the one the driven line of his asks for (and whether the line chose it).
fn own_in_use(m: &Menu, chosen: &str, line: Option<&str>, map: &str) -> Option<(usize, bool)> {
    if let Some(i) = m.own.iter().position(|e| e.key.eq_ignore_ascii_case(chosen.trim())) {
        return Some((i, false));
    }
    let line = line?;
    let own = hub_core::lines::own_lines_of_map(map);
    let o = own.iter().find(|o| o.file.eq_ignore_ascii_case(line.trim()))?;
    m.own.iter().position(|e| !o.depot_file.trim().is_empty() && e.key.eq_ignore_ascii_case(o.depot_file.trim())).map(|i| (i, true))
}

pub fn refusal_json(r: &Refusal) -> Value {
    let (kind, what) = match r {
        Refusal::Has(f) => ("has", f.clone()),
        Refusal::Taken(f) => ("taken", f.clone()),
        Refusal::NoSource => ("no_source", String::new()),
        Refusal::NoFolder => ("no_folder", String::new()),
        Refusal::Original => ("original", String::new()),
        Refusal::Unreadable(w) => ("unreadable", w.clone()),
        Refusal::Failed(w) => ("failed", w.clone()),
    };
    json!({ "kind": kind, "what": what })
}

fn fit_json(f: Fit) -> Value {
    match f {
        Fit::Maps => json!({ "kind": "maps" }),
        Fit::Fits => json!({ "kind": "fits" }),
        Fit::Partly(k, t) => json!({ "kind": "partly", "known": k, "total": t }),
        Fit::Other => json!({ "kind": "other" }),
        Fit::Unknown => json!({ "kind": "unknown" }),
    }
}

/// The menu as the page reads it, with the depot file `chosen` and the driven `line`.
pub fn menu_json(m: &Menu, chosen: &str, line: Option<&str>, map_file: &str) -> Value {
    let used = match m.used(chosen) {
        Use::Row(i) => json!({ "row": i }),
        Use::Borrowed(f) => json!({ "borrowed": f }),
        Use::Nothing => json!({}),
    };
    let own = own_in_use(m, chosen, line, map_file);
    let rows: Vec<Value> = m
        .rows
        .iter()
        .map(|r| json!({ "path": slashed(&r.path.to_string_lossy()), "file": r.file, "name": r.name, "key": r.key, "destinations": r.destinations, "known": r.known, "map": r.map, "maps": r.maps, "added": r.added, "fit": fit_json(fit_of(r, m.total)) }))
        .collect();
    let own_files: Vec<Value> = m.own.iter().map(|e| json!({ "key": e.key, "name": e.name, "termini": e.termini.len(), "specials": e.specials })).collect();
    let source = m.source.as_ref().map(|s| json!({ "path": s.path.to_string_lossy(), "file": s.file, "folder": s.folder, "from_map": s.from_map }));
    json!({
        "key": m.key, "bus": m.bus, "want": m.want, "map": m.map, "total": m.total, "rows": rows,
        "source": source, "source_name": m.source_name, "source_destinations": m.source_destinations,
        "refusal": m.refusal.as_ref().map(refusal_json), "place": m.target.as_deref().map(short_place),
        "recommended": m.recommended, "own": own_files, "lacks": m.lacks(),
        "used": used, "own_in_use": own.map(|(i, by_line)| json!({ "index": i, "by_line": by_line })),
        "choices": choices(m),
    })
}

pub fn depots(args: &Value) -> Answer {
    let vfs = super::need_vfs()?;
    let bus = arg::<String>(args, "bus")?;
    let map: Value = arg::<Option<Value>>(args, "map")?.unwrap_or(Value::Null);
    let s = |k: &str| map.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let file = s("file");
    let date = super::date_arg(args)?;
    let chosen = arg::<Option<String>>(args, "chosen")?.unwrap_or_default();
    let line = arg::<Option<String>>(args, "line")?;
    let maps: Vec<(String, String)> = arg::<Option<Vec<(String, String)>>>(args, "maps")?.unwrap_or_default();
    let want = Some(super::map_hof(&vfs, &file, &date)).filter(|w| !w.trim().is_empty()).unwrap_or_else(|| s("hof"));
    let key = format!("{bus}|{file}|{want}|{}", hub_core::depot::added().len());
    let have = menus().lock().unwrap_or_else(|e| e.into_inner()).iter().find(|m| m.key == key).cloned();
    let m = match have {
        Some(m) => m,
        None => {
            let label = if s("friendly").trim().is_empty() { s("name").trim().to_string() } else { s("friendly").trim().to_string() };
            let folder = super::map_dir_rel(&file).rsplit('/').next().unwrap_or("").to_string();
            let job = Job { key: key.clone(), bus: bus.clone(), map: file.clone(), want, map_label: label, map_hints: vec![s("name"), s("friendly"), folder], maps };
            // (an odd file of some mod that the reading cannot cope with: the tiles say there
            // is nothing)
            let m = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| analyse(&vfs, job, &hub_core::depot::find_sources))).unwrap_or_else(|_| Menu {
                key: key.clone(),
                bus: bus.clone(),
                ..Default::default()
            });
            let m = Arc::new(m);
            let mut all = menus().lock().unwrap_or_else(|e| e.into_inner());
            all.retain(|x| x.key != key);
            all.push(m.clone());
            let n = all.len();
            if n > 8 {
                all.drain(..n - 8);
            }
            m
        }
    };
    Ok(menu_json(&m, &chosen, line.as_deref(), &file))
}

/// Give the bus the map's depot file `want` from the copy `source`: `{ added: path }`, or
/// `{ refusal }` - why not.
pub fn add(bus: &str, want: &str, source: &str) -> Answer {
    match hub_core::depot::add_for_bus(bus, want, Path::new(source)) {
        Ok(p) => {
            menus().lock().unwrap_or_else(|e| e.into_inner()).clear();
            Ok(json!({ "added": p.to_string_lossy() }))
        }
        Err(e) => Ok(json!({ "refusal": refusal_json(&e) })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(file: &str, name: &str, known: Option<usize>, maps: bool) -> Row {
        Row { path: PathBuf::from(format!("Vehicles/Bus/{file}")), file: file.into(), name: name.into(), key: name.into(), destinations: 10, known, map: None, maps, added: false }
    }

    fn menu(rows: Vec<Row>, want: &str, total: Option<usize>) -> Menu {
        Menu { key: "k".into(), bus: "Vehicles/Bus/bus.bus".into(), rows, want: want.into(), map: "Hamburg".into(), total, ..Default::default() }
    }

    #[test]
    fn the_depot_files_are_weighed_for_the_map() {
        assert_eq!(fit_of(&row("a.hof", "A", Some(37), false), Some(40)), Fit::Fits);
        assert_eq!(fit_of(&row("a.hof", "A", Some(12), false), Some(40)), Fit::Partly(12, 40));
        assert_eq!(fit_of(&row("a.hof", "A", Some(0), false), Some(40)), Fit::Other);
        let rows = vec![row("A.hof", "Alpha", Some(2), false), row("B.hof", "Bravo", Some(9), false), row("C.hof", "Charlie", Some(40), true), row("D.hof", "Delta", Some(0), false)];
        assert_eq!(recommend(&rows, None, Some(40)), Some(2));
        assert_eq!(order(rows, Some(2)).iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Charlie", "Bravo", "Alpha", "Delta"]);
        let m = menu(vec![row("A.hof", "A", Some(12), false), row("B.hof", "B", Some(0), false), row("C.hof", "C", Some(30), false), row("D.hof", "D", Some(15), false)], "Hamburg", Some(40));
        assert!(m.lacks());
        assert_eq!(choices(&m), vec![2, 3, 0]);
        let rows = vec![row("Grundorf.hof", "Grundorf", None, false), row("HH_2014.hof", "Hamburg Linie 20 (2014)", None, false)];
        assert_eq!(fallback_of(&rows, "Spandau 1986", &["Berlin-Spandau"], Some("SD200".into())), Fallback::Borrowed("SD200".into()));
        assert_eq!(fallback_of(&rows, "Spandau 1986", &["Hamburg"], None), Fallback::Own(rows[1].path.clone()));
        assert_eq!(row_of(&rows, "hh_2014"), Some(1));
        assert_eq!(map_of("Grundorf", "Grundorf", &[("Grundorf (Map)".into(), "Busses".into())]), Some("Grundorf (Map)".into()));
    }

    #[test]
    fn the_test_bus_has_the_test_map_s_depot_file() {
        let v = crate::content::tests::fixture_vfs();
        let want = crate::content::map_hof(&v, "maps/Testdorf/global.cfg", "1989-05-30");
        let job = Job {
            key: "k".into(),
            bus: "Vehicles/Testbus/Testbus.bus".into(),
            map: "maps/Testdorf/global.cfg".into(),
            want: want.clone(),
            map_label: "Testdorf".into(),
            map_hints: vec!["Testdorf".into()],
            maps: vec![],
        };
        let m = analyse(&v, job, &|_, _| Vec::new());
        assert_eq!(m.rows.len(), 1);
        assert!(m.rows[0].maps, "{want}: {:?}", m.rows);
        assert!(!m.lacks());
        let j = menu_json(&m, &want, None, "maps/Testdorf/global.cfg");
        assert_eq!(j["used"], json!({ "row": 0 }));
        assert_eq!(j["rows"][0]["fit"]["kind"], "maps");
        assert_eq!(j["refusal"]["kind"], "has");
    }
}
