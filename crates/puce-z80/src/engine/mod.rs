//! Execution engine state and tick logic.

// Temporary: engine items are consumed progressively by later tasks (E03-09+).
#![allow(dead_code)]

use crate::engine::decode::{Prefix, Table};
use crate::engine::plan::{Plan, Reg};
use crate::regs::Regs;

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
    /// Current bus address for the active token (set at t0 of each bus cycle).
    pub cur_addr: u16,
    /// True if the current instruction wrote to F via set_f().
    pub f_written: bool,
    /// EI delay flag: true after EI until next instruction completes (E09).
    pub ei_delay: bool,
    /// True while the RESET pin is held active (the reset logic runs once per assertion).
    pub in_reset: bool,
    /// True while the current M1 is a HALT NOP cycle (no PC increment, no decode).
    pub halt_cycle: bool,
    /// Interrupt sequence selected at the last instruction boundary (E09-03).
    pub irq: IrqKind,
    /// A rising edge of NMI was seen and not yet serviced.
    pub nmi_latch: bool,
    /// Previous sampled level of the NMI pin (for edge detection).
    pub nmi_prev: bool,
    /// Sampled level of the INT pin on the current T-state.
    pub int_level: bool,
}

/// Kind of interrupt sequence being started at an instruction boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrqKind {
    /// No interrupt sequence.
    None,
    /// Non-maskable interrupt.
    Nmi,
    /// Maskable interrupt.
    Int,
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
            cur_addr: 0,
            f_written: false,
            ei_delay: false,
            in_reset: false,
            halt_cycle: false,
            irq: IrqKind::None,
            nmi_latch: false,
            nmi_prev: false,
            int_level: false,
        }
    }

    /// Store a value into the appropriate destination register or field.
    /// L -> tmp_l, H -> tmp_h, Z -> wz low byte, W -> wz high byte,
    /// D -> disp, Op -> opb, None -> no-op.
    pub(crate) fn store(&mut self, regs: &mut Regs, r: Reg, v: u8) {
        match r {
            Reg::L => self.tmp_l = v,
            Reg::H => self.tmp_h = v,
            Reg::Z => regs.wz = (regs.wz & 0xFF00) | (v as u16),
            Reg::W => regs.wz = (regs.wz & 0x00FF) | ((v as u16) << 8),
            Reg::D => self.disp = v,
            Reg::Op => self.opb = v,
            Reg::None => {}
        }
    }

    /// Load a value from the appropriate source register or field.
    /// Symmetric to store: L -> tmp_l, H -> tmp_h, Z -> wz low byte,
    /// W -> wz high byte, D -> disp, Op -> opb, None -> 0.
    pub(crate) fn load(&self, regs: &Regs, r: Reg) -> u8 {
        match r {
            Reg::L => self.tmp_l,
            Reg::H => self.tmp_h,
            Reg::Z => (regs.wz & 0x00FF) as u8,
            Reg::W => ((regs.wz >> 8) & 0xFF) as u8,
            Reg::D => self.disp,
            Reg::Op => self.opb,
            Reg::None => 0,
        }
    }
}

pub(crate) mod addr;
pub mod decode;
pub mod mcycle;
pub mod plan;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e03_06_store_l_h() {
        let mut eng = Engine::new();
        let mut regs = Regs::default();
        eng.store(&mut regs, Reg::L, 0xAB);
        eng.store(&mut regs, Reg::H, 0xCD);
        assert_eq!(eng.load(&regs, Reg::L), 0xAB);
        assert_eq!(eng.load(&regs, Reg::H), 0xCD);
    }

    #[test]
    fn e03_06_store_z_w() {
        let mut eng = Engine::new();
        let mut regs = Regs::default();
        eng.store(&mut regs, Reg::Z, 0x34);
        eng.store(&mut regs, Reg::W, 0x12);
        assert_eq!(regs.wz, 0x1234);
    }

    #[test]
    fn e03_06_store_d_op() {
        let mut eng = Engine::new();
        let mut regs = Regs::default();
        eng.store(&mut regs, Reg::D, 0x55);
        eng.store(&mut regs, Reg::Op, 0xAA);
        assert_eq!(eng.load(&regs, Reg::D), 0x55);
        assert_eq!(eng.load(&regs, Reg::Op), 0xAA);
    }

    #[test]
    fn e03_06_none_noop() {
        let mut eng = Engine::new();
        let mut regs = Regs::default();
        eng.tmp_l = 0x11;
        eng.store(&mut regs, Reg::None, 0xFF);
        assert_eq!(eng.tmp_l, 0x11);
        assert_eq!(eng.load(&regs, Reg::None), 0);
    }

    #[test]
    fn e03_06_z_keeps_w() {
        let mut eng = Engine::new();
        let mut regs = Regs::default();
        eng.store(&mut regs, Reg::W, 0x12);
        eng.store(&mut regs, Reg::Z, 0x34);
        assert_eq!(regs.wz, 0x1234);
    }
}
