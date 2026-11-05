# 35 - z80test (Patrik Rak)

Source verifiee (E13-15) : https://github.com/raxoft/z80test (MIT, version 1.2a du 2.12.2023).
- Depot clone dans `tests/roms/z80test/` (gitignore). Les `.tap` ne sont PAS dans le depot : ils sont dans la release
  `https://github.com/raxoft/z80test/releases/download/v1.2a/z80test-1.2a.zip`, extraite dans
  `tests/roms/z80test/release/z80test-1.2a/` (contient aussi les sources `src/`).
- Auteur : Patrik Rak. Compare les resultats a un vrai Spectrum 48K avec Z80 Zilog (CRC de 32 bits par test).

## 1. Fichiers

| Fichier (.tap) | Taille | Role | Bloc CODE |
|---|---|---|---|
| z80full.tap | 14 390 | tous les drapeaux et registres | 14298 o a 32768 |
| z80doc.tap | 14 390 | tous les registres, drapeaux DOCUMENTES seulement | 14298 o |
| z80flags.tap | 14 390 | tous les drapeaux, registres ignores | 14298 o |
| z80docflags.tap | 14 390 | drapeaux documentes seulement, registres ignores | 14298 o |
| z80ccf.tap | 14 875 | drapeaux apres un CCF place apres chaque instruction (SCF/CCF Zilog authentique) | 14783 o |
| z80memptr.tap | 14 390 | drapeaux apres un BIT n,(HL) apres chaque instruction (detecte MEMPTR) | 14298 o |
| z80ccfscr.tap | 127 | affiche a l'ecran le comportement de CCF (images de reference `img/*.png`) ; PAS un test pass/fail | 35 o |

Sources : `src/main.asm` (pilote), `idea.asm` (moteur de vecteurs/CRC), `tests.asm` (163 tests par variante, `testtable`),
`testmacros.asm`, `print.asm`, `crctab.asm`, `z80*.asm` (parametres `maskflags`, `onlyflags`, `postccf`, `memptr`), `loader.bas`.
Assembleur d'origine : sjasm ; bande : mktap.

## 2. Format .tap (verifie sur les 7 fichiers)

Suite de blocs `<longueur 16 bits LE> <flag> <donnees> <checksum xor>` :
1. entete programme BASIC (flag 0, type 0, nom `z80doc    `, longueur 42, autostart 10) ;
2. bloc programme BASIC (flag FF, 44 octets avec flag et checksum) : `10 CLEAR 32767:LOAD "" CODE:CLS` / `20 RANDOMIZE USR 32768` ;
3. entete CODE (flag 0, type 3, longueur 14298, adresse de chargement 32768) ;
4. bloc CODE (flag FF, 14298 octets + flag + checksum = 14300).

Le code machine est donc charge en `0x8000` et s'execute en `RANDOMIZE USR 32768` (point d'entree `0x8000`, `org 0x8000` dans `main.asm`).
La fin du code est vers 0xB7DA (0x8000 + 14298) ; les vecteurs/donnees de test sont dans cette zone ; la pile de test pointe vers `data.regs` (voir `idea.asm`).

## 3. Execution

- `main` : `di`, sauve IY et HL', appelle `printinit` (`ld a,2 / jp 0x1601` = CHAN-OPEN canal 2 = ecran), affiche l'entete,
  puis boucle sur `testtable` ; a la fin il fait `pop hl / exx / pop iy / ei / ret` : retour a l'appelant (BASIC).
- Chaque test calcule un CRC de 32 bits sur toutes les combinaisons d'operandes et le compare au CRC attendu (Zilog).
- Aucune sortie par code de retour : le resultat est du TEXTE imprime par RST 10h.
- Le code utilise `ei` / `di` autour de chaque RST 10h ; une machine sans interruption (aucun INT) suffit.
- Port 0xFE : le programme ecrit 0x07 sur le port 0xFE puis lit `in a,(0xfe)` et exige la valeur `0xBF`
  (seul le bit MIC/EAR = 0). Sinon : les tests IN sont signales `FAILED`, `IN FE:xx`, `Expected:BF`.
  Un Spectrum minimal doit donc renvoyer `0xBF` sur la lecture du port 0xFE (touches relachees, EAR a 0).

## 4. Sortie et message de reussite (texte exact)

Ecran de 32 colonnes via RST 10h (PRINT-A de la ROM : codes 13 = nouvelle ligne, 23 = TAB suivi de deux octets x et 1, 127 = (c)).
- En-tete : `Z80 <nom> test` + TAB + `(c) 2012 RAXOFT` (nom = `doc`, `CCF`, `MEMPTR` ou vide pour full/flags/docflags selon `testname`).
- Par test : `<numero> <nom du test>` puis, aligne a droite :
  - `OK` (reussite) ;
  - `FAILED` suivi, sur la ligne suivante, de `CRC:<8 chiffres hex>   Expected:<8 chiffres hex>` ;
  - `Skipped` (test non executable apres un echec precedent) ;
  - `FAILED` / `IN FE:<xx>` / `Expected:BF` si la lecture du port 0xFE est incorrecte.
- Resultat final (ligne `Result: ...`) :
  - succes total : `Result: all tests passed.`
  - echecs : `Result: <n> of <m> tests failed.` (m = nombre de tests executes, 163 par variante).
- Le critere pour nos tests automatiques : la sortie capturee contient `all tests passed.` et ne contient pas `FAILED`.

## 5. Points d'interception (machine ZX minimale, E13-16)

Deux modes possibles :

A. **Sans ROM (mode retenu par defaut)** : le programme n'appelle de la ROM que deux points ; la machine les simule :
   - `0x1601` (CHAN-OPEN) : retourner immediatement (`RET`) ;
   - `0x0010` (RST 10h, PRINT-A) : capturer le caractere dans A, puis `RET`. Il faut interpreter : `13` = fin de ligne, `23` = TAB (ignorer les DEUX octets suivants envoyes par RST 10h), `127` = (c) ; les autres = ASCII.
   Chargement : placer directement le bloc CODE a 0x8000 ; mettre SP dans la RAM haute (ex. 0xFFF0), empiler une adresse de retour sentinelle (ex. 0x0000), PC = 0x8000, IFF desactives ; la fin du test = PC egal a la sentinelle.
B. **Avec ROM 48K** (fournie par l'utilisateur : `tests/roms/zx48.rom`, non redistribuable) : boot ROM, `LOAD ""` avec chargement rapide en interceptant LD-BYTES `0x0556`
   (A = flag attendu, carry = LOAD, IX = destination, DE = longueur ; succes : carry = 1 au retour, en sautant le corps de la routine), puis `RANDOMIZE USR 32768`.
   Sortie capturee en interceptant `0x0010` (RST 10h) avant l'execution du code ROM, ou en lisant la memoire d'ecran (0x4000).
   Prerequis : la ROM, et les interruptions 50 Hz (le clavier de la ROM).

Prerequis communs : RAM 64 Ko (code en 0x8000+, pile haute), lecture du port 0xFE = 0xBF, aucune autre contention.

## 6. Duree attendue
Les tests sont exhaustifs (163 par variante, chacun parcourt des milliers a millions de combinaisons) : prevoir des minutes a dizaines de minutes par variante en release ; a mesurer en E13-16/E13-17.

## 7. Variantes SCF/CCF
`z80ccf` suppose le comportement Zilog authentique (X/Y dependent de Q : l'instruction precedente a modifie ou non les drapeaux) et echoue pour la moitie des tests sur les autres variantes (NEC, ST). Il est lie a la decision D-024/D-025 (SCF_CCF_USES_Q).
