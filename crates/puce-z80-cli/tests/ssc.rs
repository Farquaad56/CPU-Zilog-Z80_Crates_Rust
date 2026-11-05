//! E13-09: SingleStepTests, final state of every instruction (long).
//!
//! `cargo test --release -p puce-z80-cli --test ssc -- --ignored --nocapture`

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore]
fn e13_09_ssc_final_state() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/singlestep");
    if !dir.exists() {
        eprintln!("SKIP: run python docs/tools/fetch_roms.py --optional singlestep");
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_z80"))
        .arg("ssc")
        .arg(&dir)
        .args(["--max-fail", "50"])
        .output()
        .expect("run z80 ssc");
    let text = String::from_utf8_lossy(&out.stdout);
    let tail: Vec<&str> = text.lines().rev().take(3).collect();
    println!("{}", tail.join("\n"));
    assert!(
        out.status.success(),
        "failures outside the baseline:\n{}",
        text
    );
    assert!(text.contains("0 new failures"), "{}", text);
}

/// E13-11: SingleStepTests, final state AND cycles of every instruction (long).
///
/// `cargo test --release -p puce-z80-cli --test ssc -- --ignored --nocapture e13_11`
#[test]
#[ignore]
fn e13_11_ssc_all() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/singlestep");
    if !dir.exists() {
        eprintln!("SKIP: run python docs/tools/fetch_roms.py --optional singlestep");
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_z80"))
        .arg("ssc")
        .arg(&dir)
        .args(["--cycles", "--max-fail", "50"])
        .output()
        .expect("run z80 ssc --cycles");
    let text = String::from_utf8_lossy(&out.stdout);
    let tail: Vec<&str> = text.lines().rev().take(2).collect();
    println!("{}", tail.join("\n"));
    assert!(
        out.status.success(),
        "failures outside the baseline:\n{}",
        text
    );
    assert!(text.contains("0 new failures"), "{}", text);
}
