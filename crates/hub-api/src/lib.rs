//! Omsi-Hub's commands: what the page asks for (`ui/js/api.js` `call(cmd, args)`), answered as
//! JSON, or as bytes for pictures (`call_bytes`, the app's `hub://localhost/bytes/<cmd>`).
//!
//! A command's prefix says whose it is, and each prefix is one module (one port):
//!
//! | prefix | module | what |
//! |---|---|---|
//! | `hub_` | [`hub`] | the app: boot, texts, first-run setup, migration, drives |
//! | `hub_update` | [`update`] | the app's own updates (this repository's releases) |
//! | `addon_` | [`addon`] | the plugin in the player's openOMSI, the bridge |
//! | `co_` | [`company`] | the bus company |
//! | `rf_` | [`rosterflow`] | RosterFlow |
//! | `tg_` | [`together`] | playing together |
//! | `ol_` | [`online`] | the account, friends, the leaderboard |
//! | `cs_` | [`cloud`] | the cloud saves |
//! | `tt_` | [`timetable`] | the timetable editor |
//! | `de_` | [`depot`] | the depot editor |
//!
//! A command no module knows answers `unknown command`; one whose module is not ported yet
//! answers [`not_ported`]. Arguments are one JSON object; [`arg`] reads one of them.

pub mod addon;
pub mod cloud;
pub mod company;
pub mod depot;
pub mod drive;
pub mod hub;
pub mod launcher;
pub mod online;
pub mod privacy;
pub mod rosterflow;
pub mod timetable;
pub mod together;
pub mod update;

use serde_json::Value;

/// A command's answer.
pub type Answer = Result<Value, String>;

/// A way to tell the page something that outlives the command (a thread watching the game).
pub type Emitter = std::sync::Arc<dyn Fn(&str, Value) + Send + Sync>;

/// What the commands need of the app around them (the Tauri shell, or a test).
pub trait Platform: Send + Sync {
    /// Tell the page something (`game-ended`, `progress` …), with a JSON payload.
    fn emit(&self, _event: &str, _payload: Value) {}
    /// End the app.
    fn quit(&self) {}
    /// [`Platform::emit`] for a thread that outlives the command (None: nothing to tell).
    fn emitter(&self) -> Option<Emitter> {
        None
    }
}

/// A platform that does nothing (tests, the command line).
pub struct Quiet;

impl Platform for Quiet {}

/// The answer of a command whose module is not ported yet.
pub fn not_ported(cmd: &str) -> String {
    format!("not ported yet: {cmd}")
}

/// One argument of `args` (`args.name`), read as `T`; a missing one reads as `null` (so an
/// `Option` is None).
pub fn arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null)).map_err(|e| format!("{name}: {e}"))
}

/// Any value as an answer.
pub fn to_json<T: serde::Serialize>(v: T) -> Answer {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// A command of the page.
pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Answer {
    match cmd {
        c if c.starts_with("hub_update") => update::call(c, args, p),
        c if c.starts_with("hub_") => hub::call(c, args, p),
        c if c.starts_with("addon_") => addon::call(c, args, p),
        c if c.starts_with("co_") => company::call(c, args, p),
        c if c.starts_with("rf_") => rosterflow::call(c, args, p),
        c if c.starts_with("tg_") => together::call(c, args, p),
        c if c.starts_with("ol_") => online::call(c, args, p),
        c if c.starts_with("cs_") => cloud::call(c, args, p),
        c if c.starts_with("tt_") => timetable::call(c, args, p),
        c if c.starts_with("de_") => depot::call(c, args, p),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// A command that answers bytes (a picture, a file).
pub fn call_bytes(cmd: &str, args: &Value, p: &dyn Platform) -> Result<Vec<u8>, String> {
    match cmd {
        c if c.starts_with("hub_") => hub::call_bytes(c, args, p),
        c if c.starts_with("co_") => company::call_bytes(c, args, p),
        c if c.starts_with("rf_") => rosterflow::call_bytes(c, args, p),
        c if c.starts_with("de_") => depot::call_bytes(c, args, p),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn commands_go_by_their_prefix() {
        assert_eq!(call("nope", &json!({}), &Quiet), Err("unknown command: nope".into()));
        for cmd in ["tg_host", "tt_open", "de_open"] {
            assert_eq!(call(cmd, &json!({}), &Quiet), Err(not_ported(cmd)), "{cmd}");
        }
        // (the company and RosterFlow are ported: a command they do not know is said so)
        assert_eq!(call("co_nope", &json!({}), &Quiet), Err("unknown command: co_nope".into()));
        assert_eq!(call("rf_nope", &json!({}), &Quiet), Err("unknown command: rf_nope".into()));
        assert_eq!(call_bytes("co_portrait", &json!({ "n": 0 }), &Quiet), Err("no portrait 0".into()));
        assert!(call_bytes("co_portrait", &json!({ "n": 1 }), &Quiet).is_ok_and(|b| b.starts_with(&[0xFF, 0xD8])));
        assert_eq!(call_bytes("xx", &json!({}), &Quiet), Err("unknown command: xx".into()));
        let v = call("hub_version", &json!({}), &Quiet).unwrap();
        assert_eq!(v["app"], env!("CARGO_PKG_VERSION"));
        assert_eq!(v["min_openomsi"], "0.2.22");
    }

    #[test]
    fn arguments_are_read_by_name() {
        let a = json!({"n": 3, "s": "x"});
        assert_eq!(arg::<i64>(&a, "n"), Ok(3));
        assert_eq!(arg::<Option<String>>(&a, "missing"), Ok(None));
        assert!(arg::<i64>(&a, "s").unwrap_err().starts_with("s: "));
    }
}
