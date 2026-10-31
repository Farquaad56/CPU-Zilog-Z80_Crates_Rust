# 05 - TUTORIEL (pas a pas)

0. Prerequis: Rust stable (rustup), Python 3.8+, Node 18+ (uniquement pour regenerer les vecteurs), Hermes installe avec le profil `Z80_Crate`, acces reseau pour E13.
1. Ouvrir un terminal dans `H:\__Emulator__\CPU Zilog Z80_Crates_Rust` (guillemets autour du chemin).
2. `python docs/tools/build_tasks.py --check` doit afficher `OK`.
3. Creer le board et importer (voir `03_KANBAN_HERMES.md`), 5 taches d'abord.
4. Laisser Hermes executer E01-01. A la fin: `python docs/tools/task.py check E01-01`, puis `done`, puis `git add -A && git commit`.
5. Repeter. A chaque GATE (E01-12, E03-15, ...) executer les commandes indiquees dans la carte.
6. En cas de blocage: lire `docs/qa/inbox/`, repondre avec `python docs/tools/qa.py` (voir `02_QA_PROTOCOL.md`).
7. Les prompts manuels (sans Kanban) sont dans `04_PROMPT_TEMPLATES.md`.
8. Regeneration apres modification des specs/DSL: `python docs/tools/build_tasks.py` (n'ecrase jamais `PROGRESS.json`).

## Profil Hermes Z80_Crate (a faire AVANT l'import)
Toutes les taches sont assignees au profil `Z80_Crate`. Cree-le et branche-le sur LM Studio:
```
hermes profile create Z80_Crate
hermes profile describe Z80_Crate --text "Code Rust du crate Z80 cycle-accurate (puce-z80) a partir des docs/tasks/*.md, une tache a la fois"
hermes -p Z80_Crate config
```
Dans la config: fournisseur = endpoint compatible OpenAI (custom), base URL `http://localhost:1234/v1` (serveur local de LM Studio, bouton "Start Server"),
cle API quelconque (ex. `lm-studio`), modele = `qwen3.8-27b-turbo-fable-cold-fusion-735-882-heretic-uncensored-neo-coder-max-mtp`
(identifiant exact tel qu'affiche par LM Studio, onglet Developer). Active les outils fichiers + terminal. Contexte >= 32k.
Syntaxe exacte des sous-commandes `profile`/`config`: UNKNOWN - to confirm (`hermes profile --help`).
Verification: `hermes profile list` doit montrer `Z80_Crate`, puis `python docs/tools/kanban_import.py --dry-run --limit 5`.
