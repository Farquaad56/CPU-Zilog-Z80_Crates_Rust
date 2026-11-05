//! E13-17: the z80test programs on the minimal ZX Spectrum (long, one test per program).
//!
//! `cargo test --release -p puce-z80-cli --test z80test -- --ignored --nocapture`
//!
//! Skipped when the tape is absent (python docs/tools/fetch_roms.py --optional z80test, then
//! extract the release zip). No ROM is needed: the two ROM routines used are simulated.

use std::path::PathBuf;
use std::process::Command;

fn run_tape(name: &str) {
    let tap = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/z80test/release/z80test-1.2a")
        .join(format!("{}.tap", name));
    if !tap.exists() {
        eprintln!("SKIP: {} not found", tap.display());
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_z80"))
        .arg("z80test")
        .arg(&tap)
        .output()
        .expect("run z80 z80test");
    let text = String::from_utf8_lossy(&out.stdout);
    println!("{}", text);
    assert!(out.status.success(), "{} failed:\n{}", name, text);
    assert!(text.contains("all tests passed."), "{}", text);
}

#[test]
#[ignore]
fn e13_17_z80doc() {
    run_tape("z80doc");
}

#[test]
#[ignore]
fn e13_17_z80docflags() {
    run_tape("z80docflags");
}

#[test]
#[ignore]
fn e13_17_z80flags() {
    run_tape("z80flags");
}

#[test]
#[ignore]
fn e13_17_z80full() {
    run_tape("z80full");
}

#[test]
#[ignore]
fn e13_17_z80ccf() {
    run_tape("z80ccf");
}

#[test]
#[ignore]
fn e13_17_z80memptr() {
    run_tape("z80memptr");
}
