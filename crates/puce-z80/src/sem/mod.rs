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
    use crate::regs::flag;
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

    #[test]
    fn e05_04_ld_b_c() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0x11;
        cpu.regs.c = 0x22;
        // LD B,C (opcode 0x41)
        sem(&mut cpu, Table::Main, 0x41, 0);
        assert_eq!(cpu.regs.b, 0x22, "LD B,C copies C to B");
    }

    #[test]
    fn e05_04_ld_a_hl() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_l = 0xAB; // Simulate memory read into tmp_l
                              // LD A,(HL) (opcode 0x7E): y=7, z=6
        sem(&mut cpu, Table::Main, 0x7E, 0);
        assert_eq!(cpu.regs.a, 0xAB, "LD A,(HL) loads tmp_l into A");
    }

    #[test]
    fn e05_04_ld_hl_r() {
        let mut cpu = Cpu::new();
        cpu.regs.e = 0xCD;
        // LD (HL),E (opcode 0x73): y=6, z=3
        sem(&mut cpu, Table::Main, 0x73, 0);
        assert_eq!(cpu.eng.tmp_l, 0xCD, "LD (HL),E stores E into tmp_l");
    }

    #[test]
    fn e05_04_ld_r_n() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_l = 0x99; // Simulate immediate byte read into tmp_l
                              // LD H,n (opcode 0x26): x=0, y=4, z=6
        sem(&mut cpu, Table::Main, 0x26, 0);
        assert_eq!(cpu.regs.h, 0x99, "LD H,n loads immediate into H");
    }

    #[test]
    fn e05_04_ld_hl_n_no_hook() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_l = 0xFF; // Immediate byte already in tmp_l
                              // LD (HL),n (opcode 0x36): x=0, y=6, z=6 - no hook should run
        sem(&mut cpu, Table::Main, 0x36, 0);
        assert_eq!(cpu.eng.tmp_l, 0xFF, "LD (HL),n leaves tmp_l unchanged");
    }

    #[test]
    fn e05_05_wz_ld_bc_a() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x1234);
        cpu.regs.a = 0xAB;
        // LD (BC),A (opcode 0x02): WZ = ((BC+1)&0xFF) | (A<<8)
        sem(&mut cpu, Table::Main, 0x02, 0);
        let expected_wz: u16 = 0xAB35;
        assert_eq!(cpu.regs.wz, expected_wz, "LD (BC),A WZ formula");
        assert_eq!(cpu.eng.tmp_l, 0xAB, "LD (BC),A stores A in tmp_l");
    }

    #[test]
    fn e05_05_wz_ld_a_bc() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x1234);
        cpu.eng.tmp_l = 0xCD; // Simulate memory read into tmp_l
                              // LD A,(BC) (opcode 0x0A): WZ = BC+1
        sem(&mut cpu, Table::Main, 0x0A, 0);
        assert_eq!(cpu.regs.wz, 0x1235, "LD A,(BC) WZ = BC+1");
        assert_eq!(cpu.regs.a, 0xCD, "LD A,(BC) loads tmp_l into A");
    }

    #[test]
    fn e05_05_ld_a_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x2000; // nn address already in WZ from data fetch phase
        cpu.eng.tmp_l = 0xEF; // Simulate memory read into tmp_l
                              // LD A,(nn) (opcode 0x3A): WZ = nn+1
        sem(&mut cpu, Table::Main, 0x3A, 0);
        assert_eq!(cpu.regs.wz, 0x2001, "LD A,(nn) WZ = nn+1");
        assert_eq!(cpu.regs.a, 0xEF, "LD A,(nn) loads tmp_l into A");
    }

    #[test]
    fn e05_05_ld_nn_a_wz() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x2000; // nn address already in WZ from data fetch phase
        cpu.regs.a = 0x99;
        // LD (nn),A (opcode 0x32). UNKNOWN - to confirm: final WZ needs a post-write hook
        // (see docs/qa/inbox/E05-05-Q1.md); the hook must keep WZ = nn for the write address.
        sem(&mut cpu, Table::Main, 0x32, 0);
        assert_eq!(
            cpu.regs.wz, 0x2000,
            "LD (nn),A keeps nn in WZ for the write"
        );
        assert_eq!(cpu.eng.tmp_l, 0x99, "LD (nn),A stores A in tmp_l");
    }

    #[test]
    fn e05_06_ld_bc_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x1234; // nn already read into WZ
        sem(&mut cpu, Table::Main, 0x01, 0);
        assert_eq!(cpu.regs.bc(), 0x1234, "LD BC,nn loads WZ");
    }

    #[test]
    fn e05_06_ld_sp_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x8000;
        sem(&mut cpu, Table::Main, 0x31, 0);
        assert_eq!(cpu.regs.sp, 0x8000, "LD SP,nn loads WZ");
    }

    #[test]
    fn e05_06_ld_nn_hl() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        sem(&mut cpu, Table::Main, 0x22, 0);
        assert_eq!(cpu.eng.tmp_l, 0x34, "LD (nn),HL: TMP.L = L");
        assert_eq!(cpu.eng.tmp_h, 0x12, "LD (nn),HL: TMP.H = H");
    }

    #[test]
    fn e05_06_ld_hl_nn() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x2000;
        cpu.eng.tmp_l = 0xCD;
        cpu.eng.tmp_h = 0xAB;
        sem(&mut cpu, Table::Main, 0x2A, 0);
        assert_eq!(cpu.regs.hl(), 0xABCD, "LD HL,(nn) loads TMP");
        assert_eq!(cpu.regs.wz, 0x2001, "LD HL,(nn): WZ = nn+1");
    }

    #[test]
    fn e05_06_ld_sp_hl() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0xBEEF);
        sem(&mut cpu, Table::Main, 0xF9, 0);
        assert_eq!(cpu.regs.sp, 0xBEEF, "LD SP,HL copies HL");
    }

    #[test]
    fn e05_06_add_hl_rp_untouched() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x4321;
        cpu.regs.set_bc(0x1111);
        sem(&mut cpu, Table::Main, 0x09, 0); // ADD HL,BC is not LD rp,nn
        assert_eq!(cpu.regs.bc(), 0x1111, "ADD HL,BC must not load BC from WZ");
    }

    #[test]
    fn e05_07_inc_b() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 0x4F;
        sem(&mut cpu, Table::Main, 0x04, 0); // INC B
        assert_eq!(cpu.regs.b, 0x50, "INC B increments");
    }

    #[test]
    fn e05_07_dec_a_overflow() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x80;
        sem(&mut cpu, Table::Main, 0x3D, 0); // DEC A
        assert_eq!(cpu.regs.a, 0x7F, "DEC A from 0x80 wraps to 0x7F");
    }

    #[test]
    fn e05_07_inc_hl() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_l = 0xFF; // Simulate memory read into tmp_l
        sem(&mut cpu, Table::Main, 0x34, 0); // INC (HL)
        assert_eq!(cpu.eng.tmp_l, 0x00, "INC (HL) wraps via tmp_l");
    }

    #[test]
    fn e05_07_carry_kept() {
        let mut cpu = Cpu::new();
        cpu.regs.f = 1; // Carry set
        sem(&mut cpu, Table::Main, 0x04, 0); // INC B (doesn't affect carry)
        assert_eq!(cpu.regs.f & 1, 1, "INC preserves carry flag");
    }

    #[test]
    fn e05_08_inc_bc() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x1234);
        sem(&mut cpu, Table::Main, 0x03, 0); // INC BC
        assert_eq!(cpu.regs.bc(), 0x1235, "INC BC increments");
    }

    #[test]
    fn e05_08_dec_sp() {
        let mut cpu = Cpu::new();
        cpu.regs.sp = 0x0000;
        sem(&mut cpu, Table::Main, 0x3B, 0); // DEC SP
        assert_eq!(cpu.regs.sp, 0xFFFF, "DEC SP wraps from 0 to FFFF");
    }

    #[test]
    fn e05_08_add_hl_de() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.regs.set_de(0x5678);
        sem(&mut cpu, Table::Main, 0x19, 0); // ADD HL,DE
        assert_eq!(cpu.regs.hl(), 0x68AC, "ADD HL,DE computes correct result");
    }

    #[test]
    fn e05_08_add_hl_wz() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.regs.set_de(0x0001);
        sem(&mut cpu, Table::Main, 0x19, 0); // ADD HL,DE
        assert_eq!(cpu.regs.wz, 0x1235, "WZ = HL+1 before addition");
    }

    #[test]
    fn e05_09_add_a_b() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x44;
        cpu.regs.b = 0x22;
        sem(&mut cpu, Table::Main, 0x80, 0); // ADD A,B
        assert_eq!(cpu.regs.a, 0x66, "ADD A,B computes correct result");
    }

    #[test]
    fn e05_09_cp_n() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x44;
        cpu.eng.tmp_l = 0x22; // immediate value loaded into tmp_l
        sem(&mut cpu, Table::Main, 0xFE, 0); // CP n (y=7)
        assert_eq!(cpu.regs.a, 0x44, "CP n preserves A");
    }

    #[test]
    fn e05_09_sbc_a_hl() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x80;
        cpu.eng.tmp_l = 0x40; // (HL) value loaded into tmp_l
        sem(&mut cpu, Table::Main, 0x9E, 0); // SBC A,(HL) (y=3, z=6)
        assert_eq!(cpu.regs.a, 0x40, "SBC A,(HL) computes correct result");
    }

    #[test]
    fn e05_09_xor_a() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x44;
        sem(&mut cpu, Table::Main, 0xAF, 0); // XOR A (y=5, z=7)
        assert_eq!(cpu.regs.a, 0x00, "XOR A clears accumulator");
    }

    #[test]
    fn e05_10_rlca() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x85;
        sem(&mut cpu, Table::Main, 0x07, 0); // RLCA (y=0)
        assert_eq!(cpu.regs.a, 0x0B, "RLCA rotates left through carry");
        assert_eq!(cpu.regs.f & flag::C, flag::C, "RLCA sets carry from bit 7");
    }

    #[test]
    fn e05_10_rra() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x85;
        cpu.regs.f = flag::C; // Carry set
        sem(&mut cpu, Table::Main, 0x1F, 0); // RRA (y=3)
        assert_eq!(cpu.regs.a, 0xC2, "RRA rotates right through carry");
    }

    #[test]
    fn e05_10_daa_hook() {
        // BCD addition scenario: hex result 0x7C needs DAA adjustment.
        // Lower nibble C > 9, so add 6 -> 0x82 (upper nibble unchanged).
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x7C;
        cpu.regs.f = flag::H; // Half-carry set (from prior addition)
        sem(&mut cpu, Table::Main, 0x27, 0); // DAA (y=4)
        assert_eq!(cpu.regs.a, 0x82, "DAA adjusts BCD result");
    }

    #[test]
    fn e05_10_cpl() {
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x44;
        sem(&mut cpu, Table::Main, 0x2F, 0); // CPL (y=5)
        assert_eq!(cpu.regs.a, 0xBB, "CPL complements A");
        assert_eq!(cpu.regs.f & flag::H, flag::H, "CPL sets H");
        assert_eq!(cpu.regs.f & flag::N, flag::N, "CPL sets N");
    }

    #[test]
    fn e05_10_scf_flags() {
        let mut cpu = Cpu::new();
        sem(&mut cpu, Table::Main, 0x37, 0); // SCF (y=6)
        assert_eq!(cpu.regs.f & flag::C, flag::C, "SCF sets carry");
        assert_eq!(cpu.regs.f & flag::H, 0, "SCF clears H");
        assert_eq!(cpu.regs.f & flag::N, 0, "SCF clears N");
    }

    #[test]
    fn e05_10_ccf_h() {
        let mut cpu = Cpu::new();
        cpu.regs.f = flag::C; // Carry set
        sem(&mut cpu, Table::Main, 0x3F, 0); // CCF (y=7)
        assert_eq!(cpu.regs.f & flag::C, 0, "CCF clears carry when it was set");
    }

    #[test]
    fn e05_10_scf_ccf_q_formula() {
        // Test both Q-factor and simple formulas for SCF/CCF X/Y bits.
        // UNKNOWN - to confirm: which formula is correct will be determined
        // by z80ccf/ZEXALL in E10-03. This test documents the difference.
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x5A;
        cpu.regs.f = 0x00;
        cpu.regs.q = 0x00;

        // Simple formula: X,Y = bits 3,5 of A directly
        let simple_xy = cpu.regs.a & 0x28;

        // Q-factor formula: X,Y = bits 3,5 of (A | (F ^ Q))
        let q_formula_val = cpu.regs.a | (cpu.regs.f ^ cpu.regs.q);
        let q_xy = q_formula_val & 0x28;

        // With F=Q=0, both formulas give the same result for this case
        assert_eq!(simple_xy, q_xy, "With F=Q=0, simple and Q-factor agree");

        // Now test with different F and Q to show they can differ
        cpu.regs.f = 0x28; // Set X,Y bits in old flags
        cpu.regs.q = 0x00;
        let q_formula_val2 = cpu.regs.a | (cpu.regs.f ^ cpu.regs.q);
        let q_xy2 = q_formula_val2 & 0x28;

        // Q-factor formula ORs in the old X/Y bits, simple does not
        assert_ne!(
            simple_xy, q_xy2,
            "Q-factor differs from simple when F has X/Y set"
        );
    }

    #[test]
    fn e05_11_push_bc() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x1234);
        sem(&mut cpu, Table::Main, 0xC5, 0);
        assert_eq!(cpu.eng.tmp_h, 0x12, "PUSH BC: TMP.H = B");
        assert_eq!(cpu.eng.tmp_l, 0x34, "PUSH BC: TMP.L = C");
    }

    #[test]
    fn e05_11_pop_af() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_h = 0xAB;
        cpu.eng.tmp_l = 0xD7;
        sem(&mut cpu, Table::Main, 0xF1, 0);
        assert_eq!(cpu.regs.a, 0xAB, "POP AF: A = TMP.H");
        assert_eq!(cpu.regs.f, 0xD7, "POP AF: F = TMP.L");
    }

    #[test]
    fn e05_11_pop_hl() {
        let mut cpu = Cpu::new();
        cpu.eng.tmp_h = 0x56;
        cpu.eng.tmp_l = 0x78;
        sem(&mut cpu, Table::Main, 0xE1, 0);
        assert_eq!(cpu.regs.hl(), 0x5678, "POP HL loads TMP");
    }

    #[test]
    fn e05_12_jp_taken() {
        let mut cpu = Cpu::new();
        cpu.regs.wz = 0x2000; // Target address in WZ (already read by plan)
        sem(&mut cpu, Table::Main, 0xC3, 0); // JP nn
        assert_eq!(cpu.regs.pc, 0x2000, "JP nn jumps to WZ");
    }

    #[test]
    fn e05_12_jp_cc_not_taken() {
        let mut cpu = Cpu::new();
        cpu.regs.f = flag::Z; // Z set -> NZ condition false
        cpu.regs.wz = 0x3000; // Target address in WZ
        sem(&mut cpu, Table::Main, 0xC2, 0); // JP NZ,nn (y=0)
        assert_ne!(cpu.regs.pc, 0x3000, "JP NZ not taken when Z set");
    }

    #[test]
    fn e05_12_jp_hl() {
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x4000);
        sem(&mut cpu, Table::Main, 0xE9, 0); // JP (HL)
        assert_eq!(cpu.regs.pc, 0x4000, "JP (HL) jumps to HL");
    }
}
