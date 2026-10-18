//! The bus step's bus options: the `[setvar]` variables of the chosen bus's liveries - its
//! mirrors, rims, seats, door types - offered to choose; "As the livery" leaves one to the
//! livery chosen. A value chosen is kept per bus file (`hub_core::busoptions`) and goes to the
//! game after the paint (the duty's `set_vars`).
//!
//! The liveries are the `.cti` files of the bus's paint folder (`[CTC]` in its `model.cfg`):
//! each one's first `[item]` names it, its `[setvar]` lines (a variable, then its value) set
//! what it sets. An option is "appearance" when no script of the bus reads it and the model
//! shows it (a mesh's `[visible]`, a material's `[matl_change]`); every other one is
//! "technical". What most liveries give an option (one that does not set it counting as 0) is
//! its usual value.
//!
//! * `hub_more_bus_options {bus, paint}` - `{ options: [{ var, label, technical, switch, usual,
//!   livery, chosen, values: [{ value, liveries }] }] }`;
//! * `hub_more_bus_option_set {bus, var, value}` or `{bus, reset}` - kept at once;
//! * `hub_more_setvars {bus, paint}` - what the game gets: `[[var, value]]`.

use crate::Answer;
use hub_core::busoptions::BusOptions;
use hub_omsi::CfgFile;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// A livery as the options see it: its name and what its `[setvar]` lines set, in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Livery {
    pub name: String,
    pub vars: Vec<(String, f32)>,
}

impl Livery {
    /// The value it gives `var` (its last line for it: the game sets them in order).
    fn value(&self, var: &str) -> Option<f32> {
        self.vars.iter().rev().find(|(n, _)| n.trim().eq_ignore_ascii_case(var.trim())).map(|(_, v)| *v)
    }
}

/// A value an option takes, and the liveries that give it.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    value: f32,
    liveries: Vec<String>,
}

/// One bus option: a variable the liveries set (`technical`: a script works with it).
#[derive(Clone, Debug, PartialEq)]
struct BusOption {
    var: String,
    label: String,
    technical: bool,
    values: Vec<Seen>,
    usual: f32,
}

impl BusOption {
    /// Seen only as 0 and 1: off or on.
    fn is_switch(&self) -> bool {
        self.values.iter().all(|s| s.value == 0.0 || s.value == 1.0)
    }
}

/// What a bus offers: its liveries and the options they make.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalogue {
    liveries: Vec<Livery>,
    options: Vec<BusOption>,
}

impl Catalogue {
    fn livery(&self, name: &str) -> Option<&Livery> {
        let n = name.trim();
        if n.is_empty() {
            return None;
        }
        self.liveries.iter().find(|l| l.name.trim().eq_ignore_ascii_case(n))
    }
}

/// A variable's name as a person reads it: the setvar prefixes off, the words apart, in
/// sentence case - an abbreviation, a lone letter or a short code as it is.
fn label_of(var: &str) -> String {
    let mut rest = var.trim();
    loop {
        let lower = rest.to_ascii_lowercase();
        match ["vis_", "cti_", "sv_", "setvar_"].iter().find(|p| lower.starts_with(**p) && rest.len() > p.len()) {
            Some(p) => rest = &rest[p.len()..],
            None => break,
        }
    }
    let words = split_words(rest);
    if words.is_empty() {
        return var.trim().to_string();
    }
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        let letters = w.chars().filter(|c| c.is_alphabetic()).count();
        let digits = w.chars().filter(|c| c.is_ascii_digit()).count();
        let caps = letters > 0 && w.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());
        let word = if (caps && letters > 1) || (letters == 1 && w.chars().count() == 1) || (digits > 0 && letters > 0 && w.chars().count() <= 3) {
            w.to_uppercase()
        } else if i == 0 {
            let lower = w.to_lowercase();
            let mut c = lower.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        } else {
            w.to_lowercase()
        };
        out.push(word);
    }
    out.join(" ")
}

/// The words of a name: apart at `_ - . ` and where a small letter meets a capital, letters
/// meet digits, or a run of capitals meets a word.
fn split_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for piece in s.split(['_', '-', ' ', '.']).filter(|p| !p.is_empty()) {
        let cs: Vec<char> = piece.chars().collect();
        let mut cur = String::new();
        for (i, &c) in cs.iter().enumerate() {
            if i > 0 {
                let p = cs[i - 1];
                let next_small = cs.get(i + 1).is_some_and(|n| n.is_lowercase());
                let apart = (p.is_lowercase() && c.is_uppercase())
                    || (p.is_alphabetic() && c.is_ascii_digit() && cur.chars().count() > 1)
                    || (p.is_ascii_digit() && c.is_alphabetic())
                    || (p.is_uppercase() && c.is_uppercase() && next_small);
                if apart && !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            cur.push(c);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

/// Whether a script's text (lower case) works with the local variable `var` (lower case).
fn mentioned(text: &str, var: &str) -> bool {
    let needle = format!(".l.{var}");
    let mut from = 0;
    while let Some(i) = text[from..].find(&needle) {
        let end = from + i + needle.len();
        if !text[end..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return true;
        }
        from = end;
    }
    false
}

/// Whether an option is technical: a script (not a "visual" one) works with it, or nothing
/// of the model shows it.
fn technical(var: &str, scripts: &[(String, String)], shown: &HashSet<String>) -> bool {
    let v = var.trim().to_ascii_lowercase();
    let scripted = scripts.iter().any(|(name, text)| !name.contains("visual") && mentioned(text, &v));
    scripted || !shown.contains(&v)
}

/// What most liveries give `var`, one that does not set it counting as 0.
fn usual_of(liveries: &[Livery], var: &str) -> f32 {
    let given: Vec<f32> = liveries.iter().map(|l| l.value(var).unwrap_or(0.0) + 0.0).collect();
    let mut count: HashMap<u32, usize> = HashMap::new();
    for v in &given {
        *count.entry(v.to_bits()).or_default() += 1;
    }
    let most = count.values().copied().max().unwrap_or(0);
    given.iter().copied().find(|v| count.get(&v.to_bits()) == Some(&most)).unwrap_or(0.0)
}

/// The options `liveries` make: every variable one of them sets (`Colorscheme` aside), with
/// its values; the appearance ones first, each kind by its label.
fn gather(liveries: &[Livery], scripts: &[(String, String)], shown: &HashSet<String>) -> Vec<BusOption> {
    let mut order: Vec<String> = Vec::new();
    let mut seen: HashMap<String, Vec<Seen>> = HashMap::new();
    for l in liveries {
        let mut own: Vec<(String, f32)> = Vec::new();
        for (n, v) in &l.vars {
            let n = n.trim();
            if n.is_empty() || n.eq_ignore_ascii_case("Colorscheme") || !v.is_finite() {
                continue;
            }
            match own.iter_mut().find(|(o, _)| o.eq_ignore_ascii_case(n)) {
                Some(o) => o.1 = *v,
                None => own.push((n.to_string(), *v)),
            }
        }
        for (n, v) in own {
            let list = seen.entry(n.to_ascii_lowercase()).or_insert_with(|| {
                order.push(n.clone());
                Vec::new()
            });
            match list.iter_mut().find(|s| s.value == v) {
                Some(s) => {
                    if !s.liveries.iter().any(|x| x.eq_ignore_ascii_case(&l.name)) {
                        s.liveries.push(l.name.clone());
                    }
                }
                None => list.push(Seen { value: v, liveries: vec![l.name.clone()] }),
            }
        }
    }
    let mut out: Vec<BusOption> = order
        .into_iter()
        .map(|var| {
            let mut values = seen.remove(&var.to_ascii_lowercase()).unwrap_or_default();
            values.sort_by(|a, b| a.value.total_cmp(&b.value));
            BusOption { label: label_of(&var), technical: technical(&var, scripts, shown), usual: usual_of(liveries, &var), values, var }
        })
        .collect();
    out.sort_by(|a, b| a.technical.cmp(&b.technical).then_with(|| super::name_cmp(&a.label, &b.label)).then_with(|| a.var.to_ascii_lowercase().cmp(&b.var.to_ascii_lowercase())));
    out
}

fn same(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn pick(picks: &BTreeMap<String, f32>, var: &str) -> Option<f32> {
    picks.iter().find(|(k, _)| k.eq_ignore_ascii_case(var)).map(|(_, v)| *v)
}

/// What the game gets: every value chosen that the livery does not set to the same itself;
/// without the bus read, every value chosen.
fn setvars(cat: Option<&Catalogue>, livery: &str, picks: &BTreeMap<String, f32>) -> Vec<(String, f32)> {
    let Some(c) = cat else {
        return picks.iter().map(|(k, v)| (k.clone(), *v)).collect();
    };
    let lv = c.livery(livery);
    c.options
        .iter()
        .filter_map(|o| {
            let v = pick(picks, &o.var)?;
            (!lv.and_then(|l| l.value(&o.var)).is_some_and(|l| same(l, v))).then(|| (o.var.clone(), v))
        })
        .collect()
}

/// A `.cti` file as a livery: its first `[item]`'s name (else the file's name) and its
/// `[setvar]` lines.
pub fn livery_of(cfg: &CfgFile) -> Livery {
    let mut name = String::new();
    let mut vars = Vec::new();
    let mut r = cfg.reader();
    while let Some(k) = r.next_keyword() {
        if k.eq_ignore_ascii_case("item") && name.is_empty() {
            name = r.value().trim().to_string();
        } else if k.eq_ignore_ascii_case("setvar") {
            let v = r.values(2);
            if let [var, value] = v[..] {
                let x = hub_omsi::text::parse_f32(value);
                if !var.trim().is_empty() && x.is_finite() {
                    vars.push((var.trim().to_string(), x));
                }
            }
        }
    }
    if name.is_empty() {
        name = cfg.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    }
    Livery { name, vars }
}

/// The variables a model shows: its meshes' `[visible]` and its materials' `[matl_change]`
/// (lower case).
pub fn shown_in(model: &CfgFile) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut r = model.reader();
    while let Some(k) = r.next_keyword() {
        if k.eq_ignore_ascii_case("visible") {
            out.insert(r.value().trim().to_ascii_lowercase());
        } else if k.eq_ignore_ascii_case("matl_change") {
            if let Some(var) = r.values(3).get(2) {
                out.insert(var.trim().to_ascii_lowercase());
            }
        }
    }
    out.remove("");
    out
}

/// The liveries of the bus file `path` as the game makes them (the old app's `busoptions::read`:
/// `omsi_sim::vehicle::model_paint_schemes` of its model's `[CTC]` folders): each once.
fn game_liveries(path: &Path) -> Vec<Livery> {
    let Ok(def) = omsi_vehicle::Vehicle::load(path) else { return Vec::new() };
    let Some(model) = def.model.as_deref().map(|m| omsi_cfg::resolve_path(def.dir(), m)).and_then(|m| omsi_model::Model::load(&m).ok()) else { return Vec::new() };
    let mut out: Vec<Livery> = Vec::new();
    for s in omsi_sim::vehicle::model_paint_schemes(def.dir(), &model) {
        let name = s.name.trim().to_string();
        if !out.iter().any(|o| o.name.eq_ignore_ascii_case(&name)) {
            out.push(Livery { name, vars: s.set_vars });
        }
    }
    out
}

/// Read what the bus file `path` offers: its liveries (the paint folder's `.cti` files, each
/// livery once, in the files' order), its scripts and what its model shows.
pub fn read(path: &Path) -> Result<Catalogue, String> {
    let bus = hub_omsi::vehicle::Bus::load(path).map_err(|e| e.to_string())?;
    let dir = bus.dir().to_path_buf();
    let model = bus.model.as_deref().and_then(|m| hub_omsi::vfs::resolve_in(&dir, m)).and_then(|m| CfgFile::read(m).ok());
    // (the liveries as the game makes them, `omsi_sim`'s paint schemes; else the paint folder's
    // `.cti` files read here)
    let mut liveries: Vec<Livery> = game_liveries(path);
    if let Some(ctc) = bus.ctc_dir.as_deref().and_then(|c| hub_omsi::vfs::resolve_in(&dir, c)).filter(|_| liveries.is_empty()) {
        let mut files: Vec<PathBuf> = hub_core::paths::read_dir_paths(&ctc).into_iter().filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cti"))).collect();
        files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
        for f in files {
            let Ok(cfg) = CfgFile::read(&f) else { continue };
            let l = livery_of(&cfg);
            if !liveries.iter().any(|o| o.name.trim().eq_ignore_ascii_case(l.name.trim())) {
                liveries.push(l);
            }
        }
    }
    let scripts: Vec<(String, String)> = bus
        .scripts
        .scripts
        .iter()
        .filter_map(|p| {
            let full = hub_omsi::vfs::resolve_in(&dir, &p.to_string_lossy())?;
            let text = CfgFile::read(&full).ok()?.lines.join("\n").to_lowercase();
            Some((p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default(), text))
        })
        .collect();
    let shown = model.as_ref().map(shown_in).unwrap_or_default();
    let options = gather(&liveries, &scripts, &shown);
    Ok(Catalogue { liveries, options })
}

/// What `bus` (as the lists name it) offers, read once while its file stays as it is (None:
/// it could not be read).
fn catalogue(bus: &str) -> Option<Arc<Catalogue>> {
    type Kept = HashMap<PathBuf, (Option<SystemTime>, Arc<Catalogue>)>;
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    let path = super::vfs()?.resolve(bus)?;
    let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    let map = kept.get_or_insert_with(HashMap::new);
    if let Some((_, c)) = map.get(&path).filter(|(s, _)| *s == stamp) {
        return Some(c.clone());
    }
    let c = Arc::new(read(&path).ok()?);
    map.insert(path, (stamp, c.clone()));
    Some(c)
}

/// What the game gets for `bus` in `paint` with the choices `picks`, the bus read here and now
/// (`vars_for` without the file the choices are kept in: the checks).
#[cfg(test)]
pub(crate) fn vars_with(root: &Path, bus: &str, paint: &str, picks: &BTreeMap<String, f32>) -> Vec<(String, f32)> {
    setvars(read(&hub_core::paths::resolve_path(root, bus)).ok().as_ref(), paint, picks)
}

/// The options of `cat` as the bus step shows them, with the livery `paint` and the values
/// chosen `picks`.
fn view_of(cat: &Catalogue, paint: &str, picks: &BTreeMap<String, f32>) -> Value {
    let lv = cat.livery(paint);
    let options: Vec<Value> = cat
        .options
        .iter()
        .map(|o| {
            json!({
                "var": o.var, "label": o.label, "technical": o.technical, "switch": o.is_switch(), "usual": o.usual,
                "livery": lv.and_then(|l| l.value(&o.var)), "chosen": pick(picks, &o.var),
                "values": o.values.iter().map(|s| json!({ "value": s.value, "liveries": s.liveries })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "options": options, "reading": false })
}

pub fn view(args: &Value) -> Answer {
    let (bus, paint) = (super::str_arg(args, "bus"), super::str_arg(args, "paint"));
    let Some(cat) = catalogue(&bus) else { return Ok(json!({ "options": [], "reading": false })) };
    Ok(view_of(&cat, &paint, &BusOptions::load().of(&bus)))
}

pub fn set(args: &Value) -> Answer {
    let bus = super::str_arg(args, "bus");
    let mut picks = BusOptions::load();
    if args.get("reset").and_then(Value::as_bool).unwrap_or(false) {
        picks.reset(&bus);
    } else {
        picks.set(&bus, &super::str_arg(args, "var"), args.get("value").and_then(Value::as_f64).map(|v| v as f32));
    }
    picks.save().map_err(|e| e.to_string())?;
    Ok(json!(true))
}

pub fn for_game(args: &Value) -> Answer {
    let (bus, paint) = (super::str_arg(args, "bus"), super::str_arg(args, "paint"));
    Ok(json!(vars_for(&bus, &paint)))
}

/// The bus options chosen for `bus` in `paint`, as the game takes them.
pub fn vars_for(bus: &str, paint: &str) -> Vec<(String, f32)> {
    let picks = BusOptions::load().of(bus);
    if picks.is_empty() {
        return Vec::new();
    }
    setvars(catalogue(bus).as_deref(), paint, &picks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn livery(name: &str, vars: &[(&str, f32)]) -> Livery {
        Livery { name: name.into(), vars: vars.iter().map(|(n, v)| (n.to_string(), *v)).collect() }
    }

    #[test]
    fn the_options_are_read_and_go_to_the_game() {
        assert_eq!(label_of("vis_mirrors"), "Mirrors");
        assert_eq!(label_of("vis_CTI_LEDColor"), "LED color");
        assert_eq!(label_of("Rad_V"), "Rad V");
        let lv = [livery("BVG", &[("vis_mirrors", 1.0), ("Colorscheme", 3.0)]), livery("HHA", &[("vis_mirrors", 0.0), ("vis_seats", 2.0)]), livery("Plain", &[])];
        let shown: HashSet<String> = ["vis_mirrors".to_string()].into_iter().collect();
        let scripts = vec![("main.osc".to_string(), "(l.l.vis_seats)".to_string())];
        let o = gather(&lv, &scripts, &shown);
        assert_eq!(o.iter().map(|x| (x.var.as_str(), x.technical)).collect::<Vec<_>>(), vec![("vis_mirrors", false), ("vis_seats", true)], "the appearance ones first, Colorscheme left out");
        assert!(o[0].is_switch() && !o[1].is_switch());
        assert_eq!(o[0].usual, 0.0, "a livery that sets nothing counts as 0");
        let cat = Catalogue { liveries: lv.to_vec(), options: o };
        let picks: BTreeMap<String, f32> = [("vis_mirrors".to_string(), 1.0), ("vis_seats".to_string(), 2.0)].into_iter().collect();
        assert_eq!(setvars(Some(&cat), "BVG", &picks), vec![("vis_seats".to_string(), 2.0)], "what the livery sets the same is not passed");
        assert_eq!(setvars(None, "BVG", &picks).len(), 2, "without the bus read: every choice");
        let v = view_of(&cat, "hha", &picks);
        assert_eq!(v["options"][0]["livery"], 0.0);
        assert_eq!(v["options"][1]["chosen"], 2.0);
    }

    #[test]
    fn a_livery_and_a_model_read_from_their_files() {
        let cti = CfgFile::from_str("Texture/BVG.cti", "[item]\nBVG Berlin\nfarbschema_tex1\nBVG\\main.dds\n\n[setvar]\nvis_mirrors\n1\n\n[setvar]\nColorscheme\n3\n");
        assert_eq!(livery_of(&cti), livery("BVG Berlin", &[("vis_mirrors", 1.0), ("Colorscheme", 3.0)]));
        let model = CfgFile::from_str("model.cfg", "[mesh]\nmirror.o3d\n\n[visible]\nvis_Mirrors\n1\n\n[matl_change]\nseat.bmp\n0\nvis_seats\n\n\t[visible]\nnot_one\n1\n");
        let shown = shown_in(&model);
        assert!(shown.contains("vis_mirrors") && shown.contains("vis_seats") && !shown.contains("not_one"), "{shown:?}");
        // (the test bus: two paints, none of which sets a variable)
        let c = read(&crate::content::tests::fixture().join("Vehicles/Testbus/Testbus.bus")).unwrap();
        assert_eq!(c.liveries.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Blau", "Red"]);
        assert!(c.options.is_empty());
    }
}
