//! Execution engine state and tick logic.

use crate::engine::decode::{Prefix, Table};
use crate::engine::plan::Plan;

/// Internal execution engine state (not exposed publicly).
pub(crate) struct Engine {
    /// Current opcode table being used for decoding.
    pub table: Table,
    /// Active index prefix (IX or IY), if any.
    pub prefix: Prefix,
    /// Opcode byte of the current instruction (valid after last M1 fetch).
    pub op: u8,
    /// Execution plan for the current instruction.
    pub plan: Plan,
    /// Index into the current plan's token sequence.
    pub idx: usize,
    /// T-state counter within the current token (0 = first T of this token).
    pub t: u8,
    /// True while an instruction is in progress; false at instruction boundaries.
    pub active: bool,
    /// Number of M1 cycles already performed for the current instruction (including prefixes).
    pub fetches: u8,
    /// Condition flag set by hook X0 to control conditional tokens.
    pub cond: bool,
    /// Temporary low byte register (TMP.L).
    pub tmp_l: u8,
    /// Temporary high byte register (TMP.H).
    pub tmp_h: u8,
    /// Signed displacement byte for IX/IY indexed addressing.
    pub disp: u8,
    /// Opcode byte read as data (for DD CB d op sequences).
    pub opb: u8,
    /// True if a WAIT T-state has been inserted into the current token.
    pub wait_pending: bool,
    /// Last data byte read from D0..D7 bus.
    pub latched: u8,
    /// Computed IX/IY+d address (stored in WZ).
    pub xy_addr: u16,
    /// True while reading a DD/FD CB instruction sequence (displacement and opcode as data).
    pub ddcb: bool,
    /// T-states consumed by prefixes for the current Restart instruction.
    pub restart_cost: u32,
    /// Total T-states of the current instruction (excluding WAIT states).
    pub ts: u32,
    /// Total WAIT T-states inserted during the current instruction.
    pub ts_wait: u32,
}

impl Engine {
    /// Create a new engine in its initial idle state.
    pub fn new() -> Engine {
        Engine {
            table: Table::Main,
            prefix: Prefix::None,
            op: 0,
            plan: &[],
            idx: 0,
            t: 0,
            active: false,
            fetches: 0,
            cond: false,
            tmp_l: 0,
            tmp_h: 0,
            disp: 0,
            opb: 0,
            wait_pending: false,
            latched: 0,
            xy_addr: 0,
            ddcb: false,
            restart_cost: 0,
            ts: 0,
            ts_wait: 0,
        }
    }
}

pub(crate) mod addr;
pub mod decode;
pub mod mcycle;
pub mod plan;
