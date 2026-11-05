// The one Bench (I-060, docs/loop/items/I-060/spec.md): B in the block world
// opens the Chemistry Bench over the world in story mode (its bottles are the
// pack's and the air's), Fire is learned there by building its formula (judged
// by the game's bench.rs), closing it returns to the world with Fire learned
// on the hotbar, and the title screen's Chemistry Bench is free play and shows
// the same progress from the shared save.
//
//   bash crates/bevy-client/build-web.sh        # builds web/pkg-out
//   node tests/e2e/one-bench.mjs                # serves web/pkg-out itself
//
// Env: BASE_URL (a running server instead), SHOTS (a directory for
// screenshots). Headless frames are slow on a software GPU, so every step
// waits on state with long timeouts. Needs `playwright` with Chromium
// (NODE_PATH=$(npm root -g)). Exits 1 if any check fails.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '../../crates/bevy-client/web/pkg-out');
const shotDir = process.env.SHOTS ? process.env.SHOTS.replace(/\/?$/, '/') : null;

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.json': 'application/json', '.svg': 'image/svg+xml' };
async function serve() {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
    if (path.startsWith('..')) { res.writeHead(403).end(); return; }
    try {
      const body = await readFile(join(root, path || 'index.html'));
      res.writeHead(200, { 'content-type': types[extname(path || 'index.html')] || 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  return server;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok: !!ok });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name} ${detail}`);
}

(async () => {
  const server = process.env.BASE_URL ? null : await serve();
  const site = process.env.BASE_URL || `http://127.0.0.1:${server.address().port}/`;
  const browser = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-webgl', '--ignore-gpu-blocklist'] });
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => { errors.push(e.message); console.log('  pageerror:', e.message); });
  page.on('console', (m) => { if (/panicked/.test(m.text())) console.log('  console:', m.text().slice(0, 240)); });
  const poll = async (get, pred, ms) => {
    const t0 = Date.now();
    let x = await get().catch(() => null);
    while (!(x && pred(x)) && Date.now() - t0 < ms) { await sleep(500); x = await get().catch(() => null); }
    return x;
  };
  const shot = async (p, name) => { if (shotDir) await p.screenshot({ path: `${shotDir}${name}`, timeout: 240000 }).catch(() => {}); };

  // The block world at Seedling (the URL fixes the band, as a class link does).
  await page.goto(`${site}?debug&band=seedling`);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState && !!window.__game && !!window.__bench && !!window.__discState));) {
    if (Date.now() - t0 > 240000) throw new Error('the world did not start within 240 s');
    await sleep(500);
  }
  const st = () => page.evaluate(() => window.__game.state());
  const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
  for (const t0 = Date.now(); Date.now() - t0 < 30000; await sleep(300)) {
    const title = await page.evaluate(() => document.body.classList.contains('title-open'));
    if (title) { await page.click('.title-play', { timeout: 5000 }).catch(() => {}); continue; }
    const open = await page.evaluate(() => { const e = document.getElementById('intro'); return !!e && !e.hidden; });
    if (open) { await page.click('#intro .intro-play', { timeout: 5000 }).catch(() => {}); break; }
    if (((await st()) || {}).quest > 0) break;
  }
  await poll(st, (x) => x.quest > 0 && !x.paused, 90000);

  // Pack bottles: walking up to the workshop stash gives copper sulfate and iron.
  const packN = (s, item) => ((s && s.pack) || []).filter((e) => e.item === item).reduce((n, e) => n + e.n, 0);
  await cmd('tp 137.6 35 190.5');
  // (The stash gives its bottles once the world has loaded round the new spot: slow in software rendering.)
  const s0 = await poll(st, (x) => packN(x, 'copper_sulfate') >= 4 && packN(x, 'iron') >= 4, 360000);
  check('setup: the workshop stash puts bottles in the pack', packN(s0, 'copper_sulfate') >= 4 && packN(s0, 'iron') >= 4, JSON.stringify(s0 && s0.pack));

  // B1. B in the world opens the one Bench over it, in story mode.
  await page.mouse.click(640, 360);
  await sleep(500);
  await page.keyboard.press('b');
  const ui = () => page.evaluate(() => window.__bench.ui());
  let u = await poll(ui, (x) => x.open && x.ready, 180000);
  const frame = await (await page.$('#bench-frame'))?.contentFrame();
  const fb = (fn, arg) => frame.evaluate(fn, arg);
  // A real mouse click on a control inside the Bench's frame, at its centre
  // (Playwright's frame locators wait on the frame's animation frames, which
  // the software-rendered world starves in this headless run).
  const fclick = async (sel) => {
    const at = await poll(() => fb((q) => { const e = document.querySelector(q); if (!e) return null; e.scrollIntoView({ block: 'nearest' }); const fr = window.frameElement.getBoundingClientRect(); const b = e.getBoundingClientRect(); return b.width ? { x: fr.x + b.x + b.width / 2, y: fr.y + b.y + b.height / 2 } : null; }, sel), (x) => !!x, 30000);
    if (!at) throw new Error(`no ${sel} in the Bench`);
    await page.mouse.click(at.x, at.y);
  };
  await poll(() => fb(() => !!window.__benchFree), (x) => x, 60000);
  const shown = await page.evaluate(() => ({
    overlay: !document.getElementById('bench').hidden,
    paused: !!window.__game.state().paused,
    lock: !!document.pointerLockElement,
    inert: ['stage', 'hud'].map((id) => document.getElementById(id)?.inert),
  }));
  const mode = await fb(() => ({ mode: window.__benchFree.mode, chip: document.getElementById('modeChip').dataset.mode, chipText: document.getElementById('modeChip').textContent, jars: !document.getElementById('elementBox').hidden }));
  check('B1: B opens the one Bench over the world in story mode (paused, mouse free, world inert)',
    u && u.open && shown.overlay && shown.paused && !shown.lock && shown.inert.every(Boolean) && mode.mode === 'story' && mode.chip === 'story' && /pack/.test(mode.chipText) && !mode.jars,
    `ui=${JSON.stringify(u)} shown=${JSON.stringify(shown)} mode=${JSON.stringify(mode)}`);
  const bottles = await poll(() => fb(() => [...document.querySelectorAll('#bottles .bottle')].map((b) => ({ f: b.dataset.formula, have: b.querySelector('.have')?.textContent || '' }))), (x) => x.some((b) => b.f === 'CuSO4'), 30000);
  const allowed = new Set(['O2', 'N2', 'CuSO4', 'Fe']);
  check('B1: its bottles are the pack\'s and the air\'s',
    bottles && bottles.length === 4 && bottles.every((b) => allowed.has(b.f)) && /×4/.test(bottles.find((b) => b.f === 'CuSO4')?.have || '') && /air/.test(bottles.find((b) => b.f === 'O2')?.have || ''),
    JSON.stringify(bottles));
  // A reaction needing something the pack does not hold says where to find it.
  const gate = await fb(async () => { await window.__benchFree.run('CH4 + 2O2'); const r = document.getElementById('result'); return r.className + ' ' + r.innerText; });
  check('B1: a bottle the pack does not hold is not on the shelf, and running it says so', /bad/.test(gate) && /pack/.test(gate), gate.slice(0, 160));
  // Iron and copper sulfate react on the story Bench without using the bottles up.
  const sBefore = await st();
  const ran = await fb(async () => { await window.__benchFree.run('Fe + CuSO4'); return document.getElementById('result').innerText; });
  const s1 = await st();
  check('B1: a pack reaction runs, and running it on the Bench uses nothing up', /copper/i.test(ran) && packN(s1, 'copper_sulfate') === packN(sBefore, 'copper_sulfate') && packN(s1, 'copper_sulfate') >= 4, `${ran.slice(0, 100)} pack=${packN(s1, 'copper_sulfate')} was=${packN(sBefore, 'copper_sulfate')} setup=${packN(s0, 'copper_sulfate')}`);
  await shot(page, 'one-bench-story.png');

  // B2. Learn Fire by building its formula in the Spells tab (Seedling: the scale).
  await fb(() => window.__benchFree.showTab('spells'));
  const fire = await poll(() => fb(() => { const c = document.querySelector('.sb-spell[data-spell="fire"]'); return c ? { stage: c.querySelector('.sb-stage')?.dataset.stage, learn: !!c.querySelector('.sb-learn') } : null; }), (x) => x.learn, 30000);
  check('B2: the Spells tab lists Fire as New with Learn', fire && fire.stage === 'unknown' && fire.learn, JSON.stringify(fire));
  await fclick('.sb-spell[data-spell="fire"] .sb-learn');
  const tiles = await poll(() => fb(() => document.querySelectorAll('.sb-mount .bench-coef').length), (n) => n === 4, 60000);
  check('B2: the Seedling builder shows Fire\'s four coefficient tiles', tiles === 4, `tiles=${tiles}`);
  // A wrong build first: the judge says why, and nothing is learned.
  await fclick('.sb-mount .bench-make');
  const fizz = await poll(() => fb(() => document.querySelector('.sb-fizzle')?.innerText || ''), (t) => t.length > 0, 60000);
  check('B2: an unbalanced build fizzles with a reason', /level|atoms/i.test(fizz), fizz);
  await fclick('.sb-mount .bench-coef[data-term="1"]');
  await fclick('.sb-mount .bench-coef[data-term="3"]');
  await fclick('.sb-mount .bench-make');
  const done = await poll(() => fb(() => document.querySelector('.sb-done')?.innerText || ''), (t) => t.length > 0, 60000);
  const bs = await page.evaluate(() => JSON.parse(window.__benchState));
  const ws = await page.evaluate(() => JSON.parse(window.__workState));
  check('B2: the right build learns Fire (the game judged it)', /right|learn/i.test(done) && ws.stages[0] !== 'unknown' && (bs.spells || []).find((c) => c.id === 'fire')?.stage === 'learning', `done=${JSON.stringify(done.slice(0, 120))} stages=${ws.stages}`);
  const count = await poll(() => fb(() => document.querySelector('.sb-done-count')?.textContent || ''), (t) => /of \d+ builds/.test(t), 20000);
  check('B2: the build counts toward Practised', /1 of 3 builds/.test(count), count);
  await shot(page, 'one-bench-fire.png');

  // B3. Back to the world: the Bench closes, the world runs, Fire is learned on the hotbar.
  await fclick('#toGame');
  u = await poll(ui, (x) => !x.open, 30000);
  const back = await poll(st, (x) => !x.paused, 30000);
  const slot = await poll(() => page.evaluate(() => document.querySelector('#hotbar .slot[data-spell="1"] span.stage')?.getAttribute('data-stage') || ''), (x) => x === 'learning', 30000);
  check('B3: closing returns to the world, unpaused, with Fire learned on the hotbar',
    !u.open && back && !back.paused && slot === 'learning' && (await page.evaluate(() => document.getElementById('bench').hidden)),
    `ui=${JSON.stringify(u)} paused=${back && back.paused} slot=${slot}`);
  // Escape closes it too (the frame has focus while it is open).
  await page.keyboard.press('b');
  await poll(ui, (x) => x.open, 30000);
  await page.keyboard.press('Escape');
  u = await poll(ui, (x) => !x.open, 30000);
  check('B3: Escape inside the Bench closes it', !u.open, JSON.stringify(u));
  // The save the free-play Bench reads: the world writes Fire's progress.
  await poll(() => page.evaluate(() => localStorage.getItem('rg.progress.v1') || ''), (x) => /fire:1/.test(x), 30000);

  // B4. The title screen's Chemistry Bench is free play, with the same save.
  // (The world page closes first: two software-rendered worlds starve the
  // headless run.)
  await page.close();
  const title = await context.newPage();
  title.on('pageerror', (e) => { errors.push(e.message); console.log('  pageerror:', e.message); });
  await title.goto(`${site}?title`);
  await poll(() => title.evaluate(() => !!document.querySelector('.title-bench')), (x) => x, 120000);
  const href = await title.evaluate(() => document.querySelector('.title-bench').getAttribute('href'));
  await title.click('.title-bench', { timeout: 120000 });
  await title.waitForURL(/bench-free\.html/, { timeout: 60000 });
  await poll(() => title.evaluate(() => !!window.__benchFree), (x) => x, 120000);
  const free = await title.evaluate(() => ({
    mode: window.__benchFree.mode,
    chip: document.getElementById('modeChip').dataset.mode,
    chipText: document.getElementById('modeChip').textContent,
    jars: !document.getElementById('elementBox').hidden,
    bottles: document.querySelectorAll('#bottles .bottle').length,
  }));
  await title.evaluate(() => window.__benchFree.showTab('spells'));
  const fireFree = await poll(() => title.evaluate(() => document.querySelector('.sb-spell[data-spell="fire"] .sb-stage')?.dataset.stage || ''), (x) => x, 20000);
  check('B4: the title screen\'s Chemistry Bench is free play with every bottle',
    href.startsWith('bench-free.html') && !/mode=story/.test(href) && free.mode === 'free' && free.chip === 'free' && /Free play/.test(free.chipText) && free.jars && free.bottles > 10,
    JSON.stringify({ href, ...free }));
  check('B4: free play shows Fire as learned, from the one save', fireFree === 'learning', fireFree);
  await shot(title, 'one-bench-free.png');

  check('no page errors', errors.length === 0, errors.slice(0, 3).join(' | '));
  await browser.close();
  server?.close();
  const failed = results.filter((r) => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} passed`);
  process.exit(failed.length ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });
