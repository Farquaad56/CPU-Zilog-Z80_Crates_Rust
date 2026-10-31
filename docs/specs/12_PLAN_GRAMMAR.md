# 12 - PLAN GRAMMAR (micro-programme d'une instruction)

Chaque instruction est decrite par un **plan**: suite de jetons separes par des espaces (colonne `plan` de `docs/specs/data/*.tsv`).
Le moteur (`engine`) execute le plan T-state par T-state. Hermes n'ecrit PAS les plans: ils sont generes dans
`crates/puce-z80/src/tables/*.rs` par `python docs/tools/gen_rust_tables.py` (tache E03-01).

## 1. Jetons

| Jeton | Sens | T-states |
|---|---|---|
| `F4` `F5` `F6` | cycle M1 (lecture d'opcode ou de prefixe + refresh). `F5`/`F6` = M1 etendu (T supplementaires internes) | 4/5/6 |
| `R3:<adr>><dst>` `R4` `R5` | lecture memoire a `<adr>` vers `<dst>`; 3 T normal, 4/5 = T internes en plus | 3/4/5 |
| `W3:<adr><src>` `W5` | ecriture memoire de `<src>` a `<adr>` | 3/5 |
| `N4:<adr>><dst>` | lecture de port (IN) | 4 |
| `O4:<adr><src>` | ecriture de port (OUT) | 4 |
| `I<n>` | cycle interne de `n` T-states (aucune activite de bus) | n |
| `X0` `X1` | **hook**: appelle `sem(op_table, opcode, hook)`; dure 0 T | 0 |
| prefixe `?` | le jeton n'est execute que si le drapeau interne `cond` est vrai (positionne par X0) | 0 si saute |

## 2. Adresses `<adr>` (ce qui est sur le bus d'adresses pendant le cycle)

`PC+` (adresse PC puis PC=PC+1) | `SP` | `SP+` (adresse SP puis SP=SP+1) | `SP+1` | `-SP` (SP=SP-1 puis adresse SP) |
`HL` `BC` `DE` | `NN` (= WZ) | `NN+1` (= WZ+1) | `XY` (IX/IY + d, calcule, memorise dans WZ) | `AN` (port: A sur A15..A8, TMP.L sur A7..A0) | `BC` (port: BC complet).

## 3. Registres temporaires (internes au moteur)

- `TMP` = (`H`,`L`): donnee lue ou a ecrire. `<dst>` = `L` ou `H`; `<src>` = `L` ou `H`.
- `WZ` = (`W`,`Z`) = registre MEMPTR (adresse operande 16 bits; visible dans les tests SingleStepTests `wz`). `<dst>` = `Z` ou `W`.
- `D` = deplacement signe (octet lu pour `(IX+d)`), `OP` = octet d'opcode lu comme donnee (DD CB d op).

## 4. Regles d'execution

1. Un hook s'execute **a la fin du dernier T-state du cycle precedent** (donc avant le premier T du cycle suivant).
2. `X0` = "calcul": lit registres/TMP/WZ, ecrit les registres resultat, met dans `TMP` la valeur que les jetons `W` suivants ecriront, positionne `cond`.
3. `X1` = "post-bus": mises a jour qui doivent suivre les ecritures (PC pour CALL/JR/DJNZ, pointeurs des instructions de bloc).
4. Une ecriture/lecture ne voit jamais le resultat d'un hook place APRES elle.
5. Premier jeton d'un plan = toujours `F*`. Pour les tables a prefixe (CB, ED, DD, FD, DDCB) le plan contient AUSSI le(s) cycle(s) M1 du prefixe.
6. Fin d'instruction = apres le dernier jeton execute. C'est la que sont evalues INT/NMI (voir `22_INTERRUPTS.md`).
7. Exemple: `LD (nn),HL` = `F4 R3:PC+>Z R3:PC+>W X0 W3:NN<L W3:NN+1<H` : fetch, lit nn dans WZ, X0 met TMP=HL, ecrit L a nn, H a nn+1 (16 T).
8. Exemple conditionnel: `DJNZ e` = `F5 R3:PC+>L X0 ?I5 X1`: X0 fait B=B-1 et `cond = (B != 0)`; si vrai, 5 T internes; X1 fait PC=PC+d.

## 5. Tables

`main.tsv` (sans prefixe), `cb.tsv` (CB xx), `ed.tsv` (ED xx), `ddfd.tsv` (DD xx; FD identique avec IY), `ddcb.tsv` (DD CB d xx; FD CB identique avec IY).
Colonnes: `opcode, mnemonic, family, plan, t_min, t_max`. `t_min`/`t_max` = total de T-states sans/avec les jetons `?`.
Les valeurs ont ete verifiees contre un coeur de reference qui passe ZEXDOC et ZEXALL (`docs/tools/verify_timing.js`, 0 ecart sur 1272 lignes).
