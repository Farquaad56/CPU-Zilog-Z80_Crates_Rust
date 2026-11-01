//! CB prefix table semantics (stub).
//!
//! All CB-prefix instruction semantics will be implemented here in E05+.
//! For now, this is a no-op stub.

use crate::cpu::Cpu;

/// Execute CB-prefix instruction semantics.
pub(crate) fn sem_cb(_cpu: &mut Cpu, _op: u8, _hook_no: u8) {
    // Stub: actual semantics implemented in E05+
}
