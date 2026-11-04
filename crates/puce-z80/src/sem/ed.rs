//! ED prefix table semantics.
//!
//! Implemented: IN r,(C) / IN (C), OUT (C),r / OUT (C),0 (E07-01),
//! ADC HL,rp / SBC HL,rp (E07-02).

use super::regs::{hl, r8, rp, set_f, set_hl, set_r8, set_rp};
use crate::alu;
use crate::cpu::Cpu;
use crate::regs::flag;

/// IM mode table: [0,0,1,2,0,0,1,2] indexed by y.
const IM_TABLE: [u8; 8] = [0, 0, 1, 2, 0, 0, 1, 2];

/// Execute ED-prefix instruction semantics.
pub(crate) fn sem_ed(cpu: &mut Cpu, op: u8, hook_no: u8) {
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;

    // Block LD / CP family (ED A0/A1/A8/A9/B0/B1/B8/B9): LDI LDD LDIR LDDR CPI CPD CPIR CPDR.
    if x == 2 && y >= 4 && z < 2 {
        if z == 0 {
            ed_block_ld(cpu, y, hook_no);
        } else {
            ed_block_cp(cpu, y, hook_no);
        }
        return;
    }

    if hook_no != 0 {
        return;
    }

    if x == 1 {
        match z {
            0 => ed_in_c(cpu, y),
            1 => ed_out_c(cpu, y),
            2 => ed_adc_sbc_hl_rp(cpu, y),
            3 => ed_ld_nn_rp(cpu, y),
            4 => ed_neg(cpu),
            5 => ed_retn(cpu),
            6 => ed_im(cpu, y),
            7 => match y {
                0 => ed_ld_i_a(cpu),
                1 => ed_ld_r_a(cpu),
                2 => ed_ld_a_i(cpu),
                3 => ed_ld_a_r(cpu),
                4 => ed_rrd(cpu),
                5 => ed_rld(cpu),
                _ => {} // ED NOP (y=6,7)
            },
            _ => {} // Other ED opcodes not yet implemented
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

/// NEG (ED 44): two's complement negate of A.
/// Flags: S,Z,Y,X from result; H=(A^res)&H; PV if A==0x80; N=1; C if A!=0.
fn ed_neg(cpu: &mut Cpu) {
    let a = cpu.regs.a;
    let (res, flags) = alu::neg(a);
    cpu.regs.a = res;
    set_f(cpu, flags);
}

/// RETN / RETI (ED 45 / ED 4D): return from interrupt.
/// Plan has already popped the address into WZ. Hook sets PC=WZ and IFF1=IFF2.
fn ed_retn(cpu: &mut Cpu) {
    cpu.regs.pc = cpu.regs.wz;
    cpu.regs.iff1 = cpu.regs.iff2;
}

/// IM n (ED 46/4E/56/5E/66/6E/76/7E): set interrupt mode.
/// IM = [0,0,1,2,0,0,1,2][y].
fn ed_im(cpu: &mut Cpu, y: u8) {
    cpu.regs.im = IM_TABLE[y as usize];
}

/// LD I,A (ED 47): copy A to interrupt vector register.
fn ed_ld_i_a(cpu: &mut Cpu) {
    cpu.regs.i = cpu.regs.a;
}

/// LD R,A (ED 4F): copy A to refresh register.
fn ed_ld_r_a(cpu: &mut Cpu) {
    cpu.regs.r = cpu.regs.a;
}

/// LD A,I (ED 57): load interrupt vector into A, set flags.
/// Flags: S,Z,Y,X from value; H=N=0; PV=IFF2; C unchanged.
fn ed_ld_a_i(cpu: &mut Cpu) {
    let v = cpu.regs.i;
    cpu.regs.a = v;
    let mut flags = (cpu.regs.f & flag::C) | (v & 0xA8);
    if v == 0 {
        flags |= flag::Z;
    }
    if cpu.regs.iff2 {
        flags |= flag::PV;
    }
    set_f(cpu, flags);
}

/// LD A,R (ED 5F): load refresh register into A, set flags.
/// Same flag behavior as LD A,I.
fn ed_ld_a_r(cpu: &mut Cpu) {
    let v = cpu.regs.r;
    cpu.regs.a = v;
    let mut flags = (cpu.regs.f & flag::C) | (v & 0xA8);
    if v == 0 {
        flags |= flag::Z;
    }
    if cpu.regs.iff2 {
        flags |= flag::PV;
    }
    set_f(cpu, flags);
}

/// RRD (ED 67): rotate right deposit.
/// Memory at HL is rotated right by 4 bits into A's low nibble;
/// A's high nibble goes to memory's high nibble.
/// Plan reads memory into tmp_l before hook; hook writes new value back via tmp_l.
fn ed_rrd(cpu: &mut Cpu) {
    let m = cpu.eng.tmp_l; // byte read from (HL)
    let a = cpu.regs.a;
    // New memory byte: high nibble from A's low nibble, low nibble from old memory's high nibble
    let new_mem = ((a & 0x0F) << 4) | (m >> 4);
    // New A: high nibble unchanged, low nibble from old memory's low nibble
    let new_a = (a & 0xF0) | (m & 0x0F);
    cpu.regs.a = new_a;
    cpu.eng.tmp_l = new_mem; // will be written back to (HL) by plan

    // Flags: S,Z,Y,X from new A; H=N=0; PV=parity(new A); C unchanged
    let mut flags = (cpu.regs.f & flag::C) | (new_a & 0xA8);
    if new_a == 0 {
        flags |= flag::Z;
    }
    flags |= alu::parity(new_a);
    set_f(cpu, flags);

    cpu.regs.wz = hl(cpu).wrapping_add(1);
}

/// RLD (ED 6F): rotate left deposit.
/// Memory at HL is rotated left by 4 bits into A's low nibble;
/// A's low nibble goes to memory's low nibble.
fn ed_rld(cpu: &mut Cpu) {
    let m = cpu.eng.tmp_l; // byte read from (HL)
    let a = cpu.regs.a;
    // New memory byte: high nibble from old memory's low nibble, low nibble from A's low nibble
    let new_mem = ((m & 0x0F) << 4) | (a & 0x0F);
    // New A: high nibble unchanged, low nibble from old memory's high nibble
    let new_a = (a & 0xF0) | (m >> 4);
    cpu.regs.a = new_a;
    cpu.eng.tmp_l = new_mem; // will be written back to (HL) by plan

    // Flags: S,Z,Y,X from new A; H=N=0; PV=parity(new A); C unchanged
    let mut flags = (cpu.regs.f & flag::C) | (new_a & 0xA8);
    if new_a == 0 {
        flags |= flag::Z;
    }
    flags |= alu::parity(new_a);
    set_f(cpu, flags);

    cpu.regs.wz = hl(cpu).wrapping_add(1);
}

/// Repeat step shared by LDxR / CPxR: PC = PC - 2, WZ = PC + 1 (new PC).
fn block_repeat(cpu: &mut Cpu) {
    cpu.regs.pc = cpu.regs.pc.wrapping_sub(2);
    cpu.regs.wz = cpu.regs.pc.wrapping_add(1);
    cpu.eng.cond = true;
}

/// LDI / LDD / LDIR / LDDR (y = 4..7, z = 0).
/// X0: TMP.L = byte read from (HL); flags from (byte + A). The plan then writes TMP.L to (DE).
/// X1: HL, DE +/-1, BC - 1; for the repeating forms with BC != 0, PC -= 2 and WZ = PC + 1.
fn ed_block_ld(cpu: &mut Cpu, y: u8, hook_no: u8) {
    let dec = y & 1 == 1;
    let rep = y >= 6;
    if hook_no == 0 {
        let v = cpu.eng.tmp_l;
        let n = v.wrapping_add(cpu.regs.a);
        let bc_after = cpu.regs.bc().wrapping_sub(1);
        let flags = (cpu.regs.f & (flag::S | flag::Z | flag::C))
            | (n & flag::X)
            | ((n & 0x02) << 4)
            | if bc_after != 0 { flag::PV } else { 0 };
        set_f(cpu, flags);
    } else {
        let step: u16 = if dec { 0xFFFF } else { 1 };
        let hl_v = cpu.regs.hl().wrapping_add(step);
        let de_v = cpu.regs.de().wrapping_add(step);
        let bc_v = cpu.regs.bc().wrapping_sub(1);
        cpu.regs.set_hl(hl_v);
        cpu.regs.set_de(de_v);
        cpu.regs.set_bc(bc_v);
        if rep && bc_v != 0 {
            block_repeat(cpu);
        }
    }
}

/// CPI / CPD / CPIR / CPDR (y = 4..7, z = 1).
/// Single hook X0 (the plan has only X0, then I5 and conditional ?I5): TMP.L = byte at (HL);
/// compare with A, HL +/-1, BC - 1, WZ +/-1; repeat when BC != 0 and no match.
fn ed_block_cp(cpu: &mut Cpu, y: u8, hook_no: u8) {
    if hook_no != 0 {
        return;
    }
    let dec = y & 1 == 1;
    let rep = y >= 6;
    let v = cpu.eng.tmp_l;
    let a = cpu.regs.a;
    let res = a.wrapping_sub(v);
    let h = (a ^ v ^ res) & flag::H;
    let step: u16 = if dec { 0xFFFF } else { 1 };
    let hl_v = cpu.regs.hl().wrapping_add(step);
    let bc_v = cpu.regs.bc().wrapping_sub(1);
    cpu.regs.set_hl(hl_v);
    cpu.regs.set_bc(bc_v);
    let n = res.wrapping_sub(if h != 0 { 1 } else { 0 });
    let flags = (cpu.regs.f & flag::C)
        | flag::N
        | (res & flag::S)
        | if res == 0 { flag::Z } else { 0 }
        | h
        | (n & flag::X)
        | ((n & 0x02) << 4)
        | if bc_v != 0 { flag::PV } else { 0 };
    set_f(cpu, flags);
    cpu.regs.wz = cpu.regs.wz.wrapping_add(step);
    if rep && bc_v != 0 && res != 0 {
        block_repeat(cpu);
    }
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

    #[test]
    fn e07_04_neg() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x12;
        sem_ed(&mut cpu, 0x44, 0); // ED 44: NEG
        assert_eq!(cpu.regs.a, 0xEE, "NEG of 0x12 is 0xEE");
        assert_eq!(cpu.regs.f & flag::N, flag::N, "N set for NEG");
        assert_eq!(cpu.regs.f & flag::C, flag::C, "C set when A != 0");

        // Edge case: NEG of 0x80 sets PV (overflow)
        let mut cpu2 = Cpu::new();
        cpu2.regs.a = 0x80;
        sem_ed(&mut cpu2, 0x44, 0);
        assert_eq!(cpu2.regs.a, 0x80, "NEG of 0x80 is 0x80");
        assert_eq!(cpu2.regs.f & flag::PV, flag::PV, "PV set for NEG 0x80");

        // Edge case: NEG of 0 gives Z and no C
        let mut cpu3 = Cpu::new();
        cpu3.regs.a = 0;
        sem_ed(&mut cpu3, 0x44, 0);
        assert_eq!(cpu3.regs.f & flag::Z, flag::Z, "Z set for NEG 0");
        assert_eq!(cpu3.regs.f & flag::C, 0, "C not set for NEG 0");
    }

    #[test]
    fn e07_04_retn_iff() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x1234; // return address already popped by plan
        cpu.regs.iff1 = false;
        cpu.regs.iff2 = true;
        sem_ed(&mut cpu, 0x45, 0); // ED 45: RETN
        assert_eq!(cpu.regs.pc, 0x1234, "RETN sets PC from WZ");
        assert!(cpu.regs.iff1, "RETN copies IFF2 to IFF1");

        // RETI (ED 4D) has same behavior
        let mut cpu2 = Cpu::new();
        cpu2.regs.wz = 0x5678;
        cpu2.regs.iff1 = true;
        cpu2.regs.iff2 = false;
        sem_ed(&mut cpu2, 0x4D, 0); // ED 4D: RETI
        assert_eq!(cpu2.regs.pc, 0x5678, "RETI sets PC from WZ");
        assert!(!cpu2.regs.iff1, "RETI copies IFF2 to IFF1");
    }

    #[test]
    fn e07_04_im_modes() {
        // IM table: [0,0,1,2,0,0,1,2] indexed by y
        let expected = [0u8, 0, 1, 2, 0, 0, 1, 2];
        for (y, exp) in expected.iter().enumerate() {
            let mut cpu = Cpu::new();
            // ED IM opcodes: x=1, z=6, y varies -> op = 0x46 | (y << 3)
            let op = 0x46 | ((y as u8) << 3);
            sem_ed(&mut cpu, op, 0);
            assert_eq!(
                cpu.regs.im, *exp,
                "IM mode {} for opcode ED {:02X}",
                exp, op
            );
        }
    }

    #[test]
    fn e07_04_ld_a_i_pv() {
        let mut cpu = Cpu::new();
        cpu.regs.i = 0x81; // odd parity value
        cpu.regs.iff2 = true;
        sem_ed(&mut cpu, 0x57, 0); // ED 57: LD A,I
        assert_eq!(cpu.regs.a, 0x81, "LD A,I loads I into A");
        assert_eq!(cpu.regs.f & flag::PV, flag::PV, "P/V = IFF2 (true)");
        assert_eq!(cpu.regs.f & flag::S, flag::S, "S from bit 7 of value");

        // With IFF2 false, PV should be clear
        let mut cpu2 = Cpu::new();
        cpu2.regs.i = 0x81;
        cpu2.regs.iff2 = false;
        sem_ed(&mut cpu2, 0x57, 0);
        assert_eq!(cpu2.regs.f & flag::PV, 0, "P/V = IFF2 (false)");

        // LD A,R has same behavior
        let mut cpu3 = Cpu::new();
        cpu3.regs.r = 0x45;
        cpu3.regs.iff2 = true;
        sem_ed(&mut cpu3, 0x5F, 0); // ED 5F: LD A,R
        assert_eq!(cpu3.regs.a, 0x45, "LD A,R loads R into A");
        assert_eq!(cpu3.regs.f & flag::PV, flag::PV, "P/V = IFF2 for LD A,R");
    }

    #[test]
    fn e07_04_ld_i_a() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0xAB;
        sem_ed(&mut cpu, 0x47, 0); // ED 47: LD I,A
        assert_eq!(cpu.regs.i, 0xAB, "LD I,A copies A to I");
    }

    #[test]
    fn e07_04_ld_r_a() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0xCD;
        sem_ed(&mut cpu, 0x4F, 0); // ED 4F: LD R,A
        assert_eq!(cpu.regs.r, 0xCD, "LD R,A copies A to R");
    }

    #[test]
    fn e07_04_rrd() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x12;
        cpu.eng.tmp_l = 0x34; // value read from (HL) by plan
        sem_ed(&mut cpu, 0x67, 0); // ED 67: RRD

        // Expected: new_mem = ((A&0xF)<<4)|(m>>4) = (0x02<<4)|0x03 = 0x23
        // new_a = (A&0xF0)|(m&0x0F) = 0x10|0x04 = 0x14
        assert_eq!(cpu.regs.a, 0x14, "RRD: A low nibble from memory low nibble");
        assert_eq!(cpu.eng.tmp_l, 0x23, "RRD: new memory byte computed");

        // Flags: S,Z,Y,X from new A; PV=parity(new A); C unchanged
        let mut cpu2 = Cpu::new();
        cpu2.regs.a = 0xF0;
        cpu2.eng.tmp_l = 0x0F;
        sem_ed(&mut cpu2, 0x67, 0);
        assert_eq!(
            cpu2.regs.a,
            0xF0 | (0x0F & 0x0F),
            "RRD preserves A high nibble"
        );

        // WZ = HL + 1
        let mut cpu3 = Cpu::new();
        cpu3.regs.set_hl(0x12FF);
        cpu3.eng.tmp_l = 0;
        sem_ed(&mut cpu3, 0x67, 0);
        assert_eq!(cpu3.regs.wz, 0x1300, "RRD: WZ = HL + 1");
    }

    #[test]
    fn e07_04_rld() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x12;
        cpu.eng.tmp_l = 0x34; // value read from (HL) by plan
        sem_ed(&mut cpu, 0x6F, 0); // ED 6F: RLD

        // Expected: new_mem = ((m&0xF)<<4)|(A&0xF) = (0x04<<4)|0x02 = 0x42
        // new_a = (A&0xF0)|(m>>4) = 0x10|0x03 = 0x13
        assert_eq!(
            cpu.regs.a, 0x13,
            "RLD: A low nibble from memory high nibble"
        );
        assert_eq!(cpu.eng.tmp_l, 0x42, "RLD: new memory byte computed");

        // WZ = HL + 1
        let mut cpu2 = Cpu::new();
        cpu2.regs.set_hl(0xFFFF);
        cpu2.eng.tmp_l = 0;
        sem_ed(&mut cpu2, 0x6F, 0);
        assert_eq!(cpu2.regs.wz, 0x0000, "RLD: WZ wraps");
    }

    #[test]
    fn e07_04_nop() {
        // Undefined ED opcodes act as NOPs (no state change)
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x55;
        sem_ed(&mut cpu, 0x00, 0); // ED 00: undefined -> NOP
        assert_eq!(cpu.regs.a, 0x55, "ED 00 does not change A");

        let mut cpu2 = Cpu::new();
        cpu2.regs.a = 0xAA;
        sem_ed(&mut cpu2, 0xFF, 0); // ED FF: undefined -> NOP
        assert_eq!(cpu2.regs.a, 0xAA, "ED FF does not change A");
    }

    #[test]
    fn e07_05_ldi() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x01;
        cpu.regs.f = flag::S | flag::C | flag::H | flag::N;
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_de(0x2000);
        cpu.regs.set_bc(0x0002);
        cpu.eng.tmp_l = 0x08; // n = 0x09 -> X set, bit1 clear
        sem_ed(&mut cpu, 0xA0, 0);
        sem_ed(&mut cpu, 0xA0, 1);
        assert_eq!(cpu.regs.hl(), 0x1001);
        assert_eq!(cpu.regs.de(), 0x2001);
        assert_eq!(cpu.regs.bc(), 0x0001);
        assert_eq!(cpu.regs.f, flag::S | flag::C | flag::X | flag::PV);
    }

    #[test]
    fn e07_05_lddr_repeat_sets_pc_wz() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_de(0x2000);
        cpu.regs.set_bc(0x0002);
        cpu.regs.pc = 0x0102;
        cpu.eng.tmp_l = 0;
        sem_ed(&mut cpu, 0xB8, 0);
        sem_ed(&mut cpu, 0xB8, 1);
        assert_eq!(cpu.regs.hl(), 0x0FFF);
        assert_eq!(cpu.regs.de(), 0x1FFF);
        assert_eq!(cpu.regs.pc, 0x0100);
        assert_eq!(cpu.regs.wz, 0x0101);
        assert!(cpu.eng.cond);
    }

    #[test]
    fn e07_05_ldir_last() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x0001);
        cpu.regs.pc = 0x0102;
        sem_ed(&mut cpu, 0xB0, 0);
        sem_ed(&mut cpu, 0xB0, 1);
        assert_eq!(cpu.regs.bc(), 0);
        assert_eq!(cpu.regs.pc, 0x0102);
        assert!(!cpu.eng.cond);
        assert_eq!(cpu.regs.f & flag::PV, 0);
    }

    #[test]
    fn e07_05_cpir_stop_on_match() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x42;
        cpu.regs.set_hl(0x3000);
        cpu.regs.set_bc(0x0005);
        cpu.regs.pc = 0x0102;
        cpu.regs.wz = 0x0010;
        cpu.eng.tmp_l = 0x42;
        sem_ed(&mut cpu, 0xB1, 0);
        assert_eq!(cpu.regs.hl(), 0x3001);
        assert_eq!(cpu.regs.bc(), 0x0004);
        assert_eq!(cpu.regs.wz, 0x0011);
        assert_eq!(cpu.regs.pc, 0x0102);
        assert!(!cpu.eng.cond);
        assert_eq!(
            cpu.regs.f & (flag::Z | flag::N | flag::PV),
            flag::Z | flag::N | flag::PV
        );
    }

    #[test]
    fn e07_05_cpdr_repeat_on_mismatch() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x10;
        cpu.regs.set_hl(0x3000);
        cpu.regs.set_bc(0x0005);
        cpu.regs.pc = 0x0202;
        cpu.eng.tmp_l = 0x20;
        sem_ed(&mut cpu, 0xB9, 0);
        assert_eq!(cpu.regs.hl(), 0x2FFF);
        assert_eq!(cpu.regs.pc, 0x0200);
        assert_eq!(cpu.regs.wz, 0x0201);
        assert!(cpu.eng.cond);
    }

    #[test]
    fn e07_05_ldd() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_de(0x2000);
        cpu.regs.set_bc(0x0001);
        cpu.regs.pc = 0x0102;
        sem_ed(&mut cpu, 0xA8, 0);
        sem_ed(&mut cpu, 0xA8, 1);
        assert_eq!(cpu.regs.hl(), 0x0FFF);
        assert_eq!(cpu.regs.de(), 0x1FFF);
        assert_eq!(cpu.regs.bc(), 0);
        assert_eq!(cpu.regs.pc, 0x0102);
    }

    #[test]
    fn e07_05_ldir_repeat() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_de(0x2000);
        cpu.regs.set_bc(0x0003);
        cpu.regs.pc = 0x0102;
        sem_ed(&mut cpu, 0xB0, 0);
        sem_ed(&mut cpu, 0xB0, 1);
        assert_eq!(cpu.regs.hl(), 0x1001);
        assert_eq!(cpu.regs.pc, 0x0100);
        assert_eq!(cpu.regs.wz, 0x0101);
        assert!(cpu.eng.cond);
    }

    #[test]
    fn e07_05_cpi() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x10;
        cpu.regs.set_hl(0x3000);
        cpu.regs.set_bc(0x0001);
        cpu.regs.pc = 0x0102;
        cpu.eng.tmp_l = 0x20;
        sem_ed(&mut cpu, 0xA1, 0);
        assert_eq!(cpu.regs.hl(), 0x3001);
        assert_eq!(cpu.regs.bc(), 0);
        assert_eq!(cpu.regs.pc, 0x0102);
        assert_eq!(
            cpu.regs.f & (flag::N | flag::C | flag::PV | flag::Z),
            flag::N
        );
        assert_ne!(cpu.regs.f & flag::S, 0);
    }
}
