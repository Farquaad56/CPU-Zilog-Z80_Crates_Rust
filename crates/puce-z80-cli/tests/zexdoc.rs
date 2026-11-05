//! E13-04: ZEXDOC must print 67 `OK` lines, no `ERROR`, then `Tests complete`.
//!
//! Long (billions of instructions): `cargo test --release -p puce-z80-cli --test zexdoc -- --ignored`.

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore]
fn e13_04_zexdoc_full() {
    let rom = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/zexdoc.cim");
    if !rom.exists() {
        eprintln!("SKIP: run python docs/tools/fetch_roms.py");
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_z80"))
        .args(["suite", "zexdoc"])
        .output()
        .expect("run z80 suite zexdoc");
    let text = String::from_utf8_lossy(&out.stdout);
    let ok = text
        .lines()
        .filter(|l| l.trim_end().ends_with("OK"))
        .count();
    let err = text.lines().filter(|l| l.contains("ERROR")).count();
    println!("{}", text);
    assert_eq!(err, 0, "ERROR lines in ZEXDOC output");
    assert_eq!(ok, 67, "expected 67 OK lines");
    assert!(text.contains("Tests complete"));
    assert!(out.status.success());
}
