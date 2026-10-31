from _dsl import T

REFJS = "`docs/ref/z80_ref.js` (fait foi) et `docs/specs/20_FLAGS_AND_ALU.md`"
H21 = "21_HOOK_CONTRACTS.md"
DIFFCMD = "Z80_DIFF_TABLE=main Z80_DIFF_FAMILY={fam} cargo test -p puce-z80 --test diff -- --nocapture"
DIFFNOTE = """Acceptance: les vecteurs differentiels de ces familles (6 par opcode, `docs/specs/data/diff.tsv`) doivent TOUS passer (etat final complet, memoire ecrite, ports, T-states; WZ non compare; X/Y de SCF/CCF masques). Reproduire la commande d'acceptance en cas d'echec pour voir l'opcode fautif."""


def fam(id_, title, fams, hooks, extra="", size=100, embed=(), tsv=(), tests=(), write=None, table="main", ctx=""):
    T(id_, title, kind="code", size=size,
      context=ctx or ("Semantique (registres, drapeaux, WZ) des familles `%s` de la table %s. Le TIMING est deja fourni par les plans: tu n'ecris QUE les hooks `X0`/`X1` de ces familles." % (", ".join(fams), table)),
      embed=[H21 + "#" + h for h in hooks] + list(embed), tsv=list(tsv), reads=[REFJS, "`docs/specs/13_ENGINE.md` (etat du moteur: tmp_l, tmp_h, cond, disp, wz)", "E05-02 (helpers `sem/regs.rs`)"],
      spec=(extra + "\n\n" + DIFFNOTE).strip(),
      write=write or ["crates/puce-z80/src/sem/**"], tests=list(tests),
      accept=[DIFFCMD.format(fam=",".join(fams)).replace("main", table, 1) if table != "main" else DIFFCMD.format(fam=",".join(fams)), "cargo test -p puce-z80 %s" % (id_.lower().replace("-", "_") + "_")])


T("E05-01", "Harnais de vecteurs differentiels (diff.tsv)", kind="code", size=150,
  context="""Chaque instruction du coeur est validee contre 6 vecteurs par opcode produits par un coeur de reference qui passe ZEXALL (`docs/specs/data/diff.tsv`, 7632 cas). Cette tache cree le banc de test (tests/diff.rs) qui REGENERE l'etat initial depuis la graine, execute UNE instruction avec `Cpu::step` et compare tout.""",
  embed=["24_DIFF_VECTORS.md#"],
  reads=["`docs/specs/data/diff.tsv`", "`docs/tools/gen_diff_vectors.js` (le generateur: source de verite du PRNG; ne PAS le modifier)", "`docs/specs/data/*.tsv` colonne `family` (pour le filtre)"],
  spec="""Cree `crates/puce-z80/tests/diff.rs` (`mod common;`) et `common/diffbus.rs`:
- `Xs32`: xorshift32 EXACT de la spec (`s ^= s<<13; s ^= s>>17; s ^= s<<5`, u32 wrapping); `seed(table_id, op, vec)`; `hash(addr, seed)`; `TestBus { mem: Box<[u8;65536]>... }` (le test peut utiliser `std`): `read` renvoie `mem`, `write` enregistre `(addr,val)` ET ecrit, `io_read(port)` renvoie 0xFF et enregistre le port, `io_write` enregistre `(port,val)`.
- `fn run_vector(row) -> Result<(), String>`: reconstruit l'etat initial (ordre de tirage de la spec, section 2), regle la memoire (`hash` partout, puis octets de code), PC/SP/regs/IFF/IM, execute `cpu.step(&mut bus)` et compare: registres (A F B C D E H L, jumeaux, IX IY SP PC I R), IFF1 IFF2 IM, `halted`, les listes `writes`, `port_out`, `port_in`, et `t`. WZ non compare. Pour main 37 (SCF) et 3F (CCF): comparer `F & 0xD7`.
- Lecture de `diff.tsv`; FILTRES par variables d'environnement: `Z80_DIFF_TABLE` (main|cb|ed|ddfd|ddcb|all, defaut all), `Z80_DIFF_FAMILY` (liste de familles separees par des virgules, lues dans la colonne `family` du TSV de la table; vide = toutes), `Z80_DIFF_OP` (liste d'opcodes hex).
- Un `#[test] fn e05_01_diff_run()` qui execute les vecteurs filtres et, en cas d'echec, affiche les 20 premiers ecarts (table, opcode, vec, champ, attendu, obtenu, etat initial) puis `panic!`. 
- Sans variable ET sans semantique ecrite le test echoue: c'est normal. Les taches suivantes l'utilisent avec un filtre. Un `#[test]` auto-test `e05_01_selftest_nop` execute uniquement `main` famille `nop`.""",
  write=["crates/puce-z80/tests/diff.rs", "crates/puce-z80/tests/common/**"],
  tests=[("xorshift_vector", "xorshift32(1) donne 270369, 67634689, 2647435461, 307599695, 2398689233"), ("hash_vectors", "hash(0,1)==233 et hash(65535,7)==174"), ("seed_formula", "seed(0,0x76,0)==3642849633, seed(0,0,0)==1, seed(1,0xFF,5)==619304293"),
         ("selftest_nop", "le vecteur NOP passe (famille nop de la table main)"), ("filter_family", "Z80_DIFF_FAMILY=nop selectionne 6 vecteurs"), ("parse_rows", "diff.tsv contient 7632 lignes de donnees")],
  accept=["cargo test -p puce-z80 e05_01_", "Z80_DIFF_TABLE=main Z80_DIFF_FAMILY=nop cargo test -p puce-z80 --test diff -- e05_01_diff_run"],
  notes="La famille `nop` n'a pas de hook : elle sert de test du banc. Si le vecteur NOP echoue, le bug est dans E03 (moteur) ou dans le banc.")

T("E05-02", "Dispatch sem et helpers de registres", kind="code", size=150,
  context="""Squelette de la semantique: dispatch par table/opcode et helpers partages par toutes les familles (registres 8 bits r(i), paires rp/rp2, conditions, ecriture de F avec suivi de Q).""",
  embed=[H21 + "#", "23_UNDOCUMENTED.md#23 -"],
  spec="""Dans `sem/mod.rs` : `pub(crate) fn sem(cpu: &mut Cpu, table: Table, op: u8, hook: u8)` : `match table { Main => main::sem_main(cpu, op, hook), Cb => cb::sem_cb, Ed => ed::sem_ed, DdFd => idx::sem_idx, DdCb => idx::sem_ddcb }` ; chaque sous-module (fichiers `sem/main.rs`, `cb.rs`, `ed.rs`, `idx.rs`) contient une fonction qui ne fait RIEN pour l'instant (`let _ = ...`) sauf `ddfd` ou `ix_plain` renvoie vers `sem_main(cpu, op, hook)` (le prefixe est ignore) : pour l'instant la fonction renvoie vers `sem_main` pour TOUT opcode de DdFd (E08 affinera).
`sem/regs.rs` (helpers `pub(crate)`, semantique de l'index): 
- `r8(cpu, i) -> u8` : i 0..7 = B C D E H L (HL) A ; `(HL)` (6) lit `eng.tmp_l`. Version `r8x` : identique mais, si `eng.prefix != None`, H -> IXH/IYH et L -> IXL/IYL.
- `set_r8`, `set_r8x` symetriques ((HL) ecrit `eng.tmp_l`).
- `hl(cpu)` / `set_hl(cpu,v)` : HL, ou IX/IY selon `eng.prefix`.
- `rp(cpu, p)` / `set_rp` : 0 BC 1 DE 2 HL(index) 3 SP ; `rp2` / `set_rp2` : 0 BC 1 DE 2 HL(index) 3 AF.
- `cc(cpu, i) -> bool` via `alu::cond(regs.f, i)`.
- `set_f(cpu, f)` : ecrit `regs.f = f` et `eng.f_written = true` (nouveau champ `pub(crate) f_written: bool` de Engine, remis a false au debut de chaque instruction).
Test de duree (E03-13) : des que `sem` n'est plus vide, `tests/timing_stub.rs` n'est plus valable tel quel (les conditions deviennent vraies). Ajoute dans Cargo.toml la feature vide `stub-sem = []`, `#![cfg(feature = "stub-sem")]` en tete de `tests/timing_stub.rs`, et `if cfg!(feature = "stub-sem") { return; }` en premiere ligne de `sem()`. Le test se lance desormais avec `--features stub-sem`.
Facteur Q : a la FIN de chaque instruction (frontiere), `regs.q = if eng.f_written { regs.f } else { 0 }` (code dans le moteur, a l'endroit ou `active` passe a false).""",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/lib.rs", "crates/puce-z80/Cargo.toml", "crates/puce-z80/tests/timing_stub.rs"],
  tests=[("r8_read_write", "set_r8 puis r8 pour les 8 indices; (HL) passe par tmp_l"), ("r8x_ixh", "prefix=Dd : r8x(4)==IXH, set_r8x(5, v) modifie IXL"), ("rp_pairs", "rp/set_rp/rp2 pour p=0..3, SP et AF"), ("hl_prefix", "hl()==IX avec prefix Dd, IY avec Fd"), ("cc_conditions", "cc sur f connus"),
         ("q_tracking", "apres une instruction qui appelle set_f, regs.q==regs.f; sinon 0 (utiliser le plan [F4 X0] avec un faux hook via start_plan)"), ("dispatch_main_noop", "sem sur NOP ne modifie rien")])

fam("E05-03", "Misc: NOP DI EI HALT EX AF EXX EX DE,HL", ["nop", "di", "ei", "halt", "ex_af", "exx", "ex_de_hl"], ["Echanges, divers"], size=70,
    extra="""Hooks : `di` : IFF1=IFF2=false. `ei` : IFF1=IFF2=true et `eng.ei_delay = true` (nouveau champ Engine, utilise par E09). `halt` : `regs.halted = true; regs.pc = regs.pc - 1` (convention de la reference JS : PC reste sur HALT ; UNKNOWN - to confirm : recalage SingleStepTests en E13-09). `ex_af`, `exx`, `ex_de_hl` via les methodes de Regs.""",
    tests=[("di_ei", "DI/EI mettent IFF1/IFF2 et ei_delay"), ("halt_pc", "HALT laisse PC sur l'opcode et halted=true"), ("exx", "EXX echange les 3 paires"), ("ex_af", "EX AF,AF'")])

fam("E05-04", "LD 8 bits registre/registre/immediat/(HL)", ["ld_r_r", "ld_r_n", "ld_r_hl", "ld_hl_r", "ld_hl_n"], ["Chargements 8 bits"], size=70,
    extra="Le dispatch par (x,y,z) de `sem_main` : x==1 (sauf 0x76) -> ld_r_r/ld_r_hl/ld_hl_r selon y,z ; x==0 z==6 -> ld_r_n/ld_hl_n. Sans prefixe `r8`; l'index viendra en E08.",
    tests=[("ld_b_c", "LD B,C"), ("ld_a_hl", "LD A,(HL) via tmp_l"), ("ld_hl_r", "LD (HL),E : tmp_l = E avant l'ecriture"), ("ld_r_n", "LD H,n"), ("ld_hl_n_no_hook", "LD (HL),n n'a pas de hook")])

fam("E05-05", "LD indirects A,(BC) A,(DE) (nn) et WZ", ["ld_bc_a", "ld_de_a", "ld_a_bc", "ld_a_de", "ld_nn_a", "ld_a_nn"], ["Chargements 8 bits"], size=90,
    extra="Attention aux formules WZ de la spec (octet haut = A pour les ecritures, rp+1 pour les lectures). `ld_nn_a` / `ld_a_nn` : l'adresse nn est dans WZ avant X0.",
    tests=[("wz_ld_bc_a", "WZ = ((BC+1)&0xFF)|(A<<8)"), ("wz_ld_a_bc", "WZ = BC+1"), ("ld_a_nn", "A = tmp_l, WZ = nn+1"), ("ld_nn_a_wz", "WZ = ((nn+1)&0xFF)|(A<<8)")])

fam("E05-06", "LD 16 bits", ["ld_rp_nn", "ld_nn_hl", "ld_hl_nn", "ld_sp_hl"], ["Chargements 16 bits"], size=70,
    extra="`ld_rp_nn` : rp(p)=WZ (p=0..3, SP inclus). `ld_nn_hl` (22) : TMP=HL pour les 2 ecritures, WZ final = nn+1 ; `ld_hl_nn` (2A) : HL=TMP (tmp_h:tmp_l), WZ final nn+1. `ld_sp_hl` : SP=HL.",
    tests=[("ld_bc_nn", "LD BC,nn avec WZ"), ("ld_sp_nn", "LD SP,nn"), ("ld_nn_hl", "TMP=HL"), ("ld_hl_nn", "HL=TMP"), ("ld_sp_hl", "SP=HL")])

fam("E05-07", "INC/DEC 8 bits (registres et (HL))", ["inc_r", "dec_r", "inc_hl", "dec_hl"], ["ALU 8 bits"], size=60,
    extra="Utilise `alu::inc8` / `alu::dec8` avec `set_f`. Pour `(HL)` le resultat va dans `tmp_l` (ecrit ensuite par le plan).",
    tests=[("inc_b", "INC B"), ("dec_a_overflow", "DEC A depuis 0x80"), ("inc_hl", "INC (HL) via tmp_l"), ("carry_kept", "C conserve")])

fam("E05-08", "INC/DEC 16 bits et ADD HL,rp", ["inc_rp", "dec_rp", "add_hl_rp"], ["Arithmetique 16 bits"], size=70,
    extra="`inc_rp`/`dec_rp` : aucun drapeau. `add_hl_rp` : `HL = add16(HL, rp(p))` avec `set_f` ; WZ = HL_avant + 1.",
    tests=[("inc_bc", "INC BC sans drapeau"), ("dec_sp", "DEC SP wrap"), ("add_hl_de", "ADD HL,DE drapeaux"), ("add_hl_wz", "WZ = HL+1 avant l'addition")])

fam("E05-09", "ALU 8 bits: registres, (HL), immediat", ["alu_r", "alu_hl", "alu_n"], ["ALU 8 bits"], size=60,
    extra="`alu8(y, A, operande, F)` ; A = resultat sauf CP ; `set_f`. L'operande vient de `r8(z)`, `tmp_l` ((HL) et n).",
    tests=[("add_a_b", "ADD A,B"), ("cp_n", "CP n n'ecrit pas A"), ("sbc_a_hl", "SBC A,(HL)"), ("xor_a", "XOR A -> 0x44")])

fam("E05-10", "Accumulateur: RLCA RRCA RLA RRA DAA CPL SCF CCF", ["acc_rlca", "acc_rrca", "acc_rla", "acc_rra", "acc_daa", "acc_cpl", "acc_scf", "acc_ccf"], ["ALU 8 bits"], size=100,
    embed=["23_UNDOCUMENTED.md#23 -"],
    extra="""RLCA/RRCA/RLA/RRA : carry et X Y depuis le resultat, S Z P/V inchanges, H=N=0. DAA via `alu::daa`. CPL : A=!A, H=N=1, X Y du resultat. SCF/CCF : version FACTEUR Q de la spec 23 : X,Y = bits 3 et 5 de `(A | (F ^ Q))` (`regs.q`) ; les vecteurs diff masquent ces bits (F & 0xD7) car le JS utilise la version simple. Ecris un TEST qui verifie les DEUX formules sur des cas choisis et laisse un commentaire `UNKNOWN - to confirm` : la formule exacte (Q ou simple) sera tranchee par z80ccf/ZEXALL en E10-03.""",
    tests=[("rlca", "RLCA A=0x85 -> 0x0B C=1"), ("rra", "RRA avec C"), ("daa_hook", "DAA appelle alu::daa"), ("cpl", "CPL"), ("scf_flags", "SCF met C=1 H=0 N=0"), ("ccf_h", "CCF : H = ancien C")])

fam("E05-11", "PUSH et POP", ["push", "pop"], ["Pile, appels, retours"], size=50,
    extra="`push` : TMP.H=poids fort, TMP.L=poids faible de rp2(p) (le plan ecrit H puis L) ; `pop` : rp2(p) = (tmp_h<<8)|tmp_l (AF : A=H, F=L, via `set_f`).",
    tests=[("push_bc", "PUSH BC -> tmp"), ("pop_af", "POP AF charge F"), ("pop_hl", "POP HL")])

fam("E05-12", "JP, JP cc, JP (HL)", ["jp", "jp_cc", "jp_hl"], ["Sauts"], size=60,
    extra="`jp`/`jp_cc` : X0 : si condition (toujours pour jp) PC=WZ ; WZ=nn dans tous les cas (deja le cas). `jp_hl` : PC=HL.",
    tests=[("jp_taken", "JP nn"), ("jp_cc_not_taken", "JP NZ non pris : PC apres nn"), ("jp_hl", "JP (HL)")])

fam("E05-13", "JR, JR cc, DJNZ", ["jr", "jr_cc", "djnz"], ["Sauts"], size=70,
    extra="`jr_cc` : `cond` = condition(y-4) ; `djnz` : B-=1, `cond=B!=0`. X1 : si `cond` PC = PC + signed(D) ; WZ = PC. Verifie que les durees (7/12, 8/13, 12) sont celles du plan.",
    tests=[("jr_forward", "JR +5"), ("jr_cc_taken_12t", "JR NZ pris : 12 T"), ("jr_cc_not_taken_7t", "JR NZ non pris : 7 T"), ("djnz_loop", "DJNZ 13 T / 8 T"), ("jr_backward", "JR -2")])

fam("E05-14", "CALL, CALL cc, RET, RET cc, RST", ["call", "call_cc", "ret", "ret_cc", "rst"], ["Pile, appels, retours"], size=100,
    extra="`call*` : X0 `tmp = PC` (adresse de retour, H puis L), `call_cc` : `cond = cc(y)`; X1 : si cond PC=WZ. `rst` : X0 TMP=PC, X1 PC=y*8 et WZ=PC. `ret` : PC=WZ ; `ret_cc` : cond=cc(y), X1 PC=WZ si cond.",
    tests=[("call", "CALL nn : pile et PC"), ("call_cc_not_taken_10t", "CALL NZ non pris : 10 T"), ("ret", "RET"), ("ret_cc_11t", "RET Z pris : 11 T"), ("rst38", "RST 38H")])

fam("E05-15", "EX (SP),HL / IN A,(n) / OUT (n),A", ["ex_sp_hl", "in_a_n", "out_n_a"], ["Echanges, divers"], size=70,
    extra="Voir les contrats : `in_a_n` WZ=((A<<8)|n)+1 (A avant) ; `out_n_a` WZ=((n+1)&0xFF)|(A<<8). `ex_sp_hl` : TMP=HL puis HL=WZ.",
    tests=[("ex_sp_hl", "echange avec la pile"), ("in_a_n", "IN A,(n)"), ("out_n_a", "OUT (n),A port = (A<<8)|n"), ("wz_in", "WZ de IN")])

T("E05-16", "Table main complete: tous les vecteurs", kind="code", size=40,
  context="""Verifie la table `main` en entier (252 opcodes x 6 vecteurs) et corrige les ecarts residuels.""",
  spec="""Lance `Z80_DIFF_TABLE=main cargo test -p puce-z80 --test diff -- e05_01_diff_run --nocapture`. Corrige tout echec restant dans `sem/**` (jamais dans le test ni dans diff.tsv). Ajoute le test `e05_16_main_all` qui lance les 1512 vecteurs de la table main (appelle le meme code que le banc).""",
  write=["crates/puce-z80/src/sem/**", "crates/puce-z80/src/alu.rs", "crates/puce-z80/tests/diff.rs"],
  tests=[("main_all", "1512 vecteurs main OK")],
  accept=["Z80_DIFF_TABLE=main cargo test -p puce-z80 --test diff -- e05_01_diff_run", "cargo test -p puce-z80 e05_16_", "cargo test -p puce-z80 --features stub-sem --test timing_stub"])

T("E05-17", "GATE E05 - table principale validee", gate=True, kind="gate", size=0,
  context="""Le jeu d'instructions non prefixe est semantiquement valide.""",
  user_cmds=["Z80_DIFF_TABLE=main cargo test -p puce-z80 --test diff -- e05_01_diff_run", "cargo test -p puce-z80", "bash tools/ci.sh"], accept=[])
