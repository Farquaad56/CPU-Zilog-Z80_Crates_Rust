//! Main opcode table semantics.
//!
//! Implements hooks for NOP, DI, EI, HALT, EX AF,AF', EXX, EX DE,HL,
//! and all LD 8-bit register/register/immediate/(HL) instructions.

use super::regs::{hl, r8, rp, rp2, set_f, set_hl, set_r8, set_rp, set_rp2};
use crate::alu;
use crate::cpu::Cpu;
use crate::regs::flag;

/// Execute main-table instruction semantics.
pub(crate) fn sem_main(cpu: &mut Cpu, op: u8, hook_no: u8) {
    if hook_no != 0 {
        sem_main_x1(cpu, op);
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

        // EX (SP),HL (0xE3): X0: TMP = HL; HL = WZ (value read from stack).
        // The plan reads [SP] into Z and [SP+1] into W before this hook.
        // After the hook, the plan writes TMP.H to [SP+1] and TMP.L to [SP].
        0xE3 => {
            let hl_val = hl(cpu);
            cpu.eng.tmp_h = (hl_val >> 8) as u8;
            cpu.eng.tmp_l = hl_val as u8;
            set_hl(cpu, cpu.regs.wz);
        }

        // IN A,(n) (0xDB): X0: A = TMP.L (input byte from bus); WZ = ((A_before<<8)|n)+1.
        // The plan reads port number n into Z and performs the input via N4 before this hook.
        0xDB => {
            let a_before = cpu.regs.a;
            let n = (cpu.regs.wz & 0xFF) as u8;
            cpu.regs.a = cpu.eng.tmp_l;
            let wz_val: u16 = ((a_before as u16) << 8) | (n as u16);
            cpu.regs.wz = wz_val.wrapping_add(1);
        }

        // OUT (n),A (0xD3): X0: TMP.L = A; WZ = ((n+1)&0xFF) | (A<<8).
        // The plan reads port number n into Z before this hook, then outputs via O4.
        0xD3 => {
            // UNKNOWN - to confirm: the port address low byte is read from Z at O4,
            // after this hook, so Z must stay n here. Final Z = n+1 needs a post-bus
            // hook (see docs/qa/inbox/E05-05-Q1.md). W = A is already final.
            cpu.eng.tmp_l = cpu.regs.a;
            cpu.regs.wz = (cpu.regs.wz & 0x00FF) | ((cpu.regs.a as u16) << 8);
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

    // PUSH rp2 (x=3, z=5, q=0): TMP = rp2(p); the plan writes TMP.H then TMP.L.
    if x == 3 && z == 5 && y & 1 == 0 {
        let v = rp2(cpu, y >> 1);
        cpu.eng.tmp_h = (v >> 8) as u8;
        cpu.eng.tmp_l = v as u8;
    }

    // POP rp2 (x=3, z=1, q=0): rp2(p) = TMP (AF: A = H, F = L).
    if x == 3 && z == 1 && y & 1 == 0 {
        let p = y >> 1;
        if p == 3 {
            cpu.regs.a = cpu.eng.tmp_h;
            set_f(cpu, cpu.eng.tmp_l);
        } else {
            set_rp2(cpu, p, ((cpu.eng.tmp_h as u16) << 8) | cpu.eng.tmp_l as u16);
        }
    }

    // JP nn (0xC3): unconditional jump to WZ.
    if op == 0xC3 {
        cpu.regs.pc = cpu.regs.wz;
    }

    // JP cc,nn family (x=3, z=2): conditional jump to WZ if condition met.
    // y encodes the condition code: 0=NZ,1=Z,2=NC,3=C,4=PO,5=PE,6=P,7=M.
    if x == 3 && z == 2 && alu::cond(cpu.regs.f, y) {
        cpu.regs.pc = cpu.regs.wz;
    }

    // JP (HL) (0xE9): jump to HL.
    if op == 0xE9 {
        cpu.regs.pc = hl(cpu);
    }

    // DJNZ d (0x10): X0: B = B-1; cond = (B != 0); D = TMP.L.
    if op == 0x10 {
        let b = cpu.regs.b.wrapping_sub(1);
        cpu.regs.b = b;
        cpu.eng.cond = b != 0;
        cpu.eng.disp = cpu.eng.tmp_l;
    }

    // JR d (0x18): X0: D = TMP.L; cond = true.
    if op == 0x18 {
        cpu.eng.disp = cpu.eng.tmp_l;
        cpu.eng.cond = true;
    }

    // JR cc,d (0x20, 0x28, 0x30, 0x38): X0: D = TMP.L; cond = condition(y-4).
    if op == 0x20 || op == 0x28 || op == 0x30 || op == 0x38 {
        let cc = ((op >> 3) & 7) - 4;
        cpu.eng.disp = cpu.eng.tmp_l;
        cpu.eng.cond = alu::cond(cpu.regs.f, cc);
    }

    // CALL nn (0xCD): X0: TMP = PC (return address).
    if op == 0xCD {
        cpu.eng.tmp_h = (cpu.regs.pc >> 8) as u8;
        cpu.eng.tmp_l = cpu.regs.pc as u8;
    }

    // CALL cc,nn family (x=3, z=4): X0: cond = condition(y); TMP = PC.
    if x == 3 && z == 4 {
        cpu.eng.cond = alu::cond(cpu.regs.f, y);
        cpu.eng.tmp_h = (cpu.regs.pc >> 8) as u8;
        cpu.eng.tmp_l = cpu.regs.pc as u8;
    }

    // RST n*8 family (x=3, z=7): X0: TMP = PC.
    if x == 3 && z == 7 {
        cpu.eng.tmp_h = (cpu.regs.pc >> 8) as u8;
        cpu.eng.tmp_l = cpu.regs.pc as u8;
    }

    // RET (0xC9): X0: PC = WZ (address read from stack).
    if op == 0xC9 {
        cpu.regs.pc = cpu.regs.wz;
    }

    // RET cc family (x=3, z=0): X0: cond = condition(y).
    if x == 3 && z == 0 {
        cpu.eng.cond = alu::cond(cpu.regs.f, y);
    }
}

/// Hooks X1 (after the bus/internal cycles) of the main table.
fn sem_main_x1(cpu: &mut Cpu, op: u8) {
    // JR / JR cc / DJNZ: if cond, PC = PC + signed(D); WZ = PC.
    if matches!(op, 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38) && cpu.eng.cond {
        let target = signed_jr_target(cpu.regs.pc, cpu.eng.disp);
        cpu.regs.wz = target;
        cpu.regs.pc = target;
    }

    // CALL nn (0xCD): X1: PC = WZ.
    if op == 0xCD {
        cpu.regs.pc = cpu.regs.wz;
    }

    // CALL cc,nn family (x=3, z=4): X1: if cond, PC = WZ.
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;
    if x == 3 && z == 4 && cpu.eng.cond {
        cpu.regs.pc = cpu.regs.wz;
    }

    // RST n*8 family (x=3, z=7): X1: PC = y*8; WZ = PC.
    if x == 3 && z == 7 {
        let target = (y as u16) * 8;
        cpu.regs.wz = target;
        cpu.regs.pc = target;
    }

    // RET cc family (x=3, z=0): X1: if cond, PC = WZ.
    if x == 3 && z == 0 && cpu.eng.cond {
        cpu.regs.pc = cpu.regs.wz;
    }
}
/// Compute PC + signed(displacement) for JR/JR cc/DJNZ.
fn signed_jr_target(pc: u16, disp: u8) -> u16 {
    let offset = disp as i8 as i32;
    (pc as i32 + offset) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regs::flag;

    #[test]
    fn e05_13_jr_forward() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1000;
        cpu.eng.tmp_l = 5; // +5 displacement
        sem_main(&mut cpu, 0x18, 0); // JR d X0: store disp, cond=true
        assert_eq!(cpu.eng.disp, 5);
        assert!(cpu.eng.cond);
        sem_main_x1(&mut cpu, 0x18); // JR d X1: jump
        assert_eq!(cpu.regs.pc, 0x1005);
        assert_eq!(cpu.regs.wz, 0x1005);
    }

    #[test]
    fn e05_13_jr_cc_taken_12t() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1000;
        cpu.regs.f = 0x00; // Z clear -> NZ condition true
        cpu.eng.tmp_l = 3;
        sem_main(&mut cpu, 0x20, 0); // JR NZ,d X0: store disp, evaluate cond
        assert!(cpu.eng.cond);
        sem_main_x1(&mut cpu, 0x20); // JR NZ,d X1: jump taken
        assert_eq!(cpu.regs.pc, 0x1003);
        assert_eq!(cpu.regs.wz, 0x1003);
    }

    #[test]
    fn e05_13_jr_cc_not_taken_7t() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1000;
        cpu.regs.f = flag::Z; // Z set -> NZ condition false
        cpu.eng.tmp_l = 3;
        sem_main(&mut cpu, 0x20, 0); // JR NZ,d X0: store disp, evaluate cond
        assert!(!cpu.eng.cond);
        let pc_before = cpu.regs.pc;
        sem_main_x1(&mut cpu, 0x20); // JR NZ,d X1: jump not taken
        assert_eq!(cpu.regs.pc, pc_before);
    }

    #[test]
    fn e05_13_djnz_loop() {
        let mut cpu = Cpu::new();
        cpu.regs.b = 2;
        cpu.regs.pc = 0x1000;
        cpu.eng.tmp_l = 252; // -4 as signed byte (0xFC)

        // First iteration: B=2 -> B=1, cond=true, jump taken (13 T)
        sem_main(&mut cpu, 0x10, 0);
        assert_eq!(cpu.regs.b, 1);
        assert!(cpu.eng.cond);
        sem_main_x1(&mut cpu, 0x10);
        assert_eq!(cpu.regs.pc, 0x0FFC);

        // Second iteration: B=1 -> B=0, cond=false, jump not taken (8 T)
        cpu.regs.pc = 0x1000;
        sem_main(&mut cpu, 0x10, 0);
        assert_eq!(cpu.regs.b, 0);
        assert!(!cpu.eng.cond);
        let pc_before = cpu.regs.pc;
        sem_main_x1(&mut cpu, 0x10);
        assert_eq!(cpu.regs.pc, pc_before);
    }

    #[test]
    fn e05_13_jr_backward() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1000;
        cpu.eng.tmp_l = 254; // -2 as signed byte (0xFE)
        sem_main(&mut cpu, 0x18, 0);
        assert_eq!(cpu.eng.disp, 254);
        sem_main_x1(&mut cpu, 0x18);
        assert_eq!(cpu.regs.pc, 0x0FFE);
        assert_eq!(cpu.regs.wz, 0x0FFE);
    }

    #[test]
    fn e05_14_call() {
        // CALL nn: push return address (PC after opcode), jump to target.
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1002; // PC already past the 3-byte CALL instruction
        cpu.regs.sp = 0xFFFE;
        cpu.regs.wz = 0x2000; // target address

        // X0: TMP = PC (return address)
        sem_main(&mut cpu, 0xCD, 0);
        assert_eq!(cpu.eng.tmp_h, 0x10);
        assert_eq!(cpu.eng.tmp_l, 0x02);

        // Simulate stack writes (plan does W3:DecSp>H then W3:DecSp<L)
        cpu.regs.sp = 0xFFFC;

        // X1: PC = WZ
        sem_main_x1(&mut cpu, 0xCD);
        assert_eq!(cpu.regs.pc, 0x2000);
    }

    #[test]
    fn e05_14_call_cc_not_taken_10t() {
        // CALL NZ,nn not taken (Z flag set): PC unchanged after X1.
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1002;
        cpu.regs.f = flag::Z; // Z set -> NZ condition false
        cpu.regs.wz = 0x2000;

        // X0: evaluate condition, store return address in TMP
        sem_main(&mut cpu, 0xC4, 0); // CALL NZ,nn
        assert!(!cpu.eng.cond);

        // Simulate stack writes (still happen even if not taken)
        cpu.regs.sp = 0xFFFC;

        // X1: condition false -> PC unchanged
        sem_main_x1(&mut cpu, 0xC4);
        assert_eq!(cpu.regs.pc, 0x1002);
    }

    #[test]
    fn e05_14_ret() {
        // RET: pop return address from stack into PC.
        let mut cpu = Cpu::new();
        cpu.regs.sp = 0xFFFE;
        cpu.regs.wz = 0x3000; // address read from stack (WZ set by plan)

        // X0: PC = WZ
        sem_main(&mut cpu, 0xC9, 0);
        assert_eq!(cpu.regs.pc, 0x3000);
    }

    #[test]
    fn e05_14_ret_cc_11t() {
        // RET Z taken (Z flag set): PC = WZ.
        let mut cpu = Cpu::new();
        cpu.regs.f = flag::Z; // Z set -> Z condition true
        cpu.regs.wz = 0x4000;

        // X0: evaluate condition
        sem_main(&mut cpu, 0xC8, 0); // RET Z
        assert!(cpu.eng.cond);

        // X1: condition true -> PC = WZ
        sem_main_x1(&mut cpu, 0xC8);
        assert_eq!(cpu.regs.pc, 0x4000);
    }

    #[test]
    fn e05_14_rst38() {
        // RST 38H: push return address, jump to 0x38.
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1001; // PC already past the 1-byte RST instruction
        cpu.regs.sp = 0xFFFE;

        // X0: TMP = PC (return address)
        sem_main(&mut cpu, 0xFF, 0); // RST 38H
        assert_eq!(cpu.eng.tmp_h, 0x10);
        assert_eq!(cpu.eng.tmp_l, 0x01);

        // Simulate stack writes (plan does W3:DecSp>H then W3:DecSp<L)
        cpu.regs.sp = 0xFFFC;

        // X1: PC = y*8 = 7*8 = 56 = 0x38; WZ = PC
        sem_main_x1(&mut cpu, 0xFF);
        assert_eq!(cpu.regs.pc, 0x38);
        assert_eq!(cpu.regs.wz, 0x38);
    }

    #[test]
    fn e05_15_ex_sp_hl() {
        // EX (SP),HL: swap HL with word at [SP].
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.regs.sp = 0xFFFE;
        // Simulate plan reading [SP] into Z and [SP+1] into W (WZ = value at stack)
        cpu.regs.wz = 0xABCD;

        // X0: TMP = HL, HL = WZ
        sem_main(&mut cpu, 0xE3, 0);
        assert_eq!(cpu.eng.tmp_h, 0x12);
        assert_eq!(cpu.eng.tmp_l, 0x34);
        assert_eq!(cpu.regs.hl(), 0xABCD);

        // X1: nothing
        sem_main_x1(&mut cpu, 0xE3);
    }

    #[test]
    fn e05_15_in_a_n() {
        // IN A,(n): read from port n into A.
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x00;
        cpu.regs.wz = 0x42; // port number n in Z (low byte of WZ)
        cpu.eng.tmp_l = 0xFF; // input byte from bus (simulated)

        sem_main(&mut cpu, 0xDB, 0);
        assert_eq!(cpu.regs.a, 0xFF);
    }

    #[test]
    fn e05_15_out_n_a() {
        // OUT (n),A: write A to port n.
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x55;
        cpu.regs.wz = 0x3C; // port number n in Z

        sem_main(&mut cpu, 0xD3, 0);
        assert_eq!(cpu.eng.tmp_l, 0x55);
    }

    #[test]
    fn e05_15_wz_in() {
        // WZ formula for IN A,(n): ((A_before<<8)|n)+1.
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x12;
        cpu.regs.wz = 0x34; // port n in Z
        cpu.eng.tmp_l = 0xFF;

        sem_main(&mut cpu, 0xDB, 0);
        // WZ = ((0x12 << 8) | 0x34) + 1 = 0x1235
        assert_eq!(cpu.regs.wz, 0x1235);
    }

    #[test]
    fn e05_15_wz_out() {
        // WZ for OUT (n),A: W = A (final); Z = n (see UNKNOWN below).
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x56;
        cpu.regs.wz = 0x7F; // port n in Z

        sem_main(&mut cpu, 0xD3, 0);
        // UNKNOWN - to confirm: Z stays n (0x7F) until a post-bus hook exists; W = A.
        assert_eq!(cpu.regs.wz, 0x567F);
    }
}
