//! ED prefix table semantics.
//!
//! Implemented: IN r,(C) / IN (C), OUT (C),r / OUT (C),0 (E07-01),
//! ADC HL,rp / SBC HL,rp (E07-02).

use super::regs::{hl, r8, rp, set_f, set_hl, set_r8, set_rp};
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
            2 => ed_adc_sbc_hl_rp(cpu, y),
            3 => ed_ld_nn_rp(cpu, y),
            _ => {}
        }
    }
}

/// ADC HL,rp / SBC HL,rp (z==2).
/// p = y >> 1 selects the register pair; q = y & 1 selects ADC(1)/SBC(0).
/// WZ is set to HL_before + 1.
fn ed_adc_sbc_hl_rp(cpu: &mut Cpu, y: u8) {
    let p = y >> 1;
    let q = y & 1;
    let hl_before = hl(cpu);
    let rp_val = rp(cpu, p);

    let (res, flags) = if q == 1 {
        alu::adc16(hl_before, rp_val, cpu.regs.f)
    } else {
        alu::sbc16(hl_before, rp_val, cpu.regs.f)
    };

    set_hl(cpu, res);
    set_f(cpu, flags);
    cpu.regs.wz = hl_before.wrapping_add(1);
}

/// ED: LD (nn),rp / LD rp,(nn) (z==3).
/// p = y >> 1 selects the register pair; q = y & 1 selects store(0)/load(1).
/// WZ is set to nn + 1 after the operation.
fn ed_ld_nn_rp(cpu: &mut Cpu, y: u8) {
    let p = y >> 1;
    let q = y & 1;

    if q == 0 {
        // LD (nn),rp: store register pair at address nn
        let v = rp(cpu, p);
        cpu.eng.tmp_l = v as u8;
        cpu.eng.tmp_h = (v >> 8) as u8;
        // WZ stays at nn for the write; updated after by plan
    } else {
        // LD rp,(nn): load register pair from address nn
        let v = ((cpu.eng.tmp_h as u16) << 8) | (cpu.eng.tmp_l as u16);
        set_rp(cpu, p, v);
        cpu.regs.wz = cpu.regs.wz.wrapping_add(1);
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

    #[test]
    fn e07_02_adc_hl_bc() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.regs.set_bc(0x5678);
        cpu.regs.f = 0; // no carry
        sem_ed(&mut cpu, 0x4A, 0); // ADC HL,BC
        assert_eq!(cpu.regs.hl(), 0x68AC, "ADC HL,BC result");
        assert_eq!(cpu.regs.f & flag::C, 0, "no carry out");
    }

    #[test]
    fn e07_02_adc_hl_bc_carry() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0xFFFF);
        cpu.regs.set_bc(0x0001);
        cpu.regs.f = flag::C; // carry in
        sem_ed(&mut cpu, 0x4A, 0); // ADC HL,BC with carry
        assert_eq!(cpu.regs.hl(), 0x0001, "ADC wraps");
        assert_eq!(cpu.regs.f & flag::C, flag::C, "carry out set");
    }

    #[test]
    fn e07_02_sbc_hl_hl() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.regs.f = 0; // no borrow
        sem_ed(&mut cpu, 0x62, 0); // SBC HL,HL
        assert_eq!(cpu.regs.hl(), 0x0000, "SBC HL,HL = 0");
        assert_eq!(cpu.regs.f & flag::Z, flag::Z, "zero flag set");
        assert_eq!(cpu.regs.f & flag::N, flag::N, "N flag set for SBC");
    }

    #[test]
    fn e07_02_sbc_sp() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.sp = 0x0FFF;
        cpu.regs.f = flag::C; // borrow in
        sem_ed(&mut cpu, 0x72, 0); // SBC HL,SP with borrow
        assert_eq!(cpu.regs.hl(), 0x0000, "SBC HL,SP result");
        assert_eq!(cpu.regs.f & flag::C, 0, "no borrow out");
    }

    #[test]
    fn e07_02_wz() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.regs.set_bc(0x5678);
        sem_ed(&mut cpu, 0x4A, 0); // ADC HL,BC
        assert_eq!(cpu.regs.wz, 0x1235, "WZ = HL_before + 1");

        let mut cpu2 = Cpu::new();
        cpu2.regs.set_hl(0xFFFF);
        cpu2.regs.set_bc(0x0001);
        sem_ed(&mut cpu2, 0x4A, 0); // ADC HL,BC wraps
        assert_eq!(cpu2.regs.wz, 0x0000, "WZ wraps");
    }

    #[test]
    fn e07_03_ld_nn_bc() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x1234);
        sem_ed(&mut cpu, 0x43, 0); // ED 43: LD (nn),BC
        assert_eq!(cpu.eng.tmp_l, 0x34, "LD (nn),BC stores low byte");
        assert_eq!(cpu.eng.tmp_h, 0x12, "LD (nn),BC stores high byte");
    }

    #[test]
    fn e07_03_ld_sp_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x2000;
        cpu.eng.tmp_l = 0xCD;
        cpu.eng.tmp_h = 0xAB;
        sem_ed(&mut cpu, 0x7B, 0); // ED 7B: LD SP,(nn)
        assert_eq!(cpu.regs.sp, 0xABCD, "LD SP,(nn) loads from memory");
    }

    #[test]
    fn e07_03_ld_nn_hl_ed() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x5678);
        sem_ed(&mut cpu, 0x63, 0); // ED 63: LD (nn),HL
        assert_eq!(cpu.eng.tmp_l, 0x78, "LD (nn),HL stores low byte");
        assert_eq!(cpu.eng.tmp_h, 0x56, "LD (nn),HL stores high byte");
    }

    #[test]
    fn e07_03_wz() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x1234;
        cpu.regs.set_bc(0xABCD);
        sem_ed(&mut cpu, 0x43, 0); // ED 43: LD (nn),BC
        assert_eq!(cpu.regs.wz, 0x1234, "WZ unchanged in hook for store");

        let mut cpu2 = Cpu::new();
        cpu2.regs.wz = 0xFFFF;
        cpu2.eng.tmp_l = 0x00;
        cpu2.eng.tmp_h = 0x00;
        sem_ed(&mut cpu2, 0x4B, 0); // ED 4B: LD BC,(nn)
        assert_eq!(cpu2.regs.wz, 0x0000, "WZ wraps after load");
    }

    #[test]
    fn e07_03_ld_de_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.set_de(0x9ABC);
        sem_ed(&mut cpu, 0x53, 0); // ED 53: LD (nn),DE
        assert_eq!(cpu.eng.tmp_l, 0xBC, "LD (nn),DE stores low byte");
        assert_eq!(cpu.eng.tmp_h, 0x9A, "LD (nn),DE stores high byte");
    }

    #[test]
    fn e07_03_ld_bc_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x3000;
        cpu.eng.tmp_l = 0xEF;
        cpu.eng.tmp_h = 0xCD;
        sem_ed(&mut cpu, 0x4B, 0); // ED 4B: LD BC,(nn)
        assert_eq!(cpu.regs.bc(), 0xCDEF, "LD BC,(nn) loads from memory");
    }

    #[test]
    fn e07_03_ld_hl_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x4000;
        cpu.eng.tmp_l = 0x12;
        cpu.eng.tmp_h = 0x34;
        sem_ed(&mut cpu, 0x6B, 0); // ED 6B: LD HL,(nn)
        assert_eq!(cpu.regs.hl(), 0x3412, "LD HL,(nn) loads from memory");
    }

    #[test]
    fn e07_03_ld_nn_sp() {
        let mut cpu = Cpu::new();
        cpu.regs.sp = 0xFFFE;
        sem_ed(&mut cpu, 0x73, 0); // ED 73: LD (nn),SP
        assert_eq!(cpu.eng.tmp_l, 0xFE, "LD (nn),SP stores low byte");
        assert_eq!(cpu.eng.tmp_h, 0xFF, "LD (nn),SP stores high byte");
    }
}
