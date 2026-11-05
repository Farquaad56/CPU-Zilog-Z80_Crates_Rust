# 34 - Format des tests FUSE (z80/tests)

Source verifiee (E13-12) : https://github.com/FUSE-emulator/fuse/tree/master/z80/tests
- `tests.in` 161 511 octets, `tests.expected` 323 000 octets, `README` 2 469 octets.
- Telechargement : `https://raw.githubusercontent.com/FUSE-emulator/fuse/master/z80/tests/<fichier>`
- Copie locale (gitignoree) : `tests/roms/fuse/`.
- Contenu mesure : 1356 tests (noms uniques) dans `tests.in` ET dans `tests.expected`, meme ordre.

## 1. tests.in - un enregistrement

```
<description>                      <- ligne 1 : nom (ex. 00, 02_1, edb0_1, dd00)
AF BC DE HL AF' BC' DE' HL' IX IY SP PC MEMPTR      <- ligne 2 : 13 mots hex 4 chiffres
I R IFF1 IFF2 IM <halted> <tstates>                 <- ligne 3
<addr> <b1> <b2> ... -1            <- 0..n lignes memoire (hex), terminees par -1
-1                                 <- fin de l'enregistrement
<ligne vide>
```
- Ligne 2 : dans l'ordre AF BC DE HL AF' BC' DE' HL' IX IY SP PC MEMPTR (MEMPTR = WZ).
- Ligne 3 : I et R en hex 2 chiffres ; IFF1, IFF2, IM en decimal ; `halted` 0/1 ; `tstates` en DECIMAL (espaces de cadrage possibles, separateur = espaces multiples).
- `tstates` = budget : on execute des instructions entieres tant que le temps ecoule < tstates (la derniere peut depasser). 1333 tests sur 1356 ont `1` (= exactement une instruction) ; les autres (21, 79, 163, 37, 16, 132, 9...) sont les instructions repetees (LDIR, CPIR, INIR, OTIR...) qui boucleraient.
- Memoire : adresse hex puis octets hex ; seules les adresses listees sont initialisees (le reste = 0 dans FUSE).
- Valeurs observees : halted toujours 0 en entree ; IM 0 (1351 tests), 1 (4), 2 (1).

## 2. tests.expected - un enregistrement

```
<description>
<time> <type> <address> [<data>]     <- 0..n evenements (temps decimal cadre a droite)
AF BC DE HL AF' BC' DE' HL' IX IY SP PC MEMPTR      <- etat final (comme .in)
I R IFF1 IFF2 IM <halted> <tstates>                 <- tstates = temps FINAL (decimal)
<addr> <b1> ... -1                 <- memoire MODIFIEE par rapport a l'etat initial
<ligne vide>
```
- `time` : T-state (depuis 0 au debut du test) auquel l'evenement a lieu.
- `type` :
  | Code | Sens | data |
  |---|---|---|
  | MC | memory contend : adresse presentee pour un acces memoire (debut de cycle) | absent |
  | MR | memory read : l'octet est lu (fin de cycle) | octet lu |
  | MW | memory write : l'octet est ecrit | octet ecrit |
  | PC | port contend : adresse de port presentee | absent |
  | PR | port read | octet lu |
  | PW | port write | octet ecrit |
- `address` : adresse (memoire) ou port complet 16 bits (B ou A en octet haut), hex.
- Les MC/PC sont emis meme sans contention reelle (7844 MC, 79 PC sur le fichier) : ils marquent seulement l'instant et l'adresse du cycle ; les temps ne contiennent aucun retard de contention (machine test sans ULA).
- Comptes du fichier : MC 7844, MR 4550, MW 569, PC 79, PR 38, PW 37.
- Memoire modifiee : seulement les octets dont la valeur differe de l'etat initial (adresse hex, octets, `-1`) ; absente si rien n'a change.

## 3. Motifs de temps (exemples reels)

- Fetch d'opcode : `0 MC pc` puis `4 MR pc op` (M1 = 4 T). Prefixe : second fetch `4 MC pc+1`, `8 MR pc+1 op2`.
- Lecture memoire operande : MC a t, MR a t+3.
- Ecriture memoire : MC a t, MW a t+3 (`02: 4 MC 0001 / 7 MW 0001 56`).
- Entree/sortie : `d3_1` : `8 PW a2ed a2` ; `db_2` : `7 PC 71e2`, `8 PR 71e2 71`, `8 PC 71e2` (le port est presente puis lu a t+1 : cycle I/O de 4 T avec TW).
- Cycles internes (non M1) : `MC` repetes a la meme adresse sans MR/MW (`edb0_1` : MC c000 de 14 a 20).
- Instruction repetee en boucle : `edb0_1` se termine a 37 apres deux iterations (voir 4.3).

## 4. Regle de comparaison

1. Charger l'etat initial (registres, MEMPTR, I, R, IFF1/2, IM, halted) et la memoire.
2. Executer des instructions completes jusqu'a ce que `temps >= tstates` (budget). Pour `tstates=1` : exactement 1 instruction.
3. Comparer l'etat final (13 mots + I R IFF1 IFF2 IM halted) ; comparer `tstates` final au temps ecoule ; comparer la memoire modifiee (adresses listees = valeur attendue ; adresses non listees = valeur initiale).
4. Comparer les evenements : meme nombre, meme ordre, memes `time` / `type` / `address` / `data`. Pour notre CPU, les MR/MW/PR/PW se deduisent des broches (MREQ+RD/WR, IORQ+RD/WR) ; les MC/PC correspondent a la presentation d'adresse (premier T de chaque cycle bus) - ils ne sont comparables que comme "instants d'adresse".
5. Les cycles internes sans bus n'ont que des MC : a ignorer ou a reconstruire selon le mode de comparaison choisi par E13-13 (decision documentee la-bas).

### 4.3 Cas des boucles
LDIR/LDDR/CPIR/CPDR/INIR/INDR/OTIR/OTDR : `tstates` > 1 donne plusieurs iterations (ex. `edb0_1` : budget 4 -> 2 iterations, fin a 37 = 21 + 16). Seul le critere "terminer l'instruction en cours quand temps >= budget" est utilise.

## 5. Cas particuliers

- **HALT (`76`)** : etat final `halted=1`, PC reste a l'adresse de HALT (`0000`) dans FUSE (SingleStepTests : voir D-022 / 33_SINGLESTEP_FORMAT.md, convention differente a recalibrer).
- **MEMPTR** (README) : `02_1`, `0a_1`, `32_1`, `edb0..b3,b8..bb_1/_2`, `d3_4` verifient MEMPTR ; `cb46_1..5` verifient que les bits 3/5 de F copient ceux de MEMPTR pour BIT n,(HL).
- **SCF** (`37_1..3`) : bits 3 et 5 de F (Q / A) - voir D-024.
- **BIT** (`cb{4,5,6,7}{7,f}_1`) : redondants (X/Y copies sur tous les BIT).
- **E/S** (`d3_*`, `db_*`) : quatre cas de contention de port (octet haut 0x40-0x7f ou non, bit 0 du port 0/1).
- **dd00 / ddfd00** : timings des NOP etendus `DD 00` (8 T puis la suite) et `DD FD 00` ; l'octet 00 final verifie que l'opcode suivant s'execute au bon temps.
- Pas de test d'interruption/NMI ni de WAIT dans cette suite.

## 6. Differences avec SingleStepTests
Voir `33_SINGLESTEP_FORMAT.md` : FUSE donne des evenements de bus par instruction avec MC/PC, SST donne l'etat des broches a chaque T-state.
