//! E03-14: basic checks of the pin trace tool.

mod common;

use common::trace::{fmt_trace, trace_instruction};

fn lines(bytes: &[u8]) -> Vec<String> {
    let t = trace_instruction(bytes, |_| {});
    fmt_trace(&t).lines().map(str::to_string).collect()
}

#[test]
fn e03_14_nop_trace_len() {
    assert_eq!(lines(&[0x00]).len(), 4);
}

#[test]
fn e03_14_nop_trace_fetch() {
    let l = lines(&[0x00]);
    assert!(l[1].contains("M1 MREQ RD"), "{}", l[1]);
}

#[test]
fn e03_14_refresh_line() {
    let l = lines(&[0x00]);
    assert!(l[2].contains("RFSH"), "{}", l[2]);
}

#[test]
fn e03_14_fmt_has_t() {
    let t = trace_instruction(&[0x00], |_| {});
    assert!(fmt_trace(&t).starts_with("T1"));
}

#[test]
fn e03_14_ld_rp_nn_trace() {
    assert_eq!(lines(&[0x01, 0x34, 0x12]).len(), 10);
}
