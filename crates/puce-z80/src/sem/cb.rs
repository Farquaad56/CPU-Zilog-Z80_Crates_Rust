//! CB prefix table semantics: rotations, shifts, BIT, RES, SET.
//!
//! Implements hooks for all 128 CB-prefix instructions.
//! Opcode layout: x = op >> 6 (family), y = (op >> 3) & 7 (operation/bit), z = op & 7 (register).
//! Register encoding: 0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 6=(HL), 7=A.

use super::regs::{r8, set_f, set_r8};
use crate::alu;
use crate::cpu::Cpu;

/// Execute CB-prefix instruction semantics.
pub(crate) fn sem_cb(cpu: &mut Cpu, op: u8, hook_no: u8) {
    if hook_no != 0 {
        return; // Only X0 hooks for CB instructions
    }

    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;

    match x {
        // Rotations and shifts: RLC, RRC, RL, RR, SLA, SRA, SLL, SRL
        0 => sem_cb_rot(cpu, y, z),
        // BIT tests
        1 => sem_cb_bit(cpu, y, z),
        // RES bit clear
        2 => sem_cb_res(cpu, y, z),
        // SET bit set
        3 => sem_cb_set(cpu, y, z),
        _ => unreachable!("invalid CB family"),
    }
}

/// Rotations and shifts: RLC(y=0) RRC(1) RL(2) RR(3) SLA(4) SRA(5) SLL(6) SRL(7).
fn sem_cb_rot(cpu: &mut Cpu, y: u8, z: u8) {
    let v = r8(cpu, z);
    let (res, flags) = alu::rot(y, v, cpu.regs.f);
    set_r8(cpu, z, res);
    set_f(cpu, flags);
}

/// BIT y,r or BIT y,(HL): test bit, set flags.
fn sem_cb_bit(cpu: &mut Cpu, y: u8, z: u8) {
    let v = r8(cpu, z);
    // For (HL), X/Y come from high byte of address; for registers, from the value itself.
    let xy_src = if z == 6 { (cpu.regs.wz >> 8) as u8 } else { v };
    let flags = alu::bit_flags(v, y, cpu.regs.f, xy_src);
    set_f(cpu, flags);
}

/// RES y,r or RES y,(HL): clear bit.
fn sem_cb_res(cpu: &mut Cpu, y: u8, z: u8) {
    let v = r8(cpu, z);
    let res = v & !(1 << y);
    set_r8(cpu, z, res);
}

/// SET y,r or SET y,(HL): set bit.
fn sem_cb_set(cpu: &mut Cpu, y: u8, z: u8) {
    let v = r8(cpu, z);
    let res = v | (1 << y);
    set_r8(cpu, z, res);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regs::flag;

    #[test]
    fn e06_01_rlc_b() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0x85; // 1000 0101
        sem_cb(&mut cpu, 0x00, 0); // RLC B (op=0x00: x=0,y=0,z=0)
        assert_eq!(cpu.regs.b, 0x0B, "RLC B rotates left through carry");
        assert_eq!(cpu.regs.f & flag::C, flag::C, "RLC sets carry from bit 7");
    }

    #[test]
    fn e06_01_rrc_a() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x85; // 1000 0101
        sem_cb(&mut cpu, 0x0F, 0); // RRC A (op=0x0F: x=0,y=1,z=7)
        assert_eq!(cpu.regs.a, 0xC2, "RRC A rotates right through carry");
        assert_eq!(cpu.regs.f & flag::C, flag::C, "RRC sets carry from bit 0");
    }

    #[test]
    fn e06_01_rl_carry_in() {
        let mut cpu = Cpu::new();
        cpu.regs.c = 0x42; // 0100 0010
        cpu.regs.f = flag::C; // Carry set
        sem_cb(&mut cpu, 0x11, 0); // RL C (op=0x11: x=0,y=2,z=1)
        assert_eq!(cpu.regs.c, 0x85, "RL rotates left with carry in");
    }

    #[test]
    fn e06_01_rr_carry_in() {
        let mut cpu = Cpu::new();
        cpu.regs.d = 0x42; // 0100 0010
        cpu.regs.f = flag::C; // Carry set
        sem_cb(&mut cpu, 0x1A, 0); // RR D (op=0x1A: x=0,y=3,z=2)
        assert_eq!(cpu.regs.d, 0xA1, "RR rotates right with carry in");
    }

    #[test]
    fn e06_01_sla_e() {
        let mut cpu = Cpu::new();
        cpu.regs.e = 0x42; // 0100 0010
        sem_cb(&mut cpu, 0x23, 0); // SLA E (op=0x23: x=0,y=4,z=3)
        assert_eq!(cpu.regs.e, 0x84, "SLA shifts left arithmetic");
    }

    #[test]
    fn e06_01_sra_h() {
        let mut cpu = Cpu::new();
        cpu.regs.h = 0xC5; // 1100 0101 (negative)
        sem_cb(&mut cpu, 0x2C, 0); // SRA H (op=0x2C: x=0,y=5,z=4)
        assert_eq!(cpu.regs.h, 0xE2, "SRA shifts right preserving sign");
    }

    #[test]
    fn e06_01_sll_a() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x42; // 0100 0010
        sem_cb(&mut cpu, 0x37, 0); // SLL A (op=0x37: x=0,y=6,z=7)
        assert_eq!(cpu.regs.a, 0x85, "SLL shifts left logical, sets bit 0");
    }

    #[test]
    fn e06_01_srl_l() {
        let mut cpu = Cpu::new();
        cpu.regs.l = 0xC5; // 1100 0101
        sem_cb(&mut cpu, 0x3D, 0); // SRL L (op=0x3D: x=0,y=7,z=5)
        assert_eq!(cpu.regs.l, 0x62, "SRL shifts right logical");
    }

    #[test]
    fn e06_01_srl_hl() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_l = 0xC5; // Simulate memory read into tmp_l
        sem_cb(&mut cpu, 0x3E, 0); // SRL (HL) via z=6 -> tmp_l
        assert_eq!(cpu.eng.tmp_l, 0x62, "SRL (HL) operates on tmp_l");
    }

    #[test]
    fn e06_01_rot_flags() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0xFF; // All bits set
        sem_cb(&mut cpu, 0x00, 0); // RLC B
        assert_eq!(cpu.regs.f & flag::Z, 0, "RLC of 0xFF: Z clear");
        assert_eq!(cpu.regs.f & flag::S, flag::S, "RLC of 0xFF: S set (bit 7)");
        // Parity of 0xFF is even (8 bits set) -> PV set
        assert_eq!(
            cpu.regs.f & flag::PV,
            flag::PV,
            "RLC of 0xFF: PV set (even parity)"
        );
    }

    #[test]
    fn e06_01_bit_test() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0x42; // Bit 1 set, bit 7 clear
        sem_cb(&mut cpu, 0x48, 0); // BIT 1,B (op=0x48: x=1,y=1,z=0)
        assert_eq!(cpu.regs.f & flag::Z, 0, "BIT 1 of 0x42: Z clear (bit set)");
    }

    #[test]
    fn e06_01_res_bit() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0xFF;
        sem_cb(&mut cpu, 0xB8, 0); // RES 7,B (op=0xB8: x=2,y=7,z=0)
        assert_eq!(cpu.regs.b, 0x7F, "RES 7 clears bit 7");
    }

    #[test]
    fn e06_01_set_bit() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0x00;
        sem_cb(&mut cpu, 0xD0, 0); // SET 2,B (op=0xD0: x=3,y=2,z=0)
        assert_eq!(cpu.regs.b, 0x04, "SET 2 sets bit 2");
    }
}
