//! The timetable editor (the old `omsihub/editors/*`, its timetable part), on
//! `hub_omsi::timetable`'s readers and writers: **the target of the editors port.** Its
//! commands start with `tt_`; until the port lands each answers [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// A `tt_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}
