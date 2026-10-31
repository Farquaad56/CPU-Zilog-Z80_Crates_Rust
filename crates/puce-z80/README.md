# puce-z80

Cycle-accurate Zilog Z80 CPU emulator core.

- `#![no_std]`, no allocation, `#![forbid(unsafe_code)]`
- No runtime dependencies
- Pin-level interface: the host drives the CPU one T-state at a time with
  `tick(Pins) -> Pins` and wires its own memory and I/O to the bus pins
- Optional features: `std`, `inspect` (register/state access), `disasm`

## Status

Work in progress (0.1.x). The pin-level engine is being built and validated
against the public Z80 test ROMs. The public API may change until 1.0.

## Usage

```toml
[dependencies]
puce-z80 = { git = "https://github.com/Farquaad56/CPU-Zilog-Z80_Crates_Rust", package = "puce-z80" }
```

## License

Licensed under either of MIT or Apache-2.0, at your option.