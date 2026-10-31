from _dsl import T

T("E17-01", "CI de release et changelog", kind="tooling", size=60,
  context="Preparer la version 0.1.0.",
  spec="""- `tools/release_check.sh` : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `bash tools/hygiene.sh`, `cargo build --release --workspace`, `cargo run --release -p puce-z80-cli -- suite all --json suites.json`, `python docs/tools/suites_report.py suites.json`.
- `CHANGELOG.md` (anglais) : v0.1.0, fonctionnalites, suites validees (copie du tableau de `docs/SUITES_REPORT.md`).
- Verifie `crates/puce-z80/Cargo.toml` : `description`, `license`, `keywords`, `categories` (`emulators`, `no-std`), `repository` laisse vide (UNKNOWN - to confirm, a remplir par l'utilisateur).""",
  write=["tools/release_check.sh", "CHANGELOG.md", "crates/puce-z80/Cargo.toml"], tests=[],
  accept=["bash tools/hygiene.sh", "test -s CHANGELOG.md"])
T("E17-02", "GATE FINAL - v0.1.0", gate=True, kind="gate", size=0,
  context="""Validation finale par l'utilisateur : toutes les suites de test (PRELIM, ZEXDOC, ZEXALL, z80test, SingleStepTests/z80, FUSE, timing) passent, le frontend est connecte au crate, les exemples de reutilisation fonctionnent. L'utilisateur etiquette `v0.1.0`.""",
  user_cmds=["bash tools/release_check.sh", "(ouvrir docs/SUITES_REPORT.md : toutes les suites obligatoires PASS)", "git tag v0.1.0"], accept=[])
T("E17-03", "DIFFEREE: variante CMOS", deferred=True, kind="code", size=120,
  context="Hors v1 (D-006). Variante CMOS du Z80 (ex. Zilog Z84C00) : `OUT (C),0` ecrit 0xFF, SCF/CCF sans facteur Q, pas de bug `LD A,I`, `LD A,I` etc.",
  spec="Ajouter `enum Variant { Nmos, Cmos }` et `Cpu::with_variant(v)` ; les differences sont localisees dans `sem/**` et `int.rs`. A specifier par Claude apres v0.1.0.",
  write=["crates/puce-z80/src/**"], tests=[("cmos_out_c_ff", "OUT (C),0 ecrit 0xFF en CMOS")], accept=["cargo test -p puce-z80 e17_03_"])
T("E17-04", "DIFFEREE: serialisation de l'etat (save states)", deferred=True, kind="code", size=100,
  context="Hors v1. Feature `serde` (optionnelle) derivant Serialize/Deserialize pour `Regs` et l'etat du moteur afin de faire des sauvegardes d'etat dans les emulateurs.",
  spec="A specifier apres v0.1.0.", write=["crates/puce-z80/src/**"], tests=[("roundtrip", "Cpu -> JSON -> Cpu identique")], accept=["cargo test -p puce-z80 --features serde e17_04_"])
T("E17-05", "DIFFEREE: cible WASM du moteur (demo navigateur autonome)", deferred=True, kind="code", size=100,
  context="Hors v1 (D-004: page servie par Rust). Compiler `puce-z80` en WASM pour une demo sans serveur.",
  spec="A specifier apres v0.1.0.", write=["crates/puce-z80-wasm/**"], tests=[("builds", "cargo build --target wasm32-unknown-unknown")], accept=["cargo build -p puce-z80 --target wasm32-unknown-unknown"])
