//! E11-05: the disassembled length must equal the number of bytes the engine really reads.
#![cfg(feature = "disasm")]

use puce_z80::__internal::{plan_of, Table};
use puce_z80::disasm::disassemble;

/// Number of instruction bytes a plan reads: every M1 fetch and every PC-incrementing token.
fn engine_len(table: Table, op: u8) -> Option<usize> {
    let plan = plan_of(table, op);
    if plan.is_empty() {
        return None;
    }
    let n = plan
        .iter()
        .filter(|t| {
            let kind = format!("{:?}", t.kind);
            let addr = format!("{:?}", t.addr);
            kind == "F" || addr == "PcInc"
        })
        .count();
    Some(n)
}

/// Disassemble `bytes` (padded with operand bytes) at address 0x8000.
fn dis(bytes: &[u8]) -> (usize, String) {
    let mut buf = [0x12u8, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    buf[..bytes.len()].copy_from_slice(bytes);
    let mut read = |a: u16| {
        buf.get(a.wrapping_sub(0x8000) as usize)
            .copied()
            .unwrap_or(0)
    };
    let i = disassemble(0x8000, &mut read);
    (i.len as usize, i.text.as_str().to_string())
}

fn printable_nonempty(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

#[test]
fn e11_05_main_len() {
    let mut checked = 0;
    for op in 0..=255u8 {
        let Some(n) = engine_len(Table::Main, op) else {
            continue;
        };
        let (len, text) = dis(&[op]);
        assert_eq!(len, n, "main {op:02X} '{text}'");
        checked += 1;
    }
    assert_eq!(checked, 252, "256 opcodes minus the 4 prefixes");
}

#[test]
fn e11_05_cb_ed_len() {
    let mut checked = 0;
    for (table, prefix) in [(Table::Cb, 0xCBu8), (Table::Ed, 0xED)] {
        for op in 0..=255u8 {
            let n = engine_len(table, op).expect("CB and ED plans are never empty");
            let (len, text) = dis(&[prefix, op]);
            assert_eq!(len, n, "{prefix:02X} {op:02X} '{text}'");
            checked += 1;
        }
    }
    assert_eq!(checked, 512);
}

#[test]
fn e11_05_ddfd_len() {
    for prefix in [0xDDu8, 0xFD] {
        for op in 0..=255u8 {
            let Some(n) = engine_len(Table::DdFd, op) else {
                continue;
            };
            let (len, text) = dis(&[prefix, op]);
            if text == "DD" || text == "FD" {
                // "prefix ignored": the prefix is an instruction of its own (4 T, 1 byte).
                assert_eq!(len, 1, "{prefix:02X} {op:02X}");
            } else {
                assert_eq!(len, n, "{prefix:02X} {op:02X} '{text}'");
            }
        }
    }
}

#[test]
fn e11_05_ddcb_len() {
    for prefix in [0xDDu8, 0xFD] {
        for op in 0..=255u8 {
            let (len, text) = dis(&[prefix, 0xCB, 0x05, op]);
            assert_eq!(len, 4, "{prefix:02X} CB 05 {op:02X} '{text}'");
            assert!(engine_len(Table::DdCb, op).is_some());
        }
    }
}

#[test]
fn e11_05_text_ascii() {
    for op in 0..=255u8 {
        assert!(printable_nonempty(&dis(&[op]).1), "main {op:02X}");
        assert!(printable_nonempty(&dis(&[0xCB, op]).1), "CB {op:02X}");
        assert!(printable_nonempty(&dis(&[0xED, op]).1), "ED {op:02X}");
        for prefix in [0xDDu8, 0xFD] {
            assert!(
                printable_nonempty(&dis(&[prefix, op]).1),
                "{prefix:02X} {op:02X}"
            );
            assert!(
                printable_nonempty(&dis(&[prefix, 0xCB, 0x05, op]).1),
                "{prefix:02X} CB 05 {op:02X}"
            );
        }
    }
}
