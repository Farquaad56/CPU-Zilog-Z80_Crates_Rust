//! E13-09: bugs found by SingleStepTests (final state).

use puce_z80::{Bus, Cpu, SimpleRam};

fn run(prog: &[u8], setup: impl Fn(&mut Cpu, &mut SimpleRam)) -> Cpu {
    let mut ram = SimpleRam::new();
    for (i, b) in prog.iter().enumerate() {
        ram.write(0x0100 + i as u16, *b);
    }
    let mut cpu = Cpu::new();
    cpu.regs.pc = 0x0100;
    cpu.regs.sp = 0x8000;
    setup(&mut cpu, &mut ram);
    cpu.step(&mut ram);
    cpu
}

#[test]
fn e13_09_ld_rp_nn_keeps_wz() {
    for prog in [
        [0x01u8, 0x34, 0x12],
        [0x11, 0x34, 0x12],
        [0x21, 0x34, 0x12],
        [0x31, 0x34, 0x12],
    ] {
        let cpu = run(&prog, |c, _| c.regs.wz = 0xABCD);
        assert_eq!(cpu.regs.wz, 0xABCD, "opcode {:02X}", prog[0]);
    }
    let cpu = run(&[0xDD, 0x21, 0x34, 0x12], |c, _| c.regs.wz = 0xABCD);
    assert_eq!(cpu.regs.ix, 0x1234);
    assert_eq!(cpu.regs.wz, 0xABCD);
}

#[test]
fn e13_09_pop_af_q_zero() {
    let cpu = run(&[0xF1], |c, ram| {
        c.regs.q = 0x55;
        ram.write(0x8000, 0xA9);
        ram.write(0x8001, 0x12);
    });
    assert_eq!((cpu.regs.a, cpu.regs.f), (0x12, 0xA9));
    assert_eq!(cpu.regs.q, 0, "POP AF does not produce Q");
}
