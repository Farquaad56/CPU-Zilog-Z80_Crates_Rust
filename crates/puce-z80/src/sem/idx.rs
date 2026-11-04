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
    if is_indexed(op) {
        return;
    }
    sem_main(cpu, op, hook_no);
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
}
