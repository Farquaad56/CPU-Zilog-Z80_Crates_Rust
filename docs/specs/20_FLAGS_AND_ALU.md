# 20 - FLAGS ET ALU (reference executable)

Le fichier `docs/ref/z80_ref.js` est un coeur Z80 JavaScript qui **passe ZEXDOC et ZEXALL (67/67 chacun)**. Il sert de reference de SEMANTIQUE
(valeurs de registres et de drapeaux). Il n'est PAS une reference de timing ni de broches (le moteur Rust est cycle par cycle, le JS execute l'instruction d'un bloc).
Hermes porte la logique en Rust (entiers uniquement: u8/u16/u32, `wrapping_*`, pas de f32/f64).

Bits de F: `S=0x80 Z=0x40 Y=0x20 H=0x10 X=0x08 P/V=0x04 N=0x02 C=0x01`. `PAR[v]` = 0x04 si le nombre de bits a 1 de `v` est pair, sinon 0.
Registres du JS: `r[0..7]` = B C D E H L F A; `r2` = jeu alternatif; `this.wz` = MEMPTR; `this.hl()/setHl()` = HL ou IX/IY selon le prefixe.
Les champs `this.aOp, aA, aB, aC, aR, aF, aW` = journal "derniere operation ALU" (feature `inspect`, voir E12); ignorer dans le calcul.

## ALU 8 bits (ADD ADC SUB SBC AND XOR OR CP)

```js
  alu(y,v){
    const r=this.r,a=r[7],f=r[6];let res,nf;
    this.aOp=ALUN[y];this.aW=8;this.aA=a;this.aB=v;this.aC=(y===1||y===3)?(f&1):0;this.aF=f;
    switch(y){
      case 0:case 1:{const c=y&&(f&1);const t=a+v+c;res=t&255;nf=(res&0xA8)|(res?0:FZ)|((a^v^res)&FH)|(((a^res)&(v^res)&0x80)?FP:0)|(t>>8);break;}
      case 2:case 3:case 7:{const c=y===3?(f&1):0;const t=a-v-c;res=t&255;const xy=y===7?v:res;nf=(xy&0x28)|(res&0x80)|(res?0:FZ)|((a^v^res)&FH)|(((a^v)&(a^res)&0x80)?FP:0)|FN|((t>>8)&1);break;}
      case 4:res=a&v;nf=(res&0xA8)|(res?0:FZ)|FH|PAR[res];break;
      case 5:res=a^v;nf=(res&0xA8)|(res?0:FZ)|PAR[res];break;
      default:res=a|v;nf=(res&0xA8)|(res?0:FZ)|PAR[res];
    }
    this.aR=res;r[6]=nf;if(y!==7)r[7]=res;
  }
```

## INC et DEC 8 bits

```js
  inc8(v){const f=this.r[6];const res=(v+1)&255;this.aOp='INC';this.aW=8;this.aA=v;this.aB=1;this.aC=0;this.aR=res;this.aF=f;
    this.r[6]=(f&FC)|(res&0xA8)|(res?0:FZ)|((res&15)===0?FH:0)|(res===0x80?FP:0);return res;}

  dec8(v){const f=this.r[6];const res=(v-1)&255;this.aOp='DEC';this.aW=8;this.aA=v;this.aB=1;this.aC=0;this.aR=res;this.aF=f;
    this.r[6]=(f&FC)|(res&0xA8)|(res?0:FZ)|((res&15)===15?FH:0)|(res===0x7F?FP:0)|FN;return res;}
```

## Rotations et decalages (RLC RRC RL RR SLA SRA SLL SRL)

```js
  rot(y,v){
    let c,res;
    switch(y){
      case 0:c=v>>7;res=((v<<1)|c)&255;break;
      case 1:c=v&1;res=(v>>1)|(c<<7);break;
      case 2:c=v>>7;res=((v<<1)|(this.r[6]&1))&255;break;
      case 3:c=v&1;res=(v>>1)|((this.r[6]&1)<<7);break;
      case 4:c=v>>7;res=(v<<1)&255;break;
      case 5:c=v&1;res=(v>>1)|(v&0x80);break;
      case 6:c=v>>7;res=((v<<1)|1)&255;break;
      default:c=v&1;res=v>>1;
    }
    this.aOp=ROTN[y];this.aW=8;this.aA=v;this.aB=0;this.aC=this.r[6]&1;this.aR=res;this.aF=this.r[6];
    this.r[6]=(res&0xA8)|(res?0:FZ)|PAR[res]|c;return res;
  }
```

## Arithmetique 16 bits (ADD HL, ADC HL, SBC HL)

```js
  add16(a,b){const t=a+b;const res=t&0xFFFF;const f=this.r[6];this.aOp='ADD16';this.aW=16;this.aA=a;this.aB=b;this.aC=0;this.aR=res;this.aF=f;
    this.r[6]=(f&(FS|FZ|FP))|((res>>8)&(FY|FX))|(((a^b^res)>>8)&FH)|((t>>16)&1);this.wz=(a+1)&0xFFFF;return res;}

  adc16(a,b){const c=this.r[6]&1;const t=a+b+c;const res=t&0xFFFF;this.aOp='ADC16';this.aW=16;this.aA=a;this.aB=b;this.aC=c;this.aR=res;this.aF=this.r[6];
    this.r[6]=((res>>8)&(FS|FY|FX))|(res?0:FZ)|(((a^b^res)>>8)&FH)|((~(a^b)&(a^res)&0x8000)?FP:0)|((t>>16)&1);this.wz=(a+1)&0xFFFF;return res;}

  sbc16(a,b){const c=this.r[6]&1;const t=a-b-c;const res=t&0xFFFF;this.aOp='SBC16';this.aW=16;this.aA=a;this.aB=b;this.aC=c;this.aR=res;this.aF=this.r[6];
    this.r[6]=((res>>8)&(FS|FY|FX))|(res?0:FZ)|(((a^b^res)>>8)&FH)|(((a^b)&(a^res)&0x8000)?FP:0)|FN|((t>>16)&1);this.wz=(a+1)&0xFFFF;return res;}
```

## DAA

```js
  daa(){
    const r=this.r,A=r[7],f=r[6];let add=0,c=0;
    if((f&FH)||(A&15)>9)add|=6;
    if((f&FC)||A>0x99){add|=0x60;c=1;}
    const res=(f&FN)?(A-add)&255:(A+add)&255;
    const h=(f&FN)?(((f&FH)&&(A&15)<6)?FH:0):((A&15)>9?FH:0);
    this.aOp='DAA';this.aW=8;this.aA=A;this.aB=add;this.aC=0;this.aR=res;this.aF=f;
    r[7]=res;r[6]=(res&0xA8)|(res?0:FZ)|PAR[res]|(f&FN)|h|c;
  }
```

## Operations CB (rotations, BIT, RES, SET)

```js
  execCB(){
    const r=this.r;let a=0,op,val;
    if(this.ixy){
      const d=this.imm8();const sd=d<128?d:d-256;
      a=((this.ixy===1?this.ix:this.iy)+sd)&0xFFFF;this.wz=a;
      op=this.imm8();val=this.rd(a);
    }else{op=this.fetch();val=(op&7)===6?this.rd(this.HL):r[op&7];}
    const x=op>>6,y=(op>>3)&7,z=op&7;
    if(x===1){
      const bit=val&(1<<y);const f=r[6];
      const xy=this.ixy?(a>>8):(z===6?(this.wz>>8):val);
      this.aOp='BIT';this.aW=8;this.aA=val;this.aB=1<<y;this.aC=0;this.aR=bit;this.aF=f;
      r[6]=(f&FC)|FH|(xy&0x28)|(bit?0:(FZ|FP))|((y===7&&bit)?FS:0);
      return this.ixy?16:(z===6?12:8);
    }
    let res;
    if(x===0)res=this.rot(y,val);
    else if(x===2)res=val&~(1<<y);
    else res=val|(1<<y);
    if(x!==0){this.aOp=x===2?'RES':'SET';this.aW=8;this.aA=val;this.aB=1<<y;this.aC=0;this.aR=res;this.aF=r[6];}
    if(this.ixy){this.wr(a,res);if(z!==6)r[z]=res;return 19;}
    if(z===6){this.wr(this.HL,res);return 15;}
    r[z]=res;return 8;
  }
```

## Operations ED (IN OUT NEG RETN IM LD I/R RRD RLD ADC/SBC LD (nn))

```js
  execED(){
    const r=this.r;const op=this.fetch();
    const x=op>>6,y=(op>>3)&7,z=op&7,p=y>>1,q=y&1;
    if(x===1){
      switch(z){
      case 0:{const v=this.ioIn(this.BC);const f=r[6];r[6]=(f&FC)|(v&0xA8)|(v?0:FZ)|PAR[v];if(y!==6)r[y]=v;this.wz=(this.BC+1)&0xFFFF;return 12;}
      case 1:this.ioOut(this.BC,y===6?0:r[y]);this.wz=(this.BC+1)&0xFFFF;return 12;
      case 2:{const h=this.HL;const v=this.rp(p);const res=q?this.adc16(h,v):this.sbc16(h,v);this.setHl(res);return 15;}
      case 3:{const a=this.imm16();
        if(q===0){const v=this.rp(p);this.wr(a,v&255);this.wr(a+1,v>>8);}
        else this.setRp(p,this.rd(a)|(this.rd(a+1)<<8));
        this.wz=(a+1)&0xFFFF;return 20;}
      case 4:{const A=r[7];this.aOp='NEG';this.aW=8;this.aA=0;this.aB=A;this.aC=0;this.aF=r[6];
        const res=(-A)&255;this.aR=res;
        r[6]=(res&0xA8)|(res?0:FZ)|((0^A^res)&FH)|(A===0x80?FP:0)|FN|(A?FC:0);r[7]=res;return 8;}
      case 5:this.pc=this.pop();this.wz=this.pc;this.iff1=this.iff2;return 14;
      case 6:this.im=[0,0,1,2,0,0,1,2][y];return 8;
      default:
        switch(y){
          case 0:this.i=r[7];return 9;
          case 1:this.R=r[7];return 9;
          case 2:case 3:{const v=y===2?this.i:this.R;r[7]=v;r[6]=(r[6]&FC)|(v&0xA8)|(v?0:FZ)|(this.iff2?FP:0);return 9;}
          case 4:{const m=this.rd(this.HL);const A=r[7];this.wr(this.HL,((A<<4)|(m>>4))&255);const n=(A&0xF0)|(m&15);r[7]=n;
            this.aOp='RRD';this.aW=8;this.aA=A;this.aB=m;this.aC=0;this.aR=n;this.aF=r[6];
            r[6]=(r[6]&FC)|(n&0xA8)|(n?0:FZ)|PAR[n];this.wz=(this.HL+1)&0xFFFF;return 18;}
          case 5:{const m=this.rd(this.HL);const A=r[7];this.wr(this.HL,((m<<4)|(A&15))&255);const n=(A&0xF0)|(m>>4);r[7]=n;
            this.aOp='RLD';this.aW=8;this.aA=A;this.aB=m;this.aC=0;this.aR=n;this.aF=r[6];
            r[6]=(r[6]&FC)|(n&0xA8)|(n?0:FZ)|PAR[n];this.wz=(this.HL+1)&0xFFFF;return 18;}
          default:return 8;
        }
      }
    }
    if(x===2&&z<4&&y>=4)return this.block(y,z);
    return 8;
  }
```

## Instructions de bloc

```js
  block(y,z){
    const r=this.r;const dec=(y&1)===1,rep=y>=6;
    const inc=dec?-1:1;let t=16;
    const hl=this.HL;
    if(z===0){
      const v=this.rd(hl);this.wr(this.DE,v);
      this.setHl2(hl+inc);this.setDE(this.DE+inc);this.setBC(this.BC-1);
      const n=(v+r[7])&255;
      r[6]=(r[6]&(FS|FZ|FC))|(n&8)|((n&2)<<4)|(this.BC?FP:0);
      this.aOp=dec?'LDD':'LDI';this.aW=8;this.aA=r[7];this.aB=v;this.aC=0;this.aR=n;this.aF=r[6];
      if(rep&&this.BC){this.pc=(this.pc-2)&0xFFFF;this.wz=(this.pc+1)&0xFFFF;t=21;}
    }else if(z===1){
      const v=this.rd(hl);const A=r[7];const res=(A-v)&255;const h=(A^v^res)&FH;
      this.setHl2(hl+inc);this.setBC(this.BC-1);
      const n=(res-(h?1:0))&255;
      r[6]=(r[6]&FC)|FN|(res&0x80)|(res?0:FZ)|h|(n&8)|((n&2)<<4)|(this.BC?FP:0);
      this.aOp=dec?'CPD':'CPI';this.aW=8;this.aA=A;this.aB=v;this.aC=0;this.aR=res;this.aF=r[6];
      this.wz=(this.wz+inc)&0xFFFF;
      if(rep&&this.BC&&res){this.pc=(this.pc-2)&0xFFFF;this.wz=(this.pc+1)&0xFFFF;t=21;}
    }else if(z===2){
      const v=this.ioIn(this.BC);this.wr(hl,v);
      r[0]=(r[0]-1)&255;this.setHl2(hl+inc);
      const k=v+((r[1]+inc)&255);
      r[6]=(r[0]&0xA8)|(r[0]?0:FZ)|((v&0x80)?FN:0)|(k>255?(FH|FC):0)|PAR[(k&7)^r[0]];
      if(rep&&r[0]){this.pc=(this.pc-2)&0xFFFF;t=21;}
    }else{
      const v=this.rd(hl);r[0]=(r[0]-1)&255;this.ioOut(this.BC,v);this.setHl2(hl+inc);
      const k=v+r[5];
      r[6]=(r[0]&0xA8)|(r[0]?0:FZ)|((v&0x80)?FN:0)|(k>255?(FH|FC):0)|PAR[(k&7)^r[0]];
      if(rep&&r[0]){this.pc=(this.pc-2)&0xFFFF;t=21;}
    }
    return t;
  }
```

## Instructions principales (toutes familles)

```js
  execMain(op){
    const r=this.r,x=op>>6,y=(op>>3)&7,z=op&7,p=y>>1,q=y&1;
    let t=4;
    switch(x){
    case 0:
      switch(z){
      case 0:
        switch(y){
          case 0:break;
          case 1:{let k=r[7];r[7]=this.r2[7];this.r2[7]=k;k=r[6];r[6]=this.r2[6];this.r2[6]=k;break;}
          case 2:{const d=this.imm8();r[0]=(r[0]-1)&255;if(r[0]){this.pc=(this.pc+(d<128?d:d-256))&0xFFFF;this.wz=this.pc;t=13;}else t=8;break;}
          case 3:{const d=this.imm8();this.pc=(this.pc+(d<128?d:d-256))&0xFFFF;this.wz=this.pc;t=12;break;}
          default:{const d=this.imm8();if(this.cond(y-4)){this.pc=(this.pc+(d<128?d:d-256))&0xFFFF;this.wz=this.pc;t=12;}else t=7;}
        }
        break;
      case 1:
        if(q===0){this.setRp(p,this.imm16());t=10;}
        else{this.setHl(this.add16(this.hl(),this.rp(p)));t=11;}
        break;
      case 2:{
        switch(y){
          case 0:this.wr(this.BC,r[7]);this.wz=((this.BC+1)&255)|(r[7]<<8);t=7;break;
          case 1:r[7]=this.rd(this.BC);this.wz=(this.BC+1)&0xFFFF;t=7;break;
          case 2:this.wr(this.DE,r[7]);this.wz=((this.DE+1)&255)|(r[7]<<8);t=7;break;
          case 3:r[7]=this.rd(this.DE);this.wz=(this.DE+1)&0xFFFF;t=7;break;
          case 4:{const a=this.imm16();const h=this.hl();this.wr(a,h&255);this.wr(a+1,h>>8);this.wz=(a+1)&0xFFFF;t=16;break;}
          case 5:{const a=this.imm16();this.setHl(this.rd(a)|(this.rd(a+1)<<8));this.wz=(a+1)&0xFFFF;t=16;break;}
          case 6:{const a=this.imm16();this.wr(a,r[7]);this.wz=((a+1)&255)|(r[7]<<8);t=13;break;}
          default:{const a=this.imm16();r[7]=this.rd(a);this.wz=(a+1)&0xFFFF;t=13;}
        }
        break;}
      case 3:
        this.setRp(p,q?this.rp(p)-1:this.rp(p)+1);t=6;break;
      case 4:
        if(y===6){const a=this.ea();this.wr(a,this.inc8(this.rd(a)));t=11;}
        else{this.s8(y,this.inc8(this.g8(y)));}
        break;
      case 5:
        if(y===6){const a=this.ea();this.wr(a,this.dec8(this.rd(a)));t=11;}
        else{this.s8(y,this.dec8(this.g8(y)));}
        break;
      case 6:
        if(y===6){const a=this.ea();const n=this.imm8();this.wr(a,n);t=10;if(this.ixy)this.tx-=3;}
        else{this.s8(y,this.imm8());t=7;}
        break;
      default:{
        const f=r[6];let A=r[7],c;
        switch(y){
          case 0:c=A>>7;A=((A<<1)|c)&255;r[7]=A;r[6]=(f&(FS|FZ|FP))|(A&0x28)|c;this.aOp='RLCA';break;
          case 1:c=A&1;A=(A>>1)|(c<<7);r[7]=A;r[6]=(f&(FS|FZ|FP))|(A&0x28)|c;this.aOp='RRCA';break;
          case 2:c=A>>7;A=((A<<1)|(f&1))&255;r[7]=A;r[6]=(f&(FS|FZ|FP))|(A&0x28)|c;this.aOp='RLA';break;
          case 3:c=A&1;A=(A>>1)|((f&1)<<7);r[7]=A;r[6]=(f&(FS|FZ|FP))|(A&0x28)|c;this.aOp='RRA';break;
          case 4:this.daa();break;
          case 5:A^=255;r[7]=A;r[6]=(f&(FS|FZ|FP|FC))|FH|FN|(A&0x28);this.aOp='CPL';break;
          case 6:r[6]=(f&(FS|FZ|FP))|FC|(A&0x28);this.aOp='SCF';break;
          default:r[6]=(f&(FS|FZ|FP))|((f&FC)?FH:FC)|(A&0x28);this.aOp='CCF';
        }
        if(y<4||y>3&&y!==4){this.aW=8;this.aA=A;this.aB=0;this.aC=f&1;this.aR=r[7];this.aF=f;}
      }
      }
      break;
    case 1:
      if(op===0x76){this.halted=true;this.pc=(this.pc-1)&0xFFFF;break;}
      if(y===6){const a=this.ea();this.wr(a,r[z]);t=7;}
      else if(z===6){const a=this.ea();r[y]=this.rd(a);t=7;}
      else this.s8(y,this.g8(z));
      break;
    case 2:
      if(z===6){const a=this.ea();this.alu(y,this.rd(a));t=7;}
      else this.alu(y,this.g8(z));
      break;
    default:
      switch(z){
      case 0:
        if(this.cond(y)){this.pc=this.pop();this.wz=this.pc;t=11;}else t=5;break;
      case 1:
        if(q===0){this.setRp2(p,this.pop());t=10;}
        else switch(p){
          case 0:this.pc=this.pop();this.wz=this.pc;t=10;break;
          case 1:for(let k=0;k<6;k++){const v=r[k];r[k]=this.r2[k];this.r2[k]=v;}break;
          case 2:this.pc=this.hl();break;
          default:this.sp=this.hl();t=6;
        }
        break;
      case 2:{const a=this.imm16();this.wz=a;if(this.cond(y))this.pc=a;t=10;break;}
      case 3:
        switch(y){
          case 0:this.pc=this.imm16();this.wz=this.pc;t=10;break;
          case 2:{const n=this.imm8();const port=(r[7]<<8)|n;this.ioOut(port,r[7]);this.wz=((n+1)&255)|(r[7]<<8);t=11;break;}
          case 3:{const n=this.imm8();const port=(r[7]<<8)|n;r[7]=this.ioIn(port);this.wz=(port+1)&0xFFFF;t=11;break;}
          case 4:{const lo=this.rd(this.sp),hi=this.rd(this.sp+1);const h=this.hl();this.wr(this.sp+1,h>>8);this.wr(this.sp,h&255);this.setHl(lo|(hi<<8));this.wz=lo|(hi<<8);t=19;break;}
          case 5:{let k=r[2];r[2]=r[4];r[4]=k;k=r[3];r[3]=r[5];r[5]=k;break;}
          case 6:this.iff1=this.iff2=0;break;
          default:this.iff1=this.iff2=1;
        }
        break;
      case 4:{const a=this.imm16();this.wz=a;if(this.cond(y)){this.push(this.pc);this.pc=a;t=17;}else t=10;break;}
      case 5:
        if(q===0){this.push(this.rp2(p));t=11;}
        else{const a=this.imm16();this.push(this.pc);this.pc=a;this.wz=a;t=17;}
        break;
      case 6:this.alu(y,this.imm8());t=7;break;
      default:this.push(this.pc);this.pc=y*8;this.wz=this.pc;t=11;
      }
    }
    return t;
  }
```
