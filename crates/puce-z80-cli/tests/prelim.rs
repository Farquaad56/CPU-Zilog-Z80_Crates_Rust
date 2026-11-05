//! E13-03: PRELIM must report `Preliminary tests complete`.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn e13_03_prelim_passes() {
    let rom = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/prelim.com");
    if !rom.exists() {
        eprintln!("SKIP: run python docs/tools/fetch_roms.py");
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_z80"))
        .arg("cpm")
        .arg(&rom)
        .output()
        .expect("run z80 cpm");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Preliminary tests complete"),
        "PRELIM output:\n{}\nstderr: {}",
        text,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success());
}
