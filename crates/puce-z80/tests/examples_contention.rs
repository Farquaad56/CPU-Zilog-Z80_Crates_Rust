//! The bus of `examples/contended_bus.rs` (copied), checked.

use puce_z80::{Bus, Cpu, Cycle, Pins};

const LINE_T: u32 = 224;
const PATTERN: [u8; 8] = [6, 5, 4, 3, 2, 1, 0, 0];

struct ContendedBus {
    ram: Box<[u8; 65536]>,
    line_t: u32,
    lines: u32,
    ticks: u64,
}

impl Bus for ContendedBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }
    fn write(&mut self, addr: u16, data: u8) {
        self.ram[usize::from(addr)] = data;
    }
    fn extra_wait(&mut self, kind: Cycle, addr: u16) -> u8 {
        let memory = matches!(kind, Cycle::Fetch | Cycle::MemRead | Cycle::MemWrite);
        if memory && (0x4000..=0x7FFF).contains(&addr) && self.line_t < 128 {
            PATTERN[(self.line_t % 8) as usize]
        } else {
            0
        }
    }
    fn on_tick(&mut self, _pins: Pins) {
        self.ticks += 1;
        self.line_t += 1;
        if self.line_t == LINE_T {
            self.line_t = 0;
            self.lines += 1;
        }
    }
}

/// `LD A,0 ; INC A x count` from `origin`; returns (T-states, nominal T-states, bus).
fn run(origin: u16, count: usize) -> (u32, u32, ContendedBus) {
    let mut bus = ContendedBus {
        ram: Box::new([0; 65536]),
        line_t: 0,
        lines: 0,
        ticks: 0,
    };
    let mut at = usize::from(origin);
    bus.ram[at] = 0x3E;
    at += 2;
    for _ in 0..count {
        bus.ram[at] = 0x3C;
        at += 1;
    }
    let mut cpu = Cpu::new();
    cpu.regs.pc = origin;
    let mut total = 0;
    for _ in 0..=count {
        total += cpu.step(&mut bus);
    }
    // `instruction_tstates` and `wait_states` are running totals that do not mix.
    assert_eq!(cpu.instruction_tstates() + cpu.wait_states(), total);
    (total, cpu.instruction_tstates(), bus)
}

#[test]
fn e16_02_slower_in_contended() {
    let (fast, _, _) = run(0x8000, 200);
    let (slow, _, _) = run(0x4000, 200);
    assert!(slow > fast, "contended {} <= uncontended {}", slow, fast);
}

#[test]
fn e16_02_on_tick_every_t() {
    for origin in [0x8000, 0x4000] {
        let (total, _, bus) = run(origin, 200);
        assert_eq!(bus.ticks, u64::from(total), "origin {:04X}", origin);
        assert_eq!(u64::from(bus.lines), bus.ticks / u64::from(LINE_T));
    }
}

#[test]
fn e16_02_wait_not_in_instruction_tstates() {
    let (total, nominal, _) = run(0x4000, 200);
    // 7 T for LD A,n and 4 T for each INC A, whatever the contention.
    assert_eq!(nominal, 7 + 4 * 200);
    assert!(total > nominal, "no wait state was inserted");
}
