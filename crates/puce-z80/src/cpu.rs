//! CPU top-level struct and tick interface.

use crate::engine::addr::resolve;
use crate::engine::decode::Table;
#[cfg(test)]
use crate::engine::plan::Tok;
use crate::engine::plan::{Kind, FETCH_PROBE};
use crate::engine::Engine;
use crate::pins::Pins;
use crate::regs::Regs;
use crate::sem::sem;

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
    /// Returns outputs after this T-state.
    pub fn tick(&mut self, pins: Pins) -> Pins {
        // Step 2: If not active, start new instruction with FETCH_PROBE
        if !self.eng.active {
            self.eng.table = Table::Main;
            self.eng.prefix = crate::engine::decode::Prefix::None;
            self.eng.fetches = 0;
            self.eng.plan = &[FETCH_PROBE];
            self.eng.idx = 0;
            self.eng.t = 0;
            self.eng.active = true;
            // Reset cond at instruction boundary (spec: eng.cond reset to false)
            self.eng.cond = false;
        }

        // Step 3: Get current token
        let cur = self.eng.plan[self.eng.idx];

        // At t0 of bus tokens, resolve address and store in cur_addr
        if self.eng.t == 0 && cur.is_bus() {
            let port = matches!(cur.kind, Kind::N | Kind::O);
            self.eng.cur_addr = resolve(self, cur.addr, port);
        }

        // Step 4: Calculate pins_out for (cur, t)
        let mut out = Pins(0).set_addr(self.eng.cur_addr);

        match cur.kind {
            Kind::F => {
                // M1 cycle - but decoding not implemented yet (E03-09), treat like I
                // Just output address, no control signals for now
            }
            Kind::R => {
                // Memory read: t0=A, t1=A+MREQ+RD, t2=A (data latched)
                if self.eng.t == 1 {
                    out = out.with(Pins::MREQ | Pins::RD);
                }
                // At T3 (t==2), latch data and store to destination register
                if self.eng.t == 2 {
                    let data = pins.data();
                    self.eng.latched = data;
                    self.eng.store(&mut self.regs, cur.reg, data);
                }
            }
            Kind::W => {
                // Memory write: t0=A+D, t1=A+D+MREQ+WR, t2=A+D
                let data = self.eng.load(&self.regs, cur.reg);
                out = out.set_data(data);
                if self.eng.t == 1 {
                    out = out.with(Pins::MREQ | Pins::WR);
                }
            }
            Kind::N => {
                // Port read: t0=A, t1=A, t2=A+IORQ+RD (+TW), t3=A (data latched)
                if self.eng.t == 2 {
                    out = out.with(Pins::IORQ | Pins::RD);
                }
                // At T4 (t==3), latch data and store to destination register
                if self.eng.t == 3 {
                    let data = pins.data();
                    self.eng.latched = data;
                    self.eng.store(&mut self.regs, cur.reg, data);
                }
            }
            Kind::O => {
                // Port write: t0=A+D, t1=A+D, t2=A+D+IORQ+WR (+TW), t3=A+D
                let data = self.eng.load(&self.regs, cur.reg);
                out = out.set_data(data);
                if self.eng.t == 2 {
                    out = out.with(Pins::IORQ | Pins::WR);
                }
            }
            Kind::I => {
                // Internal cycle: last bus address, no control signals
            }
            Kind::X => {
                // Hook - should not reach here (handled in step 5)
            }
        }

        // WAIT state insertion: at sampling points, if WAIT pin is active,
        // insert a wait T-state (don't advance t, increment ts_wait).
        let sampling = match cur.kind {
            Kind::F | Kind::R | Kind::W => self.eng.t == 1,
            Kind::N | Kind::O => self.eng.t == 2,
            _ => false, // I and X don't sample WAIT
        };

        if sampling && pins.has(Pins::WAIT) {
            self.eng.ts_wait += 1;
            return out; // Repeat same outputs on next tick at same t
        }

        // Increment t and ts counter
        self.eng.t += 1;
        self.eng.ts += 1;

        // Step 5: If token complete, advance to next
        if self.eng.t == cur.len {
            self.eng.idx += 1;
            self.eng.t = 0;

            // Run any hooks that follow this token, skipping conditional tokens when cond is false
            while self.eng.idx < self.eng.plan.len() {
                let next = &self.eng.plan[self.eng.idx];
                if next.is_hook() {
                    sem(self, self.eng.table, self.eng.op, next.hook_no());
                    self.eng.idx += 1;
                } else if next.cond && !self.eng.cond {
                    // Skip conditional token (0 T-states)
                    self.eng.idx += 1;
                } else {
                    break;
                }
            }

            // If plan complete, mark as inactive (instruction boundary)
            if self.eng.idx >= self.eng.plan.len() {
                self.eng.active = false;
            }
        }

        out
    }

    /// Total WAIT T-states inserted during the current (or last completed) instruction.
    pub fn wait_states(&self) -> u32 {
        self.eng.ts_wait
    }

    /// T-states of the current (or last completed) instruction, excluding WAIT states.
    pub fn instruction_tstates(&self) -> u32 {
        self.eng.ts
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Cpu {
    /// Start executing a given plan (for testing).
    pub(crate) fn start_plan(&mut self, plan: &'static [Tok]) {
        self.eng.plan = plan;
        self.eng.idx = 0;
        self.eng.t = 0;
        self.eng.active = true;
        self.eng.ts = 0;
        self.eng.ts_wait = 0;
        // Reset cond at instruction boundary (spec: eng.cond reset to false)
        self.eng.cond = false;

        // Run any initial hooks, skipping conditional tokens when cond is false
        while self.eng.idx < self.eng.plan.len() {
            let next = &self.eng.plan[self.eng.idx];
            if next.is_hook() {
                sem(self, self.eng.table, self.eng.op, next.hook_no());
                self.eng.idx += 1;
            } else if next.cond && !self.eng.cond {
                // Skip conditional token (0 T-states)
                self.eng.idx += 1;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::decode::{Prefix, Table};
    use std::boxed::Box;

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

    // E03-07: Tick loop execution tests

    #[test]
    fn e03_07_read_plan_3t() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.start_plan(plan);

        // Tick 1 (t=0): address on bus, no control signals
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x1234);
        assert!(!out1.has(Pins::MREQ | Pins::RD));

        // Tick 2 (t=1): MREQ|RD active
        let out2 = cpu.tick(Pins(0));
        assert!(out2.has(Pins::MREQ | Pins::RD));

        // Tick 3 (t=2): data latched into tmp_l, instruction complete
        let in_data = Pins(0).set_data(0xAB);
        let _out3 = cpu.tick(in_data);
        assert_eq!(cpu.eng.tmp_l, 0xAB);
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_write_plan() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::W, 3, Addr::Hl, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x5678);
        cpu.eng.tmp_l = 0x55;
        cpu.start_plan(plan);

        // Tick 1 (t=0): address and data on bus, no control signals
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x5678);
        assert_eq!(out1.data(), 0x55);
        assert!(!out1.has(Pins::MREQ | Pins::WR));

        // Tick 2 (t=1): MREQ|WR active with data
        let out2 = cpu.tick(Pins(0));
        assert!(out2.has(Pins::MREQ | Pins::WR));
        assert_eq!(out2.data(), 0x55);

        // Tick 3 (t=2): instruction complete
        let _out3 = cpu.tick(Pins(0));
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_internal_plan() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(
            Kind::I,
            5,
            Addr::None,
            Reg::None,
            false,
        )]));
        let mut cpu = Cpu::new();
        cpu.start_plan(plan);

        // Internal cycle: no control signals for all 5 ticks
        for _ in 0..4 {
            let out = cpu.tick(Pins(0));
            assert!(!out.has(Pins::MREQ | Pins::IORQ | Pins::RD | Pins::WR));
            assert!(!cpu.at_instruction_boundary());
        }
        // Last tick completes the instruction
        let _out5 = cpu.tick(Pins(0));
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_two_cycles() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([
            Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false),
            Tok::new(Kind::R, 3, Addr::Bc, Reg::H, false),
        ]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_bc(0x2000);
        cpu.start_plan(plan);

        // First cycle: 3 ticks
        for _ in 0..3 {
            let _out = cpu.tick(Pins(0));
            assert!(!cpu.at_instruction_boundary());
        }

        // Second cycle: 3 more ticks, different address
        let out4 = cpu.tick(Pins(0).set_data(0x11));
        assert_eq!(out4.addr(), 0x2000);
        for _ in 0..1 {
            let _out = cpu.tick(Pins(0));
            assert!(!cpu.at_instruction_boundary());
        }
        // After 6 ticks total, instruction complete
        let _out7 = cpu.tick(Pins(0).set_data(0x22));
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_hook_runs_after() {
        use crate::engine::plan::{Addr, Kind, Reg};
        // Plan: R3 followed by hook X0
        let plan = Box::leak(Box::new([
            Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false),
            Tok::new(Kind::X, 0, Addr::None, Reg::None, false),
        ]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.start_plan(plan);

        // Tick through the R3 cycle (3 ticks)
        for _ in 0..2 {
            cpu.tick(Pins(0));
        }
        // On tick 3, the hook should run after pins_out is computed
        let _out = cpu.tick(Pins(0).set_data(0x42));
        assert_eq!(cpu.eng.tmp_l, 0x42);
        // Hook ran, so idx advanced past it; instruction complete
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_io_plan() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::N, 4, Addr::An, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x12;
        cpu.eng.tmp_l = 0x34;
        cpu.start_plan(plan);

        // Tick 1 (t=0): port address on bus
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x1234);
        assert!(!out1.has(Pins::IORQ | Pins::RD));

        // Tick 2 (t=1): still just address
        let out2 = cpu.tick(Pins(0));
        assert!(!out2.has(Pins::IORQ | Pins::RD));

        // Tick 3 (t=2): IORQ|RD active (+TW auto)
        let out3 = cpu.tick(Pins(0));
        assert!(out3.has(Pins::IORQ | Pins::RD));

        // Tick 4 (t=3): data latched, instruction complete
        let in_data = Pins(0).set_data(0x99);
        let _out4 = cpu.tick(in_data);
        assert_eq!(cpu.eng.tmp_l, 0x99);
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_boundary_after_last() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(
            Kind::I,
            2,
            Addr::None,
            Reg::None,
            false,
        )]));
        let mut cpu = Cpu::new();
        cpu.start_plan(plan);

        assert!(!cpu.at_instruction_boundary());
        cpu.tick(Pins(0));
        assert!(!cpu.at_instruction_boundary());
        cpu.tick(Pins(0));
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_07_xy_plan() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::R, 3, Addr::Xy, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.ix = 0x1000;
        cpu.eng.prefix = Prefix::Dd;
        cpu.eng.disp = 0xFE; // -2 signed
        cpu.start_plan(plan);

        // Tick 1: address should be IX + disp = 0x0FFE, and wz updated
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x0FFE);
        assert_eq!(cpu.regs.wz, 0x0FFE);

        // Complete the cycle
        cpu.tick(Pins(0));
        let in_data = Pins(0).set_data(0xCC);
        cpu.tick(in_data);
        assert_eq!(cpu.eng.tmp_l, 0xCC);
    }

    // E03-08: Conditional tokens (?) tests

    #[test]
    fn e03_08_cond_false_skips() {
        use crate::engine::plan::{Addr, Kind, Reg};
        // Plan: R3, ?I5 (conditional), R3 - cond=false so I5 is skipped
        let plan = Box::leak(Box::new([
            Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false),
            Tok::new(Kind::I, 5, Addr::None, Reg::None, true), // conditional
            Tok::new(Kind::R, 3, Addr::Bc, Reg::H, false),
        ]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_bc(0x2000);
        cpu.start_plan(plan);

        // First R3: 3 ticks
        for _ in 0..3 {
            let _out = cpu.tick(Pins(0));
        }
        // After first token, hooks run (none), then conditional I5 is skipped (cond=false)
        assert!(!cpu.at_instruction_boundary());

        // Second R3: 3 more ticks
        for _ in 0..3 {
            let _out = cpu.tick(Pins(0));
        }
        assert!(cpu.at_instruction_boundary());
        // Total: 6 T-states (I5 skipped)
        assert_eq!(cpu.eng.ts, 6);
    }

    #[test]
    fn e03_08_cond_true_runs() {
        use crate::engine::plan::{Addr, Kind, Reg};
        // Same plan but cond set to true before the conditional token is reached
        let plan = Box::leak(Box::new([
            Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false),
            Tok::new(Kind::I, 5, Addr::None, Reg::None, true), // conditional
            Tok::new(Kind::R, 3, Addr::Bc, Reg::H, false),
        ]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.regs.set_bc(0x2000);
        cpu.start_plan(plan);

        // First R3: tick 1, then set cond=true before token completes
        let _out = cpu.tick(Pins(0));
        cpu.eng.cond = true;
        for _ in 0..2 {
            let _out = cpu.tick(Pins(0));
        }
        // Now I5 should run (cond is true): 5 ticks
        for _ in 0..5 {
            let _out = cpu.tick(Pins(0));
        }

        // Second R3: 3 more ticks
        for _ in 0..3 {
            let _out = cpu.tick(Pins(0));
        }
        assert!(cpu.at_instruction_boundary());
        // Total: 11 T-states (I5 executed)
        assert_eq!(cpu.eng.ts, 11);
    }

    #[test]
    fn e03_08_cond_reset() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(
            Kind::I,
            2,
            Addr::None,
            Reg::None,
            false,
        )]));
        let mut cpu = Cpu::new();

        // First instruction: set cond=true during execution
        cpu.start_plan(plan);
        cpu.eng.cond = true;
        for _ in 0..2 {
            cpu.tick(Pins(0));
        }
        assert!(cpu.at_instruction_boundary());
        assert!(cpu.eng.cond);

        // Start second instruction - cond should be reset to false by start_plan
        cpu.start_plan(plan);
        assert!(!cpu.eng.cond);
    }

    #[test]
    fn e03_08_djnz_shape() {
        use crate::engine::plan::{Addr, Kind, Reg};
        // DJNZ shape: F5 R3 X0 ?I5 X1 (simplified for test)
        // Without cond: 5+3 = 8 ticks (skip I5 and hook contributes 0)
        let plan = Box::leak(Box::new([
            Tok::new(Kind::F, 5, Addr::PcInc, Reg::None, false),
            Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false),
            Tok::new(Kind::X, 0, Addr::None, Reg::None, false), // hook X0 (sets cond)
            Tok::new(Kind::I, 5, Addr::None, Reg::None, true),  // conditional I5
            Tok::new(Kind::X, 1, Addr::None, Reg::None, false), // hook X1
        ]));

        // Test with cond=false (hook doesn't set it) - should skip I5: 8 ticks
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1000);
        cpu.start_plan(plan);
        for _ in 0..8 {
            let _out = cpu.tick(Pins(0));
        }
        assert!(cpu.at_instruction_boundary());
        assert_eq!(cpu.eng.ts, 8);

        // Test with cond=true (hook sets it) - should run I5: 13 ticks
        let mut cpu2 = Cpu::new();
        cpu2.regs.set_hl(0x1000);
        cpu2.start_plan(plan);
        // Run through F5 (5 ticks), then during R3 set cond=true before R3 completes
        for _ in 0..6 {
            let _out = cpu2.tick(Pins(0));
        }
        // Set cond=true to simulate X0 hook setting it (before R3 finishes)
        cpu2.eng.cond = true;
        // Complete R3 (1 more tick), then I5 runs (5 ticks): 6 more ticks total
        for _ in 0..7 {
            let _out = cpu2.tick(Pins(0));
        }
        assert!(cpu2.at_instruction_boundary());
        assert_eq!(cpu2.eng.ts, 13);
    }

    // E03-09: WAIT state insertion tests

    #[test]
    fn e03_09_wait_extends_read() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.start_plan(plan);

        // Tick 1 (t=0): address on bus, no WAIT sampling
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x1234);

        // Tick 2 (t=1): MREQ|RD active, WAIT sampled here - apply WAIT for 2 ticks
        let out2 = cpu.tick(Pins(Pins::WAIT));
        assert!(out2.has(Pins::MREQ | Pins::RD));
        assert_eq!(cpu.wait_states(), 1);

        // Tick 3 (t=1 still): WAIT still active, another wait state inserted
        let out3 = cpu.tick(Pins(Pins::WAIT));
        assert!(out3.has(Pins::MREQ | Pins::RD));
        assert_eq!(cpu.wait_states(), 2);

        // Tick 4 (t=1 now advances): WAIT released, normal progression resumes
        let out4 = cpu.tick(Pins(0));
        assert!(out4.has(Pins::MREQ | Pins::RD));

        // Tick 5 (t=2): data latched, instruction complete
        let in_data = Pins(0).set_data(0xAB);
        let _out5 = cpu.tick(in_data);
        assert_eq!(cpu.eng.tmp_l, 0xAB);
        assert!(cpu.at_instruction_boundary());

        // Total: 3 normal T-states + 2 wait states = 5 ticks
        assert_eq!(cpu.wait_states(), 2);
    }

    #[test]
    fn e03_09_wait_not_in_internal() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::I, 4, Addr::None, Reg::None, false)]));
        let mut cpu = Cpu::new();
        cpu.start_plan(plan);

        // Internal cycle: WAIT pin is ignored for all 4 ticks
        for _ in 0..3 {
            let out = cpu.tick(Pins(Pins::WAIT));
            assert!(!out.has(Pins::MREQ | Pins::IORQ | Pins::RD | Pins::WR));
        }
        let _out4 = cpu.tick(Pins(0));
        assert!(cpu.at_instruction_boundary());

        // No wait states inserted for internal cycle
        assert_eq!(cpu.wait_states(), 0);
    }

    #[test]
    fn e03_09_wait_io() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::N, 4, Addr::An, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.a = 0x12;
        cpu.eng.tmp_l = 0x34;
        cpu.start_plan(plan);

        // Tick 1 (t=0): port address on bus
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x1234);

        // Tick 2 (t=1): still just address, no WAIT sampling for N at t=1
        let out2 = cpu.tick(Pins(Pins::WAIT));
        assert!(!out2.has(Pins::IORQ | Pins::RD));

        // Tick 3 (t=2): IORQ|RD active (+TW auto), WAIT sampled here -> wait state inserted
        let out3 = cpu.tick(Pins(Pins::WAIT));
        assert!(out3.has(Pins::IORQ | Pins::RD));
        assert_eq!(cpu.wait_states(), 1);

        // Tick 4 (t=2 still): WAIT released, advance to t=3
        let _out4 = cpu.tick(Pins(0));

        // Tick 5 (t=3): data latched here, then advance to t=4 -> instruction complete
        let in_data = Pins(0).set_data(0x99);
        let _out5 = cpu.tick(in_data);
        assert_eq!(cpu.eng.tmp_l, 0x99);
        assert!(cpu.at_instruction_boundary());

        assert_eq!(cpu.wait_states(), 1);
    }

    #[test]
    fn e03_09_outputs_repeat() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.start_plan(plan);

        // Tick 1 (t=0): address on bus
        let out1 = cpu.tick(Pins(0));
        assert_eq!(out1.addr(), 0x1234);

        // Tick 2 (t=1): MREQ|RD active - capture this output
        let out_normal = cpu.tick(Pins(Pins::WAIT));
        assert!(out_normal.has(Pins::MREQ | Pins::RD));
        assert_eq!(out_normal.addr(), 0x1234);

        // Tick 3 (t=1 still): WAIT active, outputs must be identical to tick 2
        let out_wait = cpu.tick(Pins(Pins::WAIT));
        assert_eq!(out_wait.0, out_normal.0); // Exact same pin values
        assert!(out_wait.has(Pins::MREQ | Pins::RD));
        assert_eq!(out_wait.addr(), 0x1234);

        // Release WAIT and complete instruction
        cpu.tick(Pins(0));
        let in_data = Pins(0).set_data(0xAB);
        cpu.tick(in_data);
    }

    #[test]
    fn e03_09_no_wait_total() {
        use crate::engine::plan::{Addr, Kind, Reg};
        let plan = Box::leak(Box::new([Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false)]));
        let mut cpu = Cpu::new();
        cpu.regs.set_hl(0x1234);
        cpu.start_plan(plan);

        // Tick through with WAIT inserted at t=1 for 2 ticks
        cpu.tick(Pins(0));           // t=0
        cpu.tick(Pins(Pins::WAIT));        // t=1, wait state 1
        cpu.tick(Pins(Pins::WAIT));        // t=1, wait state 2
        cpu.tick(Pins(0));           // t=1 advances to t=2
        let in_data = Pins(0).set_data(0xAB);
        cpu.tick(in_data);           // t=2 completes instruction

        assert!(cpu.at_instruction_boundary());
        // instruction_tstates excludes wait states: should be 3 (the R3 token length)
        assert_eq!(cpu.instruction_tstates(), 3);
        // wait_states counts only the inserted waits
        assert_eq!(cpu.wait_states(), 2);
    }
}
