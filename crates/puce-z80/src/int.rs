//! Interrupt, HALT and RESET handling.
//!
//! E09-01: RESET (pin handling lives in `Cpu::tick`, the register state in `Cpu::reset`).

use crate::cpu::Cpu;

impl Cpu {
    /// Apply the RESET state: PC = 0, I = R = 0, IFF1 = IFF2 = false, IM = 0, AF = SP = 0xFFFF,
    /// WZ = 0, not halted, and the engine back at an instruction boundary.
    ///
    /// The other registers (BC DE HL IX IY and the shadow set) are left unchanged, as on the NMOS
    /// chip. UNKNOWN - to confirm (Q-001): the real values of the undefined registers after a
    /// power-on RESET.
    pub fn reset(&mut self) {
        self.regs.pc = 0;
        self.regs.i = 0;
        self.regs.r = 0;
        self.regs.iff1 = false;
        self.regs.iff2 = false;
        self.regs.im = 0;
        self.regs.set_af(0xFFFF);
        self.regs.sp = 0xFFFF;
        self.regs.wz = 0;
        self.regs.q = 0;
        self.regs.halted = false;

        self.eng.active = false;
        self.eng.cond = false;
        self.eng.ei_delay = false;
        self.eng.ddcb = false;
        self.eng.f_written = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pins::Pins;

    #[test]
    fn e09_01_reset_values() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1234;
        cpu.regs.i = 0x55;
        cpu.regs.r = 0x66;
        cpu.regs.iff1 = true;
        cpu.regs.iff2 = true;
        cpu.regs.im = 2;
        cpu.regs.set_af(0x1234);
        cpu.regs.sp = 0x8000;
        cpu.regs.wz = 0xABCD;
        cpu.reset();
        assert_eq!(cpu.regs.pc, 0);
        assert_eq!(cpu.regs.i, 0);
        assert_eq!(cpu.regs.r, 0);
        assert!(!cpu.regs.iff1);
        assert!(!cpu.regs.iff2);
        assert_eq!(cpu.regs.im, 0);
        assert_eq!(cpu.regs.af(), 0xFFFF);
        assert_eq!(cpu.regs.sp, 0xFFFF);
        assert_eq!(cpu.regs.wz, 0);
        // Cpu::new() applies the same state.
        let fresh = Cpu::new();
        assert_eq!(fresh.regs.af(), 0xFFFF);
        assert_eq!(fresh.regs.sp, 0xFFFF);
        assert_eq!(fresh.regs.bc(), 0);
    }

    #[test]
    fn e09_01_reset_midstream() {
        let mut cpu = Cpu::new();
        // Start LD HL,nn (opcode 0x21): feed the opcode byte on the data pins.
        let pins = Pins(0).set_data(0x21);
        for _ in 0..5 {
            cpu.tick(pins);
        }
        assert!(!cpu.at_instruction_boundary());
        cpu.eng.cond = true;
        cpu.eng.ei_delay = true;
        cpu.reset();
        assert!(cpu.at_instruction_boundary());
        assert!(!cpu.eng.cond);
        assert!(!cpu.eng.ei_delay);
        assert_eq!(cpu.regs.pc, 0);
    }

    #[test]
    fn e09_01_reset_pin_holds() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x1234;
        let ctrl = Pins::M1 | Pins::MREQ | Pins::IORQ | Pins::RD | Pins::WR | Pins::RFSH;
        for _ in 0..5 {
            let out = cpu.tick(Pins(0).with(Pins::RESET));
            assert_eq!(out.0 & ctrl, 0, "no control output while RESET is held");
            assert_eq!(cpu.regs.pc, 0);
            assert!(cpu.at_instruction_boundary());
        }
        // After release, the next instruction starts normally: the first M1 reads PC = 0.
        cpu.tick(Pins(0));
        assert!(!cpu.at_instruction_boundary());
        assert_eq!(cpu.regs.pc, 1);
    }

    #[test]
    fn e09_01_reset_pin_resets_once() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x4000;
        cpu.tick(Pins(0).with(Pins::RESET));
        assert_eq!(cpu.regs.pc, 0);
        // Registers touched while the pin stays active are not reset again.
        cpu.regs.pc = 0x0010;
        cpu.tick(Pins(0).with(Pins::RESET));
        assert_eq!(cpu.regs.pc, 0x0010);
        // A single-tick pulse (shorter than 3 T) still resets.
        cpu.tick(Pins(0));
        cpu.regs.pc = 0x2000;
        cpu.tick(Pins(0).with(Pins::RESET));
        assert_eq!(cpu.regs.pc, 0);
    }

    #[test]
    fn e09_01_reset_clears_halt() {
        let mut cpu = Cpu::new();
        cpu.regs.halted = true;
        cpu.reset();
        assert!(!cpu.regs.halted);
    }

    #[test]
    fn e09_01_others_unchanged() {
        let mut cpu = Cpu::new();
        cpu.regs.set_bc(0x1111);
        cpu.regs.set_de(0x2222);
        cpu.regs.set_hl(0x3333);
        cpu.regs.ix = 0x4444;
        cpu.regs.iy = 0x5555;
        cpu.regs.set_bc2(0x6666);
        cpu.reset();
        assert_eq!(cpu.regs.bc(), 0x1111);
        assert_eq!(cpu.regs.de(), 0x2222);
        assert_eq!(cpu.regs.hl(), 0x3333);
        assert_eq!(cpu.regs.ix, 0x4444);
        assert_eq!(cpu.regs.iy, 0x5555);
        assert_eq!(cpu.regs.bc2(), 0x6666);
    }

    use crate::bus::{Bus, SimpleRam};

    /// CPU that has just executed HALT at address 0 (halted, PC on the HALT opcode).
    fn halted_cpu() -> (Cpu, SimpleRam) {
        let mut ram = SimpleRam::new();
        ram.write(0, 0x76);
        let mut cpu = Cpu::new();
        assert_eq!(cpu.step(&mut ram), 4);
        assert!(cpu.regs.halted);
        (cpu, ram)
    }

    #[test]
    fn e09_02_halt_4t() {
        let (mut cpu, mut ram) = halted_cpu();
        for _ in 0..3 {
            assert_eq!(cpu.step(&mut ram), 4);
        }
        assert!(cpu.regs.halted);
    }

    #[test]
    fn e09_02_halt_pin() {
        let (mut cpu, _ram) = halted_cpu();
        for t in 0..8 {
            let out = cpu.tick(Pins(0));
            assert!(out.has(Pins::HALT), "HALT pin active at T{t}");
        }
    }

    #[test]
    fn e09_02_halt_pc_stable() {
        let (mut cpu, mut ram) = halted_cpu();
        let pc = cpu.regs.pc;
        for _ in 0..3 {
            cpu.step(&mut ram);
            assert_eq!(cpu.regs.pc, pc);
        }
    }

    #[test]
    fn e09_02_halt_r_increments() {
        let (mut cpu, mut ram) = halted_cpu();
        for _ in 0..3 {
            let r = cpu.regs.r;
            cpu.step(&mut ram);
            assert_eq!(cpu.regs.r & 0x7F, (r.wrapping_add(1)) & 0x7F);
        }
    }

    #[test]
    fn e09_02_halt_no_bus_write() {
        let (mut cpu, _ram) = halted_cpu();
        for _ in 0..12 {
            let out = cpu.tick(Pins(0));
            assert!(!out.has(Pins::WR));
            assert!(!out.has(Pins::IORQ));
        }
    }
}
