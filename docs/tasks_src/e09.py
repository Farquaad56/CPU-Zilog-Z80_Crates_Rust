from _dsl import T

INT = "22_INTERRUPTS.md"
PINS = "11_PINS_AND_TICK.md#3. Sequence"

T("E09-01", "RESET (broche et methode)", kind="code", size=90,
  context="""Premiere fonctionnalite de `int.rs` : etat apres RESET. Valeurs non specifiees = `UNKNOWN - to confirm` (question Q-001).""",
  embed=[INT + "#2. Sequences", "23_UNDOCUMENTED.md#23 -"],
  spec="""Cree `int.rs` (`pub(crate) mod int;`). `Cpu::reset(&mut self)` : PC=0, I=0, R=0, IFF1=IFF2=false, IM=0, AF=0xFFFF, SP=0xFFFF, `halted=false`, WZ=0, moteur remis a la frontiere (`active=false`, `cond=false`, `ei_delay=false`). Les AUTRES registres sont laisses inchanges (NMOS) ; `Cpu::new()` appelle `reset()` : AF et SP valent 0xFFFF, BC DE HL IX IY et les jumeaux valent 0. UNKNOWN - to confirm (Q-001) : valeurs reelles des registres indefinis. Documente-le dans un commentaire `UNKNOWN - to confirm`.
Dans `tick` : si `pins.has(Pins::RESET)` : appeler la logique de reset UNE fois au debut puis, tant que RESET est actif, renvoyer des sorties inactives (A=0... controles a 0) sans executer d'instruction ; a la fin de RESET la prochaine instruction commence normalement. (RESET dure >= 3 T dans la vraie puce : si RESET est actif moins de 3 ticks, le crate reset quand meme : documente.)""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/lib.rs"],
  tests=[("reset_values", "apres reset: PC=0 I=0 R=0 IFF=0 IM=0 AF=SP=0xFFFF"), ("reset_midstream", "reset au milieu d'un LD HL,nn remet la frontiere"), ("reset_pin_holds", "RESET actif 5 ticks: aucune sortie de controle, PC reste 0"), ("reset_clears_halt", "halted=false"), ("others_unchanged", "BC DE HL IX IY inchanges par reset()")])

T("E09-02", "HALT: cycles NOP et broche HALT", kind="code", size=70,
  context="""Un Z80 en HALT execute des M1 de NOP (4 T) jusqu'a une interruption : la broche HALT est active et le refresh continue.""",
  embed=[INT + "#2. Sequences", PINS],
  spec="""Au debut d'une instruction (`!active`), si `regs.halted` : le prochain 'instruction' est un M1 de 4 T avec `fetch_pins(pc, i, r, t, 4, halt=true)` (adresse = PC, la valeur de PC ne change pas, R est incremente) ; l'octet lu est ignore. Pas de decodage. `step()` renvoie 4. `halted` reste vrai tant qu'aucune interruption n'est acceptee (E09-04). Convention d'adresse : celle de la reference JS (PC reste sur l'opcode HALT) - UNKNOWN - to confirm, recalage en E13-09.""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs"],
  tests=[("halt_4t", "HALT puis 3 steps: 4 T chacun"), ("halt_pin", "pendant le halt, pins_out contient HALT a chaque T"), ("halt_pc_stable", "PC ne change pas"), ("halt_r_increments", "R augmente de 1 par step"), ("halt_no_bus_write", "aucune ecriture")])

T("E09-03", "Cadre d'acceptation des interruptions", kind="code", size=130,
  context="""Mecanisme commun NMI/INT : echantillonnage a la frontiere d'instruction, EI retarde, pas d'acceptation entre un prefixe et son opcode. Les sequences elles-memes sont en E09-04..06.""",
  embed=[INT + "#1. Echantillonnage", INT + "#4. EI"],
  spec="""Ajoute aux champs d'`Engine` : `irq: IrqKind` (enum `None, Nmi, Int`), `nmi_latch: bool`, `nmi_prev: bool`, `int_level: bool`.
- A chaque `tick` : `int_level = pins.has(INT)` (niveau) ; `if pins.has(NMI) && !nmi_prev { nmi_latch = true }; nmi_prev = pins.has(NMI)` (front montant).
- Au debut d'une instruction (`!active`), AVANT le fetch normal : `if nmi_latch { nmi_latch=false; start_nmi() } else if int_level && regs.iff1 && !eng.ei_delay { start_int() } `. (start_* = stubs ici qui ne font rien, remplis par les taches suivantes.) 
- `ei_delay` : positionne par le hook EI (E05-03) pour la prochaine frontiere ; a la frontiere de l'instruction SUIVANT EI il est remis a false APRES le test (donc INT est ignore une fois). `EI` suivi de `EI` reste inhibe.
- Jamais d'acceptation au milieu d'une chaine de prefixes (DD/FD/ED/CB) : l'etat `active` reste vrai entre le prefixe et l'opcode, donc le test n'a pas lieu ; verifie-le par un test.
- `ei_delay` existe deja (E05-03). `reset()` remet aussi `irq=None`, `nmi_latch=false`, `nmi_prev=false`.
- `Cpu::set_irq_line` n'existe pas : tout passe par `tick(pins)` / `Bus::irq()`.""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/engine/mod.rs", "crates/puce-z80/src/sem/**"],
  tests=[("nmi_edge_latched", "NMI actif 3 ticks de suite = UNE seule demande"), ("nmi_second_edge", "retombee puis remontee = nouvelle demande"), ("int_level_ignored_when_di", "INT actif avec IFF1=0: aucun start_int"), ("ei_delay_one_instruction", "EI puis NOP avec INT actif: l'interruption n'est vue qu'apres le NOP"), ("ei_ei", "EI EI NOP : toujours inhibe pendant EI EI"), ("no_int_mid_prefix", "DD (prefixe) puis opcode: INT ignore entre les deux")])

T("E09-04", "NMI", kind="code", size=100,
  context="""Interruption non masquable : 11 T, saut en 0x0066.""",
  embed=[INT + "#2. Sequences"],
  spec="""`start_nmi()` : `eng.irq = Nmi`; plan statique `NMI_PLAN = [F5 (variante sans PC+, sans decodage), W3:-SP<H, W3:-SP<L, X0]` (ajoute si besoin les variantes `Kind`/champ necessaires : un M1 de 5 T dont l'octet lu est IGNORE, PC non incremente, R incremente, refresh normal) ; TMP = PC (adresse de retour ; si `halted` : PC+1 et `halted=false`). Au hook final : `IFF2 = IFF1; IFF1 = false; PC = 0x0066; WZ = 0x0066`. Total 11 T. `RETN` (ED 45, E07-04) restaure IFF1.""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/engine/**", "crates/puce-z80/src/cpu.rs"],
  tests=[("nmi_11t", "NMI apres un NOP: l'interruption dure 11 T"), ("nmi_pushes_pc", "PC de retour poussee: octet haut a SP+1"), ("nmi_vector", "PC=0x0066 et WZ=0x0066"), ("nmi_iff", "IFF2=ancien IFF1, IFF1=0"), ("nmi_wakes_halt", "NMI pendant HALT: pousse PC+1"), ("nmi_priority", "NMI et INT simultanes: NMI d'abord"), ("retn_restores", "RETN remet IFF1=IFF2")])

T("E09-05", "INT modes 1 et 2 (cycle d'acquittement)", kind="code", size=140,
  context="""Interruption masquable. Le cycle d'acquittement est un M1 avec IORQ (pas MREQ) de 7 T ; l'octet presente sur le bus de donnees est fourni par l'hote (`Bus::int_ack()` ou D0..D7 de `pins_in`).""",
  embed=[INT + "#2. Sequences", INT + "#3. RETI"],
  spec="""`start_int()` selon `regs.im` :
- IM 1 : plan `[IA7, W3:-SP<H, W3:-SP<L, X0]` -> 13 T ; X0 : IFF1=IFF2=false, PC=0x0038, WZ=0x38.
- IM 2 : plan `[IA7, W3:-SP<H, W3:-SP<L, R3:NN>Z, R3:NN+1>W, X0]` -> 19 T ; le vecteur est `(I<<8) | (octet_ack & 0xFE)` (place dans WZ avant les lectures, le plan lit un mot en memoire a cette adresse) ; X0 : PC = mot lu (WZ), IFF1=IFF2=false.
- `IA7` = jeton d'acquittement de 7 T qui utilise `mcycle::int_ack_pins` ; l'octet de bus est latche au dernier T d'attente (UNKNOWN - to confirm t exact : calage en E13-10 et E13-13) dans `eng.latched`. PC n'est PAS incremente. Si `halted` : PC+1 pousse, `halted=false`.
Le `Bus::int_ack()` est appele par `step()` quand `pins_out.is_int_ack()` (adapte E03-12 si necessaire).""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/engine/**", "crates/puce-z80/src/cpu.rs", "crates/puce-z80/src/bus.rs"],
  tests=[("im1_13t", "IM1: INT accepte apres NOP: 13 T, PC=0x0038"), ("im2_19t", "IM2 avec I=0x40, bus=0x20: lit le mot en 0x4020, 19 T"), ("im2_vector_bit0", "octet de bus impair: bit 0 masque"), ("ack_pins", "pendant IA7: M1 et IORQ, jamais MREQ ni RD"), ("iff_cleared", "IFF1=IFF2=0 apres acceptation"), ("im1_wakes_halt", "pousse PC+1"), ("masked_by_di", "DI: pas d'acceptation")])

T("E09-06", "INT mode 0 (instruction sur le bus)", kind="code", size=90,
  context="""En IM0 l'hote place une instruction sur le bus pendant l'acquittement (typiquement RST n).""",
  embed=[INT + "#2. Sequences"],
  spec="""IM 0 : `IA7` puis l'octet d'acquittement est decode comme un opcode `Main` : si c'est un `RST` (`(b & 0xC7) == 0xC7`) : plan `[IA7, W3:-SP<H, W3:-SP<L, X0]` (13 T), X0 : PC = b & 0x38 ; WZ=PC. Autres octets : v1 ne gere que RST ; tout autre octet est traite comme `RST 38H` (0xFF, valeur de repli standard quand le bus est tire vers le haut). Ajoute le commentaire `UNKNOWN - to confirm : CALL nn / autres instructions sur le bus (3 cycles d'acquittement)`.
IFF1=IFF2=false.""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/engine/**"],
  tests=[("im0_rst38", "bus=0xFF: PC=0x0038 en 13 T"), ("im0_rst10", "bus=0xD7: PC=0x0010"), ("im0_other_falls_back", "bus=0x00 traite comme RST 38H"), ("im0_iff", "IFF1=IFF2=0")])

T("E09-07", "BUSRQ / BUSAK", kind="code", size=110,
  context="""Systemes a bus partage (Mega Drive : le Z80 son est mis en pause par le 68000).""",
  embed=[PINS],
  spec="""Echantillonnage de `BUSRQ` au dernier T de chaque cycle machine (fin de jeton de bus F R W N O). Si actif : le CPU termine le cycle, puis passe en etat 'bus accorde' : `pins_out` = `BUSAK` seul (A=0, D=0, aucun controle : representation de la haute impedance), il ne progresse plus (ni `t` ni R) tant que `BUSRQ` est actif ; quand il retombe, `BUSAK` retombe et le jeton suivant reprend. Les T de bus accorde sont comptes dans `tick` mais pas dans `instruction_tstates()`. `Bus::busrq()` alimente la broche dans `step()`.
Pas de BUSRQ pendant les jetons internes `I` ni les hooks (le test a lieu a la fin des jetons de bus seulement).""",
  write=["crates/puce-z80/src/int.rs", "crates/puce-z80/src/engine/**", "crates/puce-z80/src/cpu.rs"],
  tests=[("busrq_after_cycle", "BUSRQ actif pendant un LD A,(nn): le CPU finit le cycle courant avant BUSAK"), ("busak_pins", "pendant l'etat accorde: BUSAK, aucun autre controle"), ("resume", "BUSRQ retombe: l'instruction se termine, meme resultat que sans BUSRQ"), ("no_progress", "R et PC ne bougent pas pendant BUSAK"), ("busak_clears", "BUSAK retombe avec BUSRQ"), ("step_with_bus", "Bus::busrq() vrai 10 ticks: step ne se termine qu'apres")])

T("E09-08", "Integration Bus: irq(), nmi() et RETI visibles", kind="code", size=100,
  context="""Les hotes utilisent surtout `Bus::irq()/nmi()` avec `step()`. Verifie la chaine complete et l'observabilite de RETI pour les peripheriques en daisy chain (Z80 CTC/PIO/SIO).""",
  embed=[INT + "#3. RETI", "32_REUSE_GUIDE.md#"],
  spec="""- `step()` : traduit `bus.irq()` -> `Pins::INT`, `bus.nmi()` -> `Pins::NMI` dans `pins_in` a CHAQUE tick ; si une interruption est acceptee, `step()` renvoie la duree de la sequence d'acceptation seule (13, 19, 11 T), l'instruction suivante est executee au `step` d'apres.
- RETI (ED 4D) : le moteur execute les deux M1 normalement (rien de special) ; ecris un test avec un bus espion qui verifie que le peripherique voit les octets `ED` puis `4D` sur les cycles M1 successifs (via `on_tick`), comme le fait la vraie daisy chain.
- Ajoute dans `tests/interrupts_bus.rs` un scenario complet : programme `EI; HALT` en IM1, le bus declenche `irq()` apres 100 ticks, handler en 0x38 `INC A; EI; RETI`.""",
  write=["crates/puce-z80/src/cpu.rs", "crates/puce-z80/tests/interrupts_bus.rs", "crates/puce-z80/src/int.rs"],
  tests=[("irq_scenario", "scenario EI/HALT/IRQ/handler/RETI: A incremente une fois, retour apres HALT"), ("reti_visible", "on_tick voit ED puis 4D en M1"), ("nmi_from_bus", "bus.nmi() declenche 0x66"), ("step_returns_ack_duration", "step de l'acceptation renvoie 13"), ("irq_level_retriggers", "INT maintenu: nouvelle acceptation si EI a nouveau")])

T("E09-09", "GATE E09 - interruptions validees", gate=True, kind="gate", size=0,
  context="""NMI, INT 0/1/2, HALT, RESET, BUSRQ fonctionnent. Les valeurs `UNKNOWN - to confirm` seront calees en E13 (FUSE/SingleStepTests).""",
  user_cmds=["cargo test -p puce-z80 e09_", "cargo test -p puce-z80 --test interrupts_bus", "bash tools/ci.sh"], accept=[])
