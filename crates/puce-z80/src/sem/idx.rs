//! DD/FD prefix table semantics (stub).
//!
//! All indexed instruction semantics will be implemented here in E05+.
//! For now, DdFd routes to sem_main (prefix ignored) and DdCb is a no-op stub.

use super::main::sem_main;
use crate::cpu::Cpu;

/// Execute DD/FD-prefix instruction semantics.
///
/// Per spec 23-4: for instructions without HL, the prefix is ignored (4 T lost).
/// For now, all DdFd opcodes route to sem_main (E08 will refine this).
pub(crate) fn sem_idx(cpu: &mut Cpu, op: u8, hook_no: u8) {
    // Prefix ignored for now; E08 will add proper indexed addressing.
    sem_main(cpu, op, hook_no);
}

/// Execute DD CB / FD CB instruction semantics (stub).
pub(crate) fn sem_ddcb(_cpu: &mut Cpu, _op: u8, _hook_no: u8) {
    // Stub: actual semantics implemented in E05+
}
