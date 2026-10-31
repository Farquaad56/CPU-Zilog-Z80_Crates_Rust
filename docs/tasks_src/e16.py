from _dsl import T

R32 = "32_REUSE_GUIDE.md#32 -"

T("E16-01", "Exemple: hote minimal a broches (tick)", kind="code", size=90,
  context="""Montre l'API la plus bas niveau : l'hote fait lui-meme la memoire avec `tick`.""",
  embed=[R32, "11_PINS_AND_TICK.md#2. API"],
  spec="""`crates/puce-z80/examples/pins_host.rs` (std autorise dans les exemples) : RAM de 64 Ko, programme `LD A,1; ADD A,A; HALT`, boucle `let out = cpu.tick(inp)` qui sert la memoire comme dans la section 2 (`MREQ|RD` -> donnee au tick suivant, `MREQ|WR` -> ecriture), affiche chaque T-state. Test d'integration `tests/examples_pins.rs` qui reprend la meme boucle (copiee) et verifie A==2.""",
  write=["crates/puce-z80/examples/pins_host.rs", "crates/puce-z80/tests/examples_pins.rs"],
  tests=[("a_equals_2", "A vaut 2 apres 3 instructions"), ("tick_count", "nombre de ticks = 7 + 4 + 4 (LD A,n 7T ; ADD A,A 4T ; HALT 4T)")],
  accept=["cargo run -q -p puce-z80 --example pins_host", "cargo test -p puce-z80 e16_01_"])
T("E16-02", "Exemple: machine ZX-like avec contention", kind="code", size=100,
  context="Bus avec `extra_wait` (contention memoire) et `on_tick` pour cadencer un peripherique (consoles/ordinateurs 8 bits).",
  embed=[R32],
  spec="""`examples/contended_bus.rs` : un `Bus` dont `extra_wait(Cycle::MemRead|MemWrite|Fetch, addr)` renvoie 0..6 pour 0x4000..=0x7FFF selon le T-state de ligne (compteur incremente par `on_tick`), et un 'peripherique' (compteur de lignes) incremente dans `on_tick`. Test `tests/examples_contention.rs` : le meme programme dure PLUS de T-states quand il s'execute dans 0x4000 que dans 0x8000 ; et `on_tick` est appele exactement une fois par T-state (y compris les T d'attente).""",
  write=["crates/puce-z80/examples/contended_bus.rs", "crates/puce-z80/tests/examples_contention.rs"],
  tests=[("slower_in_contended", "temps(0x4000) > temps(0x8000)"), ("on_tick_every_t", "on_tick compte tous les T"), ("wait_not_in_instruction_tstates", "instruction_tstates ignore l'attente")],
  accept=["cargo run -q -p puce-z80 --example contended_bus", "cargo test -p puce-z80 e16_02_"])
T("E16-03", "Exemple: bus partage (BUSRQ/BUSAK) et RESET", kind="code", size=100,
  context="Systeme type Mega Drive : un autre processeur prend le bus et remet le Z80 a zero.",
  embed=[R32],
  spec="""`examples/shared_bus.rs` : un 'CPU maitre' factice leve `busrq()` pendant N ticks, ecrit en RAM pendant que `BUSAK` est actif, puis rend le bus ; puis assert RESET. Test `tests/examples_shared_bus.rs` : le Z80 ne progresse pas pendant BUSAK, reprend ensuite, et apres RESET PC==0.""",
  write=["crates/puce-z80/examples/shared_bus.rs", "crates/puce-z80/tests/examples_shared_bus.rs"],
  tests=[("frozen_during_busak", "PC fige"), ("resumes", "reprend et termine l'instruction"), ("reset_pc_zero", "PC==0 apres RESET")],
  accept=["cargo run -q -p puce-z80 --example shared_bus", "cargo test -p puce-z80 e16_03_"])
T("E16-04", "Exemple: IM2, vecteur et daisy chain (CTC factice)", kind="code", size=120,
  context="Modes d'interruption des bornes d'arcade : IM2 avec vecteur fourni par un peripherique ; RETI observe par le peripherique.",
  embed=[R32, "22_INTERRUPTS.md#3. RETI"],
  spec="""`examples/im2_ctc.rs` : un CTC factice leve `irq()` periodiquement, fournit un vecteur via `int_ack()` et surveille la sequence `ED 4D` (RETI) sur le bus (`on_tick`) pour rearmer sa ligne `IEO`. Programme de test : IM 2, I=0x80, table de vecteurs en 0x8000, handler incremente un compteur. Test `tests/examples_im2.rs` : apres 10 interruptions, compteur == 10, et le CTC a vu 10 RETI.""",
  write=["crates/puce-z80/examples/im2_ctc.rs", "crates/puce-z80/tests/examples_im2.rs"],
  tests=[("ten_irqs", "compteur == 10"), ("reti_seen", "10 RETI vus par le peripherique"), ("vector_from_table", "le handler appele est celui de la table")],
  accept=["cargo run -q -p puce-z80 --example im2_ctc", "cargo test -p puce-z80 e16_04_"])
T("E16-05", "Plusieurs CPU, determinisme et no_std", kind="code", size=110,
  context="Bornes d'arcade a plusieurs Z80 (Galaga) ; portabilite embarquee/WASM.",
  spec="""- Test `tests/multi_cpu.rs` : deux `Cpu` avec deux RAM executent le meme programme entrelaces tick par tick (A puis B) : resultats identiques a deux executions separees ; aucun `static mut`/etat global (grep dans `src/`).
- Test de determinisme : deux executions de 1 million de T-states sur le meme programme donnent le meme hachage d'etat.
- Script `tools/nostd_check.sh` : `rustup target add thumbv7em-none-eabihf` puis `cargo build -p puce-z80 --target thumbv7em-none-eabihf` (SKIP avec message si la cible n'est pas installable: pas de reseau) ; en plus verifie `cargo build -p puce-z80 --target wasm32-unknown-unknown` si la cible est installee.""",
  write=["crates/puce-z80/tests/multi_cpu.rs", "tools/nostd_check.sh"],
  tests=[("two_cpus_interleaved", "entrelaces == separes"), ("deterministic_hash", "hachage stable"), ("no_global_state", "aucun static mut dans src")],
  accept=["cargo test -p puce-z80 e16_05_", "bash tools/nostd_check.sh"])
T("E16-06", "Documentation du crate (rustdoc + README)", kind="doc", size=0, assignee="hermes",
  context="Un autre projet doit pouvoir utiliser le crate sans lire ce depot.",
  spec="""Ecris `crates/puce-z80/README.md` (anglais, ASCII) : presentation, garanties (no_std, no alloc, no unsafe, deterministe, NMOS), installation (`puce-z80 = { git = ... }`), exemple `Bus`+`step` de 20 lignes, exemple `tick`, features (`inspect`, `disasm`, `std`), integration console/arcade (renvoi vers les exemples), suites de test validees (tableau). Complete la documentation rustdoc des types publics (`#![deny(missing_docs)]` doit passer) et ajoute `//!` en tete de lib.rs (vue d'ensemble + doctest compilable utilisant `SimpleRam`).""",
  write=["crates/puce-z80/README.md", "crates/puce-z80/src/**"],
  accept=["cargo doc -p puce-z80 --all-features --no-deps", "cargo test -p puce-z80 --doc"])
T("E16-07", "GATE E16 - reutilisation validee", gate=True, kind="gate", size=0,
  context="Les exemples couvrent les 4 familles de machines visees (CP/M, ZX contendu, bus partage, IM2/CTC) et le crate est documente.",
  user_cmds=["cargo test -p puce-z80 --all-features", "cargo doc -p puce-z80 --all-features --no-deps --open", "bash tools/nostd_check.sh"], accept=[])
