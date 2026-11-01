// I-047 round 2 builder probe (spec "Builder checks before the full golden").
// Runs the round-1 probe cases (API lock, mouse never moved / centre /
// (100,100)), break-1 case A (#resume lock, press+release at the lock point)
// and case C (+450 px move, then press+release), and golden's real-click
// sequence (mouse to the centre, 2 frames, click) against the build in
// crates/bevy-client/web/pkg-out. A recorder registered before hud.js mirrors
// hud.js's drop rule per raw unit (stream, m, matched candidate, dropped or
// kept); the game state gives the real d yaw / d pitch of each case.
// Usage: node round2-probe.cjs   (hold the chromium lock)
const { createServer } = require('node:http');
const { readFile } = require('node:fs/promises');
const { join, extname, normalize } = require('node:path');
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const root = process.env.ROOT || join(__dirname, '../../../../../../crates/bevy-client/web/pkg-out');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const recorder = () => {
  window.__p = null;
  let lastPtr = null, edgePos = null, cdpPos = null, armPtr = false, armMouse = false;
  const near0 = (c) => Math.abs(c[0]) <= 2 && Math.abs(c[1]) <= 2;
  const log = (s) => { if (window.__p) window.__p.push(s); };
  const unit = (u, stream, ts) => {
    const mx = u.movementX, my = u.movementY;
    if (mx === 0 && my === 0) return null;
    let tag = 'kept (not armed)';
    if (stream === 'ptr' ? armPtr : armMouse) {
      tag = 'kept (armed, no match)';
      const w = [innerWidth / 2, innerHeight / 2];
      const cands = [['edgePos', edgePos], ['lastPtr', lastPtr], ['cdpPos', cdpPos], ['centre', w]];
      for (const [n, c] of cands) {
        if (c && !near0(c) && Math.abs(mx + c[0]) <= 2 && Math.abs(my + c[1]) <= 2) { tag = `DROPPED (matches ${n} ${c.map((v) => +v.toFixed(1)).join(',')})`; break; }
      }
      if (stream === 'ptr') armPtr = false; else armMouse = false;
    }
    if (mx >= 0 && my >= 0) cdpPos = [mx, my];
    log(`  ${stream === 'ptr' ? 'pointermove' : 'mousemove  '} ts=${ts} m=(${mx},${my}) -> ${tag}`);
    return tag;
  };
  const edge = (e) => {
    const locked = !!document.pointerLockElement;
    if (e.type === 'pointerlockchange') { edgePos = lastPtr; cdpPos = null; }
    else { if (!locked) lastPtr = [e.clientX, e.clientY]; edgePos = [e.clientX, e.clientY]; }
    armPtr = armMouse = true;
    log(`${e.type} ts=${e.timeStamp.toFixed(1)}${e.clientX !== undefined ? ` client=(${e.clientX},${e.clientY})` : ''} locked=${locked} edgePos=${edgePos && edgePos.map((v) => +v.toFixed(1))} lastPtr=${lastPtr && lastPtr.map((v) => +v.toFixed(1))}`);
  };
  window.addEventListener('pointerdown', edge, { capture: true });
  window.addEventListener('pointerup', edge, { capture: true });
  document.addEventListener('pointerlockchange', edge);
  window.addEventListener('pointermove', (e) => {
    const ts = e.timeStamp.toFixed(1);
    if (!document.pointerLockElement) { lastPtr = [e.clientX, e.clientY]; return; }
    const parts = e.getCoalescedEvents ? e.getCoalescedEvents() : [];
    if (parts.length > 1) for (const c of parts) unit(c, 'ptr', ts); else unit(e, 'ptr', ts);
  }, { capture: true });
  document.addEventListener('mousemove', (e) => { if (document.pointerLockElement) unit(e, 'mouse', e.timeStamp.toFixed(1)); }, { capture: true });
};

(async () => {
  const server = createServer(async (req, res) => {
    const p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    try { const b = await readFile(join(root, p || 'index.html')); res.writeHead(200, { 'content-type': types[extname(p || 'index.html')] || 'application/octet-stream' }); res.end(b); } catch { res.writeHead(404).end(); }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const url = `http://127.0.0.1:${server.address().port}/?low&debug`;
  const browser = await chromium.launch();
  const W = 1280, H = 720;
  const open = async () => {
    const page = await browser.newPage({ viewport: { width: W, height: H } });
    await page.addInitScript(recorder);
    await page.goto(url);
    for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) { if (Date.now() - t0 > 180000) throw new Error('no boot'); await sleep(250); }
    await sleep(2000);
    const st = () => page.evaluate(() => { const s = window.__game.state() || {}; return { yaw: s.yaw, pitch: s.pitch, locked: s.locked, frame: s.frame, paused: s.paused }; });
    const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
    const frames = async (n) => { const f = (await st()).frame; for (let t0 = Date.now(); (await st()).frame < f + n && Date.now() - t0 < 60000;) await sleep(30); };
    return { page, st, cmd, frames };
  };
  const results = [];
  const run = async (P, label, fn) => {
    await P.cmd('look 0 10');
    await P.frames(5);
    const a = await P.st();
    await P.page.evaluate(() => { window.__p = []; });
    await fn();
    await P.frames(10);
    const b = await P.st();
    const log = await P.page.evaluate(() => window.__p);
    const dy = ((((b.yaw - a.yaw) % 360) + 540) % 360) - 180, dp = b.pitch - a.pitch;
    const pass = Math.abs(dy) < 0.5 && Math.abs(dp) < 0.5;
    results.push(`${pass ? 'PASS' : 'FAIL'} ${label}: dyaw=${dy.toFixed(2)} dpitch=${dp.toFixed(2)}`);
    console.log(`\n== ${label}: locked ${a.locked}->${b.locked} dyaw=${dy.toFixed(2)} dpitch=${dp.toFixed(2)} ${pass ? 'PASS' : 'FAIL'}`);
    for (const l of log) console.log(l);
  };
  // Round-1 probe cases: API lock without moving the mouse.
  for (const pos of [null, [W / 2, H / 2], [100, 100]]) {
    const P = await open();
    await P.page.keyboard.press('Shift');
    await P.page.evaluate(() => document.getElementById('game').requestPointerLock());
    for (let k = 0; k < 40 && !(await P.page.evaluate(() => !!document.pointerLockElement)); k++) await sleep(100);
    await run(P, `round1 mouse ${pos ? pos.join(',') : 'never moved'}: move, 500 ms, press, 150 ms, release`, async () => {
      if (pos) { await P.page.mouse.move(pos[0], pos[1]); await sleep(500); }
      await P.page.mouse.down(); await sleep(150); await P.page.mouse.up();
    });
    await P.page.close();
  }
  // Lock the way players and golden do: pause, click #resume.
  const P = await open();
  if (!(await P.st()).paused) await P.page.evaluate(() => window.__game.pause(true));
  await sleep(500);
  await P.page.evaluate(() => { window.__p = []; });
  await P.page.click('#resume');
  for (let k = 0; k < 60 && !(await P.st()).locked; k++) await sleep(100);
  console.log('\n== lock via #resume:', JSON.stringify(await P.st()));
  for (const l of await P.page.evaluate(() => window.__p)) console.log(l);
  await run(P, 'break-1 A: press+release at the #resume lock point', async () => { await P.page.mouse.down(); await sleep(100); await P.page.mouse.up(); });
  await run(P, 'break-1 C: move +450 px, 2 frames, press+release', async () => {
    await P.page.mouse.move(489.6 + 450, 645.9, { steps: 1 }); await P.frames(2);
    await P.page.mouse.down(); await sleep(100); await P.page.mouse.up();
  });
  await run(P, 'golden real click: move to centre, 2 frames, click', async () => {
    await P.page.mouse.move(W / 2, H / 2); await P.frames(2);
    await P.page.mouse.click(W / 2, H / 2);
  });
  await run(P, 'golden real click again (mouse already at centre)', async () => { await P.page.mouse.click(W / 2, H / 2); });
  console.log('\n' + results.join('\n'));
  await browser.close(); server.close();
})().catch((e) => { console.error(e); process.exit(2); });
