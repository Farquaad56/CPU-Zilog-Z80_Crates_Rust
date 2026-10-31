from _dsl import T

T("E15-01", "Page cliente: verification de conformite au protocole", kind="code", size=90, crate="puce-z80-web",
  context="""`docs/frontend/index.html` (ecrit par Claude, bilingue FR/EN, titre `Zilog Z80 Emulator`) ne contient AUCUNE logique de CPU : il envoie des commandes et affiche les etats recus. Cette tache verifie mecaniquement cette propriete.""",
  embed=["31_FRONTEND_PROTOCOL.md#31 -"],
  spec="""Test `crates/puce-z80-web/tests/frontend_contract.rs` qui lit `docs/frontend/index.html` et verifie : (1) le titre contient `Zilog Z80 Emulator` ; (2) presence d'un `WebSocket` vers `/ws` ; (3) chaque `cmd` du protocole (hello, list_roms, load, upload, run, pause, step, reset, speed, bp_add, bp_del, view, console_clear) apparait dans le fichier ; (4) AUCUN des motifs de logique CPU : pas de `opcode ===`, `function exec`, `0xDD`... (liste de mots interdits a definir dans le test: `execMain`, `execED`, `execCB`, `alu(`, `parity`, `class Z80`) ; (5) presence des cles i18n `fr` et `en` et du bouton de langue ; (6) aucun `fetch(` vers un autre hote, aucune URL http externe hors CDN Tailwind et polices.
Si la page viole une regle: NE modifie PAS la page (elle appartient a Claude): pose une question Q&R.""",
  write=["crates/puce-z80-web/tests/frontend_contract.rs"],
  tests=[("title", "titre Zilog Z80 Emulator"), ("ws_endpoint", "WebSocket /ws"), ("all_commands", "les 13 cmd sont presentes"), ("no_cpu_logic", "aucun motif interdit"), ("i18n_fr_en", "dictionnaires fr et en + bouton"), ("no_foreign_hosts", "pas d'hote externe hors CDN autorises")])

T("E15-02", "E2E: la page recoit les instantanes du crate (Playwright optionnel)", kind="tooling", size=90, crate="puce-z80-web",
  context="""Test navigateur de bout en bout : la vraie page, le vrai serveur, le vrai crate. Il est OPTIONNEL (necessite Node + Playwright) et ne bloque pas l'acceptation si ces outils sont absents.""",
  spec="""Cree `tests/e2e/browser.mjs` (Node 18+, Playwright) : lance `target/release/z80-web --port 8099`, ouvre `http://127.0.0.1:8099/`, attend que `tests/roms/prelim.com` apparaisse dans la liste, le charge, clique `Run`, attend la fin (`status` done) et verifie via le DOM : (1) le titre `Zilog Z80 Emulator`, (2) la console contient `Preliminary tests complete`, (3) le registre PC affiche n'est pas 0000, (4) la bascule FR/EN change au moins 3 libelles. Sauvegarde une capture 3840x2160 `tests/e2e/shot_4k.png`. Script `tests/e2e/run.sh` : si `node` ou `playwright` est absent, affiche `SKIP` et sort 0.""",
  write=["tests/e2e/**"], tests=[],
  accept=["bash tests/e2e/run.sh"])

T("E15-03", "E2E: ZEXDOC depuis la page", kind="tooling", size=60, crate="puce-z80-web",
  context="Le test final : lancer ZEXDOC dans la page en mode turbo et lire 67 OK sur la console.",
  spec="""Ajoute `tests/e2e/zexdoc.mjs` : meme principe, charge `zexdoc.cim`, vitesse turbo, attend `Tests complete` (timeout 2 h), verifie le compteur `tests.ok == 67`, `tests.err == 0`. `tests/e2e/run.sh --long` l'inclut. SKIP si ROM/Node absents.""",
  write=["tests/e2e/**"], tests=[], accept=["bash tests/e2e/run.sh"])

T("E15-04", "GATE E15 - frontend connecte, controle visuel", gate=True, kind="gate", size=0,
  context="""Verification visuelle par l'utilisateur sur un ecran 3840x2160 : le meme design que le prototype (`https://claude.ai/artifact/XwmEvK6mqZsortvGWs3iLG`) mais alimente par le crate.""",
  user_cmds=["cargo run --release -p puce-z80-web -- --port 8080", "(navigateur) http://127.0.0.1:8080/ : charger prelim.com puis zexdoc.cim, mode turbo", "(verifier) registres, schema de broches du CPU, ALU, desassemblage, trace, pile, memoire, console, FR/EN"], accept=[])
