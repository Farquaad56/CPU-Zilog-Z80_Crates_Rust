//! CP/M machine configuration with BDOS support.
//!
//! 64 KiB of RAM, a `.com` / `.cim` program loaded at 0100h, a BDOS emulated at 0005h
//! (functions 2 and 9 only) and a warm boot at 0000h.

use puce_z80::{Bus, Cpu};

/// Why [`CpmMachine::run`] returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunEnd {
    /// The program jumped to the warm boot address 0000h.
    Exited,
    /// The instruction budget was used up.
    MaxInstr,
    /// The CPU executed HALT.
    Halted,
}

/// Plain 64 KiB RAM seen by the CPU through the `Bus` trait.
struct RamBus<'a>(&'a mut [u8; 65536]);

impl Bus for RamBus<'_> {
    fn read(&mut self, addr: u16) -> u8 {
        self.0[usize::from(addr)]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.0[usize::from(addr)] = data;
    }
}

/// Minimal CP/M-80 machine for PRELIM, ZEXDOC and ZEXALL.
pub struct CpmMachine {
    /// The CPU.
    pub cpu: Cpu,
    /// 64 KiB of RAM.
    pub ram: Box<[u8; 65536]>,
    /// Everything printed through the BDOS.
    pub out: Vec<u8>,
    /// True once the program reached the warm boot address.
    pub exited: bool,
    /// Output lines ending with `OK`.
    pub tests_ok: u32,
    /// Output lines containing `ERROR`.
    pub tests_err: u32,
    /// Instructions executed (a BDOS call counts as one RET).
    pub instr: u64,
    /// T-states elapsed.
    pub tstates: u64,
    /// Raw mode: no warm boot or BDOS hooks, the program owns all of memory.
    pub raw: bool,
    /// Start of the line being printed, inside `out`.
    line_start: usize,
}

impl Default for CpmMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl CpmMachine {
    /// Warm boot address.
    const WARM_BOOT: u16 = 0x0000;
    /// BDOS entry point.
    const BDOS: u16 = 0x0005;
    /// T-states charged for a BDOS call (the RET it replaces).
    const BDOS_TSTATES: u32 = 10;

    /// Create a machine with empty RAM, JP 0000h at 0000h and RET at 0005h.
    pub fn new() -> Self {
        let mut ram = Box::new([0u8; 65536]);
        ram[0] = 0xC3; // JP 0000h: warm boot
        ram[5] = 0xC9; // RET: BDOS
        CpmMachine {
            cpu: Cpu::new(),
            ram,
            out: Vec::new(),
            exited: false,
            tests_ok: 0,
            tests_err: 0,
            instr: 0,
            tstates: 0,
            raw: false,
            line_start: 0,
        }
    }

    /// Load a program at 0100h, with PC = 0100h and SP = 0FFFEh.
    pub fn load_com(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(0xFF00);
        self.ram[0x0100..0x0100 + n].copy_from_slice(&bytes[..n]);
        self.cpu.regs.pc = 0x0100;
        self.cpu.regs.sp = 0xFFFE;
        self.exited = false;
    }

    /// Load a raw image at 0000h, with PC = 0000h and no CP/M hooks.
    pub fn load_raw(&mut self, bytes: &[u8]) {
        self.ram.fill(0);
        let n = bytes.len().min(0x10000);
        self.ram[..n].copy_from_slice(&bytes[..n]);
        self.cpu.regs.pc = 0x0000;
        self.cpu.regs.sp = 0xFFFE;
        self.raw = true;
        self.exited = false;
    }

    /// Execute one instruction (or one BDOS call) and return the T-states used.
    pub fn step(&mut self) -> u32 {
        let pc = self.cpu.regs.pc;
        if !self.raw && pc == Self::WARM_BOOT {
            self.exited = true;
            return 0;
        }
        let ts = if !self.raw && pc == Self::BDOS {
            self.bdos();
            Self::BDOS_TSTATES
        } else {
            self.cpu.step(&mut RamBus(&mut self.ram))
        };
        self.instr += 1;
        self.tstates += u64::from(ts);
        ts
    }

    /// Run up to `max_instr` instructions.
    pub fn run(&mut self, max_instr: u64) -> RunEnd {
        for _ in 0..max_instr {
            if self.exited {
                return RunEnd::Exited;
            }
            if self.cpu.regs.halted {
                return RunEnd::Halted;
            }
            self.step();
        }
        if self.exited {
            RunEnd::Exited
        } else {
            RunEnd::MaxInstr
        }
    }

    /// BDOS call: function 2 (print E) and 9 (print the `$`-terminated string at DE),
    /// then return to the caller as RET would.
    fn bdos(&mut self) {
        let regs = &self.cpu.regs;
        match regs.c {
            2 => {
                let ch = regs.e;
                self.put(ch);
            }
            9 => {
                let mut addr = (u16::from(regs.d) << 8) | u16::from(regs.e);
                for _ in 0..=0xFFFFu32 {
                    let ch = self.ram[usize::from(addr)];
                    if ch == b'$' {
                        break;
                    }
                    self.put(ch);
                    addr = addr.wrapping_add(1);
                }
            }
            _ => {}
        }
        let sp = self.cpu.regs.sp;
        let lo = self.ram[usize::from(sp)];
        let hi = self.ram[usize::from(sp.wrapping_add(1))];
        self.cpu.regs.pc = (u16::from(hi) << 8) | u16::from(lo);
        self.cpu.regs.sp = sp.wrapping_add(2);
    }

    /// Append one character to the output and count completed OK / ERROR lines.
    fn put(&mut self, ch: u8) {
        self.out.push(ch);
        if ch == b'\n' {
            let line = &self.out[self.line_start..];
            let text = String::from_utf8_lossy(line);
            let text = text.trim_end();
            if text.ends_with("OK") {
                self.tests_ok += 1;
            } else if text.contains("ERROR") {
                self.tests_err += 1;
            }
            self.line_start = self.out.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `LD DE,msg; LD C,9; CALL 5; JP 0` followed by the `$`-terminated message.
    fn print_prog(msg: &[u8]) -> Vec<u8> {
        let mut p = vec![
            0x11, 0x0B, 0x01, 0x0E, 0x09, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00,
        ];
        p.extend_from_slice(msg);
        p.push(b'$');
        p
    }

    #[test]
    fn e13_02_bdos_fn2() {
        let mut m = CpmMachine::new();
        m.load_com(&[0x1E, b'A', 0x0E, 0x02, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00]);
        assert_eq!(m.run(1000), RunEnd::Exited);
        assert_eq!(m.out, b"A");
    }

    #[test]
    fn e13_02_bdos_fn9() {
        let mut m = CpmMachine::new();
        m.load_com(&print_prog(b"Hello"));
        assert_eq!(m.run(1000), RunEnd::Exited);
        assert_eq!(m.out, b"Hello");
    }

    #[test]
    fn e13_02_warm_boot_exit() {
        let mut m = CpmMachine::new();
        m.load_com(&[0xC3, 0x00, 0x00]);
        assert_eq!(m.run(10), RunEnd::Exited);
        assert!(m.exited);
    }

    #[test]
    fn e13_02_load_com_pc() {
        let mut m = CpmMachine::new();
        m.load_com(&[0x00]);
        assert_eq!(m.cpu.regs.pc, 0x0100);
        assert_eq!(m.cpu.regs.sp, 0xFFFE);
        assert_eq!(m.ram[0x0100], 0x00);
    }

    #[test]
    fn e13_02_counts_ok_lines() {
        let mut m = CpmMachine::new();
        m.load_com(&print_prog(b"xx OK\r\nyy ERROR\r\nzz OK\r\n"));
        m.run(1000);
        assert_eq!((m.tests_ok, m.tests_err), (2, 1));
    }

    #[test]
    fn e13_02_max_instr() {
        let mut m = CpmMachine::new();
        m.load_com(&[0xC3, 0x00, 0x01]);
        assert_eq!(m.run(100), RunEnd::MaxInstr);
        assert!(!m.exited);
    }

    #[test]
    fn e13_02_counters() {
        let mut m = CpmMachine::new();
        m.load_com(&[0x00, 0x00, 0xC3, 0x00, 0x00]);
        m.run(100);
        assert_eq!(m.instr, 3);
        assert_eq!(m.tstates, 4 + 4 + 10);
    }
}
