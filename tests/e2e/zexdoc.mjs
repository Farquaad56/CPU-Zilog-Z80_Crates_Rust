// ZEXDOC from the page, at turbo speed: 67 tests ok, 0 err. Takes several minutes.
//
//   node tests/e2e/zexdoc.mjs
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const PORT = 8098;
const rom = join(root, 'tests', 'roms', 'zexdoc.cim');
const exe = join(root, 'target', 'release', process.platform === 'win32' ? 'z80-web.exe' : 'z80-web');
const TIMEOUT_MS = 2 * 60 * 60 * 1000; // 2 h

if (!existsSync(rom)) {
  console.log(`SKIP: ${rom} not found`);
  process.exit(0);
}
if (!existsSync(exe)) {
  console.error(`missing ${exe}: run cargo build --release -p puce-z80-web`);
  process.exit(2);
}
const { chromium } = createRequire(import.meta.url)('playwright');

const server = spawn(exe, ['--port', String(PORT), '--roms', join(root, 'tests', 'roms')], {
  stdio: ['pipe', 'ignore', 'inherit'],
});
let failures = 0;
const check = (ok, what) => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  if (!ok) failures++;
};
async function finish(code) {
  server.stdin.write('q\n');
  await new Promise((r) => setTimeout(r, 500));
  server.kill();
  process.exit(code);
}

try {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  for (let i = 0; i < 50; i++) {
    try {
      await page.goto(`http://127.0.0.1:${PORT}/`, { timeout: 2000 });
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  const btn = page.locator('#romBar button.rom[title$="zexdoc.cim"]');
  await btn.waitFor({ timeout: 15000 });
  await btn.click();
  await page.waitForFunction(() => document.querySelector('#romBar .rom.sel') !== null, null, { timeout: 5000 });

  await page.$eval('#speed', (e) => {
    e.value = 7;
    e.dispatchEvent(new Event('input', { bubbles: true }));
  });
  const t0 = Date.now();
  await page.click('#bRun');
  await page.waitForFunction(() => document.getElementById('term').textContent.includes('Tests complete'), null, {
    timeout: TIMEOUT_MS,
    polling: 2000,
  });
  const secs = Math.round((Date.now() - t0) / 1000);
  // let the final state arrive
  await page.waitForTimeout(500);

  const ok = Number((await page.textContent('#nOk')).trim());
  const err = Number((await page.textContent('#nErr')).trim());
  check(ok === 67, `tests ok == 67 (${ok}) after ${secs} s`);
  check(err === 0, `tests err == 0 (${err})`);
  await browser.close();
} catch (e) {
  console.error(e);
  failures++;
}
await finish(failures ? 1 : 0);
