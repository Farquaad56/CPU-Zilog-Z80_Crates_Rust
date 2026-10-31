//! Consistency tests for generated opcode decode tables.
//!
//! These tests verify that the generated tables are complete and consistent
//! with the TSV specifications, acting as a guard against generator defects.

use crate::engine::plan::{plan_t_max, plan_t_min, Kind};
use crate::tables;

/// All 5 tables must have exactly 256 entries (one per opcode byte).
#[test]
fn e02_03_sizes() {
    assert_eq!(tables::main::MAIN.len(), 256);
    assert_eq!(tables::cb::CB.len(), 256);
    assert_eq!(tables::ed::ED.len(), 256);
    assert_eq!(tables::ddfd::DDFD.len(), 256);
    assert_eq!(tables::ddcb::DDCB.len(), 256);

    // Timing tables must also have 256 entries each
    assert_eq!(tables::main::MAIN_T.len(), 256);
    assert_eq!(tables::cb::CB_T.len(), 256);
    assert_eq!(tables::ed::ED_T.len(), 256);
    assert_eq!(tables::ddfd::DDFD_T.len(), 256);
    assert_eq!(tables::ddcb::DDCB_T.len(), 256);
}

/// For every non-empty plan, the first token must be a fetch (Kind::F).
#[test]
fn e02_03_first_token_is_fetch() {
    let all_tables = [
        ("main", &tables::main::MAIN),
        ("cb", &tables::cb::CB),
        ("ed", &tables::ed::ED),
        ("ddfd", &tables::ddfd::DDFD),
        ("ddcb", &tables::ddcb::DDCB),
    ];

    for (name, table) in all_tables.iter() {
        for (opcode, plan) in table.iter().enumerate() {
            if !plan.is_empty() {
                assert_eq!(
                    plan[0].kind,
                    Kind::F,
                    "{}[0x{:02X}]: first token is not a fetch",
                    name,
                    opcode
                );
            }
        }
    }
}

/// For each table and opcode, the computed t_min/t_max from the plan must match
/// the values stored in the corresponding _T timing table.
#[test]
fn e02_03_t_sums_match() {
    let all_tables = [
        ("main", &tables::main::MAIN, &tables::main::MAIN_T),
        ("cb", &tables::cb::CB, &tables::cb::CB_T),
        ("ed", &tables::ed::ED, &tables::ed::ED_T),
        ("ddfd", &tables::ddfd::DDFD, &tables::ddfd::DDFD_T),
        ("ddcb", &tables::ddcb::DDCB, &tables::ddcb::DDCB_T),
    ];

    for (name, plans, timings) in all_tables.iter() {
        for opcode in 0usize..256 {
            let plan = plans[opcode];
            let (t_min_expected, t_max_expected) = timings[opcode];

            if !plan.is_empty() {
                let computed_min = plan_t_min(plan);
                let computed_max = plan_t_max(plan);

                assert_eq!(
                    computed_min,
                    u32::from(t_min_expected),
                    "{}[0x{:02X}]: t_min mismatch: computed {} vs expected {}",
                    name,
                    opcode,
                    computed_min,
                    t_min_expected
                );

                assert_eq!(
                    computed_max,
                    u32::from(t_max_expected),
                    "{}[0x{:02X}]: t_max mismatch: computed {} vs expected {}",
                    name,
                    opcode,
                    computed_max,
                    t_max_expected
                );
            } else {
                // Empty plans (prefix rows) should have (0, 0) timing
                assert_eq!(
                    t_min_expected, 0,
                    "{}[0x{:02X}]: empty plan but t_min={}",
                    name, opcode, t_min_expected
                );
                assert_eq!(
                    t_max_expected, 0,
                    "{}[0x{:02X}]: empty plan but t_max={}",
                    name, opcode, t_max_expected
                );
            }
        }
    }
}

/// Prefix rows in main table (CB, DD, ED, FD) and ddfd table (CB, DD, ED, FD)
/// must have empty plans.
#[test]
fn e02_03_prefix_rows_empty() {
    // Main table prefix opcodes
    let main_prefixes = [0xCB, 0xDD, 0xED, 0xFD];
    for opcode in &main_prefixes {
        assert!(
            tables::main::MAIN[*opcode as usize].is_empty(),
            "main[0x{:02X}] should be empty (prefix row)",
            opcode
        );
    }

    // DDFD table prefix opcodes
    let ddfd_prefixes = [0xCB, 0xDD, 0xED, 0xFD];
    for opcode in &ddfd_prefixes {
        assert!(
            tables::ddfd::DDFD[*opcode as usize].is_empty(),
            "ddfd[0x{:02X}] should be empty (prefix row)",
            opcode
        );
    }
}

/// DJNZ e (main[0x10]) must have the specific plan structure:
/// F5, R3 PcInc->L, X0, ?I5, X1 with timing 8..13.
#[test]
fn e02_03_djnz_plan() {
    let plan = tables::main::MAIN[0x10];

    // Must have exactly 5 tokens
    assert_eq!(plan.len(), 5, "DJNZ must have 5 tokens");

    // Token 0: F5 (M1 cycle with refresh)
    assert_eq!(plan[0].kind, Kind::F);
    assert_eq!(plan[0].len, 5);

    // Token 1: R3 PcInc->L (read displacement into L)
    assert_eq!(plan[1].kind, Kind::R);
    assert_eq!(plan[1].len, 3);

    // Token 2: X0 (hook for decrement and condition check)
    assert_eq!(plan[2].kind, Kind::X);
    assert_eq!(plan[2].hook_no(), 0);

    // Token 3: ?I5 (conditional internal cycle, only if B != 0)
    assert_eq!(plan[3].kind, Kind::I);
    assert_eq!(plan[3].len, 5);
    assert!(plan[3].cond, "DJNZ I5 token must be conditional");

    // Token 4: X1 (hook for PC update)
    assert_eq!(plan[4].kind, Kind::X);
    assert_eq!(plan[4].hook_no(), 1);

    // Timing: t_min=8 (skip conditional), t_max=13 (include conditional)
    let (t_min, t_max) = tables::main::MAIN_T[0x10];
    assert_eq!(t_min, 8);
    assert_eq!(t_max, 13);
}

/// LD BC,nn (main[0x01]) must have exactly 4 tokens and timing of 10 T-states.
#[test]
fn e02_03_ld_rp_nn() {
    let plan = tables::main::MAIN[0x01];

    // Must have exactly 4 tokens: F4, R3 PcInc->Z, R3 PcInc->W, X0
    assert_eq!(plan.len(), 4, "LD BC,nn must have 4 tokens");

    // First token is fetch
    assert_eq!(plan[0].kind, Kind::F);

    // Timing: both min and max are 10 (no conditional cycles)
    let (t_min, t_max) = tables::main::MAIN_T[0x01];
    assert_eq!(t_min, 10);
    assert_eq!(t_max, 10);
}

/// LD IX,nn (ddfd[0x21]) must have the plan F4 F4 R3 R3 X0 with 14 T-states.
#[test]
fn e02_03_ddfd_ld_ix_nn() {
    let plan = tables::ddfd::DDFD[0x21];

    // Must have exactly 5 tokens: two M1 cycles (prefix + opcode), two reads, one hook
    assert_eq!(plan.len(), 5, "LD IX,nn must have 5 tokens");

    // Token 0: F4 (M1 for DD prefix)
    assert_eq!(plan[0].kind, Kind::F);
    assert_eq!(plan[0].len, 4);

    // Token 1: F4 (M1 for opcode)
    assert_eq!(plan[1].kind, Kind::F);
    assert_eq!(plan[1].len, 4);

    // Tokens 2-3: R3 reads for the two bytes of nn
    assert_eq!(plan[2].kind, Kind::R);
    assert_eq!(plan[2].len, 3);
    assert_eq!(plan[3].kind, Kind::R);
    assert_eq!(plan[3].len, 3);

    // Token 4: X0 (hook to load into IX)
    assert_eq!(plan[4].kind, Kind::X);
    assert_eq!(plan[4].hook_no(), 0);

    // Timing: 14 T-states total
    let (t_min, t_max) = tables::ddfd::DDFD_T[0x21];
    assert_eq!(t_min, 14);
    assert_eq!(t_max, 14);
}

/// In every plan, X1 hooks must never appear before X0 hooks.
#[test]
fn e02_03_hooks_balanced() {
    let all_tables = [
        ("main", &tables::main::MAIN),
        ("cb", &tables::cb::CB),
        ("ed", &tables::ed::ED),
        ("ddfd", &tables::ddfd::DDFD),
        ("ddcb", &tables::ddcb::DDCB),
    ];

    for (name, table) in all_tables.iter() {
        for (opcode, plan) in table.iter().enumerate() {
            let mut seen_x0 = false;
            for tok in plan.iter() {
                if tok.is_hook() {
                    match tok.hook_no() {
                        0 => seen_x0 = true,
                        1 => {
                            assert!(seen_x0, "{}[0x{:02X}]: X1 appears before X0", name, opcode);
                        }
                        other => {
                            panic!("{}[0x{:02X}]: invalid hook number {}", name, opcode, other)
                        }
                    }
                }
            }
        }
    }
}
