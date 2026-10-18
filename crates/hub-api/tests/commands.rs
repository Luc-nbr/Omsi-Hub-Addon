//! Every command the page calls is one the dispatcher knows: the command names in `ui/js/**`
//! (the browser mock left out: it answers whatever it likes) checked against `hub-api`.
//!
//! A name is a quoted string (`'co_view'`, `"hub_drive"`) of one of the dispatcher's prefixes, on
//! a line that is not a comment. It counts as known when `hub-api`'s own code (its tests left
//! out) names it as a literal - its `match` arm. The prefix whose module is not ported yet
//! (`tg_`) is asked of the dispatcher itself, which must say "not ported yet"
//! (they do nothing): those are listed, not failed.

use std::path::{Path, PathBuf};

const PREFIXES: &[&str] = &["hub_", "addon_", "co_", "rf_", "tg_", "ol_", "cs_", "tt_", "de_", "b3d_", "lak_"];
const NOT_PORTED: &[&str] = &["tg_"];
/// The window's own commands, which `api.js` answers itself (`OWN`): never sent to hub-api.
const WINDOW: &[&str] = &["pick_folder", "pick_file", "open_url", "open_path"];
/// The page's ways to send a command (`api.js`).
const SENDERS: &[&str] = &["call(", "callBytes(", "bytesUrl("];

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

/// The command names quoted in a script's code (its comments left out).
fn names_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let code = line.trim_start();
        if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
            continue;
        }
        // (what follows `//` on a line is a comment too, unless it is inside a URL)
        let code = match code.find(" //") {
            Some(k) => &code[..k],
            None => code,
        };
        // (the first argument of the page's senders, whatever its name: an unprefixed one is
        // unknown too, unless it is one of the window's own)
        for sender in SENDERS {
            let mut from = 0;
            while let Some(k) = code[from..].find(sender).map(|k| k + from) {
                from = k + sender.len();
                // (`x.call(`, `recall(`: not the page's sender)
                if code[..k].chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                    continue;
                }
                let arg = code[from..].trim_start();
                let Some(q) = arg.chars().next().filter(|c| *c == '\'' || *c == '"') else { continue };
                let Some(end) = arg[1..].find(q) else { continue };
                let name = &arg[1..1 + end];
                let plain = !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
                if plain && !WINDOW.contains(&name) && !PREFIXES.iter().any(|p| name.starts_with(p)) {
                    out.push(name.to_string());
                }
            }
        }
        for q in ['\'', '"'] {
            let parts: Vec<&str> = code.split(q).collect();
            for s in parts.iter().skip(1).step_by(2) {
                let ok = PREFIXES.iter().any(|p| s.starts_with(p) && s.len() > p.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
                if ok {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

/// The string literals of hub-api's code, its test modules left out.
fn literals() -> String {
    let mut rs = Vec::new();
    files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), "rs", &mut rs);
    let mut all = String::new();
    for f in rs {
        let text = std::fs::read_to_string(&f).unwrap();
        let cut = text.find("#[cfg(test)]\nmod tests").or_else(|| text.find("#[cfg(test)]\r\nmod tests")).unwrap_or(text.len());
        all.push_str(&text[..cut]);
        all.push('\n');
    }
    all
}

#[test]
fn every_command_the_page_calls_is_known_to_the_dispatcher() {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/js");
    let mut js = Vec::new();
    files(&ui, "js", &mut js);
    let js: Vec<PathBuf> = js.into_iter().filter(|p| !p.to_string_lossy().replace('\\', "/").contains("/mock")).collect();
    assert!(js.len() > 100, "the page's scripts were not found in {}", ui.display());
    let rust = literals();
    let mut seen: Vec<(String, String)> = Vec::new();
    for f in &js {
        let text = std::fs::read_to_string(f).unwrap();
        for n in names_in(&text) {
            if !seen.iter().any(|(x, _)| *x == n) {
                seen.push((n, f.strip_prefix(&ui).unwrap_or(f).to_string_lossy().into_owned()));
            }
        }
    }
    assert!(seen.len() > 50, "only {} command names found", seen.len());
    let mut unknown = Vec::new();
    let mut later = Vec::new();
    for (n, file) in &seen {
        if NOT_PORTED.iter().any(|p| n.starts_with(p)) {
            let answer = hub_api::call(n, &serde_json::json!({}), &hub_api::Quiet);
            if answer != Err(hub_api::not_ported(n)) && !rust.contains(&format!("\"{n}\"")) {
                unknown.push(format!("{n} ({file}): {answer:?}"));
            } else {
                later.push(n.as_str());
            }
            continue;
        }
        // (a name of no prefix is no module's: the dispatcher answers "unknown command")
        if !PREFIXES.iter().any(|p| n.starts_with(p)) || !rust.contains(&format!("\"{n}\"")) {
            unknown.push(format!("{n} ({file})"));
        }
    }
    if !later.is_empty() {
        later.sort();
        println!("not ported yet ({}): {}", later.len(), later.join(", "));
    }
    assert!(unknown.is_empty(), "the page calls commands hub-api does not know:\n{}", unknown.join("\n"));
}

#[test]
fn a_command_name_is_read_from_code_not_from_comments() {
    let text = "// call('hub_nope')\n * `co_doc`\nconst a = call('co_view', { view: 'home' }); // 'rf_comment'\nask(\"ol_state\");\nclass='co-pg'\ncall('ibis', {});\nx.call('nope');\ncall('pick_folder', {});\n";
    assert_eq!(names_in(text), ["co_view", "ol_state", "ibis"]);
}
