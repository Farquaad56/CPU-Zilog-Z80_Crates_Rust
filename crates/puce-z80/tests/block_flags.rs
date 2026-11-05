//! E10-02 / E13-17: X/Y flags of repeated block instructions.
//!
//! When a block instruction loops back, bits 3 and 5 of F come from the high byte of the
//! rewound PC (real NMOS silicon: z80test and SingleStepTests, decision D-026). On the last
//! iteration they come from the iteration result as usual.

use puce_z80::{Bus, Cpu, SimpleRam};

const X: u8 = 0x08;
const Y: u8 = 0x20;
const START: u16 = 0x0100;

/// Run one `ED op` at 0x0100 with A, BC, HL set and the byte at HL given.
fn run_block(op: u8, a: u8, bc: u16, byte: u8) -> Cpu {
    run_block_at(START, op, a, bc, byte)
}

/// Same, with the instruction placed at `start`.
fn run_block_at(start: u16, op: u8, a: u8, bc: u16, byte: u8) -> Cpu {
    let mut ram = SimpleRam::new();
    ram.write(start, 0xED);
    ram.write(start + 1, op);
    ram.write(0x4000, byte);
    let mut cpu = Cpu::new();
    cpu.regs.pc = start;
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
    // A + byte = 0x0A would give X = Y = 1, but the loop copies PC high (0x01): X = Y = 0.
    let cpu = run_block(0xB0, 0x00, 2, 0x0A);
    assert_eq!(cpu.regs.pc, START, "the instruction loops back");
    assert_eq!(cpu.regs.f & (X | Y), 0);
    // PC high = 0x28 sets both bits whatever the data.
    let cpu = run_block_at(0x2800, 0xB0, 0x00, 2, 0x00);
    assert_eq!(cpu.regs.pc, 0x2800);
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
    // A - byte = 0x1A would give X = Y = 1, but the loop copies PC high (0x01): X = Y = 0.
    let cpu = run_block(0xB1, 0x1A, 2, 0x00);
    assert_eq!(cpu.regs.pc, START, "the instruction loops back");
    assert_eq!(cpu.regs.f & (X | Y), 0);
    let cpu = run_block_at(0x2800, 0xB1, 0x1A, 2, 0x00);
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
