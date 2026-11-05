//! E10-03: the Q factor (`regs.q`) and its effect on SCF / CCF.

use puce_z80::{Bus, Cpu, SimpleRam};

const START: u16 = 0x0100;

/// CPU at 0x0100 with `prog` loaded and a stack at 0x8000.
fn setup(prog: &[u8]) -> (Cpu, SimpleRam) {
    let mut ram = SimpleRam::new();
    for (i, b) in prog.iter().enumerate() {
        ram.write(START + i as u16, *b);
    }
    let mut cpu = Cpu::new();
    cpu.regs.pc = START;
    cpu.regs.sp = 0x8000;
    cpu.regs.f = 0;
    cpu.regs.q = 0;
    (cpu, ram)
}

#[test]
fn e10_03_q_after_add() {
    let (mut cpu, mut ram) = setup(&[0x80]);
    cpu.regs.a = 0x7F;
    cpu.regs.b = 0x01;
    cpu.step(&mut ram);
    assert_ne!(cpu.regs.f, 0);
    assert_eq!(cpu.regs.q, cpu.regs.f, "ADD writes F, so Q = F");
}

#[test]
fn e10_03_q_zero_after_ld() {
    let (mut cpu, mut ram) = setup(&[0x80, 0x3E, 0x05, 0x23, 0xF5]);
    cpu.regs.a = 0x7F;
    cpu.regs.b = 0x01;
    cpu.step(&mut ram);
    assert_ne!(cpu.regs.q, 0);
    cpu.step(&mut ram); // LD A,5
    assert_eq!(cpu.regs.q, 0);
    cpu.regs.q = 0xFF;
    cpu.step(&mut ram); // INC HL
    assert_eq!(cpu.regs.q, 0, "INC rp does not touch F");
    cpu.regs.q = 0xFF;
    cpu.step(&mut ram); // PUSH AF
    assert_eq!(cpu.regs.q, 0, "PUSH does not touch F");
}

#[test]
fn e10_03_pop_af_q() {
    let (mut cpu, mut ram) = setup(&[0xF1]);
    ram.write(0x8000, 0xD7);
    ram.write(0x8001, 0x12);
    cpu.step(&mut ram);
    assert_eq!(cpu.regs.f, 0xD7);
    // SingleStepTests (E13-09, 1000 cases): POP AF loads F from the stack but leaves Q = 0.
    assert_eq!(cpu.regs.q, 0, "POP AF does not produce Q");
}

#[test]
fn e10_03_ld_a_i_q() {
    let (mut cpu, mut ram) = setup(&[0xED, 0x57]);
    cpu.regs.i = 0x80;
    cpu.step(&mut ram);
    assert_eq!(cpu.regs.q, cpu.regs.f);
    assert_ne!(cpu.regs.q, 0);
}
