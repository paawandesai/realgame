// Chemistry quests in the block world (I-057, docs/loop/items/I-057/spec.md):
// a Bench find saved as a spell (formula.spells.v1, as the Bench page writes
// it) loads into the world, its bottles come from a stash in the valley (no
// `give`), and one cast near the hidden mark finishes the reveal quest. The
// quest's state is written back to the shared save.
//
//   bash crates/bevy-client/build-web.sh        # builds web/pkg-out
//   node tests/e2e/chem-quests.mjs              # serves web/pkg-out itself
//
// Env: BASE_URL (a running server instead), QUERY (for example `low`), SHOTS
// (a directory for screenshots). Headless frames are slow on a software GPU
// (about 0.4 fps in the world), so every step waits on game state with long
// timeouts. Needs `playwright` with Chromium. Exits 1 if any check fails.
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

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.json': 'application/json' };
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

const SPELL = 'iron_copper_sulfate';
const QUEST = 'reveal_the_hidden_mark';

(async () => {
  const server = process.env.BASE_URL ? null : await serve();
  const q = (process.env.QUERY || '').replace(/^\?/, '');
  const site = process.env.BASE_URL || `http://127.0.0.1:${server.address().port}/`;
  const url = site + `?${q ? q + '&' : ''}debug`;
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const errors = [];
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); if (/panicked/.test(m.text())) console.log('  console:', m.text().slice(0, 240)); });
  page.on('pageerror', (e) => { errors.push(e.message); console.log('  pageerror:', e.message); });
  // The Bench page saved the find as a spell and built it twice (mastered).
  await page.addInitScript(([spell]) => {
    try {
      if (!localStorage.getItem('formula.spells.v1')) {
        localStorage.setItem('formula.bench.found', JSON.stringify([spell]));
        localStorage.setItem('formula.spells.v1', JSON.stringify({ v: 1, saved: { [spell]: 2 }, slots: [spell, null, null, null], quests: [] }));
      }
    } catch { /* storage blocked: the checks below fail */ }
  }, [SPELL]);
  await page.goto(url);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState && !!window.__game && !!window.__discState));) {
    if (Date.now() - t0 > 180000) throw new Error('the world did not start within 180 s');
    await sleep(500);
  }
  const st = () => page.evaluate(() => window.__game.state());
  const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
  const disc = () => page.evaluate(() => JSON.parse(window.__discState || 'null'));
  const discCmd = (c) => page.evaluate((c) => { window.__gameIn = window.__gameIn || {}; (window.__gameIn.disc = window.__gameIn.disc || []).push(c); }, c);
  const poll = async (get, pred, ms) => {
    const t0 = Date.now();
    let x = await get();
    while (!(x && pred(x)) && Date.now() - t0 < ms) { await sleep(500); x = await get(); }
    return x;
  };
  const packN = (s, item) => ((s && s.pack) || []).filter((e) => e.item === item).reduce((n, e) => n + e.n, 0);
  const shot = async (name) => { if (shotDir) await page.screenshot({ path: `${shotDir}${name}`, timeout: 240000 }); };

  // Close the intro (as golden.mjs does).
  for (const t0 = Date.now(); Date.now() - t0 < 30000; await sleep(300)) {
    const open = await page.evaluate(() => { const e = document.getElementById('intro'); return !!e && !e.hidden; });
    if (open) { await page.click('#intro .intro-play', { timeout: 5000 }).catch(() => {}); break; }
    if (((await st()) || {}).quest > 0) break;
  }
  await poll(st, (x) => x.quest > 0 && !x.paused, 60000);

  // Q1. The save loaded: the spell is saved, built twice, on key 1; five
  // live quests, the reveal quest not done, each with a hint.
  let D = await poll(disc, (x) => x.loaded && x.saved.some((s) => s.id === SPELL), 60000);
  const row = D && D.saved.find((s) => s.id === SPELL);
  check('quests: the shared save loads the spell and the five quests',
    row && row.builds === 2 && row.effect.runs === 'reveal' && D.quests.length === 5
      && D.quests.every((x) => x.line && x.hint) && !D.quests.find((x) => x.id === QUEST).done,
    `row=${JSON.stringify(row && { builds: row.builds, runs: row.effect.runs, key: row.key })} quests=${D && D.quests.map((x) => x.id).join(',')}`);
  await discCmd(`equip ${SPELL} 1`);
  D = await poll(disc, (x) => x.on[0] && x.slots[0] === SPELL, 30000);
  // Without bottles it cannot be cast.
  const plan0 = D && D.saved.find((s) => s.id === SPELL);
  check('quests: with no bottles the spell says what it needs and where it is found',
    plan0 && !plan0.bottles && plan0.missing.includes('CuSO4') && plan0.uses.some((u) => u.species === 'CuSO4' && /workshop/.test(u.find || '')),
    `missing=${plan0 && plan0.missing} uses=${JSON.stringify(plan0 && plan0.uses.map((u) => [u.species, u.find]))}`);

  // Q2. Walking up to the stash by the workshop gives copper sulfate and iron.
  await cmd('tp 137.6 35 190.5');
  let s = await poll(st, (x) => packN(x, 'copper_sulfate') >= 4 && packN(x, 'iron') >= 4, 120000);
  check('quests: the workshop stash fills the pack with copper sulfate and iron', packN(s, 'copper_sulfate') === 4 && packN(s, 'iron') === 4,
    `pack=${JSON.stringify(s && s.pack)}`);

  // Q3. One cast near the hidden mark (west of the village) reveals it and
  // finishes the quest; the cast uses one of each bottle.
  await cmd('tp 105.5 35 200.5');
  await sleep(1500);
  await cmd('face 103.5 35 200.5');
  await cmd('select 1');
  await poll(st, (x) => x.spell === 0, 30000);
  const casts0 = (await st()).casts;
  await cmd('cast');
  D = await poll(disc, (x) => x.quests.find((q) => q.id === QUEST).done, 240000);
  s = await st();
  const done = D && D.quests.find((x) => x.id === QUEST).done;
  check('quests: a reveal spell near the hidden mark finishes the reveal quest',
    done && D.save.quests.includes(QUEST) && s.casts > casts0 && packN(s, 'copper_sulfate') === 3 && packN(s, 'iron') === 3,
    `done=${done} casts ${casts0}->${s.casts} pack=${JSON.stringify(s.pack)} caption=${s.caption}`);
  await shot('chem-quest-reveal.png');

  // Q4. The quest is in the shared save (localStorage), as the Bench reads it.
  const saved = await poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('formula.spells.v1') || 'null')), (x) => (x.quests || []).includes(QUEST), 60000);
  check('quests: the shared save records the quest', saved && saved.quests.includes(QUEST) && saved.saved[SPELL] === 2, JSON.stringify(saved));

  // Q5. The revealed mark can now be picked up.
  await cmd('tp 101.6 35 200.5');
  s = await poll(st, (x) => packN(x, 'copper_sulfate') >= 6, 120000);
  check('quests: the revealed hidden mark gives more bottles', packN(s, 'copper_sulfate') === 6 && packN(s, 'chalk') === 3, `pack=${JSON.stringify(s && s.pack)}`);

  // Q6. A reload keeps the quest done (the save round trip in the browser).
  await page.reload();
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__discState));) {
    if (Date.now() - t0 > 180000) break;
    await sleep(500);
  }
  D = await poll(disc, (x) => x.loaded && x.quests.find((q) => q.id === QUEST).done, 120000);
  check('quests: after a reload the quest is still done', D && D.quests.find((x) => x.id === QUEST).done, '');

  const real = errors.filter((e) => !/favicon|DevTools|GPU stall|WebGL|404/.test(e));
  check('quests: no page errors', real.length === 0, real.slice(0, 3).join(' | '));
  await browser.close();
  if (server) server.close();
  const failed = results.filter((r) => !r.ok);
  console.log(`${results.length - failed.length}/${results.length} passed`);
  process.exit(failed.length ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });
