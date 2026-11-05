//! DD/FD prefix table semantics.
//!
//! E08-01: classification of opcodes into "indexed" (handled from E08-02 on) and
//! "ignored" (the prefix is a 4 T no-op and the Main semantics apply unchanged).

use super::main::sem_main;
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
    if is_indexed(op) && !is_idx_pair_op(op) {
        return;
    }
    // Ignored opcodes and the 16-bit IX/IY forms (E08-02) share the Main semantics:
    // the helpers hl()/set_hl()/rp()/rp2() map HL to IX/IY from eng.prefix.
    sem_main(cpu, op, hook_no);
}

/// 16-bit IX/IY forms implemented by E08-02: LD IX,nn / LD (nn),IX / LD IX,(nn) / INC IX / DEC IX /
/// POP IX / EX (SP),IX / PUSH IX / JP (IX) / LD SP,IX.
fn is_idx_pair_op(op: u8) -> bool {
    matches!(
        op,
        0x21 | 0x22 | 0x2A | 0x23 | 0x2B | 0xE1 | 0xE3 | 0xE5 | 0xE9 | 0xF9
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
}
