//! Playing together (the old `omsihub/together*`): **later**. It needs a relay of our own, or
//! LAN only (INVENTORY §2). Its commands start with `tg_`; each answers
//! [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// A `tg_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}
