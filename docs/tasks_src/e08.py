from _dsl import T

REFJS = "`docs/ref/z80_ref.js` (fait foi: cherche `this.px`/`hl()`/`setHl()`) et `docs/specs/20_FLAGS_AND_ALU.md`"
IDX = "21_HOOK_CONTRACTS.md#Indexe (DD/FD)"


def ix(id_, title, fams, extra, tests, size=90, table="ddfd", rows=("21", "34", "36", "46", "70", "86", "E1", "E3")):
    tab = "ddfd" if table == "ddfd" else "ddcb"
    T(id_, title, kind="code", size=size,
      context="Semantique des familles `%s` des tables DD/FD (IX/IY). Les plans (timings) existent deja: tu n'ecris que les hooks. Les vecteurs de test alternent DD (IX) et FD (IY) selon la parite du numero de vecteur." % ", ".join(fams),
      embed=[IDX, "23_UNDOCUMENTED.md#23 -"], reads=[REFJS, "E05-02 (helpers `hl`, `set_hl`, `rp`, `r8x` dans sem/regs.rs)", "`docs/specs/13_ENGINE.md` section 5 (adresse Xy: IX/IY+d, calculee avant X0, `eng.xy_addr`)"],
      tsv=[("specs/data/%s.tsv" % tab, list(rows))],
      spec=extra + "\n\nAcceptance: les vecteurs differentiels `%s` de ces familles (6 par opcode: 3 DD + 3 FD) passent tous." % tab,
      write=["crates/puce-z80/src/sem/**"], tests=tests,
      accept=["Z80_DIFF_TABLE=%s Z80_DIFF_FAMILY=%s cargo test -p puce-z80 --test diff -- e05_01_diff_run" % (tab, ",".join(fams)), "cargo test -p puce-z80 %s_" % id_.lower().replace("-", "_")])


ix("E08-01", "DD/FD: dispatch et prefixe ignore", ["ix_plain"],
   """Dans `sem/idx.rs`, `sem_idx(cpu, op, hook)` : classification de `op` en "indexe" ou "ignore". Sont INDEXES (traites en E08-02..06) : x=0 avec z=1/2/3/4/5/6 (y/p) touchant HL ou H/L/(HL) : 09 19 29 39 21 22 2A 23 2B 24 25 2C 2D 26 2E 34 35 36 ; x=1 hors 76 si y==4,5,6 ou z==4,5,6 (operande H/L/(HL)) ; x=2 si z==4,5,6 ; E1 E3 E5 E9 F9. TOUT LE RESTE est 'ignore' : `sem_main(cpu, op, hook)` avec `eng.prefix` positionne mais sans effet sur ces opcodes (plan = F4 + plan Main).
Cree `fn is_indexed(op: u8) -> bool` (testee) . Pour l'instant les opcodes indexes ne font rien.
Les vecteurs de la famille `ix_plain` (166 opcodes) doivent passer.""",
   [("is_indexed_list", "is_indexed vrai pour 21 36 46 70 86 E9 F9, faux pour 00 40 76 C3 CD"), ("ignored_nop", "DD 00 = 8 T, memes effets que NOP"), ("ignored_ld_bc_nn", "DD 01 nn charge BC"), ("ignored_jp", "DD C3 saute"), ("fd_same", "FD aussi")], size=80)
ix("E08-02", "IX/IY: chargements 16 bits, pile, sauts", ["ix_ld_rp_nn", "ix_ld_nn_hl", "ix_ld_hl_nn", "ix_ld_sp_hl", "ix_jp_hl", "ix_inc_rp", "ix_dec_rp", "ix_push", "ix_pop", "ix_ex_sp_hl", "ix_ex_de_hl"],
   "Comme les familles Main `ld_rp_nn`(p=2), `ld_nn_hl`, `ld_hl_nn`, `ld_sp_hl`, `jp_hl`, `inc_rp`/`dec_rp`(p=2), `push`/`pop`(p=2), `ex_sp_hl` avec `hl()`/`set_hl()` (IX ou IY selon `eng.prefix`). `ex_de_hl` : DD EB echange DE et HL (jamais IX/IY).",
   [("ld_ix_nn", "DD 21"), ("ld_nn_ix", "DD 22"), ("ld_ix_nn_mem", "DD 2A"), ("ld_sp_ix", "DD F9"), ("jp_ix", "DD E9"), ("inc_ix", "DD 23"), ("push_ix", "DD E5"), ("pop_iy", "FD E1"), ("ex_sp_ix", "DD E3"), ("ex_de_hl_ignored", "DD EB")], size=120, rows=("21", "22", "2A", "E1", "E3", "E5", "E9", "F9", "EB"))
ix("E08-03", "IX/IY: ADD IX,rp", ["ix_add_hl_rp"],
   "`ADD IX,rp` : `p=2` signifie IX/IY lui-meme (ADD IX,IX). `HL`->`hl()` : resultat dans IX/IY ; drapeaux de `add16` ; WZ = IX_avant + 1.",
   [("add_ix_bc", "DD 09"), ("add_ix_ix", "DD 29"), ("add_iy_sp", "FD 39"), ("wz", "WZ = IX+1")], size=60, rows=("09", "19", "29", "39"))
ix("E08-04", "IX/IY: LD r,(IX+d) / LD (IX+d),r / LD (IX+d),n", ["ix_ld_r", "ix_ld_m", "ix_ld_n"],
   "Acces memoire indexes : `ix_ld_r` : r(y) = tmp_l avec le registre COMPLET H/L (pas IXH/IXL) ; `ix_ld_m` : tmp_l = r(z) (H/L reels) ; `ix_ld_n` : pas de hook utile (tmp_l = n deja lu). WZ = IX+d (deja positionne par la resolution d'adresse Xy).",
   [("ld_b_ix_d", "DD 46"), ("ld_h_ix_d_real_h", "DD 66 charge H (pas IXH)"), ("ld_ix_d_h", "DD 74 ecrit H"), ("ld_ix_d_n", "DD 36 d n"), ("negative_d", "d = -1")], size=100, rows=("46", "66", "70", "74", "36"))
ix("E08-05", "IX/IY: ALU, INC et DEC sur (IX+d)", ["ix_alu", "ix_inc", "ix_dec"],
   "`ix_alu` : `alu8(y, A, tmp_l, F)` ; `ix_inc`/`ix_dec` : `tmp_l = inc8/dec8(tmp_l)` avec `set_f`. Le plan ecrit ensuite le resultat (inc/dec).",
   [("add_a_ix_d", "DD 86"), ("cp_ix_d", "DD BE"), ("inc_ix_d", "DD 34"), ("dec_iy_d", "FD 35")], size=60, rows=("34", "35", "86", "BE"))
ix("E08-06", "IXH/IXL: registres non documentes", ["ix_ld_r_r", "ix_ld_r_n", "ix_inc_r", "ix_dec_r", "ix_alu_r"],
   "Quand l'instruction n'a PAS d'operande `(IX+d)`, H et L designent IXH/IXL (IYH/IYL). Utilise `r8x`/`set_r8x`. Exemples : DD 44 LD B,IXH ; DD 65 LD IXH,IXL ; DD 26 n LD IXH,n ; DD 24 INC IXH ; DD 84 ADD A,IXH.",
   [("ld_b_ixh", "DD 44"), ("ld_ixh_ixl", "DD 65"), ("ld_ixl_n", "DD 2E n"), ("inc_ixh", "DD 24"), ("dec_ixl", "DD 2D"), ("add_a_ixh", "DD 84"), ("ld_a_iyl_fd", "FD 7D")], size=100, rows=("44", "65", "26", "24", "2D", "84"))
ix("E08-07", "DDCB/FDCB: BIT RES SET et rotations sur (IX+d)", ["ddcb_bit", "ddcb_rot", "ddcb_res", "ddcb_set"],
   "Dans `sem_ddcb(cpu, op, hook)` (op = octet d'operation lu comme donnee) : decode x,y,z comme CB. `v = tmp_l` (octet lu en (IX+d)). x=0 : `rot(y, v)` ; x=1 BIT : `bit_flags(v, y, F, (xy_addr >> 8))` (X Y de l'octet HAUT de l'adresse IX+d) ; x=2/3 RES/SET. Resultat dans tmp_l (ecrit par le plan sauf BIT). Les variantes z != 6 (non documentees) font l'objet de E08-08.",
   [("rlc_ix_d", "DD CB d 06"), ("bit_ix_d", "DD CB d 46 : X Y depuis l'adresse haute"), ("res_ix_d", "DD CB d 86"), ("set_iy_d", "FD CB d C6"), ("sll_ix_d", "DD CB d 36")], size=100, table="ddcb", rows=("06", "36", "46", "86", "C6"))
ix("E08-08", "DDCB/FDCB non documentes: resultat copie dans un registre", ["ddcb_rot_undoc", "ddcb_res_undoc", "ddcb_set_undoc"],
   "Quand z != 6 (x != 1) le resultat est AUSSI copie dans le registre r(z) (B C D E H L A reels). Exemple DD CB d 00 = RLC (IX+d),B. Pour BIT (x==1), z est ignore. Verifie sur les 8 variantes de z.",
   [("rlc_b_copy", "DD CB d 00 copie dans B"), ("set_a_copy", "DD CB d C7 copie dans A"), ("res_l_copy", "FD CB d 85 copie dans L"), ("memory_also_written", "la memoire reste ecrite")], size=60, table="ddcb", rows=("00", "05", "85", "C7"))
T("E08-09", "Tables DD/FD et DDCB completes", kind="code", size=40,
  context="Valide `ddfd` (1512 vecteurs) et `ddcb` (1536 vecteurs).",
  spec="Lance `Z80_DIFF_TABLE=ddfd` puis `Z80_DIFF_TABLE=ddcb` (commande `cargo test -p puce-z80 --test diff -- e05_01_diff_run --nocapture`); corrige les ecarts dans `sem/**`. Ajoute `e08_09_ddfd_all` et `e08_09_ddcb_all`.",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/tests/diff.rs"], tests=[("ddfd_all", "1512 vecteurs"), ("ddcb_all", "1536 vecteurs")],
  accept=["Z80_DIFF_TABLE=ddfd cargo test -p puce-z80 --test diff -- e05_01_diff_run", "Z80_DIFF_TABLE=ddcb cargo test -p puce-z80 --test diff -- e05_01_diff_run", "cargo test -p puce-z80 e08_09_"])
T("E08-10", "Tous les vecteurs differentiels", kind="code", size=30,
  context="Les 7632 vecteurs, toutes tables, doivent passer ensemble (non-regression).",
  spec="Ajoute `e08_10_all_vectors` : execute `Z80_DIFF_TABLE=all`. Ne corrige que dans `sem/**`.",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/tests/diff.rs"], tests=[("all_vectors", "7632 vecteurs OK")],
  accept=["cargo test -p puce-z80 --test diff -- e05_01_diff_run", "cargo test -p puce-z80 e08_10_"])
T("E08-11", "GATE E08 - jeu d'instructions complet", gate=True, kind="gate", size=0,
  context="Toutes les instructions (documentees et non documentees) correspondent a la reference JS sur 7632 vecteurs; le timing est conforme aux TSV.",
  user_cmds=["cargo test -p puce-z80 --test diff", "cargo test -p puce-z80 --features stub-sem --test timing_stub", "bash tools/ci.sh"], accept=[])
