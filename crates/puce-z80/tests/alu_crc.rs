//! CRC-based exhaustive ALU validation harness.
//!
//! Each ALU primitive is tested by computing its results over the full
//! input space, hashing the output bytes with CRC-32, and comparing
//! against the reference values in docs/specs/data/alu_crc.tsv.

mod common;

use common::crc::{Crc32, crc32};
use common::tsv::alu_crc;

/// Validate a computed CRC and operation count against the reference table.
fn check(name: &str, bytes_crc: Crc32, count: u64) {
    let got_crc = bytes_crc.finish();
    let (exp_count, exp_crc) = alu_crc(name);
    assert_eq!(count, exp_count, "{}: count mismatch", name);
    assert_eq!(
        got_crc, exp_crc,
        "{}: crc mismatch (expected {:08x}, got {:08x})",
        name, exp_crc, got_crc
    );
}

#[test]
fn e04_01_crc_known_vector() {
    assert_eq!(crc32(b"123456789"), 0xCBF43926);
}

#[test]
fn e04_01_crc_empty() {
    assert_eq!(crc32(b""), 0);
}

#[test]
fn e04_01_incremental_equals_oneshot() {
    let data = b"test data for incremental crc";
    let mut inc = Crc32::new();
    for &b in data {
        inc.update(b);
    }
    assert_eq!(inc.finish(), crc32(data));
}

#[test]
fn e04_01_tsv_has_24_rows() {
    let rows = common::tsv::read_tsv("alu_crc.tsv");
    assert_eq!(rows.len(), 24, "expected 24 ALU CRC rows");
}

#[test]
fn e04_01_alu_crc_lookup() {
    let (count, crc) = alu_crc("alu8_add");
    assert_eq!(count, 131072);
    assert_eq!(crc, 0x09ee0e62);
}

#[test]
#[should_panic]
fn e04_01_tsv_missing_panics() {
    alu_crc("nope");
}