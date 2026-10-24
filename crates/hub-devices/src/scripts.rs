//! The start values of number variables as the bus's scripts set them: a FALLBACK for when the
//! plugin does not (yet) give an animation variable; the live value always goes first (the
//! career add-on's `startwaardenVan`).
//!
//! OMSI's scripts set a value with `<number> (S.L.<name>)`, and chain that:
//! `1 (S.L.cp_lenkrad_visible) (S.L.mt_ruhezeit) (S.L.trans_dauer)` in `{macro:cockpit_init}` of
//! the Hamburg bus, which `{init}` calls. Without `trans_dauer = 1` the six lines of ALMEX menu
//! 26 lie 1.3 m beside the screen.
//!
//! Only what happens at the start counts: the `{init}` blocks of the bus's scripts, in the order
//! of `[script]` in the `.bus` file, with every `(M.L.macro)` they call; then `{frame}`, where
//! what is set UNconditionally holds from the first frame. Priority: unconditional in `{frame}`
//! (the last), then unconditional in `{init}` (the last), then inside an `{if}` in `{init}` (the
//! first). Triggers (keys, the mouse) do not count: they happen only when the player acts.

use crate::text;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// The start values of all variables the scripts of this `model.cfg`'s bus set, by lower-case
/// name.
pub fn start_values(model_cfg: &Path) -> HashMap<String, f64> {
    read_start_values(&scripts_of(model_cfg))
}

/// A path without `.` and `..`, in lower case with `/`: Windows compares paths so.
pub(crate) fn norm(p: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut root = String::new();
    for c in p.components() {
        match c {
            Component::Prefix(x) => root = x.as_os_str().to_string_lossy().to_lowercase(),
            Component::RootDir => root.push('/'),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(x) => {
                for part in x.to_string_lossy().split('\\') {
                    match part {
                        "" | "." => {}
                        ".." => {
                            parts.pop();
                        }
                        _ => parts.push(part.to_lowercase()),
                    }
                }
            }
        }
    }
    format!("{root}{}", parts.join("/"))
}

/// A path as the game's files write it (`Script\main.osc`), beside a folder.
pub(crate) fn join_rel(dir: &Path, rel: &str) -> PathBuf {
    rel.split(['\\', '/']).filter(|p| !p.is_empty()).fold(dir.to_path_buf(), |d, p| d.join(p))
}

/// As NTFS lists a folder: by upper case.
pub(crate) fn by_name(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_uppercase().cmp(&b.to_uppercase())
}

/// `<OMSI>\Vehicles\<bus>` of a model.cfg, wherever under it the cfg is.
pub fn vehicle_dir_of(model_cfg: &Path) -> PathBuf {
    let mut here = model_cfg.parent().unwrap_or(Path::new("")).to_path_buf();
    for _ in 0..6 {
        let Some(up) = here.parent() else { break };
        if up.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("vehicles")) {
            return here;
        }
        if up == here {
            break;
        }
        here = up.to_path_buf();
    }
    model_cfg.parent().and_then(Path::parent).unwrap_or(Path::new("")).to_path_buf()
}

fn sorted_dir(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort_by(|a, b| by_name(a, b));
    names
}

/// The `model.cfg` a `.bus` file names under `[model]`, beside the bus file.
pub fn model_of_bus(bus_file: &Path) -> Option<PathBuf> {
    let lines = text::read_lines(bus_file)?;
    let at = lines.iter().position(|l| l.trim().eq_ignore_ascii_case("[model]"))?;
    let rel = lines.get(at + 1)?.trim();
    let cfg = join_rel(bus_file.parent()?, rel);
    (!rel.is_empty() && cfg.is_file()).then_some(cfg)
}

/// The scripts of the bus with this model.cfg: the `[script]` list of the first `.bus` in the
/// vehicle folder whose `[model]` points at this cfg. (The same folder has the scripts of other
/// variants, which do not belong.) No `.bus` found: all `.osc` under `Script\`, by name.
pub fn scripts_of(model_cfg: &Path) -> Vec<PathBuf> {
    let dir = vehicle_dir_of(model_cfg);
    let goal = norm(model_cfg);
    for bus in sorted_dir(&dir).into_iter().filter(|n| n.to_lowercase().ends_with(".bus")) {
        let Some(lines) = text::read_lines(&dir.join(&bus)) else { continue };
        let Some(model) = lines.iter().position(|l| l == "[model]") else { continue };
        if norm(&join_rel(&dir, lines.get(model + 1).map(|s| s.trim()).unwrap_or(""))) != goal {
            continue;
        }
        let mut out = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line != "[script]" {
                continue;
            }
            let count = lines.get(i + 1).and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(0).min(1000);
            for k in 0..count {
                let rel = lines.get(i + 2 + k).map(|s| s.trim()).unwrap_or("");
                if !rel.is_empty() {
                    out.push(join_rel(&dir, rel));
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    let mut out = Vec::new();
    fn walk(here: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > 3 {
            return;
        }
        for name in sorted_dir(here) {
            let full = here.join(&name);
            let lower = name.to_lowercase();
            if lower.ends_with(".osc") {
                out.push(full);
            } else if !has_extension(&lower) {
                walk(&full, depth + 1, out);
            }
        }
    }
    walk(&dir.join("Script"), 0, &mut out);
    out
}

/// `\.[a-z0-9]{1,4}$`
fn has_extension(lower: &str) -> bool {
    match lower.rfind('.') {
        Some(i) => {
            let ext = &lower[i + 1..];
            (1..=4).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        }
        None => false,
    }
}

#[derive(Debug, PartialEq)]
enum BlockKind {
    Init,
    Frame,
    Macro,
    Other,
}

struct Block {
    kind: BlockKind,
    name: String,
    words: Vec<String>,
}

fn blocks_of(file: &Path) -> Vec<Block> {
    let Some(lines) = text::read_lines(file) else { return Vec::new() };
    let mut out: Vec<Block> = Vec::new();
    let mut open = false;
    for line in lines {
        // (a comment starts with an apostrophe)
        let line = match line.find('\'') {
            Some(i) => &line[..i],
            None => &line[..],
        };
        let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        if words.is_empty() {
            continue;
        }
        let first = words[0].to_lowercase();
        if first == "{end}" {
            open = false;
            continue;
        }
        let head = first.strip_prefix('{').and_then(|r| r.strip_suffix('}')).filter(|inner| !inner.contains('}'));
        if let (Some(inner), false) = (head, matches!(first.as_str(), "{if}" | "{else}" | "{endif}")) {
            let (kind, name) = match inner {
                "init" => (BlockKind::Init, String::new()),
                "frame" => (BlockKind::Frame, String::new()),
                _ => match inner.strip_prefix("macro:") {
                    Some(n) => (BlockKind::Macro, n.trim().to_string()),
                    None => (BlockKind::Other, String::new()),
                },
            };
            out.push(Block { kind, name, words: words[1..].to_vec() });
            open = true;
            continue;
        }
        if open {
            if let Some(b) = out.last_mut() {
                b.words.extend(words);
            }
        }
    }
    out
}

/// `^[-+]?(\d+\.?\d*|\.\d+)$`
fn literal(w: &str) -> Option<f64> {
    let s = w.strip_prefix(['-', '+']).unwrap_or(w);
    let ok = match s.split_once('.') {
        Some((a, b)) => (!a.is_empty() || !b.is_empty()) && a.bytes().all(|c| c.is_ascii_digit()) && b.bytes().all(|c| c.is_ascii_digit()) && !(a.is_empty() && b.is_empty()),
        None => !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()),
    };
    if !ok {
        return None;
    }
    let v: f64 = s.parse().ok().or_else(|| format!("{s}0").parse().ok())?;
    Some(if w.starts_with('-') { -v } else { v })
}

/// `(x.l.<name>)` in lower case.
fn call<'a>(lower: &'a str, kind: &str) -> Option<&'a str> {
    lower.strip_prefix('(')?.strip_suffix(')')?.strip_prefix(kind).filter(|n| !n.is_empty() && !n.contains(')'))
}

#[derive(Default, Clone)]
struct Effect {
    /// Outside every `{if}`: the last value counts.
    fixed: Vec<(String, f64)>,
    /// Inside an `{if}`: the first value counts.
    conditional: Vec<(String, f64)>,
}

impl Effect {
    fn set(list: &mut Vec<(String, f64)>, name: &str, v: f64, first: bool) {
        match list.iter_mut().find(|(n, _)| n == name) {
            Some(e) if !first => e.1 = v,
            Some(_) => {}
            None => list.push((name.to_string(), v)),
        }
    }
}

struct Run<'a> {
    macros: HashMap<&'a str, &'a [String]>,
    done: HashMap<String, Effect>,
    busy: HashSet<String>,
}

impl Run<'_> {
    fn effect_of(&mut self, words: &[String]) -> Effect {
        let mut e = Effect::default();
        let mut depth = 0usize;
        let mut value: Option<f64> = None;
        for word in words {
            if let Some(v) = literal(word) {
                value = Some(v);
                continue;
            }
            let lower = word.to_lowercase();
            if let Some(name) = call(&lower, "s.l.") {
                // (storing leaves the number on the stack: `1 (S.L.a) (S.L.b)`)
                if let Some(v) = value {
                    if depth == 0 {
                        Effect::set(&mut e.fixed, name, v, false);
                    } else {
                        Effect::set(&mut e.conditional, name, v, true);
                    }
                }
                continue;
            }
            value = None;
            if lower == "{if}" {
                depth += 1;
            } else if lower == "{endif}" {
                depth = depth.saturating_sub(1);
            } else if let Some(name) = call(&lower, "m.l.") {
                let Some(inner) = self.macro_effect(name) else { continue };
                for (n, v) in &inner.fixed {
                    if depth == 0 {
                        Effect::set(&mut e.fixed, n, *v, false);
                    } else {
                        Effect::set(&mut e.conditional, n, *v, true);
                    }
                }
                for (n, v) in &inner.conditional {
                    Effect::set(&mut e.conditional, n, *v, true);
                }
            }
        }
        e
    }

    /// A macro is run once and then applied as its summary: run at every call, the Atron's
    /// macros of the MB C2 call each other so often that it took 6 s.
    fn macro_effect(&mut self, name: &str) -> Option<Effect> {
        if let Some(e) = self.done.get(name) {
            return Some(e.clone());
        }
        let words = *self.macros.get(name)?;
        // (a macro calling itself, round about: that second time does not count)
        if !self.busy.insert(name.to_string()) {
            return None;
        }
        let e = self.effect_of(words);
        self.busy.remove(name);
        self.done.insert(name.to_string(), e.clone());
        Some(e)
    }
}

fn read_start_values(scripts: &[PathBuf]) -> HashMap<String, f64> {
    let blocks: Vec<Block> = scripts.iter().flat_map(|s| blocks_of(s)).collect();
    let mut macros: HashMap<&str, &[String]> = HashMap::new();
    for b in &blocks {
        if b.kind == BlockKind::Macro {
            macros.entry(b.name.as_str()).or_insert(&b.words);
        }
    }
    let mut run = Run { macros, done: HashMap::new(), busy: HashSet::new() };
    let mut init: HashMap<String, f64> = HashMap::new();
    let mut init_conditional: HashMap<String, f64> = HashMap::new();
    let mut frame: HashMap<String, f64> = HashMap::new();
    for b in blocks.iter().filter(|b| b.kind == BlockKind::Init) {
        let e = run.effect_of(&b.words);
        init.extend(e.fixed);
        for (n, v) in e.conditional {
            init_conditional.entry(n).or_insert(v);
        }
    }
    for b in blocks.iter().filter(|b| b.kind == BlockKind::Frame) {
        frame.extend(run.effect_of(&b.words).fixed);
    }
    let mut out = init_conditional;
    out.extend(init);
    out.extend(frame);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_macros_and_frame_set_start_values_in_omsis_order() {
        let dir = std::env::temp_dir().join(format!("hub-devices-scripts-{}", std::process::id()));
        let bus = dir.join("Vehicles").join("Bus");
        std::fs::create_dir_all(bus.join("Model")).unwrap();
        std::fs::create_dir_all(bus.join("Script")).unwrap();
        std::fs::write(bus.join("Model").join("model.cfg"), "").unwrap();
        std::fs::write(bus.join("a.bus"), "[model]\r\nModel\\model.cfg\r\n\r\n[script]\r\n2\r\nScript\\main.osc\r\nScript\\more.osc\r\n").unwrap();
        std::fs::write(
            bus.join("Script").join("main.osc"),
            "{init}\r\n  (M.L.cockpit_init) 'comment 9 (S.L.x)\r\n  {if} 5 (S.L.cond) (S.L.both) {endif}\r\n{end}\r\n\
{macro:cockpit_init}\r\n  1 (S.L.trans_dauer) (S.L.Lenkrad)\r\n  -0.22 (S.L.vdv_move_x)\r\n{end}\r\n\
{trigger:vdv_move}\r\n  -1 (S.L.vdv_move_x)\r\n{end}\r\n",
        )
        .unwrap();
        std::fs::write(bus.join("Script").join("more.osc"), "{init}\r\n 2 (S.L.both)\r\n{end}\r\n{frame}\r\n 7 (S.L.lenkrad)\r\n{end}\r\n").unwrap();
        assert_eq!(model_of_bus(&bus.join("a.bus")), Some(bus.join("Model").join("model.cfg")));
        let v = start_values(&bus.join("Model").join("model.cfg"));
        assert_eq!(v.get("trans_dauer"), Some(&1.0));
        assert_eq!(v.get("vdv_move_x"), Some(&-0.22), "a trigger does not count");
        assert_eq!(v.get("lenkrad"), Some(&7.0), "frame goes over init");
        assert_eq!(v.get("cond"), Some(&5.0));
        assert_eq!(v.get("both"), Some(&2.0), "unconditional goes over conditional");
        assert_eq!(v.get("x"), None, "a comment");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn literals_are_numbers_only() {
        assert_eq!(literal("1"), Some(1.0));
        assert_eq!(literal("-0.22"), Some(-0.22));
        assert_eq!(literal(".5"), Some(0.5));
        assert_eq!(literal("3."), Some(3.0));
        assert_eq!(literal("."), None);
        assert_eq!(literal("1e5"), None);
        assert_eq!(literal("(L.L.x)"), None);
    }
}
