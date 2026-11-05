//! The `puce-z80` core in the browser, without any server (E17-05).
//!
//! [`Machine`] is a plain Rust 64 KB computer around the CPU: RAM, and one output port
//! (`OUT (0),A` appends a byte to a text buffer). [`Z80`] exposes it to JavaScript through
//! `wasm-bindgen`. See `README.md` for the build commands and `demo/index.html`.

#![deny(missing_docs)]

use puce_z80::{Bus, Cpu};
use wasm_bindgen::prelude::*;

/// Demo program: prints `Hello, Z80!` and a newline on port 0, then HALTs.
pub const DEMO: [u8; 25] = [
    0x21, 0x0D, 0x00, // LD HL,msg
    0x7E, // loop: LD A,(HL)
    0xB7, // OR A
    0x28, 0x05, // JR Z,end
    0xD3, 0x00, // OUT (0),A
    0x23, // INC HL
    0x18, 0xF7, // JR loop
    0x76, // end: HALT (the message follows at 0x000D)
    b'H', b'e', b'l', b'l', b'o', b',', b' ', b'Z', b'8', b'0', b'!', b'\n',
];

/// RAM plus the output port.
struct Board {
    ram: Vec<u8>,
    out: Vec<u8>,
}

impl Bus for Board {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }
    fn write(&mut self, addr: u16, data: u8) {
        self.ram[usize::from(addr)] = data;
    }
    fn io_write(&mut self, port: u16, data: u8) {
        if port & 0xFF == 0 {
            self.out.push(data);
        }
    }
}

/// A Z80 with 64 KB of RAM and a text output port.
pub struct Machine {
    cpu: Cpu,
    board: Board,
}

impl Default for Machine {
    fn default() -> Machine {
        Machine::new()
    }
}

impl Machine {
    /// A machine with zeroed RAM and the CPU in its power-on state.
    pub fn new() -> Machine {
        Machine {
            cpu: Cpu::new(),
            board: Board {
                ram: vec![0; 65536],
                out: Vec::new(),
            },
        }
    }

    /// Copy `bytes` into RAM at `addr`; what does not fit before 0xFFFF is dropped.
    pub fn load(&mut self, addr: u16, bytes: &[u8]) {
        let start = usize::from(addr);
        let n = bytes.len().min(65536 - start);
        self.board.ram[start..start + n].copy_from_slice(&bytes[..n]);
    }

    /// Reset the CPU (RAM and pending output are kept).
    pub fn reset(&mut self) {
        self.cpu = Cpu::new();
    }

    /// Run one instruction and return its T-states.
    pub fn step(&mut self) -> u32 {
        self.cpu.step(&mut self.board)
    }

    /// Run instructions until at least `budget` T-states have elapsed or the CPU halts.
    /// Returns the T-states actually run.
    pub fn run(&mut self, budget: u32) -> u32 {
        let mut done = 0;
        while done < budget && !self.cpu.regs.halted {
            done += self.step();
        }
        done
    }

    /// True when the CPU is halted.
    pub fn halted(&self) -> bool {
        self.cpu.regs.halted
    }

    /// AF, BC, DE, HL, IX, IY, SP, PC, AF', BC', DE', HL'.
    pub fn regs(&self) -> Vec<u16> {
        let r = &self.cpu.regs;
        let w = |h: u8, l: u8| u16::from(h) << 8 | u16::from(l);
        vec![
            w(r.a, r.f),
            w(r.b, r.c),
            w(r.d, r.e),
            w(r.h, r.l),
            r.ix,
            r.iy,
            r.sp,
            r.pc,
            w(r.a2, r.f2),
            w(r.b2, r.c2),
            w(r.d2, r.e2),
            w(r.h2, r.l2),
        ]
    }

    /// Read a RAM byte.
    pub fn peek(&self, addr: u16) -> u8 {
        self.board.ram[usize::from(addr)]
    }

    /// Text written on port 0 since the last call (bytes are read as Latin-1).
    pub fn take_output(&mut self) -> String {
        let s: String = self.board.out.iter().map(|&b| char::from(b)).collect();
        self.board.out.clear();
        s
    }
}

/// JavaScript handle on a [`Machine`].
#[wasm_bindgen]
pub struct Z80 {
    m: Machine,
}

#[wasm_bindgen]
impl Z80 {
    /// New machine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Z80 {
        Z80 { m: Machine::new() }
    }

    /// Load the built-in "Hello" program at address 0 and reset the CPU.
    pub fn load_demo(&mut self) {
        self.m.load(0, &demo_program());
        self.m.reset();
    }

    /// Copy `bytes` into RAM at `addr`.
    pub fn load(&mut self, addr: u16, bytes: &[u8]) {
        self.m.load(addr, bytes);
    }

    /// Reset the CPU.
    pub fn reset(&mut self) {
        self.m.reset();
    }

    /// Run one instruction; returns its T-states.
    pub fn step(&mut self) -> u32 {
        self.m.step()
    }

    /// Run about `budget` T-states (stops at HALT); returns the T-states run.
    pub fn run(&mut self, budget: u32) -> u32 {
        self.m.run(budget)
    }

    /// True when halted.
    pub fn halted(&self) -> bool {
        self.m.halted()
    }

    /// AF, BC, DE, HL, IX, IY, SP, PC, AF', BC', DE', HL'.
    pub fn regs(&self) -> Vec<u16> {
        self.m.regs()
    }

    /// Read a RAM byte.
    pub fn peek(&self, addr: u16) -> u8 {
        self.m.peek(addr)
    }

    /// Text written on port 0 since the last call.
    pub fn take_output(&mut self) -> String {
        self.m.take_output()
    }
}

impl Default for Z80 {
    fn default() -> Z80 {
        Z80::new()
    }
}

/// The demo program with its HALT in place.
pub fn demo_program() -> Vec<u8> {
    DEMO.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e17_05_demo_prints_hello() {
        let mut m = Machine::new();
        m.load(0, &demo_program());
        m.run(100_000);
        assert!(m.halted());
        assert_eq!(m.take_output(), "Hello, Z80!\n");
        assert_eq!(m.take_output(), "");
    }

    #[test]
    fn e17_05_run_budget() {
        let mut m = Machine::new();
        m.load(0, &[0x18, 0xFE]); // JR $
        let t = m.run(1000);
        assert!((1000..1000 + 12).contains(&t));
        assert!(!m.halted());
    }

    #[test]
    fn e17_05_regs_order() {
        let mut m = Machine::new();
        m.load(0, &[0x3E, 0x12, 0x01, 0x34, 0x56]); // LD A,12 ; LD BC,5634
        m.step();
        m.step();
        let r = m.regs();
        assert_eq!(r.len(), 12);
        assert_eq!(r[0] >> 8, 0x12);
        assert_eq!(r[1], 0x5634);
        assert_eq!(r[7], 5);
    }

    #[test]
    fn e17_05_load_truncates_at_64k() {
        let mut m = Machine::new();
        m.load(0xFFFE, &[1, 2, 3, 4]);
        assert_eq!(m.peek(0xFFFE), 1);
        assert_eq!(m.peek(0xFFFF), 2);
        assert_eq!(m.peek(0), 0);
    }

    #[test]
    fn e17_05_js_handle() {
        let mut z = Z80::new();
        z.load_demo();
        z.run(100_000);
        assert!(z.halted());
        assert_eq!(z.take_output(), "Hello, Z80!\n");
    }

    /// Both crates build for `wasm32-unknown-unknown` (skipped when that target is not installed).
    #[test]
    fn e17_05_builds() {
        use std::process::Command;
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sysroot = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        let sysroot = String::from_utf8_lossy(&sysroot.stdout).trim().to_string();
        let lib = std::path::Path::new(&sysroot).join("lib/rustlib/wasm32-unknown-unknown/lib");
        if !lib.is_dir() {
            eprintln!("SKIP e17_05_builds: wasm32-unknown-unknown not installed");
            return;
        }
        for pkg in ["puce-z80", "puce-z80-wasm"] {
            let st = Command::new(&cargo)
                .args(["build", "-p", pkg, "--target", "wasm32-unknown-unknown"])
                .env("CARGO_TARGET_DIR", dir.join("../../target/e17_05_wasm"))
                .current_dir(dir)
                .status()
                .unwrap();
            assert!(st.success(), "build of {} for wasm failed", pkg);
        }
    }
}
