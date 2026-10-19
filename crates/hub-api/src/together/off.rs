//! "Play together" where it cannot be: the website, the company opened in the browser from the
//! cloud (`hub-api` without "native"). Nobody hosts it and nobody joins: the pages show it as not
//! shared, and its commands answer nothing.

use crate::company::words::{tr, trf};
use crate::company::Session;
use crate::launcher::company::duty_launch::DutyRef;
use crate::launcher::company::plan::DayPlan;
use crate::launcher::company::Company;
use crate::{Answer, Platform};
use serde_json::{json, Value};

pub fn session_code() -> Option<String> {
    None
}

pub(super) fn remember_language() {}

pub fn online_guests(_: &Company) -> Vec<(u32, String, Option<DutyRef>)> {
    Vec::new()
}

pub fn rail_badge(_: &Company) -> (Value, String) {
    (Value::Null, tr("Multiplayer"))
}

pub fn tile_line(c: &Company) -> (String, &'static str) {
    match c.together.guests.len() {
        0 => (tr("Not shared"), "soft"),
        n => (trf("Not shared · %{n} players", &[("n", &n.to_string())]), "soft"),
    }
}

pub fn give_to_guest(_: &mut Session, _: &DayPlan, _: usize, _: usize, _: u32) -> Value {
    json!({})
}

pub fn closing_company(_: &mut Session, _: Option<&str>) {}

pub fn quit() {}

/// The host's commands and the joiner's (`tg_…`): none here.
pub(super) fn call(_: &str, _: &Value) -> Option<Answer> {
    None
}

pub(super) mod join {
    use super::*;

    pub(in crate::together) fn call(_: &str, _: &Value, _: &dyn Platform) -> Option<Answer> {
        None
    }
}
