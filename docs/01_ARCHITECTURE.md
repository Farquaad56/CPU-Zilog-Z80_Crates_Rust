# 01 - ARCHITECTURE (crate Z80 cycle perfect)

## 1. Decisions de cadrage (detail dans docs/qa/DECISIONS.md)

| ID | Decision |
|---|---|
| D-002 | Precision **T-state par T-state**: `Cpu::tick(pins)` = 1 T-state. Chaque cycle machine (M1, lecture, ecriture, E/S, interne, acquittement INT) avec ses broches, WAIT/INT/NMI/BUSRQ echantillonnes au bon T. |
| D-003 | Validation par TOUTES ces suites: PRELIM, ZEXDOC, ZEXALL, z80test (Patrik Rak), SingleStepTests/z80, FUSE (tests.in/expected) + ROMs de timing (Woodmass/Raxoft). |
| D-004 | Frontend = **page locale servie par Rust** (`puce-z80-web`: HTTP + WebSocket sur 127.0.0.1), connectee au VRAI crate (aucune logique CPU en JavaScript). Design de reference: `docs/frontend/index.html` (titre "Zilog Z80 Emulator", interface bilingue FR/EN). |
| D-005 | Reutilisable dans des emulateurs de consoles ET de bornes d'arcade: `no_std`, sans `alloc`, sans `unsafe`, sans dependance; API a broches (tick) + trait `Bus` de confort. |
| D-006 | CPU par defaut = **NMOS Zilog Z80** (documente + non documente, MEMPTR, facteur Q). Variante CMOS: differee (E17). |
| D-009 | `puce-z80` (le crate): entiers uniquement, deterministe, `#![forbid(unsafe_code)]`, `#![no_std]`. |
| D-011 | Logique positive pour toutes les broches (bit a 1 = actif), voir `specs/11_PINS_AND_TICK.md`. |
| D-012 | Crate `puce-z80-machines` (std) = machines d'essai partagees (CP/M, ZX minimal, session) : la logique de machine n'est jamais dans `puce-z80`. |
| D-020 | Execution strictement sequentielle: UNE tache a la fois, dans l'ordre de `docs/tasks/ORDER.md`. |

## 2. Workspace Cargo

```
Cargo.toml                      [workspace] members = crates/*
crates/puce-z80/                lib no_std (le crate reutilisable); features: inspect, disasm
crates/puce-z80-machines/       lib (std): machines d'essai reutilisees par cli et web: cpm (CP/M + BDOS), zx (ZX Spectrum 48K minimal), session (thread de simulation + instantanes)
crates/puce-z80-cli/            binaire `z80`: run | cpm | suite | ssc | fuse | z80test | disasm | fetch (clap)
crates/puce-z80-web/            binaire `z80-web`: serveur HTTP+WebSocket + page embarquee (include_str!)
tests/                          manifest.toml, baseline.json, roms/README.md (ROMs telechargees, jamais commitees)
docs/                           toute la documentation
docs/ref/z80_ref.js             coeur de reference (passe ZEXDOC/ZEXALL) = oracle de SEMANTIQUE
```

Dependances autorisees (toute autre = question Q&R avant ajout):
- `puce-z80`: AUCUNE. Dev-deps: `serde`, `serde_json` (lecture des JSON SingleStepTests dans les tests).
- `puce-z80-machines`: `puce-z80` (features inspect, disasm, std), `serde`, `serde_json`.
- `puce-z80-cli`: `puce-z80-machines`, `clap` (derive), `serde`, `serde_json`.
- `puce-z80-web`: `puce-z80-machines`, `tiny_http`, `tungstenite`, `serde`, `serde_json`, `base64`.

## 3. Arborescence de `puce-z80/src/`

```
lib.rs          #![no_std] #![forbid(unsafe_code)], re-exports, VERSION
pins.rs         struct Pins(u64) + masques + accesseurs
regs.rs         struct Regs (AF BC DE HL AF' BC' DE' HL' IX IY SP PC I R WZ, IFF1 IFF2 IM, Q), constantes de drapeaux
bus.rs          trait Bus (confort) + struct SimpleRam (tests) 
cpu.rs          struct Cpu, new/reset/tick/step/at_instruction_boundary
engine/mod.rs   etat du moteur (table courante, opcode, plan, index de jeton, T courant, drapeau cond, tmp)
engine/plan.rs  type Tok (jetons de 12_PLAN_GRAMMAR) + Plan = &'static [Tok]
engine/mcycle.rs   sorties de broches T par T pour chaque type de cycle (11_PINS_AND_TICK)
engine/decode.rs   choix de table (prefixes DD FD ED CB DDCB) + recherche du plan
alu.rs          fonctions pures de drapeaux (20_FLAGS_AND_ALU.md)
sem/mod.rs      fn sem(cpu, table, opcode, hook): dispatch vers les familles
sem/{ld,arith,logic,rot,bits,jump,stack,exchange,block,io,idx,misc}.rs
int.rs          NMI, INT (IM0/1/2), HALT, RESET, EI delay, BUSRQ/BUSAK
tables/{main,cb,ed,ddfd,ddcb}.rs   GENERES par docs/tools/gen_rust_tables.py (ne pas editer a la main)
probe.rs        (feature "inspect") journal ALU, derniere operation, historique PC, journal d'ecritures
disasm.rs       (feature "disasm") desassembleur
```

## 4. API publique (resume)

```rust
pub struct Pins(pub u64);
pub struct Cpu { pub regs: Regs, /* moteur prive */ }
pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
    fn io_read(&mut self, _port: u16) -> u8 { 0xFF }
    fn io_write(&mut self, _port: u16, _data: u8) {}
    fn int_ack(&mut self) -> u8 { 0xFF }                  // octet place sur le bus pendant l'acquittement INT
    fn extra_wait(&mut self, _kind: Cycle, _addr: u16) -> u8 { 0 }   // T-states d'attente (contention ZX, WAIT arcade)
    fn irq(&mut self) -> bool { false }  fn nmi(&mut self) -> bool { false }  fn busrq(&mut self) -> bool { false }
    fn on_tick(&mut self, _pins: Pins) {}                 // appele a CHAQUE T-state (horloge des peripheriques)
}
pub enum Cycle { Fetch, MemRead, MemWrite, IoRead, IoWrite, IntAck, Internal }
impl Cpu { pub fn new() -> Cpu; pub fn reset(&mut self); pub fn tick(&mut self, pins: Pins) -> Pins;
           pub fn step<B: Bus>(&mut self, bus: &mut B) -> u32; pub fn at_instruction_boundary(&self) -> bool; }
```

`step()` est implemente au-dessus de `tick()` (une seule logique): il boucle `tick`, sert la memoire/ports via `Bus` selon les broches, insere `extra_wait`, et s'arrete a la frontiere d'instruction. Le CPU n'a donc qu'UNE implementation de timing.

## 5. Principe du moteur

1. Chaque instruction = un plan (jetons de `specs/12_PLAN_GRAMMAR.md`), choisi par (table, opcode) apres decodage des prefixes.
2. `tick` avance d'un T-state dans le jeton courant (`engine/mcycle.rs` donne les broches de ce T).
3. Les hooks `X0`/`X1` appellent `sem(...)` (semantique portee depuis `docs/ref/z80_ref.js`).
4. Les interruptions sont evaluees a la frontiere d'instruction (`int.rs`).
5. `feature = "inspect"` ajoute des compteurs/journaux (ALU, historique, ecritures) sans changer le comportement; desactivee par defaut (zero cout).

## 6. Frontend

`puce-z80-web` pilote un `Cpu` reel (via `puce-z80-machines::session`, thread de simulation, machine CP/M: 64 Ko, BDOS emule en 0005h) et pousse des instantanes JSON a la page via WebSocket (`specs/31_FRONTEND_PROTOCOL.md`). La page n'effectue AUCUN calcul de CPU.
