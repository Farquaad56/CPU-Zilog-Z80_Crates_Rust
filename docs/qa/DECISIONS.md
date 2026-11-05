# DECISIONS (source de verite; jamais contredite par une tache)

| ID | Date | Decision | Origine |
|---|---|---|---|
| D-001 | 2026-10-31 | Projet = crate Rust `puce-z80`, Z80 NMOS Zilog cycle perfect, reutilisable; methode Hermes Kanban identique a Puce16 | utilisateur |
| D-002 | 2026-10-31 | Precision T-state par T-state (`tick` = 1 T-state) | utilisateur |
| D-003 | 2026-10-31 | Suites a passer: PRELIM, ZEXDOC, ZEXALL, z80test (Rak), SingleStepTests/z80, FUSE, ROMs de timing (Woodmass/Raxoft) | utilisateur |
| D-004 | 2026-10-31 | Frontend = page locale servie par Rust (HTTP+WebSocket, pas de WASM), connectee au vrai crate; titre "Zilog Z80 Emulator"; interface bilingue FR/EN | utilisateur |
| D-005 | 2026-10-31 | Reutilisable dans emulateurs console ET arcade: no_std, sans alloc, sans unsafe, sans dependance | utilisateur |
| D-006 | 2026-10-31 | CPU par defaut NMOS; variante CMOS differee | defaut Claude, a confirmer |
| D-007 | 2026-10-31 | Noms: crate `puce-z80`, profil Hermes `Z80_Crate`, board/tenant `z80` | defaut Claude, a confirmer |
| D-009 | 2026-10-31 | `puce-z80`: entiers uniquement, deterministe, `#![forbid(unsafe_code)]`, `#![no_std]` | defaut Claude |
| D-011 | 2026-10-31 | Broches en logique positive (bit a 1 = actif) | defaut Claude |
| D-014 | 2026-10-31 | Taches de 50 a 150 lignes de code | methode Puce16 |
| D-020 | 2026-10-31 | Execution strictement sequentielle, une tache a la fois | methode Puce16 |
| D-021 | 2026-10-31 | Oracle de semantique = `docs/ref/z80_ref.js` (passe ZEXDOC/ZEXALL); oracle hors-ligne pour Hermes = `docs/specs/data/{diff,alu_crc}.tsv` | Claude |
| D-012 | 2026-10-31 | Crate `puce-z80-machines` (std) : machines d'essai partagees par cli et web | Claude |
| D-022 | 2026-10-31 | Convention de PC en HALT = reference JS (PC reste sur HALT) ; a recaler sur SingleStepTests (E13) | Claude |
| D-023 | 2026-10-31 | Chaine multi-bots a partir de E03: codeur `z80_crate`, relecteur `relecteur`, validateur `validateur` (carte `<id>-V`, `tools/validate.ps1`: check, done, commit local). Le validateur est le seul bot autorise a lancer `task.py check/done` et `git commit`; jamais push ni approve (gates humaines). Voir docs/06_PIPELINE_MULTI_BOTS.md | utilisateur |
| D-024 | 2026-11-05 | E13-05: ZEXALL passe (67 OK, 5 764 169 610 instr) avec SCF_CCF_USES_Q = false ET = true: ZEXALL ne tranche pas. Valeur gardee: false (les 4 vecteurs DD/FD de diff.tsv, oracle JS, echouent avec true; voir docs/qa/inbox/E10-03-Q1.md). Decision finale reportee a z80ccf (E13-17); le commentaire UNKNOWN - to confirm de E10-03 reste | Claude, a confirmer |
| D-025 | 2026-11-05 | E13-09: SingleStepTests (silicon) et vecteurs JS de diff.tsv se contredisent sur SCF/CCF (formule Q, 4 vecteurs DD/FD) et sur X/Y des repetitions de bloc (41 vecteurs ED). En attendant la decision de l'utilisateur, le crate suit le JS (D-021) et ces 14 opcodes sont dans tests/baseline.json; voir docs/qa/inbox/E13-09-Q1.md | Claude, a confirmer |
