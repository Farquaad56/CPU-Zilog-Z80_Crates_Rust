//! Main opcode table semantics.
//!
//! Implements hooks for NOP, DI, EI, HALT, EX AF,AF', EXX, EX DE,HL,
//! and all LD 8-bit register/register/immediate/(HL) instructions.

use super::regs::{r8, set_r8};
use crate::cpu::Cpu;

/// Execute main-table instruction semantics.
pub(crate) fn sem_main(cpu: &mut Cpu, op: u8, hook_no: u8) {
    if hook_no != 0 {
        return;
    }

    // Decode opcode fields for LD instructions
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;

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

    // LD r,r / LD r,(HL) / LD (HL),r family (opcodes 0x40-0x7F, excluding HALT at 0x76)
    if x == 1 && op != 0x76 {
        if y == 6 && z != 6 {
            // LD (HL),r(z): store register into memory via tmp_l
            cpu.eng.tmp_l = r8(cpu, z);
        } else if y != 6 && z == 6 {
            // LD r(y),(HL): load from memory (already in tmp_l) into register
            set_r8(cpu, y, cpu.eng.tmp_l);
        } else {
            // LD r(y),r(z): register to register
            set_r8(cpu, y, r8(cpu, z));
        }
    }

    // LD r,n / LD (HL),n family (opcodes with x=0, z=6)
    // LD (HL),n (y=6) needs no hook: tmp_l already contains the byte.
    if x == 0 && z == 6 && y != 6 {
        // LD r(y),n: immediate value already in tmp_l from data fetch
        set_r8(cpu, y, cpu.eng.tmp_l);
    }

    // LD (BC),A / LD A,(BC) / LD (DE),A / LD A,(DE) / LD (nn),A / LD A,(nn)
    // All have x=0, z=2; distinguished by y.
    if x == 0 && z == 2 {
        match y {
            // LD (BC),A: store A at address in BC
            0 => {
                cpu.eng.tmp_l = cpu.regs.a;
                let bc_plus1 = cpu.regs.bc().wrapping_add(1);
                cpu.regs.wz = ((bc_plus1 & 0xFF) as u8 as u16) | ((cpu.regs.a as u16) << 8);
            }
            // LD A,(BC): load from address in BC into A
            1 => {
                cpu.regs.a = cpu.eng.tmp_l;
                let bc_plus1 = cpu.regs.bc().wrapping_add(1);
                cpu.regs.wz = bc_plus1;
            }
            // LD (DE),A: store A at address in DE
            2 => {
                cpu.eng.tmp_l = cpu.regs.a;
                let de_plus1 = cpu.regs.de().wrapping_add(1);
                cpu.regs.wz = ((de_plus1 & 0xFF) as u8 as u16) | ((cpu.regs.a as u16) << 8);
            }
            // LD A,(DE): load from address in DE into A
            3 => {
                cpu.regs.a = cpu.eng.tmp_l;
                let de_plus1 = cpu.regs.de().wrapping_add(1);
                cpu.regs.wz = de_plus1;
            }
            // LD (nn),A: store A at absolute address nn
            // UNKNOWN - to confirm: the plan is X0 W3:NN<L with no hook after the write,
            // and NN is read from WZ at the write, so WZ must stay nn here. The final
            // WZ (((nn+1)&0xFF)|(A<<8)) needs a post-write hook. See docs/qa/inbox/E05-05-Q1.md.
            6 => {
                cpu.eng.tmp_l = cpu.regs.a;
            }
            // LD A,(nn): load from absolute address nn into A
            7 => {
                cpu.regs.a = cpu.eng.tmp_l;
                let nn_plus1 = cpu.regs.wz.wrapping_add(1);
                cpu.regs.wz = nn_plus1;
            }
            _ => {}
        }
    }
}
