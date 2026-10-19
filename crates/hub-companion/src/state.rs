//! The state a device is shown (docs/COMPANION.md, "State"): what the plugin told of the game
//! ([`Heard`]) and what only the add-on knows ([`Facts`]: the assignment it gave, the control
//! room's inbox), put together. Pure functions of their inputs, so that the tests can hold the
//! JSON the app reads against what goes in.

use crate::heard::Heard;
use crate::nav::Next;
use hub_game::bridge::{Assignment, Inbox};
use serde_json::{json, Map, Value};

/// The game counts as there when it was heard within this many seconds.
pub const GAME_FRESH: f64 = 5.0;
/// A wait between two trips counts as a break from this many seconds before the next one
/// leaves.
const BREAK_MIN: f64 = 120.0;

/// What the add-on knows beside the game.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// The assignment the plugin was last given.
    pub assignment: Option<Assignment>,
    /// The control room's messages to the duty (`data/messages.json`).
    pub inbox: Option<Inbox>,
    /// The company's currency sign ("€", "£"; empty: "€").
    pub currency: String,
    /// The next turn ([`crate::nav`]), the street after it where the game named it, and the
    /// route's version.
    pub nav: Option<Next>,
    pub street: String,
    pub route_v: u64,
    /// The break between trips ([`Pause::look`]).
    pub pause: Option<Value>,
}

fn f(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn s(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// The live datagram's duty (`omsi.duty.get` as the plugin sends it).
fn duty_of(h: &Heard) -> Option<&Map<String, Value>> {
    h.get("duty").and_then(Value::as_object)
}

/// The assignment, when it is the one the game drives (or the game is not heard: the last one
/// given, for the duty card before the drive).
fn assignment_of<'a>(h: &Heard, facts: &'a Facts, game: bool) -> Option<&'a Assignment> {
    let a = facts.assignment.as_ref()?;
    let issued = f(h.get("issued")).unwrap_or(0.0) as u64;
    (!game || issued == 0 || issued == a.issued).then_some(a)
}

/// The whole state; `v` is set by the server.
pub fn build(h: &Heard, facts: &Facts, now: f64) -> Value {
    let game = h.fresh(now, GAME_FRESH);
    let a = assignment_of(h, facts, game);
    let duty_live = if game { duty_of(h) } else { None };
    let clock = if game { f(h.get("clock")) } else { None };
    json!({
        "game": game,
        "clock": clock,
        "paused": game && h.get("paused").and_then(Value::as_bool).unwrap_or(false),
        "signon": signon(h, a, game),
        "duty": duty(h, a, duty_live),
        "trip": if game { trip(h, duty_live) } else { Value::Null },
        "bus": if game { bus(h) } else { Value::Null },
        "nav": if game { nav(facts, duty_live) } else { Value::Null },
        "pause": if game { facts.pause.clone().unwrap_or(Value::Null) } else { Value::Null },
        "messages": messages(facts.inbox.as_ref()),
        "ibis": if game { ibis(h, duty_live) } else { Value::Null },
        "printer": if game { printer(h, &facts.currency) } else { Value::Null },
        "features": {
            "ibis": game && !keys(h, "ibis").is_empty(),
            "printer": game && h.keys.get("tickets").and_then(Value::as_array).is_some_and(|t| !t.is_empty()),
            "nav": facts.nav.is_some(),
            "messages": facts.inbox.is_some(),
            "signon": a.is_some_and(|a| a.personnel.is_some()),
        },
    })
}

fn signon(h: &Heard, a: Option<&Assignment>, game: bool) -> Value {
    let pass = a.and_then(|a| a.personnel.as_ref());
    let stage = match (game.then(|| h.get("signon").and_then(Value::as_str)).flatten(), pass) {
        (_, None) => "none",
        (Some(st @ ("signon" | "order" | "work")), Some(_)) => st,
        // (not in the game yet: the pass waits for it)
        (_, Some(_)) => "signon",
    };
    let hint = pass.map(|p| format!("{}…", p.number.chars().take(2).collect::<String>()));
    let wrong = game && stage == "signon" && h.get("signon_wrong").and_then(Value::as_bool).unwrap_or(false);
    json!({ "stage": stage, "number_hint": hint, "error": if wrong { json!("wrong") } else { Value::Null } })
}

/// The duty's start, end and depot from its strip (a company's duty), in seconds of the day.
fn strip_times(a: &Assignment) -> (Option<f64>, Option<f64>, String) {
    let rows = a.crew.as_ref().map(|c| c.rows.as_slice()).unwrap_or_default();
    let on = rows.iter().find(|r| r.kind == "sign_on");
    let off = rows.iter().rev().find(|r| r.kind == "sign_off");
    let start = on.map(|r| r.from as f64 * 60.0).or_else(|| rows.first().map(|r| r.from as f64 * 60.0));
    let end = off.map(|r| r.to.max(r.from) as f64 * 60.0).or_else(|| rows.last().map(|r| r.to as f64 * 60.0));
    (start, end, on.map(|r| r.at.clone()).unwrap_or_default())
}

fn duty(h: &Heard, a: Option<&Assignment>, live: Option<&Map<String, Value>>) -> Value {
    if a.is_none() && live.is_none() {
        return Value::Null;
    }
    let current = live.and_then(|d| f(d.get("trip"))).unwrap_or(0.0) as i64;
    let done = live.and_then(|d| d.get("trip_done")).and_then(Value::as_bool).unwrap_or(false);
    let trips: Vec<Value> = h
        .trips
        .items
        .iter()
        .enumerate()
        .map(|(k, t)| {
            let n = k as i64 + 1;
            let state = if live.is_none() {
                "later"
            } else if n < current || (n == current && done) {
                "done"
            } else if n == current {
                "now"
            } else if n == current + 1 || (current == 0 && n == 1) {
                "next"
            } else {
                "later"
            };
            json!({
                "n": n,
                "line": s(t.get("line")),
                "destination": s(t.get("terminus")),
                "departure": f(t.get("departure")),
                "arrival": f(t.get("arrival")),
                "state": state,
            })
        })
        .collect();
    let line = live.map(|d| s(d.get("line"))).filter(|l| !l.is_empty()).or_else(|| a.map(|a| a.line.clone())).unwrap_or_default();
    let tour = live.map(|d| s(d.get("tour"))).filter(|l| !l.is_empty()).or_else(|| a.map(|a| a.tour.clone())).unwrap_or_default();
    let kind = match a {
        Some(a) if !a.company.is_empty() => "company",
        Some(a) if !a.legs.is_empty() => "shift",
        Some(a) if !a.free_line.is_empty() => "free",
        Some(_) => "tour",
        None => "game",
    };
    let (mut start, mut end, depot) = a.map(strip_times).unwrap_or((None, None, String::new()));
    let first = |k: &str, t: &Value| f(t.get(k));
    start = start.or_else(|| h.trips.items.first().and_then(|t| first("departure", t)));
    end = end.or_else(|| h.trips.items.last().and_then(|t| first("arrival", t)));
    let name = match a {
        Some(a) if !a.duty.is_empty() => format!("Duty {}", a.duty),
        _ if !line.is_empty() => format!("{line}/{tour}"),
        _ => String::new(),
    };
    json!({
        "kind": kind,
        "name": name,
        "company": a.map(|a| a.company.clone()).unwrap_or_default(),
        "line": line, "tour": tour,
        "start": start, "end": end,
        "depot": depot,
        "trips": trips,
        "started": live.is_some(),
    })
}

fn trip(h: &Heard, live: Option<&Map<String, Value>>) -> Value {
    let Some(d) = live else { return Value::Null };
    let delay = f(d.get("delay")).unwrap_or(0.0);
    let next = f(d.get("next")).map(|n| n.max(0.0) as usize).unwrap_or(0);
    let stops: Vec<Value> = h
        .stops
        .items
        .iter()
        .enumerate()
        .map(|(k, st)| {
            let passed = st.get("passed").and_then(Value::as_bool).unwrap_or(k < next);
            // (a stop passed keeps the delay the bus had there; one to come, the delay now)
            let delay = if passed { f(st.get("delay")) } else { Some(delay) };
            json!({
                "name": s(st.get("name")),
                "arrival": f(st.get("arrival")),
                "departure": f(st.get("departure")),
                "delay": delay,
                "passed": passed,
                "calls": st.get("calls").and_then(Value::as_bool).unwrap_or(true),
                "x": f(st.get("x")), "y": f(st.get("y")),
            })
        })
        .collect();
    let line = Some(s(d.get("number"))).filter(|n| !n.is_empty()).unwrap_or_else(|| s(d.get("line")));
    json!({
        "n": f(d.get("trip")).map(|n| n as i64),
        "of": f(d.get("trips")).map(|n| n as i64),
        "line": line,
        "destination": s(d.get("terminus")),
        "departure": f(d.get("departure")),
        "arrival": f(d.get("arrival")),
        "delay": delay,
        "next": next.min(stops.len().saturating_sub(1)),
        "at_stop": d.get("at_stop").and_then(Value::as_bool).unwrap_or(false),
        "stops": stops,
    })
}

fn bus(h: &Heard) -> Value {
    let Some(b) = h.get("bus").and_then(Value::as_object) else { return Value::Null };
    json!({
        "name": s(b.get("name")),
        "number": s(b.get("number")),
        "x": f(b.get("x")), "y": f(b.get("y")),
        "heading": f(b.get("heading")),
        "speed": f(b.get("speed")).unwrap_or(0.0),
        "doors_open": b.get("doors_open").and_then(Value::as_bool),
        "stop_requested": b.get("stop_requested").and_then(Value::as_bool),
        "passengers": f(b.get("passengers")).map(|p| p as i64),
    })
}

fn nav(facts: &Facts, live: Option<&Map<String, Value>>) -> Value {
    let Some(n) = facts.nav else { return Value::Null };
    let toward = live.map(|d| s(d.get("next_name"))).filter(|t| !t.is_empty()).or_else(|| live.map(|d| s(d.get("terminus")))).unwrap_or_default();
    json!({
        "maneuver": n.maneuver.name(),
        "distance": (n.distance / 10.0).round() * 10.0,
        "street": facts.street,
        "toward": toward,
        "route_v": facts.route_v,
    })
}

/// The inbox as the app shows it: the buttons of each message that waits for an answer.
fn messages(inbox: Option<&Inbox>) -> Value {
    let Some(inbox) = inbox else { return json!({ "unread": 0, "items": [], "quick": [] }) };
    let word = |k: &str| inbox.words.get(k).cloned().unwrap_or_else(|| k.to_string());
    let choice = |c: &hub_game::bridge::Choice| json!({ "key": c.key, "label": c.label });
    let items: Vec<Value> = inbox
        .messages
        .iter()
        .map(|m| {
            let open = !m.mine && m.state != "accepted" && m.state != "declined";
            let answers: Vec<Value> = if !open {
                Vec::new()
            } else if m.action.is_some() {
                vec![json!({ "key": "accept", "label": word("Accept") }), json!({ "key": "decline", "label": word("Decline") })]
            } else if m.reply.is_empty() && m.state != "read" {
                std::iter::once(json!({ "key": "read", "label": word("Acknowledge") })).chain(inbox.replies.iter().map(choice)).collect()
            } else {
                Vec::new()
            };
            json!({
                "id": m.id, "time": m.time, "from": m.from, "priority": m.priority,
                "text": m.text, "state": m.state, "state_label": m.state_label,
                "mine": m.mine, "answers": answers, "reply": m.reply,
                "action": m.action.as_ref().map(|a| a.label.clone()),
            })
        })
        .collect();
    let unread = inbox.messages.iter().filter(|m| !m.mine && (m.state == "sent" || m.state == "delivered")).count();
    json!({ "unread": unread, "items": items, "quick": inbox.quick.iter().map(choice).collect::<Vec<_>>() })
}

/// The bus's keys of one device, as the plugin sent them.
fn keys(h: &Heard, which: &str) -> Vec<String> {
    h.keys.get(which).and_then(Value::as_array).map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// Every key a device may press now (the plugin refuses the rest too).
pub fn allowed_keys(h: &Heard) -> Vec<String> {
    ["ibis", "printer", "change"].iter().flat_map(|w| keys(h, w)).collect()
}

fn ibis(h: &Heard, live: Option<&Map<String, Value>>) -> Value {
    let keys = keys(h, "ibis");
    let vars = h.dev.get("ibis").and_then(Value::as_object);
    if keys.is_empty() && vars.is_none_or(|v| v.is_empty()) {
        return Value::Null;
    }
    let empty = Map::new();
    let vars = vars.unwrap_or(&empty);
    let strs = h.dev.get("strs").and_then(Value::as_object).unwrap_or(&empty);
    let num = |names: &[&str]| names.iter().find_map(|n| f(vars.get(*n))).map(|v| v as i64);
    let stock = (0..10).all(|d| keys.iter().any(|k| *k == format!("IBIS_{d}"))) && keys.iter().any(|k| k == "IBIS_eingabe");
    let terminus = ["IBIS_terminus_name", "IBIS_Terminus_Name"].iter().map(|n| s(strs.get(*n))).find(|t| !t.trim().is_empty()).unwrap_or_default();
    // (what is typed and what the display says, where the bus keeps them in strings)
    let named = |words: &[&str]| -> Vec<String> {
        let mut found: Vec<(&String, String)> = strs.iter().filter(|(k, _)| words.iter().any(|w| k.to_lowercase().contains(w))).map(|(k, v)| (k, s(Some(v)))).collect();
        found.sort_by(|a, b| a.0.cmp(b.0));
        found.into_iter().map(|x| x.1).filter(|t| !t.trim().is_empty()).collect()
    };
    let entry = named(&["eingabe", "input", "entry"]).into_iter().next().unwrap_or_default();
    let display = named(&["display", "anzeige", "zeile"]);
    let stop_index = num(&["IBIS_busstop", "ibox_busstop"]);
    let stop = live.and_then(|d| d.get("next_name")).map(|v| s(Some(v))).unwrap_or_default();
    json!({
        "kind": if stock { "stock" } else { "generic" },
        "line": num(&["IBIS_LinieKurs", "IBIS_Linie"]),
        "suffix": num(&["IBIS_Linie_Suffix"]),
        "kurs": num(&["IBIS_Kurs", "IBIS_kurs"]),
        "route": num(&["IBIS_Route", "IBIS_RouteIndex", "ibox_routenindex"]),
        "ziel": num(&["IBIS_TerminusCode"]),
        "terminus": terminus,
        "stop": stop,
        "stop_index": stop_index,
        "mode": num(&["IBIS_mode"]),
        "entry": entry,
        "display": display,
        "keys": keys,
    })
}

fn printer(h: &Heard, currency: &str) -> Value {
    let tickets = h.keys.get("tickets").and_then(Value::as_array).cloned().unwrap_or_default();
    let keys = keys(h, "printer");
    if tickets.is_empty() && keys.is_empty() {
        return Value::Null;
    }
    let request = h.dev.get("request").filter(|r| r.is_object()).cloned().unwrap_or(Value::Null);
    let sales = h.dev.get("sales").filter(|r| r.is_object()).cloned().unwrap_or(json!({ "count": 0, "money": 0 }));
    json!({
        "tickets": tickets,
        "request": request,
        "sales": sales,
        "currency": if currency.is_empty() { "€" } else { currency },
        "keys": keys,
        "change_keys": self::keys(h, "change"),
    })
}

/// The break between two trips: found when the bus waits at a terminus for a trip that leaves
/// in more than two minutes, kept from the moment it was found until the trip leaves - or the
/// driver ends it on the phone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pause {
    /// The departure the break waits for, and the game's clock when it began.
    pub waiting: Option<(f64, f64)>,
    /// A break the driver ended: its departure.
    pub ended: Option<f64>,
}

impl Pause {
    /// The break now, as the state shows it (None: none).
    pub fn look(&mut self, h: &Heard) -> Option<Value> {
        let d = duty_of(h)?;
        let clock = f(h.get("clock"))?;
        let n = f(d.get("trip")).unwrap_or(0.0) as usize;
        let trips = &h.trips.items;
        let done = d.get("trip_done").and_then(Value::as_bool).unwrap_or(false);
        // (the trip done: the next one; not left yet, standing at its first stop: this one)
        let (next, before) = if done {
            (trips.get(n), n.checked_sub(1).and_then(|k| trips.get(k)))
        } else if f(d.get("next")).unwrap_or(0.0) < 1.0 && d.get("at_stop").and_then(Value::as_bool).unwrap_or(false) {
            (n.checked_sub(1).and_then(|k| trips.get(k)), n.checked_sub(2).and_then(|k| trips.get(k)))
        } else {
            (None, None)
        };
        let departure = next.and_then(|t| f(t.get("departure")));
        let Some(dep) = departure.filter(|dep| dep - clock > BREAK_MIN || self.waiting.is_some_and(|w| w.0 == *dep && *dep > clock)) else {
            self.waiting = None;
            return None;
        };
        if self.ended == Some(dep) {
            return None;
        }
        let since = match self.waiting {
            Some((w, since)) if w == dep => since,
            _ => {
                self.waiting = Some((dep, clock));
                clock
            }
        };
        let place = before.map(|t| s(t.get("terminus"))).filter(|p| !p.is_empty()).or_else(|| h.stops.items.first().map(|st| s(st.get("name")))).unwrap_or_default();
        Some(json!({
            "active": true,
            "since": since, "until": dep,
            "place": place,
            "next": { "departure": dep, "line": next.map(|t| s(t.get("line"))).unwrap_or_default(), "destination": next.map(|t| s(t.get("terminus"))).unwrap_or_default() },
            "auto": true,
        }))
    }

    /// The driver ends the break now.
    pub fn end(&mut self) {
        if let Some((dep, _)) = self.waiting.take() {
            self.ended = Some(dep);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_game::bridge::{Crew, InboxAction, InboxMsg, Personnel, StripRow};

    fn heard(live: Value, now: f64) -> Heard {
        let mut h = Heard::default();
        let mut d = live;
        d["t"] = json!("live");
        h.hear(d.to_string().as_bytes(), now);
        h
    }

    fn add(h: &mut Heard, d: Value) {
        h.hear(d.to_string().as_bytes(), 0.0);
    }

    fn driving() -> Heard {
        let mut h = heard(
            json!({
                "issued": 100, "clock": 30_000.0, "signon": "work",
                "bus": { "name": "MAN NL202", "number": "2604", "x": 10.0, "y": 20.0, "heading": 90.0, "speed": 31.5, "passengers": 12 },
                "duty": { "line": "29", "tour": "3", "trip": 2, "trips": 3, "number": "M29", "terminus": "Markgraf-Berthold-Platz",
                          "departure": 29_900.0, "arrival": 31_000.0, "delay": 120.0, "next": 1, "at_stop": false, "trip_done": false,
                          "next_name": "Kunsthalle" },
            }),
            0.0,
        );
        add(
            &mut h,
            json!({ "t": "trips", "v": 1, "trips": [
            { "line": "M29", "terminus": "Hauptbahnhof", "departure": 28_000.0, "arrival": 29_000.0 },
            { "line": "M29", "terminus": "Markgraf-Berthold-Platz", "departure": 29_900.0, "arrival": 31_000.0 },
            { "line": "M29", "terminus": "Hauptbahnhof", "departure": 32_000.0, "arrival": 33_000.0 },
        ] }),
        );
        add(
            &mut h,
            json!({ "t": "trip", "v": 1, "stops": [
            { "name": "Hauptbahnhof", "arrival": 29_900.0, "departure": 29_900.0, "passed": true, "delay": 60.0, "x": 1.0, "y": 2.0 },
            { "name": "Kunsthalle", "arrival": 30_100.0, "departure": 30_120.0 },
            { "name": "Rathaus", "arrival": 30_400.0, "departure": 30_400.0, "calls": false },
        ] }),
        );
        h
    }

    fn company() -> Assignment {
        Assignment {
            issued: 100,
            company: "Stadtbus".into(),
            duty: "14".into(),
            line: "29".into(),
            tour: "3".into(),
            personnel: Some(Personnel { number: "482913".into(), code: "5821".into() }),
            crew: Some(Crew {
                rows: vec![
                    StripRow { kind: "sign_on".into(), at: "Bismarckplatz".into(), from: 462, to: 467, ..StripRow::default() },
                    StripRow { kind: "trip".into(), from: 467, to: 520, ..StripRow::default() },
                    StripRow { kind: "sign_off".into(), at: "Bismarckplatz".into(), from: 860, to: 870, ..StripRow::default() },
                ],
                ..Crew::default()
            }),
            ..Assignment::default()
        }
    }

    #[test]
    fn the_duty_its_trip_and_the_bus_as_the_game_has_them() {
        let h = driving();
        let facts = Facts { assignment: Some(company()), ..Facts::default() };
        let v = build(&h, &facts, 1.0);
        assert_eq!(v["game"], true);
        assert_eq!(v["clock"], 30_000.0);
        let d = &v["duty"];
        assert_eq!((d["kind"].as_str(), d["name"].as_str(), d["depot"].as_str()), (Some("company"), Some("Duty 14"), Some("Bismarckplatz")));
        assert_eq!((d["start"].as_f64(), d["end"].as_f64()), (Some(462.0 * 60.0), Some(870.0 * 60.0)));
        let states: Vec<&str> = d["trips"].as_array().unwrap().iter().map(|t| t["state"].as_str().unwrap()).collect();
        assert_eq!(states, ["done", "now", "next"]);
        let t = &v["trip"];
        assert_eq!((t["n"].as_i64(), t["of"].as_i64(), t["line"].as_str(), t["next"].as_u64()), (Some(2), Some(3), Some("M29"), Some(1)));
        let stops = t["stops"].as_array().unwrap();
        assert_eq!((stops[0]["delay"].as_f64(), stops[1]["delay"].as_f64()), (Some(60.0), Some(120.0)), "passed: the delay there; to come: now");
        assert_eq!((stops[1]["passed"].as_bool(), stops[2]["calls"].as_bool()), (Some(false), Some(false)));
        assert_eq!((v["bus"]["number"].as_str(), v["bus"]["passengers"].as_i64()), (Some("2604"), Some(12)));
        assert_eq!(v["signon"]["stage"], "work");
        assert_eq!(v["signon"]["number_hint"], "48…");
        assert!(!v.to_string().contains("482913") && !v.to_string().contains("5821"), "the pass is never shown");
    }

    #[test]
    fn without_the_game_the_last_assignment_is_the_duty_card_and_sign_on_waits() {
        let h = driving();
        let v = build(&h, &Facts { assignment: Some(company()), ..Facts::default() }, 60.0);
        assert_eq!(v["game"], false);
        assert_eq!(v["trip"], Value::Null);
        assert_eq!(v["duty"]["name"], "Duty 14");
        assert_eq!(v["duty"]["started"], false);
        assert_eq!(v["signon"]["stage"], "signon");
        // no assignment, no game: nothing
        assert_eq!(build(&Heard::default(), &Facts::default(), 0.0)["duty"], Value::Null);
        // a drive of the player's own with no pass: no sign-on
        let own = Assignment { issued: 100, line: "29".into(), tour: "3".into(), ..Assignment::default() };
        let v = build(&driving(), &Facts { assignment: Some(own), ..Facts::default() }, 1.0);
        assert_eq!((v["signon"]["stage"].as_str(), v["duty"]["kind"].as_str(), v["duty"]["name"].as_str()), (Some("none"), Some("tour"), Some("29/3")));
    }

    #[test]
    fn the_inbox_gives_each_open_message_its_buttons() {
        let inbox = Inbox {
            messages: vec![
                InboxMsg { id: 1, text: "Diversion".into(), priority: "urgent".into(), state: "delivered".into(), ..InboxMsg::default() },
                InboxMsg { id: 2, text: "Turn back".into(), state: "delivered".into(), action: Some(InboxAction { kind: "short_turn".into(), ..InboxAction::default() }), ..InboxMsg::default() },
                InboxMsg { id: 3, text: "Running late".into(), mine: true, state: "sent".into(), ..InboxMsg::default() },
                InboxMsg { id: 4, text: "Old".into(), state: "accepted".into(), ..InboxMsg::default() },
            ],
            quick: vec![hub_game::bridge::Choice { key: "late".into(), label: "Running late".into(), icon: String::new() }],
            words: [("Acknowledge".to_string(), "Gelesen".to_string())].into(),
            ..Inbox::default()
        };
        let v = build(&Heard::default(), &Facts { inbox: Some(inbox), ..Facts::default() }, 0.0);
        let m = &v["messages"];
        assert_eq!(m["unread"], 2);
        let keys = |i: usize| -> Vec<String> { m["items"][i]["answers"].as_array().unwrap().iter().map(|a| a["key"].as_str().unwrap().to_string()).collect() };
        assert_eq!(keys(0), ["read"]);
        assert_eq!(m["items"][0]["answers"][0]["label"], "Gelesen");
        assert_eq!(keys(1), ["accept", "decline"]);
        assert!(keys(2).is_empty() && keys(3).is_empty());
        assert_eq!(m["quick"][0]["key"], "late");
    }

    #[test]
    fn the_ibis_and_the_ticket_printer_from_the_bus_keys_and_values() {
        let mut h = driving();
        let mut ibis: Vec<String> = (0..10).map(|d| format!("IBIS_{d}")).collect();
        ibis.extend(["IBIS_eingabe".to_string(), "IBIS_loeschen".to_string()]);
        add(&mut h, json!({ "t": "keys", "ibis": ibis, "printer": ["ticketprinter_button_ticket_1"], "change": [], "tickets": [{ "i": 1, "name": "Single", "price": 2.9, "day": false }] }));
        add(
            &mut h,
            json!({ "t": "dev", "ibis": { "IBIS_LinieKurs": 29.0, "IBIS_TerminusCode": 145.0, "IBIS_mode": 0.0 }, "strs": { "IBIS_terminus_name": "Markgraf-Berthold-Platz" },
                           "request": { "name": "Single", "price": 2.9 }, "sales": { "count": 3, "money": 8.7 } }),
        );
        let v = build(&h, &Facts { currency: "£".into(), ..Facts::default() }, 1.0);
        let i = &v["ibis"];
        assert_eq!(
            (i["kind"].as_str(), i["line"].as_i64(), i["ziel"].as_i64(), i["terminus"].as_str(), i["stop"].as_str()),
            (Some("stock"), Some(29), Some(145), Some("Markgraf-Berthold-Platz"), Some("Kunsthalle"))
        );
        assert_eq!(i["keys"].as_array().unwrap().len(), 12);
        let p = &v["printer"];
        assert_eq!((p["currency"].as_str(), p["request"]["name"].as_str(), p["sales"]["count"].as_i64()), (Some("£"), Some("Single"), Some(3)));
        assert_eq!(v["features"]["ibis"], true);
        assert_eq!(allowed_keys(&h).len(), 13);
        // a bus without them
        assert_eq!(build(&driving(), &Facts::default(), 1.0)["ibis"], Value::Null);
    }

    #[test]
    fn a_wait_at_the_terminus_is_a_break_until_the_next_trip_or_the_driver_ends_it() {
        let mut h = driving();
        let mut p = Pause::default();
        assert_eq!(p.look(&h), None, "on the way: no break");
        // trip 2 done at 31_000; trip 3 leaves at 32_000
        add(&mut h, json!({ "t": "live", "issued": 100, "clock": 31_050.0, "duty": { "trip": 2, "trip_done": true, "next": 2 } }));
        let b = p.look(&h).unwrap();
        assert_eq!((b["since"].as_f64(), b["until"].as_f64(), b["place"].as_str()), (Some(31_050.0), Some(32_000.0), Some("Markgraf-Berthold-Platz")));
        assert_eq!(b["next"]["line"], "M29");
        // later on: since stays
        add(&mut h, json!({ "t": "live", "issued": 100, "clock": 31_900.0, "duty": { "trip": 2, "trip_done": true, "next": 2 } }));
        assert_eq!(p.look(&h).unwrap()["since"], 31_050.0, "within the last two minutes it goes on");
        p.end();
        assert_eq!(p.look(&h), None, "ended by the driver");
        // standing at the first stop of a trip that leaves later is a break too
        let mut q = Pause::default();
        add(&mut h, json!({ "t": "live", "issued": 100, "clock": 29_000.0, "duty": { "trip": 2, "next": 0, "at_stop": true } }));
        assert_eq!(q.look(&h).unwrap()["until"], 29_900.0);
    }
}
