/* Z80 core + disassembleur — décodage x/y/z/p/q — flags non documentés (X,Y) inclus */
(function (root) {
'use strict';
const FS=0x80,FZ=0x40,FY=0x20,FH=0x10,FX=0x08,FP=0x04,FN=0x02,FC=0x01;
const PAR=new Uint8Array(256);
for(let i=0;i<256;i++){let v=i,p=1;while(v){p^=v&1;v>>=1;}PAR[i]=p?FP:0;}
const ALUN=['ADD','ADC','SUB','SBC','AND','XOR','OR','CP'];
const ROTN=['RLC','RRC','RL','RR','SLA','SRA','SLL','SRL'];

class Z80 {
  constructor(){
    this.mem=new Uint8Array(65536);
    this.r=new Uint8Array(8);   // B C D E H L F A
    this.r2=new Uint8Array(8);
    this.hist=new Int32Array(32); this.hp=0;
    this.wlog=new Int32Array(16); this.wp=0;
    this.onChar=null; this.cpm=false;
    this.reset();
  }
  reset(){
    this.r.fill(0);this.r2.fill(0);this.r[7]=0xFF;this.r[6]=0xFF;
    this.ix=0xFFFF;this.iy=0xFFFF;this.sp=0xFFFF;this.pc=0;this.i=0;this.R=0;this.wz=0;
    this.iff1=0;this.iff2=0;this.im=0;this.halted=false;this.done=false;
    this.t=0;this.n=0;this.tLast=0;this.ixy=0;this.tx=0;
    this.bAddr=0;this.bData=0;this.bKind=0;this.lastPC=0;this.lastLen=1;
    this.aOp='—';this.aW=8;this.aA=0;this.aB=0;this.aC=0;this.aR=0;this.aF=0;
    this.hist.fill(-1);this.hp=0;this.wlog.fill(-1);this.wp=0;
  }
  /* --- bus --- */
  rd(a){a&=0xFFFF;const v=this.mem[a];this.bAddr=a;this.bData=v;this.bKind=1;return v;}
  wr(a,v){a&=0xFFFF;v&=255;this.mem[a]=v;this.bAddr=a;this.bData=v;this.bKind=2;this.wlog[this.wp++&15]=a;}
  fetch(){const a=this.pc;const v=this.mem[a];this.bAddr=a;this.bData=v;this.bKind=0;this.pc=(a+1)&0xFFFF;this.R=(this.R&0x80)|((this.R+1)&0x7F);return v;}
  imm8(){const a=this.pc;const v=this.mem[a];this.bAddr=a;this.bData=v;this.bKind=1;this.pc=(a+1)&0xFFFF;return v;}
  imm16(){const l=this.imm8();return l|(this.imm8()<<8);}
  ioIn(p){this.bAddr=p&0xFFFF;this.bData=0xFF;this.bKind=3;return 0xFF;}
  ioOut(p,v){this.bAddr=p&0xFFFF;this.bData=v&255;this.bKind=4;}
  push(v){this.sp=(this.sp-1)&0xFFFF;this.wr(this.sp,v>>8);this.sp=(this.sp-1)&0xFFFF;this.wr(this.sp,v&255);}
  pop(){const l=this.rd(this.sp);this.sp=(this.sp+1)&0xFFFF;const h=this.rd(this.sp);this.sp=(this.sp+1)&0xFFFF;return l|(h<<8);}
  /* --- registres --- */
  get AF(){return this.r[7]<<8|this.r[6];} get BC(){return this.r[0]<<8|this.r[1];}
  get DE(){return this.r[2]<<8|this.r[3];} get HL(){return this.r[4]<<8|this.r[5];}
  get AF2(){return this.r2[7]<<8|this.r2[6];} get BC2(){return this.r2[0]<<8|this.r2[1];}
  get DE2(){return this.r2[2]<<8|this.r2[3];} get HL2(){return this.r2[4]<<8|this.r2[5];}
  hl(){return this.ixy===0?(this.r[4]<<8|this.r[5]):this.ixy===1?this.ix:this.iy;}
  setHl(v){v&=0xFFFF;if(this.ixy===0){this.r[4]=v>>8;this.r[5]=v&255;}else if(this.ixy===1)this.ix=v;else this.iy=v;}
  rp(p){const r=this.r;switch(p){case 0:return r[0]<<8|r[1];case 1:return r[2]<<8|r[3];case 2:return this.hl();default:return this.sp;}}
  setRp(p,v){v&=0xFFFF;const r=this.r;switch(p){case 0:r[0]=v>>8;r[1]=v&255;break;case 1:r[2]=v>>8;r[3]=v&255;break;case 2:this.setHl(v);break;default:this.sp=v;}}
  rp2(p){return p===3?(this.r[7]<<8|this.r[6]):this.rp(p);}
  setRp2(p,v){if(p===3){this.r[7]=v>>8;this.r[6]=v&255;}else this.setRp(p,v);}
  g8(i){if(this.ixy){if(i===4)return(this.ixy===1?this.ix:this.iy)>>8;if(i===5)return(this.ixy===1?this.ix:this.iy)&255;}return this.r[i];}
  s8(i,v){if(this.ixy&&(i===4||i===5)){const w=this.ixy===1?this.ix:this.iy;const n=i===4?((w&255)|(v<<8)):((w&0xFF00)|v);if(this.ixy===1)this.ix=n;else this.iy=n;return;}this.r[i]=v;}
  ea(){
    if(this.ixy===0)return(this.r[4]<<8)|this.r[5];
    const d=this.imm8();const sd=d<128?d:d-256;
    const a=((this.ixy===1?this.ix:this.iy)+sd)&0xFFFF;this.wz=a;this.tx+=8;return a;
  }
  cond(cc){const f=this.r[6];switch(cc){case 0:return!(f&FZ);case 1:return!!(f&FZ);case 2:return!(f&FC);case 3:return!!(f&FC);case 4:return!(f&FP);case 5:return!!(f&FP);case 6:return!(f&FS);default:return!!(f&FS);}}
  /* --- ALU --- */
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
  inc8(v){const f=this.r[6];const res=(v+1)&255;this.aOp='INC';this.aW=8;this.aA=v;this.aB=1;this.aC=0;this.aR=res;this.aF=f;
    this.r[6]=(f&FC)|(res&0xA8)|(res?0:FZ)|((res&15)===0?FH:0)|(res===0x80?FP:0);return res;}
  dec8(v){const f=this.r[6];const res=(v-1)&255;this.aOp='DEC';this.aW=8;this.aA=v;this.aB=1;this.aC=0;this.aR=res;this.aF=f;
    this.r[6]=(f&FC)|(res&0xA8)|(res?0:FZ)|((res&15)===15?FH:0)|(res===0x7F?FP:0)|FN;return res;}
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
  add16(a,b){const t=a+b;const res=t&0xFFFF;const f=this.r[6];this.aOp='ADD16';this.aW=16;this.aA=a;this.aB=b;this.aC=0;this.aR=res;this.aF=f;
    this.r[6]=(f&(FS|FZ|FP))|((res>>8)&(FY|FX))|(((a^b^res)>>8)&FH)|((t>>16)&1);this.wz=(a+1)&0xFFFF;return res;}
  adc16(a,b){const c=this.r[6]&1;const t=a+b+c;const res=t&0xFFFF;this.aOp='ADC16';this.aW=16;this.aA=a;this.aB=b;this.aC=c;this.aR=res;this.aF=this.r[6];
    this.r[6]=((res>>8)&(FS|FY|FX))|(res?0:FZ)|(((a^b^res)>>8)&FH)|((~(a^b)&(a^res)&0x8000)?FP:0)|((t>>16)&1);this.wz=(a+1)&0xFFFF;return res;}
  sbc16(a,b){const c=this.r[6]&1;const t=a-b-c;const res=t&0xFFFF;this.aOp='SBC16';this.aW=16;this.aA=a;this.aB=b;this.aC=c;this.aR=res;this.aF=this.r[6];
    this.r[6]=((res>>8)&(FS|FY|FX))|(res?0:FZ)|(((a^b^res)>>8)&FH)|(((a^b)&(a^res)&0x8000)?FP:0)|FN|((t>>16)&1);this.wz=(a+1)&0xFFFF;return res;}
  daa(){
    const r=this.r,A=r[7],f=r[6];let add=0,c=0;
    if((f&FH)||(A&15)>9)add|=6;
    if((f&FC)||A>0x99){add|=0x60;c=1;}
    const res=(f&FN)?(A-add)&255:(A+add)&255;
    const h=(f&FN)?(((f&FH)&&(A&15)<6)?FH:0):((A&15)>9?FH:0);
    this.aOp='DAA';this.aW=8;this.aA=A;this.aB=add;this.aC=0;this.aR=res;this.aF=f;
    r[7]=res;r[6]=(res&0xA8)|(res?0:FZ)|PAR[res]|(f&FN)|h|c;
  }
  /* --- pas d'exécution --- */
  step(){
    if(this.cpm){
      if(this.pc===5){this.bdos();return this.tLast=17;}
      if(this.pc===0){this.done=true;return 0;}
    }
    this.hist[this.hp++&31]=this.pc;this.lastPC=this.pc;
    this.ixy=0;this.tx=0;let tt=0;
    let t;
    if(this.halted){this.R=(this.R&0x80)|((this.R+1)&0x7F);t=4;}
    else{
      let op=this.fetch();
      while(op===0xDD||op===0xFD){this.ixy=op===0xDD?1:2;tt+=4;op=this.fetch();}
      if(op===0xED){this.ixy=0;t=this.execED();}
      else if(op===0xCB)t=this.execCB();
      else t=this.execMain(op)+this.tx;
    }
    t+=tt;this.t+=t;this.n++;this.tLast=t;
    this.lastLen=(this.pc-this.lastPC)&0xFFFF;
    return t;
  }
  bdos(){
    const c=this.r[1];
    if(c===2){if(this.onChar)this.onChar(this.r[3]);}
    else if(c===9){let a=this.DE,k=0;while(this.mem[a]!==0x24&&k++<65536){if(this.onChar)this.onChar(this.mem[a]);a=(a+1)&0xFFFF;}}
    this.pc=this.pop();this.t+=17;this.n++;
  }
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
  setHl2(v){v&=0xFFFF;this.r[4]=v>>8;this.r[5]=v&255;}
  setDE(v){v&=0xFFFF;this.r[2]=v>>8;this.r[3]=v&255;}
  setBC(v){v&=0xFFFF;this.r[0]=v>>8;this.r[1]=v&255;}
  /* --- chargement ROM CP/M (.com / .cim chargés en 0x0100) --- */
  loadCPM(bytes,addr=0x100){
    this.reset();this.mem.fill(0);
    this.mem[0]=0x76;this.mem[5]=0xC9;this.mem[6]=0x00;this.mem[7]=0xFE;
    this.mem.set(bytes.subarray(0,Math.min(bytes.length,0xFE00-addr)),addr);
    this.pc=addr;this.sp=0xFFFE;this.cpm=true;this.progStart=addr;this.progLen=bytes.length;
  }
  loadRaw(bytes,addr=0){
    this.reset();this.mem.fill(0);this.mem.set(bytes.subarray(0,Math.min(bytes.length,65536-addr)),addr);
    this.pc=addr;this.sp=0xFFFF;this.cpm=false;this.progStart=addr;this.progLen=bytes.length;
  }
}

/* ---------- Désassembleur ---------- */
const R8=['B','C','D','E','H','L','(HL)','A'],RPN=['BC','DE','HL','SP'],RP2N=['BC','DE','HL','AF'];
const CCN=['NZ','Z','NC','C','PO','PE','P','M'];
const ALUD=['ADD A,','ADC A,','SUB ','SBC A,','AND ','XOR ','OR ','CP '];
const h2=v=>v.toString(16).toUpperCase().padStart(2,'0'),h4=v=>v.toString(16).toUpperCase().padStart(4,'0');
const ACC=['RLCA','RRCA','RLA','RRA','DAA','CPL','SCF','CCF'];
const EDX1_2=['LD (nn),BC','LD BC,(nn)'];
function disasm(rb,pc){
  let p=pc;const nb=()=>rb((p++)&0xFFFF);const nw=()=>{const l=nb();return l|(nb()<<8);};
  let ix='',b=nb(),pre='';
  while(b===0xDD||b===0xFD){ix=b===0xDD?'IX':'IY';pre=ix==='IX'?'DD':'FD';b=nb();}
  const hl=ix||'HL';
  const rn=i=>(i===4&&ix)?ix+'H':(i===5&&ix)?ix+'L':R8[i];
  const memop=()=>{if(!ix)return'(HL)';const d=nb();const sd=d<128?d:d-256;return`(${ix}${sd<0?'-':'+'}0x${h2(Math.abs(sd))})`;};
  const res=(text,op)=>({text,op,pre,len:(p-pc)&0xFFFF,addr:pc});
  if(b===0xED){
    const op=nb();const x=op>>6,y=(op>>3)&7,z=op&7,pp=y>>1,q=y&1;pre='ED';
    if(x===1){
      switch(z){
        case 0:return res(y===6?'IN (C)':`IN ${R8[y]},(C)`,op);
        case 1:return res(y===6?'OUT (C),0':`OUT (C),${R8[y]}`,op);
        case 2:return res(`${q?'ADC':'SBC'} HL,${RPN[pp]}`,op);
        case 3:{const a=nw();return res(q?`LD ${RPN[pp]},(0x${h4(a)})`:`LD (0x${h4(a)}),${RPN[pp]}`,op);}
        case 4:return res('NEG',op);
        case 5:return res(y===1?'RETI':'RETN',op);
        case 6:return res(`IM ${[0,0,1,2,0,0,1,2][y]}`,op);
        default:return res(['LD I,A','LD R,A','LD A,I','LD A,R','RRD','RLD','NOP','NOP'][y],op);
      }
    }
    if(x===2&&z<4&&y>=4){
      const nm=[['LDI','CPI','INI','OUTI'],['LDD','CPD','IND','OUTD'],['LDIR','CPIR','INIR','OTIR'],['LDDR','CPDR','INDR','OTDR']][y-4][z];
      return res(nm,op);
    }
    return res('NOP*',op);
  }
  if(b===0xCB){
    pre=(pre||'')+'CB';let d=null;
    if(ix){const dd=nb();d=dd<128?dd:dd-256;}
    const op=nb();const x=op>>6,y=(op>>3)&7,z=op&7;
    const tgt=ix?`(${ix}${d<0?'-':'+'}0x${h2(Math.abs(d))})`:R8[z];
    const extra=(ix&&z!==6)?','+R8[z]:'';
    if(x===0)return res(`${ROTN[y]} ${tgt}${extra}`,op);
    return res(`${['','BIT','RES','SET'][x]} ${y},${tgt}${extra}`,op);
  }
  const x=b>>6,y=(b>>3)&7,z=b&7,pp=y>>1,q=y&1;
  const rel=()=>{const d=nb();const sd=d<128?d:d-256;return'0x'+h4((p+sd)&0xFFFF);};
  switch(x){
  case 0:
    switch(z){
      case 0:
        if(y===0)return res('NOP',b);
        if(y===1)return res("EX AF,AF'",b);
        if(y===2)return res('DJNZ '+rel(),b);
        if(y===3)return res('JR '+rel(),b);
        return res(`JR ${CCN[y-4]},${rel()}`,b);
      case 1:
        if(q===0){const n=nw();return res(`LD ${pp===2?hl:RPN[pp]},0x${h4(n)}`,b);}
        return res(`ADD ${hl},${pp===2?hl:RPN[pp]}`,b);
      case 2:{
        switch(y){
          case 0:return res('LD (BC),A',b);case 1:return res('LD A,(BC)',b);
          case 2:return res('LD (DE),A',b);case 3:return res('LD A,(DE)',b);
          case 4:return res(`LD (0x${h4(nw())}),${hl}`,b);
          case 5:return res(`LD ${hl},(0x${h4(nw())})`,b);
          case 6:return res(`LD (0x${h4(nw())}),A`,b);
          default:return res(`LD A,(0x${h4(nw())})`,b);
        }}
      case 3:return res(`${q?'DEC':'INC'} ${pp===2?hl:RPN[pp]}`,b);
      case 4:return y===6?res(`INC ${memop()}`,b):res(`INC ${rn(y)}`,b);
      case 5:return y===6?res(`DEC ${memop()}`,b):res(`DEC ${rn(y)}`,b);
      case 6:
        if(y===6){const m=memop();const n=nb();return res(`LD ${m},0x${h2(n)}`,b);}
        return res(`LD ${rn(y)},0x${h2(nb())}`,b);
      default:return res(ACC[y],b);
    }
  case 1:
    if(b===0x76)return res('HALT',b);
    if(y===6){const m=memop();return res(`LD ${m},${R8[z]}`,b);}
    if(z===6){const m=memop();return res(`LD ${R8[y]},${m}`,b);}
    return res(`LD ${rn(y)},${rn(z)}`,b);
  case 2:
    if(z===6)return res(ALUD[y]+memop(),b);
    return res(ALUD[y]+rn(z),b);
  default:
    switch(z){
      case 0:return res('RET '+CCN[y],b);
      case 1:
        if(q===0)return res('POP '+(pp===2?hl:RP2N[pp]),b);
        return res(['RET','EXX',`JP (${hl})`,`LD SP,${hl}`][pp],b);
      case 2:return res(`JP ${CCN[y]},0x${h4(nw())}`,b);
      case 3:
        switch(y){
          case 0:return res(`JP 0x${h4(nw())}`,b);
          case 2:return res(`OUT (0x${h2(nb())}),A`,b);
          case 3:return res(`IN A,(0x${h2(nb())})`,b);
          case 4:return res(`EX (SP),${hl}`,b);
          case 5:return res('EX DE,HL',b);
          case 6:return res('DI',b);
          default:return res('EI',b);
        }
      case 4:return res(`CALL ${CCN[y]},0x${h4(nw())}`,b);
      case 5:
        if(q===0)return res('PUSH '+(pp===2?hl:RP2N[pp]),b);
        return res(`CALL 0x${h4(nw())}`,b);
      case 6:return res(ALUD[y]+'0x'+h2(nb()),b);
      default:return res(`RST 0x${h2(y*8)}`,b);
    }
  }
}
const api={Z80,disasm,h2,h4,PAR};
if(typeof module!=='undefined'&&module.exports)module.exports=api;else root.Z80LIB=api;
})(typeof window!=='undefined'?window:globalThis);
