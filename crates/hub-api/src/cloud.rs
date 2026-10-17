//! The cloud saves (the old `omsihub/cloud.rs`), on `hub_online::saves`: **the target of the
//! online port.** Its commands start with `cs_`; until the port lands each answers
//! [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// A `cs_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}
