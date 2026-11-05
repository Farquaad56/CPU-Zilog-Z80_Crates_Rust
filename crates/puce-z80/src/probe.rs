//! Inspection journal (feature `inspect`): last ALU operation (E12-01).
//!
//! With the feature off the recording function is an empty inline function, so the
//! instruction semantics are unchanged.
#![allow(dead_code)]

use crate::cpu::Cpu;

/// Record of the last ALU operation executed by the CPU.
///
/// `op`: 0..7 = ADD ADC SUB SBC AND XOR OR CP; 8 = INC; 9 = DEC; 16..23 = RLC RRC RL RR SLA SRA
/// SLL SRL; 32 = ADD16; 33 = ADC16; 34 = SBC16; 40 = DAA; 41 = NEG; 42 = CPL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AluRecord {
    /// Operation code (see the type documentation).
    pub op: u8,
    /// Operand width in bits: 8 or 16.
    pub w: u8,
    /// First operand.
    pub a: u16,
    /// Second operand (0 when the operation has only one).
    pub b: u16,
    /// Carry flag going in (0 or 1).
    pub c: u8,
    /// Result.
    pub r: u16,
    /// Flags before the operation.
    pub f_in: u8,
    /// Flags after the operation.
    pub f_out: u8,
}

/// Build an 8-bit record (carry-in taken from `f_in`).
#[inline(always)]
pub(crate) fn rec8(op: u8, a: u8, b: u8, r: u8, f_in: u8, f_out: u8) -> AluRecord {
    AluRecord {
        op,
        w: 8,
        a: u16::from(a),
        b: u16::from(b),
        c: f_in & 1,
        r: u16::from(r),
        f_in,
        f_out,
    }
}

/// Build a 16-bit record (carry-in taken from `f_in`).
#[inline(always)]
pub(crate) fn rec16(op: u8, a: u16, b: u16, r: u16, f_in: u8, f_out: u8) -> AluRecord {
    AluRecord {
        op,
        w: 16,
        a,
        b,
        c: f_in & 1,
        r,
        f_in,
        f_out,
    }
}

/// Store the record as the CPU's last ALU operation.
#[cfg(feature = "inspect")]
#[inline(always)]
pub(crate) fn alu(cpu: &mut Cpu, record: AluRecord) {
    cpu.last_alu = Some(record);
}

/// No-op when the `inspect` feature is off.
#[cfg(not(feature = "inspect"))]
#[inline(always)]
pub(crate) fn alu(_cpu: &mut Cpu, _record: AluRecord) {}

#[cfg(feature = "inspect")]
impl Cpu {
    /// The last ALU operation executed, or `None` before any.
    pub fn last_alu(&self) -> Option<AluRecord> {
        self.last_alu
    }
}

/// Ring of the 32 most recent values, newest first when iterated.
#[derive(Clone, Copy, Debug, Default)]
struct Ring {
    v: [u16; 32],
    head: u8,
    count: u8,
}

impl Ring {
    fn push(&mut self, x: u16) {
        self.v[usize::from(self.head & 31)] = x;
        self.head = (self.head + 1) & 31;
        if self.count < 32 {
            self.count += 1;
        }
    }

    fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0..self.count).map(move |i| self.v[usize::from(self.head.wrapping_sub(1 + i) & 31)])
    }
}

/// Start addresses of the 32 most recent instructions (HALT cycles excluded).
#[derive(Clone, Copy, Debug, Default)]
pub struct History {
    ring: Ring,
}

impl History {
    /// Record the PC of an instruction start.
    pub(crate) fn push(&mut self, pc: u16) {
        self.ring.push(pc);
    }

    /// Entries, most recent first.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.ring.iter()
    }
}

/// Addresses of the 32 most recent memory writes.
#[derive(Clone, Copy, Debug, Default)]
pub struct WriteLog {
    ring: Ring,
}

impl WriteLog {
    /// Record a write address.
    pub(crate) fn push(&mut self, addr: u16) {
        self.ring.push(addr);
    }

    /// Entries, most recent first.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.ring.iter()
    }
}

/// Counters and journals kept by the CPU when the feature is on.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Journal {
    pub(crate) history: History,
    pub(crate) writes: WriteLog,
    pub(crate) insns: u64,
    pub(crate) tstates: u64,
}

#[cfg(feature = "inspect")]
impl Cpu {
    /// Start PCs of the last 32 instructions, most recent first (HALT cycles not included).
    pub fn pc_history(&self) -> impl Iterator<Item = u16> + '_ {
        self.journal.history.iter()
    }

    /// Addresses of the last 32 memory writes, most recent first.
    pub fn recent_writes(&self) -> impl Iterator<Item = u16> + '_ {
        self.journal.writes.iter()
    }

    /// Number of instructions started (HALT cycles and interrupt acknowledges excluded).
    pub fn instruction_count(&self) -> u64 {
        self.journal.insns
    }

    /// Number of T-states elapsed, wait states included.
    pub fn tstate_count(&self) -> u64 {
        self.journal.tstates
    }
}

#[cfg(all(test, feature = "inspect"))]
mod tests {
    use crate::bus::Bus;
    use crate::{Cpu, SimpleRam};

    fn setup(prog: &[u8]) -> (Cpu, SimpleRam) {
        let mut ram = SimpleRam::new();
        for (i, b) in prog.iter().enumerate() {
            ram.write(0x0100 + i as u16, *b);
        }
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x0100;
        cpu.regs.sp = 0x8000;
        cpu.regs.f = 0;
        (cpu, ram)
    }

    fn collect(it: impl Iterator<Item = u16>) -> ([u16; 32], usize) {
        let mut a = [0u16; 32];
        let mut n = 0;
        for x in it {
            a[n] = x;
            n += 1;
        }
        (a, n)
    }

    #[test]
    fn e12_01_none_initially() {
        assert_eq!(Cpu::new().last_alu(), None);
    }

    #[test]
    fn e12_01_add_recorded() {
        let (mut cpu, mut ram) = setup(&[0x80]);
        cpu.regs.a = 0x10;
        cpu.regs.b = 0x20;
        cpu.step(&mut ram);
        let r = cpu.last_alu().unwrap();
        assert_eq!((r.op, r.w), (0, 8));
        assert_eq!((r.a, r.b, r.r), (0x10, 0x20, 0x30));
        assert_eq!(r.f_in, 0);
        assert_eq!(r.f_out, cpu.regs.f);
    }

    #[test]
    fn e12_01_inc_recorded() {
        let (mut cpu, mut ram) = setup(&[0x04]);
        cpu.regs.b = 0x0F;
        cpu.step(&mut ram);
        let r = cpu.last_alu().unwrap();
        assert_eq!((r.op, r.w, r.a, r.r), (8, 8, 0x0F, 0x10));
        assert_eq!(r.f_out, cpu.regs.f);
    }

    #[test]
    fn e12_01_add16_recorded() {
        let (mut cpu, mut ram) = setup(&[0x19]);
        cpu.regs.h = 0x10;
        cpu.regs.l = 0x00;
        cpu.regs.d = 0x02;
        cpu.regs.e = 0x34;
        cpu.step(&mut ram);
        let r = cpu.last_alu().unwrap();
        assert_eq!((r.op, r.w), (32, 16));
        assert_eq!((r.a, r.b, r.r), (0x1000, 0x0234, 0x1234));
        assert_eq!(r.f_out, cpu.regs.f);
    }

    #[test]
    fn e12_01_behavior_unchanged() {
        let (mut cpu, mut ram) = setup(&[0x80, 0x04]);
        cpu.regs.a = 0x10;
        cpu.regs.b = 0x20;
        cpu.step(&mut ram);
        assert_eq!(cpu.regs.a, 0x30);
        cpu.step(&mut ram);
        assert_eq!(cpu.regs.b, 0x21);
        assert_eq!(cpu.regs.pc, 0x0102);
    }

    #[test]
    fn e12_02_history_order() {
        let (mut cpu, mut ram) = setup(&[0x00, 0x00, 0x00]);
        for _ in 0..3 {
            cpu.step(&mut ram);
        }
        let (h, n) = collect(cpu.pc_history());
        assert_eq!(n, 3);
        assert_eq!(h[..3], [0x0102, 0x0101, 0x0100]);
    }

    #[test]
    fn e12_02_history_wraps() {
        let (mut cpu, mut ram) = setup(&[0x00; 100]);
        for _ in 0..100 {
            cpu.step(&mut ram);
        }
        let (h, n) = collect(cpu.pc_history());
        assert_eq!(n, 32);
        assert_eq!(h[0], 0x0100 + 99);
        assert_eq!(h[31], 0x0100 + 68);
    }

    #[test]
    fn e12_02_write_log() {
        let (mut cpu, mut ram) = setup(&[0x77]);
        cpu.regs.h = 0x40;
        cpu.regs.l = 0x10;
        cpu.regs.a = 0x55;
        cpu.step(&mut ram);
        let (w, n) = collect(cpu.recent_writes());
        assert_eq!((n, w[0]), (1, 0x4010));
    }

    #[test]
    fn e12_02_counters() {
        let (mut cpu, mut ram) = setup(&[0x01, 0x34, 0x12]);
        cpu.step(&mut ram);
        assert_eq!(cpu.instruction_count(), 1);
        assert_eq!(cpu.tstate_count(), 10);
    }

    #[test]
    fn e12_02_halt_not_counted() {
        let (mut cpu, mut ram) = setup(&[0x76]);
        for _ in 0..6 {
            cpu.step(&mut ram);
        }
        assert_eq!(cpu.pc_history().count(), 1);
        assert_eq!(cpu.instruction_count(), 1);
    }
}
