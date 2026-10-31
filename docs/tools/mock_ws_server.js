#!/usr/bin/env node
/* mock_ws_server.js - TEST DOUBLE of the Rust server `z80-web` (docs/specs/31_FRONTEND_PROTOCOL.md).
 * It exists ONLY to test docs/frontend/index.html before the Rust crate exists (and as an executable
 * protocol example). It uses the JavaScript reference core docs/ref/z80_ref.js as its "CPU".
 * The real product never uses it: the real server drives the Rust crate puce-z80.
 * Usage: node docs/tools/mock_ws_server.js [--port 8080] [--roms tests/roms] [--page docs/frontend/index.html] */
'use strict';
const http = require('http'), crypto = require('crypto'), fs = require('fs'), path = require('path');
const { Z80, disasm } = require('../ref/z80_ref.js');
const arg = (n, d) => { const i = process.argv.indexOf('--' + n); return i > 0 ? process.argv[i + 1] : d; };
const PORT = +arg('port', 8080), ROMS = arg('roms', 'tests/roms'), PAGE = arg('page', path.join(__dirname, '..', 'frontend', 'index.html'));
const VERSION = '0.1.0-mock';
const SPEEDS = [1, 4, 15, 60, 300, 3000, 60000, 0];
const TOTALS = { 'prelim.com': 1, 'zexdoc.cim': 67, 'zexall.cim': 67 };

class Session {
  constructor() { this.cpu = new Z80(); this.rom = null; this.console = ''; this.consoleSeen = new Map(); this.running = false; this.speed = 3; this.bps = new Set();
    this.view = { mem_base: 0x100, follow: 'pc', dis_rows: 40, trace_rows: 16, stack_rows: 14 }; this.ok = 0; this.err = 0; this.line = ''; this.acc = 0; this.last = Date.now(); this.stat = []; this.stopReason = ''; }
  load(name, bytes, raw) {
    this.cpu = new Z80(); this.console = ''; this.ok = 0; this.err = 0; this.line = ''; this.stopReason = ''; this.running = false;
    this.cpu.onChar = c => { if (c === 13) return; if (c === 10) { const l = this.line.trim(); if (/\bOK$/.test(l)) this.ok++; else if (/ERROR/.test(l)) this.err++; this.line = ''; this.console += '\n'; } else if ((c >= 32 && c < 127) || c === 9) { const ch = String.fromCharCode(c); this.line += ch; this.console += ch; } };
    raw ? this.cpu.loadRaw(bytes, 0) : this.cpu.loadCPM(bytes, 0x100);
    this.rom = { name, size: bytes.length, kind: raw ? 'raw' : 'cpm', bytes, raw };
  }
  runN(n) { const c = this.cpu; for (let i = 0; i < n; i++) { c.step(); if (c.done || c.halted) { this.stopReason = c.done ? 'done' : 'halt'; this.running = false; return; } if (this.bps.size && this.bps.has(c.pc)) { this.stopReason = 'bp'; this.running = false; return; } } }
  tick() { if (!this.running) return; const now = Date.now(), dt = Math.min(now - this.last, 100); this.last = now; const ips = SPEEDS[this.speed];
    if (ips > 0) { this.acc += dt / 1000 * ips; const n = Math.floor(this.acc); this.acc -= n; this.runN(n); } else { const t0 = Date.now(); while (this.running && Date.now() - t0 < 20) this.runN(4000); }
    this.stat.push([now, this.cpu.n, this.cpu.t]); while (this.stat.length > 30 || (this.stat.length > 2 && now - this.stat[0][0] > 900)) this.stat.shift(); }
  status() { const c = this.cpu; if (this.running) return 'running'; if (c.done) return 'done'; if (c.halted) return 'halted'; if (this.stopReason === 'bp') return 'bp'; return 'idle'; }
  snapshot(cid) {
    const c = this.cpu, r = c.r, v = this.view, rb = a => c.mem[a & 0xFFFF];
    let ips = 0, mhz = 0; if (this.stat.length > 1 && this.running) { const a = this.stat[0], b = this.stat[this.stat.length - 1], d = (b[0] - a[0]) / 1000; if (d > 0) { ips = (b[1] - a[1]) / d; mhz = (b[2] - a[2]) / d / 1e6; } }
    const dis = []; let a = c.pc; for (let i = 0; i < v.dis_rows; i++) { const d = disasm(rb, a); const bytes = []; for (let k = 0; k < d.len; k++) bytes.push(rb(a + k)); dis.push({ addr: a, bytes, text: d.text, len: d.len, pre: d.pre || '', op: d.op | 0 }); a = (a + d.len) & 0xFFFF; }
    const trace = []; for (let k = 1; k <= v.trace_rows; k++) { const pc = c.hist[(c.hp - k) & 31]; if (pc < 0 || k > c.n) break; trace.push({ pc, text: disasm(rb, pc).text }); }
    const stack = []; for (let i = 0; i < v.stack_rows; i++) { const ad = (c.sp + i * 2) & 0xFFFF; stack.push({ addr: ad, word: rb(ad) | (rb(ad + 1) << 8) }); }
    let base = v.mem_base; const fol = { pc: c.pc, sp: c.sp, hl: c.HL, de: c.DE, bc: c.BC }[v.follow]; if (v.follow === 'wr') { const w = c.wlog[(c.wp - 1) & 15]; base = (w >= 0 ? w : c.pc) & 0xFF00; } else if (fol !== undefined) base = fol & 0xFF00;
    let hexs = ''; for (let i = 0; i < 256; i++) hexs += rb(base + i).toString(16).padStart(2, '0');
    const wl = []; for (let i = 0; i < 16; i++) if (c.wlog[i] >= 0) wl.push(c.wlog[i]);
    const k = c.bKind, pins = (BigInt(c.bAddr) | (BigInt(c.bData) << 16n) | (k === 0 ? 1n << 24n : 0n) | (k <= 2 ? 1n << 25n : 0n) | (k >= 3 ? 1n << 26n : 0n) | ((k === 0 || k === 1 || k === 3) ? 1n << 27n : 0n) | ((k === 2 || k === 4) ? 1n << 28n : 0n) | (c.halted ? 1n << 29n : 0n) | (k === 0 ? 1n << 33n : 0n));
    const cur = this.consoleSeen.get(cid) || 0, delta = this.console.slice(cur); this.consoleSeen.set(cid, this.console.length);
    return { type: 'state', version: VERSION, status: this.status(), n: c.n, t: c.t, ips, mhz, tlast: c.tLast,
      regs: { af: c.AF, bc: c.BC, de: c.DE, hl: c.HL, af2: c.AF2, bc2: c.BC2, de2: c.DE2, hl2: c.HL2, ix: c.ix, iy: c.iy, sp: c.sp, pc: c.pc, i: c.i, r: c.R, wz: c.wz, iff1: c.iff1 ? 1 : 0, iff2: c.iff2 ? 1 : 0, im: c.im },
      bus: { addr: c.bAddr, data: c.bData, kind: k, pins: '0x' + pins.toString(16) },
      alu: c.aOp ? { op: c.aOp, w: c.aW, a: c.aA, b: c.aB, c: c.aC, r: c.aR, f_in: c.aF } : null,
      dis, trace, stack, mem: { base, hex: hexs }, wlog: wl, console_delta: delta, tests: { ok: this.ok, err: this.err, total: this.rom ? (TOTALS[path.basename(this.rom.name).toLowerCase()] || 0) : 0 },
      bps: [...this.bps], rom: this.rom ? { name: this.rom.name, size: this.rom.size, kind: this.rom.kind } : null }; }
}
function listRoms() { const out = []; (function walk(d, depth) { let es = []; try { es = fs.readdirSync(d, { withFileTypes: true }); } catch (e) { return; } for (const e of es) { const p = path.join(d, e.name); if (e.isDirectory() && depth < 3) walk(p, depth + 1); else if (/\.(com|cim|bin)$/i.test(e.name)) out.push({ name: path.relative(ROMS, p).split(path.sep).join('/'), size: fs.statSync(p).size, total_tests: TOTALS[e.name.toLowerCase()] || 0 }); } })(ROMS, 0); return out; }

const sess = new Session(); const clients = new Map(); let cidSeq = 1;
function frame(str) { const b = Buffer.from(str); const n = b.length; let h; if (n < 126) h = Buffer.from([0x81, n]); else if (n < 65536) { h = Buffer.alloc(4); h[0] = 0x81; h[1] = 126; h.writeUInt16BE(n, 2); } else { h = Buffer.alloc(10); h[0] = 0x81; h[1] = 127; h.writeBigUInt64BE(BigInt(n), 2); } return Buffer.concat([h, b]); }
function sendTo(c, o) { try { c.sock.write(frame(JSON.stringify(o))); } catch (e) {} }
function pushAll() { for (const [id, c] of clients) sendTo(c, sess.snapshot(id)); }
function handle(id, c, m) {
  switch (m.cmd) {
    case 'hello': sendTo(c, { type: 'hello', version: VERSION }); sendTo(c, { type: 'roms', roms: listRoms() }); break;
    case 'list_roms': sendTo(c, { type: 'roms', roms: listRoms() }); break;
    case 'load': { const p = path.join(ROMS, String(m.name)); if (String(m.name).includes('..') || !fs.existsSync(p)) return sendTo(c, { type: 'error', message: 'bad rom' }); sess.load(m.name, new Uint8Array(fs.readFileSync(p)), /\.bin$/i.test(m.name)); sess.consoleSeen.forEach((_, k) => sess.consoleSeen.set(k, 0)); break; }
    case 'upload': { const b = new Uint8Array(Buffer.from(String(m.data_b64), 'base64')); if (b.length > (1 << 20)) return sendTo(c, { type: 'error', message: 'too big' }); sess.load(m.name, b, /\.(bin|rom)$/i.test(m.name)); sess.consoleSeen.forEach((_, k) => sess.consoleSeen.set(k, 0)); break; }
    case 'run': if (sess.rom && !sess.cpu.done && !sess.cpu.halted) { sess.running = true; sess.stopReason = ''; sess.last = Date.now(); sess.acc = 0; } break;
    case 'pause': sess.running = false; break;
    case 'step': sess.running = false; sess.runN(Math.max(1, Math.min(1e6, m.n | 0 || 1))); break;
    case 'reset': if (sess.rom) { const r = sess.rom; sess.load(r.name, r.bytes, r.raw); sess.consoleSeen.forEach((_, k) => sess.consoleSeen.set(k, 0)); } break;
    case 'speed': if (m.idx >= 0 && m.idx <= 7) sess.speed = m.idx | 0; else return sendTo(c, { type: 'error', message: 'speed idx' }); break;
    case 'bp_add': sess.bps.add(m.addr & 0xFFFF); break; case 'bp_del': sess.bps.delete(m.addr & 0xFFFF); break;
    case 'view': Object.assign(sess.view, { mem_base: (m.mem_base | 0) & 0xFFF0, follow: m.follow || 'pc', dis_rows: Math.min(100, m.dis_rows | 0 || 40), trace_rows: Math.min(64, m.trace_rows | 0 || 16), stack_rows: Math.min(64, m.stack_rows | 0 || 14) }); break;
    case 'console_clear': sess.console = ''; sess.consoleSeen.forEach((_, k) => sess.consoleSeen.set(k, 0)); break;
    default: return sendTo(c, { type: 'error', message: 'unknown cmd' });
  }
  sendTo(c, sess.snapshot(id)); if (m.cmd === 'load' || m.cmd === 'upload') { sendTo(c, { type: 'roms', roms: listRoms() }); }
}
const srv = http.createServer((req, res) => {
  if (req.url === '/' || req.url === '/index.html') { res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' }); res.end(fs.readFileSync(PAGE)); }
  else if (req.url === '/health') { res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify({ ok: true, version: VERSION })); }
  else { res.writeHead(404); res.end('not found'); }
});
srv.on('upgrade', (req, sock) => {
  if (req.url !== '/ws') return sock.destroy();
  const key = req.headers['sec-websocket-key']; const acc = crypto.createHash('sha1').update(key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
  sock.write('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ' + acc + '\r\n\r\n');
  const id = cidSeq++, c = { sock }; clients.set(id, c); let buf = Buffer.alloc(0);
  sock.on('data', d => { buf = Buffer.concat([buf, d]);
    for (;;) { if (buf.length < 2) break; const op = buf[0] & 15; let len = buf[1] & 127, off = 2; if (len === 126) { if (buf.length < 4) break; len = buf.readUInt16BE(2); off = 4; } else if (len === 127) { if (buf.length < 10) break; len = Number(buf.readBigUInt64BE(2)); off = 10; }
      const masked = (buf[1] & 128) !== 0; const need = off + (masked ? 4 : 0) + len; if (buf.length < need) break;
      let payload = buf.slice(off + (masked ? 4 : 0), need); if (masked) { const mk = buf.slice(off, off + 4); payload = Buffer.from(payload.map((b, i) => b ^ mk[i & 3])); } buf = buf.slice(need);
      if (op === 8) { sock.end(); return; } if (op !== 1) continue;
      let m; try { m = JSON.parse(payload.toString()); } catch (e) { sendTo(c, { type: 'error', message: 'bad json' }); continue; } handle(id, c, m); } });
  const cl = () => clients.delete(id); sock.on('close', cl); sock.on('error', cl);
});
setInterval(() => { const was = sess.running; sess.tick(); if (was || sess.running) pushAll(); }, 33);
srv.listen(PORT, '127.0.0.1', () => console.log('mock z80-web (TEST DOUBLE) on http://127.0.0.1:' + PORT + '/'));
