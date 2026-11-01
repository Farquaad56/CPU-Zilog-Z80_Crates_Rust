# 06 - Chaine multi-bots (codeur, relecteur, validateur)

Valable a partir de E03 (D-023). E01 et E02 ont ete valides a la main.

## Les roles
| Role | Profil Hermes | Ce qu'il fait | Ce qu'il ne fait jamais |
|---|---|---|---|
| Chef de file | dispatcher Hermes (gateway) | lance la carte prete suivante (chaine D-020) | - |
| Codeur | `z80_crate` | implemente UNE tache, lance les commandes de "Acceptance", puis `kanban_request_review` | `task.py check/done/approve`, git |
| Relecteur | `relecteur` | verifie la conformite a la fiche, puis `kanban_complete` (OK) ou `kanban_request_changes` | modifier le code, git |
| Validateur | `validateur` | carte `<id>-V`: lance `tools/validate.ps1 <id>` (check, done, commit local) | coder, push, approuver une gate |
| Humain | vous | gates (`task.py approve` + cloture de la carte), push GitHub | - |

## Flux d'une tache X
1. Carte X (codeur). A la fin: `kanban_request_review(reviewer="relecteur", summary="REVIEW BRIEF: voir docs/06_PIPELINE_MULTI_BOTS.md#review. <ce qui a ete fait et verifie>")`.
2. La carte X passe en `review` (relecteur). Verdict: `kanban_complete` ou `kanban_request_changes(reason=...)` (retour au codeur, meme carte).
3. La carte `X-V` (validateur) devient prete. Elle valide et commite. En cas d'echec elle se bloque (`needs_input`) et commente la carte X.
4. La tache suivante ne demarre qu'apres `X-V` (les cartes sont chainees: X -> X-V -> suivante).

## Review
Le relecteur travaille dans le depot (lecture seule) et verifie, dans cet ordre:
1. Lis `docs/tasks/<id>.md` (fiche) et le resume du codeur.
2. Perimetre: `python docs/tools/task.py scope <id>` ne doit lister aucun fichier hors "Write only in" (`git status --short` pour voir les changements).
3. Tests: chaque test nomme dans la fiche existe (`rg "fn <nom>"`) et les commandes de "Acceptance" passent. Lance-les.
4. Regles permanentes: anglais ASCII, pas de f32/f64, pas d'unsafe, pas de dependance, aucun `UNKNOWN - to confirm` supprime ou devine, aucune valeur inventee (comparer aux specs citees).
5. Qualite: lance `powershell -NoProfile -ExecutionPolicy Bypass -File tools\ci.ps1` en entier (fmt, clippy -D warnings, tous les tests du workspace, y compris ceux des taches precedentes). Il doit finir par `all checks passed`. Toute erreur, meme un lint ou un test ancien casse, = `kanban_request_changes` (jamais `kanban_complete`).
6. Verdict. Tout OK: `kanban_complete(summary="conforme: <3 points verifies>")`. Sinon `kanban_request_changes` avec la liste PRECISE (fichier, ligne, regle violee). Ne corrige jamais toi-meme.

## Mise en place
```
python docs/tools/kanban_validators.py --dry-run --from E03-01
python docs/tools/kanban_validators.py --from E03-01
```
Les gates n'ont pas de carte `-V`. Gate: `task.py approve <id>` puis `hermes kanban --board z80 complete <carte> --summary "..."`.
Le push GitHub reste manuel (decision D-023).
