# 21 - CONTRATS DES HOOKS PAR FAMILLE

Rappel (12_PLAN_GRAMMAR.md): `X0` = calcul (registres, TMP, `cond`), `X1` = post-bus. La logique exacte (drapeaux, valeurs) est dans `20_FLAGS_AND_ALU.md`
(code JS de reference); ce fichier dit SEULEMENT quoi faire a chaque hook. `r(y)`/`r(z)` = registre 8 bits d'index y/z (B C D E H L, `(HL)` = TMP, A).
`rp(p)` = BC DE HL SP; `rp2(p)` = BC DE HL AF. Opcode = `x y z` (x=op>>6, y=(op>>3)&7, z=op&7, p=y>>1, q=y&1).

## Chargements 8 bits
- `ld_r_r`: X0: r(y) = r(z). `ld_r_n`: X0: r(y) = TMP.L. `ld_r_hl`: X0: r(y) = TMP.L. `ld_hl_r`: X0: TMP.L = r(z). `ld_hl_n`: pas de hook (TMP.L deja lu).
- `ld_bc_a`/`ld_de_a`: X0: TMP.L = A; WZ = ((rp+1)&0xFF) | (A<<8). `ld_a_bc`/`ld_a_de`: X0: A = TMP.L; WZ = rp+1.
- `ld_nn_a`: X0: TMP.L = A; WZ = ((nn+1)&0xFF) | (A<<8) (apres l'ecriture WZ garde cette valeur). `ld_a_nn`: X0: A = TMP.L; WZ = nn+1.

## Chargements 16 bits
- `ld_rp_nn`: X0: rp(p) = WZ. `ld_nn_hl`: X0: TMP = HL; WZ final = nn+1. `ld_hl_nn`: X0: HL = TMP; WZ final = nn+1.
- `ld_sp_hl`: X0: SP = HL. `ed_ld_nn_rp`/`ed_ld_rp_nn`: comme ci-dessus avec rp(p).

## Pile, appels, retours
- `push`: X0: TMP = rp2(p) (H premier ecrit, L ensuite). `pop`: X0: rp2(p) = TMP (AF: A=H, F=L).
- `call`: X0: TMP = PC (adresse de retour); X1: PC = WZ. `call_cc`: X0: cond = condition(y); TMP = PC; X1: si cond PC = WZ. WZ = nn dans tous les cas.
- `rst`: X0: TMP = PC; X1: PC = y*8; WZ = PC.
- `ret`: X0: PC = WZ (WZ = adresse lue). `ret_cc`: X0: cond = condition(y); X1: si cond PC = WZ. `ed_retn`: X0: PC = WZ; IFF1 = IFF2.

## Sauts
- `jp`/`jp_cc`: X0: si condition (toujours vrai pour jp) PC = WZ; WZ = nn dans tous les cas. `jp_hl`: X0: PC = HL (HL/IX/IY selon prefixe).
- `jr`: X0: D = TMP.L; cond=vrai (jr_cc: cond=condition(y-4)); X1: si cond PC = PC + signed(D); WZ = PC. `djnz`: X0: B = B-1; cond = (B != 0); X1 idem.

## ALU 8 bits
- `alu_r`: X0: alu(y, r(z)). `alu_hl`: X0: alu(y, TMP.L). `alu_n`: X0: alu(y, TMP.L).
- `inc_r`/`dec_r`: X0: r(y) = inc8/dec8(r(y)). `inc_hl`/`dec_hl`: X0: TMP.L = inc8/dec8(TMP.L).
- `acc_*` (RLCA RRCA RLA RRA DAA CPL SCF CCF): X0 selon le code JS `default:` de execMain (cas x=0, z=7).

## Arithmetique 16 bits
- `inc_rp`/`dec_rp`: X0: rp(p) = rp(p) +/- 1 (aucun drapeau). `add_hl_rp`: X0: HL = add16(HL, rp(p)). `ed_adc16`/`ed_sbc16`: X0: HL = adc16/sbc16(HL, rp(p)).

## Echanges, divers
- `ex_af`: X0: echange AF/AF'. `exx`: X0: echange BC,DE,HL avec les jumeaux. `ex_de_hl`: X0: echange DE,HL (jamais IX/IY).
- `ex_sp_hl`: X0: TMP = HL; HL = WZ (valeur lue: Z=octet bas, W=octet haut); X1: rien. (TMP ecrit par les W: H en SP+1, L en SP.) WZ final = valeur lue.
- `di`: X0: IFF1=IFF2=0. `ei`: X0: IFF1=IFF2=1 et armer le retard d'une instruction (22_INTERRUPTS). `halt`: X0: halted = true (le PC reste sur HALT). `nop`: aucun hook.
- `in_a_n`: X0: A = TMP.L; WZ = ((A_avant<<8)|n)+1. `out_n_a`: X0: TMP.L = A; WZ = ((n+1)&0xFF) | (A<<8).
- `ed_in_c`: X0: v = TMP.L; flags S Z Y X P depuis v, H=N=0, C inchange; si y != 6 r(y) = v; WZ = BC+1. `ed_out_c`: X0: TMP.L = (y==6 ? 0 : r(y)); WZ = BC+1.
- `ed_neg`, `ed_im`, `ed_ld_ir`, `ed_rrd`, `ed_rld`, `ed_nop`: voir code JS `execED` (20_FLAGS_AND_ALU.md).

## CB
- `cb_rot`/`cb_res`/`cb_set` (registre): X0: r(z) = op(r(z)). Version `(HL)`: X0: TMP.L = op(TMP.L) puis ecriture. `cb_bit`: X0: flags BIT (X,Y depuis r(z); pour (HL) depuis WZ haut).

## Instructions de bloc (ED A0..BB)
- `ed_block_ld*`: X0: TMP.L = octet lu; flags LDI/LDD calcules avec (octet + A); X1: HL, DE +/-1, BC-1, cond = (BC != 0) pour les variantes R.
- `ed_block_cp*`: X0: flags CPI/CPD; HL +/-1; BC-1; cond = (BC != 0 et octet != A) pour R.
- `ed_block_in*`: X0: TMP.L = octet lu sur le port (BC); B = B-1 (flags selon JS); X1 (apres ecriture en (HL)): HL +/-1; cond = (B != 0) pour R.
- `ed_block_out*`: X0: TMP.L = (HL); B = B-1; X1: HL +/-1; cond = (B != 0) pour R. Quand `cond` est vrai, `?I5` s'execute et PC = PC - 2 (X1), WZ = PC + 1.

## Indexe (DD/FD)
- Meme semantique que la famille sans prefixe, avec HL -> IX/IY (`hl()`/`setHl()` du JS). `(HL)` -> `(IX+d)`: l'adresse `XY` = IX + signed(D), WZ = XY (calculee au jeton `R3:PC+>D`).
- H/L -> IXH/IXL quand l'instruction n'a PAS d'operande `(IX+d)`. Si elle en a un, l'AUTRE operande reste H ou L reel.
- `ix_ld_n` (DD 36): D lu puis n lu (R5) dans TMP.L, puis ecriture en XY.
- DDCB: X0: v = TMP.L; res = op(v); TMP.L = res; si z != 6 (non BIT) r(z) = res aussi (non documente).
