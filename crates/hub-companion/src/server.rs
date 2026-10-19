//! The server: a thread that listens on the home network and a thread for each connection
//! (sixteen at most), a thread that hears the plugin's datagrams on this computer, and one that
//! looks at everything twice a second (the clock moves, the inbox changes, a break begins).
//!
//! They meet in [`Shared`]: the datagrams and the looks write the state the devices see, the
//! devices' commands go into the plugin's queue and wait for its ack. A device asks for news by
//! long polling: `GET /api/state?after=<version>` is answered as soon as there is a newer state
//! (or after 20 s with the same one). Anything slow - reading the map for a route or its roads -
//! happens on a thread of its own or the connection's, never under the lock.

use crate::heard::Heard;
use crate::http::{self, Command, Request, Response, Route};
use crate::nav::{self, Tracker};
use crate::pairing::{self, Pairing, Refused};
use crate::queue::Queue;
use crate::state::{self, Facts, Pause};
use crate::{Config, Source};
use hub_game::bridge::Inbox;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

/// Connections served at once; more are closed at once.
const MAX_CONNECTIONS: usize = 16;
/// How long a state request waits for news, and a command for the game to have done it.
const STATE_WAIT: Duration = Duration::from_secs(20);
const COMMAND_WAIT: Duration = Duration::from_secs(4);
/// How often everything is looked at.
const LOOK_EVERY: Duration = Duration::from_millis(500);
/// A device counts as connected this long after its last request (s).
pub const SEEN_FOR: u64 = 45;
/// Signing on: tries allowed a minute.
const SIGN_ON_TRIES: usize = 10;
/// The bus moved this far (m) from where the street was last asked for: asked again.
const PROBE_MOVE: f64 = 5.0;

/// The route of the trip under way, in the game's coordinates.
#[derive(Default)]
struct Routing {
    /// The map and the trip it is of ("" while none).
    key: String,
    loading: bool,
    tracker: Option<Tracker>,
    v: u64,
    /// Map metres as the add-on reads them, minus these, are the game's (None: not known yet).
    off: Option<[f64; 2]>,
}

pub struct Inner {
    /// The state a device sees, and its version: one up with every change.
    pub state: Value,
    pub version: u64,
    pub heard: Heard,
    pub queue: Queue,
    pub pause: Pause,
    pub pairing: Pairing,
    devices_path: PathBuf,
    /// When each device (by its key's hash) last asked something (Unix seconds).
    pub seen: HashMap<String, u64>,
    routing: Routing,
    street: String,
    /// The plugin's folder, looked up now and then; the inbox read from it (and its time).
    plugin_dir: Option<PathBuf>,
    plugin_dir_at: Option<Instant>,
    inbox: Option<(SystemTime, Inbox)>,
    sign_ons: Vec<Instant>,
    pub stopped: bool,
}

pub struct Shared {
    pub inner: Mutex<Inner>,
    pub news: Condvar,
    pub source: Arc<dyn Source>,
    pub cfg: Config,
    /// The steady clock's start (the datagrams' and the pairing's seconds).
    start: Instant,
}

impl Shared {
    pub fn new(cfg: Config, source: Arc<dyn Source>, code: String, sid: u64) -> Shared {
        let devices = std::fs::read_to_string(&cfg.devices_file).map(|t| pairing::parse_devices(&t)).unwrap_or_default();
        let inner = Inner {
            state: Value::Null,
            version: 0,
            heard: Heard::default(),
            queue: Queue { port: cfg.udp_port, ..Queue::new(sid) },
            pause: Pause::default(),
            pairing: Pairing::new(code, devices),
            devices_path: cfg.devices_file.clone(),
            seen: HashMap::new(),
            routing: Routing::default(),
            street: String::new(),
            plugin_dir: None,
            plugin_dir_at: None,
            inbox: None,
            sign_ons: Vec::new(),
            stopped: false,
        };
        Shared { inner: Mutex::new(inner), news: Condvar::new(), source, cfg, start: Instant::now() }
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Seconds of the steady clock.
    pub fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// A datagram of the plugin.
    pub fn hear(self: &Arc<Self>, text: &[u8]) {
        let now = self.now();
        let mut i = self.lock();
        if i.heard.hear(text, now) {
            self.refresh(&mut i);
        }
        drop(i);
        // (an ack changes no state, and a command waits for it)
        self.news.notify_all();
    }

    /// Everything looked at again: the state made, and the devices told when it changed.
    pub fn refresh(self: &Arc<Self>, i: &mut Inner) {
        let now = self.now();
        // the plugin's folder (from the settings: looked up every few seconds), and the inbox
        if i.plugin_dir_at.is_none_or(|t| t.elapsed() > Duration::from_secs(5)) {
            let dir = self.source.plugin_dir();
            if dir != i.plugin_dir {
                if let Some(d) = &dir {
                    // (a new folder: the plugin there starts from this session's queue)
                    let _ = i.queue.write(d);
                }
                i.plugin_dir = dir;
            }
            i.plugin_dir_at = Some(Instant::now());
        }
        let inbox_path = i.plugin_dir.as_ref().map(|d| d.join(hub_game::bridge::DATA_DIR).join(hub_game::bridge::MESSAGES_FILE));
        let modified = inbox_path.as_ref().and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        match (modified, &inbox_path) {
            (Some(m), Some(p)) if i.inbox.as_ref().is_none_or(|x| x.0 != m) => {
                i.inbox = std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).map(|x| (m, x));
            }
            (None, _) => i.inbox = None,
            _ => {}
        }
        let game = i.heard.fresh(now, state::GAME_FRESH);
        let Inner { heard, pause, .. } = &mut *i;
        let brk = if game { pause.look(heard) } else { None };
        let next = self.navigate(i, game);
        let facts = Facts {
            assignment: self.source.assignment(),
            inbox: i.inbox.as_ref().map(|x| x.1.clone()),
            currency: self.source.currency(),
            nav: next,
            street: i.street.clone(),
            route_v: i.routing.v,
            pause: brk,
        };
        let s = state::build(&i.heard, &facts, now);
        if s != i.state {
            i.state = s;
            i.version += 1;
            self.news.notify_all();
        }
    }

    /// The next turn, with the route of the trip under way read (on a thread of its own) when
    /// it is another trip than the one read last.
    fn navigate(self: &Arc<Self>, i: &mut Inner, game: bool) -> Option<nav::Next> {
        if !game {
            return None;
        }
        let num = |v: Option<&Value>| v.and_then(Value::as_f64).filter(|x| x.is_finite());
        let bus = i.heard.get("bus")?;
        let at = [num(bus.get("x"))?, num(bus.get("y"))?];
        let duty = i.heard.get("duty")?.clone();
        let next = duty.get("next").and_then(Value::as_u64).unwrap_or(0) as usize;
        let stop = i.heard.stops.items.get(next).and_then(|s| Some([num(s.get("x"))?, num(s.get("y"))?]));
        let map = i.heard.get("map").and_then(Value::as_str).unwrap_or("").to_string();
        let trip = duty.get("trip_name").and_then(Value::as_str).unwrap_or("").to_string();
        let key = format!("{map}\n{trip}");
        if !map.is_empty() && !trip.is_empty() && key != i.routing.key && !i.routing.loading {
            i.routing.key = key.clone();
            i.routing.loading = true;
            i.routing.tracker = None;
            let date = i.heard.get("date").and_then(Value::as_str).unwrap_or("").to_string();
            let reference = i.heard.get("ref").cloned();
            let me = Arc::clone(self);
            let _ = std::thread::Builder::new().name("companion route".into()).spawn(move || {
                let off = reference.and_then(|r| me.offset(&map, &r));
                let route = me.source.route(&map, &date, &trip).unwrap_or_default();
                let o = off.unwrap_or([0.0, 0.0]);
                let pts: Vec<[f64; 2]> = route.iter().map(|p| [p[0] + o[0], p[1] + o[1]]).collect();
                let mut i = me.lock();
                if i.routing.key == key {
                    i.routing.loading = false;
                    i.routing.off = off.or(i.routing.off);
                    i.routing.tracker = (pts.len() >= 2).then(|| Tracker::new(nav::Route::new(&pts)));
                    i.routing.v += 1;
                    me.refresh(&mut i);
                }
            });
        }
        let next = match i.routing.tracker.as_mut() {
            Some(t) => t.next(at, stop),
            None => nav::straight_to(at, stop),
        };
        // the street after the turn: asked of the game at a place a little way into it
        let probe = next.and_then(|n| n.probe);
        let moved = match (probe, i.queue.probe) {
            (Some(p), Some(q)) => ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt() > PROBE_MOVE,
            (a, b) => a.is_some() != b.is_some(),
        };
        if moved {
            i.queue.probe = probe;
            i.street.clear();
            if let Some(d) = &i.plugin_dir {
                let _ = i.queue.write(d);
            }
        }
        if let (Some(q), Some(p)) = (i.queue.probe, i.heard.get("probe")) {
            let (px, py) = (num(p.get("x")).unwrap_or(f64::MAX), num(p.get("y")).unwrap_or(f64::MAX));
            if (px - q[0]).abs() < 1.0 && (py - q[1]).abs() < 1.0 {
                i.street = street_name(p.get("name").and_then(Value::as_str).unwrap_or(""));
            }
        }
        next
    }

    /// The game's coordinates less the add-on's map metres, from the place the plugin gave with
    /// its tile (`ref`).
    fn offset(&self, map: &str, r: &Value) -> Option<[f64; 2]> {
        let n = |k: &str| r.get(k).and_then(Value::as_f64);
        let w = self.source.world_of(map, [n("tile_x")? as i64, n("tile_y")? as i64], [n("local_x")?, n("local_y")?])?;
        Some([n("x")? - w[0], n("y")? - w[1]])
    }
}

/// A lane's name as the game gives it, when it names a street (not a spline's file).
fn street_name(name: &str) -> String {
    let n = name.trim();
    let lower = n.to_lowercase();
    if n.is_empty() || lower.ends_with(".sli") || lower.ends_with(".sco") || n.contains('\\') || n.contains('/') || n.chars().count() > 48 {
        return String::new();
    }
    n.to_string()
}

/// The running server; dropping it stops it.
pub struct Server {
    pub addr: SocketAddr,
    pub shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let mut i = self.shared.lock();
        i.stopped = true;
        // (the plugin stops talking to a companion that is not there)
        if let Some(d) = &i.plugin_dir {
            let _ = std::fs::remove_file(crate::queue::file_of(d));
        }
        drop(i);
        self.shared.news.notify_all();
    }
}

impl Server {
    /// Listen on `listen` for the devices and on `udp` (this computer only) for the plugin.
    pub fn start(listen: SocketAddr, udp: SocketAddr, shared: Arc<Shared>) -> std::io::Result<Server> {
        let listener = TcpListener::bind(listen)?;
        let addr = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let socket = UdpSocket::bind(udp)?;
        socket.set_read_timeout(Some(Duration::from_millis(250)))?;
        let stop = Arc::new(AtomicBool::new(false));
        if let Some(d) = shared.source.plugin_dir() {
            let _ = shared.lock().queue.write(&d);
        }
        let (st, sh) = (stop.clone(), shared.clone());
        std::thread::Builder::new().name("companion".into()).spawn(move || {
            let open = Arc::new(AtomicUsize::new(0));
            while !st.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, peer)) => {
                        if !lan_peer(peer.ip()) || open.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
                            continue;
                        }
                        open.fetch_add(1, Ordering::Relaxed);
                        let (sh, held) = (sh.clone(), open.clone());
                        let spawned = std::thread::Builder::new().name("companion device".into()).spawn(move || {
                            let _ = serve(stream, &sh);
                            held.fetch_sub(1, Ordering::Relaxed);
                        });
                        if spawned.is_err() {
                            open.fetch_sub(1, Ordering::Relaxed);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(40)),
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
        })?;
        let (st, sh) = (stop.clone(), shared.clone());
        std::thread::Builder::new().name("companion plugin".into()).spawn(move || {
            let mut buf = vec![0u8; 16 * 1024];
            while !st.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    // (only the game on this computer)
                    Ok((n, from)) if from.ip().is_loopback() => sh.hear(&buf[..n]),
                    _ => {}
                }
            }
        })?;
        let (st, sh) = (stop.clone(), shared.clone());
        std::thread::Builder::new().name("companion look".into()).spawn(move || {
            while !st.load(Ordering::Relaxed) {
                {
                    let mut i = sh.lock();
                    sh.refresh(&mut i);
                }
                std::thread::sleep(LOOK_EVERY);
            }
        })?;
        Ok(Server { addr, shared, stop })
    }
}

/// Whether a device at `ip` is on the home network: this machine, a private network (a
/// router's 192.168.x.x, 10.x, 172.16-31.x) or a link without a router. A computer with an
/// address straight on the internet listens there too, and nobody from there gets in.
pub fn lan_peer(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local() || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1])),
        std::net::IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => lan_peer(v4.into()),
            // (unique local fc00::/7 and link-local fe80::/10)
            None => v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80,
        },
    }
}

/// Read one request from `s` and answer it.
fn serve(mut s: TcpStream, shared: &Arc<Shared>) -> std::io::Result<()> {
    s.set_nonblocking(false)?;
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
    let _ = s.set_nodelay(true);
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 2048];
    let (mut req, at) = loop {
        match http::parse_head(&buf) {
            Ok(r) => break r,
            Err(http::HttpError::Incomplete) => {}
            Err(e) => return s.write_all(&Response::error(if e == http::HttpError::TooLarge { 413 } else { 400 }, "bad_request").to_bytes(false)),
        }
        let n = s.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let want = match http::body_length(&req) {
        Ok(n) => n,
        Err(e) => return s.write_all(&Response::error(if e == http::HttpError::TooLarge { 413 } else { 400 }, "bad_request").to_bytes(false)),
    };
    while buf.len() < at + want {
        let n = s.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    req.body = buf.get(at..(at + want).min(buf.len())).unwrap_or_default().to_vec();
    let answer = handle(&req, shared);
    s.write_all(&answer.to_bytes(req.method == "HEAD"))?;
    s.flush()
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The answer to one request.
pub fn handle(req: &Request, shared: &Arc<Shared>) -> Response {
    let route = http::route(req);
    if route.needs_key() {
        let mut i = shared.lock();
        let Some(d) = i.pairing.device_of(req.key()) else { return Response::error(401, "not_paired") };
        let hash = i.pairing.devices[d].hash.clone();
        i.seen.insert(hash, unix_now());
    }
    match route {
        Route::Preflight => Response::new(204, "text/plain; charset=utf-8", ""),
        Route::NotFound => Response::error(404, "not_found"),
        Route::Method => Response::error(405, "method"),
        Route::Hello => Response::json(&json!({ "app": "omsi-hub", "api": crate::API, "version": shared.cfg.version, "name": shared.cfg.name, "pairing": true })),
        Route::Pair => {
            let body = req.json();
            let code = body.get("code").and_then(Value::as_str).unwrap_or("");
            let device = body.get("device").and_then(Value::as_str).unwrap_or("");
            let now = shared.now();
            let mut i = shared.lock();
            let paired = i.pairing.pair(code, device, now, unix_now(), &mut crate::random_code, &mut crate::random_key);
            match paired {
                Ok(key) => {
                    let _ = std::fs::write(&i.devices_path, pairing::devices_json(&i.pairing.devices));
                    Response::json(&json!({ "key": key, "name": shared.cfg.name }))
                }
                Err(Refused::Wrong) => Response::error(403, "wrong_code"),
                Err(Refused::Locked) => Response::error(429, "too_many"),
            }
        }
        Route::State(after) => {
            let deadline = Instant::now() + STATE_WAIT;
            let mut i = shared.lock();
            while after.is_some_and(|a| a >= i.version) && !i.stopped {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                i = shared.news.wait_timeout(i, left).unwrap_or_else(|e| e.into_inner()).0;
            }
            let mut v = i.state.clone();
            v["v"] = json!(i.version);
            Response::json(&v)
        }
        Route::Do => {
            let Some(cmd) = Command::from_json(&req.json()) else { return Response::error(400, "unknown_command") };
            Response::json(&run(shared, cmd))
        }
        Route::Route => {
            let i = shared.lock();
            let r1 = |v: f64| (v * 10.0).round() / 10.0;
            let pts: Vec<[f64; 2]> = i.routing.tracker.as_ref().map(|t| t.route.pts.iter().map(|p| [r1(p[0]), r1(p[1])]).collect()).unwrap_or_default();
            let stops: Vec<Value> = i
                .heard
                .stops
                .items
                .iter()
                .filter(|s| s.get("x").is_some_and(Value::is_number))
                .map(|s| json!({ "name": s.get("name").cloned().unwrap_or_default(), "x": s.get("x"), "y": s.get("y") }))
                .collect();
            Response::json(&json!({ "v": i.routing.v, "pts": pts, "stops": stops }))
        }
        Route::Roads { x, y, r } => {
            let (map, date, off, reference) = {
                let i = shared.lock();
                let st = |k: &str| i.heard.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                (st("map"), st("date"), i.routing.off, i.heard.get("ref").cloned())
            };
            if map.is_empty() {
                return Response::json(&json!({ "roads": [], "water": [], "green": [] }));
            }
            let off = off.or_else(|| reference.and_then(|r| shared.offset(&map, &r))).unwrap_or([0.0, 0.0]);
            let r1 = |v: f64| (v * 10.0).round() / 10.0;
            let roads: Vec<Value> = shared
                .source
                .roads(&map, &date, [x - off[0], y - off[1]], r)
                .into_iter()
                .map(|(w, pts)| json!({ "w": r1(w), "pts": pts.iter().map(|p| [r1(p[0] + off[0]), r1(p[1] + off[1])]).collect::<Vec<_>>() }))
                .collect();
            Response::json(&json!({ "roads": roads, "water": [], "green": [] }))
        }
        Route::Lines => {
            let i = shared.lock();
            Response::json(&json!({ "lines": i.heard.lines.items }))
        }
    }
}

/// A command carried out: the add-on's own at once, the game's through the plugin's queue,
/// answered when the plugin acked it.
pub fn run(shared: &Arc<Shared>, cmd: Command) -> Value {
    let refused = |e: &str| json!({ "ok": false, "error": e });
    let mut i = shared.lock();
    if cmd == Command::PauseEnd {
        i.pause.end();
        shared.refresh(&mut i);
        return json!({ "ok": true });
    }
    if let Command::IbisKey { key } | Command::PrinterKey { key } = &cmd {
        if !state::allowed_keys(&i.heard).contains(key) {
            return refused("unknown_key");
        }
    }
    if matches!(cmd, Command::SignOn { .. }) {
        i.sign_ons.retain(|t| t.elapsed() < Duration::from_secs(60));
        if i.sign_ons.len() >= SIGN_ON_TRIES {
            return refused("too_many");
        }
        i.sign_ons.push(Instant::now());
    }
    if !i.heard.fresh(shared.now(), state::GAME_FRESH) {
        return refused("game_not_running");
    }
    let Some(dir) = i.plugin_dir.clone() else { return refused("no_plugin") };
    let Some(body) = cmd.for_plugin() else { return refused("unknown_command") };
    let seq = i.queue.push(body);
    if i.queue.write(&dir).is_err() {
        return refused("no_plugin");
    }
    let deadline = Instant::now() + COMMAND_WAIT;
    loop {
        if let Some((ok, error)) = i.heard.ack(seq) {
            return if ok { json!({ "ok": true }) } else { refused(if error.is_empty() { "refused" } else { &error }) };
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || i.stopped {
            return refused("timeout");
        }
        i = shared.news.wait_timeout(i, left).unwrap_or_else(|e| e.into_inner()).0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_game::bridge::Assignment;

    /// A source with a plugin folder of its own, a route due north and roads beside it.
    struct Fake {
        dir: PathBuf,
    }

    impl Source for Fake {
        fn plugin_dir(&self) -> Option<PathBuf> {
            Some(self.dir.clone())
        }
        fn assignment(&self) -> Option<Assignment> {
            None
        }
        fn route(&self, _map: &str, _date: &str, _trip: &str) -> Option<Vec<[f64; 2]>> {
            // (the add-on's map metres: the game's are 1000 m further east)
            Some(vec![[0.0, 0.0], [0.0, 300.0], [200.0, 300.0]])
        }
        fn roads(&self, _map: &str, _date: &str, at: [f64; 2], _r: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
            vec![(7.0, vec![at, [at[0] + 10.0, at[1]]])]
        }
        fn world_of(&self, _map: &str, tile: [i64; 2], local: [f64; 2]) -> Option<[f64; 2]> {
            Some([tile[0] as f64 * 300.0 + local[0], tile[1] as f64 * 300.0 + local[1]])
        }
    }

    fn shared(tag: &str) -> (Arc<Shared>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("hub-companion-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let cfg = Config { port: 0, udp_port: 0, name: "Test-PC".into(), version: "0.1.0".into(), devices_file: dir.join("devices.json") };
        (Arc::new(Shared::new(cfg, Arc::new(Fake { dir: dir.clone() }), "123456".into(), 9)), dir)
    }

    fn req(method: &str, path: &str, key: Option<&str>, body: &str) -> Request {
        let auth = key.map(|k| format!("Authorization: Bearer {k}\r\n")).unwrap_or_default();
        let (mut r, _) = http::parse_head(format!("{method} {path} HTTP/1.1\r\n{auth}Content-Length: {}\r\n\r\n", body.len()).as_bytes()).unwrap();
        r.body = body.as_bytes().to_vec();
        r
    }

    fn json_of(r: &Response) -> Value {
        serde_json::from_slice(&r.body).unwrap()
    }

    fn pair(sh: &Arc<Shared>) -> String {
        let r = handle(&req("POST", "/api/pair", None, r#"{"code":"123456","device":"Pixel"}"#), sh);
        json_of(&r)["key"].as_str().unwrap().to_string()
    }

    fn live(sh: &Arc<Shared>, v: Value) {
        let mut d = v;
        d["t"] = json!("live");
        sh.hear(d.to_string().as_bytes());
    }

    #[test]
    fn nothing_but_hello_answers_before_pairing() {
        let (sh, dir) = shared("pair");
        assert_eq!(json_of(&handle(&req("GET", "/api/hello", None, ""), &sh))["name"], "Test-PC");
        assert_eq!(handle(&req("GET", "/api/state", None, ""), &sh).status, 401);
        assert_eq!(handle(&req("GET", "/api/state", Some("00000000000000000000000000000000"), ""), &sh).status, 401);
        assert_eq!(handle(&req("POST", "/api/pair", None, r#"{"code":"999999"}"#), &sh).status, 403);
        let key = pair(&sh);
        assert_eq!(handle(&req("GET", "/api/state", Some(&key), ""), &sh).status, 200);
        // the devices are kept, as hashes
        let kept = std::fs::read_to_string(dir.join("devices.json")).unwrap();
        assert!(kept.contains("Pixel") && !kept.contains(&key));
        assert!(sh.lock().seen.contains_key(&pairing::hash_key(&key)));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_state_is_long_polled_and_answered_as_soon_as_it_changes() {
        let (sh, dir) = shared("poll");
        let key = pair(&sh);
        let v0 = json_of(&handle(&req("GET", "/api/state", Some(&key), ""), &sh))["v"].as_u64().unwrap();
        let (sh2, key2) = (sh.clone(), key.clone());
        let t = std::thread::spawn(move || {
            let started = Instant::now();
            let v = json_of(&handle(&req("GET", &format!("/api/state?after={v0}"), Some(&key2), ""), &sh2));
            (v, started.elapsed())
        });
        std::thread::sleep(Duration::from_millis(150));
        live(&sh, json!({ "clock": 30_000.0 }));
        let (v, took) = t.join().unwrap();
        assert!(v["v"].as_u64().unwrap() > v0);
        assert_eq!(v["game"], true);
        assert!(took < Duration::from_secs(5), "answered at the news, not after the wait: {took:?}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_command_goes_through_the_plugins_queue_and_waits_for_its_ack() {
        let (sh, dir) = shared("cmd");
        let key = pair(&sh);
        // no game: refused at once
        let r = json_of(&handle(&req("POST", "/api/do", Some(&key), r#"{"do":"type_ibis"}"#), &sh));
        assert_eq!(r["error"], "game_not_running");
        live(&sh, json!({ "clock": 30_000.0 }));
        sh.hear(json!({ "t": "keys", "ibis": ["IBIS_1"] }).to_string().as_bytes());
        // a key the bus does not have is refused by the add-on already
        let r = json_of(&handle(&req("POST", "/api/do", Some(&key), r#"{"do":"ibis_key","key":"door_1"}"#), &sh));
        assert_eq!(r["error"], "unknown_key");
        // the plugin acks what it finds in the file
        let (sh2, dir2) = (sh.clone(), dir.clone());
        let plugin = std::thread::spawn(move || {
            for _ in 0..100 {
                std::thread::sleep(Duration::from_millis(20));
                let Ok(text) = std::fs::read_to_string(crate::queue::file_of(&dir2)) else { continue };
                let q: Value = serde_json::from_str(&text).unwrap();
                if let Some(c) = q["cmds"].as_array().and_then(|c| c.last()) {
                    assert_eq!((c["do"].as_str(), c["key"].as_str()), (Some("key"), Some("IBIS_1")));
                    sh2.hear(json!({ "t": "ack", "seq": c["seq"], "ok": true }).to_string().as_bytes());
                    return;
                }
            }
        });
        let r = json_of(&handle(&req("POST", "/api/do", Some(&key), r#"{"do":"ibis_key","key":"IBIS_1"}"#), &sh));
        plugin.join().unwrap();
        assert_eq!(r, json!({ "ok": true }));
        assert_eq!(handle(&req("POST", "/api/do", Some(&key), r#"{"do":"rm"}"#), &sh).status, 400);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_route_is_read_once_per_trip_in_the_games_coordinates_and_the_turn_said() {
        let (sh, dir) = shared("route");
        let key = pair(&sh);
        sh.hear(json!({ "t": "trip", "v": 1, "stops": [{ "name": "Kunsthalle", "x": 1200.0, "y": 300.0 }] }).to_string().as_bytes());
        // the game's place 1000 m east of the add-on's reading of the same tile
        let reference = json!({ "x": 1000.0, "y": 0.0, "tile_x": 0, "tile_y": 0, "local_x": 0.0, "local_y": 0.0 });
        let at = |y: f64| json!({ "clock": 30_000.0, "map": "maps/Test/global.cfg", "ref": reference, "bus": { "x": 1000.0, "y": y }, "duty": { "trip_name": "A-B", "next": 0 } });
        live(&sh, at(100.0));
        for _ in 0..100 {
            if sh.lock().routing.tracker.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let r = json_of(&handle(&req("GET", "/api/route", Some(&key), ""), &sh));
        assert_eq!(r["pts"][2], json!([1200.0, 300.0]));
        live(&sh, at(110.0));
        let s = json_of(&handle(&req("GET", "/api/state", Some(&key), ""), &sh));
        assert_eq!((s["nav"]["maneuver"].as_str(), s["nav"]["distance"].as_f64()), (Some("right"), Some(190.0)), "{}", s["nav"]);
        // the street after the turn is asked of the plugin, and taken from its answer
        let probe = sh.lock().queue.probe.unwrap();
        live(
            &sh,
            json!({ "clock": 30_001.0, "map": "maps/Test/global.cfg", "ref": reference, "bus": { "x": 1000.0, "y": 110.0 }, "duty": { "trip_name": "A-B", "next": 0 }, "probe": { "x": probe[0], "y": probe[1], "name": "Rathausstraße" } }),
        );
        let s = json_of(&handle(&req("GET", "/api/state", Some(&key), ""), &sh));
        assert_eq!(s["nav"]["street"], "Rathausstraße");
        let roads = json_of(&handle(&req("GET", "/api/roads?x=1050&y=10", Some(&key), ""), &sh));
        assert_eq!(roads["roads"][0]["pts"][0], json!([1050.0, 10.0]), "asked and answered in the game's coordinates");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn lane_names_that_are_files_are_no_streets_and_only_home_networks_get_in() {
        assert_eq!(street_name("Rathausstraße"), "Rathausstraße");
        assert_eq!(street_name("Splines\\str_6m.sli"), "");
        assert_eq!(street_name(""), "");
        assert!(lan_peer("192.168.1.20".parse().unwrap()) && lan_peer("10.0.0.3".parse().unwrap()) && lan_peer("fe80::1".parse().unwrap()));
        assert!(!lan_peer("8.8.8.8".parse().unwrap()));
    }
}
