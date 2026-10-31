from _dsl import T

ENG = "13_ENGINE.md"

T("E03-01", "Structures Cpu et Engine", kind="code", size=110,
  context="""Debut du moteur T-state. Cette tache ne fait que poser les structures (sans logique de tick) pour que les suivantes les completent.""",
  embed=[ENG + "#1. Etat", "11_PINS_AND_TICK.md#2. API"],
  spec="""- `crates/puce-z80/src/cpu.rs` (`pub mod cpu;`, re-export `Cpu`): `pub struct Cpu { pub regs: Regs, pub(crate) eng: Engine }`, `Cpu::new()` (regs par defaut, `Engine::new()`), `impl Default`, `pub fn at_instruction_boundary(&self) -> bool { !self.eng.active }`.
  `pub fn tick(&mut self, pins: Pins) -> Pins` pour l'instant: `pins` renvoye tel quel (stub, remplace par E03-07).
- `engine/mod.rs`: struct `Engine` avec EXACTEMENT les champs de la section 1 (types: `Table`, `Prefix` de decode.rs; `Plan`), `Engine::new()` (table Main, prefix None, plan `&[]`, tout a 0/false).
- Constante `pub(crate) const FETCH_PROBE: Tok = Tok::new(Kind::F, 4, Addr::None, Reg::None, false);` dans plan.rs.""",
  write=["crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/engine/plan.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("new_idle", "Cpu::new() est a une frontiere d'instruction"), ("engine_defaults", "Engine::new(): table Main, prefix None, idx 0, t 0, cond false, tmp 0"), ("fetch_probe", "FETCH_PROBE est un F de 4 T"), ("default_eq_new", "Cpu::default().regs == Cpu::new().regs")])

T("E03-02", "Broches du cycle M1 (F4/F5/F6)", kind="code", size=100,
  context="""Fonction pure qui donne les broches de sortie d'un T-state de cycle M1. Elle ne touche pas l'etat du CPU.""",
  embed=[ENG + "#3. Sorties de broches", "11_PINS_AND_TICK.md#3. Sequence"],
  spec="""Dans `engine/mcycle.rs` (`pub mod mcycle;`):
`pub fn fetch_pins(pc: u16, i: u8, r: u8, t: u8, len: u8, halt: bool) -> Pins` renvoie les sorties au T-state `t` (0..len-1) d'un M1 de `len` T (4, 5 ou 6):
- t0: A=pc, M1. t1: A=pc, M1|MREQ|RD. t2: A=(i<<8)|(r&0x7F), MREQ|RFSH. t3: idem t2 (MREQ|RFSH). t>=4 (M1 etendu): A=(i<<8)|(r&0x7F), aucun controle.
- HALT: si `halt` vrai ajouter `Pins::HALT` a chaque T.
La valeur de `r` fournie est celle APRES incrementation (l'increment est fait par l'appelant au t0, voir ENGINE section 3).""",
  write=["crates/puce-z80/src/engine/mcycle.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("t0", "T0: A=pc, M1 seul"), ("t1", "T1: M1 MREQ RD"), ("t2_refresh", "T2: A=(i<<8)|r, MREQ RFSH, sans RD ni M1"), ("t3", "T3 = T2"), ("extended", "len=6: t4 et t5 sans aucun controle, A=I:R"), ("r_bit7_masked", "r=0xFF -> A bas = 0x7F"), ("halt_flag", "halt=true ajoute HALT aux 4 T")])

T("E03-03", "Broches des cycles memoire et internes", kind="code", size=110,
  context="""Suite de `mcycle.rs`: lectures et ecritures memoire, cycles internes.""",
  embed=[ENG + "#3. Sorties de broches"],
  spec="""Ajoute dans `engine/mcycle.rs`:
- `pub fn read_pins(addr: u16, t: u8) -> Pins`: t0: A; t1: A+MREQ+RD; t>=2: A seul. (valable pour R3, R4, R5: les T supplementaires gardent A sans controle.)
- `pub fn write_pins(addr: u16, data: u8, t: u8) -> Pins`: t0: A+D; t1: A+D+MREQ+WR; t>=2: A+D sans controle. (W3, W5)
- `pub fn internal_pins(addr: u16) -> Pins`: A seul, aucun controle (cycles `I<n>`).""",
  write=["crates/puce-z80/src/engine/mcycle.rs"],
  tests=[("read_t0", "t0: seulement A"), ("read_t1", "t1: MREQ et RD"), ("read_t2_ext", "t2..t4: aucun controle"), ("write_t0", "t0: A et D, pas de WR"), ("write_t1", "t1: MREQ WR avec A et D"), ("write_ext", "t2..t4: A et D maintenus, aucun controle"), ("internal", "internal_pins n'a que A")])

T("E03-04", "Broches des cycles d'entree/sortie", kind="code", size=90,
  context="""Suite de `mcycle.rs`: cycles de port (IN/OUT) avec un T d'attente automatique, et acquittement d'interruption (utilise par E09).""",
  embed=[ENG + "#3. Sorties de broches"],
  spec="""Ajoute dans `engine/mcycle.rs`:
- `pub fn io_read_pins(addr: u16, t: u8) -> Pins` (N4): t0: A; t1: A; t2: A+IORQ+RD; t3: A+IORQ+RD (le T3 est le T-state d'attente automatique TW: IORQ/RD restent actifs). La donnee est latchee au tick t3 (dernier T).
- `pub fn io_write_pins(addr: u16, data: u8, t: u8) -> Pins` (O4): t0: A+D; t1: A+D; t2: A+D+IORQ+WR; t3: A+D+IORQ+WR.
- `pub fn int_ack_pins(pc: u16, t: u8) -> Pins` (acquittement d'interruption, 7 T = M1 de 4T + 2 TW + 1): t0: A=pc+M1; t1: A=pc+M1; t2..t5 (TW, T2): A=pc+M1+IORQ; t6: A=pc+M1+IORQ... UNKNOWN - to confirm : la sequence exacte est calee en E09/E13 sur FUSE; ici implemente la version simple ci-dessus (M1+IORQ des le t2 jusqu'a t5, puis t6 sans IORQ). Documente `UNKNOWN - to confirm` en commentaire.""",
  write=["crates/puce-z80/src/engine/mcycle.rs"],
  tests=[("in_t2", "N4 t2: IORQ et RD, pas MREQ"), ("in_t0_t1", "N4 t0 et t1: adresse seule"), ("out_t2", "O4 t2: IORQ WR avec D"), ("out_t0", "O4 t0: A et D sans controle"), ("ack_has_m1_iorq", "int_ack_pins(.., 2) contient M1 et IORQ et pas MREQ"), ("ack_first_t", "int_ack_pins t0: M1 seul")])

T("E03-05", "Resolution des adresses (Addr)", kind="code", size=120,
  context="""Convertit un `Addr` du plan en adresse de bus avec ses effets de bord sur les registres (PC+, SP+, -SP, WZ...).""",
  embed=[ENG + "#5. Adresses", "12_PLAN_GRAMMAR.md#2. Adresses"],
  spec="""Dans `engine/addr.rs` (`pub mod addr;`): `pub(crate) fn resolve(cpu: &mut Cpu, a: Addr, port: bool) -> u16` appelee UNE fois au tick t0 d'un jeton de bus:
- PcInc: renvoie PC puis PC += 1 (wrapping). Sp: SP. SpInc: SP puis SP += 1. SpPlus1: SP+1. DecSp: SP -= 1 puis renvoie SP. Hl, Bc, De: valeurs. Nn: WZ. NnPlus1: WZ+1 (wrapping).
- Xy: `base = IX ou IY selon eng.prefix`; adresse = base + signed(eng.disp); stocke dans `eng.xy_addr` ET dans `regs.wz`. 
- An (port=true): `(A << 8) | TMP.L`. Bc (port=true): BC. (Pour `port=false` l'adresse `Bc` est la paire memoire BC.)
- None: renvoie `eng.xy_addr` ou 0 (non utilise).
Tous les calculs 16 bits wrapping.""",
  write=["crates/puce-z80/src/engine/addr.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("pc_inc", "PcInc: renvoie 0x1000 et PC=0x1001; 0xFFFF -> PC=0"), ("sp_inc", "SpInc: SP=0x2000 renvoie 0x2000 et SP=0x2001"), ("dec_sp", "DecSp: SP=0x2000 -> renvoie 0x1FFF et SP=0x1FFF; SP=0 -> 0xFFFF"),
         ("sp_plus1", "SpPlus1 ne modifie pas SP"), ("nn", "Nn/NnPlus1 depuis WZ=0xFFFF: 0xFFFF / 0x0000, WZ inchange"), ("xy_ix", "IX=0x1000, disp=0xFE (-2), Dd: 0x0FFE et WZ=0x0FFE"), ("xy_iy_pos", "IY=0xFFFF, disp=2 -> 0x0001"),
         ("an_port", "A=0x12, TMP.L=0x34 -> 0x1234"), ("bc_port", "port=true Bc -> BC")])

T("E03-06", "Lecture et ecriture des destinations (Reg)", kind="code", size=90,
  context="""Fonctions qui rangent la donnee lue dans la bonne cible (TMP, WZ, D, OP) et lisent la source d'une ecriture.""",
  embed=["12_PLAN_GRAMMAR.md#3. Registres"],
  spec="""Dans `engine/mod.rs` (impl Engine ou fonctions libres `pub(crate)`):
- `store(&mut self, regs: &mut Regs, r: Reg, v: u8)`: L -> tmp_l, H -> tmp_h, Z -> wz bas (`regs.wz = (regs.wz & 0xFF00) | v`), W -> wz haut, D -> disp, Op -> opb, None -> rien.
- `load(&self, regs: &Regs, r: Reg) -> u8`: lecture symetrique (L,H,Z,W,D,Op; None -> 0).""",
  write=["crates/puce-z80/src/engine/mod.rs"],
  tests=[("store_l_h", "store L puis H, load lit les memes valeurs"), ("store_z_w", "store Z=0x34 puis W=0x12 donne wz=0x1234"), ("store_d_op", "D et Op stockes dans disp/opb"), ("none_noop", "Reg::None ne change rien et load renvoie 0"), ("z_keeps_w", "store Z ne modifie pas l'octet haut de WZ")])

T("E03-07", "Boucle de tick: execution d'un plan de jetons", kind="code", size=150,
  context="""Coeur du moteur: execute un plan T par T (algorithme de la section 2 de `13_ENGINE.md`) SANS decodage (le decodage arrive en E03-09). Pour tester, on injecte un plan a la main.""",
  embed=[ENG + "#2. Algorithme"],
  reads=["`docs/specs/11_PINS_AND_TICK.md` section 2 (contrat de tick)", "E03-02..E03-06 (mcycle, addr, store/load)"],
  spec="""- Cree `crates/puce-z80/src/sem/mod.rs` (`pub(crate) mod sem;`): `pub(crate) fn sem(_cpu: &mut Cpu, _table: Table, _op: u8, _hook: u8) {}` (stub, rempli par E05+).
- Implemente `Cpu::tick` : etapes 2 a 6 de l'algorithme pour les jetons I, R, W, N, O (le jeton F et le decodage arrivent en E03-09: pour un jeton F, fais l'equivalent d'un `I` de la meme longueur).
  Au tick t0 du jeton: `addr = resolve(...)` (memorise dans un champ `eng.cur_addr: u16` que tu ajoutes a Engine), puis a chaque tick: `pins_out = *_pins(cur_addr, ..., t)` ; au T3 de `R`/`N` (voir ENGINE section 3) `latched = pins_in.data()` et `store(reg)`; pour `W`/`O` la donnee a ecrire est `load(reg)` fixee au t0.
  A la fin du jeton: avance `idx`, execute les hooks X (appelle `sem(self, table, op, hook_no)`), `active=false` quand le plan est termine.
- Pour les tests, ajoute `#[cfg(test)] impl Cpu { pub(crate) fn start_plan(&mut self, plan: Plan) }` (met `active=true, plan, idx=0, t=0` en executant les hooks initiaux).""",
  write=["crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/sem/mod.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("read_plan_3t", "plan [R3 Hl>L]: 3 ticks, au 3e la donnee presentee dans pins_in est rangee dans tmp_l, puis frontiere"), ("write_plan", "plan [W3 Hl<L] avec tmp_l=0x55: pins_out ecriture a HL de 0x55 au 2e tick (MREQ|WR)"),
         ("internal_plan", "[I5] dure 5 ticks sans controle"), ("two_cycles", "[R3 R3] dure 6 ticks"), ("hook_runs_after", "un hook X0 (sem espion) est appele dans le meme tick que le dernier T du jeton precedent"),
         ("io_plan", "[N4 An>L]: 4 ticks, IORQ|RD au 3e"), ("boundary_after_last", "apres le dernier T, at_instruction_boundary()==true"), ("xy_plan", "plan [R3 Xy>L] avec IX et disp: adresse de bus = IX+d et wz = IX+d")])

T("E03-08", "Jetons conditionnels (?)", kind="code", size=60,
  context="""Les jetons prefixes par `?` (champ `cond` du jeton) ne s'executent que si `eng.cond` est vrai.""",
  embed=["12_PLAN_GRAMMAR.md#4. Regles d'execution"],
  spec="""Dans la boucle de saut apres un jeton (etape 5 de ENGINE): un jeton avec `tok.cond == true` est SAUTE (0 T) si `eng.cond == false`. Applique aussi au premier jeton du plan (cas d'un plan dont le 1er jeton de bus est conditionnel apres des hooks). `eng.cond` est remis a `false` au debut de chaque instruction.
Ajoute `#[cfg(test)]` un espion de `sem`: `thread_local!`-free (pas de std dans le crate: utilise un champ `#[cfg(test)] pub(crate) spy: [u8; 8]` dans Cpu, ou un test qui positionne `cpu.eng.cond` directement entre deux ticks).""",
  write=["crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/cpu.rs"],
  tests=[("cond_false_skips", "plan [R3, ?I5, R3] avec cond=false: 6 ticks"), ("cond_true_runs", "meme plan, cond mis a vrai avant la fin du premier jeton: 11 ticks"), ("cond_reset", "cond est faux au debut d'une nouvelle instruction"), ("djnz_shape", "plan DJNZ (F5 R3 X0 ?I5 X1) avec cond=false: 8 ticks; avec cond vrai positionne par un hook: 13")])

T("E03-09", "WAIT: insertion de T-states d'attente", kind="code", size=90,
  context="""La broche WAIT permet a l'hote (ZX Spectrum, bornes d'arcade) de ralentir le CPU.""",
  embed=[ENG + "#3. Sorties de broches", "11_PINS_AND_TICK.md#3. Sequence"],
  spec="""Dans `tick`: au tick d'echantillonnage de WAIT (jetons F, R, W: `t==1`; jetons N, O: `t==2`, dont le TW automatique est deja dans la longueur 4), si `pins_in.has(Pins::WAIT)`: ne PAS avancer `t`, `ts_wait += 1`, renvoyer les memes sorties que ce T-state. Au tick suivant WAIT est re-echantillonne (meme `t`). Les jetons I et X n'echantillonnent pas WAIT.
Ajoute `pub fn wait_states(&self) -> u32` (dernier total d'attente de l'instruction courante/finie) et `pub fn instruction_tstates(&self) -> u32` (T-states sans attente).""",
  write=["crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("wait_extends_read", "[R3] avec WAIT actif a t1 pendant 2 ticks: dure 5 ticks, wait_states()==2"), ("wait_not_in_internal", "[I4]: WAIT ignore"), ("wait_io", "[N4]: WAIT echantillonne a t2 insere des T"), ("outputs_repeat", "pendant l'attente les sorties sont identiques a celles du tick t1"), ("no_wait_total", "instruction_tstates ignore l'attente")])

T("E03-10", "Decodage a l'execution: M1, prefixes, DDCB", kind="code", size=150,
  context="""Relie `decode` (E02-04) au moteur: le plan d'une instruction n'est connu qu'apres la lecture de l'opcode (section 4 de `13_ENGINE.md`).""",
  embed=[ENG + "#4. Decodage"],
  tsv=[("specs/data/ddfd.tsv", ["21", "CB", "09"]), ("specs/data/ddcb.tsv", ["06", "46"])],
  spec="""Dans `tick` pour un jeton `Kind::F`:
- tick t0: `regs.r = (regs.r & 0x80) | ((regs.r + 1) & 0x7F)`; adresse = PC (jeton `PcInc` implicite); PC += 1 apres emission.
- tick t2: `latched = pins_in.data()`; appelle `decode(eng.table, latched)` et applique la reaction decrite en section 4 (Plan / Prefix / IndexCb / Restart). Pour `Plan`, le jeton F courant est REMPLACE par `plan[fetches]` (longueur 4, 5 ou 6) avant de continuer (le tick suivant utilise la nouvelle longueur).
- Le M1 de prefixe: l'octet de prefixe DD/FD memorise `eng.prefix`; ED/CB mettent `prefix=None` sauf CB venant de DD/FD (IndexCb).
- IndexCb: jetons imposes `R3:PC+>D` puis `R5:PC+>OP` (construis un petit plan statique `DDCB_HEAD: [Tok;2]`), puis `plan_of(DdCb, opb)` et `idx = 4` (voir section 4).
Le moteur ne calcule toujours AUCUNE semantique (sem est vide): on teste les durees et les broches.""",
  write=["crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/engine/plan.rs"],
  tests=[("nop_4t", "memoire [00]: 4 ticks puis frontiere, PC=1, R=1"), ("ld_bc_nn_10t", "[01 34 12]: 10 T, PC=3"), ("cb_prefix_8t", "[CB 00]: 8 T, R incremente de 2, PC=2"), ("ed_nop_8t", "[ED 00]: 8 T"),
         ("dd_ld_ix_nn_14t", "[DD 21 34 12]: 14 T, PC=4"), ("dd_dd_chain", "[DD DD 21 34 12]: 18 T, R+3 (4 T de prefixe consommes)"), ("dd_ed", "[DD ED 44]: 4 + 8 = 12 T"),
         ("ddcb_bit_20t", "[DD CB 05 46]: 20 T, PC=4, la lecture de l'octet 46 est un R5"), ("fd_cb_rlc_23t", "[FD CB 02 06]: 23 T"), ("m1_extended", "[03] (INC BC, F6): 6 T"), ("refresh_bus", "au T3 d'un M1 l'adresse est I:R et RFSH|MREQ"), ("r_bit7_kept", "R=0xFF -> apres NOP R=0x80")])

T("E03-11", "Trait Bus, enum Cycle et SimpleRam", kind="code", size=110,
  context="""Interface de confort pour les hotes qui n'ont pas besoin de manipuler les broches. C'est l'API la plus utilisee par les emulateurs de consoles/arcade.""",
  embed=["01_ARCHITECTURE.md#4. API publique", "32_REUSE_GUIDE.md#"],
  spec="""Dans `bus.rs` (`pub mod bus;`, re-exports `Bus`, `Cycle`, `SimpleRam`): le trait EXACT de l'architecture (read, write, io_read, io_write, int_ack, extra_wait, irq, nmi, busrq, on_tick) avec les valeurs par defaut de la doc, `pub enum Cycle { Fetch, MemRead, MemWrite, IoRead, IoWrite, IntAck, Internal }` (derive Clone Copy Debug PartialEq Eq),
et `pub struct SimpleRam { pub mem: [u8; 65536] }` implementant `Bus` (lit/ecrit `mem`, `io_read` renvoie 0xFF). `SimpleRam::new()` toute la memoire a 0.
Attention taille: `[u8; 65536]` sur la pile est acceptable (64 Ko) mais fournis `SimpleRam::boxed()` seulement sous `#[cfg(feature = "std")]`.""",
  write=["crates/puce-z80/src/bus.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("defaults", "un Bus minimal ne definissant que read/write: io_read==0xFF, int_ack==0xFF, extra_wait==0, irq/nmi/busrq false"), ("ram_rw", "SimpleRam read/write"), ("cycle_enum", "Cycle derive Debug/PartialEq"), ("ram_zero", "SimpleRam::new() est a zero")])

T("E03-12", "Cpu::step: instruction complete via tick", kind="code", size=120,
  context="""`step()` est le mode confort: il boucle sur `tick` en servant la memoire et les ports par le trait `Bus` (une seule implementation de timing, D-002).""",
  embed=["01_ARCHITECTURE.md#4. API publique", "11_PINS_AND_TICK.md#2. API"],
  spec="""`pub fn step<B: Bus>(&mut self, bus: &mut B) -> u32`: execute jusqu'a la prochaine frontiere d'instruction, renvoie le nombre de T-states (attente incluse).
Boucle: `pins_in` = precedent `pins_out` avec: si `MREQ|RD` -> `data = bus.read(addr)` (a fournir AU TICK SUIVANT dans `pins_in`: le CPU latche au T3); si `IORQ|RD` et pas `M1` -> `bus.io_read(addr)`; si `MREQ|WR` -> `bus.write(addr, data)` (une seule fois par cycle: au tick ou WR apparait); idem `IORQ|WR` -> `bus.io_write`.
`wait = bus.extra_wait(cycle_kind, addr)`: au 1er tick d'un cycle (A valide, t0) demande les T d'attente et maintient `Pins::WAIT` pendant ce nombre de ticks au moment de l'echantillonnage. Appelle `bus.on_tick(pins_out)` a chaque tick. `irq()/nmi()/busrq()` du Bus sont traduits en broches INT/NMI/BUSRQ dans `pins_in` (traitement par E09; ici seulement la transmission).
Garde contre boucle infinie: `debug_assert!` si plus de 100 ticks sans frontiere.""",
  write=["crates/puce-z80/src/cpu.rs"],
  tests=[("step_nop", "RAM [00]: step renvoie 4"), ("step_ld_bc_nn", "[01 34 12]: 10 T, memoire lue aux bonnes adresses (bus espion enregistre les lectures: 0,1,2)"), ("step_write_logged", "[DD 36 05 77] avec un bus espion: exactement 1 appel write(IX+5, tmp_l) (la semantique est absente donc tmp_l vaut 0x77 car lu par R5 dans TMP.L)"), ("extra_wait_adds", "un bus avec extra_wait=2 sur Fetch: NOP dure 6"), ("on_tick_count", "on_tick appele autant de fois que de T-states (4 pour NOP)"), ("io_read_called", "[DB 10] appelle io_read une fois"),
         ("returns_tstates", "step renvoie la somme T + attente")])

T("E03-13", "Test de duree: toutes les instructions (semantique absente)", kind="code", size=100,
  context="""Verifie TOUT le timing des plans avant d'ecrire la moindre semantique: avec `sem` vide, `cond` reste faux, donc chaque instruction doit durer `t_min` de sa table.""",
  tsv=[("specs/data/main.tsv", ["00", "10", "20", "C7", "CD"])],
  spec="""Test d'integration `crates/puce-z80/tests/timing_stub.rs` (via `__internal` si besoin). Pour chaque table et chaque opcode non prefixe: place les octets `[prefixes..., opcode, 0x00, 0x00, 0x00]` en RAM (SimpleRam), PC=0x1000, SP=0x8000, execute `step`, compare le resultat a `t_min` du TSV (colonne t_min; lire `docs/specs/data/*.tsv`).
Cas: main (252 hors prefixes), cb (256), ed (256), ddfd (252, prefixe DD puis FD), ddcb (256: octets `DD CB 00 op`, puis `FD CB 00 op`). Les opcodes HALT (76) sont executes 1 fois (4 T).
En cas d'ecart: le message liste table, opcode, mnemonique, attendu, obtenu.""",
  write=["crates/puce-z80/tests/timing_stub.rs"],
  tests=[("main", "252 opcodes de la table main durent t_min"), ("cb", "256 opcodes CB"), ("ed", "256 opcodes ED"), ("ddfd", "252 opcodes DD et 252 FD"), ("ddcb", "256 opcodes DDCB et 256 FDCB")],
  notes="Un echec ici signale un bug du moteur (E03-05..10) ou de la table generee: corrige le moteur, pas la table (la table est verifiee contre la reference).")

T("E03-14", "Journal de broches par T-state (outil de debug)", kind="code", size=80,
  context="""Outil pour les futures taches de calage (SingleStepTests) : enregistrer les broches de chaque T-state d'une instruction.""",
  spec="""Dans `crates/puce-z80/tests/common/trace.rs` (module de test partage, `mod common;` dans les tests): `pub fn trace_instruction(bytes: &[u8], setup: impl Fn(&mut Cpu)) -> Vec<Pins>` execute une instruction sur une RAM de test et renvoie les `Pins` de sortie de chaque tick; `pub fn fmt_trace(&[Pins]) -> String` (une ligne par T: `T3 A=0001 D=34 M1 MREQ RD`).
Test d'integration `crates/puce-z80/tests/trace_basic.rs`.""",
  write=["crates/puce-z80/tests/common/**", "crates/puce-z80/tests/trace_basic.rs"],
  tests=[("nop_trace_len", "4 lignes pour NOP"), ("nop_trace_fetch", "ligne 2 contient M1 MREQ RD"), ("refresh_line", "ligne 3 contient RFSH"), ("fmt_has_t", "fmt_trace commence par `T1`"), ("ld_rp_nn_trace", "10 lignes")])

T("E03-15", "GATE E03 - moteur T-state valide", gate=True, kind="gate", size=0,
  context="""Le moteur execute tous les plans avec la bonne duree (sans semantique). Verifier a l'oeil la trace de broches d'un M1 et d'une instruction prefixee.""",
  user_cmds=["cargo test -p puce-z80", "cargo test -p puce-z80 --test timing_stub", "cargo test -p puce-z80 --test trace_basic -- --nocapture", "bash tools/ci.sh"],
  accept=[])
