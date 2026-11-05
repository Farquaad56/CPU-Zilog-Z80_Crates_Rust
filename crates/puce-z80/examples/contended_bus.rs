//! A ZX Spectrum-like bus: memory contention and a peripheral clocked by `on_tick`.
//!
//! - `Bus::extra_wait` returns wait states when the CPU touches contended memory
//!   (0x4000..=0x7FFF) during the active part of a video line;
//! - `Bus::on_tick` is called once per T-state (wait states included) and drives the
//!   "video" counter, so contention follows the beam position.
//!
//! Run it with `cargo run -q -p puce-z80 --example contended_bus`.

use puce_z80::{Bus, Cpu, Cycle, Pins};

/// T-states per video line (48K Spectrum).
const LINE_T: u32 = 224;
/// The contention pattern repeats every 8 T-states during the first 128 T of a line.
const PATTERN: [u8; 8] = [6, 5, 4, 3, 2, 1, 0, 0];

struct ContendedBus {
    ram: Box<[u8; 65536]>,
    /// T-state inside the current video line (advanced by `on_tick`).
    line_t: u32,
    /// Peripheral: number of video lines completed.
    lines: u32,
    /// Number of `on_tick` calls.
    ticks: u64,
}

impl ContendedBus {
    fn new() -> Self {
        ContendedBus {
            ram: Box::new([0; 65536]),
            line_t: 0,
            lines: 0,
            ticks: 0,
        }
    }
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

/// `LD A,0 ; INC A x count`, run from `origin`; returns (T-states, bus).
fn run(origin: u16, count: usize) -> (u32, ContendedBus) {
    let mut bus = ContendedBus::new();
    let mut at = usize::from(origin);
    bus.ram[at] = 0x3E; // LD A,0
    at += 2;
    for _ in 0..count {
        bus.ram[at] = 0x3C; // INC A
        at += 1;
    }
    let mut cpu = Cpu::new();
    cpu.regs.pc = origin;
    let mut tstates = 0;
    for _ in 0..=count {
        tstates += cpu.step(&mut bus);
    }
    (tstates, bus)
}

fn main() {
    let count = 200;
    let (fast, bus_fast) = run(0x8000, count);
    let (slow, bus_slow) = run(0x4000, count);
    println!(
        "uncontended (0x8000): {:5} T-states, {} lines, {} on_tick calls",
        fast, bus_fast.lines, bus_fast.ticks
    );
    println!(
        "contended   (0x4000): {:5} T-states, {} lines, {} on_tick calls",
        slow, bus_slow.lines, bus_slow.ticks
    );
    println!("contention cost: {} T-states", slow - fast);
}
