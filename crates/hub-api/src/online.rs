//! The account, the friends and the leaderboard (the old `omsihub/online.rs`), on `hub-online`:
//! **the target of the online port.** Its commands start with `ol_`; until the port lands each
//! answers [`crate::not_ported`].

use crate::{not_ported, Answer, Platform};
use serde_json::Value;

/// An `ol_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}
