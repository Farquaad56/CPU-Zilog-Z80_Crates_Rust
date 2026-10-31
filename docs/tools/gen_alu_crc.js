// gen_alu_crc.js - canonical CRC32 of exhaustive ALU results from the reference core -> docs/specs/data/alu_crc.tsv
const {Z80}=require('../ref/z80_ref.js');const fs=require('fs');const path=require('path');
const cpu=new Z80();
let T=null;for(let n=0;n<256;n++){let c=n;for(let k=0;k<8;k++)c=c&1?(0xEDB88320^(c>>>1))>>>0:c>>>1;(T=T||[])[n]=c>>>0;}
function crc(bytes){let c=0xFFFFFFFF;for(const b of bytes)c=(T[(c^b)&255]^(c>>>8))>>>0;return((c^0xFFFFFFFF)>>>0).toString(16).padStart(8,'0');}
const rows=[];const add=(name,bytes,count)=>rows.push([name,count,crc(bytes)].join('\t'));
const N8=['add','adc','sub','sbc','and','xor','or','cp'];
for(let y=0;y<8;y++){const o=[];let n=0;
  for(let a=0;a<256;a++)for(let b=0;b<256;b++)for(let c=0;c<2;c++){cpu.reset();cpu.r[7]=a;cpu.r[6]=c;cpu.alu(y,b);o.push(cpu.r[7],cpu.r[6]);n++;}
  add('alu8_'+N8[y],o,n);}
{const o=[];let n=0;for(let v=0;v<256;v++)for(let c=0;c<2;c++){cpu.reset();cpu.r[6]=c;const r=cpu.inc8(v);o.push(r,cpu.r[6]);n++;}add('inc8',o,n);}
{const o=[];let n=0;for(let v=0;v<256;v++)for(let c=0;c<2;c++){cpu.reset();cpu.r[6]=c;const r=cpu.dec8(v);o.push(r,cpu.r[6]);n++;}add('dec8',o,n);}
const RN=['rlc','rrc','rl','rr','sla','sra','sll','srl'];
for(let y=0;y<8;y++){const o=[];let n=0;for(let v=0;v<256;v++)for(let c=0;c<2;c++){cpu.reset();cpu.r[6]=c;const r=cpu.rot(y,v);o.push(r,cpu.r[6]);n++;}add('rot_'+RN[y],o,n);}
for(const [nm,fn,fins] of [['add16',(a,b)=>cpu.add16(a,b),[0x00,0xFF]],['adc16',(a,b)=>cpu.adc16(a,b),[0x00,0x01]],['sbc16',(a,b)=>cpu.sbc16(a,b),[0x00,0x01]]]){
  const o=[];let n=0;for(let a=0;a<65536;a+=257)for(let b=0;b<65536;b+=263)for(const f of fins){cpu.reset();cpu.r[6]=f;const r=fn(a,b);o.push(r&255,r>>8,cpu.r[6]);n++;}add(nm,o,n);}
{const o=[];let n=0;for(let a=0;a<256;a++)for(let f=0;f<256;f++){cpu.reset();cpu.r[7]=a;cpu.r[6]=f;cpu.daa();o.push(cpu.r[7],cpu.r[6]);n++;}add('daa',o,n);}
{const o=[];let n=0;for(let a=0;a<256;a++){const res=(-a)&255;const F=(res&0xA8)|(res?0:0x40)|((0^a^res)&0x10)|(a===0x80?4:0)|2|(a?1:0);o.push(res,F);n++;}add('neg',o,n);}
{const o=[];let n=0;for(let v=0;v<256;v++)for(let y=0;y<8;y++)for(let c=0;c<2;c++){const bit=v&(1<<y);const F=c|0x10|(v&0x28)|(bit?0:0x44)|((y===7&&bit)?0x80:0);o.push(F);n++;}add('bit_reg',o,n);}
fs.writeFileSync(path.join(__dirname,'..','specs','data','alu_crc.tsv'),'name\tcount\tcrc32\n'+rows.join('\n')+'\n');
console.log(rows.length,'rows');rows.slice(0,3).forEach(r=>console.log(r));
