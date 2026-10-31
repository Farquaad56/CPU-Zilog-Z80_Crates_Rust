# 02 - PROTOCOLE QUESTIONS / REPONSES (Q&R)

But: Hermes ne devine jamais. Quand une information manque, il **pose une question
structuree et se bloque**. Un humain (utilisateur) ou Claude repond; la reponse est
tracee, rendue durable, puis la tache est debloquee.

Le systeme Q&R a deux faces:
1. **Questions bloquantes de Hermes** pendant une tache (flux principal, ci-dessous).
2. **Questions de conception** (FAQ Z80) deja pre-repondues dans `docs/qa/FAQ_Z80.md`:
   Hermes doit la consulter AVANT de poser une question.

## 1. Quand poser une question (obligatoire)

Hermes DOIT bloquer + poser une question si, et seulement si:
- une valeur (registre, bit, timing, opcode, cycle) n'est pas dans la tache ni dans les specs listees
  dans `Lire d'abord`  -> type `spec-missing`;
- deux sources de la tache se contredisent -> `conflict`;
- la tache est ambigue sur un comportement -> `spec-ambiguous`;
- un outil/fichier necessaire est absent (ROM de test, JSON, dependance) -> `tooling`;
- un choix de conception non decide (nom public, format) -> `decision`.

Hermes NE pose PAS de question pour: style de code, noms de variables internes,
organisation interne d'un fichier de son scope, choix d'algorithme equivalent.

Avant toute question: relire la tache, les specs listees, `docs/qa/FAQ_Z80.md`,
`docs/qa/DECISIONS.md`. Si la reponse y est, l'appliquer.

## 2. Format de la question (Hermes ecrit ce fichier, toujours autorise hors scope)

Fichier: `docs/qa/inbox/<TASKID>-Q<n>.md` (ex: `docs/qa/inbox/E02-03-Q1.md`), contenu exact:

```
# <TASKID>-Q<n>
TYPE: spec-missing | conflict | spec-ambiguous | tooling | decision
TASK: <TASKID>
BLOCKED_ON: <fichier>:<fonction ou ligne> (ou "n/a")
QUESTION: <une seule question, une phrase>
WHAT_I_KNOW: <faits tires de la tache/specs, avec reference de section>
OPTIONS:
  A) <option> -> consequence
  B) <option> -> consequence
DEFAULT_I_WOULD_PICK: <A|B|none> because <raison courte>
CODE_STATE: <ce qui est deja ecrit et compile / ce qui est stubbe avec "UNKNOWN - to confirm">
```

Puis Hermes appelle `kanban_block(reason="QUESTION <TASKID>-Q<n>: <la question>. See docs/qa/inbox/<TASKID>-Q<n>.md")`
(ou `hermes kanban block <id> "<meme texte>"`), laisse les stubs `UNKNOWN - to confirm`
dans le code, et s'arrete. Il ne continue pas sur autre chose dans la meme tache.

Si la tache n'utilise pas Kanban (mode terminal opencode): Hermes ecrit le fichier et
repond uniquement `BLOCKED <TASKID>-Q<n>`.

## 3. Repondre (Claude ou utilisateur)

```
python docs/tools/qa.py list                       # questions ouvertes
python docs/tools/qa.py show E02-03-Q1
python docs/tools/qa.py answer E02-03-Q1 --choice A --text "Reponse courte et precise" [--durable]
```

`qa.py answer` fait:
1. ajoute une entree **append-only** a `docs/qa/QA_LOG.md` (date, tache, question, reponse, auteur);
2. si `--durable`: ajoute une decision numerotee `D-nnn` dans `docs/qa/DECISIONS.md`;
3. deplace le fichier vers `docs/qa/answered/`;
4. affiche les commandes Kanban a executer:
   ```
   hermes kanban comment <kanban_id> "ANSWER E02-03-Q1 (choice A): <texte>. Logged in docs/qa/QA_LOG.md"
   hermes kanban unblock <kanban_id>
   ```
   (Claude fournit ces commandes pretes a coller; l'ID Kanban est dans `docs/kanban/id_map.json`
   ecrit par l'import.)

Regle de reponse: reponse **courte, chiffree, non ambigue**. Si la reponse change une spec,
Claude met aussi a jour le fichier `docs/specs/*.md` concerne dans le meme geste.

## 4. Reprise par Hermes

A la reprise (le worker relit le fil de commentaires Kanban), Hermes:
1. lit le commentaire `ANSWER`;
2. remplace les `UNKNOWN - to confirm` concernes;
3. finit la tache selon `Acceptance`;
4. ne touche pas a QA_LOG/DECISIONS (c'est Claude/utilisateur).

## 5. Priorite des sources (en cas de conflit)

1. `docs/qa/DECISIONS.md` (le plus recent gagne)
2. La tache elle-meme
3. `docs/specs/*.md`
4. Documentation Zilog Z80 UM0080 + Sean Young "Undocumented Z80" (reference de Claude, pas de Hermes)
5. Tout le reste = `UNKNOWN - to confirm`

## 6. Lecons apprises (boucle de retour)

A chaque epopee terminee, Claude relit `QA_LOG.md`: toute question posee 2 fois ou plus
signale une lacune de spec -> corriger la spec et les taches non demarrees de l'epopee suivante.
