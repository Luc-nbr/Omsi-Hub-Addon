//! Multiplayer, as the old launcher's page had it: the next duty hosted (the game opens the
//! session: `--lan-host`, its code in openOMSI's own `~/.openomsi/lan/<game>.json`, only read
//! here), a friend's session joined by its code (`--lan-join`), and servers that are always on,
//! the official one first. All of it is openOMSI's own network layer (`omsi-net`, from Luc's
//! fork): what a code means, a server's or a host's status, the Cloudflare tunnel's program.
//!
//! Commands: `hub_lan_check` (what a code or address means), `hub_lan_query` (a server's or a
//! code's host's status), `hub_lan_servers` / `hub_lan_servers_save` (the list, the data
//! folder's `servers.json`; the old launcher's in `~/.openomsi` until one is saved),
//! `hub_lan_tunnel` (the tunnel's program fetched before the hosted duty starts) and
//! `hub_lan_hosted` (the sessions games on this computer host now).

use crate::{arg, Answer};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A command of multiplayer, or None when `cmd` is not one.
pub fn call(cmd: &str, args: &Value) -> Option<Answer> {
    Some(match cmd {
        "hub_lan_check" => arg::<String>(args, "text").map(|t| check_join(&t)),
        "hub_lan_query" => arg::<String>(args, "address").and_then(|a| server_query(&a)),
        "hub_lan_servers" => Ok(servers_get()),
        "hub_lan_servers_save" => arg::<Value>(args, "list").and_then(|l| servers_save(&l)),
        "hub_lan_tunnel" => {
            std::thread::spawn(omsi_net::tunnel::ensure_cloudflared);
            Ok(Value::Null)
        }
        "hub_lan_hosted" => Ok(json!(local_hosts())),
        _ => return None,
    })
}

/// openOMSI's own folder in the home folder (the games' LAN status files, the old launcher's
/// servers).
fn openomsi_home() -> PathBuf {
    hub_core::paths::home_dir().join(".openomsi")
}

/// What a join field means (or what is wrong with it), and the sessions hosted here.
pub fn check_join(text: &str) -> Value {
    match omsi_net::describe_join(text) {
        Ok(d) => json!({ "ok": true, "text": d, "local": local_hosts() }),
        Err(e) => json!({ "ok": false, "text": e, "local": local_hosts() }),
    }
}

/// A status file is a game's that still runs: openOMSI writes it again every few seconds
/// (`updated`) and removes it when the game ends; one left by a game that crashed is old.
const FRESH_SECS: u64 = 60;

/// The sessions games on this computer host, from their status files in `dir`.
fn hosts_in(dir: &Path, now: u64) -> Vec<Value> {
    let mut out: Vec<Value> =
        std::fs::read_dir(dir).map(|rd| rd.flatten().filter_map(|e| std::fs::read(e.path()).ok()).filter_map(|b| serde_json::from_slice::<Value>(&b).ok()).collect()).unwrap_or_default();
    out.retain(|v| v.get("role").and_then(Value::as_str) == Some("host") && v.get("updated").and_then(Value::as_u64).is_some_and(|t| now.saturating_sub(t) <= FRESH_SECS));
    out
}

/// The sessions games on this computer host now (the session code the page shows).
pub fn local_hosts() -> Vec<Value> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    hosts_in(&openomsi_home().join("lan"), now)
}

/// The data folder's list of servers.
fn servers_path() -> PathBuf {
    hub_core::data_dir().join("servers.json")
}

/// The saved servers (the old launcher's until one is saved here), the official one first.
pub fn servers_get() -> Value {
    let read = |p: PathBuf| std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok());
    let mut list = read(servers_path()).or_else(|| read(openomsi_home().join("servers.json"))).unwrap_or_default();
    if !list.iter().any(|s| s.get("address").and_then(Value::as_str).is_some_and(omsi_net::official::is_alias)) {
        list.insert(0, json!({ "name": omsi_net::official::NAME, "address": omsi_net::official::ALIAS }));
    }
    json!(list)
}

pub fn servers_save(list: &Value) -> Answer {
    if !list.is_array() {
        return Err("the servers are a list".into());
    }
    let path = servers_path();
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_vec_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(json!(true))
}

/// A server's or a session code's status: its name, map, players, its buses.
pub fn server_query(address: &str) -> Answer {
    let i = if omsi_net::looks_like_code(address) { host_status(address) } else { omsi_net::ws::query(address, true) }?;
    let icon = (!i.icon.is_empty()).then(|| format!("data:image/png;base64,{}", crate::launcher::base64_encode(&i.icon)));
    Ok(json!({
        "name": i.name, "motd": i.motd, "map": i.map, "players": i.players, "max_players": i.max_players,
        "version": i.version, "icon": icon, "time": i.time, "weather": i.weather, "password": i.password,
        "vehicles": i.vehicles, "reached_at": i.reached_at,
    }))
}

/// A code's host's status page (its gateway is the session's port + 10), else its tunnel.
fn host_status(code: &str) -> Result<omsi_net::ws::ServerInfo, String> {
    let c = omsi_net::SessionCode::decode(code)?;
    for a in c.addrs().into_iter().take(3) {
        if let Ok(i) = omsi_net::ws::query(&format!("http://{}:{}", a.ip(), a.port().saturating_add(10)), false) {
            return Ok(i);
        }
    }
    match omsi_net::bridge::lookup_tunnel(c.session) {
        Some(url) => omsi_net::ws::query(&url, false),
        None => Err("the host did not answer".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sessions_hosted_here_are_the_running_games_hosts() {
        let dir = std::env::temp_dir().join(format!("omsihub-lan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let w = |n: &str, v: Value| std::fs::write(dir.join(n), v.to_string()).unwrap();
        w("1.json", json!({ "pid": 1, "role": "host", "code": "OMSI-AAAA", "updated": 1000 }));
        w("2.json", json!({ "pid": 2, "role": "client", "updated": 1000 }));
        // (a game that crashed left its file behind)
        w("3.json", json!({ "pid": 3, "role": "host", "code": "OMSI-OLD", "updated": 100 }));
        std::fs::write(dir.join("4.json"), "not json").unwrap();
        let hosts = hosts_in(&dir, 1030);
        assert_eq!(hosts.iter().map(|h| h["code"].as_str().unwrap()).collect::<Vec<_>>(), vec!["OMSI-AAAA"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_join_field_is_described() {
        assert_eq!(check_join("")["ok"], omsi_net::describe_join("").is_ok());
        assert_eq!(check_join("192.168.1.20")["ok"], true);
        assert!(check_join("192.168.1.20")["text"].as_str().is_some_and(|t| !t.is_empty()));
    }

    #[test]
    fn the_official_server_is_always_listed() {
        let list = servers_get();
        assert!(list.as_array().unwrap().iter().any(|s| s["address"].as_str().is_some_and(omsi_net::official::is_alias)));
    }
}
