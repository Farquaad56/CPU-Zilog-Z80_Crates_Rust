# ORDER - ordre d'execution strict (genere)

D-020: UNE seule tache a la fois, dans cet ordre. Une tache n'est prete que quand la precedente est `done` (et approuvee si GATE).


## E01
  1. E01-01 Squelette du crate puce-z80
  2. E01-01a Squelette du crate puce-z80-machines
  3. E01-02 Squelette du binaire puce-z80-cli
  4. E01-03 Squelette du binaire puce-z80-web
  5. E01-04 Scripts de CI et d'hygiene
  6. E01-05 Type Pins (broches du Z80)
  7. E01-06 Type Regs (registres et drapeaux)
  8. E01-07 GATE E01 - squelette valide  [GATE]

## E02
  9. E02-01 Types de plan (Tok, Kind, Addr, Reg, Plan)
 10. E02-02 Generer et integrer les tables de plans
 11. E02-03 Tests de coherence des tables
 12. E02-04 Selection de table et prefixes (decode.rs)
 13. E02-05 Tests de decode sur les 1280 combinaisons

## E03
 14. E03-01 Structures Cpu et Engine
 15. E03-02 Broches du cycle M1 (F4/F5/F6)
 16. E03-03 Broches des cycles memoire et internes
 17. E03-04 Broches des cycles d'entree/sortie
 18. E03-05 Resolution des adresses (Addr)
 19. E03-06 Lecture et ecriture des destinations (Reg)
 20. E03-07 Boucle de tick: execution d'un plan de jetons
 21. E03-08 Jetons conditionnels (?)
 22. E03-09 WAIT: insertion de T-states d'attente
 23. E03-10 Decodage a l'execution: M1, prefixes, DDCB
 24. E03-11 Trait Bus, enum Cycle et SimpleRam
 25. E03-12 Cpu::step: instruction complete via tick
 26. E03-13 Test de duree: toutes les instructions (semantique absente)
 27. E03-14 Journal de broches par T-state (outil de debug)
 28. E03-15 GATE E03 - moteur T-state valide  [GATE]

## E04
 29. E04-01 Harnais CRC des primitives ALU
 30. E04-02 ALU 8 bits: ADD ADC SUB SBC CP
 31. E04-03 ALU 8 bits: AND XOR OR
 32. E04-04 INC8 et DEC8
 33. E04-05 Rotations et decalages (RLC..SRL)
 34. E04-06 Arithmetique 16 bits (ADD HL, ADC HL, SBC HL)
 35. E04-07 DAA
 36. E04-08 NEG, BIT et conditions
 37. E04-09 GATE E04 - ALU validee  [GATE]

## E05
 38. E05-01 Harnais de vecteurs differentiels (diff.tsv)
 39. E05-02 Dispatch sem et helpers de registres
 40. E05-03 Misc: NOP DI EI HALT EX AF EXX EX DE,HL
 41. E05-04 LD 8 bits registre/registre/immediat/(HL)
 42. E05-05 LD indirects A,(BC) A,(DE) (nn) et WZ
 43. E05-06 LD 16 bits
 44. E05-07 INC/DEC 8 bits (registres et (HL))
 45. E05-08 INC/DEC 16 bits et ADD HL,rp
 46. E05-09 ALU 8 bits: registres, (HL), immediat
 47. E05-10 Accumulateur: RLCA RRCA RLA RRA DAA CPL SCF CCF
 48. E05-11 PUSH et POP
 49. E05-12 JP, JP cc, JP (HL)
 50. E05-13 JR, JR cc, DJNZ
 51. E05-14 CALL, CALL cc, RET, RET cc, RST
 52. E05-15 EX (SP),HL / IN A,(n) / OUT (n),A
 53. E05-16 Table main complete: tous les vecteurs
 54. E05-17 GATE E05 - table principale validee  [GATE]

## E06
 55. E06-01 CB: rotations et decalages
 56. E06-02 CB: BIT
 57. E06-03 CB: RES et SET
 58. E06-04 Table CB complete
 59. E06-05 GATE E06 - table CB validee  [GATE]

## E07
 60. E07-01 ED: IN r,(C) / OUT (C),r
 61. E07-02 ED: ADC HL,rp / SBC HL,rp
 62. E07-03 ED: LD (nn),rp / LD rp,(nn)
 63. E07-04 ED: NEG RETN/RETI IM LD I,A LD A,I RRD RLD NOP
 64. E07-05 ED: instructions de bloc LDI LDD CPI CPD (+ R)
 65. E07-06 ED: blocs d'E/S INI IND OUTI OUTD (+ R)
 66. E07-07 Table ED complete
 67. E07-08 GATE E07 - table ED validee  [GATE]

## E08
 68. E08-01 DD/FD: dispatch et prefixe ignore
 69. E08-02 IX/IY: chargements 16 bits, pile, sauts
 70. E08-03 IX/IY: ADD IX,rp
 71. E08-04 IX/IY: LD r,(IX+d) / LD (IX+d),r / LD (IX+d),n
 72. E08-05 IX/IY: ALU, INC et DEC sur (IX+d)
 73. E08-06 IXH/IXL: registres non documentes
 74. E08-07 DDCB/FDCB: BIT RES SET et rotations sur (IX+d)
 75. E08-08 DDCB/FDCB non documentes: resultat copie dans un registre
 76. E08-09 Tables DD/FD et DDCB completes
 77. E08-10 Tous les vecteurs differentiels
 78. E08-11 GATE E08 - jeu d'instructions complet  [GATE]

## E09
 79. E09-01 RESET (broche et methode)
 80. E09-02 HALT: cycles NOP et broche HALT
 81. E09-03 Cadre d'acceptation des interruptions
 82. E09-04 NMI
 83. E09-05 INT modes 1 et 2 (cycle d'acquittement)
 84. E09-06 INT mode 0 (instruction sur le bus)
 85. E09-07 BUSRQ / BUSAK
 86. E09-08 Integration Bus: irq(), nmi() et RETI visibles
 87. E09-09 GATE E09 - interruptions validees  [GATE]

## E10
 88. E10-01 Audit MEMPTR (WZ) de toutes les familles
 89. E10-02 Drapeaux X/Y des blocs repetes et DDCB
 90. E10-03 SCF/CCF: facteur Q
 91. E10-04 NMOS: LD A,I / LD A,R et interruption
 92. E10-05 Conformite des comportements non documentes (spec 23)
 93. E10-06 GATE E10 - comportements non documentes  [GATE]

## E11
 94. E11-01 Desassembleur: structure et formatage des operandes
 95. E11-02 Desassembleur: table principale et CB
 96. E11-03 Desassembleur: ED
 97. E11-04 Desassembleur: DD/FD et DDCB
 98. E11-05 Desassembleur: longueur coherente avec les plans

## E12
 99. E12-01 Feature inspect: journal de la derniere operation ALU
100. E12-02 Feature inspect: historique PC, journal d'ecritures, compteurs
101. E12-03 Feature inspect: dernier cycle de bus
102. E12-04 GATE E12 - crate complet (CPU, desassembleur, inspection)  [GATE]

## E13
103. E13-01 Telechargement des ROMs de test (fetch_roms)
104. E13-02 Machine CP/M (BDOS emule)
105. E13-03 PRELIM: premiere ROM de test
106. E13-04 ZEXDOC (67 tests)
107. E13-05 ZEXALL (drapeaux X/Y inclus) et decision SCF/CCF
108. E13-06 SingleStepTests/z80: decouvrir et documenter le format
109. E13-07 SingleStepTests: chargeur JSON et mise en etat
110. E13-08 SingleStepTests: comparaison de l'etat final
111. E13-09 SingleStepTests: etat final sur toutes les instructions
112. E13-10 SingleStepTests: cycles de bus T par T (calage)
113. E13-11 SingleStepTests: passe complete (etat + cycles)
114. E13-12 FUSE: decouvrir et documenter le format
115. E13-13 FUSE: lecteur et execution
116. E13-14 FUSE: suite complete
117. E13-15 z80test (Patrik Rak): decouvrir et documenter
118. E13-16 Machine ZX Spectrum minimale
119. E13-17 z80test: executer les 6 tests
120. E13-18 ROMs de timing et d'interruptions: decouvrir et documenter
121. E13-19 ROMs de timing: execution
122. E13-20 Performance: objectif 15 MHz equivalent en release
123. E13-21 Baseline et resume global des suites
124. E13-22 GATE E13 - toutes les ROMs de test validees  [GATE]

## E14
125. E14-01 Session: commandes et boucle de simulation
126. E14-02 Session: instantane d'etat (Snapshot) conforme au protocole
127. E14-03 Serveur HTTP: page et sante
128. E14-04 Serveur WebSocket (/ws)
129. E14-05 Commandes: roms, load, upload
130. E14-06 Commandes: run, pause, step, reset, speed, breakpoints, view
131. E14-07 Test bout en bout du serveur (client Rust)
132. E14-08 Binaire z80-web: demarrage, arret propre
133. E14-09 GATE E14 - serveur valide  [GATE]

## E15
134. E15-01 Page cliente: verification de conformite au protocole
135. E15-02 E2E: la page recoit les instantanes du crate (Playwright optionnel)
136. E15-03 E2E: ZEXDOC depuis la page
137. E15-04 GATE E15 - frontend connecte, controle visuel  [GATE]

## E16
138. E16-01 Exemple: hote minimal a broches (tick)
139. E16-02 Exemple: machine ZX-like avec contention
140. E16-03 Exemple: bus partage (BUSRQ/BUSAK) et RESET
141. E16-04 Exemple: IM2, vecteur et daisy chain (CTC factice)
142. E16-05 Plusieurs CPU, determinisme et no_std
143. E16-06 Documentation du crate (rustdoc + README)
144. E16-07 GATE E16 - reutilisation validee  [GATE]

## E17
145. E17-01 CI de release et changelog
146. E17-02 GATE FINAL - v0.1.0  [GATE]
147. E17-03 DIFFEREE: variante CMOS
148. E17-04 DIFFEREE: serialisation de l'etat (save states)
149. E17-05 DIFFEREE: cible WASM du moteur (demo navigateur autonome)
