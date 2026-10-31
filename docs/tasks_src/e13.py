from _dsl import T

S30 = "30_TEST_SUITES.md#30 -"
UNK = "Tout ce qui est marque `UNKNOWN - to confirm` DOIT etre verifie sur la source reelle (git clone --depth 1 / curl) puis documente dans le fichier indique; si la source est inaccessible depuis le reseau de la machine: question Q&R (docs/02_QA_PROTOCOL.md), ne rien inventer."

T("E13-01", "Telechargement des ROMs de test (fetch_roms)", kind="tooling", size=120, crate="puce-z80-cli",
  context="""Les ROMs ne sont jamais commitees (.gitignore). Un script Python et une sous-commande `z80 fetch` les telechargent dans `tests/roms/` et verifient leurs sommes SHA-256 (`tests/manifest.toml`).""",
  embed=[S30],
  spec="""Cree `docs/tools/fetch_roms.py` (Python 3.8+, stdlib seule: urllib, hashlib, zipfile) :
- Telecharge PRELIM, ZEXDOC, ZEXALL depuis les URL du tableau (octets attendus 1280 / 8588 / 8588) vers `tests/roms/`.
- Sources `UNKNOWN - to confirm` (z80test de Rak, SingleStepTests/z80, FUSE tests, ROMs de timing) : le script a une section `OPTIONAL` par source avec `git clone --depth 1` (SingleStepTests, z80test) vers `tests/roms/<nom>/`; pour chacune, VERIFIE que l'URL repond (HTTP 200 / clone ok) et ecris les URL REELLES trouvees + la structure des dossiers dans `docs/qa/FAQ_Z80.md` section `## ROMs: sources verifiees` (date, commit, arborescence). Si inaccessible : question Q&R.
- `tests/manifest.toml` : pour chaque fichier telecharge `name`, `url`, `size`, `sha256`. Option `--verify` recalcule et compare; `--only prelim,zexdoc,zexall`.
- `docs/tools/fetch_roms.py --verify` rend 0 si tout ce qui est present est conforme.
Ne telecharge JAMAIS de ROM de ZX Spectrum (zx48.rom est a fournir par l'utilisateur).""",
  notes=UNK,
  write=["docs/tools/fetch_roms.py", "tests/manifest.toml", "docs/qa/FAQ_Z80.md", "tests/roms/README.md"],
  tests=[],
  accept=["python docs/tools/fetch_roms.py --only prelim,zexdoc,zexall", "python docs/tools/fetch_roms.py --verify", "ls -la tests/roms"])

T("E13-02", "Machine CP/M (BDOS emule)", kind="code", size=150, crate="puce-z80-machines",
  context="""Machine d'essai CP/M-80 minimale pour PRELIM/ZEXDOC/ZEXALL et pour le frontend : 64 Ko de RAM, programme .com/.cim charge en 0100h, BDOS emule au point d'entree 0005h, warm boot en 0000h.""",
  embed=["30_TEST_SUITES.md#30 -"],
  reads=["`docs/ref/z80_ref.js` et `tools` JS de test CP/M (cherche 'BDOS'/'0x0005') pour la semantique exacte des fonctions 2 et 9"],
  spec="""Dans `crates/puce-z80-machines/src/cpm.rs` :
```rust
pub struct CpmMachine { pub cpu: Cpu, pub ram: Box<[u8; 65536]>, pub out: Vec<u8>, pub exited: bool, pub tests_ok: u32, pub tests_err: u32, pub instr: u64, pub tstates: u64 }
impl CpmMachine { pub fn new() -> Self; pub fn load_com(&mut self, bytes: &[u8]); /* en 0100h, PC=0100h, SP=0xFFFE, octets 0000h=C3 .. warm boot, 0005h=C9 RET */
    pub fn step(&mut self) -> u32;   // une instruction (Cpu::step avec un Bus interne)
    pub fn run(&mut self, max_instr: u64) -> RunEnd; }
```
BDOS : intercepte quand `PC == 0x0005` AVANT l'execution: fonction C=2 (E = caractere -> `out`), C=9 (DE = chaine terminee par '$' -> `out`), puis simule RET (PC = pile). `PC == 0x0000` (warm boot) -> `exited = true`. Compte les lignes `OK`/`ERROR` de `out` apres chaque ajout (`tests_ok`, `tests_err`). `RunEnd { Exited, MaxInstr, Halted }`. Pas de dependance autre que le crate.""",
  write=["crates/puce-z80-machines/src/cpm.rs", "crates/puce-z80-machines/src/lib.rs"],
  tests=[("bdos_fn2", "programme 'MVI E,\"A\"; MVI C,2; CALL 5; JMP 0' ecrit 'A'"), ("bdos_fn9", "affiche une chaine '$'"), ("warm_boot_exit", "JMP 0 -> exited"), ("load_com_pc", "PC=0x0100 apres load_com"), ("counts_ok_lines", "la sortie 'xx OK\\r\\n' incremente tests_ok"), ("max_instr", "boucle infinie s'arrete sur MaxInstr"), ("counters", "instr et tstates augmentent")])

T("E13-03", "PRELIM: premiere ROM de test", kind="code", size=70, crate="puce-z80-cli",
  context="""Premiere validation par ROM officielle: `prelim.com` verifie les instructions de base avant ZEXDOC.""",
  embed=[S30],
  spec="""Implemente `z80 cpm <rom> [--max-instr N]` dans la CLI (`commands/cpm.rs`) : charge le fichier avec `CpmMachine`, execute jusqu'a `exited` ou `max_instr`, affiche la sortie sur stdout, renvoie code 0 si `exited` et pas de ligne `ERROR`, sinon 1.
Test d'integration `crates/puce-z80-cli/tests/prelim.rs` : cherche `tests/roms/prelim.com` ; si absent `eprintln!("SKIP: run python docs/tools/fetch_roms.py")` et passe ; sinon lance PRELIM et verifie que la sortie contient `Preliminary tests complete`.""",
  write=["crates/puce-z80-cli/src/commands/**", "crates/puce-z80-cli/src/main.rs", "crates/puce-z80-cli/tests/prelim.rs"],
  tests=[("cpm_exit_code_ok", "un petit programme qui affiche et JMP 0 renvoie 0"), ("cpm_missing_file", "fichier absent -> erreur"), ("prelim_passes", "prelim.com affiche 'Preliminary tests complete' (SKIP si ROM absente)")],
  accept=["cargo test -p puce-z80-cli e13_03_", "cargo run -q --release -p puce-z80-cli -- cpm tests/roms/prelim.com"],
  notes="Si PRELIM echoue, la cause est dans `puce-z80` (pas dans le test): note la ligne affichee et corrige l'instruction fautive (sem/** ou alu.rs).")

T("E13-04", "ZEXDOC (67 tests)", kind="code", size=100, crate="puce-z80-cli",
  context="""ZEXDOC (Frank Cringle/Yaze) verifie les drapeaux DOCUMENTES de 67 groupes d'instructions via CRC. Le coeur de reference JS le passe; le crate doit afficher 67 `OK`.""",
  embed=[S30],
  spec="""- `z80 suite zexdoc|zexall|prelim [--max-instr N]` : lance la suite correspondante depuis `tests/roms/`, affiche la progression, produit un resume JSON `{suite, tests, ok, err, instr, tstates, seconds, mips}` sur la derniere ligne.
- Test `crates/puce-z80-cli/tests/zexdoc.rs` `#[ignore]` (long) : `cargo test --release -p puce-z80-cli --test zexdoc -- --ignored` : exige 67 `OK`, 0 `ERROR`, et `Tests complete`. SKIP si ROM absente.
- Attendu : ~5,8 milliards d'instructions (~46,7 milliards de T-states) : en release ce test doit tenir en < 1 h (voir E13-20 pour la performance).
Pour DIAGNOSTIQUER un echec : la sortie affiche le nom du groupe (ex. `<adc,sbc> hl,<bc,de,hl,sp>`), le CRC attendu et obtenu. Retrouve l'instruction concernee, ecris un test cible dans `puce-z80/tests/` si utile, corrige.""",
  write=["crates/puce-z80-cli/src/commands/**", "crates/puce-z80-cli/tests/zexdoc.rs", "crates/puce-z80/src/**"],
  tests=[("suite_json_summary", "la derniere ligne de sortie est un JSON valide avec les champs requis (sur un mini programme)"), ("zexdoc_full", "67 OK (ignore, long, SKIP si ROM absente)")],
  accept=["cargo test -p puce-z80-cli e13_04_suite", "cargo test --release -p puce-z80-cli --test zexdoc -- --ignored --nocapture"])

T("E13-05", "ZEXALL (drapeaux X/Y inclus) et decision SCF/CCF", kind="code", size=70, crate="puce-z80-cli",
  context="""ZEXALL verifie aussi les bits non documentes 3 et 5 de F. C'est le test qui tranche la formule SCF/CCF (E10-03).""",
  embed=["23_UNDOCUMENTED.md#23 -"],
  spec="""- Test `crates/puce-z80-cli/tests/zexall.rs` `#[ignore]` : exige 67 `OK`.
- Lance ZEXALL avec `SCF_CCF_USES_Q = true` puis `false` (modifie la constante de `puce-z80/src/sem/**`) et note dans `docs/qa/DECISIONS.md` (ligne D-023) laquelle passe (ou si les deux passent). Garde la valeur qui passe; si les deux passent garde `true` (D-006: NMOS Zilog). Supprime le commentaire `UNKNOWN - to confirm` de E10-03 une fois la decision prise (ou garde-le si z80ccf (E13-17) doit encore trancher).
- Si des groupes echouent : meme methode de diagnostic qu'en E13-04.""",
  write=["crates/puce-z80-cli/tests/zexall.rs", "crates/puce-z80/src/sem/**", "crates/puce-z80/src/alu.rs", "docs/qa/DECISIONS.md"],
  tests=[("zexall_full", "67 OK (ignore, long, SKIP si ROM absente)")],
  accept=["cargo test --release -p puce-z80-cli --test zexall -- --ignored --nocapture"])

T("E13-06", "SingleStepTests/z80: decouvrir et documenter le format", kind="doc", size=0, assignee="hermes",
  context="""SingleStepTests/z80 fournit, pour chaque opcode, des milliers de cas JSON (etat initial, etat final, liste de cycles de bus). Le format exact n'est pas dans ce depot : il faut le LIRE sur les vrais fichiers.""",
  notes=UNK,
  spec="""1. `git clone --depth 1 https://github.com/SingleStepTests/z80 tests/roms/singlestep` (si l'URL est fausse: cherche le depot exact et note-le).
2. Lis le README du depot et OUVRE 3 fichiers de test (par ex. un NOP, un `LD A,(nn)`, un `DD CB`). Ecris `docs/specs/33_SINGLESTEP_FORMAT.md` (ASCII, francais) qui decrit : la structure des dossiers, le nom des fichiers (par opcode, prefixes), les champs JSON (`name`, `initial`, `final`, `cycles`, `ports`...), l'encodage de `ram` (paires [addr, val]), les registres presents (`wz`, `iff1`, `iff2`, `im`, `ei`, `p`, `q`, `af_`, ...), la forme exacte d'un element de `cycles` (adresse, donnee, broches sous forme de chaine type `read`/`write`/`---`), le nombre de cas par fichier, et les cas particuliers (HALT, interruptions, ports).
3. Ajoute une section `## Convention de broches observee` : pour 3 instructions, recopie la liste des cycles du JSON a cote de ce que produit `fmt_trace` (E03-14), et liste les differences.
4. Mets a jour `docs/specs/11_PINS_AND_TICK.md` ligne `UNKNOWN - to confirm` du paragraphe 3 en renvoyant vers ce nouveau fichier. N'ecris PAS de code Rust.""",
  write=["docs/specs/33_SINGLESTEP_FORMAT.md", "docs/specs/11_PINS_AND_TICK.md", "docs/qa/FAQ_Z80.md"],
  accept=["test -s docs/specs/33_SINGLESTEP_FORMAT.md"])

T("E13-07", "SingleStepTests: chargeur JSON et mise en etat", kind="code", size=140, crate="puce-z80-cli",
  context="""Lire les fichiers JSON (serde) et initialiser un `Cpu` + une RAM sparse depuis `initial`.""",
  reads=["`docs/specs/33_SINGLESTEP_FORMAT.md` (cree en E13-06: FAIT FOI sur le format)"],
  spec="""Dans `crates/puce-z80-machines/src/ssc.rs` (module declare dans lib.rs) : structs serde `Case { name, initial: State, final_: State, cycles: Vec<...> }` conformes au format documente, `State` -> `Cpu` (tous les registres, `wz`, `iff1/2`, `im`, `ei`->`ei_delay`, `q`, `p`...) et `RamMap` (HashMap addr -> octet; lecture d'une adresse absente = 0xFF ou valeur documentee). `load_file(path) -> Vec<Case>`, `apply_initial(&Case) -> (Cpu, SparseBus)`.
Pas de comparaison ici (E13-08).""",
  write=["crates/puce-z80-machines/src/ssc.rs", "crates/puce-z80-machines/src/lib.rs", "crates/puce-z80-machines/Cargo.toml"],
  tests=[("parse_sample", "un JSON de 2 cas (ecrit en dur dans le test, copie du format reel) se charge"), ("apply_regs", "les registres sont recopies (af bc de hl ix iy sp pc i r wz iff im)"), ("apply_ram", "la RAM sparse contient les paires"), ("missing_field_error", "un champ obligatoire absent donne une erreur claire"), ("load_real_file", "charge un fichier reel de tests/roms/singlestep si present (SKIP sinon)")])

T("E13-08", "SingleStepTests: comparaison de l'etat final", kind="code", size=140, crate="puce-z80-cli",
  context="""Execute chaque cas avec `Cpu::step` et compare l'etat final (registres, RAM, WZ, Q...) avec `final`.""",
  reads=["`docs/specs/33_SINGLESTEP_FORMAT.md`"],
  spec="""- `z80 ssc <dir> [--filter 00,dd/21] [--max-fail N]` : pour chaque fichier/cas, `apply_initial`, `step`, `diff_final(&Case, &Cpu, &SparseBus) -> Vec<String>` (liste lisible des ecarts : champ, attendu, obtenu) ; resume par opcode (cas passes/total) ; code de retour 1 si ecart.
- Compare : registres 8/16 bits, `wz`, `iff1`, `iff2`, `im`, `r`, `i`, `q`, RAM (adresses du `final.ram` ET toute ecriture parasite), ports si presents. Les cycles de bus (pins) NE SONT PAS compares ici (E13-09).
- Les ecarts sur HALT (convention de PC, D-022), SCF/CCF (Q) et WZ des instructions de bloc sont attendus au debut : liste-les dans `tests/baseline.json` (`{"known_failures": ["<opcode>", ...]}`) avec une raison, au lieu de les masquer.""",
  write=["crates/puce-z80-cli/src/commands/ssc.rs", "crates/puce-z80-cli/src/main.rs", "crates/puce-z80-machines/src/ssc.rs", "tests/baseline.json"],
  tests=[("diff_final_reports", "un etat final volontairement faux donne des ecarts nommes"), ("diff_final_ok", "un etat correct donne une liste vide"), ("summary_format", "le resume par opcode est stable (une ligne par opcode)"), ("filter", "--filter limite aux opcodes demandes")],
  accept=["cargo test -p puce-z80-cli e13_08_", "cargo run -q --release -p puce-z80-cli -- ssc tests/roms/singlestep --filter 00,01,3e --max-fail 20"])

T("E13-09", "SingleStepTests: etat final sur toutes les instructions", kind="code", size=100, crate="puce-z80-cli",
  context="""Passe 1 : tous les opcodes, etat final seulement. Corriger les bugs de semantique reveles (WZ, Q, flags non documentes, interruptions...).""",
  spec="""Lance `z80 ssc tests/roms/singlestep --max-fail 50` (release). Classe les echecs par cause, corrige dans `puce-z80/src/**` (sem, alu, moteur) ce qui est un VRAI bug, et inscris dans `tests/baseline.json` seulement ce qui releve d'une decision documentee (D-022 HALT, formule Q...). Ajoute un test `e13_09_ssc_final_state` `#[ignore]` qui lance tout et exige 0 echec hors baseline.
Si un cas montre qu'une semantique du JS de reference etait fausse (le JS n'est pas infaillible), corrige le crate ET ajoute un test cible dans `puce-z80/tests/`.""",
  write=["crates/puce-z80/src/**", "crates/puce-z80/tests/**", "crates/puce-z80-cli/tests/**", "tests/baseline.json", "docs/qa/DECISIONS.md"],
  tests=[("ssc_final_state", "0 echec hors baseline (ignore, long)")],
  accept=["cargo test --release -p puce-z80-cli --test ssc -- --ignored --nocapture"])

T("E13-10", "SingleStepTests: cycles de bus T par T (calage)", kind="code", size=150, crate="puce-z80-cli",
  context="""Passe 2 : comparer la liste `cycles` (une entree par T-state : adresse, donnee, broches) a la trace de `Cpu::tick`. C'est ici que la CONVENTION de broches (UNKNOWN de 11_PINS_AND_TICK.md) est calee.""",
  reads=["`docs/specs/33_SINGLESTEP_FORMAT.md` section 'Convention de broches observee'"],
  spec="""- Etend le comparateur : `diff_cycles(&Case, &[Pins])` aligne le i-eme element de `cycles` sur le i-eme tick (le nombre de ticks doit etre egal). Mappe `read/write/fetch...` du JSON sur nos broches selon le format documente.
- Lance sur un echantillon (NOP, LD r,n, LD A,(nn), PUSH, CALL, IN/OUT, DD CB) et CALE `engine/mcycle.rs` / le moteur jusqu'a ce que les cycles correspondent : ordre des broches au T1..T4, valeur de A pendant le refresh, adresse pendant les T internes, etc.
- Documente chaque ecart de convention resolu dans `docs/specs/11_PINS_AND_TICK.md` (remplace la ligne `UNKNOWN - to confirm` par la convention validee).
- Ne casse AUCUN test existant (diff.tsv, timing). Si le calage impose de changer la forme de `Pins`, pose une question Q&R.""",
  write=["crates/puce-z80/src/engine/**", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/int.rs", "crates/puce-z80-machines/src/ssc.rs", "crates/puce-z80-cli/src/commands/ssc.rs", "docs/specs/11_PINS_AND_TICK.md", "docs/specs/13_ENGINE.md"],
  tests=[("cycles_match_sample", "un echantillon de 20 opcodes a des cycles identiques (SKIP si dossier absent)"), ("cycle_diff_message", "un ecart de cycle affiche le T, attendu et obtenu")],
  accept=["cargo test -p puce-z80-cli e13_10_", "cargo run -q --release -p puce-z80-cli -- ssc tests/roms/singlestep --cycles --filter 00,01,3e,3a,c5,cd,db,d3,dd/cb --max-fail 20"])

T("E13-11", "SingleStepTests: passe complete (etat + cycles)", kind="code", size=60, crate="puce-z80-cli",
  context="""Toute la suite SingleStepTests/z80 doit passer (D-003).""",
  spec="""`z80 ssc tests/roms/singlestep --cycles` sur TOUS les fichiers ; corrige jusqu'a 0 echec; `tests/baseline.json` ne doit plus contenir de `known_failures` (ou seulement des cas justifies par ecrit dans DECISIONS.md et valides par l'utilisateur: pose une question Q&R avant d'en garder un). Test `e13_11_ssc_all` `#[ignore]`.""",
  write=["crates/puce-z80/src/**", "crates/puce-z80-cli/tests/**", "tests/baseline.json", "docs/qa/DECISIONS.md"],
  tests=[("ssc_all", "0 echec (etat final + cycles), ignore, long")],
  accept=["cargo test --release -p puce-z80-cli --test ssc -- --ignored --nocapture e13_11"])

T("E13-12", "FUSE: decouvrir et documenter le format", kind="doc", size=0, assignee="hermes",
  context="""Les tests FUSE (`z80/tests/tests.in` et `tests.expected`) donnent, par opcode, un etat initial et l'etat final + les evenements de bus horodates (MR MW PR PW MC PC).""",
  notes=UNK,
  spec="""Trouve la source exacte (depot fuse-emulator ou miroir) et recupere `tests.in` et `tests.expected` dans `tests/roms/fuse/`. Ecris `docs/specs/34_FUSE_FORMAT.md` : structure d'un enregistrement de `tests.in` (nom, registres sur 2 lignes, I R IFF1 IFF2 IM halted tstates, memoire `addr bytes -1`), de `tests.expected` (evenements `tstate type addr data`, etat final, memoire modifiee), signification de chaque code d'evenement, regle de comparaison (nombre de T-states de l'instruction, ordre des evenements), cas particuliers. Mets a jour la ligne FUSE du tableau de `docs/specs/30_TEST_SUITES.md` (URL reelle). Pas de code Rust.""",
  write=["docs/specs/34_FUSE_FORMAT.md", "docs/specs/30_TEST_SUITES.md", "docs/qa/FAQ_Z80.md"],
  accept=["test -s docs/specs/34_FUSE_FORMAT.md"])

T("E13-13", "FUSE: lecteur et execution", kind="code", size=150, crate="puce-z80-cli",
  context="""Implemente le lecteur de `tests.in`/`tests.expected` et l'executeur.""",
  reads=["`docs/specs/34_FUSE_FORMAT.md` (FAIT FOI)"],
  spec="""Dans `crates/puce-z80-machines/src/fuse.rs` : parseurs (texte), `FuseTest { name, regs, mem_blocks, tstates }`, `FuseExpected { events, regs, mem_changes }`, executeur `run_fuse(test) -> FuseResult` : execute des instructions jusqu'a ce que le total de T-states atteigne `tstates` du test (regle FUSE: l'instruction qui franchit la limite est executee), enregistre les evenements de bus (T-state, type, adresse, donnee) via `Bus::on_tick`/le journal et compare avec `expected`. `z80 fuse <dir>` affiche les echecs et le resume.""",
  write=["crates/puce-z80-machines/src/fuse.rs", "crates/puce-z80-machines/src/lib.rs", "crates/puce-z80-cli/src/commands/fuse.rs", "crates/puce-z80-cli/src/main.rs"],
  tests=[("parse_in", "un enregistrement tests.in en dur est parse"), ("parse_expected", "un enregistrement tests.expected en dur est parse"), ("run_nop", "le test NOP (00) passe"), ("event_order", "les evenements d'un LD A,(nn) sont MC/MR dans l'ordre FUSE"), ("mismatch_reported", "un evenement faux est signale avec son T-state")])

T("E13-14", "FUSE: suite complete", kind="code", size=60, crate="puce-z80-cli",
  context="Tous les tests FUSE doivent passer (D-003).",
  spec="Lance `z80 fuse tests/roms/fuse` ; corrige jusqu'a 0 echec (timing des cycles d'E/S, contention, interruptions, instructions de bloc...). Test `e13_14_fuse_all` (SKIP si fichiers absents). Les corrections de timing doivent aussi laisser passer E13-11 (SingleStepTests).",
  write=["crates/puce-z80/src/**", "crates/puce-z80-cli/tests/**", "crates/puce-z80-machines/src/fuse.rs"],
  tests=[("fuse_all", "0 echec")],
  accept=["cargo test --release -p puce-z80-cli e13_14_ -- --nocapture"])

T("E13-15", "z80test (Patrik Rak): decouvrir et documenter", kind="doc", size=0, assignee="hermes",
  context="""z80test (z80doc, z80docflags, z80flags, z80full, z80ccf, z80memptr) est la suite la plus severe sur les drapeaux non documentes et MEMPTR. Elle tourne sur un ZX Spectrum (fichiers .tap/.z80) et affiche ses resultats a l'ecran.""",
  notes=UNK,
  spec="""Clone `https://github.com/raxoft/z80test` (verifie l'URL) dans `tests/roms/z80test/`. Ecris `docs/specs/35_Z80TEST.md` : fichiers disponibles et formats (.tap, .z80, .sna, sources .asm), comment chaque test s'execute (point d'entree, ROM requise), COMMENT il rapporte le resultat (texte a l'ecran via RST 10h ? code de sortie ?), le texte exact de reussite ('Passed' etc.), les adresses ROM a intercepter pour un chargement rapide de bande (LD-BYTES 0556h) et pour capturer la sortie (RST 10h / PRINT-A), et les prerequis (ROM 48K fournie par l'utilisateur: `tests/roms/zx48.rom`). Pas de code Rust.""",
  write=["docs/specs/35_Z80TEST.md", "docs/qa/FAQ_Z80.md"],
  accept=["test -s docs/specs/35_Z80TEST.md"])

T("E13-16", "Machine ZX Spectrum minimale", kind="code", size=150, crate="puce-z80-machines",
  context="""Machine ZX Spectrum 48K minimale pour executer z80test et les ROMs de timing : ROM 16 Ko fournie par l'utilisateur, RAM 48 Ko, port 0xFE (clavier = rien, bordure ignoree), chargement rapide de bande, capture du texte.""",
  reads=["`docs/specs/35_Z80TEST.md` (FAIT FOI)", "`docs/specs/32_REUSE_GUIDE.md` (contention)"],
  spec="""Dans `crates/puce-z80-machines/src/zx.rs` : `ZxMachine { cpu, mem: [u8; 65536], rom_loaded, tap, out: String, ... }` ; `new(rom: &[u8])` (refuse si != 16384 octets) ; `step()` ; interruption 50 Hz generee tous les 69888 T-states (INT pendant 32 T) ; `Bus::io_read(0xFE)` renvoie 0xBF (pas de touche) ; **fast-load** : quand `PC == 0x0556` (LD-BYTES) copie le bloc suivant du .tap vers l'adresse DE, longueur IX/DE, positionne les registres comme le ROM a la sortie (carry set = succes) puis simule RET ; capture de sortie : `PC == 0x0010` (RST 10h, caractere dans A) ajoute a `out`. Contention de la memoire (extra_wait) en OPTION (`with_contention(bool)`) : table standard des 6 T-states de motif sur 8 pour les adresses 0x4000-0x7FFF pendant les lignes d'ecran (UNKNOWN - to confirm, necessaire seulement pour les ROMs de timing, E13-19).
`load_tap(bytes)` parse les blocs `.tap` (flag, donnees, checksum).""",
  write=["crates/puce-z80-machines/src/zx.rs", "crates/puce-z80-machines/src/lib.rs"],
  tests=[("rom_size_check", "ROM de mauvaise taille refusee"), ("tap_parse", "un .tap de 2 blocs en dur est parse"), ("fastload_copies_block", "avec une fausse ROM contenant CALL 0556h, le bloc est copie a DE"), ("rst10_capture", "un programme qui fait RST 10h avec A='X' capture 'X'"), ("frame_interrupt", "apres 69888 T l'INT est genere"), ("io_fe", "IN A,(FE) renvoie 0xBF")])

T("E13-17", "z80test: executer les 6 tests", kind="code", size=120, crate="puce-z80-cli",
  context="""Execute z80doc, z80docflags, z80flags, z80full, z80ccf, z80memptr sur la machine ZX et exige `Passed` pour tous (D-003).""",
  reads=["`docs/specs/35_Z80TEST.md`"],
  spec="""`z80 z80test <fichier.tap> --rom tests/roms/zx48.rom [--max-frames N]` : lance, affiche la sortie capturee, sort 0 si le motif de reussite documente est present et aucun echec. Test `crates/puce-z80-cli/tests/z80test.rs` : un test `#[ignore]` par programme (SKIP si `zx48.rom` ou le .tap est absent). Corrige les ecarts dans `puce-z80/src/**` : `z80ccf` tranche SCF/CCF (note la decision dans DECISIONS.md D-024), `z80memptr` valide WZ (corrige E10-01 si besoin), `z80flags`/`z80full` valident les blocs repetes (E10-02) : remplace les `UNKNOWN - to confirm` correspondants par la regle prouvee.""",
  write=["crates/puce-z80-cli/src/commands/**", "crates/puce-z80-cli/src/main.rs", "crates/puce-z80-cli/tests/z80test.rs", "crates/puce-z80/src/**", "docs/qa/DECISIONS.md"],
  tests=[("z80doc", "Passed"), ("z80docflags", "Passed"), ("z80flags", "Passed"), ("z80full", "Passed"), ("z80ccf", "Passed"), ("z80memptr", "Passed")],
  accept=["cargo test --release -p puce-z80-cli --test z80test -- --ignored --nocapture"])

T("E13-18", "ROMs de timing et d'interruptions: decouvrir et documenter", kind="doc", size=0, assignee="hermes",
  context="""Des ROMs de test de timing (Woodmass, Raxoft, et autres: contention, floating bus, interruptions, HALT) completent les suites precedentes. La liste exacte n'est pas dans ce depot.""",
  notes=UNK,
  spec="""Recherche les ROMs de timing/interruptions du Z80 disponibles publiquement (Woodmass `Z80 timing/contention test`, Raxoft `z80test`/`intr`/`halt`, jsmoo...) : pour chacune note l'URL reelle, la licence, le format, comment elle rapporte son resultat (ecran, bordure, code), la machine exigee. Ecris `docs/specs/36_TIMING_ROMS.md` avec un tableau et, pour chaque ROM retenue, le critere de reussite. Ajoute les fichiers retenus a `tests/manifest.toml` (url, sha256). N'invente aucune ROM : si une source est inaccessible, pose une question Q&R et note-la `UNAVAILABLE`.""",
  write=["docs/specs/36_TIMING_ROMS.md", "tests/manifest.toml", "docs/qa/FAQ_Z80.md"],
  accept=["test -s docs/specs/36_TIMING_ROMS.md"])

T("E13-19", "ROMs de timing: execution", kind="code", size=120, crate="puce-z80-cli",
  context="Execute les ROMs de timing retenues en E13-18 sur la machine ZX (avec contention si necessaire).",
  reads=["`docs/specs/36_TIMING_ROMS.md`"],
  spec="""Pour chaque ROM retenue : un test `#[ignore]` dans `crates/puce-z80-cli/tests/timing_roms.rs` (SKIP si absente) avec le critere du doc. Complete la contention de `ZxMachine` (table exacte documentee dans le doc, 48K et 128K si pertinent) et corrige les ecarts de timing du crate (WAIT, INT, HALT, E/S contendues). Chaque ROM acceptee passe; celles marquees `UNAVAILABLE` sont listees comme telles dans le test.""",
  write=["crates/puce-z80-cli/tests/timing_roms.rs", "crates/puce-z80-machines/src/zx.rs", "crates/puce-z80/src/**"],
  tests=[("timing_roms_all", "toutes les ROMs retenues passent")],
  accept=["cargo test --release -p puce-z80-cli --test timing_roms -- --ignored --nocapture"])

T("E13-20", "Performance: objectif 15 MHz equivalent en release", kind="code", size=100, crate="puce-z80",
  context="""ZEXDOC/ZEXALL font ~46,7 milliards de T-states : le moteur T par T doit etre assez rapide (>= 15 MHz equivalent soit < 1 h).""",
  embed=["30_TEST_SUITES.md#30 -"],
  spec="""Ecris `crates/puce-z80-cli/src/commands/bench.rs` (pas de dependance de bench) (`z80 bench --seconds 10`) qui execute une boucle de NOP/LD/ADD/JR en release et affiche T-states/s ('MHz equivalent'). Mesure; si < 15 MHz equivalent, optimise SANS changer le comportement : (1) evite les recherches de table par jeton (pointeur direct), (2) `#[inline]` sur les chemins chauds de `tick`, (3) evite les `match` sur Kind repetes (precalcule la fonction de broches du jeton), (4) pas d'allocation, (5) hors feature `inspect` aucun cout. Toute optimisation garde les tests (diff, ssc, fuse) verts. Consigne les mesures avant/apres dans `docs/qa/FAQ_Z80.md` section `## Performance`.""",
  write=["crates/puce-z80/src/**", "crates/puce-z80-cli/src/commands/bench.rs", "crates/puce-z80-cli/src/main.rs", "docs/qa/FAQ_Z80.md"],
  tests=[("bench_runs", "z80 bench --seconds 1 s'execute et affiche un debit"), ("behavior_unchanged", "Z80_DIFF_TABLE=all passe apres optimisation")],
  accept=["cargo run -q --release -p puce-z80-cli -- bench --seconds 5", "Z80_DIFF_TABLE=all cargo test --release -p puce-z80 --test diff -- e05_01_diff_run"])

T("E13-21", "Baseline et resume global des suites", kind="code", size=100, crate="puce-z80-cli",
  context="""Une commande unique `z80 suite all` lance toutes les suites disponibles et ecrit un resume machine-lisible (utilise pour le GATE et la CI de release).""",
  spec="""`z80 suite all [--skip zexall,...] [--json out.json]` : lance prelim, zexdoc, zexall, ssc (cycles), fuse, z80test (6), timing ROMs, diff.tsv (appel `cargo test` impossible: reimplemente un appel direct au banc via la lib de machines ou lance `cargo test` en sous-processus) ; chaque suite : `PASS | FAIL | SKIP(reason) | UNAVAILABLE`. Sortie JSON `{suites:[{name,status,details,seconds}]}`, code 0 si aucune FAIL et aucune SKIP des suites obligatoires (prelim, zexdoc, zexall, ssc, fuse, z80test). Ecris `docs/tools/suites_report.py` qui transforme le JSON en tableau Markdown `docs/SUITES_REPORT.md`.""",
  write=["crates/puce-z80-cli/src/commands/**", "crates/puce-z80-cli/src/main.rs", "docs/tools/suites_report.py", "docs/SUITES_REPORT.md"],
  tests=[("all_skip_missing", "sans aucune ROM: toutes SKIP et code != 0"), ("json_shape", "le JSON a le champ suites avec name/status"), ("skip_flag", "--skip exclut une suite"), ("report_table", "suites_report.py produit un tableau")],
  accept=["cargo test -p puce-z80-cli e13_21_"])

T("E13-22", "GATE E13 - toutes les ROMs de test validees", gate=True, kind="gate", size=0,
  context="""Toutes les suites officielles passent : PRELIM, ZEXDOC, ZEXALL, z80test, SingleStepTests/z80, FUSE, ROMs de timing. Dernier controle avant le frontend.""",
  user_cmds=["python docs/tools/fetch_roms.py --verify", "cargo run --release -p puce-z80-cli -- suite all --json suites.json", "python docs/tools/suites_report.py suites.json", "bash tools/ci.sh"], accept=[])
