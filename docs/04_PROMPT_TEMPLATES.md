# 04 - PROMPTS PRETS A COLLER (mode terminal opencode, sans Kanban)

Avec le Kanban, la carte contient deja tout. Ces prompts servent quand on lance Hermes a la main (opencode) ou pour relancer une tache.
Claude les remplit (`<ID>`, extraits) et les donne a l'utilisateur. Tout en ASCII cote code, reponses de Hermes courtes.

## A. Demarrer une tache

```
Tache <ID>. Lis docs/tasks/<ID>.md EN ENTIER, puis les specs qu'elle cite.
Ecris uniquement dans les chemins de "Write only in". Code et commentaires en anglais ASCII.
Valeur inconnue: commentaire exact `UNKNOWN - to confirm` + question (docs/02_QA_PROTOCOL.md), ne devine pas.
Quand les tests de la section Acceptance passent, reponds uniquement: DONE <ID> + la liste des fichiers modifies.
Ne lance ni task.py ni git commit/push.
```

## B. Corriger apres un echec de `task.py check`

```
La verification de <ID> a echoue. Sortie (recopiee telle quelle):
<sortie de task.py check>
Corrige uniquement ce qui est en cause, dans le meme scope "Write only in" de docs/tasks/<ID>.md.
Ne reecris pas ce qui passe. Relance les commandes de la section Acceptance, puis reponds: DONE <ID>.
```

## C. Reprendre apres une reponse a une question

```
Reprise de <ID>. La question <ID>-Qn a recu la reponse suivante (aussi dans docs/qa/QA_LOG.md):
<reponse>
Remplace les `UNKNOWN - to confirm` concernes, termine la tache selon Acceptance, reponds DONE <ID>.
```

## D. Tache de reglage TUNE-nn (creee par Claude)

```
Tache <TUNE-nn>. Symptome observe par l'utilisateur: <symptome>. Cause probable identifiee par Claude: <cause>.
Modification demandee (et rien d'autre): <changement precis, fichier, fonction, valeur>.
Test a ajouter ou ajuster: <nom et attendu>. Acceptance: <commandes>.
```

## E. Diagnostic par Claude (ce que Claude repond a l'utilisateur, en francais)

1. Verdict en une phrase (cause).
2. Le correctif: soit le prompt B, soit une correction de spec/tache + `python docs/tools/build_tasks.py`.
3. Les commandes de reprise (git, task.py, Kanban).
