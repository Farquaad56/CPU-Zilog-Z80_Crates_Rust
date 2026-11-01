//! Main opcode table semantics.
//!
//! Implements hooks for NOP, DI, EI, HALT, EX AF,AF', EXX, and EX DE,HL.

use crate::cpu::Cpu;

/// Execute main-table instruction semantics.
pub(crate) fn sem_main(cpu: &mut Cpu, op: u8, hook_no: u8) {
    if hook_no != 0 {
        return;
    }
    match op {
        // NOP (0x00): no hook needed - already a no-op
        0x00 => {}

        // DI (0xF3): disable interrupts
        0xF3 => {
            cpu.regs.iff1 = false;
            cpu.regs.iff2 = false;
        }

        // EI (0xFB): enable interrupts with delay
        0xFB => {
            cpu.regs.iff1 = true;
            cpu.regs.iff2 = true;
            cpu.eng.ei_delay = true;
        }

        // HALT (0x76): halt CPU, PC stays on HALT opcode
        0x76 => {
            cpu.regs.halted = true;
            cpu.regs.pc = cpu.regs.pc.wrapping_sub(1);
        }

        // EX AF,AF' (0x08)
        0x08 => {
            cpu.regs.ex_af();
        }

        // EXX (0xD9): exchange BC,DE,HL with BC',DE',HL'
        0xD9 => {
            cpu.regs.exx();
        }

        // EX DE,HL (0xEB)
        0xEB => {
            cpu.regs.ex_de_hl();
        }

        _ => {}
    }
}
