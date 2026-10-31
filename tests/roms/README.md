# tests/roms - ROMs de test (jamais commitees)

Ce dossier est rempli par `python docs/tools/fetch_roms.py` (tache E13-01) ou par `z80 fetch`. Contenu attendu:

| Fichier | Origine |
|---|---|
| prelim.com, zexdoc.cim, zexall.cim | superzazu/z80 (voir docs/specs/30_TEST_SUITES.md) |
| z80test-*/  | raxoft/z80test (UNKNOWN - to confirm) |
| singlestep/z80/v1/*.json | SingleStepTests/z80 (UNKNOWN - to confirm) |
| fuse/tests.in, tests.expected | FUSE (UNKNOWN - to confirm) |
| zx48.rom | a fournir par l'utilisateur (non redistribuable ici) |

Les sommes SHA-256 attendues sont dans `tests/manifest.toml`.
