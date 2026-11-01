//! E03-13: exhaustive timing test with empty semantics.
//!
//! With the semantic stub empty, `cond` stays false, so every instruction must
//! last exactly `t_min` of its table row. The expected values come from the
//! TSV files in `docs/specs/data/`.
//!
//! Run with: cargo test -p puce-z80 --features stub-sem e03_13

#![cfg(feature = "stub-sem")]

use puce_z80::{Cpu, SimpleRam};

const PC_START: u16 = 0x1000;
const SP_START: u16 = 0x8000;

/// One row of a specification TSV file.
struct Row {
    op: u8,
    mnemonic: String,
    t_min: u32,
}

/// Read `docs/specs/data/<name>.tsv`; rows without a plan (prefix bytes) are skipped.
fn load(name: &str) -> Vec<Row> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/specs/data/");
    let text = std::fs::read_to_string(format!("{path}{name}.tsv")).expect("read TSV");
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 || f[3] == "-" || f[2] == "prefix" {
            continue;
        }
        rows.push(Row {
            op: u8::from_str_radix(f[0], 16).expect("opcode"),
            mnemonic: f[1].to_string(),
            t_min: f[4].parse().expect("t_min"),
        });
    }
    rows
}

/// Execute one instruction made of `bytes` at PC_START and return its T-states.
fn run(ram: &mut SimpleRam, bytes: &[u8]) -> u32 {
    let base = PC_START as usize;
    for i in 0..8 {
        ram.mem[base + i] = 0;
    }
    ram.mem[base..base + bytes.len()].copy_from_slice(bytes);
    let mut cpu = Cpu::new();
    cpu.regs.pc = PC_START;
    cpu.regs.sp = SP_START;
    cpu.step(ram)
}

/// Check every row of table `name`; `build` makes the instruction bytes for an opcode.
fn check(name: &str, expected_rows: usize, build: &dyn Fn(u8) -> Vec<u8>) {
    let rows = load(name);
    assert_eq!(rows.len(), expected_rows, "row count of {name}");
    let mut ram = SimpleRam::new();
    let mut errors = Vec::new();
    for r in &rows {
        let bytes = build(r.op);
        let got = run(&mut ram, &bytes);
        if got != r.t_min {
            errors.push(format!(
                "table {name} op {:02X} {}: expected {} got {} (bytes {:02X?})",
                r.op, r.mnemonic, r.t_min, got, bytes
            ));
        }
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

#[test]
fn e03_13_main() {
    check("main", 252, &|op| vec![op, 0, 0, 0]);
}

#[test]
fn e03_13_cb() {
    check("cb", 256, &|op| vec![0xCB, op, 0, 0]);
}

#[test]
fn e03_13_ed() {
    check("ed", 256, &|op| vec![0xED, op, 0, 0]);
}

#[test]
fn e03_13_ddfd() {
    check("ddfd", 252, &|op| vec![0xDD, op, 0, 0, 0]);
    check("ddfd", 252, &|op| vec![0xFD, op, 0, 0, 0]);
}

#[test]
fn e03_13_ddcb() {
    check("ddcb", 256, &|op| vec![0xDD, 0xCB, 0x00, op]);
    check("ddcb", 256, &|op| vec![0xFD, 0xCB, 0x00, op]);
}
