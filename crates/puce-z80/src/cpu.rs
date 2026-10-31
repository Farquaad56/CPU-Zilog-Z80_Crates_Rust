//! CPU top-level struct and tick interface.

use crate::engine::{Engine};
use crate::engine::plan::FETCH_PROBE;
use crate::pins::Pins;
use crate::regs::Regs;

/// The Z80 CPU state: registers plus execution engine.
pub struct Cpu {
    /// Register file (public for inspection).
    pub regs: Regs,
    /// Execution engine (internal).
    pub(crate) eng: Engine,
}

impl Cpu {
    /// Create a new CPU in its initial state (all registers zeroed, engine idle).
    pub fn new() -> Cpu {
        Cpu {
            regs: Regs::default(),
            eng: Engine::new(),
        }
    }

    /// Returns true when the CPU is at an instruction boundary (not mid-instruction).
    pub fn at_instruction_boundary(&self) -> bool {
        !self.eng.active
    }

    /// Execute one T-state.
    ///
    /// `pins_in` contains inputs seen by the CPU during this T-state.
    /// Returns outputs after this T-state. Currently a stub that echoes pins back;
    /// real tick logic will be implemented in E03-07.
    pub fn tick(&mut self, pins: Pins) -> Pins {
        // Stub: return pins unchanged until engine tick is implemented.
        pins
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::decode::{Prefix, Table};

    #[test]
    fn e03_01_new_idle() {
        let cpu = Cpu::new();
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_01_default_eq_new() {
        let default_cpu = Cpu::default();
        let new_cpu = Cpu::new();
        assert_eq!(default_cpu.regs, new_cpu.regs);
    }

    #[test]
    fn e03_01_engine_defaults() {
        let eng = Engine::new();
        assert_eq!(eng.table, Table::Main);
        assert_eq!(eng.prefix, Prefix::None);
        assert_eq!(eng.idx, 0);
        assert_eq!(eng.t, 0);
        assert!(!eng.cond);
        assert_eq!(eng.tmp_l, 0);
        assert_eq!(eng.tmp_h, 0);
    }

    #[test]
    fn e03_01_fetch_probe() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let probe = FETCH_PROBE;
        assert_eq!(probe.kind, Kind::F);
        assert_eq!(probe.len, 4);
        assert_eq!(probe.addr, Addr::PcInc);
        assert_eq!(probe.reg, Reg::None);
    }
}
