//! End-to-end: a WebSocket client drives the real `z80-web` binary and the answers
//! are compared with the crate used directly.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use puce_z80::disasm::disassemble;
use puce_z80_machines::cpm::CpmMachine;
use puce_z80_machines::session::alu_name;
use serde_json::Value;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

/// LD A,5 ; LD B,3 ; ADD A,B ; LD E,A ; LD C,2 ; CALL 5 (prints E) ; JP 0
const PROGRAM: [u8; 15] = [
    0x3E, 0x05, 0x06, 0x03, 0x80, 0x5F, 0x0E, 0x02, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00, 0x00,
];

type Client = WebSocket<MaybeTlsStream<std::net::TcpStream>>;

/// A running server process plus its ROM directory; both are cleaned up on drop.
struct Server {
    child: Child,
    port: u16,
    dir: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start(tag: &str) -> Server {
    let dir = std::env::temp_dir().join(format!("puce_z80_e2e_{}_{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("prog.com"), PROGRAM).unwrap();
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let child = Command::new(env!("CARGO_BIN_EXE_z80-web"))
        .args(["--port", &port.to_string(), "--roms"])
        .arg(&dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn z80-web");
    Server { child, port, dir }
}

fn connect(srv: &Server) -> Client {
    let url = format!("ws://127.0.0.1:{}/ws", srv.port);
    let t0 = Instant::now();
    loop {
        match tungstenite::connect(url.as_str()) {
            Ok((mut ws, _)) => {
                if let MaybeTlsStream::Plain(s) = ws.get_mut() {
                    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                }
                return ws;
            }
            Err(e) => {
                assert!(
                    t0.elapsed() < Duration::from_secs(15),
                    "cannot connect: {}",
                    e
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
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

fn state_where(ws: &mut Client, f: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..500 {
        let v = recv(ws);
        if v["type"] == "state" && f(&v) {
            return v;
        }
    }
    panic!("no matching state");
}

fn send(ws: &mut Client, text: &str) {
    ws.send(Message::text(text)).unwrap();
}

/// Connect, load `prog.com` and wait for the loaded state.
fn loaded(srv: &Server) -> Client {
    let mut ws = connect(srv);
    send(&mut ws, r#"{"cmd":"load","name":"prog.com"}"#);
    state_where(&mut ws, |v| v["rom"]["name"] == "prog.com");
    ws
}

fn direct() -> CpmMachine {
    let mut m = CpmMachine::new();
    m.load_com(&PROGRAM);
    m
}

#[test]
fn e14_07_regs_equal_direct_cpu() {
    let srv = start("regs");
    let mut ws = loaded(&srv);
    send(&mut ws, r#"{"cmd":"step","n":3}"#);
    let s = state_where(&mut ws, |v| v["n"] == 3);

    let mut m = direct();
    for _ in 0..3 {
        m.step();
    }
    let r = &m.cpu.regs;
    assert_eq!(s["regs"]["af"], r.af());
    assert_eq!(s["regs"]["bc"], r.bc());
    assert_eq!(s["regs"]["de"], r.de());
    assert_eq!(s["regs"]["hl"], r.hl());
    assert_eq!(s["regs"]["pc"], r.pc);
    assert_eq!(s["regs"]["sp"], r.sp);
    assert_eq!(s["regs"]["af"].as_u64().unwrap() >> 8, 8, "A = 5 + 3");
}

#[test]
fn e14_07_dis_from_crate() {
    let srv = start("dis");
    let mut ws = loaded(&srv);
    send(&mut ws, r#"{"cmd":"step","n":3}"#);
    let s = state_where(&mut ws, |v| v["n"] == 3);

    let m = {
        let mut m = direct();
        for _ in 0..3 {
            m.step();
        }
        m
    };
    let pc = m.cpu.regs.pc;
    let insn = disassemble(pc, &mut |a| m.ram[usize::from(a)]);
    assert_eq!(s["dis"][0]["addr"], pc);
    assert_eq!(s["dis"][0]["text"], insn.text.as_str());
    assert_eq!(s["dis"][0]["len"], insn.len);
    assert_eq!(s["dis"][0]["text"], "LD E,A");
}

#[test]
fn e14_07_run_to_done() {
    let srv = start("run");
    let mut ws = loaded(&srv);
    send(&mut ws, r#"{"cmd":"run"}"#);
    let mut console = String::new();
    let last = loop {
        let s = state_where(&mut ws, |_| true);
        console.push_str(s["console_delta"].as_str().unwrap_or(""));
        if s["status"] == "done" {
            break s;
        }
    };

    let mut m = direct();
    m.run(1_000_000);
    assert!(m.exited);
    assert_eq!(last["n"], m.instr);
    assert_eq!(last["t"], m.tstates);
    assert_eq!(console, String::from_utf8_lossy(&m.out));
    assert_eq!(console, "\u{8}");
}

#[test]
fn e14_07_alu_from_crate() {
    let srv = start("alu");
    let mut ws = loaded(&srv);
    send(&mut ws, r#"{"cmd":"step","n":3}"#);
    let s = state_where(&mut ws, |v| v["n"] == 3);

    let mut m = direct();
    for _ in 0..3 {
        m.step();
    }
    let a = m.cpu.last_alu().expect("ALU operation recorded");
    assert_eq!(s["alu"]["op"], alu_name(a.op));
    assert_eq!(s["alu"]["op"], "ADD");
    assert_eq!(s["alu"]["r"], a.r);
    assert_eq!(s["alu"]["r"], 8);
    assert_eq!(s["alu"]["a"], 5);
    assert_eq!(s["alu"]["b"], 3);
}
