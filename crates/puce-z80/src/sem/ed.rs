//! ED prefix table semantics.
//!
//! Implemented so far: IN r,(C) / IN (C) and OUT (C),r / OUT (C),0 (E07-01).

use super::regs::{r8, set_f, set_r8};
use crate::alu;
use crate::cpu::Cpu;
use crate::regs::flag;

/// Execute ED-prefix instruction semantics.
pub(crate) fn sem_ed(cpu: &mut Cpu, op: u8, hook_no: u8) {
    if hook_no != 0 {
        return;
    }

    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;

    if x == 1 {
        match z {
            0 => ed_in_c(cpu, y),
            1 => ed_out_c(cpu, y),
            _ => {}
        }
    }
}

/// IN r,(C) (y != 6) and IN (C) (y == 6, value discarded).
/// X0: v = TMP.L; flags S Z Y X P from v, H = N = 0, C unchanged; WZ = BC + 1.
fn ed_in_c(cpu: &mut Cpu, y: u8) {
    let v = cpu.eng.tmp_l;
    let flags = (cpu.regs.f & flag::C)
        | (v & (flag::S | flag::Y | flag::X))
        | if v == 0 { flag::Z } else { 0 }
        | alu::parity(v);
    set_f(cpu, flags);
    if y != 6 {
        set_r8(cpu, y, v);
    }
    cpu.regs.wz = cpu.regs.bc().wrapping_add(1);
}

/// OUT (C),r (y != 6) and OUT (C),0 (y == 6, NMOS).
/// X0: TMP.L = value to output; WZ = BC + 1.
fn ed_out_c(cpu: &mut Cpu, y: u8) {
    cpu.eng.tmp_l = if y == 6 { 0 } else { r8(cpu, y) };
    cpu.regs.wz = cpu.regs.bc().wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e07_01_in_b_c() {
        let mut cpu = Cpu::new();
        cpu.regs.f = flag::C | flag::H | flag::N;
        cpu.eng.tmp_l = 0x80; // input byte from the bus
        sem_ed(&mut cpu, 0x40, 0); // IN B,(C)
        assert_eq!(cpu.regs.b, 0x80, "IN B,(C) loads B");
        assert_eq!(cpu.regs.f & flag::S, flag::S, "S from bit 7");
        assert_eq!(cpu.regs.f & flag::C, flag::C, "C unchanged");
        assert_eq!(cpu.regs.f & (flag::H | flag::N), 0, "H = N = 0");
    }

    #[test]
    fn e07_01_in_f_c() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0x12;
        cpu.regs.f = 0;
        cpu.eng.tmp_l = 0x00;
        sem_ed(&mut cpu, 0x70, 0); // IN (C): value discarded, flags only
        assert_eq!(cpu.regs.b, 0x12, "IN (C) must not change B");
        assert_eq!(cpu.regs.f & flag::Z, flag::Z, "Z set for zero");
        assert_eq!(cpu.regs.f & flag::PV, flag::PV, "even parity for zero");
    }

    #[test]
    fn e07_01_out_c_a() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x5A;
        sem_ed(&mut cpu, 0x79, 0); // OUT (C),A
        assert_eq!(cpu.eng.tmp_l, 0x5A, "OUT (C),A outputs A");
    }

    #[test]
    fn e07_01_out_c_0() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_l = 0xFF;
        sem_ed(&mut cpu, 0x71, 0); // OUT (C),0 (NMOS)
        assert_eq!(cpu.eng.tmp_l, 0x00, "OUT (C),0 outputs zero");
    }

    #[test]
    fn e07_01_wz_bc_plus1() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x12FF);
        sem_ed(&mut cpu, 0x78, 0); // IN A,(C)
        assert_eq!(cpu.regs.wz, 0x1300, "IN: WZ = BC + 1");
        cpu.regs.set_bc(0xFFFF);
        sem_ed(&mut cpu, 0x41, 0); // OUT (C),B
        assert_eq!(cpu.regs.wz, 0x0000, "OUT: WZ = BC + 1 wraps");
    }
}
