# 13 - MOTEUR T-STATE (machine a etats de `Cpu::tick`)

Complete `11_PINS_AND_TICK.md` (broches) et `12_PLAN_GRAMMAR.md` (plans). Les champs ci-dessous sont `pub(crate)`; les noms sont IMPOSES (les taches suivantes les utilisent).

## 1. Etat

```rust
pub struct Cpu { pub regs: Regs, pub(crate) eng: Engine }
pub(crate) struct Engine {
    pub table: Table, pub prefix: Prefix,      // table courante, IX ou IY (pour DdFd/DdCb)
    pub op: u8,                                 // opcode de la table courante (valide apres le dernier M1 d'opcode)
    pub plan: Plan, pub idx: usize,             // plan courant et jeton courant
    pub t: u8,                                  // T-state dans le jeton courant (0 = premier T)
    pub active: bool,                           // une instruction est en cours (false = frontiere)
    pub fetches: u8,                            // nombre de M1 deja faits dans l'instruction (prefixes inclus)
    pub cond: bool,                             // drapeau `cond` positionne par X0
    pub tmp_l: u8, pub tmp_h: u8,               // TMP
    pub disp: u8, pub opb: u8,                  // D (deplacement) et OP (octet d'operation DDCB)
    pub wait_pending: bool,                     // un T d'attente a ete insere dans le jeton courant
    pub latched: u8,                            // derniere donnee lue sur D0..D7
    pub xy_addr: u16,                           // adresse IX/IY+d calculee
    pub ddcb: bool,                             // instruction DD/FD CB en cours de lecture (d, op)
    pub restart_cost: u32,                      // T-states de prefixes consommes (Restart)
    pub ts: u32,                                // T-states de l'instruction courante (sans WAIT: voir ts_wait)
    pub ts_wait: u32,
}
```

## 2. Algorithme de `tick(pins_in) -> pins_out`

1. (E09) RESET, BUSRQ, interruptions: traites avant tout (voir 22_INTERRUPTS.md). Jusque-la le moteur les ignore.
2. Si `!active`: debut d'instruction: `table=Main, prefix=None, fetches=0, plan = &[FETCH_PROBE]` ou `FETCH_PROBE = Tok F4` (un M1 de 4 T dont l'opcode n'est pas encore connu), `idx=0, t=0, active=true`.
3. Le jeton courant `cur = plan[idx]` est un jeton de bus (F R W N O I). Les hooks et les jetons `?` faux ont deja ete traites (voir 5).
4. Calculer `pins_out` pour (`cur`, `t`) selon 11_PINS_AND_TICK.md (sorties de broches de chaque type de cycle, section 3 ci-dessous); incrementer `t`; incrementer le compteur de T-states.
5. Si `t == longueur du jeton`: fin de jeton: `idx += 1; t = 0;` puis boucle: tant que `idx < plan.len()` et le jeton courant est un hook: executer `sem(...)`, `idx+=1`; s'il est conditionnel (`?`) et `cond` faux: sauter. Si `idx == plan.len()`: `active=false` (frontiere).
6. Retour de `pins_out`.

Ordre important: les hooks suivant un jeton s'executent dans le meme tick que le dernier T de ce jeton, APRES le calcul de `pins_out` de ce tick (regle 1 de 12_PLAN_GRAMMAR).

## 3. Sorties de broches (rappel et precisions)

Numerotation T1.. = `t` 0..; le CPU latche la donnee lue au tick du T3 (`t==2`) pour R/F/N; WAIT est echantillonne dans `pins_in` au tick du T2 (`t==1`) des jetons F, R, W et au tick du dernier T d'attente automatique (`t==1` de N/O).
- `F4` (M1): t0: A=PC, M1. t1: A=PC, M1 MREQ RD. t2: A=I:R, RFSH MREQ (opcode latche ici, `latched = pins_in.data()`). t3: A=I:R RFSH MREQ. R incremente (7 bits) au tick t0. PC n'est PAS incremente par le moteur sur ce cycle: c'est le jeton qui le fait (`PC+` implicite pour tout F sauf HALT et acquittement) a la fin du T2.
- `F5`/`F6`: comme F4 puis 1/2 T supplementaires: A=I:R, aucune broche de controle.
- `R3`: t0: A=adr. t1: A=adr MREQ RD. t2: A=adr, donnee latchee -> destination. `R4`/`R5`: puis 1/2 T avec A=adr sans controle (la donnee est latchee au t2 comme R3).
- `W3`: t0: A=adr, D=donnee. t1: A, D, MREQ WR. t2: A, D. `W5`: W3 puis 2 T internes (A et D maintenus, sans controle).
- `N4`: t0: A=port. t1: A=port. t2: A=port IORQ RD (WAIT echantillonne ici, 1 TW automatique). t3: A=port, donnee latchee. `O4`: idem avec D=donnee et IORQ WR.
- `I<n>`: n T avec A = derniere adresse du bus, sans controle.
- `WAIT` actif a l'echantillonnage: le moteur insere un T supplementaire (`t` ne progresse pas, mais `ts_wait` +1) en repetant les sorties du T echantillonne, jusqu'a WAIT inactif.

## 4. Decodage et prefixes (le plan d'une instruction n'est connu qu'apres le dernier M1)

- Le premier M1 est `FETCH_PROBE`. A son T3 (`t==2`) l'opcode est latche et `decode(table, byte)` (E02-04) est appele:
  - `Plan(tbl, plan)`: `self.table=tbl; self.plan=plan; self.op=byte;` le M1 en cours devient le jeton `plan[fetches]` (sa longueur peut etre 5 ou 6: M1 etendu), `fetches += 1`, et le moteur continue a `idx = fetches`.
  - `Prefix(tbl)`: `table=tbl` (+ `prefix=Dd/Fd` si DD/FD), `fetches += 1`, le jeton suivant est un nouveau `FETCH_PROBE` (4 T) -> idem.
  - `IndexCb`: `ddcb = true`; jetons suivants imposes: `R3:PC+>D`, `R5:PC+>OP` (lecture de d puis de l'octet d'operation, 5 T pour OP); apres la lecture de OP: `plan = plan_of(DdCb, OP)`, `idx = 4` (les 4 premiers jetons `F4 F4 R3 R5` sont communs), `op = OP`.
  - `Restart(b)`: le prefixe courant est consomme sans effet (le M1 deja fait compte 4 T, R deja incremente), `prefix`/`table` sont recalcules d'apres `b`, `fetches = 0`, puis un nouveau `FETCH_PROBE`. (Cas DD DD, DD FD, DD ED, FD DD...). Aucune interruption n'est acceptee pendant ces enchainements.
- `idx` apres un `Plan`: le plan contient AUSSI les M1 des prefixes (voir 12_PLAN_GRAMMAR regle 5), donc `idx = fetches` (nombre de M1 deja executes).
- Pour `ix_plain` ("prefix ignored"): la semantique du hook est celle de la table `Main` pour le meme opcode (la table DdFd ne fait que dupliquer le plan Main precede d'un M1).

## 5. Adresses (`Addr`)

`PcInc`: adresse = PC puis PC += 1 (l'incrementation a lieu au tick t0 de ce cycle, apres emission de l'adresse). `Sp`: SP. `SpInc`: SP puis SP+=1. `SpPlus1`: SP+1 (SP inchange). `DecSp`: SP-=1 puis adresse SP (le decrement a lieu au tick t0 AVANT emission). `Hl/Bc/De`: registres (jamais IX/IY: l'indexation passe par `Xy`). `Nn`: WZ. `NnPlus1`: WZ+1 (et WZ n'est pas modifie). `Xy`: IX/IY + signed(disp), memorise dans `xy_addr` et dans WZ au premier usage (jeton `R3:PC+>D` termine). `An`: port: A15..A8 = A, A7..A0 = TMP.L (octet n lu); `Bc` en tant qu'adresse de port: BC complet (jetons `N4:BC` `O4:BC`).
