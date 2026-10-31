//! Instruction semantics dispatch (stub).
//!
//! Called by the engine when a hook token (Kind::X) is encountered.
//! The actual instruction semantics will be implemented in E05+.

use crate::cpu::Cpu;
use crate::engine::decode::Table;

/// Execute instruction semantics for a hook.
///
/// # Arguments
/// * `cpu` - CPU state (mutated by the instruction)
/// * `table` - Current opcode table
/// * `op` - Opcode byte
/// * `hook_no` - Hook number (0 or 1)
pub(crate) fn sem(_cpu: &mut Cpu, _table: Table, _op: u8, _hook_no: u8) {
    // Stub: actual semantics implemented in E05+
}
