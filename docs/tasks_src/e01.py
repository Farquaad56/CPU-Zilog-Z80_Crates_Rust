from _dsl import T

T("E01-01", "Squelette du crate puce-z80", kind="code", size=60,
  context="""Premier fichier du projet. Le workspace racine (`Cargo.toml`, members = crates/*) existe deja. Cree le crate bibliotheque reutilisable `puce-z80`
(Z80 cycle perfect, reutilisable par des emulateurs de consoles et de bornes d'arcade, D-005): `no_std`, sans alloc, sans unsafe, sans dependance (D-009).""",
  embed=["01_ARCHITECTURE.md#2. Workspace Cargo", "01_ARCHITECTURE.md#3. Arborescence"],
  spec="""Cree:
- `crates/puce-z80/Cargo.toml`: `name = "puce-z80"`, `version.workspace = true`, `edition.workspace = true`, `license.workspace = true`, `description`,
  `[features] default = []`, `inspect = []`, `disasm = []`, `std = []`; `[dependencies]` VIDE; `[dev-dependencies] serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.
- `crates/puce-z80/src/lib.rs`: `#![no_std]` (sauf `#[cfg(test)]`/feature std: `#[cfg(any(test, feature = "std"))] extern crate std;`), `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]` (documente chaque element public),
  `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`, et les declarations de modules COMMENTEES (une ligne `// pub mod pins;` par module de l'arborescence, elles seront decommentees par les taches suivantes).
Rien d'autre. Pas de logique.""",
  write=["crates/puce-z80/Cargo.toml", "crates/puce-z80/src/lib.rs", "Cargo.lock"],
  tests=[("version_const", "`VERSION` est non vide et egal a `0.1.0`"), ("no_std_attr", "le fichier `src/lib.rs` contient `#![no_std]` et `#![forbid(unsafe_code)]` (lecture du source via `include_str!`)")],
  accept=["cargo build -p puce-z80", "cargo build -p puce-z80 --features inspect,disasm", "cargo test -p puce-z80 e01_01_"])

T("E01-01a", "Squelette du crate puce-z80-machines", kind="code", size=50, crate="puce-z80-machines",
  context="""Bibliotheque `std` qui contiendra les machines d'essai (CP/M, ZX minimal) et la session de simulation reutilisees par la CLI et le serveur web (D-012). Ici: squelette seulement.""",
  embed=["01_ARCHITECTURE.md#2. Workspace Cargo"],
  spec="""- `crates/puce-z80-machines/Cargo.toml`: package `puce-z80-machines`, dependances `puce-z80 = { path = "../puce-z80", features = ["inspect", "disasm", "std"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.
- `src/lib.rs`: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, `pub mod cpm; pub mod zx; pub mod session;` avec trois fichiers vides documentes (`//! ...`), et `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`.""",
  write=["crates/puce-z80-machines/**", "Cargo.lock"],
  tests=[("version", "VERSION non vide")],
  accept=["cargo build -p puce-z80-machines", "cargo test -p puce-z80-machines e01_01a_"])

T("E01-02", "Squelette du binaire puce-z80-cli", kind="code", size=70, crate="puce-z80-cli",
  context="""Binaire `z80` (outil en ligne de commande qui servira a lancer les suites de test, E13). Ici: uniquement le squelette `clap` avec les sous-commandes declarees mais non implementees.""",
  embed=["01_ARCHITECTURE.md#2. Workspace Cargo"],
  spec="""- `crates/puce-z80-cli/Cargo.toml`: package `puce-z80-cli`, `[[bin]] name = "z80"`, dependances `puce-z80 = { path = "../puce-z80", features = ["inspect", "disasm", "std"] }`, `clap = { version = "4", features = ["derive"] }`, `serde`, `serde_json`.
- `src/main.rs`: struct `Cli` (clap derive) avec sous-commandes: `Run { rom: PathBuf }`, `Cpm { rom: PathBuf, max_instr: Option<u64> }`, `Suite { name: String }`, `Ssc { dir: PathBuf, filter: Option<String> }`, `Fuse { dir: PathBuf }`, `Z80test { rom: PathBuf }`, `Disasm { file: PathBuf, org: String }`, `Fetch`.
  Chaque sous-commande appelle `commands::<nom>::run(args) -> Result<(), String>` qui renvoie pour l'instant `Err("not implemented".into())` (un fichier `src/commands/<nom>.rs` par commande, ou un seul `commands.rs`).
  `main` affiche l'erreur sur stderr et sort avec le code 1.""",
  write=["crates/puce-z80-cli/**", "Cargo.lock"],
  tests=[("parse_cpm", "`Cli::try_parse_from([\"z80\",\"cpm\",\"x.com\"])` reussit et donne la sous-commande Cpm"), ("unknown_cmd", "une sous-commande inconnue echoue au parsing")],
  accept=["cargo build -p puce-z80-cli", "cargo test -p puce-z80-cli e01_02_", "cargo run -q -p puce-z80-cli -- --help"])

T("E01-03", "Squelette du binaire puce-z80-web", kind="code", size=50, crate="puce-z80-web",
  context="""Binaire `z80-web` (serveur du frontend, E14). Ici: squelette et analyse des arguments uniquement. Dependances reseau autorisees: `tiny_http`, `tungstenite`, `base64` (D-004).""",
  embed=["01_ARCHITECTURE.md#6. Frontend"],
  spec="""- `crates/puce-z80-web/Cargo.toml`: package `puce-z80-web`, `[[bin]] name = "z80-web"`, dependances: `puce-z80` (path, features inspect+disasm+std), `puce-z80-machines` (path), `tiny_http = "0.12"`, `tungstenite = "0.24"`, `serde`, `serde_json`, `base64 = "0.22"`.
- `src/main.rs` + `src/args.rs`: `struct Args { port: u16 (defaut 8080), roms_dir: PathBuf (defaut "tests/roms") }`, parseur MANUEL (pas de clap): `--port N`, `--roms DIR`, `--help`. Pas de serveur ici: `main` affiche `z80-web: port N, roms DIR` puis sort.
  `fn parse_args(args: &[String]) -> Result<Args, String>`.""",
  write=["crates/puce-z80-web/**", "Cargo.lock"],
  tests=[("defaults", "`parse_args(&[])` donne port 8080 et roms_dir `tests/roms`"), ("port_ok", "`--port 9000` donne 9000"), ("port_bad", "`--port abc` renvoie Err"), ("unknown_flag", "`--foo` renvoie Err")],
  accept=["cargo build -p puce-z80-web", "cargo test -p puce-z80-web e01_03_"])

T("E01-04", "Scripts de CI et d'hygiene", kind="tooling", size=60, crate="puce-z80",
  context="""Scripts shell portables (git-bash/Linux) qui verifient les regles du projet: formatage, clippy sans avertissement, tests, et hygiene du crate reutilisable.""",
  spec="""Cree:
- `tools/ci.sh`: `set -e`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`.
- `tools/hygiene.sh`: sur `crates/puce-z80/src` UNIQUEMENT, echoue (exit 1, message clair avec fichier:ligne) si on trouve: `unsafe` (hors commentaires), `f32`, `f64`, `std::` (sauf apres un `#[cfg(test)]` ou `#[cfg(any(test, feature = "std"))]` dans le meme fichier, ce qui couvre `extern crate std;`), `println!`, `unwrap()` hors tests, des caracteres non ASCII (`grep -nP '[^\\x00-\\x7F]'`), ou `TODO` sans `UNKNOWN - to confirm`.
  Doit passer sur le squelette actuel.
- `tools/hygiene_selftest.sh`: cree un fichier temporaire contenant `let x: f32 = 1.0;` dans un dossier temporaire, lance la verification dessus et verifie qu'elle ECHOUE (le script hygiene accepte un dossier en argument, defaut `crates/puce-z80/src`).""",
  write=["tools/**"],
  tests=[],
  accept=["bash tools/hygiene.sh", "bash tools/hygiene_selftest.sh"])

T("E01-05", "Type Pins (broches du Z80)", kind="code", size=110,
  context="""Representation des broches du CPU sur 64 bits, en logique positive (D-011). C'est l'interface a la fois du CPU (`tick`) et des hotes (emulateurs de consoles/arcade).""",
  embed=["11_PINS_AND_TICK.md#1. Type `Pins`"],
  spec="""Dans `crates/puce-z80/src/pins.rs` (decommente `pub mod pins;` dans lib.rs et re-exporte `pub use pins::Pins;`):
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pins(pub u64);
impl Pins {
    pub const A_MASK: u64 = 0xFFFF; pub const D_SHIFT: u32 = 16; pub const D_MASK: u64 = 0xFF << 16;
    pub const M1: u64 = 1 << 24; pub const MREQ: u64 = 1 << 25; pub const IORQ: u64 = 1 << 26; pub const RD: u64 = 1 << 27; pub const WR: u64 = 1 << 28;
    pub const HALT: u64 = 1 << 29; pub const WAIT: u64 = 1 << 30; pub const INT: u64 = 1 << 31; pub const NMI: u64 = 1 << 32;
    pub const RFSH: u64 = 1 << 33; pub const BUSRQ: u64 = 1 << 34; pub const BUSAK: u64 = 1 << 35; pub const RESET: u64 = 1 << 36;
    pub const fn addr(self) -> u16; pub const fn data(self) -> u8; pub const fn set_addr(self, a: u16) -> Pins; pub const fn set_data(self, d: u8) -> Pins;
    pub const fn has(self, mask: u64) -> bool;   // TOUS les bits du masque sont a 1
    pub const fn with(self, mask: u64) -> Pins;  pub const fn without(self, mask: u64) -> Pins;
    pub const fn is_mem_read(self) -> bool;  // MREQ et RD
    pub const fn is_mem_write(self) -> bool; pub const fn is_io_read(self) -> bool; pub const fn is_io_write(self) -> bool;
    pub const fn is_fetch(self) -> bool;     // M1 et MREQ et RD
    pub const fn is_int_ack(self) -> bool;   // M1 et IORQ
}
impl core::fmt::LowerHex for Pins {}  // affiche `0x` + hex de la valeur u64 (utile au debug)
```
Toutes les fonctions sont `const fn`, documentees.""",
  write=["crates/puce-z80/src/pins.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("addr_data_roundtrip", "set_addr/set_data puis addr()/data() pour 0x0000, 0xFFFF, 0x1234 et 0x00, 0xFF, 0xA5; les autres bits ne changent pas"), ("masks_distinct", "les 13 masques de controle sont deux a deux disjoints et disjoints de A et D"),
         ("fetch_predicate", "M1|MREQ|RD -> is_fetch vrai; MREQ|RD seul -> is_fetch faux, is_mem_read vrai"), ("int_ack_predicate", "M1|IORQ -> is_int_ack vrai; IORQ|RD -> is_io_read vrai"),
         ("with_without", "with puis without restitue la valeur initiale"), ("hex_fmt", "format!(\"{:x}\", Pins(0x1F)) == \"0x1f\"")])

T("E01-06", "Type Regs (registres et drapeaux)", kind="code", size=140,
  context="""Etat des registres visible de l'exterieur (debuggers, sauvegardes d'etat, frontend). Les registres 8 bits sont stockes separement; les paires sont des accesseurs.""",
  embed=["01_ARCHITECTURE.md#3. Arborescence"],
  spec="""Dans `crates/puce-z80/src/regs.rs` (`pub mod regs;`, re-export `Regs`):
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs { pub a: u8, pub f: u8, pub b: u8, pub c: u8, pub d: u8, pub e: u8, pub h: u8, pub l: u8,
  pub a2: u8, pub f2: u8, pub b2: u8, pub c2: u8, pub d2: u8, pub e2: u8, pub h2: u8, pub l2: u8,
  pub ix: u16, pub iy: u16, pub sp: u16, pub pc: u16, pub i: u8, pub r: u8, pub wz: u16,
  pub iff1: bool, pub iff2: bool, pub im: u8, pub q: u8, pub halted: bool }
pub mod flag { pub const S: u8 = 0x80; pub const Z: u8 = 0x40; pub const Y: u8 = 0x20; pub const H: u8 = 0x10; pub const X: u8 = 0x08; pub const PV: u8 = 0x04; pub const N: u8 = 0x02; pub const C: u8 = 0x01; }
impl Regs {
  pub fn af(&self)->u16; pub fn bc; pub fn de; pub fn hl;  pub fn set_af(&mut self, v: u16); set_bc; set_de; set_hl;
  pub fn af2; bc2; de2; hl2 et set_*;   // jumeaux
  pub fn ixh(&self)->u8; ixl; iyh; iyl; set_ixh(&mut self, v: u8); set_ixl; set_iyh; set_iyl;
  pub fn exx(&mut self);      // echange BC DE HL avec BC' DE' HL'
  pub fn ex_af(&mut self);    // echange AF avec AF'
  pub fn ex_de_hl(&mut self);
}
```
`Regs::default()` = tout a zero. Aucun code de reset ici (E09).""",
  write=["crates/puce-z80/src/regs.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("pair_roundtrip", "set_bc(0x1234) donne b=0x12 c=0x34 et bc()==0x1234, idem af de hl et jumeaux"), ("ixh_ixl", "set_ixh/set_ixl modifient ix; ixh()/ixl() lisent; idem iy"),
         ("exx", "exx echange BC DE HL avec leurs jumeaux sans toucher AF"), ("ex_af", "ex_af echange AF/AF' sans toucher le reste"), ("ex_de_hl", "ex_de_hl echange DE et HL"),
         ("flag_values", "S=0x80 Z=0x40 Y=0x20 H=0x10 X=0x08 PV=0x04 N=0x02 C=0x01, tous disjoints, OR = 0xFF")])

T("E01-07", "GATE E01 - squelette valide", gate=True, kind="gate", size=0,
  context="""Fin de l'epopee E01: workspace compilable, crates squelettes, scripts de CI, types Pins et Regs.""",
  user_cmds=["cargo build --workspace", "cargo test --workspace", "bash tools/ci.sh", "bash tools/hygiene.sh", "python docs/tools/task.py status"],
  accept=[])
