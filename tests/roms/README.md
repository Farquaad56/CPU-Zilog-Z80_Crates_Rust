# tests/roms - ROMs de test (jamais commitees)

Ce dossier est rempli par `python docs/tools/fetch_roms.py` (tache E13-01) ou par `z80 fetch`. Contenu attendu:

| Fichier | Origine |
|---|---|
| prelim.com, zexdoc.cim, zexall.cim | superzazu/z80 (voir docs/specs/30_TEST_SUITES.md) |
| z80test/ (sources) ; .tap: release v1.2a | raxoft/z80test (voir FAQ_Z80.md, ROMs: sources verifiees) |
| singlestep/v1/*.json | SingleStepTests/z80 (voir FAQ_Z80.md) |
| fuse/tests.in, tests.expected | FUSE z80/tests (voir FAQ_Z80.md) |
| zx48.rom | a fournir par l'utilisateur (non redistribuable ici) |

Les sommes SHA-256 attendues sont dans `tests/manifest.toml`.
