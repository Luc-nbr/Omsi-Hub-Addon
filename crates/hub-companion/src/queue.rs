//! The commands on their way into the game: `data/companion.json` in the plugin's folder, which
//! the plugin reads four times a second (`plugin/companion.lua`). The file holds the last
//! commands with their numbers (`seq`) and the session they belong to (`sid`, new with every
//! start of the server); the plugin carries out each number of the session it has not done yet
//! and acks it by datagram. A plugin that starts takes the commands already in the file as
//! done, so a command is never carried out twice and never late, after a restart of the game.
//!
//! The file also says where the plugin sends its datagrams (`port`), and asks it for the name of
//! the street at a place (`probe`: the road after the next turn, `crate::nav`). While the file is
//! there the plugin talks to the companion; the server takes it away when it stops.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The file, in the plugin's data folder.
pub const FILE: &str = "companion.json";
/// Commands the file keeps.
const KEPT: usize = 20;

#[derive(Debug, Clone, Default)]
pub struct Queue {
    pub sid: u64,
    /// The port the plugin sends its datagrams to.
    pub port: u16,
    pub seq: u64,
    pub cmds: Vec<Value>,
    pub probe: Option<[f64; 2]>,
}

impl Queue {
    pub fn new(sid: u64) -> Queue {
        Queue { sid, ..Queue::default() }
    }

    /// A command for the plugin (its JSON from `Command::for_plugin`): its number.
    pub fn push(&mut self, mut cmd: Value) -> u64 {
        self.seq += 1;
        cmd["seq"] = json!(self.seq);
        self.cmds.push(cmd);
        let over = self.cmds.len().saturating_sub(KEPT);
        self.cmds.drain(..over);
        self.seq
    }

    /// The file's text.
    pub fn text(&self) -> String {
        json!({ "sid": self.sid, "port": self.port, "seq": self.seq, "cmds": self.cmds, "probe": self.probe }).to_string()
    }

    /// Written into the plugin's folder (through a temporary file: the game never reads half).
    pub fn write(&self, plugin_dir: &Path) -> std::io::Result<()> {
        let path = file_of(plugin_dir);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("omsihub-new");
        std::fs::write(&tmp, self.text())?;
        std::fs::rename(&tmp, &path)
    }
}

pub fn file_of(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(hub_game::bridge::DATA_DIR).join(FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_numbered_and_the_file_keeps_the_last_ones() {
        let mut q = Queue::new(77);
        assert_eq!(q.push(json!({ "do": "key", "key": "IBIS_1" })), 1);
        for _ in 0..KEPT + 4 {
            q.push(json!({ "do": "type_ibis" }));
        }
        assert_eq!(q.seq, KEPT as u64 + 5);
        assert_eq!(q.cmds.len(), KEPT);
        assert_eq!(q.cmds[0]["seq"], 6);
        q.probe = Some([1.5, -2.0]);
        let v: Value = serde_json::from_str(&q.text()).unwrap();
        assert_eq!((v["sid"].as_u64(), v["seq"].as_u64()), (Some(77), Some(KEPT as u64 + 5)));
        assert_eq!(v["probe"], json!([1.5, -2.0]));
        let dir = std::env::temp_dir().join(format!("hub-companion-queue-{}", std::process::id()));
        q.write(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(file_of(&dir)).unwrap(), q.text());
        let _ = std::fs::remove_dir_all(dir);
    }
}
