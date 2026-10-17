//! Playing together (the old `omsihub/together*`): **later**. It needs a relay of our own, or
//! LAN only (INVENTORY §2). Its commands start with `tg_`; each answers
//! [`crate::not_ported`].
//!
//! What the company's and RosterFlow's pages ask of it is here already, as it is while "Play
//! together" is off (the old launcher's kill switch): [`enabled`] says no, nobody is online, no
//! duty goes to a player. The company's own side of it (`hub_core::company::together`: the
//! players, their roles, the duties they claimed) is ported and kept in the saves.

use crate::company::words::{tr, trf};
use crate::company::Session;
use crate::{not_ported, Answer, Platform};
use hub_core::company::duty_launch::DutyRef;
use hub_core::company::plan::DayPlan;
use hub_core::company::Company;
use serde_json::{json, Value};

/// A `tg_` command.
pub fn call(cmd: &str, _args: &Value, _p: &dyn Platform) -> Answer {
    Err(not_ported(cmd))
}

/// The colour of a player's duties on the planning's pages.
pub const GUEST_COLOUR: &str = "#e58fd8";

/// "Play together" is on: not yet - the way in for friends needs a relay of Omsi-Hub's own.
pub fn enabled() -> bool {
    false
}

/// What the gate says while it is closed (English: a key of the translations).
pub const BLOCKED: &str = "Multiplayer - coming soon (beta)";

/// The gate's answer to a command: whether it is open, and what is said while it is not.
pub fn gate() -> Value {
    json!({ "on": enabled(), "label": tr(BLOCKED) })
}

/// The AI driving a duty a player left.
pub fn ai_label(name: &str) -> String {
    trf("AI (%{name} left)", &[("name", name)])
}

/// A player of "Play together" as the company's pages name him on his duties: his name, or "AI
/// (his name left)" while the AI drives a duty he left (`book::guest_left`).
pub fn guest_label(c: &Company, g: u32) -> String {
    let name = hub_core::company::together::guest_name(c, g);
    if hub_core::company::together::book::guest_left(c, g) {
        ai_label(&name)
    } else {
        name
    }
}

/// The players online in the company's session, with the duty each drives: none (no session).
pub fn online_guests(_c: &Company) -> Vec<(u32, String, Option<DutyRef>)> {
    Vec::new()
}

/// The Multiplayer page's badge in the strip, and its tip: the company is not shared.
pub fn rail_badge(_c: &Company) -> (Value, String) {
    (Value::Null, format!("{}\n{}", tr("Multiplayer"), tr("Not shared: start sharing to let friends join")))
}

/// The Multiplayer tile on the start screen.
pub fn tile_line(c: &Company) -> (String, &'static str) {
    if c.together.guests.is_empty() {
        (tr("Not shared"), "soft")
    } else {
        (trf("Not shared · %{n} players", &[("n", &c.together.guests.len().to_string())]), "soft")
    }
}

/// A duty given to a player of the session: there is no session.
pub fn give_to_guest(_s: &mut Session, _p: &DayPlan, _ti: usize, _k: usize, _guest: u32) -> Value {
    json!({})
}

/// The company `s` has open closes (or another opens, `to`): a session of it would end here.
pub fn closing_company(_s: &mut Session, _to: Option<&str>) {}

/// The company's pages for a player whose role allows them: what each needs (the old
/// `together/relay.rs`, the parts the company's pages use).
pub mod relay {
    use crate::company::{CONCESSIONS_TAB, CONTRACTS_TAB, DEPOT_TAB, INBOX_TAB, MAP_TAB, OPS_TAB, REPUTATION_TAB, RESEARCH_TAB, SANDBOX_TAB, WORKSHOP_TAB};
    use hub_core::company::together::Perm;
    use serde_json::{json, Value};

    /// Why a relayed command is refused (English: keys of the translations).
    pub const FROM_AFAR: &str = "Only the boss can do this, on his own launcher.";

    /// What a command needs of a player.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Need {
        /// Any player of the company.
        Member,
        /// One of these permissions.
        Any(&'static [Perm]),
        /// Never from afar.
        Never,
    }

    use Need::{Any, Member, Never};

    const FINANCES: Need = Any(&[Perm::Finances]);
    const SPEND: Need = Any(&[Perm::Spend]);
    const STAFF: Need = Any(&[Perm::Staff]);
    const LINES: Need = Any(&[Perm::Lines]);
    const PLANS: Need = Any(&[Perm::Plans]);
    const SERVICE: Need = Any(&[Perm::Service]);
    const SETTINGS: Need = Any(&[Perm::Settings]);
    /// The fleet's page: who buys, plans or runs the day's service.
    const FLEET: Need = Any(&[Perm::Spend, Perm::Plans, Perm::Service]);
    /// RosterFlow and the planning seen: who plans, runs the lines or the day's service, or
    /// gives and approves the players' duties.
    pub const RF_VIEW: Need = Any(&[Perm::Plans, Perm::Lines, Perm::Service, Perm::Assign, Perm::Approve]);

    impl Need {
        /// Whether `perms` meet it.
        pub fn met(self, perms: &[Perm]) -> bool {
            match self {
                Member => true,
                Any(ps) => ps.iter().any(|p| perms.contains(p)),
                Never => false,
            }
        }
    }

    /// What a page of the company needs.
    pub fn tab_need(tab: usize) -> Need {
        match tab {
            // Overview, Finances, Reputation: the money
            0 | 4 | REPUTATION_TAB => FINANCES,
            1 => FLEET,
            2 => STAFF,
            3 => LINES,
            // (the planning - RosterFlow with its add-on on - seen read-only without PLANS)
            5 => RF_VIEW,
            14 => PLANS,
            // the boss's own career
            6 => Never,
            DEPOT_TAB | CONCESSIONS_TAB | RESEARCH_TAB | WORKSHOP_TAB | CONTRACTS_TAB => SPEND,
            MAP_TAB => Member,
            INBOX_TAB | SANDBOX_TAB => SETTINGS,
            OPS_TAB => SERVICE,
            _ => Never,
        }
    }

    /// The company's shell (`company::shell`) as a player with `perms` sees it: the pages he may
    /// see, the money only when he may see it, no clock to run and nothing of the boss's own.
    pub fn shell_for(mut sh: Value, perms: &[Perm]) -> Value {
        let may = |tab: usize| tab_need(tab).met(perms);
        if let Some(rail) = sh.get_mut("rail").and_then(Value::as_array_mut) {
            for g in rail.iter_mut() {
                if let Some(items) = g.as_array_mut() {
                    items.retain(|it| it.get("tab").and_then(Value::as_u64).is_some_and(|t| may(t as usize)));
                }
            }
            rail.retain(|g| g.as_array().is_some_and(|a| !a.is_empty()));
        }
        if let Some(pages) = sh.get_mut("pages").and_then(Value::as_array_mut) {
            pages.retain(|t| t.as_u64().is_some_and(|t| may(t as usize)));
        }
        if let Some(status) = sh.get_mut("status").and_then(Value::as_array_mut) {
            for it in status.iter_mut() {
                let tab = it.get("tab").and_then(Value::as_u64);
                if tab.is_some_and(|t| !may(t as usize)) {
                    it["tab"] = Value::Null;
                }
            }
            if !Need::met(FINANCES, perms) {
                status.retain(|it| it.get("icon").and_then(Value::as_str) != Some("payments"));
            }
        }
        if let Some(k) = sh.get_mut("clock") {
            k["speed"] = json!(0.0);
            k["tip"] = Value::Null;
        }
        for key in ["reports", "away", "launch", "asking"] {
            sh[key] = json!(false);
        }
        sh["casual_pages"] = Value::Null;
        sh["bell"] = json!(0);
        sh["together"] = json!({ "on": false });
        sh["remote"] = json!({ "perms": perms.iter().map(|p| p.key()).collect::<Vec<_>>() });
        sh
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_player_sees_the_pages_his_role_allows() {
            assert!(RF_VIEW.met(&[Perm::Plans]) && !RF_VIEW.met(&[Perm::Drive]));
            assert!(!tab_need(6).met(&[Perm::Plans, Perm::Finances]), "the boss's career is his own");
            let sh = json!({ "pages": [0, 1, 5, 6], "rail": [[{ "tab": 0 }, { "tab": 5 }], [{ "tab": 6 }]], "status": [], "clock": { "speed": 2.0 } });
            let out = shell_for(sh, &[Perm::Plans]);
            assert_eq!(out["pages"], json!([1, 5]));
            assert_eq!(out["rail"], json!([[{ "tab": 5 }]]));
            assert_eq!(out["clock"]["speed"], json!(0.0));
        }
    }
}
