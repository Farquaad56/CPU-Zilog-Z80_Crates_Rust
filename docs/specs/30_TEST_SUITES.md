# 30 - SUITES DE TEST (le CPU doit TOUTES les passer)

Aucune ROM n'est commitee (`.gitignore`). Hermes les telecharge (tache E13-01) dans `tests/roms/`. Toute URL ou format marque `UNKNOWN - to confirm`
doit etre verifie par Hermes (`curl` / `git clone --depth 1`) et note dans `docs/qa/FAQ_Z80.md`; si inaccessible: question Q&R.

| Suite | Source | Format | Contrat de reussite |
|---|---|---|---|
| PRELIM | https://raw.githubusercontent.com/superzazu/z80/master/roms/prelim.com (200, 1280 octets) | CP/M .com en 0100h | affiche `Preliminary tests complete` |
| ZEXDOC | https://raw.githubusercontent.com/superzazu/z80/master/roms/zexdoc.cim (200, 8588 octets) | CP/M .cim en 0100h | 67 lignes `OK`, aucune `ERROR`, puis `Tests complete` |
| ZEXALL | https://raw.githubusercontent.com/superzazu/z80/master/roms/zexall.cim (200, 8588 octets) | idem | idem (drapeaux X/Y inclus) |
| z80test (Patrik Rak) | https://github.com/raxoft/z80test (UNKNOWN - to confirm: release, formats .tap/.z80/.com) | ZX Spectrum (.tap) probable | z80doc, z80docflags, z80flags, z80full, z80ccf, z80memptr: toutes `Passed` |
| SingleStepTests/z80 | https://github.com/SingleStepTests/z80 (UNKNOWN - to confirm: dossier `v1/`, fichiers `<opcode>.json`) | JSON: `name, initial, final, cycles` | 100 % des cas (etat final + cycles de bus) |
| FUSE | UNKNOWN - to confirm: `z80/tests/tests.in` et `tests.expected` du depot fuse-emulator (SourceForge/GitHub) | texte | etat final + evenements de bus MR/MW/PR/PW/MC/PC horodates |
| Timing / interruptions (Woodmass, Raxoft) | UNKNOWN - to confirm | ROMs ZX Spectrum | liste exacte a etablir en E13-18 |

Notes:
- ZEXDOC/ZEXALL tournent ~5,8 milliards d'instructions (46,7 milliards de T-states): le moteur T par T doit tenir >= 15 MHz equivalent en `--release` pour finir en < 1 h (objectif de perf E13-20). Le coeur de reference JS les passe en ~90 s.
- z80test et les ROMs de timing exigent une machine ZX Spectrum minimale (ROM 48K a fournir par l'utilisateur dans `tests/roms/zx48.rom`, jamais commitee): E13-15/16.
- SingleStepTests: chaque test = etat initial (registres, ram, `wz`, `iff1/2`, `im`, `ei`, `p`, `q`) -> etat final + liste de `cycles` T par T. Format exact des `cycles` = UNKNOWN - to confirm (E13-06 le documente dans 33_SINGLESTEP_FORMAT.md).
- Les echecs sont toujours rapportes avec registres complets, T-state et broches (jamais un `TIMEOUT` nu).
