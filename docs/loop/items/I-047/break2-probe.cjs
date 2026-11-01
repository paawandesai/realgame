// I-047 break-2 probe (Breaker, wave 12). Usage: ROOT=<pkg-out> WEB=<crates/bevy-client/web> node break2-probe.cjs  (hold the chromium lock)
// A: headless mouse.click / golden real-click pattern; B: synthetic real-mouse-like flicks after a click; C: small window; D: age picker (served with rg-analytics on).
const { createServer } = require('node:http');
const { readFile } = require('node:fs/promises');
const { join, extname, normalize } = require('node:path');
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const root = process.env.ROOT;
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const LPP = 10.89 / 200; // deg per px at sens 1 (T3: +300 clamps to 200 -> 10.89)
(async () => {
  const server = createServer(async (req, res) => {
    const p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    try { let b; try { b = await readFile(join(root, p || 'index.html')); } catch { b = await readFile(join(process.env.WEB, p)); }
      if ((p || 'index.html') === 'index.html' && req.url.includes('age=on')) b = Buffer.from(String(b).replace(/(name="rg-analytics"\s+content=")off/, '$1on'));
      res.writeHead(200, { 'content-type': types[extname(p || 'index.html')] || 'application/octet-stream' }); res.end(b); } catch { res.writeHead(404).end(); }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const base = `http://127.0.0.1:${server.address().port}/`;
  const browser = await chromium.launch();
  const results = [];
  const rec = (ok, label, note) => { const l = `${ok ? 'PASS' : 'FAIL'} ${label}: ${note}`; results.push(l); console.log(l); };
  const open = async (W = 1280, H = 720, q = '?low&debug') => {
    const page = await browser.newPage({ viewport: { width: W, height: H } });
    await page.goto(base + q);
    for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) { if (Date.now() - t0 > 180000) throw new Error('no boot'); await sleep(250); }
    await sleep(1500);
    const st = () => page.evaluate(() => { const s = window.__game.state() || {}; return { yaw: s.yaw, pitch: s.pitch, locked: s.locked, frame: s.frame, paused: s.paused, casts: s.casts }; });
    const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
    const frames = async (n) => { const f = (await st()).frame; for (let t0 = Date.now(); (await st()).frame < f + n && Date.now() - t0 < 60000;) await sleep(30); };
    const lockResume = async () => {
      if (!(await st()).paused) await page.evaluate(() => window.__game.pause(true));
      await sleep(400); await page.click('#resume');
      for (let k = 0; k < 60 && !(await st()).locked; k++) await sleep(100);
      return (await st()).locked;
    };
    const measure = async (fn) => {
      await cmd('look 0 10'); await frames(5);
      const a = await st(); await fn(); await frames(10); const b = await st();
      return { dy: ((((b.yaw - a.yaw) % 360) + 540) % 360) - 180, dp: b.pitch - a.pitch, locked: b.locked };
    };
    return { page, st, cmd, frames, lockResume, measure };
  };
  const syn = (P, list) => P.page.evaluate((list) => {
    const g = document.getElementById('game');
    for (const [t, a, b] of list) {
      if (t === 'm') g.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse', movementX: a, movementY: b }));
      else document.dispatchEvent(new PointerEvent(t === 'd' ? 'pointerdown' : 'pointerup', { bubbles: true, button: 0, pointerType: 'mouse', clientX: 489, clientY: 645 }));
    }
  }, list);

  // A: headless "fast click": mouse.click to a new spot (its move, press and release land together).
  {
    const P = await open(); const ok = await P.lockResume(); console.log('locked', ok);
    await P.cmd('select 2'); // water: avoid lighting things
    const pts = [[640, 360], [300, 200], [900, 500], [640, 360], [200, 600], [1000, 150]];
    let fails = 0;
    for (const [x, y] of pts) {
      const r = await P.measure(async () => { await P.page.mouse.click(x, y); });
      const pass = Math.abs(r.dy) < 0.5 && Math.abs(r.dp) < 0.5; if (!pass) fails++;
      console.log(`  click(${x},${y}) dyaw=${r.dy.toFixed(2)} dpitch=${r.dp.toFixed(2)} locked=${r.locked}`);
    }
    rec(fails === 0, 'A headless mouse.click to a new spot under lock does not turn', `${fails}/${pts.length} turned`);
    // A2: move then 2 frames then click (golden real click pattern), x4
    let f2 = 0;
    for (const [x, y] of [[640, 360], [300, 200], [900, 500], [640, 360]]) {
      const r = await P.measure(async () => { await P.page.mouse.move(x, y); await P.frames(2); await P.page.mouse.click(x, y); });
      const pass = Math.abs(r.dy) < 0.5 && Math.abs(r.dp) < 0.5; if (!pass) f2++;
      console.log(`  move+2f+click(${x},${y}) dyaw=${r.dy.toFixed(2)} dpitch=${r.dp.toFixed(2)}`);
    }
    rec(f2 === 0, 'A2 golden real-click pattern (move, 2 frames, click) does not turn', `${f2}/4 turned`);
    // B: genuine flicks right after a press/release (synthetic, real-mouse-like)
    const cases = [
      ['B1 flick -150,-100 after press+release', [['d'], ['u'], ['m', -150, -100]], -150, -100],
      ['B2 flick +200,+50 after press', [['d'], ['m', 200, 50], ['u']], 200, 50],
      ['B3 small right move (3,1), click, then back (-3,-1) x3', [['m', 3, 1], ['d'], ['u'], ['m', -3, -1], ['m', -3, -1], ['m', -3, -1]], -6, -2],
      ['B4 aim right (40,0), click, flick back (-40,0)', [['m', 40, 0], ['d'], ['u'], ['m', -40, 0]], 0, 0],
      ['B5 aim down-right (120,80), press, first held move (-120,-80)', [['m', 120, 80], ['d'], ['m', -120, -80], ['u']], 0, 0],
    ];
    for (const [label, list, ex, ey] of cases) {
      const r = await P.measure(() => syn(P, list));
      const wy = -ex * LPP, wp = -ey * LPP;
      const ok2 = Math.abs(r.dy - wy) < 0.5 && Math.abs(r.dp - wp) < 0.5;
      rec(ok2, label, `expected dyaw ${wy.toFixed(2)} dpitch ${wp.toFixed(2)}, got ${r.dy.toFixed(2)} / ${r.dp.toFixed(2)}`);
    }
    await P.page.close();
  }
  // C: small window, centre candidate vs a genuine max flick
  {
    const P = await open(400, 300); const ok = await P.lockResume(); console.log('small locked', ok);
    const r = await P.measure(() => syn(P, [['d'], ['u'], ['m', -200, -150]]));
    rec(Math.abs(r.dy - 200 * LPP) < 0.5, 'C 400x300 window: genuine flick (-200,-150) after a click', `expected dyaw ${(200 * LPP).toFixed(2)}, got ${r.dy.toFixed(2)} / ${r.dp.toFixed(2)}`);
    await P.page.close();
  }
  // D: age picker still shows, picks, and the game locks and clicks without turning afterwards
  {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    await page.goto(base + '?low&debug&age=on');
    for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState));) { if (Date.now() - t0 > 180000) throw new Error('no boot'); await sleep(250); }
    await sleep(1000);
    const shown = await page.evaluate(() => { const r = document.getElementById('age'); return !!r && !r.hidden; });
    let picked = false;
    if (shown) { await page.click('button[data-band]'); await sleep(800); picked = await page.evaluate(() => document.getElementById('age').hidden); }
    rec(shown && picked, 'D age picker shows and closes on a pick', `shown=${shown} closedAfterPick=${picked}`);
    await page.close();
  }
  console.log('\n' + results.join('\n'));
  await browser.close(); server.close();
})().catch((e) => { console.error(e); process.exit(2); });
