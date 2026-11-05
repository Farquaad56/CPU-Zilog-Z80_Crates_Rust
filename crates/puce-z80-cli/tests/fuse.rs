//! E13-14: the whole FUSE suite (`tests/roms/fuse`), skipped when the files are absent.
//!
//! `cargo test --release -p puce-z80-cli e13_14_ -- --nocapture`

use puce_z80_machines::fuse::{check_all, compare, parse_expected, parse_in, run_fuse, KNOWN};
use std::path::PathBuf;

fn load() -> Option<(
    Vec<puce_z80_machines::fuse::FuseTest>,
    Vec<puce_z80_machines::fuse::FuseExpected>,
)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/fuse");
    let tin = std::fs::read_to_string(dir.join("tests.in")).ok()?;
    let tex = std::fs::read_to_string(dir.join("tests.expected")).ok()?;
    Some((parse_in(&tin).unwrap(), parse_expected(&tex).unwrap()))
}

#[test]
fn e13_14_fuse_all() {
    let Some((tests, expected)) = load() else {
        eprintln!("SKIP: run python docs/tools/fetch_roms.py (fuse files) first");
        return;
    };
    assert_eq!(tests.len(), 1356);
    let fails = check_all(&tests, &expected);
    for (n, d) in &fails {
        eprintln!("FAIL {}: {:?}", n, d);
    }
    assert!(fails.is_empty(), "{} FUSE tests fail", fails.len());
}

/// The documented exceptions must still be real disagreements (otherwise remove them).
#[test]
fn e13_14_known_exceptions_still_fail() {
    let Some((tests, expected)) = load() else {
        eprintln!("SKIP: fuse files absent");
        return;
    };
    for (name, why) in KNOWN {
        let t = tests.iter().find(|t| t.name == *name).expect(name);
        let e = expected.iter().find(|e| e.name == *name).expect(name);
        assert!(
            !compare(&run_fuse(t), e).is_empty(),
            "{} now passes: {}",
            name,
            why
        );
    }
}
