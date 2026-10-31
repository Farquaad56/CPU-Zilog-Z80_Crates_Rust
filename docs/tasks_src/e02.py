from _dsl import T

T("E02-01", "Types de plan (Tok, Kind, Addr, Reg, Plan)", kind="code", size=120,
  context="""Le moteur execute des PLANS (micro-programmes) decrits par des jetons. Cette tache cree les types Rust utilises par les tables generees (`gen_rust_tables.py` emet `Tok::new(Kind::F, 4, Addr::None, Reg::None, false)`). Les noms des variantes sont IMPOSES par le generateur.""",
  embed=["12_PLAN_GRAMMAR.md#1. Jetons", "12_PLAN_GRAMMAR.md#2. Adresses", "12_PLAN_GRAMMAR.md#3. Registres"],
  reads=["`docs/tools/gen_rust_tables.py` (dictionnaires ADDR et KIND: noms exacts des variantes)"],
  spec="""Cree `crates/puce-z80/src/engine/mod.rs` (vide sauf `pub mod plan;`) et `engine/plan.rs`; dans lib.rs: `pub(crate) mod engine;`.
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Kind { F, R, W, N, O, I, X }   // F=M1 R=lecture W=ecriture N=lecture port O=ecriture port I=interne X=hook
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Addr { None, PcInc, Sp, SpInc, SpPlus1, DecSp, Hl, Bc, De, Nn, NnPlus1, Xy, An }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Reg { None, L, H, Z, W, D, Op }     // cible/source: TMP.L TMP.H WZ.Z WZ.W deplacement octet-opcode-donnee
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Tok { pub kind: Kind, pub len: u8, pub addr: Addr, pub reg: Reg, pub cond: bool }
impl Tok { pub const fn new(kind: Kind, len: u8, addr: Addr, reg: Reg, cond: bool) -> Tok }
pub type Plan = &'static [Tok];
impl Tok { pub const fn is_hook(&self) -> bool; pub const fn is_bus(&self) -> bool /* F R W N O */ }
pub fn plan_t_min(p: Plan) -> u32   // somme des len des jetons NON conditionnels
pub fn plan_t_max(p: Plan) -> u32   // somme de tous les jetons
```
Pour un hook, le generateur emet `Tok::new(Kind::X, 0, ...)` pour X0 et `Tok::new(Kind::X, 1, ...)` pour X1. Documente-le: pour `Kind::X`, `len` = numero du hook (0 ou 1) et dure 0 T; ajoute `pub const fn hook_no(&self) -> u8`. `plan_t_*` doivent donc ignorer `Kind::X`.""",
  write=["crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/engine/plan.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("tok_new_const", "construire un Tok dans une `const` compile"), ("t_min_max", "plan [F4, R3, ?I5(cond), X0] -> t_min 7, t_max 12 (les X comptent 0)"), ("hook_no", "X0 -> 0, X1 -> 1"), ("is_bus", "F R W N O sont is_bus; I et X non")])

T("E02-02", "Generer et integrer les tables de plans", kind="tooling", size=40,
  context="""Les 5 tables de plans (`main`, `cb`, `ed`, `ddfd`, `ddcb`) sont GENEREES depuis `docs/specs/data/*.tsv`: personne ne les ecrit a la main.""",
  spec="""1. Lance `python docs/tools/gen_rust_tables.py` depuis la racine du depot (cree `crates/puce-z80/src/tables/{main,cb,ed,ddfd,ddcb,mnem,mod}.rs`).
2. Dans lib.rs ajoute `pub(crate) mod tables;`. Le fichier `tables/mnem.rs` est sous `#[cfg(feature = "disasm")]` (declare dans `tables/mod.rs`).
3. Verifie que `cargo build -p puce-z80` ET `cargo build -p puce-z80 --features disasm` compilent sans avertissement. Si un avertissement `dead_code` apparait, ajoute `#![allow(dead_code)]` EN TETE de `tables/mod.rs` UNIQUEMENT (fichier genere: modifie plutot le generateur `docs/tools/gen_rust_tables.py` si besoin, et relance-le).
4. Ne modifie JAMAIS a la main les fichiers generes. Si le generateur est faux, pose une question Q&R.""",
  write=["crates/puce-z80/src/tables/**", "crates/puce-z80/src/lib.rs", "docs/tools/gen_rust_tables.py"],
  accept=["python docs/tools/gen_rust_tables.py", "cargo build -p puce-z80", "cargo build -p puce-z80 --features disasm"])

T("E02-03", "Tests de coherence des tables", kind="code", size=100,
  context="""Verifier que les tables generees sont completes et coherentes avec les TSV (garde-fou contre un generateur defectueux).""",
  embed=["12_PLAN_GRAMMAR.md#4. Regles d'execution", "12_PLAN_GRAMMAR.md#5. Tables"],
  tsv=[("specs/data/main.tsv", ["00", "01", "10", "CB", "DD"]), ("specs/data/ddfd.tsv", ["21", "CB"])],
  spec="""Ajoute dans `tables/` UNE fonction accessible aux tests: dans un NOUVEAU fichier `crates/puce-z80/src/tables_check.rs` (`#[cfg(test)] mod tables_check;` dans lib.rs) ecris les tests ci-dessous.
Les statiques generees sont `tables::main::MAIN`, `...::MAIN_T` (tuples (t_min,t_max)), idem `CB/CB_T`, `ED/ED_T`, `DDFD/DDFD_T`, `DDCB/DDCB_T`. Les lignes de prefixe (main CB DD ED FD; ddfd CB DD ED FD) ont un plan VIDE et (0,0).""",
  write=["crates/puce-z80/src/tables_check.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("sizes", "les 5 tables ont 256 entrees"), ("first_token_is_fetch", "pour toute entree NON vide, le premier jeton est de Kind::F"),
         ("t_sums_match", "pour chaque table et chaque opcode: plan_t_min == t_min du tableau _T et plan_t_max == t_max"),
         ("prefix_rows_empty", "main[0xCB], main[0xDD], main[0xED], main[0xFD] sont vides; ddfd[0xCB/DD/ED/FD] vides"),
         ("djnz_plan", "main[0x10] = [F5, R3 PcInc->L, X0, ?I5, X1] (t 8..13)"), ("ld_rp_nn", "main[0x01] a 4 jetons et t 10"),
         ("ddfd_ld_ix_nn", "ddfd[0x21] a pour plan F4 F4 R3 R3 X0 (14 T)"), ("hooks_balanced", "dans chaque plan X1 n'apparait jamais avant X0")])

T("E02-04", "Selection de table et prefixes (decode.rs)", kind="code", size=120,
  context="""Le decodage ne depend que de l'octet lu par le M1 et de l'etat de prefixe. Fonction PURE, testee sans CPU.""",
  embed=["23_UNDOCUMENTED.md#"],
  spec="""Dans `engine/decode.rs` (`pub mod decode;` dans engine/mod.rs):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Table { Main, Cb, Ed, DdFd, DdCb }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Prefix { None, Dd, Fd }
pub fn plan_of(table: Table, op: u8) -> Plan            // renvoie les tables generees (plan vide pour les prefixes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoded { Plan(Table, Plan),   // instruction identifiee
                   Prefix(Table),       // l'octet est un prefixe: la table suivante est Cb, Ed ou DdFd (le CPU relira un M1)
                   IndexCb,             // octet CB apres DD/FD: mode DDCB (d puis opcode lus comme donnees)
                   Restart(u8) }        // DD/FD/ED apres DD/FD: le prefixe precedent est consomme (4 T), l'octet recommence une decode
pub fn decode(table: Table, byte: u8) -> Decoded
```
Regles: dans `Main`: CB->Prefix(Cb), ED->Prefix(Ed), DD/FD->Prefix(DdFd) (le choix IX/IY est memorise par le moteur, pas ici), sinon Plan(Main, plan). Dans `Cb`, `Ed`: toujours Plan. Dans `DdFd`: CB->IndexCb; DD, FD, ED -> Restart(byte); sinon Plan(DdFd, plan).
`DdCb` n'est jamais renvoye par `decode` (son plan est choisi apres lecture de l'octet d'operation: `plan_of(Table::DdCb, op)`).""",
  write=["crates/puce-z80/src/engine/decode.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("main_plain", "decode(Main, 0x00) -> Plan(Main, plan de NOP)"), ("main_prefixes", "CB->Prefix(Cb) ED->Prefix(Ed) DD->Prefix(DdFd) FD->Prefix(DdFd)"),
         ("ddfd_cb", "decode(DdFd, 0xCB) -> IndexCb"), ("ddfd_restart", "decode(DdFd, 0xDD/0xFD/0xED) -> Restart(byte)"), ("ddfd_plain", "decode(DdFd, 0x21) -> Plan(DdFd, 14T)"),
         ("ed_nop", "decode(Ed, 0x00) -> Plan avec t 8"), ("ddcb_plan", "plan_of(DdCb, 0x46) = BIT 0,(IX+d), t 20")])

T("E02-05", "Tests de decode sur les 1280 combinaisons", kind="code", size=70,
  context="""Balayage exhaustif de `decode` contre les TSV (aucune ligne ne doit etre oubliee).""",
  reads=["docs/specs/data/{main,cb,ed,ddfd,ddcb}.tsv"],
  spec="""Test d'integration `crates/puce-z80/tests/decode_all.rs` (le crate expose `pub use engine::decode::{decode, Decoded, Table, plan_of}` sous `#[doc(hidden)] pub mod __internal` pour les tests; ajoute ce module dans lib.rs).
Le test lit les 5 TSV (chemin `concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/specs/data/<nom>.tsv")`) et pour chaque ligne verifie: la somme des durees du plan (min/max) == colonnes t_min/t_max; pour les lignes `prefix`/`-`: `decode` renvoie Prefix/IndexCb/Restart selon la regle de E02-04; sinon `decode` renvoie `Plan` avec `plan.len()` == nombre de jetons de la colonne plan.""",
  write=["crates/puce-z80/tests/decode_all.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("all_rows_main", "256 lignes main"), ("all_rows_cb_ed", "512 lignes cb+ed"), ("all_rows_ddfd_ddcb", "512 lignes ddfd+ddcb"), ("token_counts", "nombre de jetons == colonne plan pour toutes les lignes")])
