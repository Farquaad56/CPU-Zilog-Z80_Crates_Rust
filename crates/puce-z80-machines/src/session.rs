//! Simulation session with thread management and snapshot support.
//!
//! A [`Session`] owns a real [`CpmMachine`] (D-004) that runs in its own thread. Commands
//! reach the thread through a channel; the latest [`Snapshot`] is shared behind a mutex.

use crate::cpm::{CpmMachine, RunEnd};
use std::collections::BTreeSet;
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
            mem_base: 0,
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

/// State published by the simulation thread (completed in E14-02).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// `idle`, `running`, `halted`, `done` or `bp`.
    pub status: String,
    /// Instructions executed since the last reset.
    pub n: u64,
    /// T-states executed since the last reset.
    pub t: u64,
}

/// Handle on the simulation thread.
pub struct Session {
    tx: Sender<Command>,
    shared: Arc<Mutex<Snapshot>>,
}

impl Session {
    /// Start the simulation thread (idle, nothing loaded).
    pub fn spawn() -> Session {
        let (tx, rx) = channel();
        let shared = Arc::new(Mutex::new(Snapshot {
            status: "idle".to_string(),
            ..Snapshot::default()
        }));
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
                if w.running {
                    w.advance(now.duration_since(last));
                }
                last = now;
                w.publish(&out);
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
        self.shared.lock().map(|s| s.clone()).unwrap_or_default()
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
    #[allow(dead_code)] // read by the full snapshot (E14-02)
    view: ViewParams,
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
    }

    fn handle(&mut self, c: Command) {
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
            Command::ConsoleClear => self.machine.out.clear(),
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

    fn publish(&self, out: &Arc<Mutex<Snapshot>>) {
        if let Ok(mut s) = out.lock() {
            s.status = self.status.to_string();
            s.n = self.machine.instr;
            s.t = self.machine.tstates;
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
}
