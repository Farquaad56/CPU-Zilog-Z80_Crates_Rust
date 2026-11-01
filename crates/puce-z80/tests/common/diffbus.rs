//! Differential test harness: PRNG, seed, hash, and recording TestBus.
//!
//! Implements the exact xorshift32 PRNG from docs/specs/24_DIFF_VECTORS.md
//! so that initial CPU state can be regenerated from a (table, opcode, vec)
//! triple without storing any initial bytes in diff.tsv.

use puce_z80::bus::{Bus, Cycle};
use puce_z80::pins::Pins;

/// xorshift32 PRNG state.
#[derive(Clone, Copy)]
pub struct Xs32 {
    s: u32,
}

impl Xs32 {
    /// Create a new PRNG with the given seed (must be non-zero).
    pub fn new(seed: u32) -> Self {
        Xs32 { s: seed }
    }

    /// Advance one step and return the new state value.
    pub fn next(&mut self) -> u32 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.s = x;
        x
    }

    /// Draw an 8-bit random value (top byte of next()).
    pub fn n8(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }

    /// Draw a 16-bit random value (top two bytes of next()).
    pub fn n16(&mut self) -> u16 {
        (self.next() >> 16) as u16
    }
}

/// Compute the seed for a given test vector.
///
/// table_id: main=0, cb=1, ed=2, ddfd=3, ddcb=4
pub fn seed(table_id: u32, opcode: u8, vec: u8) -> u32 {
    let base = (table_id << 16) | ((opcode as u32) << 4) | (vec as u32);
    (base.wrapping_mul(2654435761)) | 1
}

/// Compute the memory byte at address `addr` for a given seed.
pub fn hash(addr: u16, seed: u32) -> u8 {
    let mut h = ((addr as u32) ^ (seed.wrapping_mul(0x9E3779B1))).wrapping_mul(0x45D9F3B);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45D9F3B);
    h ^= h >> 16;
    (h >> 24) as u8
}

/// A Bus implementation that records all memory writes and I/O operations.
pub struct TestBus {
    pub mem: Box<[u8; 65536]>,
    /// Recorded memory writes in order: (addr, value).
    pub writes: Vec<(u16, u8)>,
    /// Recorded port outputs in order: (port, value).
    pub port_out: Vec<(u16, u8)>,
    /// Recorded port inputs in order: ports read.
    pub port_in: Vec<u16>,
}

impl TestBus {
    /// Create a new TestBus with memory initialized from the hash function.
    pub fn new(seed: u32) -> Self {
        let mut mem = Box::new([0u8; 65536]);
        for addr in 0..=65535u16 {
            mem[addr as usize] = hash(addr, seed);
        }
        TestBus {
            mem,
            writes: Vec::new(),
            port_out: Vec::new(),
            port_in: Vec::new(),
        }
    }

    /// Write code bytes at PC for the given table/opcode.
    pub fn write_code(&mut self, pc: u16, table: &str, opcode: u8) {
        match table {
            "main" => {
                self.mem[pc as usize] = opcode;
            }
            "cb" => {
                self.mem[pc as usize] = 0xCB;
                self.mem[(pc + 1) as usize] = opcode;
            }
            "ed" => {
                self.mem[pc as usize] = 0xED;
                self.mem[(pc + 1) as usize] = opcode;
            }
            "ddfd" => {
                // Odd vec uses FD (IY), even uses DD (IX) - handled by caller via prefix byte
                let pre = if pc & 1 != 0 { 0xFD } else { 0xDD };
                self.mem[pc as usize] = pre;
                self.mem[(pc + 1) as usize] = opcode;
            }
            "ddcb" => {
                // Odd vec uses FD (IY), even uses DD (IX) - handled by caller via prefix byte
                let pre = if pc & 1 != 0 { 0xFD } else { 0xDD };
                self.mem[pc as usize] = pre;
                self.mem[(pc + 1) as usize] = 0xCB;
                // Third byte is a random displacement from the PRNG - caller provides it
                self.mem[(pc + 2) as usize] = hash(pc + 2, seed(4, opcode, (pc & 7) as u8));
                self.mem[(pc + 3) as usize] = opcode;
            }
            _ => {}
        }
    }

    /// Format writes list for comparison.
    pub fn writes_str(&self) -> String {
        if self.writes.is_empty() {
            return "-".to_string();
        }
        let mut parts = Vec::new();
        for &(addr, val) in &self.writes {
            parts.push(format!("{:04X}={:02X}", addr, val));
        }
        parts.join(",")
    }

    /// Format port_out list for comparison.
    pub fn port_out_str(&self) -> String {
        if self.port_out.is_empty() {
            return "-".to_string();
        }
        let mut parts = Vec::new();
        for &(port, val) in &self.port_out {
            parts.push(format!("{:04X}={:02X}", port, val));
        }
        parts.join(",")
    }

    /// Format port_in list for comparison.
    pub fn port_in_str(&self) -> String {
        if self.port_in.is_empty() {
            return "-".to_string();
        }
        let mut parts = Vec::new();
        for &port in &self.port_in {
            parts.push(format!("{:04X}", port));
        }
        parts.join(",")
    }
}

impl Bus for TestBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.writes.push((addr, data));
        self.mem[addr as usize] = data;
    }

    fn io_read(&mut self, port: u16) -> u8 {
        self.port_in.push(port);
        0xFF
    }

    fn io_write(&mut self, port: u16, data: u8) {
        self.port_out.push((port, data));
    }

    fn extra_wait(&mut self, _kind: Cycle, _addr: u16) -> u8 {
        0
    }

    fn on_tick(&mut self, _pins: Pins) {}
}
