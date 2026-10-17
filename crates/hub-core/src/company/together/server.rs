//! The boss's launcher's company server: plain HTTP on a TCP port of its own, on every address
//! of the machine (the LAN's, a VPN's).
//!
//! **Not in the add-on yet:** the way in from the internet. The old Omsi-Hub put a Cloudflare
//! quick tunnel in front of the server and posted its address on openOMSI's relay; Omsi-Hub
//! needs a relay of its own for that ([`Server::open_tunnel`] leaves the tunnel off). Until
//! then friends play over the LAN or a VPN.
//!
//! `POST /together` with a `wire::Request` is answered by the handler the launcher gives (JSON
//! in, JSON out); `GET /together` says the server is there. One short connection a message, a
//! thread each (a handful of players), bodies of at most `MAX_BODY`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The port the server tries first (the next free one after it, else any).
pub const DEFAULT_PORT: u16 = 27045;
/// The largest request taken (a duty's trip reports with their stops).
const MAX_BODY: usize = 1 << 20;
const MAX_HEAD: usize = 16 << 10;
/// At most this many connections at once.
const MAX_OPEN: usize = 48;

/// What answers a request's body (JSON) with the answer's (JSON).
pub type Handler = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// Where the tunnel stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TunnelState {
    /// Not asked for, or not available (Omsi-Hub has no relay of its own yet).
    Off,
    /// cloudflared is fetched or starting.
    Starting,
    /// Reached at this address.
    Ready(String),
    /// No cloudflared could be had or started.
    Failed,
}

/// A running server; it stops when dropped (its tunnel with it).
pub struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    tunnel: Arc<Mutex<TunnelState>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // (the accept loop wakes to see it)
        let _ = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], self.port)), Duration::from_millis(300));
    }
}

impl Server {
    /// Listen on `port` (0: `DEFAULT_PORT` or one of the ten after it, else any free one) on
    /// every address (`local_only`: the loopback alone, for tests).
    pub fn start(port: u16, local_only: bool, handler: Handler) -> std::io::Result<Server> {
        let host = if local_only { [127, 0, 0, 1] } else { [0, 0, 0, 0] };
        let tries: Vec<u16> = if port == 0 { (DEFAULT_PORT..DEFAULT_PORT + 10).chain([0]).collect() } else { vec![port] };
        let mut last = None;
        let mut listener = None;
        for p in tries {
            match TcpListener::bind(SocketAddr::from((host, p))) {
                Ok(l) => {
                    listener = Some(l);
                    break;
                }
                Err(e) => last = Some(e),
            }
        }
        let listener = match listener {
            Some(l) => l,
            None => return Err(last.unwrap_or_else(|| std::io::Error::other("no port"))),
        };
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let open = Arc::new(AtomicUsize::new(0));
        let s = stop.clone();
        std::thread::Builder::new().name("company server".into()).spawn(move || {
            for conn in listener.incoming() {
                if s.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(conn) = conn else { continue };
                if open.load(Ordering::Relaxed) >= MAX_OPEN {
                    continue;
                }
                open.fetch_add(1, Ordering::Relaxed);
                let (h, o) = (handler.clone(), open.clone());
                let spawned = std::thread::Builder::new().name("company client".into()).spawn(move || {
                    serve(conn, &h);
                    o.fetch_sub(1, Ordering::Relaxed);
                });
                if spawned.is_err() {
                    open.fetch_sub(1, Ordering::Relaxed);
                }
            }
        })?;
        Ok(Server { port, stop, tunnel: Arc::new(Mutex::new(TunnelState::Off)) })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn tunnel(&self) -> TunnelState {
        self.tunnel.lock().map(|t| t.clone()).unwrap_or(TunnelState::Failed)
    }

    /// Open the way in from the internet. **Not available yet:** it needs a relay of
    /// Omsi-Hub's own (the old one was openOMSI's), so the tunnel stays off and friends join
    /// over the LAN or a VPN.
    pub fn open_tunnel(&self) {
        if let Ok(mut t) = self.tunnel.lock() {
            *t = TunnelState::Off;
        }
    }
}

/// One connection: a request read, answered, closed.
fn serve(mut conn: TcpStream, handler: &Handler) {
    let _ = conn.set_read_timeout(Some(Duration::from_secs(15)));
    let _ = conn.set_write_timeout(Some(Duration::from_secs(15)));
    let (status, body) = match read_request(&mut conn) {
        Ok((method, path, body)) => match (method.as_str(), path.split('?').next().unwrap_or("")) {
            ("POST", "/together") => ("200 OK", handler(&String::from_utf8_lossy(&body))),
            ("GET", "/together") => ("200 OK", r#"{"app":"omsihub-together","v":1}"#.to_string()),
            _ => ("404 Not Found", r#"{"ok":false}"#.to_string()),
        },
        Err(code) => (code, r#"{"ok":false}"#.to_string()),
    };
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len());
    let _ = conn.write_all(head.as_bytes());
    let _ = conn.write_all(body.as_bytes());
    let _ = conn.flush();
}

/// The request's method, path and body; Err the status to answer.
fn read_request(conn: &mut TcpStream) -> Result<(String, String, Vec<u8>), &'static str> {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_HEAD {
            return Err("431 Request Header Fields Too Large");
        }
        let n = conn.read(&mut chunk).map_err(|_| "408 Request Timeout")?;
        if n == 0 {
            return Err("400 Bad Request");
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("").to_string();
    let path = first.next().unwrap_or("").to_string();
    let length = lines.filter_map(|l| l.split_once(':')).find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length")).and_then(|(_, v)| v.trim().parse::<usize>().ok()).unwrap_or(0);
    if length > MAX_BODY {
        return Err("413 Payload Too Large");
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        let n = conn.read(&mut chunk).map_err(|_| "408 Request Timeout")?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(length);
    Ok((method, path, body))
}
