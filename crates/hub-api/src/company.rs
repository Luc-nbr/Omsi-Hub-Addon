//! The bus company (the old `omsihub/company*` and `company_more/*`): **the target of the
//! company port.** Its data and logic are `hub_core::company`; its commands start with `co_`.
//! Until the port lands each answers [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// A `co_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}

/// A `co_` command that answers bytes (staff portraits, the fleet map).
pub fn call_bytes(cmd: &str, _args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    Err(not_ported(cmd))
}
