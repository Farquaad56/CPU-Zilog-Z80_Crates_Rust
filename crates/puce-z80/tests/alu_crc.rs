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

use puce_z80::alu::alu8;

/// Run ALU operation across all inputs and compute CRC of outputs.
fn alu8_crc(op: u8) -> (u64, u32) {
    let mut crc = Crc32::new();
    let mut count = 0u64;
    for a in 0u8..=255 {
        for b in 0u8..=255 {
            for c in 0u8..=1 {
                let (a_out, f_out) = alu8(op, a, b, c);
                crc.update(a_out);
                crc.update(f_out);
                count += 1;
            }
        }
    }
    (count, crc.finish())
}

#[test]
fn e04_02_alu8_add() {
    let (count, crc) = alu8_crc(0);
    let (exp_count, exp_crc) = alu_crc("alu8_add");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "ADD CRC mismatch");
}

#[test]
fn e04_02_alu8_adc() {
    let (count, crc) = alu8_crc(1);
    let (exp_count, exp_crc) = alu_crc("alu8_adc");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "ADC CRC mismatch");
}

#[test]
fn e04_02_alu8_sub() {
    let (count, crc) = alu8_crc(2);
    let (exp_count, exp_crc) = alu_crc("alu8_sub");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "SUB CRC mismatch");
}

#[test]
fn e04_02_alu8_sbc() {
    let (count, crc) = alu8_crc(3);
    let (exp_count, exp_crc) = alu_crc("alu8_sbc");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "SBC CRC mismatch");
}

#[test]
fn e04_02_alu8_cp() {
    let (count, crc) = alu8_crc(7);
    let (exp_count, exp_crc) = alu_crc("alu8_cp");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "CP CRC mismatch");
}

#[test]
fn e04_02_parity_table() {
    use puce_z80::alu::parity;
    assert_eq!(parity(0), 4);
    assert_eq!(parity(1), 0);
    assert_eq!(parity(3), 4);
    assert_eq!(parity(0xFF), 4);
    assert_eq!(parity(0x7F), 0);
}

#[test]
fn e04_02_add_example() {
    let (a, f) = alu8(0, 0x7F, 1, 0);
    assert_eq!(a, 0x80);
    assert_eq!(f, 0x94);
}

#[test]
fn e04_03_alu8_and() {
    let (count, crc) = alu8_crc(4);
    let (exp_count, exp_crc) = alu_crc("alu8_and");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "AND CRC mismatch");
}

#[test]
fn e04_03_alu8_xor() {
    let (count, crc) = alu8_crc(5);
    let (exp_count, exp_crc) = alu_crc("alu8_xor");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "XOR CRC mismatch");
}

#[test]
fn e04_03_alu8_or() {
    let (count, crc) = alu8_crc(6);
    let (exp_count, exp_crc) = alu_crc("alu8_or");
    assert_eq!(count, exp_count);
    assert_eq!(crc, exp_crc, "OR CRC mismatch");
}

#[test]
fn e04_03_and_h_set() {
    let (_, f) = alu8(4, 0xFF, 0x0F, 0);
    assert_eq!(f & 0x10, 0x10, "AND must set H flag");
}

#[test]
fn e04_03_xor_zero() {
    let (a, f) = alu8(5, 0xAA, 0xAA, 0xFF);
    assert_eq!(a, 0);
    assert_eq!(f, 0x44);
}