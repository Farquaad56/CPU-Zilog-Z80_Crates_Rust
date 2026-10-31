//! Opcode decoding: maps (table, byte) to execution plans or prefix transitions.
//!
//! This module is pure - it depends only on the generated opcode tables and
//! produces no side effects. The CPU engine calls decode() for each M1 cycle
//! to determine what instruction to execute next.

use crate::engine::plan::Plan;
use crate::tables::{cb, ddcb, ddfd, ed, main};

/// Opcode table selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    /// Main opcode table (256 entries).
    Main,
    /// CB prefix table (bit/rotate operations).
    Cb,
    /// ED prefix table (extended instructions).
    Ed,
    /// DD/FD prefix table (IX/IY indexed instructions).
    DdFd,
    /// DDCB/DDCB table (indexed bit/rotate operations).
    DdCb,
}

/// Active prefix state (IX vs IY).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefix {
    /// No active index prefix.
    None,
    /// DD prefix active (IX register).
    Dd,
    /// FD prefix active (IY register).
    Fd,
}

/// Result of decoding a byte in the given table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoded {
    /// Instruction identified: (table used, execution plan).
    Plan(Table, Plan),
    /// Byte is a prefix: next M1 should use the indicated table.
    Prefix(Table),
    /// CB after DD/FD: enter DDCB mode (displacement and opcode read as data).
    IndexCb,
    /// DD/FD/ED after DD/FD: previous prefix consumed (4 T-states), restart decode.
    Restart(u8),
}

/// Look up the execution plan for an instruction in the given table.
/// Returns empty slice for prefix opcodes that have no plan of their own.
pub fn plan_of(table: Table, op: u8) -> Plan {
    match table {
        Table::Main => main::MAIN[op as usize],
        Table::Cb => cb::CB[op as usize],
        Table::Ed => ed::ED[op as usize],
        Table::DdFd => ddfd::DDFD[op as usize],
        Table::DdCb => ddcb::DDCB[op as usize],
    }
}

/// Decode a byte in the given table context.
///
/// Rules:
/// - Main table: CB->Prefix(Cb), ED->Prefix(Ed), DD/FD->Prefix(DdFd), else Plan(Main, plan)
/// - Cb/Ed tables: always Plan (no prefixes within these tables)
/// - DdFd table: CB->IndexCb; DD/FD/ED->Restart(byte); else Plan(DdFd, plan)
pub fn decode(table: Table, byte: u8) -> Decoded {
    match table {
        Table::Main => {
            if byte == 0xCB {
                return Decoded::Prefix(Table::Cb);
            }
            if byte == 0xED {
                return Decoded::Prefix(Table::Ed);
            }
            if byte == 0xDD || byte == 0xFD {
                return Decoded::Prefix(Table::DdFd);
            }
            let plan = main::MAIN[byte as usize];
            Decoded::Plan(Table::Main, plan)
        }
        Table::Cb => {
            let plan = cb::CB[byte as usize];
            Decoded::Plan(Table::Cb, plan)
        }
        Table::Ed => {
            let plan = ed::ED[byte as usize];
            Decoded::Plan(Table::Ed, plan)
        }
        Table::DdFd => {
            if byte == 0xCB {
                return Decoded::IndexCb;
            }
            if byte == 0xDD || byte == 0xFD || byte == 0xED {
                return Decoded::Restart(byte);
            }
            let plan = ddfd::DDFD[byte as usize];
            Decoded::Plan(Table::DdFd, plan)
        }
        // DdCb is never returned by decode; its plan is chosen after reading the op byte.
        Table::DdCb => {
            unreachable!("decode() should not be called with Table::DdCb");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::plan::plan_t_min;

    #[test]
    fn e02_04_main_plain() {
        // NOP (0x00) should decode to a Plan in the Main table
        let decoded = decode(Table::Main, 0x00);
        match decoded {
            Decoded::Plan(t, plan) => {
                assert_eq!(t, Table::Main);
                // NOP is F4 (single M1 cycle, 4 T-states)
                assert_eq!(plan_t_min(plan), 4);
            }
            _ => panic!("expected Plan for NOP"),
        }
    }

    #[test]
    fn e02_04_main_prefixes() {
        // CB -> Prefix(Cb)
        match decode(Table::Main, 0xCB) {
            Decoded::Prefix(t) => assert_eq!(t, Table::Cb),
            _ => panic!("expected Prefix(Cb) for 0xCB"),
        }

        // ED -> Prefix(Ed)
        match decode(Table::Main, 0xED) {
            Decoded::Prefix(t) => assert_eq!(t, Table::Ed),
            _ => panic!("expected Prefix(Ed) for 0xED"),
        }

        // DD -> Prefix(DdFd)
        match decode(Table::Main, 0xDD) {
            Decoded::Prefix(t) => assert_eq!(t, Table::DdFd),
            _ => panic!("expected Prefix(DdFd) for 0xDD"),
        }

        // FD -> Prefix(DdFd)
        match decode(Table::Main, 0xFD) {
            Decoded::Prefix(t) => assert_eq!(t, Table::DdFd),
            _ => panic!("expected Prefix(DdFd) for 0xFD"),
        }
    }

    #[test]
    fn e02_04_ddfd_cb() {
        // CB after DD/FD -> IndexCb (DDCB mode)
        match decode(Table::DdFd, 0xCB) {
            Decoded::IndexCb => {}
            _ => panic!("expected IndexCb for 0xCB in DdFd table"),
        }
    }

    #[test]
    fn e02_04_ddfd_restart() {
        // DD after DD/FD -> Restart(DD)
        match decode(Table::DdFd, 0xDD) {
            Decoded::Restart(b) => assert_eq!(b, 0xDD),
            _ => panic!("expected Restart for 0xDD in DdFd table"),
        }

        // FD after DD/FD -> Restart(FD)
        match decode(Table::DdFd, 0xFD) {
            Decoded::Restart(b) => assert_eq!(b, 0xFD),
            _ => panic!("expected Restart for 0xFD in DdFd table"),
        }

        // ED after DD/FD -> Restart(ED)
        match decode(Table::DdFd, 0xED) {
            Decoded::Restart(b) => assert_eq!(b, 0xED),
            _ => panic!("expected Restart for 0xED in DdFd table"),
        }
    }

    #[test]
    fn e02_04_ddfd_plain() {
        // LD IX,nn (DD 21) should decode to a Plan with 14 T-states minimum
        match decode(Table::DdFd, 0x21) {
            Decoded::Plan(t, plan) => {
                assert_eq!(t, Table::DdFd);
                // F5 + R3 + R3 + I3 = 14 T-states (hook contributes 0)
                assert_eq!(plan_t_min(plan), 14);
            }
            _ => panic!("expected Plan for 0x21 in DdFd table"),
        }
    }

    #[test]
    fn e02_04_ed_nop() {
        // ED 00 is undocumented NOP, should be 8 T-states (F5 + I3)
        match decode(Table::Ed, 0x00) {
            Decoded::Plan(t, plan) => {
                assert_eq!(t, Table::Ed);
                assert_eq!(plan_t_min(plan), 8);
            }
            _ => panic!("expected Plan for ED 00"),
        }
    }

    #[test]
    fn e02_04_ddcb_plan() {
        // BIT 0,(IX+d) via DDCB: op=0x46, should be 20 T-states minimum
        let plan = plan_of(Table::DdCb, 0x46);
        assert_eq!(plan_t_min(plan), 20);
    }
}
