//! The depot editor (the old `omsihub/editors/*`, its depot part), on `hub_omsi::hof`: **the
//! target of the editors port.** Its commands start with `de_`; until the port lands each
//! answers [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// A `de_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}

/// A `de_` command that answers bytes (a display's preview).
pub fn call_bytes(cmd: &str, _args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    Err(not_ported(cmd))
}
