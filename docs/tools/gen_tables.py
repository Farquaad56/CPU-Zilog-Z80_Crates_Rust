#!/usr/bin/env python
"""gen_tables.py - writes docs/specs/data/{main,cb,ed,ddfd,ddcb}.tsv : one row per opcode.
Columns: opcode, mnemonic, family, plan, t_min, t_max   (plan grammar: docs/specs/12_PLAN_GRAMMAR.md)
Timings were cross-checked against a reference core that passes ZEXDOC/ZEXALL (docs/tools/verify_timing.js).
"""
import re, sys
from pathlib import Path
OUT = Path(__file__).resolve().parents[1] / "specs" / "data"
R8 = ['B','C','D','E','H','L','(HL)','A']
RP = ['BC','DE','HL','SP']; RP2 = ['BC','DE','HL','AF']
CC = ['NZ','Z','NC','C','PO','PE','P','M']
ALU = ['ADD A,','ADC A,','SUB ','SBC A,','AND ','XOR ','OR ','CP ']
ROT = ['RLC','RRC','RL','RR','SLA','SRA','SLL','SRL']
ACC = ['RLCA','RRCA','RLA','RRA','DAA','CPL','SCF','CCF']
RSRC = ['B','C','D','E','H','L',None,'A']

def tsum(plan):
    mn = mx = 0
    for tok in plan.split():
        m = re.match(r'^(\?)?[FRWINO](\d+)', tok)
        if m:
            n = int(m.group(2))
            mx += n
            if not m.group(1): mn += n
    return mn, mx

def main_row(op):
    x, y, z = op >> 6, (op >> 3) & 7, op & 7
    p, q = y >> 1, y & 1
    if x == 0:
        if z == 0:
            if y == 0: return 'NOP', 'nop', 'F4'
            if y == 1: return "EX AF,AF'", 'ex_af', 'F4 X0'
            if y == 2: return 'DJNZ e', 'djnz', 'F5 R3:PC+>L X0 ?I5 X1'
            if y == 3: return 'JR e', 'jr', 'F4 R3:PC+>L X0 I5 X1'
            return 'JR %s,e' % CC[y-4], 'jr_cc', 'F4 R3:PC+>L X0 ?I5 X1'
        if z == 1:
            if q == 0: return 'LD %s,nn' % RP[p], 'ld_rp_nn', 'F4 R3:PC+>Z R3:PC+>W X0'
            return 'ADD HL,%s' % RP[p], 'add_hl_rp', 'F4 X0 I4 I3'
        if z == 2:
            return [('LD (BC),A','ld_bc_a','F4 X0 W3:BC<L'),('LD A,(BC)','ld_a_bc','F4 R3:BC>L X0'),
                    ('LD (DE),A','ld_de_a','F4 X0 W3:DE<L'),('LD A,(DE)','ld_a_de','F4 R3:DE>L X0'),
                    ('LD (nn),HL','ld_nn_hl','F4 R3:PC+>Z R3:PC+>W X0 W3:NN<L W3:NN+1<H'),
                    ('LD HL,(nn)','ld_hl_nn','F4 R3:PC+>Z R3:PC+>W R3:NN>L R3:NN+1>H X0'),
                    ('LD (nn),A','ld_nn_a','F4 R3:PC+>Z R3:PC+>W X0 W3:NN<L'),
                    ('LD A,(nn)','ld_a_nn','F4 R3:PC+>Z R3:PC+>W R3:NN>L X0')][y][0:1] + \
                   ([('LD (BC),A','ld_bc_a','F4 X0 W3:BC<L'),('LD A,(BC)','ld_a_bc','F4 R3:BC>L X0'),
                    ('LD (DE),A','ld_de_a','F4 X0 W3:DE<L'),('LD A,(DE)','ld_a_de','F4 R3:DE>L X0'),
                    ('LD (nn),HL','ld_nn_hl','F4 R3:PC+>Z R3:PC+>W X0 W3:NN<L W3:NN+1<H'),
                    ('LD HL,(nn)','ld_hl_nn','F4 R3:PC+>Z R3:PC+>W R3:NN>L R3:NN+1>H X0'),
                    ('LD (nn),A','ld_nn_a','F4 R3:PC+>Z R3:PC+>W X0 W3:NN<L'),
                    ('LD A,(nn)','ld_a_nn','F4 R3:PC+>Z R3:PC+>W R3:NN>L X0')][y][1:])
        if z == 3: return '%s %s' % ('DEC' if q else 'INC', RP[p]), 'dec_rp' if q else 'inc_rp', 'F6 X0'
        if z == 4:
            if y == 6: return 'INC (HL)', 'inc_hl', 'F4 R4:HL>L X0 W3:HL<L'
            return 'INC %s' % R8[y], 'inc_r', 'F4 X0'
        if z == 5:
            if y == 6: return 'DEC (HL)', 'dec_hl', 'F4 R4:HL>L X0 W3:HL<L'
            return 'DEC %s' % R8[y], 'dec_r', 'F4 X0'
        if z == 6:
            if y == 6: return 'LD (HL),n', 'ld_hl_n', 'F4 R3:PC+>L W3:HL<L'
            return 'LD %s,n' % R8[y], 'ld_r_n', 'F4 R3:PC+>L X0'
        return ACC[y], 'acc_' + ACC[y].lower(), 'F4 X0'
    if x == 1:
        if op == 0x76: return 'HALT', 'halt', 'F4 X0'
        if y == 6: return 'LD (HL),%s' % R8[z], 'ld_hl_r', 'F4 X0 W3:HL<L'
        if z == 6: return 'LD %s,(HL)' % R8[y], 'ld_r_hl', 'F4 R3:HL>L X0'
        return 'LD %s,%s' % (R8[y], R8[z]), 'ld_r_r', 'F4 X0'
    if x == 2:
        if z == 6: return '%s(HL)' % ALU[y], 'alu_hl', 'F4 R3:HL>L X0'
        return '%s%s' % (ALU[y], R8[z]), 'alu_r', 'F4 X0'
    if z == 0: return 'RET %s' % CC[y], 'ret_cc', 'F5 X0 ?R3:SP+>Z ?R3:SP+>W X1'
    if z == 1:
        if q == 0: return 'POP %s' % RP2[p], 'pop', 'F4 R3:SP+>L R3:SP+>H X0'
        return [('RET','ret','F4 R3:SP+>Z R3:SP+>W X0'),('EXX','exx','F4 X0'),('JP (HL)','jp_hl','F4 X0'),('LD SP,HL','ld_sp_hl','F6 X0')][p]
    if z == 2: return 'JP %s,nn' % CC[y], 'jp_cc', 'F4 R3:PC+>Z R3:PC+>W X0'
    if z == 3:
        return [('JP nn','jp','F4 R3:PC+>Z R3:PC+>W X0'), None,
                ('OUT (n),A','out_n_a','F4 R3:PC+>Z X0 O4:AN<L'),('IN A,(n)','in_a_n','F4 R3:PC+>Z N4:AN>L X0'),
                ('EX (SP),HL','ex_sp_hl','F4 R3:SP>Z R4:SP+1>W X0 W3:SP+1<H W5:SP<L X1'),
                ('EX DE,HL','ex_de_hl','F4 X0'),('DI','di','F4 X0'),('EI','ei','F4 X0')][y]
    if z == 4: return 'CALL %s,nn' % CC[y], 'call_cc', 'F4 R3:PC+>Z R3:PC+>W X0 ?I1 ?W3:-SP<H ?W3:-SP<L X1'
    if z == 5:
        if q == 0: return 'PUSH %s' % RP2[p], 'push', 'F5 X0 W3:-SP<H W3:-SP<L'
        if p == 0: return 'CALL nn', 'call', 'F4 R3:PC+>Z R4:PC+>W X0 W3:-SP<H W3:-SP<L X1'
        return None
    if z == 6: return '%sn' % ALU[y], 'alu_n', 'F4 R3:PC+>L X0'
    return 'RST %02XH' % (y*8), 'rst', 'F5 X0 W3:-SP<H W3:-SP<L X1'

def uses_hl(row):
    m, fam = row[0], row[1]
    return '(HL)' in m
def uses_h_or_l(op):
    x, y, z = op >> 6, (op >> 3) & 7, op & 7
    if x == 1 and op != 0x76: return (y in (4,5) and z != 6) or (z in (4,5) and y != 6)
    if x == 2: return z in (4,5)
    if x == 0 and z in (4,5,6): return y in (4,5)
    return False

def ddfd_row(op):
    r = main_row(op)
    if op in (0xDD, 0xFD, 0xED, 0xCB): return None
    if r is None: return None
    m, fam, plan = r
    xy = lambda s: s.replace('HL', 'IX')
    if fam in ('ld_hl_n',):  return 'LD (IX+d),n', 'ix_ld_n', 'F4 F4 R3:PC+>D R5:PC+>L X0 W3:XY<L'
    if fam in ('inc_hl','dec_hl'): return xy(m).replace('(IX)','(IX+d)'), 'ix_'+fam[:3], 'F4 F4 R3:PC+>D I5 R4:XY>L X0 W3:XY<L'
    if fam == 'ld_r_hl': return xy(m).replace('(IX)','(IX+d)'), 'ix_ld_r', 'F4 F4 R3:PC+>D I5 R3:XY>L X0'
    if fam == 'ld_hl_r': return xy(m).replace('(IX)','(IX+d)'), 'ix_ld_m', 'F4 F4 R3:PC+>D I5 X0 W3:XY<L'
    if fam == 'alu_hl': return xy(m).replace('(IX)','(IX+d)'), 'ix_alu', 'F4 F4 R3:PC+>D I5 R3:XY>L X0'
    if 'HL' in m or fam in ('ld_sp_hl',) or uses_h_or_l(op):
        mm = m
        if uses_h_or_l(op): mm = re.sub(r'\bH\b','IXH',re.sub(r'\bL\b','IXL',m))
        mm = mm.replace('HL','IX')
        return mm, 'ix_'+fam, 'F4 ' + plan
    return '(prefix ignored) ' + m, 'ix_plain', 'F4 ' + plan

def cb_row(op):
    x, y, z = op >> 6, (op >> 3) & 7, op & 7
    t = R8[z]
    if x == 0: name = '%s %s' % (ROT[y], t); fam = 'cb_rot'
    else: name = '%s %d,%s' % (['','BIT','RES','SET'][x], y, t); fam = 'cb_' + ['','bit','res','set'][x]
    if z == 6:
        if x == 1: return name, fam, 'F4 F4 R4:HL>L X0'
        return name, fam, 'F4 F4 R4:HL>L X0 W3:HL<L'
    return name, fam, 'F4 F4 X0'

def ddcb_row(op):
    x, y, z = op >> 6, (op >> 3) & 7, op & 7
    if x == 0: name = '%s (IX+d)' % ROT[y]; fam = 'ddcb_rot'
    else: name = '%s %d,(IX+d)' % (['','BIT','RES','SET'][x], y); fam = 'ddcb_' + ['','bit','res','set'][x]
    if z != 6 and x != 1: name += ',' + R8[z]; fam += '_undoc'
    plan = 'F4 F4 R3:PC+>D R5:PC+>OP R4:XY>L X0' + ('' if x == 1 else ' W3:XY<L')
    return name, fam, plan

def ed_row(op):
    x, y, z = op >> 6, (op >> 3) & 7, op & 7
    p, q = y >> 1, y & 1
    if x == 1:
        if z == 0: return ('IN (C)' if y == 6 else 'IN %s,(C)' % R8[y]), 'ed_in_c', 'F4 F4 N4:BC>L X0'
        if z == 1: return ('OUT (C),0' if y == 6 else 'OUT (C),%s' % R8[y]), 'ed_out_c', 'F4 F4 X0 O4:BC<L'
        if z == 2: return '%s HL,%s' % ('ADC' if q else 'SBC', RP[p]), 'ed_adc16' if q else 'ed_sbc16', 'F4 F4 X0 I4 I3'
        if z == 3:
            if q == 0: return 'LD (nn),%s' % RP[p], 'ed_ld_nn_rp', 'F4 F4 R3:PC+>Z R3:PC+>W X0 W3:NN<L W3:NN+1<H'
            return 'LD %s,(nn)' % RP[p], 'ed_ld_rp_nn', 'F4 F4 R3:PC+>Z R3:PC+>W R3:NN>L R3:NN+1>H X0'
        if z == 4: return 'NEG', 'ed_neg', 'F4 F4 X0'
        if z == 5: return ('RETI' if y == 1 else 'RETN'), 'ed_retn', 'F4 F4 R3:SP+>Z R3:SP+>W X0'
        if z == 6: return 'IM %d' % [0,0,1,2,0,0,1,2][y], 'ed_im', 'F4 F4 X0'
        if y in (0,1,2,3): return ['LD I,A','LD R,A','LD A,I','LD A,R'][y], 'ed_ld_ir', 'F4 F5 X0'
        if y == 4: return 'RRD', 'ed_rrd', 'F4 F4 R3:HL>L I4 X0 W3:HL<L'
        if y == 5: return 'RLD', 'ed_rld', 'F4 F4 R3:HL>L I4 X0 W3:HL<L'
        return 'NOP*', 'ed_nop', 'F4 F4'
    if x == 2 and z < 4 and y >= 4:
        nm = [['LDI','CPI','INI','OUTI'],['LDD','CPD','IND','OUTD'],['LDIR','CPIR','INIR','OTIR'],['LDDR','CPDR','INDR','OTDR']][y-4][z]
        rep = y >= 6
        if z == 0: plan = 'F4 F4 R3:HL>L X0 W5:DE<L X1' + (' ?I5' if rep else '')
        elif z == 1: plan = 'F4 F4 R3:HL>L X0 I5' + (' ?I5' if rep else '')
        elif z == 2: plan = 'F4 F5 N4:BC>L X0 W3:HL<L X1' + (' ?I5' if rep else '')
        else: plan = 'F4 F5 R3:HL>L X0 O4:BC<L X1' + (' ?I5' if rep else '')
        return nm, 'ed_block_' + ['ld','cp','in','out'][z] + ('r' if rep else ''), plan
    return 'NOP*', 'ed_nop', 'F4 F4'

def write(name, fn, rows_hdr='opcode\tmnemonic\tfamily\tplan\tt_min\tt_max'):
    lines = [rows_hdr]
    for op in range(256):
        r = fn(op)
        if r is None:
            lines.append('%02X\t-\t-\t-\t0\t0' % op); continue
        mn, mx = tsum(r[2])
        lines.append('%02X\t%s\t%s\t%s\t%d\t%d' % (op, r[0], r[1], r[2], mn, mx))
    (OUT / name).write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')

if __name__ == '__main__':
    OUT.mkdir(parents=True, exist_ok=True)
    write('main.tsv', lambda o: main_row(o) if o not in (0xCB,0xDD,0xED,0xFD) else ('prefix','prefix','F4'))
    write('cb.tsv', cb_row); write('ed.tsv', ed_row); write('ddfd.tsv', ddfd_row); write('ddcb.tsv', ddcb_row)
    print('tables written to', OUT)
