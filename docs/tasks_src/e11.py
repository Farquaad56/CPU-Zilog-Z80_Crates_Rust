from _dsl import T

T("E11-01", "Desassembleur: structure et formatage des operandes", kind="code", size=120,
  context="""Le frontend affiche les mnemoniques FOURNIS PAR LE CRATE (feature `disasm`, aucune logique JS). Les modeles de mnemoniques sont generes (`tables/mnem.rs`: `MNEM_MAIN`, `MNEM_CB`, `MNEM_ED`, `MNEM_DDFD`, `MNEM_DDCB`).""",
  reads=["`crates/puce-z80/src/tables/mnem.rs` (genere)", "`docs/specs/data/*.tsv` colonne `mnemonic` (placeholders: nn n d e)"],
  spec="""Dans `disasm.rs` (`#[cfg(feature = "disasm")] pub mod disasm;`, sans alloc) :
```rust
pub struct DisText { buf: [u8; 40], len: u8 }       // texte ASCII ; impl core::fmt::Display ; fn as_str(&self) -> &str
pub struct Insn { pub len: u8, pub text: DisText, pub prefix: u8 /* 0, 0xCB, 0xDD, 0xED, 0xFD, ou 0xDD/0xFD+CB */, pub op: u8 }
pub fn disassemble(addr: u16, read: &mut dyn FnMut(u16) -> u8) -> Insn
```
Formatage : nombres hexadecimaux `$1F` (8 bits) / `$1F2E` (16 bits) en majuscules ; `nn` -> 16 bits ; `n` -> 8 bits ; `d` -> deplacement SIGNE : `(IX+$05)` / `(IX-$02)` ; `e` -> adresse absolue de la cible `addr + 2 + signed(e)` en `$XXXX`. Cette tache fournit UNIQUEMENT : `DisText` (ecriture octet par octet, troncature sans panique), les formateurs `put_hex8/put_hex16/put_disp`, et `disassemble` qui ne gere que le cas sans prefixe, sans operande (NOP, HALT, ...) en copiant le modele ; les operandes arrivent en E11-02.""",
  write=["crates/puce-z80/src/disasm.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("distext_display", "DisText ecrit 'LD A,B'"), ("hex8", "put_hex8(0x0F) -> $0F"), ("hex16", "put_hex16(0xBEEF) -> $BEEF"), ("disp_pos_neg", "(IX+$05) et (IX-$02)"), ("truncation", "un texte de 100 caracteres ne panique pas"), ("nop", "disassemble(0, NOP) -> 'NOP' len 1")])
T("E11-02", "Desassembleur: table principale et CB", kind="code", size=120,
  context="Operandes des tables Main et CB.",
  spec="""Complete `disassemble` pour `MNEM_MAIN` et `MNEM_CB` : remplace `nn`, `n`, `e` dans le modele (attention `nn` avant `n`), calcule `len` (1 + nombre d'octets d'operande : nn=2, n=1, e=1), prefixe CB -> len 2. Les modeles contiennent aussi des noms de registres contenant 'n' (ex. 'INC', 'NZ') : remplace uniquement les jetons ISOLES `nn`, `n`, `e` delimites par `,` `(` `)` espace ou fin de chaine.""",
  write=["crates/puce-z80/src/disasm.rs"],
  tests=[("ld_bc_nn", "01 34 12 -> 'LD BC,$1234' len 3"), ("jr_target", "18 FE a 0x8000 -> 'JR $8000'"), ("jr_forward", "20 05 a 0x8000 -> 'JR NZ,$8007'"), ("ld_a_n", "3E 7F -> 'LD A,$7F'"), ("cb_rlc", "CB 00 -> 'RLC B' len 2"), ("cb_bit_hl", "CB 46 -> 'BIT 0,(HL)'"), ("djnz", "10 FC"), ("jp_nn", "C3 00 80 -> 'JP $8000'")])
T("E11-03", "Desassembleur: ED", kind="code", size=90,
  context="Table ED (instructions documentees, `NOP*` pour les ED inconnus).",
  spec="Complete `disassemble` pour le prefixe ED (`MNEM_ED`): operandes `nn` (LD (nn),rp / LD rp,(nn)), len 2 ou 4. Les ED inconnus s'affichent `NOP*` (len 2).",
  write=["crates/puce-z80/src/disasm.rs"],
  tests=[("ld_nn_bc", "ED 43 34 12 -> 'LD ($1234),BC' len 4"), ("ldir", "ED B0 -> 'LDIR'"), ("neg", "ED 44 -> 'NEG'"), ("im2", "ED 5E -> 'IM 2'"), ("unknown", "ED 00 -> 'NOP*' len 2"), ("reti", "ED 4D -> 'RETI'")])
T("E11-04", "Desassembleur: DD/FD et DDCB", kind="code", size=110,
  context="Instructions indexees : `(IX+d)`, IXH/IXL et 'prefix ignored'.",
  spec="""`DD`/`FD` : modele `MNEM_DDFD` avec `IX` (DD) ou `IY` (FD) (le modele contient 'IX' ; remplace par 'IY' pour FD, y compris IXH/IXL -> IYH/IYL). `d` signe. Les modeles '(prefix ignored) X' s'affichent `DD` suivi de la suite : pour une ligne 'prefix ignored', le desassembleur renvoie `DD` seul (len 1) - c'est le comportement reel (le prefixe est une instruction de 4 T). DDCB : format `MNEM_DDCB` avec `d` apres DD CB, len 4.""",
  write=["crates/puce-z80/src/disasm.rs"],
  tests=[("ld_ix_nn", "DD 21 34 12 -> 'LD IX,$1234'"), ("ld_b_ix_d", "DD 46 05 -> 'LD B,(IX+$05)'"), ("ld_iy_neg", "FD 7E FE -> 'LD A,(IY-$02)'"), ("ld_ixh", "DD 44 -> 'LD B,IXH'"), ("rlc_ix", "DD CB 05 06 -> 'RLC (IX+$05)' len 4"), ("prefix_alone", "DD 00 -> 'DD' len 1"), ("set_iy_undoc", "FD CB 01 C0 -> 'SET 0,(IY+$01),B'")])
T("E11-05", "Desassembleur: longueur coherente avec les plans", kind="code", size=70,
  context="Garde-fou : la longueur desassemblee d'une instruction doit etre egale au nombre d'octets que le moteur lit reellement.",
  spec="""Test d'integration `tests/disasm_len.rs` (feature disasm) : pour chaque opcode de chaque table (via `__internal::plan_of`), calcule la longueur 'moteur' = nombre de jetons `PcInc` + nombre de jetons F (compte 1 octet chacun) pour les plans non vides ; verifie `disassemble(...).len`. Exceptions explicites : lignes `prefix ignored` (len 1) et `DDCB` (4). Teste aussi que le texte n'est jamais vide et ne contient que de l'ASCII imprimable.""",
  write=["crates/puce-z80/tests/disasm_len.rs", "crates/puce-z80/src/disasm.rs"],
  tests=[("main_len", "252 opcodes main"), ("cb_ed_len", "512 opcodes"), ("ddfd_len", "DD et FD"), ("ddcb_len", "DDCB et FDCB"), ("text_ascii", "ASCII imprimable non vide")],
  accept=["cargo test -p puce-z80 --features disasm e11_05_"])
