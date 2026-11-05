//! HTTP part of the server: the embedded page and `/health`. Listens on 127.0.0.1 only.
//!
//! Plain `std::net` (no HTTP crate): a request is a head ending with an empty line, an answer
//! carries a Content-Length and closes the connection. `/ws` is added by the WebSocket part.

use base64::Engine;
use puce_z80_machines::session::{Command, Session};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// The page, embedded so that `docs/frontend/index.html` stays the single source.
const INDEX_HTML: &str = include_str!("../../../docs/frontend/index.html");

/// How often the accept loop looks at the stop flag.
const POLL: Duration = Duration::from_millis(10);
/// A client that sends nothing for this long is dropped.
const READ_TIMEOUT: Duration = Duration::from_secs(5);
/// Largest accepted request head.
const MAX_HEAD: usize = 16 * 1024;

/// Largest file accepted from the page (1 MiB).
const MAX_UPLOAD: usize = 1 << 20;
/// Largest CP/M program (it lives between 0100h and FFFFh).
const MAX_COM: usize = 0xFF00;
/// Largest raw image (all of memory).
const MAX_RAW: usize = 0x10000;
/// Directory levels searched for ROMs.
const MAX_DEPTH: usize = 3;

/// What every client of the server shares: the simulation and the ROM directory.
pub struct Ctx {
    /// The simulation session.
    pub session: Session,
    /// Root of the ROM files.
    pub roms_dir: PathBuf,
}

impl Ctx {
    /// A fresh session and the ROM directory to list.
    pub fn new(roms_dir: PathBuf) -> Ctx {
        Ctx {
            session: Session::spawn(),
            roms_dir,
        }
    }

    /// ROM files (`.com`, `.cim`, `.bin`) below the ROM directory: (name, size).
    pub fn list_roms(&self) -> Vec<(String, u64)> {
        fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<(String, u64)>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for e in entries.flatten() {
                let path = e.path();
                let Ok(meta) = e.metadata() else { continue };
                if meta.is_dir() {
                    if depth < MAX_DEPTH {
                        walk(root, &path, depth + 1, out);
                    }
                    continue;
                }
                let ext = path
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !["com", "cim", "bin"].contains(&ext.as_str()) {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(root) {
                    let name = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/");
                    out.push((name, meta.len()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.roms_dir, &self.roms_dir, 0, &mut out);
        out.sort();
        out
    }

    /// The `roms` message.
    pub fn roms_message(&self) -> String {
        let roms: Vec<_> = self
            .list_roms()
            .into_iter()
            .map(|(name, size)| {
                let stem = name.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
                // Result lines printed by each suite (hard-coded by file name).
                let total = if stem.starts_with("zexdoc") || stem.starts_with("zexall") {
                    67
                } else if stem.starts_with("prelim") {
                    1
                } else {
                    0
                };
                serde_json::json!({"name": name, "size": size, "total_tests": total})
            })
            .collect();
        serde_json::json!({"type": "roms", "roms": roms}).to_string()
    }

    /// The `Load` command for a ROM that `list_roms` knows.
    pub fn load_listed(&self, name: &str) -> Result<Command, String> {
        if !self.list_roms().iter().any(|(n, _)| n == name) {
            return Err(format!("unknown ROM '{}'", name));
        }
        let path = self.roms_dir.join(name);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        program(name, bytes)
    }

    /// The `Load` command for bytes sent by the page.
    pub fn upload(&self, name: &str, data_b64: &str) -> Result<Command, String> {
        if name.is_empty() || name.len() > 128 {
            return Err("upload needs a name of 1 to 128 characters".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data_b64.trim())
            .map_err(|e| format!("data_b64 is not valid base64: {}", e))?;
        program(name, bytes)
    }
}

fn program(name: &str, bytes: Vec<u8>) -> Result<Command, String> {
    if bytes.is_empty() {
        return Err(format!("'{}' is empty", name));
    }
    if bytes.len() > MAX_UPLOAD {
        return Err(format!(
            "'{}' is {} bytes: the limit is {} (1 MiB)",
            name,
            bytes.len(),
            MAX_UPLOAD
        ));
    }
    let (limit, what) = if name.to_ascii_lowercase().ends_with(".bin") {
        (MAX_RAW, "a raw image")
    } else {
        (MAX_COM, "a CP/M program")
    };
    if bytes.len() > limit {
        return Err(format!(
            "'{}' is {} bytes: {} holds at most {}",
            name,
            bytes.len(),
            what,
            limit
        ));
    }
    Ok(Command::Load {
        name: name.to_string(),
        bytes,
    })
}

/// A running HTTP server.
pub struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// The address the server listens on (always 127.0.0.1).
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop accepting requests and wait for the accept thread.
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.wait();
    }

    /// Block until the accept thread ends (after [`Server::shutdown`] from another handle).
    pub fn wait(&mut self) {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Start listening on `127.0.0.1:port` (0 picks a free port) with the default ROM directory.
pub fn start(port: u16) -> Result<Server, String> {
    start_with(port, Arc::new(Ctx::new(PathBuf::from("tests/roms"))))
}

/// Start listening on `127.0.0.1:port` for the given shared state.
pub fn start_with(port: u16, ctx: Arc<Ctx>) -> Result<Server, String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("cannot listen on 127.0.0.1:{}: {}", port, e))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("cannot read the listening address: {}", e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("cannot configure the listener: {}", e))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let thread = std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let ctx = Arc::clone(&ctx);
                    std::thread::spawn(move || serve(stream, ctx));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(POLL),
                Err(_) => break,
            }
        }
    });
    Ok(Server {
        addr,
        stop,
        thread: Some(thread),
    })
}

/// Read the request head (up to the empty line); `None` when the client sent nothing usable.
fn read_head(stream: &mut TcpStream) -> Option<String> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 1024];
    while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 || raw.len() + n > MAX_HEAD {
            return None;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    Some(String::from_utf8_lossy(&raw).into_owned())
}

/// Write one complete answer and close the connection.
fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    // A client that went away is not an error of the server.
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
}

/// Answer one connection.
fn serve(mut stream: TcpStream, ctx: Arc<Ctx>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let Some(head) = read_head(&mut stream) else {
        return;
    };
    let mut line = head.lines().next().unwrap_or("").split_whitespace();
    let method = line.next().unwrap_or("");
    let target = line.next().unwrap_or("");
    let path = target.split('?').next().unwrap_or("");
    let header = |name: &str| -> Option<String> {
        head.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
    };
    if method == "GET" && path == "/ws" {
        let upgrade = header("upgrade").unwrap_or_default().to_ascii_lowercase();
        match (upgrade.as_str(), header("sec-websocket-key")) {
            ("websocket", Some(key)) => crate::ws::serve_client(stream, &key, ctx),
            _ => respond(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                "a WebSocket upgrade is expected on /ws",
            ),
        }
        return;
    }
    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX_HTML,
        ),
        ("GET", "/health") => {
            let body = serde_json::json!({"ok": true, "version": env!("CARGO_PKG_VERSION")});
            respond(&mut stream, "200 OK", "application/json", &body.to_string())
        }
        _ => respond(&mut stream, "404 Not Found", "text/plain", "not found"),
    }
}

/// Tests that open sockets run one at a time: simultaneous loopback connections from several
/// tests are sometimes reset on the Windows development host.
#[cfg(test)]
pub(crate) static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::ONE_AT_A_TIME;
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    /// Minimal HTTP client: returns (status line, body). It stops reading once the announced
    /// Content-Length has arrived, so a reset after the answer does not matter.
    fn get(addr: SocketAddr, path: &str) -> (String, String) {
        // GET is idempotent: a connection reset before any byte arrived is retried.
        let mut last = (String::new(), String::new());
        for _ in 0..5 {
            last = get_once(addr, path);
            if !last.0.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        last
    }

    fn get_once(addr: SocketAddr, path: &str) -> (String, String) {
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            path
        );
        s.write_all(request.as_bytes()).unwrap();
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let text = String::from_utf8_lossy(&raw).into_owned();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let want = head
                    .lines()
                    .find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if body.len() >= want {
                    let status = head.lines().next().unwrap_or("").to_string();
                    return (status, body.to_string());
                }
            }
            match s.read(&mut buf) {
                Ok(0) | Err(_) => {
                    let text = String::from_utf8_lossy(&raw).into_owned();
                    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
                    return (
                        head.lines().next().unwrap_or("").to_string(),
                        body.to_string(),
                    );
                }
                Ok(n) => raw.extend_from_slice(&buf[..n]),
            }
        }
    }

    #[test]
    fn e14_03_index_served() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let srv = start(0).unwrap();
        let (status, body) = get(srv.addr(), "/");
        assert!(status.contains("200"), "{}", status);
        assert!(body.contains("Zilog Z80 Emulator"));
        let (status, _) = get(srv.addr(), "/index.html");
        assert!(status.contains("200"));
    }

    #[test]
    fn e14_03_health() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let srv = start(0).unwrap();
        let (status, body) = get(srv.addr(), "/health");
        assert!(status.contains("200"));
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["version"].is_string());
    }

    #[test]
    fn e14_03_not_found() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let srv = start(0).unwrap();
        let (status, _) = get(srv.addr(), "/x");
        assert!(status.contains("404"), "{}", status);
    }

    #[test]
    fn e14_03_bind_localhost() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let srv = start(0).unwrap();
        assert_eq!(srv.addr().ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn e14_03_port_in_use() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let a = start(0).unwrap();
        let b = start(a.addr().port());
        let err = b.err().expect("second server on the same port must fail");
        assert!(err.contains("cannot listen"), "{}", err);
    }
}
