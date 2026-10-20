//! The devices of a bus, put together from its model by the add-on itself (the career add-on's
//! `core/busmodule.ts`). A `model.cfg` describes the bus part by part, and with each part what
//! is on it:
//!
//! ```text
//! [mesh]              17_almex_s_hst1.o3d
//! [useTextTexture]    25       -> the 26th [texttexture] block of the file is drawn on it
//! [visible]           almex_vis_hst1
//! 1
//! [mesh]              17_almex_clickN7.o3d
//! [mouseevent]        almex_clickN7     -> clicked here, and the bus's script hears this
//! ```
//!
//! So everything is there: which text fields belong together, which keys go with them and what
//! they are called. Nothing has to be done by hand for a bus.

use crate::layout::{self, BoxOf};
use crate::text;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A text field of the model: a `[texttexture]` block as the device list reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The string variable that gives the text.
    pub variable: String,
    /// The font, as it is called inside its `.oft`.
    pub font: String,
}

/// A text field of a device, and the part it lies on.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceField {
    pub field: Field,
    pub part: String,
    /// A field on a key: the ALMEX's eight sales tiles have text in them and are tapped.
    pub action: Option<String>,
}

/// A key as the model describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// The name the bus's script listens to.
    pub action: String,
    /// The part it hangs on.
    pub part: String,
    /// What is written on it, from its action's name.
    pub label: String,
    /// A digit key: its digit.
    pub digit: Option<u8>,
    /// Where its part lies in the bus, when its `.o3d` could be read.
    pub at: Option<BoxOf>,
}

/// A device of the bus: its text fields and its keys.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The key it is kept under: `afr200/afr200`, `/almex`.
    pub id: String,
    /// What it is called above it.
    pub name: String,
    pub fields: Vec<DeviceField>,
    pub keys: Vec<Key>,
}

/// What a device is, for the app's IBIS and ticket printer screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Ibis,
    Printer,
    Other,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Ibis => "ibis",
            Role::Printer => "printer",
            Role::Other => "other",
        }
    }
}

const IS_IBIS: [&str; 7] = ["ibis", "lawo", "annax", "matrix", "bordcomputer", "rbl", "vdv"];
const IS_PRINTER: [&str; 9] = ["afr", "ticketprinter", "ticket_printer", "almex", "atron", "efad", "kasse", "drucker", "kasownik"];

impl Module {
    /// The ticket printer first (a word of one in its names, its fields' or its fonts'), then
    /// the IBIS (`busscherm.ts`' `soortVan`, over the whole device).
    pub fn role(&self) -> Role {
        let words: Vec<String> = std::iter::once(self.id.clone())
            .chain(self.fields.iter().flat_map(|f| [f.field.variable.clone(), f.field.font.clone()]))
            .chain(self.keys.iter().map(|k| k.action.clone()))
            .map(|w| w.to_lowercase())
            .collect();
        let any = |list: &[&str]| words.iter().any(|w| list.iter().any(|x| w.contains(x)));
        if any(&IS_PRINTER) {
            Role::Printer
        } else if any(&IS_IBIS) {
            Role::Ibis
        } else {
            Role::Other
        }
    }
}

/// Words in a name that say nothing about the device: `Bedienteil` is the panel itself,
/// `Taste` and `Num` only say it is a key, `Clickspot` that it is a click field. Used for the
/// keys' labels and for the device a part belongs to.
const GENERIC: &str = "taste tasten tast num numpad button buttons btn knopf knoepfe switch sw key keys \
bedienteil bedienpult panel pult dash dashboard cp cockpit generic misc model mesh obj \
body display displays screen textfeld textfelder text anzeige clickspot klickspot \
click klick spot var lod solo main trail teil part dummy overlay ghost low high \
neu alt links rechts oben unten vorne hinten front heck seite mitte innen aussen \
ticket setmode set mov move toggle push mouse ticketprinter";

fn generic() -> &'static HashSet<&'static str> {
    static SET: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();
    SET.get_or_init(|| GENERIC.split(' ').collect())
}

/// German sounds as a builder writes them without umlauts.
fn sound(word: &str) -> Option<&'static str> {
    Some(match word {
        "ausloesung" => "AUSLÖSUNG",
        "ausloesen" => "AUSLÖSEN",
        "loeschen" => "LÖSCHEN",
        "rueck" => "RÜCK",
        "zurueck" => "ZURÜCK",
        "ueber" => "ÜBER",
        "pfeil" => "▶",
        "enter" => "ENTER",
        "geld" => "GELD",
        "fahrschein" => "FAHRSCHEIN",
        "haltestelle" | "hst" => "HALTE",
        _ => return None,
    })
}

/// The ALMEX, with the names its script writer gives its keys (from the comments of the bus's
/// `Script\17_ticketprinter_almex.osc`): their labels are painted into the screen's pictures, so
/// without this table they would be CLICKN7 and CLICKU3.
fn almex(action: &str) -> Option<String> {
    let a = action.to_lowercase();
    if let Some(i) = a.rfind("_clickn") {
        let rest = &a[i + 7..];
        if rest.len() == 1 && rest.as_bytes()[0].is_ascii_digit() {
            return Some(rest.to_string());
        }
        return match rest {
            "c" => Some("C".into()),
            "b" => Some("OK".into()),
            _ => None,
        };
    }
    if let Some(i) = a.rfind("_clicku") {
        return match &a[i + 7..] {
            "1" => Some("▲".into()),
            "2" => Some("▼".into()),
            "3" => Some("STORNO".into()),
            "4" => Some("PAUSE".into()),
            "5" => Some("MENÜ".into()),
            "6" => Some("ZURÜCK".into()),
            "7" => Some("FIMS".into()),
            _ => None,
        };
    }
    None
}

/// The words of a name, small and without `.o3d`.
fn words_of(name: &str) -> Vec<String> {
    let base = name.strip_suffix(".o3d").or_else(|| name.strip_suffix(".O3D")).unwrap_or(name);
    base.split(|c: char| c == '_' || c.is_whitespace() || c == '.' || c == '-').filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

fn says_nothing(w: &str, noise: &HashSet<String>) -> bool {
    w.chars().count() < 2 || w.bytes().all(|b| b.is_ascii_digit()) || generic().contains(w) || noise.contains(w)
}

fn first_word(name: &str, noise: &HashSet<String>) -> String {
    words_of(name).into_iter().find(|w| !says_nothing(w, noise)).unwrap_or_default()
}

fn last_piece(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// The words that stand in front everywhere in THIS bus (`O550_` in the Setra, `17_` in the
/// Hamburg bus): in front of at least 4 of 10 file names; and their beginnings.
fn noise_words(parts: &[String]) -> HashSet<String> {
    let mut count: HashMap<String, usize> = HashMap::new();
    for p in parts {
        if let Some(w) = words_of(last_piece(p)).into_iter().next() {
            *count.entry(w).or_default() += 1;
        }
    }
    let limit = (parts.len() as f64 * 0.4).max(2.0);
    let mut noise: HashSet<String> = count.into_iter().filter(|(_, n)| *n as f64 >= limit).map(|(w, _)| w).collect();
    for w in noise.clone() {
        let chars: Vec<char> = w.chars().collect();
        for n in 2..chars.len() {
            noise.insert(chars[..n].iter().collect());
        }
    }
    noise
}

/// The folder a part belongs to: the OUTERMOST folder with a word (the HOH MAN A20 puts each
/// page of its ticket printer in a folder of its own; with the innermost that were 26 devices).
fn folder_of(part: &str, noise: &HashSet<String>) -> String {
    let pieces: Vec<&str> = part.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
    for p in pieces.iter().take(pieces.len().saturating_sub(1)) {
        let w = first_word(p, noise);
        if !w.is_empty() {
            return w;
        }
    }
    String::new()
}

/// Actions that belong to a device: what the game has to hand on is named alike by every
/// builder. The doors, the wipers and the seats in the same folder are not.
fn is_device_action(a: &str) -> bool {
    const WORDS: [&str; 26] = [
        "ibis",
        "ticketprinter",
        "ticket_printer",
        "lawo",
        "matrix",
        "annax",
        "rollband",
        "almex",
        "atron",
        "efad",
        "kasse",
        "kasownik",
        "cashdesk",
        "cashlever",
        "drucker",
        "fahrschein",
        "setmode",
        "zielcode",
        "linie",
        "kurs",
        "vdv",
        "rbl",
        "actia",
        "bordcomputer",
        "ticket",
        "printer",
    ];
    let a = a.to_lowercase();
    WORDS.iter().any(|w| a.contains(w))
}

/// Plates and fleet numbers are no screens.
fn no_screen(variable: &str, font: &str) -> bool {
    let v = variable.to_lowercase();
    let f = font.to_lowercase();
    ["ident", "number", "anh_ident", "wagennr", "pvg_wagennr", "kennz"].iter().any(|p| v.starts_with(p)) || ["kennz", "euro", "wagen"].iter().any(|p| f.contains(p))
}

/// One block as the device list reads it, or `None` when it is no screen.
fn field_of(lines: &[String], i: usize) -> Option<Field> {
    let at = |k: usize| lines.get(i + k).map(|l| l.trim()).unwrap_or("");
    let (variable, font) = (at(1), at(2));
    let (w, h) = (at(3).parse::<f64>().ok()?, at(4).parse::<f64>().ok()?);
    if variable.is_empty() || font.is_empty() || no_screen(variable, font) || !(w > 0.0 && h > 0.0) {
        return None;
    }
    Some(Field { variable: variable.to_string(), font: font.to_string() })
}

/// What hangs on one part of the model.
struct Part {
    path: String,
    fields: Vec<usize>,
    actions: Vec<String>,
    visible: Option<String>,
}

/// The model stripped to its parts and text fields (the headings trimmed and small, as this
/// list has always read them).
fn read_model(lines: &[String]) -> (Vec<Option<Field>>, Vec<Part>) {
    let mut fields = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let h = l.trim().to_lowercase();
        if h == "[texttexture]" || h == "[texttexture_enh]" {
            fields.push(field_of(lines, i));
        }
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut open = false;
    for (i, l) in lines.iter().enumerate() {
        let h = l.trim().to_lowercase();
        let next = lines.get(i + 1).map(|l| l.trim().to_string()).unwrap_or_default();
        if h == "[mesh]" {
            open = !next.is_empty();
            if open {
                parts.push(Part { path: next, fields: Vec::new(), actions: Vec::new(), visible: None });
            }
            continue;
        }
        let Some(p) = parts.last_mut().filter(|_| open) else { continue };
        match h.as_str() {
            "[usetexttexture]" => {
                if let Ok(n) = next.parse::<usize>() {
                    if n < fields.len() && !p.fields.contains(&n) {
                        p.fields.push(n);
                    }
                }
            }
            "[mouseevent]" => {
                if !next.is_empty() && !p.actions.contains(&next) {
                    p.actions.push(next);
                }
            }
            "[visible]" if !next.is_empty() && p.visible.is_none() => {
                p.visible = Some(next);
            }
            _ => {}
        }
    }
    (fields, parts)
}

/// A group being made.
#[derive(Debug, Clone, Default)]
struct Group {
    folder: String,
    stem: String,
    fields: Vec<(usize, String, Option<String>)>,
    keys: Vec<(String, String)>,
}

impl Group {
    fn stands_for_something(&self) -> bool {
        !self.fields.is_empty() || self.keys.len() >= 3
    }
    fn add_key(&mut self, action: &str, part: &str) {
        if !self.keys.iter().any(|k| k.0 == action) {
            self.keys.push((action.to_string(), part.to_string()));
        }
    }
}

/// The devices of a bus from its model.cfg (see the module's head).
///
/// The key of a device: the FOLDER and the FIRST WORD that says something. `O550_AFR200_Num_7`
/// belongs to `afr200`, `17_almex_clickN7` and `17_almex_s_hst1` to `almex`. When the file name
/// says nothing (`SD82\SD82_panel.o3d`), what is ON the part decides: the action, the field's
/// variable, the variable that switches it.
///
/// Then tidied twice: a folder is cut in pieces only when two pieces stand for something
/// themselves (else it is one device whose builder named each key), and groups that stand for
/// nothing go together on the stem of their action (the twelve coin trays `cashlever005` to
/// `cashlever8` are one money drawer).
pub fn modules_of(model_cfg: &Path) -> Vec<Module> {
    let Some(lines) = text::read_lines(model_cfg) else { return Vec::new() };
    let model_dir = model_cfg.parent().unwrap_or(Path::new(""));
    modules_from(&lines, model_dir)
}

pub fn modules_from(lines: &[String], model_dir: &Path) -> Vec<Module> {
    let (fields, parts) = read_model(lines);
    if parts.is_empty() {
        return Vec::new();
    }
    let noise = noise_words(&parts.iter().map(|p| p.path.clone()).collect::<Vec<_>>());
    let stem_of = |p: &Part| -> String {
        let w = first_word(last_piece(&p.path), &noise);
        if !w.is_empty() {
            return w;
        }
        for a in &p.actions {
            let w = first_word(a, &noise);
            if !w.is_empty() {
                return w;
            }
        }
        for n in &p.fields {
            let w = first_word(fields[*n].as_ref().map(|f| f.variable.as_str()).unwrap_or(""), &noise);
            if !w.is_empty() {
                return w;
            }
        }
        first_word(p.visible.as_deref().unwrap_or(""), &noise)
    };

    let mut groups: Vec<(String, Group)> = Vec::new();
    fn take<'a>(groups: &'a mut Vec<(String, Group)>, folder: &str, stem: &str) -> &'a mut Group {
        let key = format!("{folder}/{stem}");
        let at = match groups.iter().position(|g| g.0 == key) {
            Some(k) => k,
            None => {
                groups.push((key, Group { folder: folder.into(), stem: stem.into(), ..Group::default() }));
                groups.len() - 1
            }
        };
        &mut groups[at].1
    }
    for p in &parts {
        let actions: Vec<&String> = p.actions.iter().filter(|a| is_device_action(a)).collect();
        if p.fields.is_empty() && actions.is_empty() {
            continue;
        }
        let g = take(&mut groups, &folder_of(&p.path, &noise), &stem_of(p));
        for n in &p.fields {
            if fields[*n].is_some() {
                g.fields.push((*n, p.path.clone(), actions.first().map(|a| a.to_string())));
            }
        }
        for a in &actions {
            g.add_key(a, &p.path);
        }
    }

    // a folder with only one piece that stands for something is one device
    let folders: Vec<String> = groups.iter().map(|g| g.1.folder.clone()).filter(|f| !f.is_empty()).collect::<HashSet<_>>().into_iter().collect();
    for folder in folders {
        let in_it: Vec<usize> = groups.iter().enumerate().filter(|(_, g)| g.1.folder == folder).map(|(k, _)| k).collect();
        if in_it.len() < 2 || in_it.iter().filter(|k| groups[**k].1.stands_for_something()).count() >= 2 {
            continue;
        }
        let members: Vec<Group> = in_it.iter().map(|k| groups[*k].1.clone()).collect();
        let keys_of: Vec<String> = in_it.iter().map(|k| groups[*k].0.clone()).collect();
        groups.retain(|g| !keys_of.contains(&g.0) || g.0 == format!("{folder}/"));
        let all = take(&mut groups, &folder, "");
        for m in members {
            if m.stem.is_empty() {
                continue;
            }
            all.fields.extend(m.fields);
            for (a, p) in m.keys {
                all.add_key(&a, &p);
            }
        }
    }

    // what stands for nothing alone, together on the stem of its action
    let mut k = 0;
    while k < groups.len() {
        let g = &groups[k].1;
        if g.stands_for_something() || g.keys.is_empty() {
            k += 1;
            continue;
        }
        let stem = first_word(&g.keys[0].0, &HashSet::new());
        // (a group put together here already stays: with fewer than three keys it came back to
        // this line for ever - the stock MAN SD80)
        if stem.is_empty() || stem == g.stem || g.stem.starts_with('~') {
            k += 1;
            continue;
        }
        let moved = groups.remove(k).1;
        let target = format!("~{stem}");
        let all = take(&mut groups, &moved.folder, &target);
        all.fields.extend(moved.fields);
        for (a, p) in moved.keys {
            all.add_key(&a, &p);
        }
    }

    // keys away from their screen: the action's name brings them to the device whose fields
    // start alike (`IBIS_setmode_linie_kurs` to `IBIS_...`); only with a clear winner
    let word = |n: &str| n.split(|c: char| c == '_' || c.is_whitespace() || c == '-').next().unwrap_or("").to_lowercase();
    for k in 0..groups.len() {
        if !groups[k].1.fields.is_empty() || groups[k].1.keys.is_empty() {
            continue;
        }
        let words: HashSet<String> = groups[k].1.keys.iter().map(|(a, _)| word(a)).collect();
        let mut fits: Vec<(usize, usize)> = groups
            .iter()
            .enumerate()
            .filter(|(_, g)| !g.1.fields.is_empty())
            .map(|(j, g)| (j, g.1.fields.iter().filter(|(n, _, _)| words.contains(&word(fields[*n].as_ref().map(|f| f.variable.as_str()).unwrap_or("")))).count()))
            .filter(|(_, hits)| *hits > 0)
            .collect();
        fits.sort_by_key(|f| std::cmp::Reverse(f.1));
        if fits.is_empty() || (fits.len() > 1 && fits[0].1 == fits[1].1) {
            continue;
        }
        let keys = std::mem::take(&mut groups[k].1.keys);
        for (a, p) in keys {
            groups[fits[0].0].1.add_key(&a, &p);
        }
    }

    // and made up: a text field, or enough keys to be a control panel (six; fewer is a switch
    // on the dashboard). The newest buses draw their computer with a script texture, but its
    // keys are there and it can still be worked from the tablet.
    let mut out = Vec::new();
    for (key, g) in groups {
        if g.fields.is_empty() && g.keys.len() < 6 {
            continue;
        }
        let paths: Vec<String> = g.fields.iter().map(|f| f.1.clone()).chain(g.keys.iter().map(|k| k.1.clone())).collect::<Vec<_>>();
        let mut seen_paths = HashSet::new();
        let paths: Vec<String> = paths.into_iter().filter(|p| seen_paths.insert(p.clone())).collect();
        let boxes = layout::boxes_of_parts(model_dir, &paths);
        let mut seen = HashSet::new();
        let mut dfields = Vec::new();
        for (n, part, action) in &g.fields {
            let Some(f) = fields[*n].clone() else { continue };
            if !seen.insert(f.variable.clone()) {
                continue;
            }
            dfields.push(DeviceField { field: f, part: part.clone(), action: action.clone() });
        }
        // what stands in front of at least two actions is a prefix, not a label (the AFR 200's
        // keys are `IBIS_7` and `ticketprinter_enter`)
        let mut front: HashMap<String, usize> = HashMap::new();
        for (a, _) in &g.keys {
            let w: Vec<&str> = a.split(|c: char| c == '_' || c.is_whitespace() || c == '-').filter(|w| !w.is_empty()).collect();
            if w.len() >= 2 {
                *front.entry(w[0].to_lowercase()).or_default() += 1;
            }
        }
        let mut prefixes: HashSet<String> = HashSet::from([g.stem.clone()]);
        prefixes.extend(front.into_iter().filter(|(_, n)| *n >= 2).map(|(w, _)| w));
        let keys = g
            .keys
            .iter()
            .map(|(a, p)| {
                let label = label_of(a, &prefixes, p, &noise);
                let digit = (label.len() == 1 && label.as_bytes()[0].is_ascii_digit()).then(|| label.as_bytes()[0] - b'0');
                Key { action: a.clone(), part: p.clone(), label, digit, at: boxes.get(p).copied() }
            })
            .collect();
        out.push(Module { name: name_of(&key, &dfields), id: key, fields: dfields, keys });
    }
    // a device that can be worked first; the rest are gauges and clocks
    out.sort_by_key(|m| m.keys.is_empty());
    out
}

/// A key's label from its action's name: the device off it, then what the key does. `IBIS_7`
/// is "7", `ticketprinter_button_enter` "ENTER", `cashdesk_changer_0_50` "0,50". When only a
/// number is left and the part's name says more (`ticketprinter_1` on `..._Ticket_Kurz`), the
/// part's words.
pub fn label_of(action: &str, prefixes: &HashSet<String>, part: &str, noise: &HashSet<String>) -> String {
    if let Some(l) = almex(action) {
        return l;
    }
    if let Some(l) = coin_label(action) {
        return l;
    }
    let words = words_of(action);
    let without: Vec<&String> =
        words.iter().enumerate().filter(|(i, w)| !(*i == 0 && prefixes.contains(*w)) && (!generic().contains(w.as_str()) || w.bytes().all(|b| b.is_ascii_digit()))).map(|(_, w)| w).collect();
    let core: Vec<&String> = if !without.is_empty() {
        without
    } else if words.len() > 1 {
        words[1..].iter().collect()
    } else {
        words.iter().collect()
    };
    let shown = |w: &str| -> String {
        if let Some(s) = sound(w) {
            return s.to_string();
        }
        if w.bytes().all(|b| b.is_ascii_digit()) {
            return w.parse::<u64>().map(|n| n.to_string()).unwrap_or_else(|_| w.to_string());
        }
        w.to_uppercase()
    };
    let text = core.iter().map(|w| shown(w)).collect::<Vec<_>>().join(" ").trim().to_string();
    let from_part = words_of(last_piece(part))
        .into_iter()
        .filter(|w| w.bytes().all(|b| b.is_ascii_digit()) || (!says_nothing(w, noise) && !prefixes.contains(w)))
        .map(|w| sound(&w).map(str::to_string).unwrap_or_else(|| w.to_uppercase()))
        .collect::<Vec<_>>()
        .join(" ");
    let only_number = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if !text.is_empty() && !(only_number && from_part.chars().any(|c| c.is_alphabetic())) {
        return text;
    }
    if !from_part.is_empty() {
        return from_part;
    }
    if !text.is_empty() {
        return text;
    }
    action.to_uppercase()
}

/// Money: `cashdesk_changer_0_50` is fifty cents, `cashdesk_change_0500` five euros.
fn coin_label(action: &str) -> Option<String> {
    let a = action.to_lowercase();
    for w in ["_changer", "_change", "_geldscheine", "_schein", "_muenze"] {
        let Some(i) = a.rfind(w) else { continue };
        let rest = a[i + w.len()..].trim_start_matches('_');
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.is_empty() || parts.len() > 2 || !parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())) {
            continue;
        }
        let (euro, cent) = if parts.len() == 2 {
            (parts[0].parse::<u64>().ok()?, parts[1].parse::<u64>().ok()?)
        } else {
            let n = parts[0].parse::<u64>().ok()?;
            (n / 100, n % 100)
        };
        return Some(format!("{euro},{cent:02}"));
    }
    None
}

/// What the device is called: its key (`afr200/afr200` is AFR200, `/almex` ALMEX), or the
/// first field's variable.
fn name_of(key: &str, fields: &[DeviceField]) -> String {
    let mut unique: Vec<String> = Vec::new();
    for piece in key.split('/') {
        let p = piece.trim_start_matches('~').replace(['_', '-'], " ").trim().to_lowercase();
        if !p.is_empty() && !unique.contains(&p) {
            unique.push(p);
        }
    }
    if !unique.is_empty() {
        return unique.join(" ").to_uppercase();
    }
    let first = fields.first().map(|f| f.field.variable.as_str()).unwrap_or("screen");
    first.replace(['_', '-'], " ").trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ').trim().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(t: &str) -> Vec<String> {
        text::lines_of(t.as_bytes())
    }

    #[test]
    fn labels_come_from_the_actions_and_the_almex_has_its_own() {
        let none = HashSet::new();
        let ibis: HashSet<String> = ["ibis".to_string()].into();
        assert_eq!(label_of("IBIS_7", &ibis, "x.o3d", &none), "7");
        assert_eq!(label_of("ticketprinter_button_enter", &none, "x.o3d", &none), "ENTER");
        assert_eq!(label_of("IBIS_loeschen", &ibis, "x.o3d", &none), "LÖSCHEN");
        assert_eq!(label_of("17_almex_clickN7", &none, "x.o3d", &none), "7");
        assert_eq!(label_of("almex_clickU5", &none, "x.o3d", &none), "MENÜ");
        assert_eq!(label_of("cashdesk_changer_0_50", &none, "x.o3d", &none), "0,50");
        assert_eq!(label_of("cashdesk_change_0500", &none, "x.o3d", &none), "5,00");
        // (`O550` stands in front of everything in that bus, `afr200` is the device's stem)
        let tp: HashSet<String> = ["ticketprinter".to_string(), "afr200".to_string()].into();
        let o550: HashSet<String> = ["o550".to_string()].into();
        assert_eq!(label_of("ticketprinter_1", &tp, "O550_AFR200_Ticket_Kurz.o3d", &o550), "KURZ");
    }

    #[test]
    fn a_device_is_its_fields_and_keys_found_by_their_names() {
        let cfg = "[mesh]\r\nibis\\ibis_screen.o3d\r\n[matl]\r\nlcd.bmp\r\n0\r\n[useTextTexture]\r\n0\r\n\
[mesh]\r\nibis\\ibis_key_7.o3d\r\n[mouseevent]\r\nIBIS_7\r\n[mesh]\r\nibis\\ibis_key_8.o3d\r\n[mouseevent]\r\nIBIS_8\r\n\
[mesh]\r\ndoor.o3d\r\n[mouseevent]\r\ndoor_1\r\n[mesh]\r\nseat.o3d\r\n[mesh]\r\nwheel.o3d\r\n[mesh]\r\nmirror.o3d\r\n[mesh]\r\nbody.o3d\r\n\
[texttexture]\r\nIBIS_display_1\r\nIBIS font\r\n256\r\n32\r\n0\r\n0\r\n0\r\n0\r\n\
[texttexture]\r\nident\r\nKennzeichen\r\n128\r\n32\r\n0\r\n0\r\n0\r\n0\r\n";
        let m = modules_from(&lines(cfg), Path::new("/nowhere"));
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(m[0].id, "ibis/ibis");
        assert_eq!(m[0].name, "IBIS");
        assert_eq!(m[0].fields[0].field.variable, "IBIS_display_1");
        let keys: Vec<(&str, &str, Option<u8>)> = m[0].keys.iter().map(|k| (k.action.as_str(), k.label.as_str(), k.digit)).collect();
        assert_eq!(keys, [("IBIS_7", "7", Some(7)), ("IBIS_8", "8", Some(8))]);
        assert_eq!(m[0].role(), Role::Ibis);
    }

    #[test]
    fn six_printer_keys_without_a_screen_are_a_device_too() {
        let mut cfg = String::new();
        for i in 1..=6 {
            cfg += &format!("[mesh]\r\nTB_printer_key_{i}.o3d\r\n[mouseevent]\r\nticketprinter_button_ticket_{i}\r\n");
        }
        cfg += "[mesh]\r\nTB_body.o3d\r\n[mesh]\r\nTB_seat.o3d\r\n";
        let m = modules_from(&lines(&cfg), Path::new("/nowhere"));
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].keys.len(), 6);
        assert_eq!(m[0].role(), Role::Printer);
    }

    #[test]
    fn a_lone_key_goes_to_the_stem_of_its_action_once() {
        // (one key on a part named otherwise: to `~ibis`, where it still stands for nothing -
        // and before, it was moved there again and again; the stock MAN SD80 never finished)
        let cfg = "[mesh]\r\nbox.o3d\r\n[mouseevent]\r\nIBIS_7\r\n[mesh]\r\nbody.o3d\r\n[mesh]\r\nseat.o3d\r\n";
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(modules_from(&lines(cfg), Path::new("/nowhere")));
        });
        let m = rx.recv_timeout(std::time::Duration::from_secs(10)).expect("the devices are found, and in time");
        // (and one key without a screen is no device)
        assert!(m.is_empty(), "{m:?}");
    }
}
