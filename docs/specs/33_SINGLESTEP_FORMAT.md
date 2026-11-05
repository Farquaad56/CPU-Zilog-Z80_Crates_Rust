# 33 - FORMAT SINGLESTEPTESTS/Z80

Source: https://github.com/SingleStepTests/z80 (clone `--depth 1`, commit ebe1875d48f374bcfd4b505d8eb8ee751568b5f7, 2026-04-11).
Releve le 2026-11-05 (E13-06) en lisant le README et les fichiers reels; scan complet des 1 604 fichiers.
Ces tests ont ete produits a partir du coeur Z80 d'Ares (corrige); ils sont "probablement pas parfaits" selon l'auteur.

## Structure des dossiers

```
singlestep/            (clone dans tests/roms/singlestep/, jamais commite)
  README.MD  LICENSE  .gitignore  .github/  generation/
  v1/                  1 604 fichiers .json
```

## Noms de fichiers

Un fichier par instruction (par opcode et prefixe), 1 000 cas par fichier, 1 604 000 cas au total.

| Famille | Nom de fichier | Exemple |
|---|---|---|
| table principale | `<op>.json` (hex minuscule, 2 chiffres) | `00.json`, `3a.json`, `ff.json` |
| CB | `cb <op>.json` | `cb 34.json` |
| ED | `ed <op>.json` | `ed 44.json`, `ed a0.json` |
| DD / FD | `dd <op>.json`, `fd <op>.json` | `dd 21.json` |
| DD CB / FD CB | `dd cb __ <op>.json`, `fd cb __ <op>.json` (`__` = deplacement d) | `dd cb __ 06.json` |

Le champ `name` des cas s'ecrit en hexa majuscule: `"00 0000"`, `"3A 0000"`, `"CB 34 0000"`, `"DD CB __ 06 0000"`, `"FD FF 0000"`
(l'opcode puis un numero de cas sur 4 chiffres, `0000` pour le premier).

## Structure d'un cas (JSON)

Quatre champs communs, un cinquieme optionnel:

| Champ | Contenu |
|---|---|
| `name` | texte lisible (voir plus haut) |
| `initial` | etat avant l'instruction |
| `final` | etat apres l'instruction |
| `cycles` | liste de broches, une entree par T-state |
| `ports` | present dans 30 fichiers (IN/OUT et variantes): une transaction d'espace E/S |

### `initial` et `final` (memes cles)

Registres 8 bits (entiers): `a b c d e f h l i r`.
Registres 16 bits (entiers): `pc sp ix iy wz`, et les registres alternes `af_ bc_ de_ hl_` (paire AF', BC', DE', HL').
Autres: `iff1 iff2` (0/1), `im` (0/1/2), `ei` (0/1), `p` (0/1), `q` (octet).
`ram`: liste de paires `[adresse, valeur]` (au plus 6 entrees observees); seules les cases touchees apparaissent.

Signification:
- `wz`: registre interne MEMPTR (voir spec 23).
- `q`: dernier F ecrit par l'instruction precedente (0 si elle n'a pas modifie F); sert a SCF/CCF.
- `p`: 1 si l'instruction precedente etait `LD A,I` ou `LD A,R`.
- `ei`: 1 si l'instruction precedente etait `EI` (acceptation d'INT retardee d'une instruction). Dans `final`, `ei` vaut 0 sauf si l'instruction testee est `EI`.
- `r`: incremente de 1 par M1 (donc +2 pour un prefixe CB/DD/ED/FD + opcode, +2 aussi pour DD CB d op).
- Les registres 8 bits `a b c d e f h l` sont dans le JSON; `af`, `bc`, `de`, `hl` n'y existent pas (seuls les alternes sont en 16 bits).

### `cycles`: forme exacte d'un element

```json
[ 49774, null, "r-m-" ]      // [adresse, donnee ou null, broches]
```

- Element 0: adresse (entier 0..65535, jamais `null` dans les donnees telles que livrees: option "last address" activee).
- Element 1: donnee 0..255 ou `null` (bus electriquement deconnecte ou donnee non significative).
- Element 2: chaine de 4 caracteres, positions fixes `r w m i`: `r` = RD, `w` = WR, `m` = MREQ, `i` = IORQ; `-` si inactif.
  Chaines observees sur tout le jeu (22 millions de lignes): `----`, `r-m-`, `-wm-`, `r--i`, `-w-i`. Aucune ligne n'a M1, RFSH, HALT, WAIT, ni BUSAK.
- Les lignes sont echantillonnees ENTRE les T-states (apres chaque tick). Une ligne par T-state; le nombre de lignes est la duree de l'instruction en T-states
  (observe de 4 a 23 lignes par cas).

Options de generation par defaut (README):
- "Simplified memory access T-states" ACTIVE: MREQ/RD et MREQ/WR ne durent qu'UN T-state (reel: deux).
- Adresse de rafraichissement placee sur le bus pendant le M1 (adresse = `I << 8 | R`).
- "null pendant les attentes" DESACTIVE: pendant les cycles internes l'adresse reste la derniere adresse (pas de `null` en adresse).
- Variante NMOS (CMOS decochee).
- Aucun etat WAIT n'existe dans les cas: pas d'attente inseree.

### Cycle de recherche (M1, 4 T), exemple NOP (`00.json`)

```
T1 [19935, null, "----"]    adresse = PC
T2 [19935, null, "r-m-"]    MREQ + RD
T3 [42512, 0,    "----"]    adresse = I<<8|R (rafraichissement); donnee = octet d'opcode lu
T4 [42512, null, "----"]
```

### Lecture memoire (3 T)

```
[addr, null, "----"]   [addr, null, "r-m-"]   [addr, valeur, "----"]
```
La valeur lue apparait sur la troisieme ligne.

### Ecriture memoire (3 T)

```
[addr, null, "----"]   [addr, valeur, "-wm-"]   [addr, null, "----"]
```
La valeur ecrite apparait sur la ligne `-wm-` elle-meme (ex. DD CB 06: `[25296, 162, "-wm-"]`).

### Entree/sortie (4 T)

```
IN : [port, null, "----"]  [port, null, "----"]  [port, null, "r--i"]  [port, valeur, "----"]
OUT: [port, null, "----"]  [port, null, "----"]  [port, valeur, "-w-i"] [port, null, "----"]
```
`port` est l'adresse 16 bits complete (poids fort = registre selon l'instruction). L'entree `ports` vaut `[port, valeur, "r"|"w"]`
(30 fichiers); les lignes `r--i` et `-w-i` n'apparaissent que la, 15 000 fois chacune.
Un seul T-state porte IORQ (pas de T-state d'attente automatique).

## Cas particuliers

- HALT (`76.json`): 4 lignes (un M1); `final.pc` = PC initial + 1 (PC avance PAST le HALT). Notre convention D-022 laisse PC sur HALT: a recaler en E13-10.
  Pas de fichier pour les cycles "HALT en cours" (NOP repetes).
- Interruptions (INT/NMI) et RESET: aucun fichier ni aucun champ. Le jeu ne couvre que l'execution d'instructions.
- Les repetitions (LDIR, LDDR, CPIR, CPDR, INIR, INDR, OTIR, OTDR) sont decrites comme UNE iteration: 21 T et PC inchange (l'instruction se re-execute),
  ou 16 T et PC + 2 quand la boucle se termine (ex. `ed b1.json`: 997 cas a 21 T, 3 cas a 16 T; `ed b0.json`: 1000 cas a 21 T).
- Nombre de fichiers: table principale 252 (les 4 octets de prefixe CB DD ED FD n'ont pas de fichier), `cb` 256, `ed` 80, `dd` 252, `fd` 252
  (hors DD CB/DD DD/DD ED/DD FD), `dd cb __` 256, `fd cb __` 256: total 1 604.
- DD/FD suivis d'un opcode non indexe (ex. `dd 00.json`, `dd fb.json`): 4 T de prefixe + l'instruction (8 T pour `dd 00`); PC avance de 2.
- EI (`fb.json`): `final.ei` vaut 1 (et vaut 0 pour toutes les autres instructions testees); `dd fb.json` aussi.
- DD CB / FD CB: 23 T (4+4 pour les deux M1, 3 pour `d`, 5 pour l'octet d'operation + 2 T internes, 3+? pour (IX+d), 3 pour l'ecriture): voir l'exemple ci-dessous;
  `d` et l'octet d'operation sont lus comme des lectures memoire normales (pas de M1) et `r` avance de 2.

Repartition des durees (nombre de cas par nombre de T-states): 4:135000, 5:4043, 6:9000, 7:43024, 8:512004, 9:12070, 10:40984, 11:65951,
12:26987, 13:2996, 14:51976, 15:75930, 16:16019, 17:11005, 18:2000, 19:47000, 20:140000, 21:18011, 23:390000.

## Convention de broches observee

Comparaison entre le JSON et `fmt_trace` (tests/common/trace.rs, E03-14) pour le meme octet en memoire (numeros T1..Tn):

### NOP (4 T)

| T | SingleStepTests | puce-z80 (fmt_trace) |
|---|---|---|
| 1 | `[PC, null, ----]` | `A=PC M1` |
| 2 | `[PC, null, r-m-]` | `A=PC M1 MREQ RD` |
| 3 | `[I<<8|R, opcode, ----]` | `A=I<<8|R MREQ RFSH` |
| 4 | `[I<<8|R, null, ----]` | `A=I<<8|R MREQ RFSH` |

### LD A,(nn) (13 T)

Les 13 lignes se correspondent une a une (M1 de 4 T; lecture de n bas, n haut puis (nn) en 3 T chacune).
SingleStepTests: `[nn, null, ----] [nn, null, r-m-] [nn, valeur, ----]` pour le dernier cycle.

### DD CB d op (RLC (IX+d)), 23 T

Les 23 lignes se correspondent: lignes 1-8 (deux M1), 9-11 (lecture de d), 12-16 (lecture de op sans M1 puis 2 T internes),
17-21 (lecture (IX+d) puis 2 T internes), 22-23 (ecriture `-wm-` puis un T final).

### Differences a gerer (comparaison de broches de E13-10)

1. RFSH et M1: le JSON n'a pas de M1/RFSH; il faut les ignorer (ne comparer que R, W, MREQ, IORQ).
2. MREQ au rafraichissement (T3 et T4 du M1): notre sortie met `MREQ RFSH`, le JSON met `----`. Ne comparer MREQ qu'avec RD ou WR (ou ignorer MREQ sans RD/WR).
3. Donnee: le JSON donne la donnee sur la ligne SUIVANT le `r-m-` (lecture) ou sur la ligne `-wm-` (ecriture) et `null` ailleurs;
   notre bus de sortie porte la donnee ecrite sur toutes les lignes d'un cycle d'ecriture et l'entree lue est fournie par l'hote.
   Comparer la donnee seulement aux lignes ou le JSON la fournit (non `null`).
4. IORQ: le JSON ne porte `i` que sur 1 T-state (ligne 3 des 4); notre sortie met IORQ+RD/WR au T3 (index 2): meme position (a verifier en E13-10).
5. HALT: voir ci-dessus (PC final).
6. `r` final: +1 par M1 (7 bits bas); le bit 7 est conserve.
