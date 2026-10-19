//! What the plugin told the add-on of the game: its datagrams (`plugin/companion.lua`), one JSON
//! object each, put together here. A list too long for one datagram (8 KB) comes in pages that
//! carry their list's version (`v`), page and pages: a list is taken once every page of one
//! version is in, so the state never shows half of a trip's stops.

use serde_json::{Map, Value};

/// A list the plugin sends in pages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Paged {
    /// The last version taken whole (the plugin counts from 1), and its items.
    pub v: i64,
    pub items: Vec<Value>,
    /// The version being put together, and its pages so far.
    filling: i64,
    pages: Vec<Option<Vec<Value>>>,
}

impl Paged {
    /// A page of the list (`key`: where its items are in the datagram). Whether the list is
    /// whole now with a new version.
    fn take(&mut self, d: &Value, key: &str) -> bool {
        let v = d.get("v").and_then(Value::as_i64).unwrap_or(0);
        let pages = d.get("pages").and_then(Value::as_u64).unwrap_or(1).clamp(1, 64) as usize;
        let page = d.get("page").and_then(Value::as_u64).unwrap_or(1).clamp(1, pages as u64) as usize;
        let items = d.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
        if v != self.filling || self.pages.len() != pages {
            self.filling = v;
            self.pages = vec![None; pages];
        }
        self.pages[page - 1] = Some(items);
        if self.pages.iter().all(Option::is_some) && v != self.v {
            self.items = self.pages.iter_mut().flat_map(|p| p.take().unwrap_or_default()).collect();
            self.v = v;
            self.pages.clear();
            self.filling = i64::MIN;
            return true;
        }
        false
    }
}

/// Everything heard, and when (seconds of a steady clock).
#[derive(Debug, Clone, Default)]
pub struct Heard {
    /// The last `live` datagram, and when it came.
    pub live: Map<String, Value>,
    pub live_at: Option<f64>,
    /// The trip's stops, the duty's trips, the map's lines.
    pub stops: Paged,
    pub trips: Paged,
    pub lines: Paged,
    /// The bus's keys of the IBIS and the ticket printer (`keys`) and their live values (`dev`).
    pub keys: Map<String, Value>,
    pub dev: Map<String, Value>,
    /// The commands the plugin has carried out: (seq, ok, error).
    pub acks: Vec<(u64, bool, String)>,
}

impl Heard {
    /// One datagram; whether it changed what the devices are shown.
    pub fn hear(&mut self, text: &[u8], now: f64) -> bool {
        let Ok(d) = serde_json::from_slice::<Value>(text) else { return false };
        match d.get("t").and_then(Value::as_str) {
            Some("live") => {
                let Value::Object(mut m) = d else { return false };
                m.remove("t");
                let changed = m != self.live;
                self.live = m;
                self.live_at = Some(now);
                changed
            }
            Some("trip") => self.stops.take(&d, "stops"),
            Some("trips") => self.trips.take(&d, "trips"),
            Some("lines") => self.lines.take(&d, "lines"),
            Some("keys") => {
                let Value::Object(mut m) = d else { return false };
                m.remove("t");
                let changed = m != self.keys;
                self.keys = m;
                changed
            }
            Some("dev") => {
                let Value::Object(mut m) = d else { return false };
                m.remove("t");
                let changed = m != self.dev;
                self.dev = m;
                changed
            }
            Some("ack") => {
                let Some(seq) = d.get("seq").and_then(Value::as_u64) else { return false };
                let ok = d.get("ok").and_then(Value::as_bool).unwrap_or(false);
                let error = d.get("error").and_then(Value::as_str).unwrap_or("").to_string();
                self.acks.push((seq, ok, error));
                // (the server takes them as they come: a few are plenty)
                let over = self.acks.len().saturating_sub(64);
                self.acks.drain(..over);
                false
            }
            _ => false,
        }
    }

    /// The game was heard within `secs` seconds of `now`.
    pub fn fresh(&self, now: f64, secs: f64) -> bool {
        self.live_at.is_some_and(|t| now - t <= secs)
    }

    /// One of the live datagram's values.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.live.get(key).filter(|v| !v.is_null())
    }

    /// The ack of command `seq`, if it came.
    pub fn ack(&self, seq: u64) -> Option<(bool, String)> {
        self.acks.iter().rev().find(|a| a.0 == seq).map(|a| (a.1, a.2.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dg(v: Value) -> Vec<u8> {
        v.to_string().into_bytes()
    }

    #[test]
    fn a_paged_list_is_taken_once_all_its_pages_are_in() {
        let mut h = Heard::default();
        assert!(!h.hear(&dg(json!({ "t": "trip", "v": 3, "page": 2, "pages": 2, "stops": [{ "name": "B" }] })), 0.0));
        assert!(h.stops.items.is_empty(), "half a trip is not shown");
        assert!(h.hear(&dg(json!({ "t": "trip", "v": 3, "page": 1, "pages": 2, "stops": [{ "name": "A" }] })), 0.0));
        let names: Vec<&str> = h.stops.items.iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["A", "B"]);
        // the same version again changes nothing; a new one of one page replaces it
        assert!(!h.hear(&dg(json!({ "t": "trip", "v": 3, "page": 1, "pages": 2, "stops": [{ "name": "A" }] })), 0.0));
        assert!(h.hear(&dg(json!({ "t": "trip", "v": 4, "stops": [{ "name": "C" }] })), 0.0));
        assert_eq!(h.stops.items.len(), 1);
        // a page of a newer version throws the older half away
        assert!(!h.hear(&dg(json!({ "t": "trip", "v": 5, "page": 1, "pages": 2, "stops": [] })), 0.0));
        assert!(!h.hear(&dg(json!({ "t": "trip", "v": 6, "page": 2, "pages": 2, "stops": [] })), 0.0));
        assert_eq!(h.stops.v, 4);
    }

    #[test]
    fn the_live_datagram_says_when_the_game_was_heard_and_acks_are_kept() {
        let mut h = Heard::default();
        assert!(!h.fresh(10.0, 5.0));
        assert!(h.hear(&dg(json!({ "t": "live", "clock": 30240.0 })), 10.0));
        assert!(!h.hear(&dg(json!({ "t": "live", "clock": 30240.0 })), 10.5), "the same: nothing new");
        assert!(h.fresh(14.0, 5.0) && !h.fresh(16.0, 5.0));
        assert_eq!(h.get("clock"), Some(&json!(30240.0)));
        h.hear(&dg(json!({ "t": "ack", "seq": 7, "ok": false, "error": "wrong" })), 11.0);
        assert_eq!(h.ack(7), Some((false, "wrong".into())));
        assert_eq!(h.ack(8), None);
        assert!(!h.hear(b"not json", 12.0));
    }
}
