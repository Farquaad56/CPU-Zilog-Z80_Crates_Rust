//! E10-01: MEMPTR (WZ) rules, one test per rule. Each test runs ONE instruction at 0x0100.

use puce_z80::{Bus, Cpu, SimpleRam};

const SENTINEL: u16 = 0xA5A5;
const START: u16 = 0x0100;

/// Run a single instruction (`prog` at 0x0100) after `setup`; WZ starts at the sentinel.
fn run(prog: &[u8], setup: impl FnOnce(&mut Cpu, &mut SimpleRam)) -> Cpu {
    let mut ram = SimpleRam::new();
    for (i, b) in prog.iter().enumerate() {
        ram.write(START + i as u16, *b);
    }
    let mut cpu = Cpu::new();
    cpu.regs.pc = START;
    cpu.regs.sp = 0x8000;
    cpu.regs.f = 0;
    cpu.regs.wz = SENTINEL;
    setup(&mut cpu, &mut ram);
    cpu.step(&mut ram);
    cpu
}

fn wz(prog: &[u8], setup: impl FnOnce(&mut Cpu, &mut SimpleRam)) -> u16 {
    run(prog, setup).regs.wz
}

fn none(_: &mut Cpu, _: &mut SimpleRam) {}

#[test]
fn e10_01_ld_a_nn() {
    assert_eq!(wz(&[0x3A, 0x34, 0x12], none), 0x1235);
}

#[test]
fn e10_01_ld_nn_a() {
    assert_eq!(wz(&[0x32, 0xFF, 0x12], |c, _| c.regs.a = 0x77), 0x7700);
    assert_eq!(wz(&[0x32, 0x34, 0x12], |c, _| c.regs.a = 0x77), 0x7735);
}

#[test]
fn e10_01_ld_a_bc() {
    assert_eq!(wz(&[0x0A], |c, _| c.regs.set_bc(0x2000)), 0x2001);
    assert_eq!(wz(&[0x1A], |c, _| c.regs.set_de(0x20FF)), 0x2100);
    // LD (BC),A and LD (DE),A
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.a = 0x55;
        c.regs.set_bc(0x20FF);
        c.regs.set_de(0x2010);
    };
    assert_eq!(wz(&[0x02], f), 0x5500);
    assert_eq!(wz(&[0x12], f), 0x5511);
}

#[test]
fn e10_01_ld_nn_hl() {
    assert_eq!(wz(&[0x22, 0x00, 0x30], none), 0x3001);
    assert_eq!(wz(&[0x2A, 0x00, 0x30], none), 0x3001);
    assert_eq!(wz(&[0xED, 0x43, 0xFF, 0xFF], none), 0x0000);
    assert_eq!(wz(&[0xED, 0x4B, 0x10, 0x40], none), 0x4011);
    assert_eq!(wz(&[0xED, 0x73, 0x10, 0x40], none), 0x4011);
    assert_eq!(wz(&[0xDD, 0x22, 0x10, 0x40], none), 0x4011);
    assert_eq!(wz(&[0xFD, 0x2A, 0x10, 0x40], none), 0x4011);
}

#[test]
fn e10_01_ex_sp_hl() {
    let f = |_: &mut Cpu, r: &mut SimpleRam| {
        r.write(0x8000, 0x34);
        r.write(0x8001, 0x12);
    };
    assert_eq!(wz(&[0xE3], f), 0x1234);
    assert_eq!(wz(&[0xDD, 0xE3], f), 0x1234);
    assert_eq!(wz(&[0xFD, 0xE3], f), 0x1234);
}

#[test]
fn e10_01_add_hl() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.set_hl(0x1000);
        c.regs.set_bc(0x0234);
        c.regs.ix = 0x3000;
        c.regs.iy = 0x5000;
    };
    assert_eq!(wz(&[0x09], f), 0x1001);
    assert_eq!(wz(&[0xED, 0x4A], f), 0x1001);
    assert_eq!(wz(&[0xED, 0x42], f), 0x1001);
    assert_eq!(wz(&[0xDD, 0x09], f), 0x3001);
    assert_eq!(wz(&[0xFD, 0x09], f), 0x5001);
}

#[test]
fn e10_01_rld() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| c.regs.set_hl(0x4000);
    assert_eq!(wz(&[0xED, 0x6F], f), 0x4001);
    assert_eq!(wz(&[0xED, 0x67], f), 0x4001);
}

#[test]
fn e10_01_jp_call_ret() {
    assert_eq!(wz(&[0xC3, 0x34, 0x12], none), 0x1234);
    assert_eq!(wz(&[0xCD, 0x34, 0x12], none), 0x1234);
    assert_eq!(wz(&[0xE9], none), SENTINEL, "JP (HL) leaves WZ alone");
    assert_eq!(wz(&[0xDF], none), 0x0018);
    let f = |_: &mut Cpu, r: &mut SimpleRam| {
        r.write(0x8000, 0x56);
        r.write(0x8001, 0x34);
    };
    assert_eq!(wz(&[0xC9], f), 0x3456);
    assert_eq!(wz(&[0xED, 0x4D], f), 0x3456);
    assert_eq!(wz(&[0xED, 0x45], f), 0x3456);
    // RET cc taken (Z set, RET Z)
    let g = |c: &mut Cpu, r: &mut SimpleRam| {
        c.regs.f = 0x40;
        f(c, r);
    };
    assert_eq!(wz(&[0xC8], g), 0x3456);
}

#[test]
fn e10_01_jp_cc_not_taken() {
    // JP NZ with Z set: not taken, WZ = nn anyway
    assert_eq!(wz(&[0xC2, 0x34, 0x12], |c, _| c.regs.f = 0x40), 0x1234);
    // taken
    assert_eq!(wz(&[0xC2, 0x34, 0x12], none), 0x1234);
}

#[test]
fn e10_01_call_cc_not_taken() {
    assert_eq!(wz(&[0xC4, 0x34, 0x12], |c, _| c.regs.f = 0x40), 0x1234);
    assert_eq!(wz(&[0xC4, 0x34, 0x12], none), 0x1234);
}

#[test]
fn e10_01_ret_cc_not_taken_unchanged() {
    // RET NZ with Z set: not taken, WZ unchanged
    assert_eq!(wz(&[0xC0], |c, _| c.regs.f = 0x40), SENTINEL);
}

#[test]
fn e10_01_jr_taken() {
    assert_eq!(wz(&[0x18, 0x05], none), 0x0107);
    assert_eq!(wz(&[0x20, 0x05], none), 0x0107);
    assert_eq!(wz(&[0x10, 0x05], |c, _| c.regs.set_bc(0x0200)), 0x0107);
}

#[test]
fn e10_01_jr_not_taken() {
    assert_eq!(wz(&[0x20, 0x05], |c, _| c.regs.f = 0x40), SENTINEL);
    assert_eq!(wz(&[0x10, 0x05], |c, _| c.regs.set_bc(0x0100)), SENTINEL);
}

#[test]
fn e10_01_in_a_n() {
    assert_eq!(wz(&[0xDB, 0x34], |c, _| c.regs.a = 0x12), 0x1235);
    assert_eq!(wz(&[0xDB, 0xFF], |c, _| c.regs.a = 0x12), 0x1300);
}

#[test]
fn e10_01_out_n_a() {
    assert_eq!(wz(&[0xD3, 0xFF], |c, _| c.regs.a = 0x77), 0x7700);
    assert_eq!(wz(&[0xD3, 0x34], |c, _| c.regs.a = 0x77), 0x7735);
}

#[test]
fn e10_01_in_r_c() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| c.regs.set_bc(0x1234);
    assert_eq!(wz(&[0xED, 0x40], f), 0x1235);
    assert_eq!(wz(&[0xED, 0x41], f), 0x1235);
}

#[test]
fn e10_01_cpi_cpd() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.set_hl(0x4000);
        c.regs.set_bc(0x0005);
        c.regs.a = 0x10;
    };
    assert_eq!(wz(&[0xED, 0xA1], f), SENTINEL + 1);
    assert_eq!(wz(&[0xED, 0xA9], f), SENTINEL - 1);
    // LDI/LDD leave WZ alone
    assert_eq!(wz(&[0xED, 0xA0], f), SENTINEL);
    assert_eq!(wz(&[0xED, 0xA8], f), SENTINEL);
}

#[test]
fn e10_01_ini_ind() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.set_bc(0x1234);
        c.regs.set_hl(0x4000);
    };
    assert_eq!(wz(&[0xED, 0xA2], f), 0x1235);
    assert_eq!(wz(&[0xED, 0xAA], f), 0x1233);
}

#[test]
fn e10_01_outi_outd() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.set_bc(0x1234);
        c.regs.set_hl(0x4000);
    };
    assert_eq!(wz(&[0xED, 0xA3], f), 0x1135);
    assert_eq!(wz(&[0xED, 0xAB], f), 0x1133);
}

#[test]
fn e10_01_ldir_repeat() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.set_bc(0x0002);
        c.regs.set_hl(0x4000);
        c.regs.set_de(0x5000);
    };
    // The instruction repeats (BC != 0 after the step): WZ = PC of the instruction + 1.
    assert_eq!(wz(&[0xED, 0xB0], f), 0x0101);
    assert_eq!(wz(&[0xED, 0xB8], f), 0x0101);
}

#[test]
fn e10_01_ix_d() {
    let f = |c: &mut Cpu, _: &mut SimpleRam| {
        c.regs.ix = 0x2000;
        c.regs.iy = 0x3000;
    };
    assert_eq!(wz(&[0xDD, 0x7E, 0x05], f), 0x2005);
    assert_eq!(wz(&[0xFD, 0x77, 0xFE], f), 0x2FFE);
    assert_eq!(wz(&[0xDD, 0x36, 0x05, 0x99], f), 0x2005);
    assert_eq!(wz(&[0xDD, 0x34, 0x05], f), 0x2005);
    assert_eq!(wz(&[0xDD, 0xCB, 0x05, 0x06], f), 0x2005);
}
