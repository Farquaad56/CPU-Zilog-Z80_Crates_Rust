// End-to-end browser test: the real page, the real z80-web server, the real puce-z80 crate.
// Needs Node 18+ and Playwright (see run.sh, which skips when they are missing).
//
//   node tests/e2e/browser.mjs
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const PORT = 8099;
const URL_ = `http://127.0.0.1:${PORT}/`;
const exe = join(root, 'target', 'release', process.platform === 'win32' ? 'z80-web.exe' : 'z80-web');

// require() honours NODE_PATH, unlike a bare ESM import.
const { chromium } = createRequire(import.meta.url)('playwright');

if (!existsSync(exe)) {
  console.error(`missing ${exe}: run cargo build --release -p puce-z80-web`);
  process.exit(2);
}

const server = spawn(exe, ['--port', String(PORT), '--roms', join(root, 'tests', 'roms')], {
  stdio: ['pipe', 'ignore', 'inherit'],
});
let failures = 0;
const check = (ok, what) => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  if (!ok) failures++;
};

async function finish(code) {
  server.stdin.write('q\n'); // clean stop
  await new Promise((r) => setTimeout(r, 500));
  server.kill();
  process.exit(code);
}

try {
  const browser = await chromium.launch();
  // 1920x1080 at device scale 2 = a 3840x2160 capture.
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 2 });
  let opened = false;
  for (let i = 0; i < 50 && !opened; i++) {
    try {
      await page.goto(URL_, { timeout: 2000 });
      opened = true;
    } catch {
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  check(opened, 'page opens');

  // (1) title
  check((await page.title()).includes('Zilog Z80 Emulator'), 'title is "Zilog Z80 Emulator"');

  // the ROM list arrives over the WebSocket; load prelim.com
  const prelim = page.locator('#romBar button.rom[title$="prelim.com"]');
  await prelim.waitFor({ timeout: 15000 });
  await prelim.click();
  await page.waitForFunction(() => document.querySelector('#romBar .rom.sel') !== null, null, { timeout: 5000 });

  const pcLoaded = (await page.textContent('#v_PC')).trim();

  // turbo, then Run
  await page.$eval('#speed', (e) => {
    e.value = 7;
    e.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await page.click('#bRun');
  await page.waitForFunction(() => document.getElementById('status').classList.contains('b-done'), null, {
    timeout: 120000,
  });

  // (2) console
  const term = await page.textContent('#term');
  check(term.includes('Preliminary tests complete'), 'console shows "Preliminary tests complete"');

  // (3) the registers were refreshed from real states (instruction count > 0; PC is shown)
  // At "done" the program has returned to CP/M, so PC is 0000 by design: "PC is not 0000" is
  // checked on the loaded program (before Run) and the other registers after the run.
  const instr = (await page.textContent('#sInstr')).replace(/\D/g, '');
  const af = (await page.textContent('#v_AF')).trim();
  const sp = (await page.textContent('#v_SP')).trim();
  check(pcLoaded !== '0000', `PC after load is not 0000 (${pcLoaded})`);
  check(Number(instr) > 0, `instruction counter advanced (${instr})`);
  check(af !== '0000' || sp !== '0000', `registers refreshed (AF=${af}, SP=${sp})`);

  // (4) FR / EN switch changes at least 3 labels
  const labels = () => page.$$eval('.lbl, button, h1, h2, h3', (els) => els.map((e) => e.textContent.trim()));
  const fr = await labels();
  await page.click('#langBar button[data-l="en"]');
  await page.waitForTimeout(300);
  const en = await labels();
  const changed = fr.filter((t, i) => t !== en[i]).length;
  check(changed >= 3, `FR/EN switch changed ${changed} labels`);

  await page.screenshot({ path: join(here, 'shot_4k.png') });
  await browser.close();
} catch (e) {
  console.error(e);
  failures++;
}
await finish(failures ? 1 : 0);
