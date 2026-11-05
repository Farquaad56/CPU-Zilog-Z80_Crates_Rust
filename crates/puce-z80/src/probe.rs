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
}
