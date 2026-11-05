//! The loop of `examples/pins_host.rs` (copied), without printing.

use puce_z80::{Cpu, Pins};

const PROGRAM: [u8; 4] = [0x3E, 0x01, 0x87, 0x76];

/// Run `instructions` instructions from address 0; returns the CPU and the T-states used.
fn run(instructions: u32) -> (Cpu, u32) {
    let mut ram = [0u8; 65536];
    ram[..PROGRAM.len()].copy_from_slice(&PROGRAM);
    let mut cpu = Cpu::new();
    let mut out = Pins(0);
    let mut ticks = 0;
    let mut done = 0;
    while done < instructions {
        let mut inp = Pins(0);
        if out.has(Pins::MREQ) && out.has(Pins::RD) {
            inp = inp.set_data(ram[usize::from(out.addr())]);
        }
        if out.has(Pins::MREQ) && out.has(Pins::WR) {
            ram[usize::from(out.addr())] = out.data();
        }
        out = cpu.tick(inp);
        ticks += 1;
        if cpu.at_instruction_boundary() {
            done += 1;
        }
    }
    (cpu, ticks)
}

#[test]
fn e16_01_a_equals_2() {
    let (cpu, _) = run(3);
    assert_eq!(cpu.regs.af() >> 8, 2);
}

#[test]
fn e16_01_tick_count() {
    // LD A,n = 7 T ; ADD A,A = 4 T ; HALT = 4 T
    let (_, ticks) = run(3);
    assert_eq!(ticks, 7 + 4 + 4);
}
