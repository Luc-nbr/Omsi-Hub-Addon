//! Where the bus is, thirty times a second, for the app's navigator to follow it smoothly: the
//! plugin writes it to `data/pos.json` ([`DashPos`], `plugin/dash.lua`), only the latest; the
//! server reads it as often as it can change while a device asks for it, and keeps the last
//! second or so of them ([`Feed`]). `GET /api/pos?after=<n>` hands out those newer than `n`
//! (`server`), so the app gets every one, a few to a request.

use hub_game::bridge::{self, DashPos};
use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

/// How many places are kept (a little over a second of them).
pub const KEEP: usize = 40;
/// How often the file is read while a device follows the bus.
pub const READ_EVERY: Duration = Duration::from_millis(8);
/// How long the reading goes on after the last request.
pub const FOLLOWED_FOR: Duration = Duration::from_secs(5);
/// A place written longer ago than this is not where the bus is now (a game in its menu, or
/// none): the app goes by the state's bus then.
pub const FRESH: Duration = Duration::from_secs(3);
/// How long a request waits for a newer place.
pub const WAIT: Duration = Duration::from_secs(2);

/// The places read so far, newest last.
#[derive(Debug, Default)]
pub struct Feed {
    pub places: VecDeque<DashPos>,
    /// When a device last asked (the file is read only while one does).
    pub asked: Option<Instant>,
}

impl Feed {
    /// A place read: kept when it is new. A count lower than the last is a game started again
    /// (the plugin counts from 1): what was kept goes. True when it was kept.
    pub fn take(&mut self, p: DashPos) -> bool {
        match self.places.back() {
            Some(last) if p.n == last.n => return false,
            Some(last) if p.n < last.n => self.places.clear(),
            _ => {}
        }
        self.places.push_back(p);
        while self.places.len() > KEEP {
            self.places.pop_front();
        }
        true
    }

    /// The places after the `after`-th; with none asked for (or a count from before the game
    /// started again) the latest alone.
    pub fn since(&self, after: Option<u64>) -> Vec<DashPos> {
        let Some(last) = self.places.back() else { return Vec::new() };
        match after {
            Some(a) if a <= last.n => self.places.iter().filter(|p| p.n > a).cloned().collect(),
            _ => vec![last.clone()],
        }
    }

    /// Whether there is a place newer than `after` (any, when none is asked for).
    pub fn newer(&self, after: Option<u64>) -> bool {
        self.places.back().is_some_and(|last| after.is_none_or(|a| last.n != a))
    }

    /// Forget the places (the game is gone).
    pub fn clear(&mut self) {
        self.places.clear();
    }

    /// Whether a device follows the bus now.
    pub fn followed(&self) -> bool {
        self.asked.is_some_and(|t| t.elapsed() < FOLLOWED_FOR)
    }
}

/// The plugin's latest place, when it was written in the last few seconds.
pub fn read_fresh(plugin_dir: &Path) -> Option<DashPos> {
    let path = plugin_dir.join(bridge::DATA_DIR).join(bridge::POS_FILE);
    let written = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
    if SystemTime::now().duration_since(written).unwrap_or_default() > FRESH {
        return None;
    }
    bridge::read_pos(plugin_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(n: u64) -> DashPos {
        DashPos { n, t: n as f64 / 30.0, x: n as f64, y: 0.0, heading: Some(90.0), speed: 20.0 }
    }

    #[test]
    fn every_place_after_the_one_asked_for_and_a_new_game_starts_over() {
        let mut f = Feed::default();
        assert!(f.since(None).is_empty());
        assert!(f.take(at(1)) && f.take(at(2)) && !f.take(at(2)) && f.take(at(3)));
        assert_eq!(f.since(None).iter().map(|p| p.n).collect::<Vec<_>>(), [3]);
        assert_eq!(f.since(Some(1)).iter().map(|p| p.n).collect::<Vec<_>>(), [2, 3]);
        assert!(f.since(Some(3)).is_empty() && !f.newer(Some(3)) && f.newer(Some(2)));
        // (the game started again: its count from 1, the app's `after` from before)
        assert!(f.take(at(1)));
        assert_eq!(f.since(Some(3)).iter().map(|p| p.n).collect::<Vec<_>>(), [1]);
        assert!(f.newer(Some(3)));
        for n in 2..=100 {
            f.take(at(n));
        }
        assert_eq!(f.places.len(), KEEP);
        assert_eq!(f.since(Some(0)).first().map(|p| p.n), Some(100 - KEEP as u64 + 1));
    }
}
