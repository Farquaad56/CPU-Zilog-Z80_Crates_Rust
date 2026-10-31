//! E02-05: Exhaustive decode tests across all 1280 opcode combinations.
//!
//! Reads the 5 TSV spec files and verifies that decode() produces correct
//! results for every row, including prefix handling and plan token counts.

use puce_z80::__internal::{decode, Decoded, Table};
use std::fs;

fn tsv_path(name: &str) -> String {
    format!(
        "{}/../../docs/specs/data/{}.tsv",
        env!("CARGO_MANIFEST_DIR"),
        name
    )
}

fn read_tsv(name: &str) -> Vec<Vec<String>> {
    let path = tsv_path(name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
    let mut rows = Vec::new();
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        rows.push(line.split('\t').map(|s| s.to_string()).collect());
    }
    // Skip header
    rows.into_iter().skip(1).collect()
}

fn count_plan_tokens(plan_str: &str) -> usize {
    if plan_str == "-" {
        return 0;
    }
    plan_str.split_whitespace().count()
}

/// Verify that decode produces the expected result for a main table row.
fn check_main_row(row: &[String]) {
    let opcode = u8::from_str_radix(&row[0], 16).unwrap();
    let mnemonic = &row[1];
    let plan_str = &row[3];
    let expected_tokens = count_plan_tokens(plan_str);

    match decode(Table::Main, opcode) {
        Decoded::Prefix(t) => {
            // Prefix rows: CB->Cb, ED->Ed, DD/FD->DdFd
            assert!(
                mnemonic == "prefix",
                "expected prefix mnemonic for 0x{:02X}",
                opcode
            );
            match opcode {
                0xCB => assert_eq!(t, Table::Cb),
                0xED => assert_eq!(t, Table::Ed),
                0xDD | 0xFD => assert_eq!(t, Table::DdFd),
                _ => panic!("unexpected prefix byte 0x{:02X}", opcode),
            }
        }
        Decoded::Plan(t, plan) => {
            assert_eq!(t, Table::Main);
            assert_eq!(
                plan.len(),
                expected_tokens,
                "token count mismatch for 0x{:02X} {}",
                opcode,
                mnemonic
            );
        }
        other => panic!(
            "unexpected decode result for main 0x{:02X}: {:?}",
            opcode, other
        ),
    }
}

/// Verify that decode produces the expected result for a cb/ed table row.
fn check_cb_ed_row(table: Table, name: &str, row: &[String]) {
    let opcode = u8::from_str_radix(&row[0], 16).unwrap();
    let mnemonic = &row[1];
    let plan_str = &row[3];
    let expected_tokens = count_plan_tokens(plan_str);

    match decode(table, opcode) {
        Decoded::Plan(t, plan) => {
            assert_eq!(t, table);
            assert_eq!(
                plan.len(),
                expected_tokens,
                "token count mismatch for {} 0x{:02X} {}",
                name,
                opcode,
                mnemonic
            );
        }
        other => panic!(
            "unexpected decode result for {} 0x{:02X}: {:?}",
            name, opcode, other
        ),
    }
}

/// Verify that decode produces the expected result for a ddfd table row.
fn check_ddfd_row(row: &[String]) {
    let opcode = u8::from_str_radix(&row[0], 16).unwrap();
    let mnemonic = &row[1];
    let plan_str = &row[3];
    let expected_tokens = count_plan_tokens(plan_str);

    match decode(Table::DdFd, opcode) {
        Decoded::IndexCb => {
            assert!(
                mnemonic == "-",
                "expected '-' mnemonic for IndexCb at 0x{:02X}",
                opcode
            );
            assert_eq!(opcode, 0xCB);
        }
        Decoded::Restart(b) => {
            assert!(
                mnemonic == "-",
                "expected '-' mnemonic for Restart at 0x{:02X}",
                opcode
            );
            assert_eq!(b, opcode);
        }
        Decoded::Plan(t, plan) => {
            assert_eq!(t, Table::DdFd);
            assert_eq!(
                plan.len(),
                expected_tokens,
                "token count mismatch for ddfd 0x{:02X} {}",
                opcode,
                mnemonic
            );
        }
        other => panic!(
            "unexpected decode result for ddfd 0x{:02X}: {:?}",
            opcode, other
        ),
    }
}

/// Verify that plan_of produces the correct token count for a ddcb row.
fn check_ddcb_row(row: &[String]) {
    let opcode = u8::from_str_radix(&row[0], 16).unwrap();
    let mnemonic = &row[1];
    let plan_str = &row[3];
    let expected_tokens = count_plan_tokens(plan_str);

    let plan = puce_z80::__internal::plan_of(Table::DdCb, opcode);
    assert_eq!(
        plan.len(),
        expected_tokens,
        "token count mismatch for ddcb 0x{:02X} {}",
        opcode,
        mnemonic
    );
}

#[test]
fn e02_05_all_rows_main() {
    let rows = read_tsv("main");
    assert_eq!(rows.len(), 256, "expected 256 main rows");
    for row in &rows {
        check_main_row(row);
    }
}

#[test]
fn e02_05_all_rows_cb_ed() {
    let cb_rows = read_tsv("cb");
    assert_eq!(cb_rows.len(), 256, "expected 256 cb rows");
    for row in &cb_rows {
        check_cb_ed_row(Table::Cb, "cb", row);
    }

    let ed_rows = read_tsv("ed");
    assert_eq!(ed_rows.len(), 256, "expected 256 ed rows");
    for row in &ed_rows {
        check_cb_ed_row(Table::Ed, "ed", row);
    }
}

#[test]
fn e02_05_all_rows_ddfd_ddcb() {
    let ddfd_rows = read_tsv("ddfd");
    assert_eq!(ddfd_rows.len(), 256, "expected 256 ddfd rows");
    for row in &ddfd_rows {
        check_ddfd_row(row);
    }

    let ddcb_rows = read_tsv("ddcb");
    assert_eq!(ddcb_rows.len(), 256, "expected 256 ddcb rows");
    for row in &ddcb_rows {
        check_ddcb_row(row);
    }
}

#[test]
fn e02_05_token_counts() {
    // Verify token counts across all tables
    let main_rows = read_tsv("main");
    for row in &main_rows {
        if row[1] != "prefix" {
            check_main_row(row);
        }
    }

    let cb_rows = read_tsv("cb");
    for row in &cb_rows {
        check_cb_ed_row(Table::Cb, "cb", row);
    }

    let ed_rows = read_tsv("ed");
    for row in &ed_rows {
        check_cb_ed_row(Table::Ed, "ed", row);
    }

    let ddfd_rows = read_tsv("ddfd");
    for row in &ddfd_rows {
        if row[1] != "-" {
            check_ddfd_row(row);
        }
    }

    let ddcb_rows = read_tsv("ddcb");
    for row in &ddcb_rows {
        check_ddcb_row(row);
    }
}
