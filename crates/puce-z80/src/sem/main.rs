//! Main opcode table semantics.
//!
//! Implements hooks for NOP, DI, EI, HALT, EX AF,AF', EXX, EX DE,HL,
//! and all LD 8-bit register/register/immediate/(HL) instructions.

use super::regs::{hl, r8, rp, set_f, set_hl, set_r8, set_rp};
use crate::alu;
use crate::cpu::Cpu;
use crate::regs::flag;

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

        // LD SP,HL (0xF9): copy HL to SP
        0xF9 => {
            set_rp(cpu, 3, hl(cpu));
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

    // LD rp,nn family (opcodes with x=0, z=1): load register pair with immediate
    // Only even y (q=0); odd y is ADD HL,rp, which has its own semantics.
    if x == 0 && z == 1 && y & 1 == 0 {
        let p = y >> 1;
        set_rp(cpu, p, cpu.regs.wz);
    }

    // LD (nn),HL / LD HL,(nn) family (opcodes with x=0, z=2, y=4 or y=5)
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
            // LD (nn),HL: store HL at absolute address nn
            4 => {
                cpu.eng.tmp_l = cpu.regs.l;
                cpu.eng.tmp_h = cpu.regs.h;
            }
            // LD HL,(nn): load from absolute address nn into HL
            5 => {
                set_hl(cpu, ((cpu.eng.tmp_h as u16) << 8) | (cpu.eng.tmp_l as u16));
                let nn_plus1 = cpu.regs.wz.wrapping_add(1);
                cpu.regs.wz = nn_plus1;
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

    // INC r / DEC r family (opcodes with x=0, z=4 or z=5)
    if x == 0 && (z == 4 || z == 5) {
        let v = r8(cpu, y);
        let old_f = cpu.regs.f;
        if z == 4 {
            // INC r: increment register
            let (res, new_f) = alu::inc8(v, old_f);
            set_r8(cpu, y, res);
            set_f(cpu, new_f);
        } else {
            // DEC r: decrement register
            let (res, new_f) = alu::dec8(v, old_f);
            set_r8(cpu, y, res);
            set_f(cpu, new_f);
        }
    }

    // INC rp family (opcodes with x=0, z=3, y even): increment register pair, no flags
    if x == 0 && z == 3 && y & 1 == 0 {
        let p = y >> 1;
        let v = rp(cpu, p).wrapping_add(1);
        set_rp(cpu, p, v);
    }

    // DEC rp family (opcodes with x=0, z=3, y odd): decrement register pair, no flags
    if x == 0 && z == 3 && y & 1 != 0 {
        let p = y >> 1;
        let v = rp(cpu, p).wrapping_sub(1);
        set_rp(cpu, p, v);
    }

    // ADD HL,rp family (opcodes with x=0, z=1, y odd): add register pair to HL with flags
    if x == 0 && z == 1 && y & 1 != 0 {
        let p = y >> 1;
        let hl_before = hl(cpu);
        let rp_val = rp(cpu, p);
        // WZ = HL + 1 (before addition)
        cpu.regs.wz = hl_before.wrapping_add(1);
        let (res, new_f) = alu::add16(hl_before, rp_val, cpu.regs.f);
        set_hl(cpu, res);
        set_f(cpu, new_f);
    }

    // ALU 8-bit register family (opcodes with x=2): ADD/ADC/SUB/SBC/AND/XOR/OR/CP A,r
    if x == 2 {
        let a = cpu.regs.a;
        let b = r8(cpu, z);
        let (res, new_f) = alu::alu8(y, a, b, cpu.regs.f);
        // CP preserves A; all other ALU ops store result in A
        if y != 7 {
            set_r8(cpu, 7, res);
        }
        set_f(cpu, new_f);
    }

    // ALU 8-bit immediate family (opcodes with x=3, z=6): ADD/ADC/SUB/SBC/AND/XOR/OR/CP A,n
    if x == 3 && z == 6 {
        let a = cpu.regs.a;
        let b = cpu.eng.tmp_l; // immediate value already loaded into tmp_l
        let (res, new_f) = alu::alu8(y, a, b, cpu.regs.f);
        if y != 7 {
            set_r8(cpu, 7, res);
        }
        set_f(cpu, new_f);
    }

    // Accumulator operations family (opcodes with x=0, z=7): RLCA RRCA RLA RRA DAA CPL SCF CCF
    if x == 0 && z == 7 {
        let a = cpu.regs.a;
        let old_f = cpu.regs.f;

        match y {
            // RLCA: rotate left through carry (bit 7 -> bit 0 and carry)
            0 => {
                let c_out = (a >> 7) & 1;
                let res = (a << 1) | c_out;
                cpu.regs.a = res;
                // S Z P/V unchanged, H=N=0, C=c_out, X Y from result
                let mut flags = old_f & (flag::S | flag::Z | flag::PV);
                flags |= res & 0x28; // X Y from result
                if c_out != 0 {
                    flags |= flag::C;
                }
                set_f(cpu, flags);
            }

            // RRCA: rotate right through carry (bit 0 -> bit 7 and carry)
            1 => {
                let c_out = a & 1;
                let res = (a >> 1) | (c_out << 7);
                cpu.regs.a = res;
                // S Z P/V unchanged, H=N=0, C=c_out, X Y from result
                let mut flags = old_f & (flag::S | flag::Z | flag::PV);
                flags |= res & 0x28; // X Y from result
                if c_out != 0 {
                    flags |= flag::C;
                }
                set_f(cpu, flags);
            }

            // RLA: rotate left through carry (carry -> bit 0, bit 7 -> carry)
            2 => {
                let c_in = old_f & 1;
                let c_out = (a >> 7) & 1;
                let res = (a << 1) | c_in;
                cpu.regs.a = res;
                // S Z P/V unchanged, H=N=0, C=c_out, X Y from result
                let mut flags = old_f & (flag::S | flag::Z | flag::PV);
                flags |= res & 0x28; // X Y from result
                if c_out != 0 {
                    flags |= flag::C;
                }
                set_f(cpu, flags);
            }

            // RRA: rotate right through carry (carry -> bit 7, bit 0 -> carry)
            3 => {
                let c_in = old_f & 1;
                let c_out = a & 1;
                let res = (a >> 1) | (c_in << 7);
                cpu.regs.a = res;
                // S Z P/V unchanged, H=N=0, C=c_out, X Y from result
                let mut flags = old_f & (flag::S | flag::Z | flag::PV);
                flags |= res & 0x28; // X Y from result
                if c_out != 0 {
                    flags |= flag::C;
                }
                set_f(cpu, flags);
            }

            // DAA: decimal adjust accumulator
            4 => {
                let (res, new_f) = alu::daa(a, old_f);
                cpu.regs.a = res;
                set_f(cpu, new_f);
            }

            // CPL: complement A (S Z P/V C unchanged, H=N=1, X Y from result)
            5 => {
                let res = !a;
                cpu.regs.a = res;
                let flags = (old_f & (flag::S | flag::Z | flag::PV | flag::C))
                    | flag::H
                    | flag::N
                    | (res & 0x28);
                set_f(cpu, flags);
            }

            // SCF: C=1, H=N=0, S Z P/V unchanged, X Y from A (reference model)
            6 => {
                let flags = (old_f & (flag::S | flag::Z | flag::PV)) | flag::C | (a & 0x28);
                set_f(cpu, flags);
            }

            // CCF: H = old C, C = !old C, N=0, S Z P/V unchanged, X Y from A
            7 => {
                let c_or_h = if old_f & flag::C != 0 {
                    flag::H
                } else {
                    flag::C
                };
                let flags = (old_f & (flag::S | flag::Z | flag::PV)) | c_or_h | (a & 0x28);
                set_f(cpu, flags);
            }
            _ => {}
        }
    }
}
