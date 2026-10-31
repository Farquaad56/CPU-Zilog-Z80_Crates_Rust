# FAQ_Z80 (reponses reutilisables; Hermes la lit avant toute question)

## Q: Ou est la semantique exacte d'une instruction (drapeaux, WZ)?
`docs/ref/z80_ref.js` (fait foi, passe ZEXDOC+ZEXALL) et `docs/specs/20_FLAGS_AND_ALU.md` (extraits). Ne jamais deviner un drapeau: comparer aux vecteurs `docs/specs/data/diff.tsv`.

## Q: Comment lancer un sous-ensemble de vecteurs differentiels?
`Z80_DIFF_TABLE=main Z80_DIFF_FAMILY=ld_r_r,ld_r_n cargo test -p puce-z80 --test diff` (voir tache E05-01). Sans variable: tout.

## Q: Le moteur est-il generique ou une enorme fonction?
Generique: plans (tables generees) + hooks `sem`. Ne pas ecrire de `match opcode` de 256 branches pour le timing; seule la semantique est groupee par famille.

## Q: Les ROMs ne sont pas dans le depot.
Normal (D-003 et .gitignore). Utiliser `python docs/tools/fetch_roms.py`. URL en echec -> question Q&R, ne pas inventer d'URL.

## Q: PC dans `Regs` pendant une instruction?
`regs.pc` vaut l'adresse du prochain octet a lire (PC+ incremente a chaque lecture). Hors frontiere d'instruction il est donc "au milieu" de l'instruction.

## Q: Quelle valeur pour R apres reset, registres indefinis?
UNKNOWN - to confirm (Q-001). Defaut du crate: AF=SP=0xFFFF, autres registres inchanges (D-006).
