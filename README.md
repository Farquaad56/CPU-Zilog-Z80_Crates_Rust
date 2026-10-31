# CPU-Zilog-Z80_Crates_Rust

Reusable, cycle-accurate Zilog Z80 emulator written in Rust.

| Crate | Role | Published |
|-------|------|-----------|
| `puce-z80` | CPU core (`no_std`, no unsafe, no dependencies) | yes (planned) |
| `puce-z80-machines` | Test machines (CP/M, ZX Spectrum) and sessions | no |
| `puce-z80-cli` | Command-line tool (`z80`) | no |
| `puce-z80-web` | Web frontend server (`z80-web`) | no |

Status: work in progress. See `docs/` for the plan, decisions and task list.

## Build and check

```powershell
cargo build --workspace
cargo test --workspace
powershell -File tools\ci.ps1
```

## License

Licensed under either of MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`), at your option.