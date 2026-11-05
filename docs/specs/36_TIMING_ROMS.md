# 36 - ROMs de timing, d'interruptions et de MEMPTR (E13-18)

Recherche faite le 2026-11-05 ; chaque URL ci-dessous a ete telechargee avec succes depuis la machine de developpement
(sauf mention contraire). Copies locales dans `tests/roms/timing/` (gitignore) ; tailles et sha256 dans `tests/manifest.toml`.
Aucune ROM Spectrum n'est fournie : la ROM 48K (copyright Amstrad/Sinclair) est apportee par l'utilisateur dans `tests/roms/zx48.rom`.

## 1. Tableau

| ID | Programme | Auteur | URL verifiee | Licence | Format | Machine | Statut |
|---|---|---|---|---|---|---|---|
| W1 | Z80 Test Suite (flags + MEMPTR) | Mark Woodmass (2008) | https://raw.githubusercontent.com/MrKWatkins/EmulatorTestSuites/main/src/MrKWatkins.EmulatorTestSuites.Z80/Program/MarkWoodmass/z80tests.tap (5 573 o) | aucune licence dans la bande ("Copyright 2008 Mark Woodmass") ; le wiki redcode la donne GPLv2 | .tap : BASIC `loader` + CODE 5 452 o a 32768 | ZX 48K + ROM 48K | retenu |
| T1 | 48K timing tests v1.0 | equipe ZXSpectrum4.net | https://raw.githubusercontent.com/MrKWatkins/EmulatorTestSuites/main/src/MrKWatkins.EmulatorTestSuites.ZXSpectrum/Timing/timing_tests_48k_v1.0.z80 (10 883 o) ; page d'origine https://www.zxspectrum4.net/op_timing.php | non indiquee (programme publie pour que le public l'execute) | snapshot .z80 v2/3 | ZX 48K, contention memoire + E/S, bus flottant pour les tests 35-37 | retenu, verdict UNAVAILABLE (voir 3.2) |
| B1 | Z80 Block Flags Test v5.0-beta (2022-01-11) | Peter Helcmanovsky (Ped7g) | https://zxe.io/depot/software/ZX%20Spectrum/Z80%20Block%20Flags%20Test%20v5.0-beta%20%282022-01-11%29%28Helcmanovsky%2C%20Peter%29%5B%21%5D.tap (1 529 o) | MIT | .tap : BASIC `z80bltst` + CODE 1 435 o a 32768 | ZX 48K + ROM ; IM2 ; ports FE, 1F, FF | retenu |
| I1 | Z80 IM1 vs JR (2022-02-11) | Peter Helcmanovsky | https://zxe.io/depot/software/ZX%20Spectrum/Z80%20IM1%20vs%20JR%20%282022-02-11%29%28Helcmanovsky%2C%20Peter%29%5B%21%5D.tap (186 o) | MIT | .tap : BASIC + CODE 33 o a 32768 | ZX avec ROM (handler 0038h ; BC_SPACES a 0030h) | retenu |
| F1 | FDDD / FDDD2 | Mark Woodmass (2008) | https://zxe.io/depot/software/ZX%20Spectrum/FDDD%20%282008-03-08%29%28Woodmass%2C%20Mark%29%5B%21%5D.tap (184 o, telecharge) | non indiquee | .tap | exige le mode "allRAM" du +3 / +2A | NON RETENU (hors machine 48K) |
| C1 | z80-int.c (interruptions IM0/IM1/IM2, EI differe, HALT, instructions chevauchees) | Andre Weissflog (floooh/chips-test) | https://github.com/floooh/chips-test/blob/master/tests/z80-int.c | voir le depot | code C, pas une ROM | n'importe quel Z80 pas a pas | a reecrire en tests Rust (pas telecharge) |
| R1 | EI48K, EIHALT, HALT Contention Test, Z80 INT Skip, ED777F, Z80 DDFD3, Z80 Illegal Instruction Test, TSTR... | divers (wiki redcode/Z80) | https://github.com/redcode/Z80/wiki/Z80-Test-Suite | wiki FDL ; chaque test a sa licence | .tap/.asm sur zxe.io | Spectrum | NON TELECHARGES (hors perimetre E13, pistes futures) |

Jeux d'instructions deja couverts ailleurs : z80test (35_Z80TEST.md), FUSE (34), SingleStepTests (33), ZEXDOC/ZEXALL (30).

## 2. Comment chaque programme rapporte son resultat

### W1 - Woodmass
Menu a l'ecran : `1: Run Z80 Flags test`, `2: Run Z80 MEMPTR test`, `3: Exit to BASIC`. Pendant l'execution `      Testing Z80 flags...` ou `      Testing Z80 MEMPTR...`, puis une ligne par groupe (`ADD16`, `ADC16`, `SBC16`, `RLA/RRA`, `RLCA/RRCA`, `RLC/RRC`, `RL/RR`, `SLA/SRA`, `SLL/SRL`, `RLD/RRD`, `LD A,I/R`, `BIT n,(HL)`, `BIT n,(IX+d)`, ...) suivie de `passed` ou `failed - expected <valeur>`. Fin : `All tests finished - press a key`. Les resultats de reference sont ceux d'un NEC D780C-1.
Critere : toutes les lignes de groupe finissent par `passed`, aucune ligne `failed`, pour les choix 1 et 2. Il faut injecter les touches `1` puis `2` (clavier lu par la ROM).

### T1 - Timing 48K
Chaque test (72 cas : 34 groupes en memoire non contendue/contendue + 4 variantes ROM/BASIC) execute un lot d'instructions en boucle jusqu'a l'interruption suivante et affiche (flux en direct sur l'ecran) la valeur du registre R, le compteur de boucle et l'adresse atteinte (SP). Le programme detecte aussi si la machine est de type "early" ou "late" (1 T-state de decalage) via une boucle `JP (HL)`. Les tests 22, 32, 33, 34 exigent la contention meme en "non contendu" ; les tests 35-37 exigent un bus flottant de type 48K. Les ecrans de reference 35-37 sont dans le depot MrKWatkins (`ZXSpectrum/Timing/Screens/*.scr`).

### B1 - Block Flags
Affiche un tableau `Instr @80xx @88xx @A0xx @A8xx` : pour LDDR/LDIR, CPIR/CPDR, INIR/INDR, OTIR/OTDR interrompus en pleine repetition (interruption IM2), compare F a la regle silicium (X = bit 11 de PC, Y = bit 13 de PC ; pour IN/OT aussi PF et HF modifies). Messages d'erreur du programme : `unexpected IN 1F,FE,FF` (lecture des ports 1Fh/FEh/FFh en interruption : FEh bit 7 = 1, 1Fh bit 7 = 0, FFh = FFh), `failed delay calibration`, `is frame > 73500T?`. Un journal binaire de HF vs B est ecrit a 8900h.
Critere : aucune ligne ne signale d'ecart (le tableau reproduit les valeurs attendues du wiki) ; sur silicium Zilog la sortie de reference est la capture du wiki redcode "Z80 Block Flags Test".

### I1 - IM1 vs JR
Attend une interruption avec `halt`, execute des `jr $+2` pour en recevoir une seconde pendant un `jr`. Retourne au BASIC avec HL = 0 si le handler 0038h a ete execute ; HL != 0 si c'est 0030h (BC_SPACES) qui l'a ete. Le BASIC imprime `IM1 operating normally` ou `"IM1 landed to 0x30"`.
Critere : HL = 0 au retour (verifiable sans ROM : handler 0038h = `EI / RET`, adresse 0030h = piege).

## 3. Decisions pour E13-19

1. Mode sans ROM (`ZxMachine::new_romless`) : applicable a **I1** (handler 0038h et piege en 0030h), avec critere HL = 0.
2. Mode avec ROM 48K fournie par l'utilisateur (`tests/roms/zx48.rom`, absente de ce depot) : **W1**, **B1**, **T1** exigent la ROM (menu et clavier, PRINT, interruptions du moniteur ROM). Les tests correspondants sont `#[ignore]` et SKIP si la ROM est absente.
3. **T1** : le verdict automatique est `UNAVAILABLE` : les valeurs attendues (R, boucle, SP) ne sont pas publiees sous forme exploitable ; elles sont dans le harnais C# de MrKWatkins (GPL-3.0) et ne sont pas copiees ici. Le test T1 peut seulement verifier que le snapshot s'execute sans plantage. Contention a implementer : table 48K standard (6,5,4,3,2,1,0,0 sur 8 T-states, premiere ligne a la T-state 14335, 192 lignes de 224 T-states) -- deja dans `ZxMachine::with_contention` ; contention des ports (E/S) et bus flottant restent a ecrire.
4. **C1** n'est pas une ROM : ses scenarios (IM0/1/2, `EI` differe, interruption qui libere HALT, instruction recouvrante) sont a couvrir par des tests Rust pas a pas dans `puce-z80/tests/` (E13-19).
5. **F1** n'est pas retenu (mode allRAM du +3 hors perimetre).

## 4. Fichiers locaux

| Fichier | Taille | sha256 |
|---|---|---|
| tests/roms/timing/woodmass_z80tests.tap | 5 573 | 42ce7fc8393de83876b89015be54f7d0da3c8c5b07a075981c12dd9d84c8b95d |
| tests/roms/timing/timing_tests_48k_v1.0.z80 | 10 883 | 1e66230a7b23737294f35d2778b8384ce3f81412b98883d35e564091377382af |
| tests/roms/timing/z80_block_flags_v5.0beta.tap | 1 529 | 3b5fc9a527e7b97a00e36820dba2812e55d83a773427f23dd53ecabcf34c439e |
| tests/roms/timing/z80_im1_vs_jr.tap | 186 | 01ff836ea68e273b332cd7d9c2bd18dde3f9d4fcc62f369443517131e8e47a18 |

Le script `docs/tools/fetch_roms.py --write-manifest` ne connait que les ROMs CP/M : ces quatre entrees de `tests/manifest.toml` sont tenues a la main (le script les ignorerait/effacerait a la regeneration).
