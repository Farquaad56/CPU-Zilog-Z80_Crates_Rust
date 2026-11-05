//! DD/FD prefix table semantics.
//!
//! E08-01: classification of opcodes into "indexed" (handled from E08-02 on) and
//! "ignored" (the prefix is a 4 T no-op and the Main semantics apply unchanged).

use super::main::sem_main;
use super::regs::{r8x, set_f, set_r8x};
use crate::alu;
use crate::cpu::Cpu;

/// True when the DD/FD prefix changes the instruction (HL/H/L/(HL) become IX/IY forms).
///
/// Indexed set (spec E08-01):
/// - x=0: 09 19 29 39 21 22 2A 23 2B 24 25 2C 2D 26 2E 34 35 36
/// - x=1 except 76: y in 4..=6 or z in 4..=6
/// - x=2: z in 4..=6
/// - x=3: E1 E3 E5 E9 F9
pub(crate) fn is_indexed(op: u8) -> bool {
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;
    match x {
        0 => matches!(
            op,
            0x09 | 0x19
                | 0x29
                | 0x39
                | 0x21
                | 0x22
                | 0x2A
                | 0x23
                | 0x2B
                | 0x24
                | 0x25
                | 0x2C
                | 0x2D
                | 0x26
                | 0x2E
                | 0x34
                | 0x35
                | 0x36
        ),
        1 => op != 0x76 && ((4..=6).contains(&y) || (4..=6).contains(&z)),
        2 => (4..=6).contains(&z),
        _ => matches!(op, 0xE1 | 0xE3 | 0xE5 | 0xE9 | 0xF9),
    }
}

/// Execute DD/FD-prefix instruction semantics.
///
/// Ignored opcodes run the Main semantics (the prefix only costs 4 T, taken by the plan).
/// Indexed opcodes do nothing yet (E08-02..E08-06).
pub(crate) fn sem_idx(cpu: &mut Cpu, op: u8, hook_no: u8) {
    if !is_indexed(op) || is_idx_pair_op(op) || is_idx_mem_ld(op) || is_idx_mem_alu(op) {
        // Ignored opcodes, 16-bit IX/IY forms (E08-02/03) and (IX+d) memory forms
        // (E08-04/05) share the Main semantics: hl()/set_hl() map HL to IX/IY from
        // eng.prefix, and index 6 is the byte travelling through TMP.L.
        sem_main(cpu, op, hook_no);
        return;
    }
    // Remaining indexed opcodes use H/L as IXH/IXL (IYH/IYL) registers (E08-06).
    if hook_no == 0 {
        idx_reg_op(cpu, op);
    }
}

/// Undocumented IXH/IXL forms: LD r,r' / LD r,n / INC r / DEC r / ALU A,r with H/L replaced
/// by IXH/IXL (IYH/IYL). Only opcodes without a (IX+d) operand reach this function.
fn idx_reg_op(cpu: &mut Cpu, op: u8) {
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;
    match x {
        1 => {
            // LD r(y),r(z)
            let v = r8x(cpu, z);
            set_r8x(cpu, y, v);
        }
        0 if z == 6 => {
            // LD r(y),n: n is already in TMP.L
            let v = cpu.eng.tmp_l;
            set_r8x(cpu, y, v);
        }
        0 => {
            // INC r (z == 4) / DEC r (z == 5)
            let v = r8x(cpu, y);
            let (res, f) = if z == 4 {
                alu::inc8(v, cpu.regs.f)
            } else {
                alu::dec8(v, cpu.regs.f)
            };
            set_r8x(cpu, y, res);
            set_f(cpu, f);
        }
        _ => {
            // ALU A,r(z)
            let b = r8x(cpu, z);
            let (res, f) = alu::alu8(y, cpu.regs.a, b, cpu.regs.f);
            if y != 7 {
                cpu.regs.a = res;
            }
            set_f(cpu, f);
        }
    }
}

/// Indexed memory loads implemented by E08-04: LD r,(IX+d) / LD (IX+d),r / LD (IX+d),n.
/// The other operand stays the real H/L, which is what the Main r8()/set_r8() helpers use.
/// The address (IX+d) and WZ are resolved by the plan; the byte travels through TMP.L.
fn is_idx_mem_ld(op: u8) -> bool {
    let y = (op >> 3) & 7;
    let z = op & 7;
    op == 0x36 || (op >> 6 == 1 && op != 0x76 && ((y == 6) != (z == 6)))
}

/// Indexed memory ALU forms implemented by E08-05: ALU A,(IX+d), INC (IX+d), DEC (IX+d).
/// The operand travels through TMP.L (read by the plan, written back for INC/DEC), which is
/// what the Main r8()/set_r8() helpers use for index 6.
fn is_idx_mem_alu(op: u8) -> bool {
    op == 0x34 || op == 0x35 || (op >> 6 == 2 && op & 7 == 6)
}

/// 16-bit IX/IY forms implemented by E08-02: LD IX,nn / LD (nn),IX / LD IX,(nn) / INC IX / DEC IX /
/// POP IX / EX (SP),IX / PUSH IX / JP (IX) / LD SP,IX, plus ADD IX,rp (E08-03).
fn is_idx_pair_op(op: u8) -> bool {
    matches!(
        op,
        0x21 | 0x22
            | 0x2A
            | 0x23
            | 0x2B
            | 0xE1
            | 0xE3
            | 0xE5
            | 0xE9
            | 0xF9
            | 0x09
            | 0x19
            | 0x29
            | 0x39
    )
}

/// Execute DD CB / FD CB instruction semantics (stub).
pub(crate) fn sem_ddcb(_cpu: &mut Cpu, _op: u8, _hook_no: u8) {
    // Stub: actual semantics implemented in E05+
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e08_01_is_indexed_list() {
        for op in [0x21u8, 0x36, 0x46, 0x70, 0x86, 0xE9, 0xF9] {
            assert!(is_indexed(op), "0x{op:02X} should be indexed");
        }
        for op in [0x00u8, 0x40, 0x76, 0xC3, 0xCD] {
            assert!(!is_indexed(op), "0x{op:02X} should be ignored");
        }
    }

    #[test]
    fn e08_01_ignored_nop() {
        let mut cpu = Cpu::new();
        let before = cpu.regs;
        sem_idx(&mut cpu, 0x00, 0);
        assert_eq!(cpu.regs, before);
    }

    #[test]
    fn e08_01_ignored_ld_bc_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x1234;
        sem_idx(&mut cpu, 0x01, 0);
        assert_eq!(cpu.regs.bc(), 0x1234);
    }

    #[test]
    fn e08_01_ignored_jp() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x4321;
        sem_idx(&mut cpu, 0xC3, 0);
        assert_eq!(cpu.regs.pc, 0x4321);
    }

    #[test]
    fn e08_01_fd_same() {
        // FD uses the same semantics function as DD.
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x00FF;
        sem_idx(&mut cpu, 0x11, 0);
        assert_eq!(cpu.regs.de(), 0x00FF);
        assert!(!is_indexed(0x11));
    }

    use crate::engine::decode::Prefix;

    fn with_prefix(prefix: Prefix) -> Cpu {
        let mut cpu = Cpu::new();
        cpu.eng.prefix = prefix;
        cpu
    }

    #[test]
    fn e08_02_ld_ix_nn() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.wz = 0xBEEF;
        sem_idx(&mut cpu, 0x21, 0);
        assert_eq!(cpu.regs.ix, 0xBEEF);
        assert_eq!(cpu.regs.hl(), Cpu::new().regs.hl());
    }

    #[test]
    fn e08_02_ld_iy_nn() {
        let mut cpu = with_prefix(Prefix::Fd);
        cpu.regs.wz = 0x1234;
        sem_idx(&mut cpu, 0x21, 0);
        assert_eq!(cpu.regs.iy, 0x1234);
    }

    #[test]
    fn e08_02_inc_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0xFFFF;
        sem_idx(&mut cpu, 0x23, 0);
        assert_eq!(cpu.regs.ix, 0x0000);
        sem_idx(&mut cpu, 0x2B, 0);
        assert_eq!(cpu.regs.ix, 0xFFFF);
    }

    #[test]
    fn e08_02_jp_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x4000;
        sem_idx(&mut cpu, 0xE9, 0);
        assert_eq!(cpu.regs.pc, 0x4000);
    }

    #[test]
    fn e08_02_ld_sp_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x8000;
        sem_idx(&mut cpu, 0xF9, 0);
        assert_eq!(cpu.regs.sp, 0x8000);
    }

    #[test]
    fn e08_02_ex_de_hl_ignored() {
        // DD EB swaps DE and HL (never IX/IY): the opcode is not in the indexed set.
        assert!(!is_indexed(0xEB));
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.set_de(0x1111);
        cpu.regs.set_hl(0x2222);
        cpu.regs.ix = 0x3333;
        sem_idx(&mut cpu, 0xEB, 0);
        assert_eq!(cpu.regs.de(), 0x2222);
        assert_eq!(cpu.regs.hl(), 0x1111);
        assert_eq!(cpu.regs.ix, 0x3333);
    }

    #[test]
    fn e08_02_ld_nn_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0xABCD;
        sem_idx(&mut cpu, 0x22, 0);
        assert_eq!(cpu.eng.tmp_l, 0xCD);
        assert_eq!(cpu.eng.tmp_h, 0xAB);
    }

    #[test]
    fn e08_02_ld_ix_nn_mem() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.eng.tmp_l = 0x34;
        cpu.eng.tmp_h = 0x12;
        cpu.regs.wz = 0x2000;
        sem_idx(&mut cpu, 0x2A, 0);
        assert_eq!(cpu.regs.ix, 0x1234);
        assert_eq!(cpu.regs.wz, 0x2001);
    }

    #[test]
    fn e08_02_push_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x1234;
        sem_idx(&mut cpu, 0xE5, 0);
        assert_eq!(cpu.eng.tmp_h, 0x12);
        assert_eq!(cpu.eng.tmp_l, 0x34);
    }

    #[test]
    fn e08_02_pop_iy() {
        let mut cpu = with_prefix(Prefix::Fd);
        cpu.eng.tmp_l = 0x78;
        cpu.eng.tmp_h = 0x56;
        sem_idx(&mut cpu, 0xE1, 0);
        assert_eq!(cpu.regs.iy, 0x5678);
    }

    #[test]
    fn e08_02_ex_sp_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0xAAAA;
        cpu.regs.wz = 0xBBBB; // value read from the stack by the plan
        sem_idx(&mut cpu, 0xE3, 0);
        assert_eq!(cpu.regs.ix, 0xBBBB);
        assert_eq!(cpu.eng.tmp_h, 0xAA);
        assert_eq!(cpu.eng.tmp_l, 0xAA);
    }

    #[test]
    fn e08_03_add_ix_bc() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x1000;
        cpu.regs.set_bc(0x0234);
        sem_idx(&mut cpu, 0x09, 0);
        assert_eq!(cpu.regs.ix, 0x1234);
    }

    #[test]
    fn e08_03_add_ix_ix() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x8001;
        sem_idx(&mut cpu, 0x29, 0);
        assert_eq!(cpu.regs.ix, 0x0002);
        assert_ne!(cpu.regs.f & crate::regs::flag::C, 0);
    }

    #[test]
    fn e08_03_add_iy_sp() {
        let mut cpu = with_prefix(Prefix::Fd);
        cpu.regs.iy = 0x0100;
        cpu.regs.sp = 0x0023;
        sem_idx(&mut cpu, 0x39, 0);
        assert_eq!(cpu.regs.iy, 0x0123);
    }

    #[test]
    fn e08_03_wz() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x2000;
        cpu.regs.set_bc(0x0001);
        sem_idx(&mut cpu, 0x09, 0);
        assert_eq!(cpu.regs.wz, 0x2001);
    }

    #[test]
    fn e08_04_ld_b_ix_d() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.eng.tmp_l = 0x5A;
        sem_idx(&mut cpu, 0x46, 0);
        assert_eq!(cpu.regs.b, 0x5A);
    }

    #[test]
    fn e08_04_ld_h_ix_d_real_h() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x1234;
        cpu.eng.tmp_l = 0x77;
        sem_idx(&mut cpu, 0x66, 0);
        assert_eq!(cpu.regs.h, 0x77);
        assert_eq!(cpu.regs.ix, 0x1234);
    }

    #[test]
    fn e08_04_ld_ix_d_h() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.h = 0x42;
        cpu.regs.ix = 0x9900;
        sem_idx(&mut cpu, 0x74, 0);
        assert_eq!(cpu.eng.tmp_l, 0x42);
    }

    #[test]
    fn e08_04_ld_ix_d_n() {
        // LD (IX+d),n needs no hook: TMP.L already holds n and the plan writes it.
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.eng.tmp_l = 0xC3;
        sem_idx(&mut cpu, 0x36, 0);
        assert_eq!(cpu.eng.tmp_l, 0xC3);
    }

    #[test]
    fn e08_04_negative_d() {
        // The classification covers the whole memory-load set, independent of d.
        for op in [
            0x46u8, 0x4E, 0x56, 0x5E, 0x66, 0x6E, 0x7E, 0x70, 0x71, 0x77, 0x36,
        ] {
            assert!(is_idx_mem_ld(op), "0x{op:02X}");
        }
        for op in [0x76u8, 0x44, 0x60, 0x7C, 0x26] {
            assert!(!is_idx_mem_ld(op), "0x{op:02X}");
        }
    }

    #[test]
    fn e08_05_add_a_ix_d() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.a = 0x10;
        cpu.eng.tmp_l = 0x22;
        sem_idx(&mut cpu, 0x86, 0);
        assert_eq!(cpu.regs.a, 0x32);
    }

    #[test]
    fn e08_05_cp_ix_d() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.a = 0x40;
        cpu.eng.tmp_l = 0x40;
        sem_idx(&mut cpu, 0xBE, 0);
        assert_eq!(cpu.regs.a, 0x40);
        assert_ne!(cpu.regs.f & crate::regs::flag::Z, 0);
    }

    #[test]
    fn e08_05_inc_ix_d() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.eng.tmp_l = 0xFF;
        sem_idx(&mut cpu, 0x34, 0);
        assert_eq!(cpu.eng.tmp_l, 0x00);
        assert_ne!(cpu.regs.f & crate::regs::flag::Z, 0);
    }

    #[test]
    fn e08_05_dec_iy_d() {
        let mut cpu = with_prefix(Prefix::Fd);
        cpu.eng.tmp_l = 0x01;
        sem_idx(&mut cpu, 0x35, 0);
        assert_eq!(cpu.eng.tmp_l, 0x00);
        assert_ne!(cpu.regs.f & crate::regs::flag::N, 0);
    }

    #[test]
    fn e08_06_ld_b_ixh() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0xA155;
        sem_idx(&mut cpu, 0x44, 0);
        assert_eq!(cpu.regs.b, 0xA1);
    }

    #[test]
    fn e08_06_ld_ixh_ixl() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x1234;
        sem_idx(&mut cpu, 0x65, 0);
        assert_eq!(cpu.regs.ix, 0x3434);
    }

    #[test]
    fn e08_06_ld_ixl_n() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x1200;
        cpu.eng.tmp_l = 0x99;
        sem_idx(&mut cpu, 0x2E, 0);
        assert_eq!(cpu.regs.ix, 0x1299);
    }

    #[test]
    fn e08_06_inc_ixh() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x7F00;
        sem_idx(&mut cpu, 0x24, 0);
        assert_eq!(cpu.regs.ix, 0x8000);
    }

    #[test]
    fn e08_06_dec_ixl() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.ix = 0x1200;
        sem_idx(&mut cpu, 0x2D, 0);
        assert_eq!(cpu.regs.ix, 0x12FF);
    }

    #[test]
    fn e08_06_add_a_ixh() {
        let mut cpu = with_prefix(Prefix::Dd);
        cpu.regs.a = 0x01;
        cpu.regs.ix = 0x0F00;
        sem_idx(&mut cpu, 0x84, 0);
        assert_eq!(cpu.regs.a, 0x10);
    }

    #[test]
    fn e08_06_ld_a_iyl_fd() {
        let mut cpu = with_prefix(Prefix::Fd);
        cpu.regs.iy = 0x1234;
        cpu.regs.ix = 0xFFFF;
        sem_idx(&mut cpu, 0x7D, 0);
        assert_eq!(cpu.regs.a, 0x34);
    }
}
