from _dsl import T

UND = "23_UNDOCUMENTED.md#23 -"

T("E10-01", "Audit MEMPTR (WZ) de toutes les familles", kind="code", size=140,
  context="""WZ (MEMPTR) n'est pas compare par les vecteurs differentiels (le JS n'est pas une reference fiable pour WZ). Cette tache ecrit des tests de REGLES CONNUES (documentation 'MEMPTR' de Boo-boo / Sean Young) et corrige `sem/**` si une regle n'est pas respectee. Les ROMs z80memptr et SingleStepTests (E13) font foi : toute regle marquee (U) sera recalee la-bas.""",
  embed=[UND],
  spec="""Cree `crates/puce-z80/tests/memptr.rs` (une fonction de test par regle, execute UNE instruction avec `SimpleRam`, lit `cpu.regs.wz`). Regles a tester :
- `LD A,(nn)` : WZ = nn+1. `LD (nn),A` : WZ_bas = (nn+1)&0xFF, WZ_haut = A. `LD A,(BC|DE)` : WZ = rp+1. `LD (BC|DE),A` : WZ_bas = (rp+1)&0xFF, WZ_haut = A.
- `LD (nn),rp` et `LD rp,(nn)` (HL, ED xx, IX/IY) : WZ = nn+1. `EX (SP),HL/IX/IY` : WZ = nouvelle valeur du registre.
- `ADD HL,rp`, `ADC`, `SBC` (et IX/IY) : WZ = valeur 16 bits de destination AVANT l'operation + 1. `RLD`/`RRD` : WZ = HL+1.
- `JP nn`, `JP cc,nn` (pris ou non), `CALL nn`, `CALL cc,nn` (pris ou non) : WZ = nn. `JP (HL)` : WZ inchange. `RST n` : WZ = adresse du vecteur. `RET`, `RETI`, `RETN` : WZ = adresse de retour ; `RET cc` : WZ = adresse seulement si pris.
- `JR`, `DJNZ` : WZ = cible si pris ; inchange sinon. 
- `IN A,(n)` : WZ = ((A_avant << 8) + n) + 1. `OUT (n),A` : WZ_bas = (n+1)&0xFF, WZ_haut = A. `IN r,(C)`, `OUT (C),r` : WZ = BC+1.
- `LDI/LDD` : inchange. `CPI` : WZ+1 ; `CPD` : WZ-1. `INI` : WZ = BC_avant+1 ; `IND` : BC_avant-1 ; `OUTI` : BC_apres+1 ; `OUTD` : BC_apres-1. Variantes repetees quand elles reboucle : WZ = PC_de_l_instruction + 1.
- `(IX+d)` (toute instruction indexee) : WZ = IX+d.
Pour chaque regle, initialise WZ a une valeur sentinelle (0xA5A5) quand "inchange" est attendu.""",
  write=["crates/puce-z80/tests/memptr.rs", "crates/puce-z80/src/sem/**"],
  tests=[("ld_a_nn", "regle 1"), ("ld_nn_a", "regle 2"), ("ld_a_bc", "regle 3"), ("ld_nn_hl", "regle 4"), ("ex_sp_hl", "regle"), ("add_hl", "regle"), ("rld", "regle"), ("jp_cc_not_taken", "WZ=nn meme non pris"), ("call_cc_not_taken", "idem"),
         ("ret_cc_not_taken_unchanged", "RET cc non pris: inchange"), ("jr_taken", "WZ=cible"), ("jr_not_taken", "inchange"), ("in_a_n", "regle"), ("out_n_a", "regle"), ("in_r_c", "regle"), ("cpi_cpd", "+1 / -1"), ("ini_ind", "regle"), ("outi_outd", "regle"), ("ldir_repeat", "PC+1"), ("ix_d", "WZ=IX+d")])

T("E10-02", "Drapeaux X/Y des blocs repetes et DDCB", kind="code", size=100,
  context="""Quand une instruction de bloc rebouclee (BC != 0) s'execute, les bits 3 et 5 de F proviennent de l'octet HAUT de PC (le PC 'recule' de 2), pas du resultat de l'iteration (comportement observe sur NMOS par z80test/Rak).""",
  embed=[UND],
  spec="""Dans `sem/block.rs` (ou se trouve la logique de bloc) : quand `cond` est vrai (le plan va executer `?I5` et X1 fait PC=PC-2) : `F = (F & !(X|Y)) | ((PC_apres_recul >> 8) & (X|Y))` pour LDIR/LDDR/CPIR/CPDR. Pour INIR/INDR/OTIR/OTDR repetes, des regles supplementaires de P/V et H existent (UNKNOWN - to confirm : formules exactes recalees en E13-17 sur z80test `z80flags`/SingleStepTests ; ici implemente SEULEMENT X/Y comme ci-dessus pour INxR/OTxR et laisse un commentaire `UNKNOWN - to confirm`).
Le dernier tour (BC==0) ne change PAS le comportement vs les vecteurs differentiels existants : TOUS les vecteurs de E07 doivent continuer a passer (les vecteurs de `diff.tsv` utilisent des BC aleatoires, donc presque toujours en repetition : si un vecteur casse, c'est que la formule X/Y est celle du JS pour cette famille ; dans ce cas, conserve la formule du JS pour que `diff` passe et note le conflit dans une question Q&R `E10-02-Q1` - la ROM z80test tranchera).""",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/tests/block_flags.rs"],
  tests=[("ldir_repeat_xy", "LDIR rebouclant: X/Y = PC haut"), ("ldir_last_iteration", "LDIR dernier tour: X/Y selon l'octet"), ("cpir_repeat_xy", "CPIR rebouclant"), ("diff_still_passes", "les vecteurs ed passent encore (appel du banc, table ed)")])

T("E10-03", "SCF/CCF: facteur Q", kind="code", size=70,
  context="""Le registre `q` (E05-02) vaut F si l'instruction precedente a modifie F, sinon 0. SCF/CCF utilisent `(A | (F ^ Q))` pour X/Y sur NMOS Zilog. ZEXALL (E13) et z80ccf (E13) trancheront entre la formule simple et la formule Q.""",
  embed=[UND],
  spec="""Ajoute la constante `pub(crate) const SCF_CCF_USES_Q: bool = true;` dans `sem/misc.rs` (ou `sem/main.rs`) et fais dependre SCF/CCF de cette constante : true -> `xy = (A | (F ^ Q)) & (X|Y)`, false -> `xy = A & (X|Y)`. Ecris des tests directs : (1) apres `CCF` suivi de `SCF`, apres `LD A,n` (F inchange donc Q=0) puis `SCF`, apres `ADD A,B` puis `SCF`. Verifie que `regs.q` suit bien : `ADD` -> Q=F ; `LD` -> Q=0 ; `INC rp` -> Q=0 ; `PUSH` -> Q=0 ; `POP AF` -> Q=F ; `LD A,I` -> Q=F.
La decision finale (true/false) est prise en E13-05/E13-17 : un `UNKNOWN - to confirm` reste dans le commentaire.""",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/tests/q_factor.rs"],
  tests=[("q_after_add", "Q=F apres ADD"), ("q_zero_after_ld", "Q=0 apres LD A,n"), ("scf_q_xy", "SCF apres ADD: X/Y de (A|(F^Q))"), ("scf_after_ld", "SCF apres LD: Q=0"), ("pop_af_q", "POP AF: Q=F"), ("const_switch", "SCF_CCF_USES_Q existe")])

T("E10-04", "NMOS: LD A,I / LD A,R et interruption", kind="code", size=60,
  context="""Bug NMOS documente : si une INT est acceptee immediatement apres `LD A,I` ou `LD A,R`, P/V est mis a 0 au lieu de IFF2 (D-006).""",
  embed=["22_INTERRUPTS.md#4. EI"],
  spec="""Ajoute `eng.last_ld_air: bool` (vrai si la derniere instruction terminee est `ED 57` ou `ED 5F`, remis a false a la fin de toute autre instruction). Dans `start_int()` (E09-05) : si `last_ld_air` alors `regs.f &= !PV` avant d'empiler. La NMI n'est pas concernee. Le test doit utiliser un bus qui declenche `irq()` juste apres l'instruction.""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/sem/**", "crates/puce-z80/tests/ld_air_bug.rs"],
  tests=[("int_after_ld_a_i", "P/V=0 apres l'acceptation (IFF2 = 1)"), ("int_after_ld_a_r", "idem"), ("int_after_nop_keeps", "pas de modification apres un NOP"), ("nmi_not_affected", "NMI apres LD A,I: P/V inchange")])

T("E10-05", "Conformite des comportements non documentes (spec 23)", kind="code", size=140,
  context="""Tests directs des 12 points de la spec 23 (certains sont deja couverts par les vecteurs ; ceci fixe les regles par ecrit).""",
  embed=[UND],
  spec="""Cree `crates/puce-z80/tests/undocumented.rs` avec un test par point : (1) X/Y de CP depuis l'operande ; (2) IXH/IXL/IYH/IYL accessibles (LD B,IXH) ; (3) SLL ; (4) DDCB avec copie registre ; (5) `OUT (C),0` ecrit 0 ; (6) `IN (C)` positionne F sans registre ; (7) ED inconnu = NOP de 8 T ; (8) prefixe DD/FD ignore devant une instruction sans HL (4 T de plus) ; (9) `DD DD DD 21 nn` : 4 T par prefixe en trop ; (10) R : bit 7 conserve, `LD R,A` ecrase tout ; (11) apres RESET AF=SP=0xFFFF ; (12) BIT n,(HL) prend X/Y de WZ haut.
Corrige `sem/**` ou le moteur pour tout ecart.""",
  write=["crates/puce-z80/tests/undocumented.rs", "crates/puce-z80/src/sem/**", "crates/puce-z80/src/engine/**"],
  tests=[("cp_xy_operand", "(1)"), ("ixh_ixl", "(2)"), ("sll", "(3)"), ("ddcb_copy", "(4)"), ("out_c_zero", "(5)"), ("in_c_flags_only", "(6)"), ("ed_unknown_nop", "(7)"), ("dd_ignored_4t", "(8)"), ("prefix_chain", "(9)"), ("r_bit7", "(10)"), ("reset_state", "(11)"), ("bit_hl_wz", "(12)")])

T("E10-06", "GATE E10 - comportements non documentes", gate=True, kind="gate", size=0,
  context="Les comportements non documentes connus sont implementes; les `UNKNOWN - to confirm` restants sont listes pour E13.",
  user_cmds=["cargo test -p puce-z80", "grep -rn \"UNKNOWN - to confirm\" crates/puce-z80/src | wc -l", "bash tools/ci.sh"], accept=[])
