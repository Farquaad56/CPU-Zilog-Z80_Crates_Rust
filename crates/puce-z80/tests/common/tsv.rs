//! TSV reader for docs/specs/data/ reference tables.

use std::env;
use std::fs;

/// Read a TSV file from docs/specs/data/, skip the header row, split on tabs.
pub fn read_tsv(rel: &str) -> Vec<Vec<String>> {
    let base = format!("{}/../../docs/specs/data/", env!("CARGO_MANIFEST_DIR"));
    let path = format!("{}{}", base, rel);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read {}: {}", path, e);
    });
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue; // skip header
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        rows.push(line.split('\t').map(|s| s.to_string()).collect());
    }
    rows
}

/// Look up an ALU CRC row by name; returns (count, crc32).
pub fn alu_crc(name: &str) -> (u64, u32) {
    let rows = read_tsv("alu_crc.tsv");
    for row in rows {
        if row[0] == name {
            let count: u64 = row[1].parse().unwrap_or_else(|_| {
                panic!("bad count '{}' for {}", row[1], name);
            });
            let crc_hex = &row[2];
            let crc = u32::from_str_radix(crc_hex, 16).unwrap_or_else(|_| {
                panic!("bad crc '{}' for {}", crc_hex, name);
            });
            return (count, crc);
        }
    }
    panic!("alu_crc: '{}' not found in alu_crc.tsv", name);
}
