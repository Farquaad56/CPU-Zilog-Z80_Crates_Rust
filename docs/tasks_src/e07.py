from _dsl import T

REFJS = "`docs/ref/z80_ref.js` (fait foi, fonction execED et blocs) et `docs/specs/20_FLAGS_AND_ALU.md`"
DIFF = "Z80_DIFF_TABLE=ed Z80_DIFF_FAMILY={f} cargo test -p puce-z80 --test diff -- e05_01_diff_run"


def ed(id_, title, fams, hooks, extra, tests, size=90):
    T(id_, title, kind="code", size=size,
      context="Semantique des familles `%s` de la table ED (prefixe ED)." % ", ".join(fams),
      embed=["21_HOOK_CONTRACTS.md#" + h for h in hooks] + ["20_FLAGS_AND_ALU.md#Operations ED"],
      reads=[REFJS, "E05-02 (helpers sem/regs.rs)"], tsv=[("specs/data/ed.tsv", ["40", "41", "44", "45", "47", "4D", "57", "67", "6F", "A0", "B0"])],
      spec=extra + "\n\nAcceptance: les vecteurs differentiels `ed` de ces familles passent tous.",
      write=["crates/puce-z80/src/sem/**"], tests=tests,
      accept=[DIFF.format(f=",".join(fams)), "cargo test -p puce-z80 %s_" % id_.lower().replace("-", "_")])


ed("E07-01", "ED: IN r,(C) / OUT (C),r", ["ed_in_c", "ed_out_c"], ["Echanges, divers"],
   "Dans `sem/ed.rs`: `sem_ed(cpu, op, hook)`. `ed_in_c` : voir contrat (flags S Z Y X P depuis la valeur, H=N=0, C inchange ; y==6 : valeur jetee) ; WZ=BC+1. `ed_out_c` : y==6 ecrit 0 (NMOS) ; WZ=BC+1. Le port est BC complet (plan N4:BC / O4:BC).",
   [("in_b_c", "ED 40"), ("in_f_c", "ED 70 : flags sans registre"), ("out_c_a", "ED 79"), ("out_c_0", "ED 71 ecrit 0"), ("wz_bc_plus1", "WZ=BC+1")])
ed("E07-02", "ED: ADC HL,rp / SBC HL,rp", ["ed_adc16", "ed_sbc16"], ["Arithmetique 16 bits"],
   "`HL = adc16/sbc16(HL, rp(p))` avec les drapeaux de `alu` ; WZ = HL_avant + 1.",
   [("adc_hl_bc", "ED 4A"), ("sbc_hl_hl", "ED 62"), ("sbc_sp", "ED 72"), ("wz", "WZ = HL+1 avant")])
ed("E07-03", "ED: LD (nn),rp / LD rp,(nn)", ["ed_ld_nn_rp", "ed_ld_rp_nn"], ["Chargements 16 bits"],
   "Comme `ld_nn_hl`/`ld_hl_nn` mais avec rp(p) (BC DE HL SP, non indexe). WZ final = nn+1.",
   [("ld_nn_bc", "ED 43"), ("ld_sp_nn", "ED 7B"), ("ld_nn_hl_ed", "ED 63"), ("wz", "WZ = nn+1")])
ed("E07-04", "ED: NEG RETN/RETI IM LD I,A LD A,I RRD RLD NOP", ["ed_neg", "ed_retn", "ed_im", "ed_ld_ir", "ed_rrd", "ed_rld", "ed_nop"], ["Echanges, divers", "Pile, appels, retours"],
   "`ed_neg` via `alu::neg`. `ed_retn` : PC=WZ ; IFF1=IFF2 (aussi pour RETI : memes effets). `ed_im` : IM = 0,0,1,2,0,0,1,2 selon y. `ed_ld_ir` : LD I,A / LD R,A / LD A,I / LD A,R (flags S Z de la valeur, H=N=0, P/V=IFF2, C inchange ; X Y de la valeur). `ed_rrd`/`ed_rld` : voir execED ; WZ = HL+1. `ed_nop` : aucun hook. Le bug NMOS 'INT juste apres LD A,I/R' est traite en E09/E10 : ne PAS le faire ici.",
   [("neg", "ED 44"), ("retn_iff", "ED 45 copie IFF2 dans IFF1"), ("im_modes", "IM 0/1/2 pour les 8 codes"), ("ld_a_i_pv", "LD A,I met P/V = IFF2"), ("rrd", "RRD"), ("rld", "RLD")], size=140)
ed("E07-05", "ED: instructions de bloc LDI LDD CPI CPD (+ R)", ["ed_block_ld", "ed_block_ldr", "ed_block_cp", "ed_block_cpr"], ["Instructions de bloc (ED A0..BB)"],
   "Voir la section 'Instructions de bloc' de 20_FLAGS_AND_ALU.md. X0 : flags LDI/LDD/CPI/CPD (X et Y depuis octet+A pour LD, A-octet-H pour CP) ; X1 : HL/DE +-1, BC-1 ; `cond` = BC != 0 (LDxR) ou BC != 0 et non Z (CPxR). Quand `cond` vrai : le plan execute `?I5` puis X1 doit faire PC = PC - 2 et WZ = PC + 1 (nouvelle valeur de PC).",
   [("ldi", "LDI"), ("ldir_repeat", "LDIR avec BC>1 : 21 T, PC revient"), ("cpi", "CPI"), ("cpir_stop_on_match", "CPIR s'arrete quand A==octet"), ("ldd", "LDD"), ("ldir_last", "LDIR avec BC==1 : 16 T")], size=150)
ed("E07-06", "ED: blocs d'E/S INI IND OUTI OUTD (+ R)", ["ed_block_in", "ed_block_inr", "ed_block_out", "ed_block_outr"], ["Instructions de bloc (ED A0..BB)"],
   "Voir 'Instructions de bloc'. INI/IND : octet lu dans tmp_l, B-1, flags selon le JS, X1 HL+-1 ; OUTI/OUTD : B-1 AVANT l'ecriture, port = BC apres decrement. Repetitions : PC-2 et WZ comme les autres blocs. Les bits X Y/H/P des repetitions (U) sont traites en E10-04.",
   [("ini", "INI"), ("outi_b_before", "OUTI decremente B avant la sortie"), ("inir_repeat", "INIR 21 T"), ("otir_end", "OTIR avec B==1 : 16 T")], size=150)
T("E07-07", "Table ED complete", kind="code", size=40,
  context="Valide la table `ed` entiere (256 x 6 = 1536 vecteurs; les 178 ED inconnus sont des NOP de 8 T).",
  spec="Lance `Z80_DIFF_TABLE=ed cargo test -p puce-z80 --test diff -- e05_01_diff_run --nocapture`; corrige tout ecart dans `sem/**`. Ajoute `e07_07_ed_all`.",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/tests/diff.rs"], tests=[("ed_all", "1536 vecteurs ED")],
  accept=["Z80_DIFF_TABLE=ed cargo test -p puce-z80 --test diff -- e05_01_diff_run", "cargo test -p puce-z80 e07_07_"])
T("E07-08", "GATE E07 - table ED validee", gate=True, kind="gate", size=0, context="Le prefixe ED est valide.",
  user_cmds=["Z80_DIFF_TABLE=ed cargo test -p puce-z80 --test diff -- e05_01_diff_run", "bash tools/ci.sh"], accept=[])
