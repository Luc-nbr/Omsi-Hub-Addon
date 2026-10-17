//! The drive's steps (`hub_`): what the page's start and steps read of the player's OMSI 2 -
//! the maps, the buses, the weather, the day's lines, the depot files, the shifts, the bus's
//! options and display fonts, the route map. Routed here by [`crate::hub`] for every `hub_`
//! command it does not answer itself.

use crate::{Answer, Platform};
use serde_json::Value;

/// A `hub_` command of the drive's steps.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(format!("unknown command: {cmd}"))
}

/// A `hub_` command of the drive's steps that answers bytes.
pub fn call_bytes(cmd: &str, _args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    Err(format!("unknown command: {cmd}"))
}
