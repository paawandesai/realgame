// I-047 probe: timeStamp gap between a locked button edge and the bogus move,
// and the bogus move's movement/client with the mouse at three positions.
const { createServer } = require('node:http');
const { readFile } = require('node:fs/promises');
const { join, extname, normalize } = require('node:path');
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const root = '/home/claude/realgame/.claude/worktrees/w12-I-047/crates/bevy-client/web/pkg-out';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
(async () => {
  const server = createServer(async (req, res) => {
    const p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    try { const b = await readFile(join(root, p || 'index.html')); res.writeHead(200, { 'content-type': types[extname(p || 'index.html')] || 'application/octet-stream' }); res.end(b); } catch { res.writeHead(404).end(); }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const url = `http://127.0.0.1:${server.address().port}/?low&debug`;
  const browser = await chromium.launch();
  const W = 1280, H = 720;
  for (const pos of [null, [W / 2, H / 2], [100, 100]]) {
    const page = await browser.newPage({ viewport: { width: W, height: H } });
    // Recorder registered before hud.js (hud.js stops pointermove at window capture).
    await page.addInitScript(() => {
      window.__p = null;
      for (const t of ['pointerdown', 'pointerup', 'pointermove', 'mousemove']) {
        window.addEventListener(t, (e) => { if (window.__p) window.__p.push({ t, ts: +e.timeStamp.toFixed(2), mx: e.movementX, my: e.movementY, cx: e.clientX, cy: e.clientY, b: e.buttons, trusted: e.isTrusted, parts: typeof e.getCoalescedEvents === 'function' ? e.getCoalescedEvents().map((c) => [c.movementX, c.movementY]) : null }); }, { capture: true });
      }
    });
    await page.goto(url);
    for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) { if (Date.now() - t0 > 120000) throw new Error('no boot'); await sleep(250); }
    await sleep(2000);
    // Lock without moving the mouse: a key press gives user activation.
    await page.keyboard.press('Shift');
    await page.evaluate(() => document.getElementById('game').requestPointerLock());
    for (let k = 0; k < 40 && !(await page.evaluate(() => !!document.pointerLockElement)); k++) await sleep(100);
    const locked = await page.evaluate(() => !!document.pointerLockElement);
    if (pos) { await page.mouse.move(pos[0], pos[1]); await sleep(500); }
    await page.evaluate(() => { window.__p = []; });
    await page.mouse.down();
    await sleep(150);
    await page.mouse.up();
    await sleep(500);
    const log = await page.evaluate(() => window.__p);
    console.log(`\n== mouse ${pos ? pos.join(',') : 'never moved'} locked=${locked}`);
    let edge = null;
    for (const e of log) {
      if (e.t === 'pointerdown' || e.t === 'pointerup') edge = e.ts;
      console.log(`${e.t.padEnd(11)} ts=${e.ts} dEdge=${edge == null ? '-' : (e.ts - edge).toFixed(2)} mv=(${e.mx},${e.my}) client=(${e.cx},${e.cy}) buttons=${e.b} trusted=${e.trusted} parts=${JSON.stringify(e.parts)}`);
    }
    await page.close();
  }
  await browser.close(); server.close();
})().catch((e) => { console.error(e); process.exit(2); });
