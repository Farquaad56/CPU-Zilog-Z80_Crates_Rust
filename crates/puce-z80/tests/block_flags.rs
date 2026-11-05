//! E10-02: X/Y flags of repeated block instructions.
//!
//! The differential vectors (JS reference) derive bits 3 and 5 of F from the iteration result
//! even when the instruction loops back. Real NMOS silicon copies PC high instead (z80test).
//! The crate follows the vectors for now; see docs/qa/inbox/E10-02-Q1.md.

use puce_z80::{Bus, Cpu, SimpleRam};

const X: u8 = 0x08;
const Y: u8 = 0x20;
const START: u16 = 0x0100;

/// Run one `ED op` at 0x0100 with A, BC, HL set and the byte at HL given.
fn run_block(op: u8, a: u8, bc: u16, byte: u8) -> Cpu {
    let mut ram = SimpleRam::new();
    ram.write(START, 0xED);
    ram.write(START + 1, op);
    ram.write(0x4000, byte);
    let mut cpu = Cpu::new();
    cpu.regs.pc = START;
    cpu.regs.sp = 0x8000;
    cpu.regs.f = 0;
    cpu.regs.a = a;
    cpu.regs.set_bc(bc);
    cpu.regs.set_hl(0x4000);
    cpu.regs.set_de(0x5000);
    cpu.step(&mut ram);
    cpu
}

#[test]
fn e10_02_ldir_repeat_xy() {
    // A + byte = 0x0A: bits 3 and 1 set -> X and Y set. PC high (0x01) would give X = Y = 0.
    let cpu = run_block(0xB0, 0x00, 2, 0x0A);
    assert_eq!(cpu.regs.pc, START, "the instruction loops back");
    assert_eq!(cpu.regs.f & (X | Y), X | Y);
}

#[test]
fn e10_02_ldir_last_iteration() {
    let cpu = run_block(0xB0, 0x00, 1, 0x0A);
    assert_eq!(cpu.regs.pc, START + 2);
    assert_eq!(cpu.regs.f & (X | Y), X | Y);
    let cpu = run_block(0xB0, 0x00, 1, 0x00);
    assert_eq!(cpu.regs.f & (X | Y), 0);
}

#[test]
fn e10_02_cpir_repeat_xy() {
    // A - byte = 0x1A, no half carry: X and Y set from the result.
    let cpu = run_block(0xB1, 0x1A, 2, 0x00);
    assert_eq!(cpu.regs.pc, START, "the instruction loops back");
    assert_eq!(cpu.regs.f & (X | Y), X | Y);
}

#[test]
fn e10_02_diff_still_passes() {
    // Run the differential bench on the ED table in a child cargo process.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = std::process::Command::new(cargo)
        .args(["test", "-p", "puce-z80", "--test", "diff", "e07_07_ed_all"])
        .current_dir(root)
        .output()
        .expect("cannot run cargo");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}
