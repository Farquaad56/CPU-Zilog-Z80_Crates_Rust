# puce-z80-wasm

The `puce-z80` core running in the browser, with no server.

```
rustup target add wasm32-unknown-unknown
cargo build -p puce-z80-wasm --target wasm32-unknown-unknown --release
cargo install wasm-bindgen-cli        # once
wasm-bindgen --target web --out-dir crates/puce-z80-wasm/demo/pkg target/wasm32-unknown-unknown/release/puce_z80_wasm.wasm
```

Then serve `crates/puce-z80-wasm/demo` over HTTP and open `index.html`.
The `Z80` class has `load_demo`, `load(addr, bytes)`, `step`, `run(tstates)`, `regs`, `peek`, `take_output`.
`OUT (0),A` writes a character to the output text.