// verify_timing.js - compares the T-state totals of docs/specs/data/*.tsv with the reference core (docs/ref/z80_ref.js)
// usage: node docs/tools/verify_timing.js   (maintenance tool for Claude, not for Hermes)
const {Z80}=require('../ref/z80_ref.js');const fs=require('fs');const path=require('path');
const D=path.join(__dirname,'..','specs','data');
function rows(f){return fs.readFileSync(path.join(D,f),'utf8').trim().split('\n').slice(1).map(l=>l.split('\t'));}
const cpu=new Z80();
function measure(bytes){
  let mn=1e9,mx=0;
  for(const F of [0x00,0xFF,0x40,0x01,0x04,0x80,0x41,0x44,0x81]) for(const BC of [0x0001,0x0002,0x0100,0x0200]) for(const A of [0,1]){
    cpu.reset();cpu.mem.fill(0);cpu.halted=false;
    cpu.mem.set(bytes,0x100);cpu.pc=0x100;cpu.sp=0x8000;
    cpu.r[6]=F;cpu.r[7]=A;cpu.r[0]=BC>>8;cpu.r[1]=BC&255;cpu.r[4]=0x40;cpu.r[5]=0;cpu.ix=0x4000;cpu.iy=0x4000;
    const t=cpu.step();mn=Math.min(mn,t);mx=Math.max(mx,t);
  }
  return [mn,mx];
}
let bad=0,n=0;
const T={main:['main.tsv',op=>[op,0,0]],cb:['cb.tsv',op=>[0xCB,op]],ed:['ed.tsv',op=>[0xED,op,0,0,0]],ddfd:['ddfd.tsv',op=>[0xDD,op,0,0,0]],ddcb:['ddcb.tsv',op=>[0xDD,0xCB,0,op]]};
for(const k in T){
  const [f,mk]=T[k];
  rows(f).forEach(r=>{
    const op=parseInt(r[0],16);if(r[1]==='-'||r[1]==='prefix')return;
    if(k==='ddfd'&&/^\(prefix ignored\)/.test(r[1])&&0)return;
    const [mn,mx]=measure(mk(op));n++;
    if(mn!==+r[4]||mx!==+r[5]){bad++;if(bad<40)console.log('MISMATCH',k,r[0],r[1],'tsv',r[4],r[5],'ref',mn,mx);}
  });
}
console.log('checked',n,'rows, mismatches:',bad);
process.exit(bad?1:0);
