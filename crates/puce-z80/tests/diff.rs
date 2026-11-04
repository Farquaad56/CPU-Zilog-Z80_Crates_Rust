//! Differential test harness: validate CPU against 7632 reference vectors.
//!
//! Each vector in docs/specs/data/diff.tsv specifies only the FINAL state;
//! initial state is regenerated from a seed using xorshift32 PRNG (see spec).
//! The test executes one instruction via Cpu::step and compares all outputs.

mod common;

use common::diffbus::{hash, seed, TestBus, Xs32};
use puce_z80::cpu::Cpu;

/// Parse a hex byte from string.
fn parse_hex8(s: &str) -> u8 {
    u8::from_str_radix(s.trim(), 16).unwrap_or_else(|_| panic!("bad hex8 '{}'", s))
}

/// Parse a hex word from string.
fn parse_hex16(s: &str) -> u16 {
    u16::from_str_radix(s.trim(), 16).unwrap_or_else(|_| panic!("bad hex16 '{}'", s))
}

/// Parse the writes column into a list of (addr, value) pairs.
fn parse_writes(s: &str) -> Vec<(u16, u8)> {
    let s = s.trim();
    if s == "-" {
        return Vec::new();
    }
    let mut result = Vec::new();
    for pair in s.split(',') {
        let parts: Vec<&str> = pair.split('=').collect();
        if parts.len() == 2 {
            let addr = parse_hex16(parts[0]);
            let val = parse_hex8(parts[1]);
            result.push((addr, val));
        }
    }
    result
}

/// Parse the port_out column into a list of (port, value) pairs.
fn parse_port_out(s: &str) -> Vec<(u16, u8)> {
    let s = s.trim();
    if s == "-" {
        return Vec::new();
    }
    let mut result = Vec::new();
    for pair in s.split(',') {
        let parts: Vec<&str> = pair.split('=').collect();
        if parts.len() == 2 {
            let port = parse_hex16(parts[0]);
            let val = parse_hex8(parts[1]);
            result.push((port, val));
        }
    }
    result
}

/// Parse the port_in column into a list of ports.
fn parse_port_in(s: &str) -> Vec<u16> {
    let s = s.trim();
    if s == "-" {
        return Vec::new();
    }
    let mut result = Vec::new();
    for p in s.split(',') {
        result.push(parse_hex16(p));
    }
    result
}

/// Reconstruct initial CPU state from seed and execute one instruction.
fn run_vector(table: &str, opcode: u8, vec: u8) -> Result<(Cpu, TestBus), String> {
    let table_id = match table {
        "main" => 0u32,
        "cb" => 1u32,
        "ed" => 2u32,
        "ddfd" => 3u32,
        "ddcb" => 4u32,
        _ => return Err(format!("unknown table '{}'", table)),
    };

    let s = seed(table_id, opcode, vec);
    let mut rng = Xs32::new(s);

    // Step 1: Draw all registers (main then alternate)
    let a = rng.n8();
    let f = rng.n8();
    let b = rng.n8();
    let c = rng.n8();
    let d = rng.n8();
    let e = rng.n8();
    let h = rng.n8();
    let l = rng.n8();
    let a2 = rng.n8();
    let f2 = rng.n8();
    let b2 = rng.n8();
    let c2 = rng.n8();
    let d2 = rng.n8();
    let e2 = rng.n8();
    let h2 = rng.n8();
    let l2 = rng.n8();

    // Step 2: Draw index registers, SP, PC
    let ix = rng.n16();
    let iy = rng.n16();
    let sp = 0x4000 + (rng.n16() & 0x3FFF);
    let pc = 0x8000 + (rng.n16() & 0xFF);

    // Step 3: Draw I, R, flags for interrupt state
    let i = rng.n8();
    let r = rng.n8();
    let fl = rng.n8();
    let iff1 = (fl & 1) != 0;
    let iff2 = ((fl >> 1) & 1) != 0;
    let im = (fl >> 2) % 3;

    // Step 4: Initialize memory with hash values, then write code bytes
    let mut bus = TestBus::new(s);

    // Write instruction bytes at PC based on table type
    match table {
        "main" => {
            bus.mem[pc as usize] = opcode;
        }
        "cb" => {
            bus.mem[pc as usize] = 0xCB;
            bus.mem[(pc + 1) as usize] = opcode;
        }
        "ed" => {
            bus.mem[pc as usize] = 0xED;
            bus.mem[(pc + 1) as usize] = opcode;
        }
        "ddfd" => {
            // Odd vec uses FD (IY), even uses DD (IX)
            let pre = if !vec.is_multiple_of(2) { 0xFD } else { 0xDD };
            bus.mem[pc as usize] = pre;
            bus.mem[(pc + 1) as usize] = opcode;
        }
        "ddcb" => {
            // Odd vec uses FD (IY), even uses DD (IX)
            let pre = if !vec.is_multiple_of(2) { 0xFD } else { 0xDD };
            bus.mem[pc as usize] = pre;
            bus.mem[(pc + 1) as usize] = 0xCB;
            // Third byte is random displacement from PRNG
            let disp = rng.n8();
            bus.mem[(pc + 2) as usize] = disp;
            bus.mem[(pc + 3) as usize] = opcode;
        }
        _ => {}
    }

    // Build CPU with initial state
    let mut cpu = Cpu::new();
    cpu.regs.a = a;
    cpu.regs.f = f;
    cpu.regs.b = b;
    cpu.regs.c = c;
    cpu.regs.d = d;
    cpu.regs.e = e;
    cpu.regs.h = h;
    cpu.regs.l = l;
    cpu.regs.a2 = a2;
    cpu.regs.f2 = f2;
    cpu.regs.b2 = b2;
    cpu.regs.c2 = c2;
    cpu.regs.d2 = d2;
    cpu.regs.e2 = e2;
    cpu.regs.h2 = h2;
    cpu.regs.l2 = l2;
    cpu.regs.ix = ix;
    cpu.regs.iy = iy;
    cpu.regs.sp = sp;
    cpu.regs.pc = pc;
    cpu.regs.i = i;
    cpu.regs.r = r;
    cpu.regs.iff1 = iff1;
    cpu.regs.iff2 = iff2;
    cpu.regs.im = im;

    // Execute one instruction
    let _tstates = cpu.step(&mut bus);

    Ok((cpu, bus))
}

/// Compare actual vs expected state for a single vector.
fn compare_vector(
    table: &str,
    opcode: u8,
    _vec: u8,
    row: &[String],
    cpu: &Cpu,
    bus: &TestBus,
) -> Result<(), String> {
    // Expected values from TSV columns (after t):
    // 4=A 5=F 6=B 7=C 8=D 9=E 10=H 11=L
    // 12=A2 13=F2 14=B2 15=C2 16=D2 17=E2 18=H2 19=L2
    // 20=IX 21=IY 22=SP 23=PC
    // 24=I 25=R 26=IFF1 27=IFF2 28=IM 29=HALTED
    // 30=writes 31=port_out 32=port_in

    let exp_a = parse_hex8(&row[4]);
    let exp_f = parse_hex8(&row[5]);
    let exp_b = parse_hex8(&row[6]);
    let exp_c = parse_hex8(&row[7]);
    let exp_d = parse_hex8(&row[8]);
    let exp_e = parse_hex8(&row[9]);
    let exp_h = parse_hex8(&row[10]);
    let exp_l = parse_hex8(&row[11]);

    if cpu.regs.a != exp_a {
        return Err(format!("A: expected {:02X}, got {:02X}", exp_a, cpu.regs.a));
    }

    // Special case for SCF (0x37) and CCF (0x3F): compare F & 0xD7 only
    if table == "main" && (opcode == 0x37 || opcode == 0x3F) {
        if cpu.regs.f & 0xD7 != exp_f & 0xD7 {
            return Err(format!(
                "F&0xD7: expected {:02X}, got {:02X}",
                exp_f & 0xD7,
                cpu.regs.f & 0xD7
            ));
        }
    } else if cpu.regs.f != exp_f {
        return Err(format!("F: expected {:02X}, got {:02X}", exp_f, cpu.regs.f));
    }

    if cpu.regs.b != exp_b {
        return Err(format!("B: expected {:02X}, got {:02X}", exp_b, cpu.regs.b));
    }
    if cpu.regs.c != exp_c {
        return Err(format!("C: expected {:02X}, got {:02X}", exp_c, cpu.regs.c));
    }
    if cpu.regs.d != exp_d {
        return Err(format!("D: expected {:02X}, got {:02X}", exp_d, cpu.regs.d));
    }
    if cpu.regs.e != exp_e {
        return Err(format!("E: expected {:02X}, got {:02X}", exp_e, cpu.regs.e));
    }
    if cpu.regs.h != exp_h {
        return Err(format!("H: expected {:02X}, got {:02X}", exp_h, cpu.regs.h));
    }
    if cpu.regs.l != exp_l {
        return Err(format!("L: expected {:02X}, got {:02X}", exp_l, cpu.regs.l));
    }

    // Alternate registers
    let exp_a2 = parse_hex8(&row[12]);
    let exp_f2 = parse_hex8(&row[13]);
    let exp_b2 = parse_hex8(&row[14]);
    let exp_c2 = parse_hex8(&row[15]);
    let exp_d2 = parse_hex8(&row[16]);
    let exp_e2 = parse_hex8(&row[17]);
    let exp_h2 = parse_hex8(&row[18]);
    let exp_l2 = parse_hex8(&row[19]);

    if cpu.regs.a2 != exp_a2 {
        return Err(format!(
            "A2: expected {:02X}, got {:02X}",
            exp_a2, cpu.regs.a2
        ));
    }
    if cpu.regs.f2 != exp_f2 {
        return Err(format!(
            "F2: expected {:02X}, got {:02X}",
            exp_f2, cpu.regs.f2
        ));
    }
    if cpu.regs.b2 != exp_b2 {
        return Err(format!(
            "B2: expected {:02X}, got {:02X}",
            exp_b2, cpu.regs.b2
        ));
    }
    if cpu.regs.c2 != exp_c2 {
        return Err(format!(
            "C2: expected {:02X}, got {:02X}",
            exp_c2, cpu.regs.c2
        ));
    }
    if cpu.regs.d2 != exp_d2 {
        return Err(format!(
            "D2: expected {:02X}, got {:02X}",
            exp_d2, cpu.regs.d2
        ));
    }
    if cpu.regs.e2 != exp_e2 {
        return Err(format!(
            "E2: expected {:02X}, got {:02X}",
            exp_e2, cpu.regs.e2
        ));
    }
    if cpu.regs.h2 != exp_h2 {
        return Err(format!(
            "H2: expected {:02X}, got {:02X}",
            exp_h2, cpu.regs.h2
        ));
    }
    if cpu.regs.l2 != exp_l2 {
        return Err(format!(
            "L2: expected {:02X}, got {:02X}",
            exp_l2, cpu.regs.l2
        ));
    }

    // 16-bit registers
    let exp_ix = parse_hex16(&row[20]);
    let exp_iy = parse_hex16(&row[21]);
    let exp_sp = parse_hex16(&row[22]);
    let exp_pc = parse_hex16(&row[23]);

    if cpu.regs.ix != exp_ix {
        return Err(format!(
            "IX: expected {:04X}, got {:04X}",
            exp_ix, cpu.regs.ix
        ));
    }
    if cpu.regs.iy != exp_iy {
        return Err(format!(
            "IY: expected {:04X}, got {:04X}",
            exp_iy, cpu.regs.iy
        ));
    }
    if cpu.regs.sp != exp_sp {
        return Err(format!(
            "SP: expected {:04X}, got {:04X}",
            exp_sp, cpu.regs.sp
        ));
    }
    if cpu.regs.pc != exp_pc {
        return Err(format!(
            "PC: expected {:04X}, got {:04X}",
            exp_pc, cpu.regs.pc
        ));
    }

    // I and R
    let exp_i = parse_hex8(&row[24]);
    let exp_r = parse_hex8(&row[25]);
    if cpu.regs.i != exp_i {
        return Err(format!("I: expected {:02X}, got {:02X}", exp_i, cpu.regs.i));
    }
    if cpu.regs.r != exp_r {
        return Err(format!("R: expected {:02X}, got {:02X}", exp_r, cpu.regs.r));
    }

    // Interrupt flags and mode
    let exp_iff1 = row[26].trim() == "1";
    let exp_iff2 = row[27].trim() == "1";
    let exp_im: u8 = row[28].trim().parse().expect("bad IM");

    if cpu.regs.iff1 != exp_iff1 {
        return Err(format!(
            "IFF1: expected {}, got {}",
            exp_iff1, cpu.regs.iff1
        ));
    }
    if cpu.regs.iff2 != exp_iff2 {
        return Err(format!(
            "IFF2: expected {}, got {}",
            exp_iff2, cpu.regs.iff2
        ));
    }
    if cpu.regs.im != exp_im {
        return Err(format!("IM: expected {}, got {}", exp_im, cpu.regs.im));
    }

    // HALTED state
    let exp_halted = row[29].trim() == "1";
    if cpu.regs.halted != exp_halted {
        return Err(format!(
            "HALTED: expected {}, got {}",
            exp_halted, cpu.regs.halted
        ));
    }

    // Memory writes
    let exp_writes = parse_writes(&row[30]);
    if bus.writes.len() != exp_writes.len() {
        return Err(format!(
            "writes count: expected {}, got {}",
            exp_writes.len(),
            bus.writes.len()
        ));
    }
    for (i, &(exp_addr, exp_val)) in exp_writes.iter().enumerate() {
        if i < bus.writes.len() {
            let (got_addr, got_val) = bus.writes[i];
            if got_addr != exp_addr || got_val != exp_val {
                return Err(format!(
                    "write[{}]: expected {:04X}={:02X}, got {:04X}={:02X}",
                    i, exp_addr, exp_val, got_addr, got_val
                ));
            }
        }
    }

    // Port outputs
    let exp_port_out = parse_port_out(&row[31]);
    if bus.port_out.len() != exp_port_out.len() {
        return Err(format!(
            "port_out count: expected {}, got {}",
            exp_port_out.len(),
            bus.port_out.len()
        ));
    }
    for (i, &(exp_port, exp_val)) in exp_port_out.iter().enumerate() {
        if i < bus.port_out.len() {
            let (got_port, got_val) = bus.port_out[i];
            if got_port != exp_port || got_val != exp_val {
                return Err(format!(
                    "port_out[{}]: expected {:04X}={:02X}, got {:04X}={:02X}",
                    i, exp_port, exp_val, got_port, got_val
                ));
            }
        }
    }

    // Port inputs
    let exp_port_in = parse_port_in(&row[32]);
    if bus.port_in.len() != exp_port_in.len() {
        return Err(format!(
            "port_in count: expected {}, got {}",
            exp_port_in.len(),
            bus.port_in.len()
        ));
    }
    for (i, &exp_port) in exp_port_in.iter().enumerate() {
        if i < bus.port_in.len() {
            let got_port = bus.port_in[i];
            if got_port != exp_port {
                return Err(format!(
                    "port_in[{}]: expected {:04X}, got {:04X}",
                    i, exp_port, got_port
                ));
            }
        }
    }

    Ok(())
}

/// Filters from the environment: Z80_DIFF_TABLE, Z80_DIFF_FAMILY, Z80_DIFF_OP.
fn get_filtered_rows() -> Vec<(String, u8, u8, Vec<String>)> {
    let table = std::env::var("Z80_DIFF_TABLE").unwrap_or_else(|_| "all".to_string());
    let family = std::env::var("Z80_DIFF_FAMILY").unwrap_or_default();
    let op = std::env::var("Z80_DIFF_OP").unwrap_or_default();
    filtered_rows(&table, &family, &op)
}

/// Read diff.tsv and keep the rows matching the table, family list and opcode list.
fn filtered_rows(
    table_filter: &str,
    family_filter: &str,
    op_filter: &str,
) -> Vec<(String, u8, u8, Vec<String>)> {
    let all_rows = common::tsv::read_tsv("diff.tsv");

    // Parse opcode filter into set of hex values
    let mut op_set: Option<Vec<u8>> = None;
    if !op_filter.is_empty() {
        op_set = Some(op_filter.split(',').map(parse_hex8).collect());
    }

    // Parse family filter into set of names
    let mut family_set: Option<Vec<String>> = None;
    if !family_filter.is_empty() {
        family_set = Some(
            family_filter
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    // Instruction family of every opcode, read from the specification TSV of each table.
    let mut family_map: std::collections::HashMap<(String, u8), String> =
        std::collections::HashMap::new();
    for name in ["main", "cb", "ed", "ddfd", "ddcb"] {
        for row in common::tsv::read_tsv(&format!("{name}.tsv")) {
            if row.len() >= 3 {
                family_map.insert(
                    (name.to_string(), parse_hex8(&row[0])),
                    row[2].trim().to_string(),
                );
            }
        }
    }
    // Filter rows
    let mut result = Vec::new();
    for raw in all_rows.iter() {
        // A cell can hold several values (register groups): flatten to 33 fields.
        let row: Vec<String> = raw
            .iter()
            .flat_map(|c| c.split_whitespace().map(str::to_string))
            .collect();
        if row.len() < 33 {
            continue;
        }

        let table = row[0].trim().to_string();
        let opcode = parse_hex8(&row[1]);
        let vec: u8 = row[2].trim().parse().expect("bad vec");

        // Apply table filter
        if table_filter != "all" && table != table_filter {
            continue;
        }

        // Apply opcode filter
        if let Some(ref ops) = op_set {
            if !ops.contains(&opcode) {
                continue;
            }
        }

        // Apply family filter
        if let Some(ref fams) = family_set {
            match family_map.get(&(table.clone(), opcode)) {
                Some(fam) if fams.contains(fam) => {}
                _ => continue,
            }
        }

        result.push((table, opcode, vec, row.clone()));
    }
    result
}

/// Self-test: verify xorshift32 PRNG produces expected sequence.
#[test]
fn e05_01_xorshift_vector() {
    let mut rng = Xs32::new(1);
    assert_eq!(rng.next(), 270369);
    assert_eq!(rng.next(), 67634689);
    assert_eq!(rng.next(), 2647435461);
    assert_eq!(rng.next(), 307599695);
    assert_eq!(rng.next(), 2398689233);
}

/// Self-test: verify hash function produces expected values.
#[test]
fn e05_01_hash_vectors() {
    assert_eq!(hash(0, 1), 233);
    assert_eq!(hash(65535, 7), 174);
}

/// Self-test: verify seed formula.
#[test]
fn e05_01_seed_formula() {
    assert_eq!(seed(0, 0x76, 0), 3642849633);
    assert_eq!(seed(0, 0, 0), 1);
    assert_eq!(seed(1, 0xFF, 5), 619304293);
}

/// Self-test: NOP instruction should pass (validates the harness itself).
#[test]
fn e05_01_selftest_nop() {
    // NOP is opcode 0x00 in main table, family "nop"
    let rows = filtered_rows("main", "nop", "");
    println!("get_filtered_rows returned {} rows", rows.len());
    if !rows.is_empty() {
        println!("First row: {:?}", rows[0]);
    }
    let nop_rows: Vec<_> = rows
        .iter()
        .filter(|(table, op, _, _)| *table == "main" && *op == 0x00)
        .collect();

    assert!(!nop_rows.is_empty(), "No NOP vectors found");

    for (table, opcode, vec, row) in nop_rows {
        let (cpu, bus) = run_vector(table, *opcode, *vec).expect("run_vector failed");
        compare_vector(table, *opcode, *vec, row, &cpu, &bus)
            .unwrap_or_else(|e| panic!("NOP vector {} failed: {}", vec, e));
    }
}

/// Verify that family filter works (should select exactly 6 NOP vectors).
#[test]
fn e05_01_filter_family() {
    let rows = filtered_rows("all", "nop", "");
    // NOP has 6 vectors in main table
    assert_eq!(rows.len(), 6, "Expected 6 NOP vectors with family filter");
}

/// Verify that diff.tsv contains the expected number of data rows.
#[test]
fn e05_01_parse_rows() {
    let all_rows = common::tsv::read_tsv("diff.tsv");
    assert_eq!(all_rows.len(), 7632, "Expected 7632 diff vectors");
}

/// Main differential test: run all filtered vectors and report failures.
#[test]
fn e05_01_diff_run() {
    let any_filter = ["Z80_DIFF_TABLE", "Z80_DIFF_FAMILY", "Z80_DIFF_OP"]
        .iter()
        .any(|k| std::env::var(k).is_ok());
    if !any_filter {
        println!(
            "e05_01_diff_run: no filter set, skipped (set Z80_DIFF_TABLE=all to run every vector)"
        );
        return;
    }
    let rows = get_filtered_rows();
    println!("Running {} differential vectors", rows.len());

    if rows.is_empty() {
        panic!("No vectors matched filters");
    }

    let mut failures = Vec::new();
    for (i, (table, opcode, vec, row)) in rows.iter().enumerate() {
        match run_vector(table, *opcode, *vec) {
            Ok((cpu, bus)) => {
                if let Err(e) = compare_vector(table, *opcode, *vec, row, &cpu, &bus) {
                    failures.push(format!(
                        "[{}/{}] {} {:02X} vec{}: {}",
                        i + 1,
                        rows.len(),
                        table,
                        opcode,
                        vec,
                        e
                    ));
                }
            }
            Err(e) => {
                failures.push(format!(
                    "[{}/{}] {} {:02X} vec{}: run failed: {}",
                    i + 1,
                    rows.len(),
                    table,
                    opcode,
                    vec,
                    e
                ));
            }
        }

        // Print progress every 100 vectors
        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{}", i + 1, rows.len());
        }
    }

    if !failures.is_empty() {
        println!("\n{} failures:", failures.len());
        for f in &failures[..20.min(failures.len())] {
            println!("  {}", f);
        }
        if failures.len() > 20 {
            println!("  ... and {} more", failures.len() - 20);
        }
        panic!("Differential test failed");
    }

    println!("All {} vectors passed", rows.len());
}

/// E05-16: Run all 1512 vectors from the main table.
#[test]
fn e05_16_main_all() {
    let rows = filtered_rows("main", "", "");
    assert_eq!(rows.len(), 1512, "Expected 1512 main table vectors");

    println!("Running {} differential vectors (main table)", rows.len());

    let mut failures = Vec::new();
    for (i, (table, opcode, vec, row)) in rows.iter().enumerate() {
        match run_vector(table, *opcode, *vec) {
            Ok((cpu, bus)) => {
                if let Err(e) = compare_vector(table, *opcode, *vec, row, &cpu, &bus) {
                    failures.push(format!(
                        "[{}/{}] {} {:02X} vec{}: {}",
                        i + 1,
                        rows.len(),
                        table,
                        opcode,
                        vec,
                        e
                    ));
                }
            }
            Err(e) => {
                failures.push(format!(
                    "[{}/{}] {} {:02X} vec{}: run failed: {}",
                    i + 1,
                    rows.len(),
                    table,
                    opcode,
                    vec,
                    e
                ));
            }
        }

        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{}", i + 1, rows.len());
        }
    }

    if !failures.is_empty() {
        println!("\n{} failures:", failures.len());
        for f in &failures[..20.min(failures.len())] {
            println!("  {}", f);
        }
        if failures.len() > 20 {
            println!("  ... and {} more", failures.len() - 20);
        }
        panic!("Differential test failed");
    }

    println!("All {} main table vectors passed", rows.len());
}

/// E06-04: Run all 1536 vectors from the CB prefix table.
#[test]
fn e06_04_cb_all() {
    let rows = filtered_rows("cb", "", "");
    assert_eq!(rows.len(), 1536, "Expected 1536 CB table vectors");

    println!("Running {} differential vectors (CB table)", rows.len());

    let mut failures = Vec::new();
    for (i, (table, opcode, vec, row)) in rows.iter().enumerate() {
        match run_vector(table, *opcode, *vec) {
            Ok((cpu, bus)) => {
                if let Err(e) = compare_vector(table, *opcode, *vec, row, &cpu, &bus) {
                    failures.push(format!(
                        "[{}/{}] {} {:02X} vec{}: {}",
                        i + 1,
                        rows.len(),
                        table,
                        opcode,
                        vec,
                        e
                    ));
                }
            }
            Err(e) => {
                failures.push(format!(
                    "[{}/{}] {} {:02X} vec{}: run failed: {}",
                    i + 1,
                    rows.len(),
                    table,
                    opcode,
                    vec,
                    e
                ));
            }
        }

        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{}", i + 1, rows.len());
        }
    }

    if !failures.is_empty() {
        println!("\n{} failures:", failures.len());
        for f in &failures[..20.min(failures.len())] {
            println!("  {}", f);
        }
        if failures.len() > 20 {
            println!("  ... and {} more", failures.len() - 20);
        }
        panic!("Differential test failed");
    }

    println!("All {} CB table vectors passed", rows.len());
}

/// E07-07: Run all 1536 vectors from the ED prefix table.
#[test]
fn e07_07_ed_all() {
    let rows = filtered_rows("ed", "", "");
    assert_eq!(rows.len(), 1536, "Expected 1536 ED table vectors");

    println!("Running {} differential vectors (ED table)", rows.len());

    let mut failures = Vec::new();
    for (i, (table, opcode, vec, row)) in rows.iter().enumerate() {
        match run_vector(table, *opcode, *vec) {
            Ok((cpu, bus)) => {
                if let Err(e) = compare_vector(table, *opcode, *vec, row, &cpu, &bus) {
                    failures.push(format!(
                        "[{}/{}] {} {:02X} vec{}: {}",
                        i + 1,
                        rows.len(),
                        table,
                        opcode,
                        vec,
                        e
                    ));
                }
            }
            Err(e) => {
                failures.push(format!(
                    "[{}/{}] {} {:02X} vec{}: run failed: {}",
                    i + 1,
                    rows.len(),
                    table,
                    opcode,
                    vec,
                    e
                ));
            }
        }

        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{}", i + 1, rows.len());
        }
    }

    if !failures.is_empty() {
        println!("\n{} failures:", failures.len());
        for f in &failures[..20.min(failures.len())] {
            println!("  {}", f);
        }
        if failures.len() > 20 {
            println!("  ... and {} more", failures.len() - 20);
        }
        panic!("Differential test failed");
    }

    println!("All {} ED table vectors passed", rows.len());
}
