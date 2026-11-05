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
Resolu (D-028): apres RESET, PC=I=R=0, IFF1=IFF2=0, IM=0, AF=SP=0xFFFF; BC DE HL IX IY et le jeu alterne restent inchanges (Zilog ne specifie pas leur valeur).

## ROMs: sources verifiees
Verifie le 2026-11-05 (E13-01) avec `git clone --depth 1` / HTTP HEAD depuis la machine du projet.

- PRELIM / ZEXDOC / ZEXALL: les 3 URL superzazu/z80 repondent 200; tailles 1280 / 8588 / 8588; SHA-256 dans `tests/manifest.toml`.
- z80test (Patrik Rak): https://github.com/raxoft/z80test, clone ok, commit c490c0ca94f1021509f8bd656b413861bd7cfb41 (2023-12-02).
  Le depot ne contient QUE les sources: `src/*.asm` (main, tests, z80full, z80doc, z80flags, z80docflags, z80ccf, z80ccfscr, z80memptr, ...),
  `img/`, `readme.txt`, `changes.txt`, `license.txt`. Pas de .tap dans le depot.
  Les .tap precompiles sont dans les RELEASES: https://github.com/raxoft/z80test/releases/download/v1.2a/z80test-1.2a.zip
  (dernier tag v1.2a; contient z80full.tap, z80doc.tap, z80flags.tap, z80docflags.tap, z80ccf.tap, z80memptr.tap, z80ccfscr.tap).
  Pour E13-15: telecharger ce zip (aucune ROM Spectrum dedans), ne pas assembler.
- SingleStepTests/z80: https://github.com/SingleStepTests/z80, clone ok, commit ebe1875d48f374bcfd4b505d8eb8ee751568b5f7 (2026-04-11).
  Arborescence: `v1/` (1604 fichiers .json, ~600-770 Ko chacun, 1000 cas par fichier), `generation/`, `.github/`, `README.MD`, `LICENSE`.
  Noms de fichiers: `00.json` (main), `cb 34.json`, `dd 00.json`, `ed ..`, `fd ..`, `dd cb __ 00.json` (DDCB/FDCB).
  Cas: `name, initial, final, cycles`; `initial/final` = a b c d e f h l i r af_ bc_ de_ hl_ ix iy pc sp wz iff1 iff2 im ei p q ram.
  `cycles` = liste de `[adresse, donnee|null, "rwmi"-style pins]`, ex. `[19935, null, "----"]`, `[19935, null, "r-m-"]` (detail: E13-06).
- FUSE: https://raw.githubusercontent.com/FUSE-emulator/fuse/master/z80/tests/tests.in (161511 octets),
  `tests.expected` (323000 octets) et `README` (2469 octets) repondent 200.
- ROMs de timing / interruptions (Woodmass, Raxoft): non verifiees ici, la liste exacte est etablie en E13-18.
- `fetch_roms.py --optional singlestep,z80test` clone dans `tests/roms/<nom>/` (jamais commite).

## z80test: sortie et execution (E13-15)

- Detail complet: `docs/specs/35_Z80TEST.md`. Le message de reussite est `Result: all tests passed.` (pas "Passed" comme dit dans 30_TEST_SUITES.md); en cas d'echec: `Result: <n> of <m> tests failed.` et, par test, `FAILED` + `CRC:... Expected:...`.
- Le programme ne retourne aucun code: il imprime par RST 10h (codes 13, 23+2 octets de TAB, 127). Il faut `0xBF` en lecture du port 0xFE, sinon les tests IN sont refuses.
- Une ROM 48K n'est pas obligatoire: simuler `0x1601` (CHAN-OPEN = RET) et `0x0010` (capture de A) suffit; CODE charge en `0x8000`, entree `0x8000`.
- Les `.tap` sont dans la release v1.2a (zip), pas dans le depot git.

## ROMs de timing et d'interruptions (E13-18)

- Detail et criteres: `docs/specs/36_TIMING_ROMS.md`. Retenus (telecharges, sha256 dans `tests/manifest.toml`, copies dans `tests/roms/timing/`): Woodmass `z80tests.tap`, timing 48K `timing_tests_48k_v1.0.z80`, Block Flags v5.0-beta, IM1 vs JR.
- Seul IM1 vs JR se verifie sans ROM (HL = 0 au retour). Les autres exigent la ROM 48K de l'utilisateur (`tests/roms/zx48.rom`, jamais commitee).
- Le verdict automatique du test de timing 48K est `UNAVAILABLE` (valeurs de reference non publiees sous forme exploitable).
- FDDD (Woodmass) exige le mode allRAM du +3: non retenu.

## Performance
`z80 bench --seconds N` (E13-20) runs a NOP / LD A,n / ADD A,B / LD B,A / LD B,n / JR loop on the
romless ZX machine in release mode and prints instructions/s and T-states/s ("MHz equivalent").
- Objectif: >= 15 MHz equivalent (ZEXDOC/ZEXALL, 46,7 G T-states, < 1 h).
- Mesure (avant toute optimisation, release, 5 s): 20.1 MIPS, 127.6 MHz equivalent.
- Resultat: l'objectif est atteint avec une marge de ~8x; aucune optimisation n'a ete necessaire
  (le moteur est inchange; ZEXALL complet ~6 min estimees a ce debit).
- Le debit reel varie avec la machine et avec le melange d'instructions (les instructions
  prefixees et les blocs sont plus lents que cette boucle).
