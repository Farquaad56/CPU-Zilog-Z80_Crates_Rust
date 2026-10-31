from _dsl import T

REF = "`docs/ref/z80_ref.js` (fait foi pour la semantique) et `docs/specs/20_FLAGS_AND_ALU.md`"
CRC = "Le test calcule le CRC32 IEEE (poly reflechi 0xEDB88320) des octets produits en suivant EXACTEMENT l'ordre des boucles ci-dessous et le compare a la colonne `crc32` de `docs/specs/data/alu_crc.tsv` (ligne `name`)."

T("E04-01", "Harnais CRC des primitives ALU", kind="code", size=100,
  context="""L'ALU Rust (`alu.rs`) est une suite de fonctions PURES sur des entiers. Elle est validee de facon EXHAUSTIVE hors-ligne par des CRC32 de resultats produits par le coeur de reference (qui passe ZEXALL). Cette tache cree le harnais; les taches suivantes ajoutent une fonction ALU et son test CRC chacune.""",
  reads=["`docs/specs/data/alu_crc.tsv` (24 lignes: name, count, crc32)", "`docs/tools/gen_alu_crc.js` (le generateur: source de l'ordre des boucles, ne PAS le modifier)"],
  spec="""Cree `crates/puce-z80/tests/common/mod.rs` (declare `pub mod crc; pub mod tsv;` + re-exports) et:
- `common/crc.rs`: `pub struct Crc32(u32)` avec `new()`, `update(&mut self, b: u8)`, `finish(self) -> u32`; `pub fn crc32(data: &[u8]) -> u32`. Table calculee a la volee (const fn ou tableau local).
- `common/tsv.rs`: `pub fn read_tsv(rel: &str) -> Vec<Vec<String>>` lit `docs/specs/data/<rel>` (chemin base sur `env!("CARGO_MANIFEST_DIR")/../../docs/specs/data/`), saute la ligne d'en-tete, decoupe sur la tabulation. `pub fn alu_crc(name: &str) -> (u64, u32)` renvoie (count, crc) de la ligne `name` (panic clair si absente).
- `tests/alu_crc.rs` (`mod common;`) : les helpers ci-dessus + fonction `fn check(name: &str, bytes_crc: Crc32, count: u64)` qui compare crc ET count a la table (message: nom, attendu, obtenu).
Les futures taches ajouteront des fonctions `#[test] fn e04_NN_<name>()` dans `tests/alu_crc.rs`.""",
  write=["crates/puce-z80/tests/common/**", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("crc_known_vector", "crc32(b\"123456789\") == 0xCBF43926"), ("crc_empty", "crc32(b\"\") == 0"), ("incremental_equals_oneshot", "update octet par octet == crc32 d'un bloc"), ("tsv_has_24_rows", "read_tsv(\"alu_crc.tsv\") a 24 lignes"), ("alu_crc_lookup", "alu_crc(\"alu8_add\") == (131072, 0x09ee0e62)"), ("tsv_missing_panics", "alu_crc(\"nope\") panique (#[should_panic])")])

T("E04-02", "ALU 8 bits: ADD ADC SUB SBC CP", kind="code", size=110,
  context="""Premiere primitive ALU, validee par CRC exhaustif (3 x 65536 combinaisons par operation, avec retenue d'entree).""",
  embed=["20_FLAGS_AND_ALU.md#ALU 8 bits"], reads=[REF],
  spec="""Cree `crates/puce-z80/src/alu.rs` (`pub mod alu;`, fonctions `pub`, documentees):
`pub fn alu8(op: u8, a: u8, b: u8, f: u8) -> (u8, u8)` : `op` 0..7 = ADD ADC SUB SBC AND XOR OR CP. Renvoie `(nouvel A, nouveau F)` (pour CP, A inchange; X et Y de F viennent de l'OPERANDE b). Dans cette tache implemente 0 (ADD), 1 (ADC), 2 (SUB), 3 (SBC), 7 (CP); les autres valeurs: `unreachable!()` pour l'instant (E04-03).
`pub fn parity(v: u8) -> u8` (0x04 si nombre de bits pair, sinon 0). Utilise `crate::regs::flag`.
""" + CRC + """
Ordre (pour op = 0..): `for a in 0..=255 { for b in 0..=255 { for c in 0..=1 { f_in = c; (A', F') = alu8(op, a, b, c); push A'; push F' } } }` -> count 131072. Noms de lignes: `alu8_add`, `alu8_adc`, `alu8_sub`, `alu8_sbc`, `alu8_cp`.""",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/src/lib.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("alu8_add", "CRC alu8_add"), ("alu8_adc", "CRC alu8_adc"), ("alu8_sub", "CRC alu8_sub"), ("alu8_sbc", "CRC alu8_sbc"), ("alu8_cp", "CRC alu8_cp"), ("parity_table", "parity(0)=4, parity(1)=0, parity(3)=4, parity(0xFF)=4, parity(0x7F)=0"), ("add_example", "alu8(0, 0x7F, 1, 0) == (0x80, 0x94)")])

T("E04-03", "ALU 8 bits: AND XOR OR", kind="code", size=60,
  context="""Suite de `alu8` pour les operations logiques.""",
  embed=["20_FLAGS_AND_ALU.md#ALU 8 bits"], reads=[REF],
  spec="""Complete `alu8` pour op 4 (AND: H=1), 5 (XOR), 6 (OR): H=0 N=0 C=0, S Z X Y depuis le resultat, P/V = parite.
""" + CRC + " Memes boucles qu'en E04-02. Lignes: `alu8_and`, `alu8_xor`, `alu8_or`.",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("alu8_and", "CRC alu8_and"), ("alu8_xor", "CRC alu8_xor"), ("alu8_or", "CRC alu8_or"), ("and_h_set", "alu8(4, 0xFF, 0x0F, 0) a H=1"), ("xor_zero", "alu8(5, 0xAA, 0xAA, 0xFF) == (0, 0x44)")])

T("E04-04", "INC8 et DEC8", kind="code", size=60,
  context="""INC et DEC 8 bits conservent C. Validation CRC exhaustive.""",
  embed=["20_FLAGS_AND_ALU.md#INC et DEC 8 bits"], reads=[REF],
  spec="""Dans `alu.rs`: `pub fn inc8(v: u8, f: u8) -> (u8, u8)` et `pub fn dec8(v: u8, f: u8) -> (u8, u8)` (renvoient (resultat, F)).
""" + CRC + " Ordre: `for v in 0..=255 { for c in 0..=1 { (r, f) = inc8(v, c); push r; push f } }` (count 512). Lignes `inc8`, `dec8`.",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("inc8", "CRC inc8"), ("dec8", "CRC dec8"), ("inc_overflow", "inc8(0x7F,0) a P/V=1 et H=1"), ("dec_overflow", "dec8(0x80,0) a P/V=1"), ("carry_kept", "inc8(0xFF,1) conserve C=1")])

T("E04-05", "Rotations et decalages (RLC..SRL)", kind="code", size=80,
  context="""Les 8 operations de rotation/decalage de CB, y compris SLL (non documente).""",
  embed=["20_FLAGS_AND_ALU.md#Rotations et decalages"], reads=[REF],
  spec="""Dans `alu.rs`: `pub fn rot(y: u8, v: u8, f: u8) -> (u8, u8)` avec y = 0 RLC, 1 RRC, 2 RL, 3 RR, 4 SLA, 5 SRA, 6 SLL, 7 SRL. Renvoie (resultat, F complet: S Z X Y depuis le resultat, H=N=0, P/V parite, C = bit sorti).
""" + CRC + " Ordre: `for v in 0..=255 { for c in 0..=1 { (r,f)=rot(y,v,c); push r; push f } }` (count 512). Lignes: `rot_rlc rot_rrc rot_rl rot_rr rot_sla rot_sra rot_sll rot_srl`.",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("rot_rlc", "CRC"), ("rot_rrc", "CRC"), ("rot_rl", "CRC"), ("rot_rr", "CRC"), ("rot_sla", "CRC"), ("rot_sra", "CRC"), ("rot_sll", "CRC"), ("rot_srl", "CRC"), ("sll_sets_bit0", "rot(6, 0x00, 0) == (0x01, ..)")])

T("E04-06", "Arithmetique 16 bits (ADD HL, ADC HL, SBC HL)", kind="code", size=90,
  context="""Les 3 operations 16 bits. `add16` ne touche pas S Z P/V; `adc16`/`sbc16` calculent tous les drapeaux.""",
  embed=["20_FLAGS_AND_ALU.md#Arithmetique 16 bits"], reads=[REF],
  spec="""Dans `alu.rs`: `pub fn add16(a: u16, b: u16, f: u8) -> (u16, u8)`, `pub fn adc16(...)`, `pub fn sbc16(...)` (meme signature). X et Y viennent de l'octet HAUT du resultat. H = retenue du bit 11.
""" + CRC + """ Ordre: `let mut a = 0u32; while a < 65536 { let mut b = 0u32; while b < 65536 { for f in F_IN { (r, f2) = fn(a as u16, b as u16, f); push (r & 0xFF); push (r >> 8); push f2 } b += 263 } a += 257 }`
avec F_IN = [0x00, 0xFF] pour add16 et [0x00, 0x01] pour adc16/sbc16 (count 128000). Lignes `add16`, `adc16`, `sbc16`.""",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("add16", "CRC add16"), ("adc16", "CRC adc16"), ("sbc16", "CRC sbc16"), ("add16_keeps_szpv", "add16 conserve S Z P/V de f"), ("sbc16_zero", "sbc16(0x1234,0x1234,0) donne Z=1 N=1")])

T("E04-07", "DAA", kind="code", size=50,
  context="""Ajustement decimal, valide pour les 65536 combinaisons (A, F).""",
  embed=["20_FLAGS_AND_ALU.md#DAA"], reads=[REF],
  spec="""Dans `alu.rs`: `pub fn daa(a: u8, f: u8) -> (u8, u8)`.
""" + CRC + " Ordre: `for a in 0..=255 { for f in 0..=255 { (a2, f2) = daa(a, f); push a2; push f2 } }` (count 65536). Ligne `daa`.",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("daa", "CRC daa"), ("daa_after_add", "daa(0x0A,0) == (0x10, ..) avec H=1")])

T("E04-08", "NEG, BIT et conditions", kind="code", size=100,
  context="""Dernieres primitives pures: NEG, drapeaux de BIT, evaluation des conditions cc.""",
  embed=["20_FLAGS_AND_ALU.md#Operations CB", "20_FLAGS_AND_ALU.md#Operations ED"], reads=[REF],
  spec="""Dans `alu.rs`:
- `pub fn neg(a: u8) -> (u8, u8)`: res = 0 - a; `F = (res & 0xA8) | (res==0 ? 0x40 : 0) | ((0 ^ a ^ res) & 0x10) | (a==0x80 ? 0x04 : 0) | 0x02 | (a != 0 ? 0x01 : 0)`.
- `pub fn bit_flags(v: u8, y: u8, f: u8, xy: u8) -> u8`: `bit = v & (1<<y)`; F = (f & 0x01) | 0x10 | (xy & 0x28) | (bit==0 ? 0x44 : 0) | (y==7 && bit!=0 ? 0x80 : 0). (`xy` = octet dont les bits 3 et 5 vont dans X et Y: pour BIT sur registre c'est v lui-meme.)
- `pub fn cond(f: u8, cc: u8) -> bool`: cc 0..7 = NZ Z NC C PO PE P M.
""" + CRC + " Lignes: `neg` (for a in 0..=255 push res, F; count 256) et `bit_reg` (for v in 0..=255, for y in 0..=7, for c in 0..=1: push bit_flags(v, y, c, v); count 4096).",
  write=["crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/alu_crc.rs"],
  tests=[("neg", "CRC neg"), ("bit_reg", "CRC bit_reg"), ("cond_table", "cond pour f=0x00 et f=0xFF sur les 8 codes"), ("bit7_sets_s", "bit_flags(0x80,7,0,0x80) a S=1"), ("neg_80", "neg(0x80) == (0x80, 0x87)")])

T("E04-09", "GATE E04 - ALU validee", gate=True, kind="gate", size=0,
  context="""Toutes les primitives ALU passent leur CRC exhaustif.""",
  user_cmds=["cargo test -p puce-z80 --test alu_crc", "cargo test -p puce-z80 e04_", "bash tools/ci.sh"], accept=[])
