//! Instruction semantics dispatch.
//!
//! Called by the engine when a hook token (Kind::X) is encountered.
//! Routes to family-specific semantic functions based on the opcode table.

use crate::cpu::Cpu;
use crate::engine::decode::Table;

pub(crate) mod cb;
pub(crate) mod ed;
pub(crate) mod idx;
pub(crate) mod main;
pub(crate) mod regs;

/// Execute instruction semantics for a hook.
///
/// # Arguments
/// * `cpu` - CPU state (mutated by the instruction)
/// * `table` - Current opcode table
/// * `op` - Opcode byte
/// * `hook_no` - Hook number (0 or 1)
pub(crate) fn sem(cpu: &mut Cpu, table: Table, op: u8, hook_no: u8) {
    // When stub-sem feature is active, skip all semantics (for timing tests).
    if cfg!(feature = "stub-sem") {
        return;
    }
    match table {
        Table::Main => main::sem_main(cpu, op, hook_no),
        Table::Cb => cb::sem_cb(cpu, op, hook_no),
        Table::Ed => ed::sem_ed(cpu, op, hook_no),
        Table::DdFd => idx::sem_idx(cpu, op, hook_no),
        Table::DdCb => idx::sem_ddcb(cpu, op, hook_no),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::decode::Prefix;
    use crate::engine::plan::{Addr, Kind, Reg, Tok};
    use crate::Pins;
    use std::boxed::Box;

    #[test]
    fn e05_02_r8_read_write() {
        let mut cpu = Cpu::new();
        for i in 0..8u8 {
            regs::set_r8(&mut cpu, i, (i + 1) * 10);
        }
        for i in 0..8u8 {
            assert_eq!(regs::r8(&cpu, i), (i + 1) * 10, "register {}", i);
        }
        cpu.eng.tmp_l = 0x42;
        assert_eq!(regs::r8(&cpu, 6), 0x42, "(HL) reads tmp_l");
        regs::set_r8(&mut cpu, 6, 0xFF);
        assert_eq!(cpu.eng.tmp_l, 0xFF, "set_r8(6,v) writes tmp_l");
    }

    #[test]
    fn e05_02_r8x_ixh() {
        let mut cpu = Cpu::new();
        cpu.regs.ix = 0xABCD;
        cpu.eng.prefix = Prefix::Dd;
        assert_eq!(regs::r8x(&cpu, 4), 0xAB, "r8x(4) reads IXH");
        regs::set_r8x(&mut cpu, 5, 0xEF);
        assert_eq!(cpu.regs.ixl(), 0xEF, "set_r8x(5,v) writes IXL");
        assert_eq!(cpu.regs.ix, 0xABEF, "IX updated via r8x");
    }

    #[test]
    fn e05_02_rp_pairs() {
        let mut cpu = Cpu::new();
        regs::set_rp(&mut cpu, 0, 0x1122);
        assert_eq!(regs::rp(&cpu, 0), 0x1122, "BC");
        regs::set_rp(&mut cpu, 1, 0x3344);
        assert_eq!(regs::rp(&cpu, 1), 0x3344, "DE");
        regs::set_rp(&mut cpu, 2, 0x5566);
        assert_eq!(regs::rp(&cpu, 2), 0x5566, "HL");
        regs::set_rp(&mut cpu, 3, 0x7788);
        assert_eq!(regs::rp(&cpu, 3), 0x7788, "SP");

        regs::set_rp2(&mut cpu, 0, 0xAABB);
        assert_eq!(regs::rp2(&cpu, 0), 0xAABB, "BC via rp2");
        regs::set_rp2(&mut cpu, 1, 0xCCDD);
        assert_eq!(regs::rp2(&cpu, 1), 0xCCDD, "DE via rp2");
        regs::set_rp2(&mut cpu, 2, 0xEEFF);
        assert_eq!(regs::rp2(&cpu, 2), 0xEEFF, "HL via rp2");
        regs::set_rp2(&mut cpu, 3, 0x1234);
        assert_eq!(regs::rp2(&cpu, 3), 0x1234, "AF via rp2");
    }

    #[test]
    fn e05_02_hl_prefix() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1111);
        cpu.regs.ix = 0x2222;
        cpu.regs.iy = 0x3333;

        assert_eq!(regs::hl(&cpu), 0x1111, "no prefix -> HL");

        cpu.eng.prefix = Prefix::Dd;
        assert_eq!(regs::hl(&cpu), 0x2222, "Dd prefix -> IX");
        regs::set_hl(&mut cpu, 0x4444);
        assert_eq!(cpu.regs.ix, 0x4444, "set_hl with Dd writes IX");

        cpu.eng.prefix = Prefix::Fd;
        assert_eq!(regs::hl(&cpu), 0x3333, "Fd prefix -> IY");
        regs::set_hl(&mut cpu, 0x5555);
        assert_eq!(cpu.regs.iy, 0x5555, "set_hl with Fd writes IY");
    }

    #[test]
    fn e05_02_cc_conditions() {
        let mut cpu = Cpu::new();
        cpu.regs.f = 0x00;
        assert!(regs::cc(&cpu, 0), "NZ true when Z clear");
        cpu.regs.f = 0x40;
        assert!(!regs::cc(&cpu, 0), "NZ false when Z set");

        assert!(regs::cc(&cpu, 1), "Z true when Z set");
        cpu.regs.f = 0x00;
        assert!(!regs::cc(&cpu, 1), "Z false when Z clear");

        cpu.regs.f = 0x00;
        assert!(regs::cc(&cpu, 2), "NC true when C clear");
        cpu.regs.f = 0x01;
        assert!(!regs::cc(&cpu, 2), "NC false when C set");

        assert!(regs::cc(&cpu, 3), "C true when C set");
    }

    #[test]
    fn e05_02_q_tracking() {
        let mut cpu = Cpu::new();
        cpu.regs.f = 0x10;
        cpu.eng.prefix = Prefix::None;

        // Case 1: f_written = true -> Q should be set to F at boundary
        cpu.eng.f_written = true;
        cpu.regs.f = 0x5A;
        let plan1 = Box::leak(Box::new([Tok::new(
            Kind::I,
            2,
            Addr::None,
            Reg::None,
            false,
        )]));
        cpu.start_plan(plan1);
        for _ in 0..2 {
            cpu.tick(Pins(0));
        }
        assert!(cpu.at_instruction_boundary());
        assert_eq!(cpu.regs.q, 0x5A, "Q should equal F when f_written is true");

        // Case 2: f_written = false -> Q should be 0 at boundary
        cpu.eng.f_written = false;
        cpu.regs.f = 0xFF;
        let plan2 = Box::leak(Box::new([Tok::new(
            Kind::I,
            2,
            Addr::None,
            Reg::None,
            false,
        )]));
        cpu.start_plan(plan2);
        for _ in 0..2 {
            cpu.tick(Pins(0));
        }
        assert!(cpu.at_instruction_boundary());
        assert_eq!(cpu.regs.q, 0, "Q should be 0 when f_written is false");
    }

    #[test]
    fn e05_03_di_ei() {
        let mut cpu = Cpu::new();
        // DI: clear IFF1 and IFF2
        sem(&mut cpu, Table::Main, 0xF3, 0);
        assert!(!cpu.regs.iff1, "DI clears IFF1");
        assert!(!cpu.regs.iff2, "DI clears IFF2");

        // EI: set IFF1 and IFF2, arm ei_delay
        sem(&mut cpu, Table::Main, 0xFB, 0);
        assert!(cpu.regs.iff1, "EI sets IFF1");
        assert!(cpu.regs.iff2, "EI sets IFF2");
        assert!(cpu.eng.ei_delay, "EI arms ei_delay");
    }

    #[test]
    fn e05_03_halt_pc() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 100;
        sem(&mut cpu, Table::Main, 0x76, 0);
        assert!(cpu.regs.halted, "HALT sets halted flag");
        assert_eq!(cpu.regs.pc, 99, "HALT decrements PC (stays on opcode)");
    }

    #[test]
    fn e05_03_exx() {
        let mut cpu = Cpu::new();
        // Set up main and alternate register pairs with different values
        cpu.regs.set_bc(0x1122);
        cpu.regs.set_de(0x3344);
        cpu.regs.set_hl(0x5566);
        cpu.regs.set_bc2(0xAABB);
        cpu.regs.set_de2(0xCCDD);
        cpu.regs.set_hl2(0xEEFF);

        sem(&mut cpu, Table::Main, 0xD9, 0);

        // After EXX: main pairs should have alternate values and vice versa
        assert_eq!(cpu.regs.bc(), 0xAABB, "EXX swaps BC");
        assert_eq!(cpu.regs.de(), 0xCCDD, "EXX swaps DE");
        assert_eq!(cpu.regs.hl(), 0xEEFF, "EXX swaps HL");
        assert_eq!(cpu.regs.bc2(), 0x1122, "EXX swaps BC'");
        assert_eq!(cpu.regs.de2(), 0x3344, "EXX swaps DE'");
        assert_eq!(cpu.regs.hl2(), 0x5566, "EXX swaps HL'");
    }

    #[test]
    fn e05_03_ex_af() {
        let mut cpu = Cpu::new();
        // Set up AF and AF' with different values
        cpu.regs.a = 0x12;
        cpu.regs.f = 0x34;
        cpu.regs.a2 = 0xAB;
        cpu.regs.f2 = 0xCD;

        sem(&mut cpu, Table::Main, 0x08, 0);

        // After EX AF,AF': values should be swapped
        assert_eq!(cpu.regs.a, 0xAB, "EX AF swaps A");
        assert_eq!(cpu.regs.f, 0xCD, "EX AF swaps F");
        assert_eq!(cpu.regs.a2, 0x12, "EX AF swaps A'");
        assert_eq!(cpu.regs.f2, 0x34, "EX AF swaps F'");
    }

    #[test]
    fn e05_03_ex_de_hl() {
        let mut cpu = Cpu::new();
        // Set up DE and HL with different values
        cpu.regs.set_de(0x1234);
        cpu.regs.set_hl(0x5678);

        sem(&mut cpu, Table::Main, 0xEB, 0);

        // After EX DE,HL: values should be swapped
        assert_eq!(cpu.regs.de(), 0x5678, "EX DE,HL swaps to DE");
        assert_eq!(cpu.regs.hl(), 0x1234, "EX DE,HL swaps to HL");
    }

    #[test]
    fn e05_03_nop_no_effect() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0xAA;
        cpu.regs.b = 0xBB;
        cpu.regs.f = 0xCC;
        cpu.regs.iff1 = true;
        cpu.regs.halted = true;

        sem(&mut cpu, Table::Main, 0x00, 0);

        // NOP should not change any state
        assert_eq!(cpu.regs.a, 0xAA, "NOP preserves A");
        assert_eq!(cpu.regs.b, 0xBB, "NOP preserves B");
        assert_eq!(cpu.regs.f, 0xCC, "NOP preserves F");
        assert!(cpu.regs.iff1, "NOP preserves IFF1");
        assert!(cpu.regs.halted, "NOP preserves halted");
    }

    #[test]
    fn e05_02_dispatch_main_noop() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0xAA;
        cpu.regs.b = 0xBB;
        cpu.regs.f = 0xCC;

        sem(&mut cpu, Table::Main, 0x00, 0);

        assert_eq!(cpu.regs.a, 0xAA, "A unchanged after NOP");
        assert_eq!(cpu.regs.b, 0xBB, "B unchanged after NOP");
        assert_eq!(cpu.regs.f, 0xCC, "F unchanged after NOP");
    }
}
