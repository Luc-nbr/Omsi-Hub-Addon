//! The content beside a cloud save, for the website's bus company (docs/WEB.md): what the
//! company's pages read of this computer, packed in three parts (`hub_core::webpack`) and sent
//! into the save's slot after the save itself (`cloud.rs`):
//!
//! * `map` - the company's map as its timetable is read: `global.cfg`, `timezone.txt`,
//!   `Holidays.txt`, `ailists.cfg`, its picture and its depot files, all of `TTData/`, and each
//!   `Chrono/<x>/` with its `Chrono.cfg`, `ailists*.cfg` and `TTData/` (not the tiles: the map is
//!   drawn from `web/map.json`, below);
//! * `buses` - every bus of the computer as the dealer and the fleet read it: the `.bus` files
//!   of `Vehicles/<pack>/` (and those the fleet names deeper down), and of what each names only
//!   what is read - its model's `[CTC]` (a file of that block alone), the `.cti` liveries, the
//!   number and registration lists, the passenger cabin; and the depot files of the packs;
//! * `data` - the data folder's own files the pages read (the lines, the own depots, the driver's
//!   trips and career, the bus fonts and options, the vehicle classes), the photos of the fleet's
//!   buses (`webpack::photo_name`), the company's logo (`web/logo`) and the map drawn
//!   (`web/map.json`: `content::mapview::fleet_export`, the routes of the company's trips).
//!
//! Each part goes up zstd'd, in pieces of at most `PIECE` (`content-<kind>-<sha>.<n>`), and
//! `content.json` names them: `{ version: 1, at, parts: [{ kind, sha256, size, pieces }] }`. A part
//! that did not change is not sent again; the pieces no longer named go. It is built at most every
//! `EVERY` per company, on the cloud's worker thread.

use crate::launcher as core;
use crate::launcher::company::Company;
use hub_core::webpack;
use hub_omsi::vfs::Vfs;
use hub_online::saves::{self, sha256_hex};
use hub_online::Error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Mutex;

/// The largest piece (the bucket takes 25 MiB a file).
const PIECE: usize = 24 << 20;
/// How often a company's content is built again.
const EVERY: u64 = 30 * 60;
/// A file larger than this is left out (a stray model or picture).
const FILE_MAX: usize = 8 << 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PartMeta {
    pub kind: String,
    pub sha256: String,
    pub size: u64,
    pub pieces: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContentMeta {
    pub version: u32,
    pub at: u64,
    pub parts: Vec<PartMeta>,
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// --- what goes in --------------------------------------------------------------------------------

/// The files of one part, by name, each once.
#[derive(Default)]
struct Files(HashMap<String, Vec<u8>>);

impl Files {
    fn put(&mut self, name: String, bytes: Vec<u8>) {
        if bytes.len() <= FILE_MAX && webpack::safe_name(&name) {
            self.0.entry(name).or_insert(bytes);
        }
    }

    /// `rel` (under the OMSI 2 folder, as `vfs` finds it) as `omsi/<rel>`.
    fn omsi(&mut self, vfs: &Vfs, rel: &str) {
        let rel = hub_omsi::vfs::normalize(rel);
        if rel.is_empty() || self.0.contains_key(&format!("omsi/{rel}")) {
            return;
        }
        if let Ok(b) = vfs.read(&rel) {
            self.put(format!("omsi/{rel}"), b);
        }
    }

    /// Every file under `rel`, its folders too (`depth` deep at most).
    fn tree(&mut self, vfs: &Vfs, rel: &str, depth: usize) {
        for (name, dir) in vfs.list_dir(rel) {
            let path = format!("{rel}/{name}");
            if dir {
                if depth > 0 {
                    self.tree(vfs, &path, depth - 1);
                }
            } else {
                self.omsi(vfs, &path);
            }
        }
    }

    /// The files of `rel` whose names `keep` takes (no folders).
    fn some(&mut self, vfs: &Vfs, rel: &str, keep: impl Fn(&str) -> bool) {
        for (name, dir) in vfs.list_dir(rel) {
            if !dir && keep(&name.to_lowercase()) {
                self.omsi(vfs, &format!("{rel}/{name}"));
            }
        }
    }

    fn into_vec(self) -> Vec<(String, Vec<u8>)> {
        self.0.into_iter().collect()
    }
}

fn ext(name: &str) -> &str {
    name.rsplit_once('.').map(|x| x.1).unwrap_or("")
}

/// The company's map, as its timetable is read.
fn map_part(vfs: &Vfs, c: &Company) -> Files {
    let mut f = Files::default();
    let dir = hub_omsi::vfs::normalize(&c.map).rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
    if dir.is_empty() {
        return f;
    }
    f.some(vfs, &dir, |n| matches!(n, "global.cfg" | "timezone.txt" | "holidays.txt" | "ailists.cfg") || n.starts_with("picture.") || ext(n) == "hof");
    for (name, is_dir) in vfs.list_dir(&dir) {
        if !is_dir {
            continue;
        }
        let sub = format!("{dir}/{name}");
        match name.to_lowercase().as_str() {
            "ttdata" => f.tree(vfs, &sub, 2),
            "chrono" => {
                for (x, d) in vfs.list_dir(&sub) {
                    if !d {
                        continue;
                    }
                    let at = format!("{sub}/{x}");
                    f.some(vfs, &at, |n| n == "chrono.cfg" || (n.starts_with("ailists") && ext(n) == "cfg"));
                    for (y, dd) in vfs.list_dir(&at) {
                        if dd && y.eq_ignore_ascii_case("TTData") {
                            f.tree(vfs, &format!("{at}/{y}"), 2);
                        }
                    }
                }
            }
            // (depot files in the map's own folders)
            _ => f.some(vfs, &sub, |n| ext(n) == "hof"),
        }
    }
    f
}

/// A model's `[CTC]` alone, as a file of its own (UTF-8 with its BOM).
fn ctc_only(model: &Path) -> Option<Vec<u8>> {
    let cfg = hub_omsi::text::CfgFile::read(model).ok()?;
    let mut r = cfg.reader();
    while let Some(k) = r.next_keyword() {
        if k.eq_ignore_ascii_case("CTC") {
            let v = r.values(2);
            let mut out = "\u{feff}[CTC]\r\n".to_string();
            for x in v {
                out.push_str(x.trim_end_matches(['\r', '\n']));
                out.push_str("\r\n");
            }
            return Some(out.into_bytes());
        }
    }
    None
}

/// One bus and what of its own files is read.
fn bus(f: &mut Files, vfs: &Vfs, rel: &str, seen: &mut BTreeSet<String>) {
    let rel = hub_omsi::vfs::normalize(rel);
    if !seen.insert(rel.to_lowercase()) {
        return;
    }
    let Some(path) = vfs.resolve(&rel) else { return };
    f.omsi(vfs, &rel);
    let Ok(b) = hub_omsi::vehicle::Bus::load(&path) else { return };
    let dir = b.dir().to_path_buf();
    let name_of = |p: &Path| vfs.relative(p);
    if let Some(m) = b.model.as_deref().and_then(|m| hub_omsi::vfs::resolve_in(&dir, m)) {
        if let (Some(r), Some(bytes)) = (name_of(&m), ctc_only(&m)) {
            f.put(format!("omsi/{}", hub_omsi::vfs::normalize(&r)), bytes);
        }
    }
    if let Some(ctc) = b.ctc_dir.as_deref().and_then(|c| hub_omsi::vfs::resolve_in(&dir, c)).and_then(|p| name_of(&p)) {
        f.some(vfs, &ctc, |n| ext(n) == "cti");
    }
    for list in [&b.number_list, &b.registration_list, &b.passenger_cabin] {
        if let Some(r) = list.as_deref().and_then(|l| hub_omsi::vfs::resolve_in(&dir, l)).and_then(|p| name_of(&p)) {
            f.omsi(vfs, &r);
        }
    }
    if let Some(r) = b.couple_back_path().and_then(|p| name_of(&p)) {
        bus(f, vfs, &r, seen);
    }
}

/// Every bus of the computer, and the depot files of the packs.
fn buses_part(vfs: &Vfs, c: &Company) -> Files {
    let mut f = Files::default();
    let mut seen = BTreeSet::new();
    for (pack, dir) in vfs.list_dir("Vehicles") {
        if !dir {
            continue;
        }
        let at = format!("Vehicles/{pack}");
        for (name, d) in vfs.list_dir(&at) {
            let path = format!("{at}/{name}");
            let lower = name.to_lowercase();
            if !d && ext(&lower) == "bus" {
                bus(&mut f, vfs, &path, &mut seen);
            } else if !d && ext(&lower) == "hof" {
                f.omsi(vfs, &path);
            } else if d {
                f.some(vfs, &path, |n| ext(n) == "hof");
            }
        }
    }
    // (the fleet's buses deeper down)
    for v in &c.fleet {
        bus(&mut f, vfs, &v.bus, &mut seen);
    }
    f
}

/// The data folder's own files the pages read, the fleet's photos, the logo and the map drawn.
fn data_part(data: &Path, c: &Company) -> Files {
    let mut f = Files::default();
    let mut file = |name: &str, path: &Path| {
        if let Ok(b) = std::fs::read(path) {
            f.put(format!("data/{name}"), b);
        }
    };
    for n in ["bus-fonts.json", "bus-options.json", "vehicle-classes.json", "company-tour.json", "rosterflow-tour.json"] {
        file(n, &data.join(n));
    }
    for d in ["lines", "depots"] {
        if let Ok(rd) = std::fs::read_dir(data.join(d)) {
            for e in rd.flatten().filter(|e| e.path().is_file()) {
                file(&format!("{d}/{}", e.file_name().to_string_lossy()), &e.path());
            }
        }
    }
    for p in [hub_core::trips::trips_file(data, &c.profile), core::company::career::path_of(data, &c.profile)] {
        if let Ok(rel) = p.strip_prefix(data) {
            file(&rel.to_string_lossy().replace('\\', "/"), &p);
        }
    }
    // (the photos of the fleet, by the name the website asks for them by)
    for v in &c.fleet {
        for paint in [v.livery.as_str(), ""] {
            if let Ok(b) = crate::content::bus3d::photo::get(&json!({ "bus": v.bus, "paint": paint })) {
                f.put(format!("data/{}", webpack::photo_name(&v.bus, paint)), b);
            }
        }
    }
    if let Some(logo) = c.logo.as_deref().filter(|l| !l.trim().is_empty()) {
        if let Ok(b) = std::fs::read(logo) {
            f.put("data/web/logo".into(), b);
        }
    }
    if let Some(m) = map_drawn(c) {
        f.put("data/web/map.json".into(), m);
    }
    f
}

/// The company's map as the fleet map draws it, with the route of every trip of its lines:
/// `{origin, lo, hi, roads, colors, tracks: {trip: {p, stops}}}` (`content/mapview_web.rs`).
fn map_drawn(c: &Company) -> Option<Vec<u8>> {
    let global = hub_core::paths::resolve_content(&c.map).filter(|p| p.is_file())?;
    let lines = core::list_lines(&c.map, &c.date).ok()?;
    let mut trips: Vec<String> = Vec::new();
    for cl in &c.lines {
        if let Some(line) = lines.iter().find(|x| x.name.eq_ignore_ascii_case(&cl.name)) {
            for t in line.tours.iter().flat_map(|t| t.trips.iter()) {
                if !trips.contains(&t.name) {
                    trips.push(t.name.clone());
                }
            }
        }
    }
    let mut v = crate::content::mapview::fleet_export(&c.map, &global, &c.date, &trips, 0.6);
    let tracks: serde_json::Map<String, Value> = v
        .get("tracks")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|t| Some((t.get("name")?.as_str()?.to_string(), json!({ "p": t.get("p")?, "stops": t.get("stops")? })))).collect())
        .unwrap_or_default();
    v["tracks"] = Value::Object(tracks);
    serde_json::to_vec(&v).ok()
}

/// The three parts of company `c`, packed (each its files' archive).
pub fn build(data: &Path, c: &Company) -> Vec<(&'static str, Vec<u8>)> {
    let vfs = hub_core::paths::vfs();
    let mut out = Vec::new();
    if let Some(vfs) = &vfs {
        out.push(("map", webpack::pack(&mut map_part(vfs, c).into_vec())));
        out.push(("buses", webpack::pack(&mut buses_part(vfs, c).into_vec())));
    }
    out.push(("data", webpack::pack(&mut data_part(data, c).into_vec())));
    out
}

// --- up into the slot ----------------------------------------------------------------------------

static BUILT: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Company `id`'s content into its slot `slot`, unless it went up less than `EVERY` ago; parts
/// unchanged stay as they are. Run on the cloud's worker thread, after the save went up.
pub fn send(data: &Path, id: &str, slot: u8) -> Result<(), Error> {
    {
        let mut b = BUILT.lock().unwrap_or_else(|e| e.into_inner());
        let at = b.get_or_insert_with(HashMap::new);
        if at.get(id).is_some_and(|t| now().saturating_sub(*t) < EVERY) {
            return Ok(());
        }
        at.insert(id.to_string(), now());
    }
    let c = core::company::store::load(data, id).map_err(|_| Error::Code("server"))?;
    let parts = build(data, &c);
    let (at, transfer) = crate::online::with(|cl| cl.content_file(slot, "content.json", true).map(|a| (a, cl.transfer())))?;
    let had: ContentMeta = transfer.get_file(&at)?.and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let mut meta = ContentMeta { version: 1, at: now(), parts: Vec::new() };
    for (kind, archive) in parts {
        let z = saves::zstd(&archive)?;
        let sha = sha256_hex(&z);
        if let Some(p) = had.parts.iter().find(|p| p.kind == kind && p.sha256 == sha) {
            meta.parts.push(p.clone());
            continue;
        }
        let mut pieces = Vec::new();
        for (n, piece) in z.chunks(PIECE).enumerate() {
            let name = format!("content-{kind}-{}.{n}", &sha[..16]);
            let at = crate::online::with(|cl| cl.content_file(slot, &name, false))?;
            transfer.put_file(&at, piece)?;
            pieces.push(name);
        }
        meta.parts.push(PartMeta { kind: kind.into(), sha256: sha, size: z.len() as u64, pieces });
    }
    let body = serde_json::to_vec(&meta).map_err(|_| Error::Code("server"))?;
    let at = crate::online::with(|cl| cl.content_file(slot, "content.json", false))?;
    transfer.put_file(&at, &body)?;
    // (the pieces no longer named)
    let keep: BTreeSet<&String> = meta.parts.iter().flat_map(|p| p.pieces.iter()).collect();
    let gone: Vec<String> = had.parts.iter().flat_map(|p| p.pieces.iter()).filter(|n| !keep.contains(n)).cloned().collect();
    if !gone.is_empty() {
        let _ = crate::online::with(|cl| cl.content_remove(slot, &gone));
    }
    Ok(())
}
