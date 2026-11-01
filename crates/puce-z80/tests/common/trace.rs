//! Pin trace of one instruction, one entry per T-state (debug tool).

use puce_z80::{Bus, Cpu, Pins, SimpleRam};

/// Run one instruction made of `bytes` (placed at address 0) on a test RAM and
/// return the output pins of every tick. `setup` can prepare the CPU first.
pub fn trace_instruction(bytes: &[u8], setup: impl Fn(&mut Cpu)) -> Vec<Pins> {
    let mut ram = SimpleRam::new();
    ram.mem[..bytes.len()].copy_from_slice(bytes);
    let mut cpu = Cpu::new();
    setup(&mut cpu);

    let mut trace = Vec::new();
    let mut last = Pins(0);
    let mut write_done = false;
    loop {
        let mut pins_in = Pins(0);
        if last.has(Pins::MREQ) && last.has(Pins::RD) {
            pins_in = pins_in.set_data(ram.read(last.addr()));
        }
        if last.has(Pins::MREQ) && last.has(Pins::WR) {
            if !write_done {
                ram.write(last.addr(), last.data());
                write_done = true;
            }
        } else {
            write_done = false;
        }
        last = cpu.tick(pins_in);
        trace.push(last);
        if cpu.at_instruction_boundary() || trace.len() > 64 {
            break;
        }
    }
    trace
}

/// Format a trace, one line per T-state, for example `T3 A=0001 D=34 M1 MREQ RD`.
pub fn fmt_trace(trace: &[Pins]) -> String {
    let flags = [
        (Pins::M1, "M1"),
        (Pins::MREQ, "MREQ"),
        (Pins::IORQ, "IORQ"),
        (Pins::RD, "RD"),
        (Pins::WR, "WR"),
        (Pins::RFSH, "RFSH"),
        (Pins::HALT, "HALT"),
        (Pins::WAIT, "WAIT"),
    ];
    let mut lines = Vec::new();
    for (i, p) in trace.iter().enumerate() {
        let mut line = format!("T{} A={:04X} D={:02X}", i + 1, p.addr(), p.data());
        for (mask, name) in flags {
            if p.has(mask) {
                line.push(' ');
                line.push_str(name);
            }
        }
        lines.push(line);
    }
    lines.join("\n")
}
