//! RosterFlow (the old `omsihub/rosterflow*`): **the target of the RosterFlow port**, on
//! `hub_core::company` and `hub_omsi` (timetable, `global.cfg`, `ailists.cfg`). Its commands
//! start with `rf_`; until the port lands each answers [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// An `rf_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}

/// An `rf_` command that answers bytes (printouts).
pub fn call_bytes(cmd: &str, _args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    Err(not_ported(cmd))
}
