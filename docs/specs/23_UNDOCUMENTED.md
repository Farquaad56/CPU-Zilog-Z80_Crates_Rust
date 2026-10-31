# 23 - COMPORTEMENTS NON DOCUMENTES (a reproduire exactement)

Tous valides par ZEXALL, z80test (Rak), FUSE, SingleStepTests.

1. **Bits 3 et 5 du registre F (X et Y)**: copies des bits 3 et 5 du resultat pour la plupart des operations (voir `20_FLAGS_AND_ALU.md`); pour CP ils viennent de l'OPERANDE; pour BIT n,(HL) de WZ haut; pour BIT n,(IX+d) de l'adresse haute.
2. **WZ (MEMPTR)**: mis a jour par JP/CALL/RET/LD (nn)/LD A,(BC|DE)/ADD HL/ADC/SBC/IN/OUT/blocs... (voir tables `sem` par famille; reference: docs/ref/z80_ref.js, champ `wz`). Visible via BIT n,(HL) et les tests z80memptr.
3. **Q (facteur Q)**: valeur du dernier F ecrit par l'instruction precedente (0 si elle n'a pas modifie F). SCF/CCF: `X,Y = (A | (F ^ Q))` bits 3 et 5 (NMOS Zilog). (U) La reference JS utilise la version simple `X,Y = A`; le crate vise la version Q (z80ccf de Rak la distingue). Premier choix pour les tests ZEXALL: tester LES DEUX.
4. **Opcodes DD/FD** sur instruction sans HL: le prefixe est ignore (4 T perdus). `DD DD`, `DD FD` etc.: seul le dernier prefixe compte.
5. **IXH, IXL, IYH, IYL** accessibles par DD/FD + opcodes qui utilisent H/L.
6. **SLL** (CB 30-37) = decale a gauche, bit 0 = 1.
7. **DDCB/FDCB**: le resultat est AUSSI copie dans le registre (z != 6); BIT ignore le registre.
8. **OUT (C),0** (ED 71): ecrit 0 sur NMOS (255 sur CMOS). **IN (C)** (ED 70): met les flags sans ranger la valeur.
9. **ED xx inconnus**: NOP de 8 T.
10. **LDI/LDD/CPI/CPD**: bits 3/5 de F derives de (A+octet) ou (A-octet-H). Repetitions (LDIR...): bits 3/5 recopies de PC haut (U) sur NMOS lors du rebouclage.
11. **Etat apres RESET**: AF=SP=0xFFFF, PC=0, I=R=0, IFF=0, IM=0, autres registres indefinis (le crate les laisse a 0xFFFF? UNKNOWN - to confirm Q-001).
12. **Registre R**: bit 7 conserve par l'incrementation, ecrasable par LD R,A.
