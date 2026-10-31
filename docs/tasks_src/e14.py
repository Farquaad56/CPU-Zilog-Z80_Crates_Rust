from _dsl import T

P31 = "31_FRONTEND_PROTOCOL.md#31 -"

T("E14-01", "Session: commandes et boucle de simulation", kind="code", size=150, crate="puce-z80-machines",
  context="""Le serveur web ne calcule rien : il pilote une `Session` qui possede un VRAI `CpmMachine` (D-004). La session tourne dans son propre thread et recoit des commandes par canal.""",
  embed=[P31],
  spec="""Dans `crates/puce-z80-machines/src/session.rs` :
```rust
pub enum Command { Load{name:String, bytes:Vec<u8>}, Run, Pause, Step(u32), Reset, Speed(u8), BpAdd(u16), BpDel(u16), View(ViewParams), ConsoleClear, Quit }
pub struct Session { tx: Sender<Command>, shared: Arc<Mutex<Snapshot>> }
impl Session { pub fn spawn() -> Session; pub fn send(&self, c: Command); pub fn snapshot(&self) -> Snapshot /* clone */ }
```
Thread : boucle qui traite les commandes en attente, puis si `running`, execute des instructions selon `speed` (table [1,4,15,60,300,3000,60000,0=turbo] instructions/s : pacing par budget de temps de 10 ms ; turbo = budget 40 ms par tranche), s'arrete sur point d'arret (`status="bp"`), HALT (`halted`), fin de programme (`done`). `Snapshot` ici = struct vide `Snapshot { status: String, n: u64, t: u64 }` (completee en E14-02). `Step(n)` execute n instructions meme en pause. `Reset` recharge la ROM courante. Aucune attente active (utilise `recv_timeout`).""",
  write=["crates/puce-z80-machines/src/session.rs", "crates/puce-z80-machines/src/lib.rs"],
  tests=[("load_and_step", "Load d'un mini programme puis Step(3): n==3"), ("run_until_exit", "Run d'un programme qui se termine: status passe a done"), ("breakpoint_stops", "BpAdd puis Run: status bp et PC au breakpoint"), ("pause_stops", "Run puis Pause: n cesse d'augmenter"), ("reset_reloads", "Reset remet n a 0"), ("speed_table", "Speed(2) limite le debit a ~15 instr/s (mesure large)")])

T("E14-02", "Session: instantane d'etat (Snapshot) conforme au protocole", kind="code", size=150, crate="puce-z80-machines",
  context="""Construit le JSON `state` du protocole 31 depuis le VRAI crate (registres, bus, ALU, desassemblage, trace, pile, memoire, journal d'ecritures).""",
  embed=[P31],
  spec="""Complete `Snapshot` (serde `Serialize`) avec les champs EXACTS du protocole (alu.op = NOM en chaine, dis[].pre = chaine, bus.pins = hex en chaine, tests{ok,err,total}, hello{version}) (type="state", status, n, t, ips, mhz, tlast, regs{...}, bus{...}, alu{...}|null, dis[...], trace[...], stack[...], mem{base,hex}, wlog[...], console_delta, tests{ok,err,total}, bps[...], rom{name,size,kind}). Source des donnees : `cpu.regs`, `cpu.last_bus()`, `cpu.last_alu()`, `disassemble(...)` (feature disasm), `cpu.pc_history()`, `cpu.recent_writes()`, RAM de la machine. `ViewParams { mem_base, dis_rows, trace_rows, stack_rows, follow }` avec les valeurs par defaut du protocole. `ips` et `mhz` mesures sur une fenetre glissante de 500 ms (T-states reels du crate / secondes). `console_delta` = texte ajoute depuis le dernier instantane envoye (curseur par client: la session fournit `console_since(pos)`).
Aucune valeur ne doit etre calculee ailleurs que dans le crate (sauf mise en forme).""",
  write=["crates/puce-z80-machines/src/session.rs"],
  tests=[("json_fields", "serde_json::to_value(snapshot) contient tous les champs du protocole"), ("regs_match_cpu", "regs.pc/af/... egalent cpu.regs"), ("dis_rows", "dis a dis_rows lignes a partir de PC, textes issus du desassembleur"), ("stack_words", "stack liste les mots au-dessus de SP"), ("mem_hex_len", "mem.hex a 512 caracteres"), ("bus_view", "bus reflete le dernier cycle"), ("alu_after_add", "alu non null apres ADD"), ("wlog_after_write", "wlog contient l'adresse ecrite")])

T("E14-03", "Serveur HTTP: page et sante", kind="code", size=100, crate="puce-z80-web",
  context="""Le serveur sert la page `docs/frontend/index.html` embarquee dans le binaire (`include_str!`), uniquement sur 127.0.0.1.""",
  embed=[P31],
  spec="""Dans `crates/puce-z80-web/src/http.rs` : copie `docs/frontend/index.html` vers `crates/puce-z80-web/assets/index.html` (via `build.rs` ou copie manuelle commitee: choisis `include_str!("../../../docs/frontend/index.html")` pour garder UNE seule source). `GET /` et `/index.html` -> 200 text/html; charset=utf-8 ; `GET /health` -> 200 `{"ok":true,"version":"..."}` ; autre -> 404. Ecoute `127.0.0.1:<port>` UNIQUEMENT (jamais 0.0.0.0). `fn start(port) -> Result<Server, String>` ; erreurs d'ouverture du port rapportees clairement.""",
  write=["crates/puce-z80-web/src/**", "crates/puce-z80-web/Cargo.toml"],
  tests=[("index_served", "GET / renvoie 200 et contient `Zilog Z80 Emulator`"), ("health", "GET /health -> JSON ok"), ("not_found", "GET /x -> 404"), ("bind_localhost", "l'adresse d'ecoute est 127.0.0.1"), ("port_in_use", "un second serveur sur le meme port renvoie Err")])

T("E14-04", "Serveur WebSocket (/ws)", kind="code", size=150, crate="puce-z80-web",
  context="""Meme port que HTTP : upgrade WebSocket sur `/ws`. Un thread par client.""",
  notes="UNKNOWN - to confirm : `tiny_http::Request::upgrade` + `tungstenite::WebSocket::from_raw_socket` + `tungstenite::handshake::derive_accept_key` est l'approche prevue. Si elle ne fonctionne pas, pose une question Q&R avant de passer a un second port.",
  embed=[P31],
  spec="""Dans `ws.rs` : sur `GET /ws` avec `Upgrade: websocket` : calcule `Sec-WebSocket-Accept` (derive_accept_key), repond 101 via `request.upgrade("websocket", response)`, enveloppe le flux avec `WebSocket::from_raw_socket(stream, Role::Server, None)`. Boucle client : lecture bloquante avec timeout court (`set_read_timeout` impossible sur le flux boxe: utilise un thread lecteur + canal), messages texte JSON -> `ClientMsg` (serde, `#[serde(tag="cmd")]`, champs du protocole) -> `Command` de la Session. Un message invalide renvoie `{"type":"error","message":"..."}` sans fermer.
Un thread d'envoi pousse `{"type":"state",...}` : immediatement apres chaque commande, et au plus 30 fois/s pendant `run` (seulement si l'instantane a change).""",
  write=["crates/puce-z80-web/src/**"],
  tests=[("handshake", "un client tungstenite se connecte a /ws"), ("hello_reply", "hello -> roms puis state"), ("bad_json", "message invalide -> type error, connexion ouverte"), ("two_clients", "deux clients recoivent chacun les instantanes"), ("state_rate", "pendant run: <= 31 messages/s")])

T("E14-05", "Commandes: roms, load, upload", kind="code", size=120, crate="puce-z80-web",
  context="Liste et chargement de ROMs (dossier `tests/roms`, sous-dossiers inclus) et envoi depuis la page.",
  embed=[P31],
  spec="""`list_roms` : parcourt `--roms DIR` (recursif, profondeur <= 3) pour `.com`, `.cim`, `.bin` ; renvoie `{type:"roms",roms:[{name,size,total_tests}]}` (`total_tests` = 67 pour zexdoc/zexall, 1 pour prelim, 0 sinon ; table en dur par nom). `load{name}` : lit le fichier (refus si le chemin sort du dossier: pas de `..`), `Command::Load`, reset, repond `state`. `upload{name,data_b64}` : decode base64 (taille max 1 Mo), charge. `.com/.cim` -> CP/M a 0100h ; `.bin` -> brut a 0000h (kind="raw").""",
  write=["crates/puce-z80-web/src/**"],
  tests=[("list_finds_roms", "un dossier temporaire avec 2 ROMs: roms les liste"), ("load_ok", "load charge et renvoie un state avec rom.name"), ("path_traversal_rejected", "load de ../x -> error"), ("upload_b64", "upload d'un mini programme"), ("upload_too_big", "> 1 Mo -> error"), ("raw_kind", ".bin -> rom.kind raw")])

T("E14-06", "Commandes: run, pause, step, reset, speed, breakpoints, view", kind="code", size=110, crate="puce-z80-web",
  context="Transmission des commandes de pilotage a la session et verification du comportement de bout en bout (cote serveur).",
  embed=[P31],
  spec="""Relie les `ClientMsg` restants (`run`, `pause`, `step{n}`, `reset`, `speed{idx}` (0..7, sinon error), `bp_add{addr}`, `bp_del{addr}`, `view{...}` (valeurs bornees: dis_rows<=100, trace_rows<=64, stack_rows<=64, mem_base aligne sur 16), `console_clear`) a `Command`. Chaque commande declenche un `state`. Erreurs d'argument -> `error`.""",
  write=["crates/puce-z80-web/src/**"],
  tests=[("step_advances", "step n=5 -> n du state == 5"), ("run_pause", "run puis pause: status idle et n stable"), ("speed_idx_range", "speed idx 9 -> error"), ("bp_roundtrip", "bp_add puis bps contient l'adresse; bp_del la retire"), ("view_bounds", "dis_rows 1000 est borne a 100"), ("console_clear", "console_clear vide la console")])

T("E14-07", "Test bout en bout du serveur (client Rust)", kind="code", size=140, crate="puce-z80-web",
  context="""Prouve que le serveur est CONNECTE au vrai crate : un client WebSocket pilote un programme et on compare avec une execution directe du crate.""",
  spec="""`crates/puce-z80-web/tests/e2e.rs` : lance le serveur sur un port libre (thread), se connecte avec `tungstenite`, `load` d'un petit programme CP/M ecrit en octets (ex. `LD A,5; LD B,3; ADD A,B; LD E,A; ... JMP 0`), envoie `step{n:3}` et verifie : `regs.af` et `regs.bc` du JSON == valeurs d'un `Cpu` execute DIRECTEMENT dans le test avec le meme programme (`CpmMachine`) ; verifie aussi `dis[0].text` == `disassemble(...)`; puis `run` jusqu'a `done` et compare `n`, `t` et la console.""",
  write=["crates/puce-z80-web/tests/e2e.rs"],
  tests=[("regs_equal_direct_cpu", "regs du JSON == regs du Cpu direct apres 3 instructions"), ("dis_from_crate", "dis[0].text == disassemble"), ("run_to_done", "n, t et console identiques a l'execution directe"), ("alu_from_crate", "alu.op/r issus du crate apres ADD")])

T("E14-08", "Binaire z80-web: demarrage, arret propre", kind="code", size=70, crate="puce-z80-web",
  context="Assemble les morceaux dans `main`.",
  spec="""`main` : parse les arguments (E01-03), lance `Session`, `http::start`, affiche `Zilog Z80 Emulator: http://127.0.0.1:<port>/` ; Ctrl-C (SIGINT) arrete proprement via un `AtomicBool` et `Command::Quit` (pas de crate externe: utilise `std::sync` + un thread qui lit stdin pour la touche `q`+Entree comme alternative portable). Code de sortie 0.""",
  write=["crates/puce-z80-web/src/main.rs", "crates/puce-z80-web/src/**"],
  tests=[("startup_message", "la fonction `banner(port)` renvoie la chaine attendue"), ("quit_command", "Command::Quit termine le thread de session")],
  accept=["cargo test -p puce-z80-web e14_08_", "cargo build --release -p puce-z80-web"])

T("E14-09", "GATE E14 - serveur valide", gate=True, kind="gate", size=0,
  context="Le serveur pilote le vrai crate et sert la page. Essai manuel avant l'integration du client.",
  user_cmds=["cargo test -p puce-z80-web -p puce-z80-machines", "cargo run --release -p puce-z80-web -- --port 8080", "(ouvrir http://127.0.0.1:8080/ dans un navigateur)"], accept=[])
