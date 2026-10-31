from _dsl import T

REFJS = "`docs/ref/z80_ref.js` (fait foi) et `docs/specs/20_FLAGS_AND_ALU.md`"
DIFF = "Z80_DIFF_TABLE=cb Z80_DIFF_FAMILY={f} cargo test -p puce-z80 --test diff -- e05_01_diff_run"


def cb(id_, title, fams, extra, tests, size=80):
    T(id_, title, kind="code", size=size,
      context="Semantique des instructions `%s` de la table CB (prefixe CB). Le timing est dans les plans (8 T registre, 12/15 T avec (HL))." % ", ".join(fams),
      embed=["21_HOOK_CONTRACTS.md#CB", "20_FLAGS_AND_ALU.md#Operations CB"], reads=[REFJS, "E05-02 (helpers sem/regs.rs)"],
      tsv=[("specs/data/cb.tsv", ["00", "06", "40", "46", "80", "86", "C0", "C6"])],
      spec=extra + "\n\nAcceptance: les vecteurs differentiels `cb` de ces familles (6 par opcode) passent tous.",
      write=["crates/puce-z80/src/sem/**"], tests=tests,
      accept=[DIFF.format(f=",".join(fams)), "cargo test -p puce-z80 %s_" % id_.lower().replace("-", "_")])


cb("E06-01", "CB: rotations et decalages", ["cb_rot"],
   "Dans `sem/cb.rs` : `sem_cb(cpu, op, hook)` decode `x=op>>6, y=(op>>3)&7, z=op&7`. x==0 : `(res,f) = alu::rot(y, r8(z), F)` ; `r8(z)` ou tmp_l pour z=6 (le plan ecrit ensuite tmp_l en memoire) ; `set_f`. Les registres sont CB-plains : `r8`, pas `r8x`.",
   [("rlc_b", "CB 00"), ("sll_a", "CB 37 SLL"), ("srl_hl", "CB 3E sur (HL) via tmp_l"), ("rot_flags", "parite et X Y du resultat")])
cb("E06-02", "CB: BIT", ["cb_bit"],
   "x==1 : `f = alu::bit_flags(v, y, F, xy)` ; xy = v pour un registre ; pour `BIT n,(HL)` (z==6) xy = octet haut de WZ (`regs.wz >> 8`). Aucun registre n'est modifie.",
   [("bit_reg", "BIT 3,B"), ("bit_hl_xy_from_wz", "BIT 0,(HL) : X Y viennent de WZ haut"), ("bit7_sign", "BIT 7,A a S=1"), ("bit_no_write", "BIT (HL) n'ecrit pas (plan sans W)")])
cb("E06-03", "CB: RES et SET", ["cb_res", "cb_set"],
   "x==2 : RES (v & !(1<<y)) ; x==3 : SET (v | (1<<y)). Aucun drapeau modifie. Pour (HL) le resultat va dans tmp_l.",
   [("res_b", "RES 3,B"), ("set_a", "SET 7,A"), ("res_hl", "RES 0,(HL) via tmp_l"), ("flags_kept", "F inchange")], size=50)

T("E06-04", "Table CB complete", kind="code", size=40,
  context="Valide la table `cb` en entier (256 opcodes x 6 = 1536 vecteurs).",
  spec="Lance `Z80_DIFF_TABLE=cb cargo test -p puce-z80 --test diff -- e05_01_diff_run --nocapture`; corrige tout ecart dans `sem/**`. Ajoute `e06_04_cb_all`.",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/tests/diff.rs"],
  tests=[("cb_all", "1536 vecteurs CB")], accept=["Z80_DIFF_TABLE=cb cargo test -p puce-z80 --test diff -- e05_01_diff_run", "cargo test -p puce-z80 e06_04_"])
T("E06-05", "GATE E06 - table CB validee", gate=True, kind="gate", size=0, context="Le prefixe CB est valide.",
  user_cmds=["Z80_DIFF_TABLE=cb cargo test -p puce-z80 --test diff -- e05_01_diff_run", "bash tools/ci.sh"], accept=[])
