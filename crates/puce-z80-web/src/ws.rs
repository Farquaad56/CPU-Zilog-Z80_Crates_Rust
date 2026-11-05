//! WebSocket part of the server (`/ws`, protocol 31): one thread per client, one shared session.

use crate::http::Ctx;
use puce_z80_machines::session::{hello, Command, ViewParams};
use serde::Deserialize;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tungstenite::handshake::derive_accept_key;
use tungstenite::protocol::Role;
use tungstenite::{Error as WsError, Message, WebSocket};

/// Read timeout of the client loop: also the granularity of the state pushes.
const TICK: Duration = Duration::from_millis(10);
/// Minimum time between two pushed states (30 per second at most).
const MIN_PUSH: Duration = Duration::from_millis(33);
/// How long to wait for the session to apply a command before answering with a state.
const APPLY_WAIT: Duration = Duration::from_millis(300);

fn one() -> u32 {
    1
}

/// A message from the page (`cmd` selects the variant).
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum ClientMsg {
    /// Ask for hello, roms and state.
    Hello,
    /// Ask for the ROM list.
    ListRoms,
    /// Load a listed ROM.
    Load {
        /// File name of the ROM.
        name: String,
    },
    /// Load bytes sent by the page.
    Upload {
        /// File name shown to the user.
        name: String,
        /// Base64 content.
        data_b64: String,
    },
    /// Start the simulation.
    Run,
    /// Stop the simulation.
    Pause,
    /// Execute `n` instructions.
    Step {
        /// Number of instructions (default 1).
        #[serde(default = "one")]
        n: u32,
    },
    /// Reload the current ROM.
    Reset,
    /// Select a speed.
    Speed {
        /// Index 0..7.
        idx: u8,
    },
    /// Add a breakpoint.
    BpAdd {
        /// Address.
        addr: u32,
    },
    /// Remove a breakpoint.
    BpDel {
        /// Address.
        addr: u32,
    },
    /// Change the observation window.
    View {
        /// Memory base (used when `follow` is `man`).
        mem_base: Option<u32>,
        /// Disassembly rows.
        dis_rows: Option<u32>,
        /// Trace rows.
        trace_rows: Option<u32>,
        /// Stack rows.
        stack_rows: Option<u32>,
        /// pc, sp, hl, de, bc, wr or man.
        follow: Option<String>,
    },
    /// Clear the console.
    ConsoleClear,
}

/// What a message produced: replies to send now, and whether the session was commanded.
#[derive(Default)]
pub struct Outcome {
    /// JSON texts to send to this client, in order.
    pub replies: Vec<String>,
    /// A state must follow once the session has applied the command.
    pub state_after: bool,
}

/// `{"type":"error","message":...}`.
pub fn error_text(message: &str) -> String {
    serde_json::json!({"type": "error", "message": message}).to_string()
}

fn addr16(v: u32) -> Result<u16, String> {
    u16::try_from(v).map_err(|_| format!("address {} is outside 0..65535", v))
}

/// Largest number of rows a `view` may ask for.
const MAX_DIS_ROWS: u32 = 100;
const MAX_TRACE_ROWS: u32 = 64;
const MAX_STACK_ROWS: u32 = 64;

/// Apply a parsed message.
pub fn handle(ctx: &Ctx, msg: ClientMsg, view: &mut ViewParams) -> Outcome {
    let mut out = Outcome::default();
    let send = |c: Command, out: &mut Outcome| {
        ctx.session.send(c);
        out.state_after = true;
    };
    match msg {
        ClientMsg::Hello => {
            out.replies.push(hello().to_string());
            out.replies.push(ctx.roms_message());
            out.state_after = true;
        }
        ClientMsg::ListRoms => out.replies.push(ctx.roms_message()),
        ClientMsg::Load { name } => match ctx.load_listed(&name) {
            Ok(c) => send(c, &mut out),
            Err(e) => out.replies.push(error_text(&e)),
        },
        ClientMsg::Upload { name, data_b64 } => match ctx.upload(&name, &data_b64) {
            Ok(c) => send(c, &mut out),
            Err(e) => out.replies.push(error_text(&e)),
        },
        ClientMsg::Run => send(Command::Run, &mut out),
        ClientMsg::Pause => send(Command::Pause, &mut out),
        ClientMsg::Step { n } => send(Command::Step(n), &mut out),
        ClientMsg::Reset => send(Command::Reset, &mut out),
        ClientMsg::Speed { idx } => {
            if idx > 7 {
                out.replies.push(error_text("speed idx must be 0..7"));
            } else {
                send(Command::Speed(idx), &mut out);
            }
        }
        ClientMsg::BpAdd { addr } => match addr16(addr) {
            Ok(a) => send(Command::BpAdd(a), &mut out),
            Err(e) => out.replies.push(error_text(&e)),
        },
        ClientMsg::BpDel { addr } => match addr16(addr) {
            Ok(a) => send(Command::BpDel(a), &mut out),
            Err(e) => out.replies.push(error_text(&e)),
        },
        ClientMsg::View {
            mem_base,
            dis_rows,
            trace_rows,
            stack_rows,
            follow,
        } => {
            let mut v = view.clone();
            // Row counts are clamped to their maximum rather than refused.
            let clamp = |x: Option<u32>, max: u32, dst: &mut u32| {
                if let Some(x) = x {
                    *dst = x.min(max);
                }
            };
            let r = (|| {
                if let Some(b) = mem_base {
                    v.mem_base = addr16(b)? & !0xF;
                }
                clamp(dis_rows, MAX_DIS_ROWS, &mut v.dis_rows);
                clamp(trace_rows, MAX_TRACE_ROWS, &mut v.trace_rows);
                clamp(stack_rows, MAX_STACK_ROWS, &mut v.stack_rows);
                if let Some(f) = follow {
                    if !["pc", "sp", "hl", "de", "bc", "wr", "man"].contains(&f.as_str()) {
                        return Err(format!("unknown follow '{}'", f));
                    }
                    v.follow = f;
                }
                Ok::<(), String>(())
            })();
            match r {
                Ok(()) => {
                    *view = v.clone();
                    send(Command::View(v), &mut out);
                }
                Err(e) => out.replies.push(error_text(&e)),
            }
        }
        ClientMsg::ConsoleClear => send(Command::ConsoleClear, &mut out),
    }
    out
}

/// Answer the upgrade request and serve the client until it goes away.
pub fn serve_client(mut stream: TcpStream, key: &str, ctx: Arc<Ctx>) {
    use std::io::Write;
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        derive_accept_key(key.trim().as_bytes())
    );
    if stream.write_all(response.as_bytes()).is_err() || stream.flush().is_err() {
        return;
    }
    let _ = stream.set_nodelay(true);
    if stream.set_read_timeout(Some(TICK)).is_err() {
        return;
    }
    let mut ws = WebSocket::from_raw_socket(stream, Role::Server, None);
    let mut view = ViewParams::default();
    let mut cursor = 0usize;
    let mut last_sent: Option<puce_z80_machines::session::Snapshot> = None;
    let mut last_push = Instant::now() - MIN_PUSH;
    let mut last_gen = 0u64;
    // Generation to pass before the state that answers a command is sent, and its deadline.
    let mut waiting: Option<(u64, Instant)> = None;
    let mut hello_seen = false;

    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                let outcome = match serde_json::from_str::<ClientMsg>(text.as_str()) {
                    Ok(msg) => {
                        if matches!(msg, ClientMsg::Hello) {
                            hello_seen = true;
                        }
                        handle(&ctx, msg, &mut view)
                    }
                    Err(e) => Outcome {
                        replies: vec![error_text(&format!("invalid message: {}", e))],
                        state_after: false,
                    },
                };
                for r in outcome.replies {
                    if ws.send(Message::text(r)).is_err() {
                        return;
                    }
                }
                if outcome.state_after {
                    waiting = Some((ctx.session.generation(), Instant::now() + APPLY_WAIT));
                }
            }
            Ok(Message::Close(_)) => return,
            Ok(_) => {}
            Err(WsError::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return,
        }

        let gen = ctx.session.generation();
        let due = match waiting {
            Some((g, deadline)) => gen > g || Instant::now() >= deadline,
            None => hello_seen && gen != last_gen && last_push.elapsed() >= MIN_PUSH,
        };
        if !due {
            continue;
        }
        let mut snap = ctx.session.snapshot();
        let (text, pos) = ctx.session.console_since(cursor);
        snap.console_delta = text;
        let changed = match &last_sent {
            Some(prev) => {
                let mut a = prev.clone();
                let mut b = snap.clone();
                a.ips = 0.0;
                a.mhz = 0.0;
                a.console_delta.clear();
                b.ips = 0.0;
                b.mhz = 0.0;
                b.console_delta.clear();
                a != b || !snap.console_delta.is_empty()
            }
            None => true,
        };
        // After a command a state is always sent; a plain push only when something changed.
        if waiting.is_some() || changed {
            let body = serde_json::to_string(&snap).unwrap_or_default();
            if ws.send(Message::text(body)).is_err() {
                return;
            }
            cursor = pos;
            last_sent = Some(snap);
            last_push = Instant::now();
        }
        last_gen = gen;
        waiting = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{start_with, ONE_AT_A_TIME};
    use base64::Engine;
    use serde_json::Value;
    use std::path::PathBuf;
    use tungstenite::stream::MaybeTlsStream;

    type Client = WebSocket<MaybeTlsStream<TcpStream>>;

    /// LD E,'A'; LD C,2; CALL 5; JP 0
    const PRINT_A: [u8; 10] = [0x1E, b'A', 0x0E, 0x02, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00];
    /// JR $
    const SPIN: [u8; 2] = [0x18, 0xFE];

    fn server() -> (crate::http::Server, std::net::SocketAddr) {
        server_with(PathBuf::from("no-such-roms-dir"))
    }

    fn server_with(roms: PathBuf) -> (crate::http::Server, std::net::SocketAddr) {
        let ctx = Arc::new(Ctx::new(roms));
        let srv = start_with(0, ctx).unwrap();
        let addr = srv.addr();
        (srv, addr)
    }

    /// A temporary tree: roms/{a.com, zexdoc.cim, prelim.com, notes.txt, sub/b.bin}, x.com above.
    fn rom_tree(tag: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("puce_z80_web_{}_{}", std::process::id(), tag));
        let roms = base.join("roms");
        std::fs::create_dir_all(roms.join("sub")).unwrap();
        std::fs::write(roms.join("a.com"), PRINT_A).unwrap();
        std::fs::write(roms.join("zexdoc.cim"), PRINT_A).unwrap();
        std::fs::write(roms.join("prelim.com"), PRINT_A).unwrap();
        std::fs::write(roms.join("notes.txt"), b"ignored").unwrap();
        std::fs::write(roms.join("sub").join("b.bin"), [0x3E, 0x07, 0x76]).unwrap();
        std::fs::write(base.join("x.com"), PRINT_A).unwrap();
        roms
    }

    fn connect(addr: std::net::SocketAddr) -> Client {
        // A reset before the handshake completes is retried (see ONE_AT_A_TIME).
        let mut last = None;
        for _ in 0..5 {
            match tungstenite::connect(format!("ws://{}/ws", addr)) {
                Ok((mut ws, _)) => {
                    if let MaybeTlsStream::Plain(s) = ws.get_mut() {
                        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                    }
                    return ws;
                }
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
        panic!("cannot connect: {:?}", last);
    }

    fn recv(ws: &mut Client) -> Value {
        loop {
            match ws.read().expect("message") {
                Message::Text(t) => return serde_json::from_str(t.as_str()).unwrap(),
                Message::Ping(_) | Message::Pong(_) => {}
                other => panic!("unexpected {:?}", other),
            }
        }
    }

    fn send(ws: &mut Client, text: &str) {
        ws.send(Message::text(text)).unwrap();
    }

    fn recv_state_where(ws: &mut Client, f: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..200 {
            let v = recv(ws);
            if v["type"] == "state" && f(&v) {
                return v;
            }
        }
        panic!("no matching state");
    }

    fn upload(ws: &mut Client, bytes: &[u8]) {
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        send(
            ws,
            &format!(r#"{{"cmd":"upload","name":"t.com","data_b64":"{}"}}"#, b64),
        );
    }

    #[test]
    fn e14_04_handshake() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let _ws = connect(addr);
    }

    #[test]
    fn e14_04_hello_reply() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        send(&mut ws, r#"{"cmd":"hello"}"#);
        let h = recv(&mut ws);
        assert_eq!(h["type"], "hello");
        assert!(h["version"].is_string());
        assert_eq!(recv(&mut ws)["type"], "roms");
        let s = recv(&mut ws);
        assert_eq!(s["type"], "state");
        assert_eq!(s["status"], "idle");
    }

    #[test]
    fn e14_04_bad_json() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        send(&mut ws, "this is not json");
        assert_eq!(recv(&mut ws)["type"], "error");
        send(&mut ws, r#"{"cmd":"no_such_command"}"#);
        assert_eq!(recv(&mut ws)["type"], "error");
        send(&mut ws, r#"{"cmd":"speed","idx":99}"#);
        assert_eq!(recv(&mut ws)["type"], "error");
        // The connection is still open.
        send(&mut ws, r#"{"cmd":"hello"}"#);
        assert_eq!(recv(&mut ws)["type"], "hello");
    }

    #[test]
    fn e14_04_two_clients() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut a = connect(addr);
        let mut b = connect(addr);
        send(&mut a, r#"{"cmd":"hello"}"#);
        send(&mut b, r#"{"cmd":"hello"}"#);
        recv_state_where(&mut a, |_| true);
        recv_state_where(&mut b, |_| true);
        upload(&mut a, &PRINT_A);
        send(&mut a, r#"{"cmd":"step","n":2}"#);
        let sa = recv_state_where(&mut a, |v| v["n"] == 2);
        let sb = recv_state_where(&mut b, |v| v["n"] == 2);
        assert_eq!(sa["regs"]["pc"], sb["regs"]["pc"]);
    }

    #[test]
    fn e14_04_state_rate() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        send(&mut ws, r#"{"cmd":"hello"}"#);
        recv_state_where(&mut ws, |_| true);
        upload(&mut ws, &SPIN);
        send(&mut ws, r#"{"cmd":"speed","idx":7}"#);
        send(&mut ws, r#"{"cmd":"run"}"#);
        recv_state_where(&mut ws, |v| v["status"] == "running");
        if let MaybeTlsStream::Plain(s) = ws.get_mut() {
            s.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
        }
        let start = Instant::now();
        let mut count = 0;
        while start.elapsed() < Duration::from_secs(1) {
            match ws.read() {
                Ok(Message::Text(_)) => count += 1,
                Ok(_) => {}
                Err(WsError::Io(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
        }
        assert!((10..=31).contains(&count), "{} states in one second", count);
        send(&mut ws, r#"{"cmd":"pause"}"#);
    }

    fn rom<'a>(roms: &'a Value, name: &str) -> Option<&'a Value> {
        roms["roms"].as_array()?.iter().find(|r| r["name"] == name)
    }

    #[test]
    fn e14_05_list_finds_roms() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = rom_tree("list");
        let (_srv, addr) = server_with(dir.clone());
        let mut ws = connect(addr);
        send(&mut ws, r#"{"cmd":"list_roms"}"#);
        let r = recv(&mut ws);
        assert_eq!(r["type"], "roms");
        assert_eq!(r["roms"].as_array().unwrap().len(), 4, "{}", r);
        assert_eq!(rom(&r, "a.com").unwrap()["size"], 10);
        assert_eq!(rom(&r, "a.com").unwrap()["total_tests"], 0);
        assert_eq!(rom(&r, "zexdoc.cim").unwrap()["total_tests"], 67);
        assert_eq!(rom(&r, "prelim.com").unwrap()["total_tests"], 1);
        assert!(rom(&r, "sub/b.bin").is_some());
        assert!(rom(&r, "notes.txt").is_none());
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn e14_05_load_ok() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = rom_tree("load");
        let (_srv, addr) = server_with(dir.clone());
        let mut ws = connect(addr);
        send(&mut ws, r#"{"cmd":"load","name":"a.com"}"#);
        let s = recv_state_where(&mut ws, |v| v["rom"]["name"] == "a.com");
        assert_eq!(s["rom"]["kind"], "cpm");
        assert_eq!(s["rom"]["size"], 10);
        assert_eq!(s["status"], "idle");
        assert_eq!(s["regs"]["pc"], 0x0100);
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn e14_05_path_traversal_rejected() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = rom_tree("trav");
        let (_srv, addr) = server_with(dir.clone());
        let mut ws = connect(addr);
        for name in [
            "../x.com",
            "sub/../../x.com",
            "/x.com",
            "..\\x.com",
            "missing.com",
        ] {
            let msg = serde_json::json!({"cmd": "load", "name": name}).to_string();
            send(&mut ws, &msg);
            let r = recv(&mut ws);
            assert_eq!(r["type"], "error", "{} was not rejected: {}", name, r);
        }
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn e14_05_upload_b64() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        upload(&mut ws, &PRINT_A);
        let s = recv_state_where(&mut ws, |v| v["rom"]["name"] == "t.com");
        assert_eq!(s["rom"]["size"], 10);
        send(
            &mut ws,
            r#"{"cmd":"upload","name":"bad.com","data_b64":"***"}"#,
        );
        assert_eq!(recv(&mut ws)["type"], "error");
    }

    #[test]
    fn e14_05_upload_too_big() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        upload(&mut ws, &vec![0u8; (1 << 20) + 1]);
        let r = recv(&mut ws);
        assert_eq!(r["type"], "error", "{}", r);
        assert!(r["message"].as_str().unwrap().contains("limit"));
    }

    #[test]
    fn e14_05_raw_kind() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = rom_tree("raw");
        let (_srv, addr) = server_with(dir.clone());
        let mut ws = connect(addr);
        send(&mut ws, r#"{"cmd":"load","name":"sub/b.bin"}"#);
        let s = recv_state_where(&mut ws, |v| v["rom"]["name"] == "sub/b.bin");
        assert_eq!(s["rom"]["kind"], "raw");
        assert_eq!(s["regs"]["pc"], 0);
        send(&mut ws, r#"{"cmd":"run"}"#);
        let s = recv_state_where(&mut ws, |v| v["status"] == "halted");
        assert_eq!(s["regs"]["af"].as_u64().unwrap() >> 8, 7);
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    fn load_spin(ws: &mut Client) {
        upload(ws, &SPIN);
        recv_state_where(ws, |v| v["rom"]["name"] == "t.com");
    }

    #[test]
    fn e14_06_step_advances() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        load_spin(&mut ws);
        send(&mut ws, r#"{"cmd":"step","n":5}"#);
        let s = recv_state_where(&mut ws, |v| v["n"] == 5);
        assert_eq!(s["status"], "idle");
    }

    #[test]
    fn e14_06_run_pause() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        load_spin(&mut ws);
        send(&mut ws, r#"{"cmd":"speed","idx":5}"#);
        send(&mut ws, r#"{"cmd":"run"}"#);
        recv_state_where(&mut ws, |v| {
            v["status"] == "running" && v["n"].as_u64().unwrap() > 0
        });
        send(&mut ws, r#"{"cmd":"pause"}"#);
        let a = recv_state_where(&mut ws, |v| v["status"] == "idle");
        std::thread::sleep(Duration::from_millis(150));
        send(&mut ws, r#"{"cmd":"step","n":0}"#);
        let b = recv_state_where(&mut ws, |v| v["status"] == "idle");
        assert_eq!(a["n"], b["n"]);
    }

    #[test]
    fn e14_06_speed_idx_range() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        send(&mut ws, r#"{"cmd":"speed","idx":9}"#);
        assert_eq!(recv(&mut ws)["type"], "error");
        send(&mut ws, r#"{"cmd":"speed","idx":7}"#);
        recv_state_where(&mut ws, |_| true);
    }

    #[test]
    fn e14_06_bp_roundtrip() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        load_spin(&mut ws);
        send(&mut ws, r#"{"cmd":"bp_add","addr":258}"#);
        let s = recv_state_where(&mut ws, |v| {
            v["bps"].as_array().is_some_and(|b| !b.is_empty())
        });
        assert_eq!(s["bps"], serde_json::json!([258]));
        send(&mut ws, r#"{"cmd":"bp_del","addr":258}"#);
        recv_state_where(&mut ws, |v| {
            v["bps"].as_array().is_some_and(|b| b.is_empty())
        });
        send(&mut ws, r#"{"cmd":"bp_add","addr":70000}"#);
        assert_eq!(recv(&mut ws)["type"], "error");
    }

    #[test]
    fn e14_06_view_bounds() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        load_spin(&mut ws);
        send(
            &mut ws,
            r#"{"cmd":"view","dis_rows":1000,"trace_rows":1000,"stack_rows":1000,"mem_base":4103,"follow":"man"}"#,
        );
        let s = recv_state_where(&mut ws, |v| {
            v["dis"].as_array().is_some_and(|d| d.len() == 100)
        });
        assert!(s["trace"].as_array().unwrap().len() <= 64);
        assert!(s["stack"].as_array().unwrap().len() <= 64);
        assert_eq!(s["mem"]["base"], 4096);
        send(&mut ws, r#"{"cmd":"view","follow":"nowhere"}"#);
        assert_eq!(recv(&mut ws)["type"], "error");
    }

    #[test]
    fn e14_06_console_clear() {
        let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let (_srv, addr) = server();
        let mut ws = connect(addr);
        upload(&mut ws, &PRINT_A);
        recv_state_where(&mut ws, |v| v["rom"]["name"] == "t.com");
        send(&mut ws, r#"{"cmd":"run"}"#);
        let mut text = String::new();
        loop {
            let s = recv_state_where(&mut ws, |_| true);
            text.push_str(s["console_delta"].as_str().unwrap_or(""));
            if s["status"] == "done" {
                break;
            }
        }
        assert!(text.contains('A'), "{:?}", text);
        send(&mut ws, r#"{"cmd":"console_clear"}"#);
        let s = recv_state_where(&mut ws, |_| true);
        assert_eq!(s["console_delta"].as_str().unwrap_or(""), "");
        send(&mut ws, r#"{"cmd":"step","n":0}"#);
        let s = recv_state_where(&mut ws, |_| true);
        assert_eq!(s["console_delta"].as_str().unwrap_or(""), "");
    }
}
