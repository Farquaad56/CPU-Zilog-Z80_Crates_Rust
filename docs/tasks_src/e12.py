from _dsl import T

T("E12-01", "Feature inspect: journal de la derniere operation ALU", kind="code", size=110,
  context="""Le frontend affiche l'operation ALU en cours (op, operandes, resultat, drapeaux). Elle doit venir du crate, sous la feature `inspect` (zero cout par defaut, comportement identique).""",
  embed=["31_FRONTEND_PROTOCOL.md#2. Serveur"],
  spec="""Dans `probe.rs` (`#[cfg(feature = "inspect")] pub mod probe;`) : `pub struct AluRecord { pub op: u8 /*0..7 alu8, 8 inc,9 dec, 16.. rot, 32 add16, 33 adc16, 34 sbc16, 40 daa, 41 neg, 42 cpl*/, pub w: u8 /*8 ou 16*/, pub a: u16, pub b: u16, pub c: u8, pub r: u16, pub f_in: u8, pub f_out: u8 }` et `Cpu::last_alu(&self) -> Option<AluRecord>` (champ `#[cfg(feature = "inspect")] pub(crate) last_alu` dans Cpu).
`sem/**` enregistre l'operation via une fonction `probe::alu(cpu, record)` qui est un `#[inline(always)]` vide hors feature. Branche-la dans les appels a `alu8`, `inc8/dec8`, `rot`, `add16/adc16/sbc16`, `daa`, `neg`.""",
  write=["crates/puce-z80/src/probe.rs", "crates/puce-z80/src/lib.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/sem/**"],
  tests=[("add_recorded", "ADD A,B: last_alu() donne op 0, a, b, r, f_out"), ("none_initially", "last_alu() None apres new"), ("inc_recorded", "INC B enregistre op 8"), ("add16_recorded", "ADD HL,DE w=16"), ("behavior_unchanged", "memes resultats avec et sans la feature (diff E05-16 passe avec --features inspect)")],
  accept=["cargo test -p puce-z80 --features inspect e12_01_", "Z80_DIFF_TABLE=all cargo test -p puce-z80 --features inspect --test diff -- e05_01_diff_run"])
T("E12-02", "Feature inspect: historique PC, journal d'ecritures, compteurs", kind="code", size=110,
  context="Donnees d'observation pour le panneau 'trace', la memoire (zones ecrites recemment) et les compteurs.",
  spec="""Dans `probe.rs` : `pub struct History { pc: [u16; 32], head: u8, count: u8 }` (anneau, plus recents d'abord via `iter()`), `pub struct WriteLog { addr: [u16; 32], ... }` (anneau). Sur `Cpu` (feature inspect) : `pub fn pc_history(&self) -> impl Iterator<Item = u16>` (PC de DEBUT des 32 dernieres instructions, plus recente en premier), `pub fn recent_writes(&self) -> impl Iterator<Item = u16>`, `pub fn instruction_count(&self) -> u64`, `pub fn tstate_count(&self) -> u64`. L'historique est mis a jour au debut de chaque instruction non-HALTee; le journal d'ecritures a chaque jeton W reellement emis. Pas d'alloc.""",
  write=["crates/puce-z80/src/probe.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("history_order", "apres 3 NOP: [2,1,0] en premier"), ("history_wraps", "100 NOP: 32 entrees, plus recente d'abord"), ("write_log", "LD (HL),A enregistre l'adresse"), ("counters", "instruction_count et tstate_count apres LD BC,nn: 1 et 10"), ("halt_not_counted", "les NOP de HALT ne polluent pas l'historique")],
  accept=["cargo test -p puce-z80 --features inspect e12_02_"])
T("E12-03", "Feature inspect: dernier cycle de bus", kind="code", size=60,
  context="Le frontend affiche `bus: {addr, data, kind, pins}`.",
  spec="""`Cpu::last_bus(&self) -> BusView { addr: u16, data: u8, kind: u8 /*0 fetch 1 read 2 write 3 io-read 4 io-write*/, pins: Pins }` : dernier cycle de bus COMPLET (mis a jour au tick ou la donnee est latchee/ecrite). Feature `inspect`.""",
  write=["crates/puce-z80/src/probe.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("fetch_view", "apres NOP: kind 0, addr 0"), ("write_view", "apres LD (HL),A: kind 2 avec la donnee"), ("io_view", "apres OUT (n),A: kind 4"), ("read_view", "LD A,(nn): kind 1")],
  accept=["cargo test -p puce-z80 --features inspect e12_03_"])
T("E12-04", "GATE E12 - crate complet (CPU, desassembleur, inspection)", gate=True, kind="gate", size=0,
  context="Le crate expose tout ce que le frontend affichera : etat, mnemoniques, derniere operation ALU, bus, historique.",
  user_cmds=["cargo test -p puce-z80 --all-features", "cargo doc -p puce-z80 --all-features --no-deps", "bash tools/ci.sh"], accept=[])
