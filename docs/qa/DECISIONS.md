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
| D-022 | 2026-10-31 | Convention de PC en HALT = reference JS (PC reste sur HALT) ; SUPERSEDE par D-027 (PC apres HALT, comme SingleStepTests) | Claude |
| D-023 | 2026-10-31 | Chaine multi-bots a partir de E03: codeur `z80_crate`, relecteur `relecteur`, validateur `validateur` (carte `<id>-V`, `tools/validate.ps1`: check, done, commit local). Le validateur est le seul bot autorise a lancer `task.py check/done` et `git commit`; jamais push ni approve (gates humaines). Voir docs/06_PIPELINE_MULTI_BOTS.md | utilisateur |
| D-024 | 2026-11-05 | E13-05: ZEXALL passe (67 OK, 5 764 169 610 instr) avec SCF_CCF_USES_Q = false ET = true: ZEXALL ne tranche pas. Valeur gardee: false (les 4 vecteurs DD/FD de diff.tsv, oracle JS, echouent avec true; voir docs/qa/inbox/E10-03-Q1.md). Decision finale reportee a z80ccf (E13-17); le commentaire UNKNOWN - to confirm de E10-03 reste | Claude, a confirmer ; SUPERSEDE par D-026 (E13-17) |
| D-025 | 2026-11-05 | E13-09: SingleStepTests (silicon) et vecteurs JS de diff.tsv se contredisent sur SCF/CCF (formule Q, 4 vecteurs DD/FD) et sur X/Y des repetitions de bloc (41 vecteurs ED). En attendant la decision de l'utilisateur, le crate suit le JS (D-021) et ces 14 opcodes sont dans tests/baseline.json; voir docs/qa/inbox/E13-09-Q1.md | Claude, a confirmer ; SUPERSEDE par D-026 (E13-17) |
| D-026 | 2026-11-05 | E13-17: le crate suit le SILICIUM Zilog (z80test z80full/z80flags/z80ccf/z80doc/z80docflags/z80memptr: 6/6 passent; SingleStepTests SCF/CCF 6000/6000 et blocs repetes 8000/8000). (1) `SCF_CCF_USES_Q = true`: X/Y = A \| (F ^ Q), Q = 0 apres un prefixe DD/FD. (2) Bloc repete (LDIR/LDDR/CPIR/CPDR/INIR/INDR/OTIR/OTDR) quand la boucle repart: X/Y = octet haut du PC rembobine ; pour INxR/OTxR H et P/V sont en plus recalcules (donnee bit 7, C, nouveau B). Les 49 lignes de `diff.tsv` en desaccord (4 DD/FD + 45 ED; F seulement) sont corrigees: l'oracle JS n'etait pas fidele au silicium sur ces points. D-021 reste vrai pour le reste. Reste a decider: HALT (D-022), 3 stems dans tests/baseline.json | utilisateur, confirme le 2026-11-05 |
| D-027 | 2026-11-05 | Decisions de l'utilisateur au gate E13: (1) D-026 CONFIRMEE (silicium Zilog fait foi). (2) HALT: suivre SingleStepTests (PC avance au-dela du HALT); D-022 est supersedee et IMPLEMENTEE (HALT ne rembobine plus le PC; adresse de retour d'interruption = PC; 12 vecteurs HALT de diff.tsv recales; tests/baseline.json vide; SingleStepTests 1 604 000/1 604 000). Consequence: FUSE test `76` (PC reste sur HALT) rejoint les exceptions FUSE (6 au total). (3) Les 5 exceptions FUSE (edb9_2, edb2_1, edb3_1, edba_1, edbb_1) sont acceptees comme permanentes (FUSE plus ancien que le silicium), gardees dans KNOWN. | utilisateur |
