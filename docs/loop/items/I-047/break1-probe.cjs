// I-047 breaker probe: lock by clicking #resume like the game; then various sequences.
const { createServer } = require('node:http');
const { readFile } = require('node:fs/promises');
const { join, extname, normalize } = require('node:path');
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const root = '/tmp/claude-0/brk12-I-047/crates/bevy-client/web/pkg-out';
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
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
  const page = await browser.newPage({ viewport: { width: W, height: H } });
  await page.addInitScript(() => {
    window.__p = null;
    for (const t of ['pointerdown', 'pointerup', 'pointermove', 'mousemove']) {
      window.addEventListener(t, (e) => { if (window.__p) window.__p.push({ t, ts: +e.timeStamp.toFixed(1), mx: e.movementX, my: e.movementY, cx: e.clientX, cy: e.clientY, tr: e.isTrusted }); }, { capture: true });
    }
  });
  await page.goto(url);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) { if (Date.now() - t0 > 180000) throw new Error('no boot'); await sleep(250); }
  await sleep(2000);
  const st = () => page.evaluate(() => { const s = window.__game.state() || {}; return { yaw: s.yaw, pitch: s.pitch, locked: s.locked, frame: s.frame, casts: s.casts, paused: s.paused }; });
  const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
  console.log('keys', await page.evaluate(() => Object.keys(window.__game || {}).join(',')));
  console.log('state0', JSON.stringify(await st()));
  // Lock the way players and golden do: pause then click #resume.
  const lockViaResume = async () => {
    if (!(await st()).paused) await page.evaluate(() => window.__game.pause(true));
    await sleep(500);
    await page.click('#resume');
    for (let k = 0; k < 60 && !(await st()).locked; k++) await sleep(100);
  };
  await lockViaResume();
  console.log('after lock', JSON.stringify(await st()), 'mouse at #resume');
  const frames = async (n) => { const f = (await st()).frame; for (let t0 = Date.now(); (await st()).frame < f + n && Date.now() - t0 < 30000;) await sleep(30); };
  const run = async (label, fn) => {
    try { await cmd('look 0 10'); } catch (e) { console.log('cmd err', e.message); }
    await frames(4);
    const a = await st();
    await page.evaluate(() => { window.__p = []; });
    await fn();
    await frames(8);
    const b = await st();
    const log = await page.evaluate(() => window.__p);
    const d = ((((b.yaw - a.yaw) % 360) + 540) % 360) - 180;
    console.log(`\n== ${label}: locked ${a.locked}->${b.locked} dyaw=${d.toFixed(2)} dpitch=${(b.pitch - a.pitch).toFixed(2)} casts ${a.casts}->${b.casts}`);
    let edge = null;
    for (const e of log) { if (e.t === 'pointerdown' || e.t === 'pointerup') edge = e.ts; if (e.t === 'mousemove') continue; console.log(`  ${e.t.padEnd(11)} dEdge=${edge == null ? '-' : (e.ts - edge).toFixed(0)} mv=(${e.mx},${e.my}) client=(${e.cx},${e.cy}) tr=${e.tr}`); }
  };
  // A. press/release at lock point (golden T1 case)
  await run('A press+release at lock point', async () => { await page.mouse.down(); await sleep(100); await page.mouse.up(); });
  // B. genuine flick: one CDP move of +450 x, no press nearby
  await run('B flick +450,0 (1 step)', async () => { await page.mouse.move(489.6 + 450, 645.9, { steps: 1 }); });
  // C. press/release after the mouse moved away from the lock point
  await run('C press+release after mouse moved +450', async () => { await page.mouse.down(); await sleep(100); await page.mouse.up(); });
  // D. flick back -600,-300 (two big axes)
  await run('D flick -600,-300 (1 step)', async () => { await page.mouse.move(339.6, 345.9, { steps: 1 }); });
  // E. press, then flick, then release
  await run('E press, flick +300 (5 steps), release', async () => { await page.mouse.down(); await page.mouse.move(639.6, 345.9, { steps: 5 }); await page.mouse.up(); });
  // F. synthetic untrusted event pair at 0ms, like spec T2
  await run('F synthetic pointerup + (-640,-360)', async () => { await page.evaluate(() => { document.dispatchEvent(new PointerEvent('pointerup', { button: 0, pointerType: 'mouse', bubbles: true })); document.getElementById('game').dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse', movementX: -640, movementY: -360 })); }); });
  // H. park mouse at (0,0) then press/release (test-side workaround)
  await run('H move to (0,0)', async () => { await page.mouse.move(0, 0); });
  await run('H2 press+release with mouse parked at (0,0)', async () => { await page.mouse.down(); await sleep(100); await page.mouse.up(); });
  // G. keyboard look unaffected (arrow key)
  await run('G ArrowLeft held 600ms', async () => { await page.keyboard.down('ArrowLeft'); await sleep(600); await page.keyboard.up('ArrowLeft'); });
  await browser.close(); server.close();
})().catch((e) => { console.error(e); process.exit(2); });
