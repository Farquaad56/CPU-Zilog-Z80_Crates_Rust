//! E13-19: timing ROMs (tests/roms/timing, gitignored) on the minimal ZX Spectrum.
//!
//! - z80_im1_vs_jr: runs romless (IM 1, EI, stub handlers); passes when BC = HL = 0 on return.
//! - Woodmass, Block Flags and the 48K timing test need the Spectrum ROM (tests/roms/zx48.rom):
//!   SKIP when it is absent. The verdict of the 48K timing test is only visible on screen:
//!   UNAVAILABLE (no video model).
//!
//! `cargo test --release -p puce-z80-cli --test timing_roms -- --ignored --nocapture`

use puce_z80_machines::zx::{parse_tap, ZxMachine};
use std::path::PathBuf;

fn roms() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms")
}

/// Load the CODE block of a tape at its start address; returns (machine, start).
fn load_code_tape(m: &mut ZxMachine, tap: &[u8]) -> u16 {
    let blocks = parse_tap(tap).expect("tap");
    let hdr = blocks
        .iter()
        .position(|b| b.flag == 0 && b.data.first() == Some(&3) && b.data.len() >= 15)
        .expect("CODE header");
    let start = u16::from(blocks[hdr].data[13]) | (u16::from(blocks[hdr].data[14]) << 8);
    m.load_code(start, &blocks[hdr + 1].data);
    start
}

fn im1_vs_jr() {
    let path = roms().join("timing/z80_im1_vs_jr.tap");
    if !path.exists() {
        eprintln!("SKIP: {} not found", path.display());
        return;
    }
    let tap = std::fs::read(&path).unwrap();
    let mut m = ZxMachine::new_romless();
    m.load_code(0x0030, &[0x21, 0x55, 0x55, 0xC9]); // LD HL,5555h; RET (trap)
    m.load_code(0x0038, &[0xFB, 0xC9]); // EI; RET
    let start = load_code_tape(&mut m, &tap);
    m.cpu.regs.pc = start;
    m.cpu.regs.sp = 0x7FEE;
    m.load_code(0x7FEE, &[0x00, 0x00]);
    m.cpu.regs.im = 1;
    m.cpu.regs.iff1 = true;
    m.cpu.regs.iff2 = true;
    // Step by hand: run_until_pc stops at HALT, which the frame interrupt must wake.
    let limit = m.tstates + 300 * 69888;
    while m.tstates < limit && m.cpu.regs.pc != 0x0000 {
        m.step();
    }
    let reached = m.cpu.regs.pc == 0x0000;
    let (bc, hl) = (
        u16::from(m.cpu.regs.b) << 8 | u16::from(m.cpu.regs.c),
        u16::from(m.cpu.regs.h) << 8 | u16::from(m.cpu.regs.l),
    );
    println!(
        "im1_vs_jr: returned={} BC={:04X} HL={:04X}",
        reached, bc, hl
    );
    assert!(reached, "did not return (PC={:04X})", m.cpu.regs.pc);
    assert_eq!((bc, hl), (0, 0), "IM 1 interrupt taken at the wrong time");
}

fn rom_dependent_roms() {
    let rom = roms().join("zx48.rom");
    for name in [
        "woodmass_z80tests.tap",
        "z80_block_flags_v5.0beta.tap",
        "timing_tests_48k_v1.0.z80",
    ] {
        if !rom.exists() {
            eprintln!("SKIP: {} (needs {})", name, rom.display());
        } else if name.starts_with("timing_tests") {
            eprintln!("UNAVAILABLE: {} (verdict is on screen only)", name);
        } else {
            eprintln!("INFO: {} needs interactive keys; not automated", name);
        }
    }
}

#[test]
#[ignore]
fn e13_19_timing_roms_all() {
    im1_vs_jr();
    rom_dependent_roms();
}
