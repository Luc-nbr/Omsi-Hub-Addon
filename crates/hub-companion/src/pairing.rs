//! Pairing a phone or tablet with the add-on (the old game's companion's, without its tunnel).
//!
//! The server listens on the home network, and everyone on the same Wi-Fi can see its port. So
//! nothing but `/api/hello` answers without a key: the launcher shows a six-digit pairing code,
//! the app sends it once and gets a device key of 128 random bits back, which it keeps and
//! sends with every request. A code that was used is replaced by a new one, so a code read over
//! someone's shoulder is worth nothing after the pairing it was shown for.
//!
//! Ten wrong codes within a minute and the door stays shut for the rest of that minute: a
//! million codes at ten a minute is more than two months of trying.
//!
//! The device keys are kept (as SHA-256, never the keys themselves) in
//! `companion-devices.json` of the data folder, so that a paired phone is still paired after
//! the launcher was restarted; the launcher's list removes one.

use sha2::{Digest, Sha256};

/// Wrong codes allowed within [`MISS_WINDOW`] seconds.
pub const MAX_MISSES: u32 = 10;
pub const MISS_WINDOW: f64 = 60.0;
/// The pairing code's digits.
pub const CODE_LEN: usize = 6;
/// Devices kept paired; a further one pushes out the one paired longest ago.
pub const MAX_DEVICES: usize = 8;

/// A paired device.
#[derive(Debug, Clone, PartialEq)]
pub struct Paired {
    /// SHA-256 of the device key, as hex.
    pub hash: String,
    /// What the device said it is ("Luc's iPhone"), for the list of devices.
    pub name: String,
    /// Seconds since 1970 when it was paired, and when it last asked something.
    pub since: u64,
    pub seen: u64,
}

/// Why a pairing was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    Wrong,
    /// Too many wrong codes lately.
    Locked,
}

#[derive(Debug, Clone, Default)]
pub struct Pairing {
    code: String,
    pub devices: Vec<Paired>,
    misses: u32,
    misses_since: Option<f64>,
    /// The door is shut until then (steady seconds).
    locked_until: Option<f64>,
}

pub fn hash_key(key: &str) -> String {
    let d = Sha256::digest(key.as_bytes());
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compare two secrets in a time that does not tell how much of them matched.
fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut d = (a.len() != b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        d |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    d == 0
}

impl Pairing {
    pub fn new(code: String, devices: Vec<Paired>) -> Pairing {
        Pairing { code, devices, ..Default::default() }
    }

    /// The code the launcher shows now.
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn set_code(&mut self, code: String) {
        self.code = code;
    }

    /// A device offers `code`: its new device key, or why not. `now` in seconds of a steady
    /// clock, `unix` the date for the list. `new_code` and `new_key` make the next pairing
    /// code and the device key (random: [`crate::random_digits`], [`crate::random_key`]).
    pub fn pair(&mut self, code: &str, name: &str, now: f64, unix: u64, new_code: &mut dyn FnMut() -> String, new_key: &mut dyn FnMut() -> String) -> Result<String, Refused> {
        if let Some(t) = self.locked_until {
            if now < t {
                return Err(Refused::Locked);
            }
            self.locked_until = None;
            self.misses_since = None;
        }
        if self.misses_since.is_none_or(|t| now - t > MISS_WINDOW) {
            self.misses_since = Some(now);
            self.misses = 0;
        }
        if self.code.is_empty() || !same_secret(code.trim(), &self.code) {
            self.misses += 1;
            if self.misses >= MAX_MISSES {
                // (for the rest of the minute)
                self.locked_until = Some(self.misses_since.unwrap_or(now) + MISS_WINDOW);
            }
            return Err(Refused::Wrong);
        }
        self.misses = 0;
        let key = new_key();
        if self.devices.len() >= MAX_DEVICES {
            // (the oldest pairing goes)
            if let Some(oldest) = self.devices.iter().enumerate().min_by_key(|(_, d)| d.since).map(|(i, _)| i) {
                self.devices.remove(oldest);
            }
        }
        let name: String = name.chars().filter(|c| !c.is_control()).take(60).collect();
        let name = if name.trim().is_empty() { "Phone".to_string() } else { name.trim().to_string() };
        self.devices.push(Paired { hash: hash_key(&key), name, since: unix, seen: unix });
        self.code = new_code();
        Ok(key)
    }

    /// The device of this key, if it is paired.
    pub fn device_of(&self, key: &str) -> Option<usize> {
        if key.len() < 16 || key.len() > 128 {
            return None;
        }
        let h = hash_key(key);
        self.devices.iter().position(|d| same_secret(&d.hash, &h))
    }

    /// The device of this hash taken off the list (the launcher's "Remove"): whether it was on it.
    pub fn forget(&mut self, hash: &str) -> bool {
        let n = self.devices.len();
        self.devices.retain(|d| d.hash != hash);
        n != self.devices.len()
    }
}

/// The paired devices as `companion-devices.json` holds them.
pub fn devices_json(devices: &[Paired]) -> String {
    let list: Vec<serde_json::Value> = devices.iter().map(|d| serde_json::json!({ "key_sha256": d.hash, "name": d.name, "since": d.since, "seen": d.seen })).collect();
    serde_json::to_string_pretty(&serde_json::json!({ "devices": list })).unwrap_or_default()
}

pub fn parse_devices(text: &str) -> Vec<Paired> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let Some(list) = v.get("devices").and_then(|d| d.as_array()) else { return Vec::new() };
    list.iter()
        .filter_map(|d| {
            let hash = d.get("key_sha256")?.as_str()?.to_ascii_lowercase();
            let since = d.get("since").and_then(|s| s.as_u64()).unwrap_or(0);
            (hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| Paired {
                hash,
                name: d.get("name").and_then(|n| n.as_str()).unwrap_or("").chars().take(60).collect(),
                since,
                seen: d.get("seen").and_then(|s| s.as_u64()).unwrap_or(since),
            })
        })
        .take(MAX_DEVICES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("{n:032x}")
        }
    }

    #[test]
    fn the_right_code_pairs_once_and_is_replaced() {
        let mut p = Pairing::new("123456".into(), Vec::new());
        let (mut code, mut key) = (|| "654321".to_string(), keys());
        let k = p.pair("123456", "Luc's iPhone", 0.0, 1, &mut code, &mut key).unwrap();
        assert_eq!(p.device_of(&k), Some(0));
        assert_eq!(p.devices[0].name, "Luc's iPhone");
        assert_eq!(p.code(), "654321");
        // the old code is worth nothing now
        assert_eq!(p.pair("123456", "x", 1.0, 1, &mut code, &mut key), Err(Refused::Wrong));
        assert_eq!(p.device_of("00000000000000000000000000000099"), None);
        assert_eq!(p.device_of("short"), None);
        let hash = p.devices[0].hash.clone();
        assert!(p.forget(&hash));
        assert_eq!(p.device_of(&k), None, "removed in the launcher: not paired any more");
    }

    #[test]
    fn ten_wrong_codes_lock_the_door_for_the_minute() {
        let mut p = Pairing::new("123456".into(), Vec::new());
        let (mut code, mut key) = (|| "111111".to_string(), keys());
        for i in 0..MAX_MISSES {
            assert_eq!(p.pair(&format!("{i:06}"), "x", i as f64, 0, &mut code, &mut key), Err(Refused::Wrong));
        }
        assert_eq!(p.pair("123456", "x", 20.0, 0, &mut code, &mut key), Err(Refused::Locked));
        assert!(p.pair("123456", "x", 61.0, 0, &mut code, &mut key).is_ok());
    }

    #[test]
    fn the_oldest_device_goes_when_the_list_is_full() {
        let mut p = Pairing::new("1".into(), Vec::new());
        let (mut code, mut key) = (|| "1".to_string(), keys());
        let first = p.pair("1", "first", 0.0, 10, &mut code, &mut key).unwrap();
        for i in 1..MAX_DEVICES as u64 + 1 {
            p.pair("1", "next", 0.0, 10 + i, &mut code, &mut key).unwrap();
        }
        assert_eq!(p.devices.len(), MAX_DEVICES);
        assert_eq!(p.device_of(&first), None);
    }

    #[test]
    fn devices_are_kept_as_hashes_and_read_back() {
        let mut p = Pairing::new("1".into(), Vec::new());
        let (mut code, mut key) = (|| "2".to_string(), keys());
        let k = p.pair("1", "Pixel\u{7}", 0.0, 99, &mut code, &mut key).unwrap();
        let text = devices_json(&p.devices);
        assert!(!text.contains(&k), "the key itself is not written");
        let back = parse_devices(&text);
        assert_eq!(back, p.devices);
        assert_eq!(back[0].name, "Pixel");
        let again = Pairing::new("3".into(), back);
        assert_eq!(again.device_of(&k), Some(0));
        assert!(parse_devices("{\"devices\":[{\"key_sha256\":\"nothex\"}]}").is_empty());
    }
}
