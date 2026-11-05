//! E13-05: ZEXALL (undocumented flags X/Y included) must print 67 `OK` lines, no `ERROR`.
//!
//! Long (billions of instructions): `cargo test --release -p puce-z80-cli --test zexall -- --ignored`.

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore]
fn e13_05_zexall_full() {
    let rom = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/zexall.cim");
    if !rom.exists() {
        eprintln!("SKIP: run python docs/tools/fetch_roms.py");
        return;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_z80"))
        .args(["suite", "zexall"])
        .output()
        .expect("run z80 suite zexall");
    let text = String::from_utf8_lossy(&out.stdout);
    let ok = text
        .lines()
        .filter(|l| l.trim_end().ends_with("OK"))
        .count();
    let err = text.lines().filter(|l| l.contains("ERROR")).count();
    println!("{}", text);
    assert_eq!(err, 0, "ERROR lines in ZEXALL output");
    assert_eq!(ok, 67, "expected 67 OK lines");
    assert!(text.contains("Tests complete"));
    assert!(out.status.success());
}
