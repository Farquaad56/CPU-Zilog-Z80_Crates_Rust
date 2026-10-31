# 24 - VECTEURS DE TEST DIFFERENTIELS (docs/specs/data/diff.tsv)

7632 cas produits par le coeur de reference (`docs/ref/z80_ref.js`, passe ZEXDOC/ZEXALL) via `node docs/tools/gen_diff_vectors.js`:
6 vecteurs par opcode pour chaque table (`main`, `cb`, `ed`, `ddfd`, `ddcb`). Pour `ddfd` et `ddcb`, les vecteurs de numero impair utilisent le prefixe FD (IY), les pairs DD (IX).
Chaque ligne donne l'ETAT FINAL; l'etat initial est reconstruit par le test Rust a partir d'une graine (aucun octet initial n'est stocke).

## 1. PRNG et memoire (a reimplementer EXACTEMENT)

```
xorshift32:  s ^= s << 13; s ^= s >> 17; s ^= s << 5     (u32, decalages logiques, wrapping)   xorshift32(1) donne 270369, 67634689, 2647435461, 307599695, 2398689233
seed(table_id, opcode, vec) = ( (table_id*0x10000 + opcode*16 + vec) * 2654435761 mod 2^32 ) | 1        (table_id: main=0 cb=1 ed=2 ddfd=3 ddcb=4)
n8()  = next() >> 24 ;  n16() = next() >> 16     (next() = un pas de xorshift32 puis lecture de s)
mem(a) = hash(a, seed):  h = (a ^ (seed * 0x9E3779B1)) * 0x45D9F3B ; h ^= h >> 16 ; h = h * 0x45D9F3B ; h ^= h >> 16 ; renvoie h >> 24     (tout en u32 wrapping)
                         exemples: hash(0, 1) = 233 ; hash(65535, 7) = 174
```

## 2. Etat initial (ordre de tirage)

1. n8() pour A, F, B, C, D, E, H, L (dans cet ordre), puis n8() pour A', F', B', C', D', E', H', L'.
2. n16() pour IX, puis IY. `SP = 0x4000 + (n16() & 0x3FFF)`. `PC = 0x8000 + (n16() & 0xFF)`.
3. `I = n8()`, `R = n8()`, `fl = n8()`: `IFF1 = fl & 1`, `IFF2 = (fl >> 1) & 1`, `IM = (fl >> 2) % 3`. WZ = 0.
4. Memoire: `mem(a)` pour toute adresse; puis on ECRASE les octets de code a partir de PC: main `[op]`, cb `[CB, op]`, ed `[ED, op]`, ddfd `[pre, op]`, ddcb `[pre, CB, mem(PC+2), op]` (pre = DD ou FD).
5. Bus de test: lectures de port renvoient 0xFF; les ecritures memoire/port et les lectures de port sont journalisees; aucune interruption.

## 3. Colonnes d'une ligne

`table  opcode(hex)  vec  t  [A F B C D E H L]  [A2 F2 B2 C2 D2 E2 H2 L2]  IX IY SP PC  I R  IFF1 IFF2 IM  HALTED  writes  port_out  port_in`
(octets en hexadecimal; `writes` = `addr=val,addr=val` dans l'ordre; `-` si vide; `port_out` = `port=val`; `port_in` = `port`; PC apres l'instruction; `HALTED` 1 si HALT).

## 4. Comparaison

Le test execute UNE instruction complete (`Cpu::step` sur le bus de test) puis compare TOUT, sauf:
- **WZ** (verifie plus tard par z80memptr et SingleStepTests);
- les bits 3 et 5 de F pour les opcodes `SCF` (37) et `CCF` (3F) de la table `main` (le JS utilise la version simple, le crate vise le facteur Q): comparer `F & 0xD7`.
- `t` = nombre de T-states de l'instruction (sans WAIT).
Un echec affiche: table, opcode, vec, champ different (attendu/obtenu) et l'etat initial complet.
