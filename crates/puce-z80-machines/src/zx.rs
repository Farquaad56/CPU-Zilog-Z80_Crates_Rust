//! ZX Spectrum 48K minimal machine configuration (docs/specs/35_Z80TEST.md).
//!
//! 16 KiB of ROM (or none), 48 KiB of RAM, port 0xFE without keyboard, 50 Hz frame interrupt,
//! fast tape loading at LD-BYTES (0556h), text capture at RST 10h (0010h) and an optional
//! model of the ULA memory contention.

use puce_z80::{Bus, Cpu, Cycle};

/// Size of the Spectrum 48K ROM.
pub const ROM_SIZE: usize = 16384;
/// T-states per frame of the 48K machine.
pub const FRAME_TSTATES: u32 = 69888;
/// T-states during which INT stays active at the start of a frame.
pub const INT_TSTATES: u32 = 32;
/// Address of the LD-BYTES routine of the 48K ROM.
pub const LD_BYTES: u16 = 0x0556;
/// Address of RST 10h (PRINT-A).
pub const PRINT_A: u16 = 0x0010;
/// Address of CHAN-OPEN in the ROM.
pub const CHAN_OPEN: u16 = 0x1601;
/// Value read from port 0xFE when no key is pressed (EAR/MIC bit low).
pub const PORT_FE_IDLE: u8 = 0xBF;

/// T-states charged for an intercepted routine (the RET that replaces it).
const HOOK_TSTATES: u32 = 10;
/// First contended T-state of a 48K frame.
const CONTENTION_START: u32 = 14335;
/// Contention delay by position inside a group of 8 T-states.
const CONTENTION_PATTERN: [u8; 8] = [6, 5, 4, 3, 2, 1, 0, 0];

/// One block of a `.tap` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapBlock {
    /// Flag byte (0x00 header, 0xFF data).
    pub flag: u8,
    /// Data without the flag byte and the checksum.
    pub data: Vec<u8>,
    /// True when the xor checksum of the block is correct.
    pub checksum_ok: bool,
}

/// Parse the blocks of a `.tap` file.
pub fn parse_tap(bytes: &[u8]) -> Result<Vec<TapBlock>, String> {
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 2 > bytes.len() {
            return Err(format!("truncated block length at offset {}", i));
        }
        let n = usize::from(bytes[i]) | (usize::from(bytes[i + 1]) << 8);
        let body = bytes
            .get(i + 2..i + 2 + n)
            .ok_or_else(|| format!("truncated block of {} bytes at offset {}", n, i))?;
        if n < 2 {
            return Err(format!("block too short ({} bytes) at offset {}", n, i));
        }
        let xor = body.iter().fold(0u8, |a, b| a ^ b);
        blocks.push(TapBlock {
            flag: body[0],
            data: body[1..n - 1].to_vec(),
            checksum_ok: xor == 0,
        });
        i += 2 + n;
    }
    Ok(blocks)
}

/// The bus seen by the CPU: borrows the memory and the counters of the machine.
struct ZxBus<'a> {
    mem: &'a mut [u8; 65536],
    frame_t: &'a mut u32,
    tstates: &'a mut u64,
    fe_out: &'a mut u8,
    contention: bool,
}

impl Bus for ZxBus<'_> {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[usize::from(addr)]
    }

    fn write(&mut self, addr: u16, data: u8) {
        if usize::from(addr) >= ROM_SIZE {
            self.mem[usize::from(addr)] = data;
        }
    }

    fn io_read(&mut self, port: u16) -> u8 {
        if port & 1 == 0 {
            PORT_FE_IDLE
        } else {
            0xFF
        }
    }

    fn io_write(&mut self, port: u16, data: u8) {
        if port & 1 == 0 {
            *self.fe_out = data;
        }
    }

    fn irq(&mut self) -> bool {
        *self.frame_t < INT_TSTATES
    }

    fn on_tick(&mut self, _pins: puce_z80::Pins) {
        *self.frame_t = (*self.frame_t + 1) % FRAME_TSTATES;
        *self.tstates += 1;
    }

    fn extra_wait(&mut self, kind: Cycle, addr: u16) -> u8 {
        let contended_kind = matches!(kind, Cycle::Fetch | Cycle::MemRead | Cycle::MemWrite);
        if !self.contention || !contended_kind || !(0x4000..=0x7FFF).contains(&addr) {
            return 0;
        }
        // `on_tick` has already counted T1 of this cycle.
        let t1 = (*self.frame_t + FRAME_TSTATES - 1) % FRAME_TSTATES;
        contention_delay(t1)
    }
}

/// ULA contention delay for an access whose first T-state is `t` (counted from the frame start).
pub fn contention_delay(t: u32) -> u8 {
    if t < CONTENTION_START {
        return 0;
    }
    let rel = t - CONTENTION_START;
    let (line, col) = (rel / 224, rel % 224);
    if line >= 192 || col >= 128 {
        return 0;
    }
    CONTENTION_PATTERN[(col % 8) as usize]
}

/// Minimal ZX Spectrum 48K.
pub struct ZxMachine {
    /// The CPU.
    pub cpu: Cpu,
    /// 64 KiB address space: ROM at 0000h-3FFFh (read-only), RAM above.
    pub mem: Box<[u8; 65536]>,
    /// True when a ROM image was provided.
    pub rom_loaded: bool,
    /// Blocks of the loaded tape.
    pub tap: Vec<TapBlock>,
    /// Index of the next tape block.
    pub tap_pos: usize,
    /// Characters printed through RST 10h.
    pub out: String,
    /// T-states since power on.
    pub tstates: u64,
    /// T-states since the start of the frame (INT is active while below 32).
    pub frame_t: u32,
    /// Last value written to port 0xFE (border, MIC, speaker).
    pub fe_out: u8,
    contention: bool,
    /// Number of bytes of a PRINT-A control sequence still to skip (AT, TAB take two).
    skip: u8,
}

impl ZxMachine {
    /// Create a machine with a 16 KiB ROM; any other size is refused.
    pub fn new(rom: &[u8]) -> Result<ZxMachine, String> {
        if rom.len() != ROM_SIZE {
            return Err(format!("ROM must be {} bytes, got {}", ROM_SIZE, rom.len()));
        }
        let mut m = ZxMachine::new_romless();
        m.mem[..ROM_SIZE].copy_from_slice(rom);
        m.rom_loaded = true;
        Ok(m)
    }

    /// Create a machine without ROM: the ROM area reads 00h and the routines needed by
    /// z80test (CHAN-OPEN, PRINT-A) are simulated.
    pub fn new_romless() -> ZxMachine {
        ZxMachine {
            cpu: Cpu::new(),
            mem: Box::new([0u8; 65536]),
            rom_loaded: false,
            tap: Vec::new(),
            tap_pos: 0,
            out: String::new(),
            tstates: 0,
            frame_t: 0,
            fe_out: 0,
            contention: false,
            skip: 0,
        }
    }

    /// Enable or disable the ULA memory contention (needed by the timing ROMs only).
    pub fn with_contention(mut self, on: bool) -> ZxMachine {
        self.contention = on;
        self
    }

    /// Parse a `.tap` file and rewind the tape.
    pub fn load_tap(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.tap = parse_tap(bytes)?;
        self.tap_pos = 0;
        Ok(())
    }

    /// Copy `bytes` to `addr` (also in the ROM area) without going through the bus.
    pub fn load_code(&mut self, addr: u16, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            self.mem[usize::from(addr.wrapping_add(i as u16))] = *b;
        }
    }

    fn advance(&mut self, n: u32) {
        self.frame_t = (self.frame_t + n) % FRAME_TSTATES;
        self.tstates += u64::from(n);
    }

    /// Pop the return address as RET does.
    fn ret(&mut self) {
        let sp = self.cpu.regs.sp;
        let lo = self.mem[usize::from(sp)];
        let hi = self.mem[usize::from(sp.wrapping_add(1))];
        self.cpu.regs.pc = (u16::from(hi) << 8) | u16::from(lo);
        self.cpu.regs.sp = sp.wrapping_add(2);
        self.advance(HOOK_TSTATES);
    }

    /// Fast tape load at LD-BYTES: A = expected flag, carry = LOAD, IX = destination,
    /// DE = length. Ends with carry set on success (DE = 0, IX advanced), reset on failure.
    fn fast_load(&mut self) {
        let want = self.cpu.regs.a;
        let load = self.cpu.regs.f & 1 != 0;
        let dest = self.cpu.regs.ix;
        let len = (u16::from(self.cpu.regs.d) << 8) | u16::from(self.cpu.regs.e);
        let mut ok = false;
        while self.tap_pos < self.tap.len() {
            let blk = self.tap[self.tap_pos].clone();
            self.tap_pos += 1;
            if blk.flag != want {
                continue; // the ROM skips blocks with another flag
            }
            ok = blk.checksum_ok && blk.data.len() == usize::from(len);
            if ok && load {
                self.load_code(dest, &blk.data);
            }
            break;
        }
        let regs = &mut self.cpu.regs;
        if ok {
            regs.ix = dest.wrapping_add(len);
            regs.d = 0;
            regs.e = 0;
            regs.f |= 1;
        } else {
            regs.f &= !1;
        }
        self.ret();
    }

    /// RST 10h: capture the character in A, interpreting the control codes of PRINT-A.
    fn print_a(&mut self) {
        let ch = self.cpu.regs.a;
        if self.skip > 0 {
            self.skip -= 1;
        } else {
            match ch {
                13 => self.out.push('\n'),
                22 | 23 => self.skip = 2,
                127 => self.out.push_str("(c)"),
                32..=126 => self.out.push(char::from(ch)),
                _ => {}
            }
        }
        self.ret();
    }

    /// Execute one instruction (or one intercepted routine) and return the T-states used.
    pub fn step(&mut self) -> u32 {
        match self.cpu.regs.pc {
            PRINT_A => {
                self.print_a();
                return HOOK_TSTATES;
            }
            LD_BYTES => {
                self.fast_load();
                return HOOK_TSTATES;
            }
            CHAN_OPEN if !self.rom_loaded => {
                self.ret();
                return HOOK_TSTATES;
            }
            _ => {}
        }
        let mut bus = ZxBus {
            mem: &mut self.mem,
            frame_t: &mut self.frame_t,
            tstates: &mut self.tstates,
            fe_out: &mut self.fe_out,
            contention: self.contention,
        };
        self.cpu.step(&mut bus)
    }

    /// Run until PC reaches `stop_pc`, the CPU halts, or `max_tstates` have elapsed.
    /// Returns true when `stop_pc` was reached.
    pub fn run_until_pc(&mut self, stop_pc: u16, max_tstates: u64) -> bool {
        let limit = self.tstates + max_tstates;
        while self.tstates < limit {
            if self.cpu.regs.pc == stop_pc {
                return true;
            }
            if self.cpu.regs.halted {
                return false;
            }
            self.step();
        }
        self.cpu.regs.pc == stop_pc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap_block(flag: u8, data: &[u8]) -> Vec<u8> {
        let mut body = vec![flag];
        body.extend_from_slice(data);
        let xor = body.iter().fold(0u8, |a, b| a ^ b);
        body.push(xor);
        let mut out = vec![body.len() as u8, (body.len() >> 8) as u8];
        out.extend(body);
        out
    }

    #[test]
    fn e13_16_rom_size_check() {
        assert!(ZxMachine::new(&[0u8; 100]).is_err());
        assert!(ZxMachine::new(&vec![0u8; ROM_SIZE]).is_ok());
        assert!(ZxMachine::new(&vec![0u8; ROM_SIZE + 1]).is_err());
    }

    #[test]
    fn e13_16_tap_parse() {
        let mut tap = tap_block(0x00, &[3, b'a', b'b']);
        tap.extend(tap_block(0xFF, &[1, 2, 3, 4]));
        let blocks = parse_tap(&tap).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].flag, 0x00);
        assert_eq!(blocks[0].data, vec![3, b'a', b'b']);
        assert_eq!(blocks[1].flag, 0xFF);
        assert_eq!(blocks[1].data, vec![1, 2, 3, 4]);
        assert!(blocks.iter().all(|b| b.checksum_ok));
        let mut bad = tap.clone();
        *bad.last_mut().unwrap() ^= 0xFF;
        assert!(!parse_tap(&bad).unwrap()[1].checksum_ok);
        assert!(parse_tap(&tap[..tap.len() - 1]).is_err());
    }

    #[test]
    fn e13_16_fastload_copies_block() {
        // fake ROM: LD A,FFh; SCF; LD IX,9000h; LD DE,4; CALL 0556h; HALT
        let mut rom = vec![0u8; ROM_SIZE];
        let prog = [
            0x3E, 0xFF, 0x37, 0xDD, 0x21, 0x00, 0x90, 0x11, 0x04, 0x00, 0xCD, 0x56, 0x05, 0x76,
        ];
        rom[..prog.len()].copy_from_slice(&prog);
        let mut m = ZxMachine::new(&rom).unwrap();
        m.cpu.regs.sp = 0xFF00;
        m.load_tap(&tap_block(0xFF, &[0xDE, 0xAD, 0xBE, 0xEF]))
            .unwrap();
        for _ in 0..20 {
            if m.cpu.regs.halted {
                break;
            }
            m.step();
        }
        assert!(m.cpu.regs.halted);
        assert_eq!(&m.mem[0x9000..0x9004], &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(m.cpu.regs.f & 1, 1, "carry set = success");
        assert_eq!(m.cpu.regs.ix, 0x9004);
        assert_eq!((m.cpu.regs.d, m.cpu.regs.e), (0, 0));
    }

    #[test]
    fn e13_16_fastload_wrong_length_fails() {
        let mut m = ZxMachine::new_romless();
        m.load_tap(&tap_block(0xFF, &[1, 2, 3])).unwrap();
        m.cpu.regs.a = 0xFF;
        m.cpu.regs.f = 1;
        m.cpu.regs.e = 4;
        m.cpu.regs.sp = 0xFF00;
        m.mem[0xFF00] = 0x00;
        m.mem[0xFF01] = 0x80;
        m.cpu.regs.pc = LD_BYTES;
        m.step();
        assert_eq!(m.cpu.regs.f & 1, 0);
        assert_eq!(m.cpu.regs.pc, 0x8000);
    }

    #[test]
    fn e13_16_rst10_capture() {
        let mut m = ZxMachine::new_romless();
        // LD A,'X'; RST 10h; LD A,13; RST 10h; HALT
        m.load_code(0x8000, &[0x3E, b'X', 0xD7, 0x3E, 13, 0xD7, 0x76]);
        m.cpu.regs.pc = 0x8000;
        m.cpu.regs.sp = 0xFF00;
        for _ in 0..20 {
            if m.cpu.regs.halted {
                break;
            }
            m.step();
        }
        assert_eq!(m.out, "X\n");
    }

    #[test]
    fn e13_16_rst10_skips_tab_arguments() {
        let mut m = ZxMachine::new_romless();
        // PRINT CHR$ 23, 30, 1, "OK" -> the two bytes after 23 are not text
        let mut prog = Vec::new();
        for ch in [23u8, 30, 1, b'O', b'K'] {
            prog.extend_from_slice(&[0x3E, ch, 0xD7]);
        }
        prog.push(0x76);
        m.load_code(0x8000, &prog);
        m.cpu.regs.pc = 0x8000;
        m.cpu.regs.sp = 0xFF00;
        for _ in 0..30 {
            if m.cpu.regs.halted {
                break;
            }
            m.step();
        }
        assert_eq!(m.out, "OK");
    }

    #[test]
    fn e13_16_chan_open_hook_without_rom() {
        let mut m = ZxMachine::new_romless();
        m.load_code(0x8000, &[0xCD, 0x01, 0x16, 0x76]); // CALL 1601h; HALT
        m.cpu.regs.pc = 0x8000;
        m.cpu.regs.sp = 0xFF00;
        for _ in 0..10 {
            if m.cpu.regs.halted {
                break;
            }
            m.step();
        }
        assert!(m.cpu.regs.halted);
        assert_eq!(m.cpu.regs.pc, 0x8003);
    }

    #[test]
    fn e13_16_frame_interrupt() {
        let mut m = ZxMachine::new_romless();
        // IM 1; 16 NOPs (past the first INT window); EI; JR $
        let mut prog = vec![0xED, 0x56];
        prog.extend(std::iter::repeat_n(0x00, 16));
        prog.extend_from_slice(&[0xFB, 0x18, 0xFE]);
        m.load_code(0x8000, &prog);
        m.cpu.regs.pc = 0x8000;
        m.cpu.regs.sp = 0xFF00;
        assert!(m.run_until_pc(0x0038, 200_000), "interrupt never taken");
        let t = m.tstates;
        assert!(
            (u64::from(FRAME_TSTATES)..u64::from(FRAME_TSTATES) + 40).contains(&t),
            "INT taken at T{}",
            t
        );
    }

    #[test]
    fn e13_16_io_fe() {
        let mut m = ZxMachine::new_romless();
        m.load_code(0x8000, &[0xDB, 0xFE, 0x76]); // IN A,(FEh); HALT
        m.cpu.regs.pc = 0x8000;
        m.cpu.regs.a = 0x00;
        m.step();
        assert_eq!(m.cpu.regs.a, 0xBF);
    }

    #[test]
    fn e13_16_contention_table() {
        assert_eq!(contention_delay(0), 0);
        assert_eq!(contention_delay(14335), 6);
        assert_eq!(contention_delay(14336), 5);
        assert_eq!(contention_delay(14341), 0);
        assert_eq!(contention_delay(14335 + 128), 0);
        assert_eq!(contention_delay(14335 + 224), 6);
        assert_eq!(contention_delay(14335 + 224 * 192), 0);
    }

    #[test]
    fn e13_16_contention_slows_screen_code() {
        // LD A,(4000h) at the first contended T-state costs 6 more T-states.
        let run = |on: bool| {
            let mut m = ZxMachine::new_romless().with_contention(on);
            m.load_code(0x8000, &[0x3A, 0x00, 0x40, 0x76]);
            m.cpu.regs.pc = 0x8000;
            m.cpu.regs.sp = 0xFF00;
            m.frame_t = 14335 - 10; // fetch at 8000h is uncontended; the read at 4000h is not
            m.step()
        };
        let (off, on) = (run(false), run(true));
        assert_eq!(off, 13);
        assert!(
            on > off,
            "contention should add wait states ({} vs {})",
            on,
            off
        );
    }
}
