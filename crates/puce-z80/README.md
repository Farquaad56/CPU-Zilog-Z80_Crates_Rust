# puce-z80

Cycle-accurate Zilog Z80 CPU emulator core, for consoles, computers and arcade boards.

## Guarantees

- `#![no_std]`, no allocation, `#![forbid(unsafe_code)]`, no dependency.
- Deterministic: a `Cpu` owns all of its state. No global, no `static mut`, no internal clock.
  Several CPUs can live in the same program (arcade boards with 2 or 3 Z80).
- NMOS Z80 behaviour, including the undocumented flags (X/Y), `WZ` (MEMPTR) and `Q`
  (SCF/CCF), the block instructions and the interrupt corner cases.
- T-state accurate: one call to `tick` is one clock period, bus cycles and wait states included.

## Installation

```toml
[dependencies]
puce-z80 = { git = "https://github.com/Farquaad56/CPU-Zilog-Z80_Crates_Rust", package = "puce-z80" }
```

## Easy API: `step` and a `Bus`

You implement `Bus` (memory is mandatory, everything else has a default), then call `step`
to run one instruction. It returns the T-states used, wait states included.

```rust
use puce_z80::{Bus, Cpu};

struct Machine {
    ram: Box<[u8; 65536]>,
}

impl Bus for Machine {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }
    fn write(&mut self, addr: u16, data: u8) {
        self.ram[usize::from(addr)] = data;
    }
    // Optional: io_read, io_write, int_ack, irq, nmi, busrq, extra_wait, on_tick.
}

fn main() {
    let mut m = Machine { ram: Box::new([0; 65536]) };
    // LD A,1 ; ADD A,A ; HALT
    m.ram[..4].copy_from_slice(&[0x3E, 0x01, 0x87, 0x76]);

    let mut cpu = Cpu::new();
    let mut tstates = 0;
    for _ in 0..3 {
        tstates += cpu.step(&mut m);
    }
    assert_eq!(cpu.regs.a, 2);
    assert_eq!(tstates, 15);
}
```

`Bus` hooks used by real machines:

| Hook | Use |
|---|---|
| `extra_wait(kind, addr)` | memory contention (ZX Spectrum), arcade WAIT states |
| `on_tick(pins)` | clock a video chip, a sound chip or a CTC once per T-state |
| `irq()`, `nmi()`, `int_ack()` | INT/NMI lines and the vector byte put on the bus (IM 0/1/2) |
| `busrq()` | shared buses (Mega Drive: another CPU takes the Z80 bus) |

## Pin API: `tick`

One call is one T-state. The CPU only shows pins; the host reacts in the next tick.

```rust
use puce_z80::{Cpu, Pins};

let mut ram = [0u8; 65536];
ram[..4].copy_from_slice(&[0x3E, 0x01, 0x87, 0x76]); // LD A,1 ; ADD A,A ; HALT
let mut cpu = Cpu::new();
let mut out = Pins(0);
for _ in 0..15 {
    let mut inp = Pins(0);
    if out.has(Pins::MREQ) && out.has(Pins::RD) {
        inp = inp.set_data(ram[usize::from(out.addr())]); // memory read
    }
    if out.has(Pins::MREQ) && out.has(Pins::WR) {
        ram[usize::from(out.addr())] = out.data(); // memory write
    }
    out = cpu.tick(inp);
}
assert_eq!(cpu.regs.a, 2);
```

Input pins: data, `WAIT`, `INT`, `NMI`, `BUSRQ`, `RESET`. Output pins: address, `M1`, `MREQ`,
`IORQ`, `RD`, `WR`, `HALT`, `RFSH`, `BUSAK`.

## Features

| Feature | Effect |
|---|---|
| `std` | `SimpleRam::boxed()` |
| `inspect` | last ALU operation, bus journal, for debuggers |
| `disasm` | instruction disassembler (`puce_z80::disasm`) |

## Examples (consoles and arcade boards)

```text
cargo run -q -p puce-z80 --example pins_host      # lowest level: the host serves memory pin by pin
cargo run -q -p puce-z80 --example contended_bus  # ZX-like memory contention and on_tick
cargo run -q -p puce-z80 --example shared_bus     # BUSRQ/BUSAK and RESET (Mega Drive style)
cargo run -q -p puce-z80 --example im2_ctc        # IM 2, vector from a peripheral, RETI snooping
```

The workspace also holds a CP/M machine (`puce-z80-machines`), a command line tool (`z80`)
and a browser front end (`z80-web`).

## Validated test suites

| Suite | Result |
|---|---|
| PRELIM | passes |
| ZEXDOC | 67 / 67 OK |
| ZEXALL | 67 / 67 OK (undocumented flags included) |
| z80test (Patrik Rak) | 6 / 6 programs pass |
| SingleStepTests/z80 | 1,604,000 / 1,604,000 cases (state and bus cycles) |
| FUSE | 1356 / 1356 tests; 6 documented exceptions where FUSE and the silicon differ |

Speed: about 12 million instructions per second in release mode on the CP/M exercisers.

## Status

Version 0.1.x. The public API may change until 1.0. The NMOS (default) and CMOS variants are available (`Cpu::with_variant`); the feature `serde` adds save states (`Cpu::save_state` / `Cpu::from_state`).

## License

Licensed under either of MIT or Apache-2.0, at your option.
