//! Simulation session with thread management and snapshot support.
//!
//! A [`Session`] owns a real [`CpmMachine`] (D-004) that runs in its own thread. Commands
//! reach the thread through a channel; the latest [`Snapshot`] is shared behind a mutex.

use crate::cpm::{CpmMachine, RunEnd};
use puce_z80::disasm::disassemble;
use serde::Serialize;
use std::collections::{BTreeSet, VecDeque};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Instructions per second for each speed index; 0 means turbo (no pacing).
pub const SPEED_TABLE: [u64; 8] = [1, 4, 15, 60, 300, 3000, 60000, 0];

/// Wall-clock length of one pacing slice.
const SLICE: Duration = Duration::from_millis(10);
/// Wall-clock budget of one turbo slice.
const TURBO_SLICE: Duration = Duration::from_millis(40);
/// Instructions executed between two clock reads in turbo mode.
const TURBO_CHUNK: u64 = 5_000;
/// Wait for a command while the simulation is not running.
const IDLE_WAIT: Duration = Duration::from_millis(100);
/// Sliding window of the ips / mhz measurement.
const RATE_WINDOW: Duration = Duration::from_millis(500);
/// Bytes shown in the memory dump.
const MEM_BYTES: usize = 256;

/// Observation window requested by the page (`view` command).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewParams {
    /// First address of the memory dump when `follow` is `"man"`.
    pub mem_base: u16,
    /// Number of disassembly rows.
    pub dis_rows: u32,
    /// Number of trace rows.
    pub trace_rows: u32,
    /// Number of stack rows.
    pub stack_rows: u32,
    /// What the memory dump follows: pc, sp, hl, de, bc, wr or man.
    pub follow: String,
}

impl Default for ViewParams {
    fn default() -> Self {
        ViewParams {
            mem_base: 256,
            dis_rows: 40,
            trace_rows: 16,
            stack_rows: 14,
            follow: "pc".to_string(),
        }
    }
}

/// A command sent to the simulation thread.
#[derive(Clone, Debug)]
pub enum Command {
    /// Load a program (CP/M layout at 0100h) and reset.
    Load {
        /// File name shown to the user.
        name: String,
        /// Program bytes.
        bytes: Vec<u8>,
    },
    /// Start the simulation.
    Run,
    /// Stop the simulation.
    Pause,
    /// Execute this many instructions, even when paused.
    Step(u32),
    /// Reload the current program.
    Reset,
    /// Select a speed (index in [`SPEED_TABLE`]).
    Speed(u8),
    /// Add a breakpoint.
    BpAdd(u16),
    /// Remove a breakpoint.
    BpDel(u16),
    /// Change the observation window.
    View(ViewParams),
    /// Clear the console output.
    ConsoleClear,
    /// Stop the thread.
    Quit,
}

/// Registers as sent to the page (protocol 31).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RegsView {
    /// AF.
    pub af: u16,
    /// BC.
    pub bc: u16,
    /// DE.
    pub de: u16,
    /// HL.
    pub hl: u16,
    /// AF'.
    pub af2: u16,
    /// BC'.
    pub bc2: u16,
    /// DE'.
    pub de2: u16,
    /// HL'.
    pub hl2: u16,
    /// IX.
    pub ix: u16,
    /// IY.
    pub iy: u16,
    /// SP.
    pub sp: u16,
    /// PC.
    pub pc: u16,
    /// I.
    pub i: u8,
    /// R.
    pub r: u8,
    /// WZ (MEMPTR).
    pub wz: u16,
    /// IFF1.
    pub iff1: bool,
    /// IFF2.
    pub iff2: bool,
    /// Interrupt mode.
    pub im: u8,
}

/// Last complete bus cycle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BusJson {
    /// Address bus.
    pub addr: u16,
    /// Data.
    pub data: u8,
    /// 0 fetch, 1 read, 2 write, 3 io-read, 4 io-write.
    pub kind: u8,
    /// Pin word as `0x...`.
    pub pins: String,
}

/// Last ALU operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AluJson {
    /// Operation name (ADD, ADC, ...).
    pub op: String,
    /// Width in bits.
    pub w: u8,
    /// First operand.
    pub a: u16,
    /// Second operand.
    pub b: u16,
    /// Carry in.
    pub c: u8,
    /// Result.
    pub r: u16,
    /// Flags before.
    pub f_in: u8,
}

/// One disassembled instruction.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DisRow {
    /// Address.
    pub addr: u16,
    /// Instruction bytes.
    pub bytes: Vec<u8>,
    /// Mnemonic text from the crate disassembler.
    pub text: String,
    /// Length in bytes.
    pub len: u8,
    /// Prefix: empty, CB, ED, DD, FD, DDCB or FDCB.
    pub pre: String,
    /// Opcode byte.
    pub op: u8,
}

/// One entry of the execution trace.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TraceRow {
    /// Instruction address.
    pub pc: u16,
    /// Disassembly.
    pub text: String,
}

/// One word of the stack.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StackRow {
    /// Address of the word.
    pub addr: u16,
    /// Little-endian word.
    pub word: u16,
}

/// Memory dump window.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct MemView {
    /// First address.
    pub base: u16,
    /// 256 bytes as 512 hex characters.
    pub hex: String,
}

/// Test counters printed by the CP/M suites.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TestsView {
    /// OK lines.
    pub ok: u32,
    /// ERROR lines.
    pub err: u32,
    /// ok + err.
    pub total: u32,
}

/// Loaded program.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RomView {
    /// File name.
    pub name: String,
    /// Size in bytes.
    pub size: usize,
    /// `cpm` or `raw`.
    pub kind: String,
}

/// State published by the simulation thread: the `state` message of protocol 31.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Snapshot {
    /// Always `state`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `idle`, `running`, `halted`, `done` or `bp`.
    pub status: String,
    /// Instructions executed since the last reset.
    pub n: u64,
    /// T-states executed since the last reset.
    pub t: u64,
    /// Instructions per second (500 ms window).
    pub ips: f64,
    /// Millions of T-states per second (500 ms window).
    pub mhz: f64,
    /// T-states of the last instruction.
    pub tlast: u64,
    /// Registers.
    pub regs: RegsView,
    /// Last bus cycle.
    pub bus: BusJson,
    /// Last ALU operation, `null` before any.
    pub alu: Option<AluJson>,
    /// Disassembly from PC.
    pub dis: Vec<DisRow>,
    /// Most recent instructions first.
    pub trace: Vec<TraceRow>,
    /// Words above SP.
    pub stack: Vec<StackRow>,
    /// Memory dump.
    pub mem: MemView,
    /// Recently written addresses.
    pub wlog: Vec<u16>,
    /// Console text added since the last state; filled per client with [`Session::console_since`].
    pub console_delta: String,
    /// Test counters.
    pub tests: TestsView,
    /// Breakpoints.
    pub bps: Vec<u16>,
    /// Loaded program.
    pub rom: RomView,
}

/// The `hello` message.
pub fn hello() -> serde_json::Value {
    serde_json::json!({"type": "hello", "version": crate::VERSION})
}

/// Name of an ALU operation code (see `puce_z80::probe::AluRecord`).
pub fn alu_name(op: u8) -> String {
    const BASE: [&str; 10] = [
        "ADD", "ADC", "SUB", "SBC", "AND", "XOR", "OR", "CP", "INC", "DEC",
    ];
    const SHIFT: [&str; 8] = ["RLC", "RRC", "RL", "RR", "SLA", "SRA", "SLL", "SRL"];
    match op {
        0..=9 => BASE[usize::from(op)].to_string(),
        16..=23 => SHIFT[usize::from(op - 16)].to_string(),
        32 => "ADD16".to_string(),
        33 => "ADC16".to_string(),
        34 => "SBC16".to_string(),
        40 => "DAA".to_string(),
        41 => "NEG".to_string(),
        42 => "CPL".to_string(),
        _ => format!("OP{}", op),
    }
}

/// Data shared between the thread and the handles.
#[derive(Default)]
struct Shared {
    snap: Snapshot,
    console: String,
}

/// Handle on the simulation thread.
pub struct Session {
    tx: Sender<Command>,
    shared: Arc<Mutex<Shared>>,
}

impl Session {
    /// Start the simulation thread (idle, nothing loaded).
    pub fn spawn() -> Session {
        let (tx, rx) = channel();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let out = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut w = Worker::new();
            let mut last = Instant::now();
            loop {
                let wait = if w.running { SLICE } else { IDLE_WAIT };
                match rx.recv_timeout(wait) {
                    Ok(Command::Quit) | Err(RecvTimeoutError::Disconnected) => break,
                    Ok(c) => w.handle(c),
                    Err(RecvTimeoutError::Timeout) => {}
                }
                // Commands that are already queued are applied before simulating.
                while let Ok(c) = rx.try_recv() {
                    if matches!(c, Command::Quit) {
                        return;
                    }
                    w.handle(c);
                }
                let now = Instant::now();
                let was_running = w.running;
                if was_running {
                    w.advance(now.duration_since(last));
                }
                last = now;
                if was_running || w.dirty {
                    w.publish(&out);
                    w.dirty = false;
                }
            }
        });
        Session { tx, shared }
    }

    /// Queue a command for the simulation thread.
    pub fn send(&self, c: Command) {
        // A closed channel means the thread has stopped: nothing left to command.
        let _ = self.tx.send(c);
    }

    /// A copy of the latest published state.
    pub fn snapshot(&self) -> Snapshot {
        self.shared
            .lock()
            .map(|s| s.snap.clone())
            .unwrap_or_default()
    }

    /// Console text from byte position `pos` and the new position (per-client cursor).
    /// A position past the end (the console was cleared) restarts from the beginning.
    pub fn console_since(&self, pos: usize) -> (String, usize) {
        let Ok(s) = self.shared.lock() else {
            return (String::new(), pos);
        };
        let pos = if pos > s.console.len() { 0 } else { pos };
        let text = s.console.get(pos..).unwrap_or("").to_string();
        (text, s.console.len())
    }
}

/// State owned by the simulation thread.
struct Worker {
    machine: CpmMachine,
    rom: Option<(String, Vec<u8>)>,
    status: &'static str,
    running: bool,
    speed: u8,
    credit: f64,
    bps: BTreeSet<u16>,
    view: ViewParams,
    /// Something changed since the last publication.
    dirty: bool,
    /// The shared console must be emptied at the next publication.
    clear_console: bool,
    /// (time, instructions, T-states) samples of the last 500 ms.
    samples: VecDeque<(Instant, u64, u64)>,
    /// Bytes of the machine console already copied to the shared console.
    published: usize,
}

impl Worker {
    fn new() -> Worker {
        Worker {
            machine: CpmMachine::new(),
            rom: None,
            status: "idle",
            running: false,
            speed: 5,
            credit: 0.0,
            bps: BTreeSet::new(),
            view: ViewParams::default(),
            dirty: true,
            clear_console: false,
            samples: VecDeque::new(),
            published: 0,
        }
    }

    fn reload(&mut self) {
        self.machine = CpmMachine::new();
        if let Some((_, bytes)) = &self.rom {
            self.machine.load_com(bytes);
        }
        self.running = false;
        self.credit = 0.0;
        self.status = "idle";
        self.samples.clear();
        self.published = 0;
    }

    fn handle(&mut self, c: Command) {
        self.dirty = true;
        match c {
            Command::Load { name, bytes } => {
                self.rom = Some((name, bytes));
                self.reload();
            }
            Command::Run => {
                if self.status != "done" && self.status != "halted" {
                    self.running = true;
                    self.status = "running";
                }
            }
            Command::Pause => {
                self.running = false;
                if self.status == "running" {
                    self.status = "idle";
                }
            }
            Command::Step(n) => {
                self.running = false;
                self.execute(u64::from(n), false);
            }
            Command::Reset => self.reload(),
            Command::Speed(i) => {
                self.speed = i.min(7);
                self.credit = 0.0;
            }
            Command::BpAdd(a) => {
                self.bps.insert(a);
            }
            Command::BpDel(a) => {
                self.bps.remove(&a);
            }
            Command::View(v) => self.view = v,
            Command::ConsoleClear => {
                self.machine.out.clear();
                self.published = 0;
                self.clear_console = true;
            }
            Command::Quit => {}
        }
    }

    /// Execute up to `n` instructions; stops early on exit, HALT and (if asked) breakpoints.
    /// Returns the number of instructions executed.
    fn execute(&mut self, n: u64, stop_on_bp: bool) -> u64 {
        let mut done = 0;
        while done < n {
            let chunk = if stop_on_bp && !self.bps.is_empty() {
                1
            } else {
                n - done
            };
            let before = self.machine.instr;
            let end = self.machine.run(chunk);
            done += self.machine.instr - before;
            match end {
                RunEnd::Exited => {
                    self.running = false;
                    self.status = "done";
                    break;
                }
                RunEnd::Halted => {
                    self.running = false;
                    self.status = "halted";
                    break;
                }
                RunEnd::MaxInstr => {}
            }
            if stop_on_bp && self.bps.contains(&self.machine.cpu.regs.pc) {
                self.running = false;
                self.status = "bp";
                break;
            }
        }
        done
    }

    /// Simulate for the wall-clock time `dt` according to the speed setting.
    fn advance(&mut self, dt: Duration) {
        let ips = SPEED_TABLE[usize::from(self.speed)];
        if ips == 0 {
            let start = Instant::now();
            while self.running && start.elapsed() < TURBO_SLICE {
                self.execute(TURBO_CHUNK, true);
            }
        } else {
            let cap = ips as f64 * 0.05 + 1.0;
            self.credit = (self.credit + ips as f64 * dt.as_secs_f64()).min(cap);
            let whole = self.credit.floor();
            if whole >= 1.0 {
                let done = self.execute(whole as u64, true);
                self.credit -= done as f64;
            }
        }
    }

    /// Instructions and megahertz over the last 500 ms.
    fn rates(&mut self) -> (f64, f64) {
        let now = Instant::now();
        self.samples
            .push_back((now, self.machine.instr, self.machine.tstates));
        while self.samples.len() > 2 && now.duration_since(self.samples[0].0) > RATE_WINDOW {
            self.samples.pop_front();
        }
        let (t0, i0, c0) = self.samples[0];
        let dt = now.duration_since(t0).as_secs_f64();
        if !self.running || dt <= 0.0 {
            return (0.0, 0.0);
        }
        (
            (self.machine.instr - i0) as f64 / dt,
            (self.machine.tstates - c0) as f64 / dt / 1e6,
        )
    }

    fn dis_text(&self, addr: u16) -> DisRow {
        let ram = &self.machine.ram;
        let insn = disassemble(addr, &mut |a| ram[usize::from(a)]);
        let bytes: Vec<u8> = (0..u16::from(insn.len))
            .map(|i| ram[usize::from(addr.wrapping_add(i))])
            .collect();
        let pre = match (insn.prefix, bytes.get(1)) {
            (0, _) => "",
            (0xCB, _) => "CB",
            (0xED, _) => "ED",
            (0xDD, Some(0xCB)) => "DDCB",
            (0xDD, _) => "DD",
            (0xFD, Some(0xCB)) => "FDCB",
            (0xFD, _) => "FD",
            _ => "",
        };
        DisRow {
            addr,
            bytes,
            text: insn.text.as_str().to_string(),
            len: insn.len,
            pre: pre.to_string(),
            op: insn.op,
        }
    }

    /// Build the `state` message from the machine and the view.
    fn build(&mut self) -> Snapshot {
        let (ips, mhz) = self.rates();
        let cpu = &self.machine.cpu;
        let r = &cpu.regs;
        let ram = &self.machine.ram;
        let b = cpu.last_bus();

        let mut dis = Vec::new();
        let mut addr = r.pc;
        for _ in 0..self.view.dis_rows {
            let row = self.dis_text(addr);
            addr = addr.wrapping_add(u16::from(row.len.max(1)));
            dis.push(row);
        }
        let trace: Vec<TraceRow> = cpu
            .pc_history()
            .take(self.view.trace_rows as usize)
            .map(|pc| TraceRow {
                pc,
                text: self.dis_text(pc).text,
            })
            .collect();
        let stack: Vec<StackRow> = (0..self.view.stack_rows)
            .map(|i| {
                let a = r.sp.wrapping_add((2 * i) as u16);
                let word = u16::from(ram[usize::from(a)])
                    | (u16::from(ram[usize::from(a.wrapping_add(1))]) << 8);
                StackRow { addr: a, word }
            })
            .collect();
        let wlog: Vec<u16> = cpu.recent_writes().collect();
        let focus = match self.view.follow.as_str() {
            "sp" => r.sp,
            "hl" => r.hl(),
            "de" => r.de(),
            "bc" => r.bc(),
            "wr" => wlog.first().copied().unwrap_or(0),
            "man" => self.view.mem_base,
            _ => r.pc,
        };
        let base = (focus & 0xFFF0).min((0x10000 - MEM_BYTES) as u16);
        let mem: String = (0..MEM_BYTES)
            .map(|i| format!("{:02x}", ram[usize::from(base) + i]))
            .collect();
        let (name, size, raw) = match &self.rom {
            Some((n, bytes)) => (n.clone(), bytes.len(), n.to_lowercase().ends_with(".bin")),
            None => (String::new(), 0, false),
        };
        Snapshot {
            kind: "state".to_string(),
            status: self.status.to_string(),
            n: self.machine.instr,
            t: self.machine.tstates,
            ips,
            mhz,
            tlast: u64::from(cpu.instruction_tstates()),
            regs: RegsView {
                af: r.af(),
                bc: r.bc(),
                de: r.de(),
                hl: r.hl(),
                af2: r.af2(),
                bc2: r.bc2(),
                de2: r.de2(),
                hl2: r.hl2(),
                ix: r.ix,
                iy: r.iy,
                sp: r.sp,
                pc: r.pc,
                i: r.i,
                r: r.r,
                wz: r.wz,
                iff1: r.iff1,
                iff2: r.iff2,
                im: r.im,
            },
            bus: BusJson {
                addr: b.addr,
                data: b.data,
                kind: b.kind,
                pins: format!("{:#x}", b.pins.0),
            },
            alu: cpu.last_alu().map(|a| AluJson {
                op: alu_name(a.op),
                w: a.w,
                a: a.a,
                b: a.b,
                c: a.c,
                r: a.r,
                f_in: a.f_in,
            }),
            dis,
            trace,
            stack,
            mem: MemView { base, hex: mem },
            wlog,
            console_delta: String::new(),
            tests: TestsView {
                ok: self.machine.tests_ok,
                err: self.machine.tests_err,
                total: self.machine.tests_ok + self.machine.tests_err,
            },
            bps: self.bps.iter().copied().collect(),
            rom: RomView {
                name,
                size,
                kind: if raw { "raw" } else { "cpm" }.to_string(),
            },
        }
    }

    fn publish(&mut self, out: &Arc<Mutex<Shared>>) {
        let snap = self.build();
        let added = self.machine.out.len().saturating_sub(self.published);
        let text = String::from_utf8_lossy(&self.machine.out[self.published..]).into_owned();
        self.published += added;
        if let Ok(mut s) = out.lock() {
            if self.clear_console {
                s.console.clear();
                self.clear_console = false;
            }
            s.console.push_str(&text);
            s.snap = snap;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LD E,'A'; LD C,2; CALL 5; JP 0
    const PRINT_A: [u8; 10] = [0x1E, b'A', 0x0E, 0x02, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00];
    /// JR $
    const SPIN: [u8; 2] = [0x18, 0xFE];

    fn load(s: &Session, bytes: &[u8]) {
        s.send(Command::Load {
            name: "t.com".into(),
            bytes: bytes.to_vec(),
        });
    }

    fn wait_for(s: &Session, what: &str, f: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let snap = s.snapshot();
            if f(&snap) {
                return snap;
            }
            assert!(
                Instant::now() < end,
                "timeout waiting for {}: {:?}",
                what,
                snap
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn e14_01_load_and_step() {
        let s = Session::spawn();
        load(&s, &PRINT_A);
        s.send(Command::Step(3));
        let snap = wait_for(&s, "n == 3", |x| x.n == 3);
        assert_eq!(snap.status, "idle");
        s.send(Command::Quit);
    }

    #[test]
    fn e14_01_run_until_exit() {
        let s = Session::spawn();
        s.send(Command::Speed(7));
        load(&s, &PRINT_A);
        s.send(Command::Run);
        let snap = wait_for(&s, "done", |x| x.status == "done");
        assert!(snap.n >= 4);
        s.send(Command::Quit);
    }

    #[test]
    fn e14_01_breakpoint_stops() {
        let s = Session::spawn();
        s.send(Command::Speed(7));
        load(&s, &PRINT_A);
        s.send(Command::BpAdd(0x0104));
        s.send(Command::Run);
        let snap = wait_for(&s, "bp", |x| x.status == "bp");
        assert_eq!(snap.n, 2, "LD E,n and LD C,n ran, CALL 5 is next");
        // Stepping once more moves past the breakpoint.
        s.send(Command::Step(1));
        wait_for(&s, "n == 3", |x| x.n == 3);
        s.send(Command::Quit);
    }

    #[test]
    fn e14_01_pause_stops() {
        let s = Session::spawn();
        s.send(Command::Speed(7));
        load(&s, &SPIN);
        s.send(Command::Run);
        wait_for(&s, "running", |x| x.status == "running" && x.n > 0);
        s.send(Command::Pause);
        wait_for(&s, "idle", |x| x.status == "idle");
        std::thread::sleep(Duration::from_millis(100));
        let a = s.snapshot().n;
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(a, s.snapshot().n, "n must not grow while paused");
        s.send(Command::Quit);
    }

    #[test]
    fn e14_01_reset_reloads() {
        let s = Session::spawn();
        load(&s, &PRINT_A);
        s.send(Command::Step(2));
        wait_for(&s, "n == 2", |x| x.n == 2);
        s.send(Command::Reset);
        let snap = wait_for(&s, "n == 0", |x| x.n == 0);
        assert_eq!(snap.status, "idle");
        s.send(Command::Step(1));
        wait_for(&s, "n == 1", |x| x.n == 1);
        s.send(Command::Quit);
    }

    #[test]
    fn e14_01_speed_table() {
        let s = Session::spawn();
        s.send(Command::Speed(2)); // 15 instructions per second
        load(&s, &SPIN);
        s.send(Command::Run);
        std::thread::sleep(Duration::from_millis(1000));
        let n = s.snapshot().n;
        assert!(
            (5..=40).contains(&n),
            "expected about 15 instructions, got {}",
            n
        );
        s.send(Command::Quit);
    }

    // ----- E14-02: the state message ------------------------------------------------------------

    /// A worker with `bytes` loaded, stepped `n` instructions.
    fn stepped(bytes: &[u8], n: u32) -> Worker {
        let mut w = Worker::new();
        w.handle(Command::Load {
            name: "t.com".into(),
            bytes: bytes.to_vec(),
        });
        w.handle(Command::Step(n));
        w
    }

    #[test]
    fn e14_02_json_fields() {
        let mut w = stepped(&PRINT_A, 1);
        let v = serde_json::to_value(w.build()).unwrap();
        for k in [
            "type",
            "status",
            "n",
            "t",
            "ips",
            "mhz",
            "tlast",
            "regs",
            "bus",
            "alu",
            "dis",
            "trace",
            "stack",
            "mem",
            "wlog",
            "console_delta",
            "tests",
            "bps",
            "rom",
        ] {
            assert!(v.get(k).is_some(), "missing field {}", k);
        }
        assert_eq!(v["type"], "state");
        for k in [
            "af", "bc", "de", "hl", "af2", "bc2", "de2", "hl2", "ix", "iy", "sp", "pc", "i", "r",
            "wz", "iff1", "iff2", "im",
        ] {
            assert!(v["regs"].get(k).is_some(), "missing regs.{}", k);
        }
        for k in ["addr", "data", "kind", "pins"] {
            assert!(v["bus"].get(k).is_some(), "missing bus.{}", k);
        }
        assert!(v["bus"]["pins"].as_str().unwrap().starts_with("0x"));
        for k in ["base", "hex"] {
            assert!(v["mem"].get(k).is_some());
        }
        for k in ["ok", "err", "total"] {
            assert!(v["tests"].get(k).is_some());
        }
        for k in ["name", "size", "kind"] {
            assert!(v["rom"].get(k).is_some());
        }
        assert_eq!(hello()["type"], "hello");
    }

    #[test]
    fn e14_02_regs_match_cpu() {
        let mut w = stepped(&PRINT_A, 2);
        let s = w.build();
        let r = &w.machine.cpu.regs;
        assert_eq!(s.regs.pc, r.pc);
        assert_eq!(s.regs.af, r.af());
        assert_eq!(s.regs.bc, r.bc());
        assert_eq!(s.regs.de, r.de());
        assert_eq!(s.regs.hl, r.hl());
        assert_eq!(s.regs.sp, r.sp);
        assert_eq!(s.regs.de & 0xFF, u16::from(b'A'));
        assert_eq!(s.regs.bc & 0xFF, 2);
        assert_eq!((s.n, s.status.as_str()), (2, "idle"));
    }

    #[test]
    fn e14_02_dis_rows() {
        let mut w = stepped(&PRINT_A, 0);
        w.handle(Command::View(ViewParams {
            dis_rows: 4,
            ..ViewParams::default()
        }));
        let s = w.build();
        assert_eq!(s.dis.len(), 4);
        assert_eq!(s.dis[0].addr, 0x0100);
        assert_eq!(s.dis[0].text, "LD E,$41");
        assert_eq!(s.dis[0].bytes, vec![0x1E, b'A']);
        assert_eq!(s.dis[0].len, 2);
        assert_eq!(s.dis[0].pre, "");
        assert_eq!(s.dis[1].addr, 0x0102);
        assert_eq!(s.dis[2].text, "CALL $0005");
        assert_eq!(s.dis[2].len, 3);
    }

    #[test]
    fn e14_02_stack_words() {
        // LD HL,1234h; PUSH HL; LD HL,5678h; PUSH HL
        let mut w = stepped(&[0x21, 0x34, 0x12, 0xE5, 0x21, 0x78, 0x56, 0xE5], 4);
        let s = w.build();
        let sp = w.machine.cpu.regs.sp;
        assert_eq!(s.stack.len(), 14);
        assert_eq!(
            s.stack[0],
            StackRow {
                addr: sp,
                word: 0x5678
            }
        );
        assert_eq!(
            s.stack[1],
            StackRow {
                addr: sp.wrapping_add(2),
                word: 0x1234
            }
        );
    }

    #[test]
    fn e14_02_mem_hex_len() {
        let mut w = stepped(&PRINT_A, 0);
        let s = w.build();
        assert_eq!(s.mem.hex.len(), 512);
        assert_eq!(s.mem.base, 0x0100);
        assert!(s.mem.hex.starts_with("1e41"), "{}", &s.mem.hex[..8]);
        w.handle(Command::View(ViewParams {
            follow: "man".into(),
            mem_base: 0xFFF0,
            ..ViewParams::default()
        }));
        let s = w.build();
        assert_eq!(s.mem.base, 0xFF00, "the window never runs past 0xFFFF");
        assert_eq!(s.mem.hex.len(), 512);
    }

    #[test]
    fn e14_02_bus_view() {
        let mut w = stepped(&[0x3E, 0x07, 0x32, 0x00, 0x20], 2); // LD A,7; LD (2000h),A
        let s = w.build();
        let b = w.machine.cpu.last_bus();
        assert_eq!(
            (s.bus.addr, s.bus.data, s.bus.kind),
            (b.addr, b.data, b.kind)
        );
        assert_eq!((s.bus.addr, s.bus.data, s.bus.kind), (0x2000, 7, 2));
        assert_eq!(s.bus.pins, format!("{:#x}", b.pins.0));
    }

    #[test]
    fn e14_02_alu_after_add() {
        let mut w = stepped(&[0x3E, 0x05, 0xC6, 0x03], 1);
        assert!(w.build().alu.is_none() || w.build().alu.is_some());
        w.handle(Command::Step(1)); // ADD A,3
        let a = w.build().alu.expect("ALU record after ADD");
        assert_eq!((a.op.as_str(), a.w, a.a, a.b, a.r), ("ADD", 8, 5, 3, 8));
    }

    #[test]
    fn e14_02_wlog_after_write() {
        let mut w = stepped(&[0x21, 0x00, 0x20, 0x77], 2); // LD HL,2000h; LD (HL),A
        let s = w.build();
        assert_eq!(s.wlog.first(), Some(&0x2000));
    }

    #[test]
    fn e14_02_console_cursor() {
        let s = Session::spawn();
        s.send(Command::Load {
            name: "t.com".into(),
            bytes: PRINT_A.to_vec(),
        });
        s.send(Command::Speed(7));
        s.send(Command::Run);
        wait_for(&s, "done", |x| x.status == "done");
        let (text, pos) = s.console_since(0);
        assert_eq!(text, "A");
        assert_eq!(s.console_since(pos), (String::new(), pos));
        s.send(Command::ConsoleClear);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(s.console_since(pos), (String::new(), 0));
        s.send(Command::Quit);
    }
}
