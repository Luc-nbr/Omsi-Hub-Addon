//! Omsi-Hub's log: `omsi-hub.log` in the data folder, one line per event with its Unix time.
//! Never a password, a token or a path of the player's beyond the folders they chose.

use std::io::Write;

/// The file in the data folder.
pub const FILE: &str = "omsi-hub.log";
/// Past this size the log starts again (the old one kept as `omsi-hub.log.1`).
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// One line into the log.
pub fn log_line(line: &str) {
    let p = crate::paths::data_dir().join(FILE);
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(&p, p.with_extension("log.1"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "{now} {}", line.replace(['\r', '\n'], " "));
    }
}
