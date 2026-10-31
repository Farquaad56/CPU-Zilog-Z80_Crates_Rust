// gen_diff_vectors.js - differential test vectors from the reference core (docs/ref/z80_ref.js)
// writes docs/specs/data/diff.tsv ; format and PRNG are specified in docs/specs/24_DIFF_VECTORS.md
const {Z80}=require('../ref/z80_ref.js');const fs=require('fs');const path=require('path');
const D=path.join(__dirname,'..','specs','data');
const NV=6; // vectors per opcode
function xs(s){s^=s<<13;s>>>=0;s^=s>>>17;s^=s<<5;return s>>>0;}
function hash(a,seed){let h=Math.imul((a^Math.imul(seed,0x9E3779B1))>>>0,0x45D9F3B)>>>0;h^=h>>>16;h=Math.imul(h,0x45D9F3B)>>>0;h^=h>>>16;return (h>>>24)&255;}
const TABLES=[['main',0],['cb',1],['ed',2],['ddfd',3],['ddcb',4]];
function rows(f){return fs.readFileSync(path.join(D,f),'utf8').trim().split('\n').slice(1).map(l=>l.split('\t'));}
const hx=(v,n)=>(v>>>0).toString(16).padStart(n,'0');
const out=[];
const cpu=new Z80();
for(const [name,tid] of TABLES){
  for(const r of rows(name+'.tsv')){
    const op=parseInt(r[0],16);if(r[1]==='-'||r[1]==='prefix')continue;
    if(name==='main'&&[0xCB,0xDD,0xED,0xFD].includes(op))continue;
    for(let v=0;v<NV;v++){
      const seed=(Math.imul((tid*0x10000+op*16+v),2654435761)|1)>>>0;
      let s=seed;const n8=()=>{s=xs(s);return s>>>24;};const n16=()=>{s=xs(s);return s>>>16;};
      cpu.reset();cpu.cpm=false;cpu.halted=false;
      const R=cpu.r,R2=cpu.r2;
      for(const i of [7,6,0,1,2,3,4,5])R[i]=n8();
      for(const i of [7,6,0,1,2,3,4,5])R2[i]=n8();
      cpu.ix=n16();cpu.iy=n16();cpu.sp=0x4000+(n16()&0x3FFF);cpu.pc=0x8000+(n16()&0xFF);
      cpu.i=n8();cpu.R=n8();const fl=n8();cpu.iff1=fl&1;cpu.iff2=(fl>>1)&1;cpu.im=(fl>>2)%3;
      cpu.wz=0;cpu.t=0;cpu.n=0;
      // memory: hash(addr,seed) everywhere, then overlay the code
      for(let a=0;a<65536;a++)cpu.mem[a]=hash(a,seed);
      const pre=(name==='ddfd'||name==='ddcb')?((v&1)?0xFD:0xDD):0;
      let code;
      if(name==='main')code=[op];else if(name==='cb')code=[0xCB,op];else if(name==='ed')code=[0xED,op];
      else if(name==='ddfd')code=[pre,op];else code=[pre,0xCB,cpu.mem[(cpu.pc+2)&0xFFFF],op];
      const pc0=cpu.pc;for(let k=0;k<code.length;k++)cpu.mem[(pc0+k)&0xFFFF]=code[k];
      const W=[],O=[],I=[];
      const ow=cpu.wr.bind(cpu),oo=cpu.ioOut.bind(cpu),oi=cpu.ioIn.bind(cpu);
      cpu.wr=(a,x)=>{W.push(hx(a&0xFFFF,4)+'='+hx(x&255,2));ow(a,x);};
      cpu.ioOut=(p,x)=>{O.push(hx(p&0xFFFF,4)+'='+hx(x&255,2));oo(p,x);};
      cpu.ioIn=(p)=>{I.push(hx(p&0xFFFF,4));return oi(p);};
      const t=cpu.step();
      delete cpu.wr;delete cpu.ioOut;delete cpu.ioIn;
      const g=i=>hx(R[i],2),g2=i=>hx(R2[i],2);
      out.push([name,hx(op,2),v,t,
        [7,6,0,1,2,3,4,5].map(g).join(' '),[7,6,0,1,2,3,4,5].map(g2).join(' '),
        hx(cpu.ix,4),hx(cpu.iy,4),hx(cpu.sp,4),hx(cpu.pc,4),hx(cpu.i,2),hx(cpu.R,2),cpu.iff1,cpu.iff2,cpu.im,cpu.halted?1:0,
        W.join(',')||'-',O.join(',')||'-',I.join(',')||'-'].join('\t'));
    }
  }
}
const hdr='table\topcode\tvec\tt\tA F B C D E H L\tA2 F2 B2 C2 D2 E2 H2 L2\tIX\tIY\tSP\tPC\tI\tR\tIFF1\tIFF2\tIM\tHALTED\twrites\tport_out\tport_in';
fs.writeFileSync(path.join(D,'diff.tsv'),hdr+'\n'+out.join('\n')+'\n');
// PRNG self-test vectors for the spec
let s=1;const first=[];for(let i=0;i<5;i++){s=xs(s);first.push(s);}
console.log('rows',out.length,'xorshift(1)=',first.join(','),'hash(0,1)=',hash(0,1),'hash(65535,7)=',hash(65535,7));
