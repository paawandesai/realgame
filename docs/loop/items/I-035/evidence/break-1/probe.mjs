// Breaker probes for I-035 level 2. Usage: node probe.mjs <pkg-out dir>
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire('/opt/node-tools/node_modules/');
const { chromium } = require('playwright');
const root = process.argv[2];
const shots = process.argv[3];
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm' };
const server = createServer(async (req, res) => {
  const p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^[/\\]+/, '');
  try { const b = await readFile(join(root, p || 'index.html')); res.writeHead(200, { 'content-type': types[extname(p || 'index.html')] || 'application/octet-stream' }); res.end(b); }
  catch { res.writeHead(404).end(); }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const origin = `http://127.0.0.1:${server.address().port}/`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const out = [];
const log = (...a) => { const s = a.map((x) => typeof x === 'string' ? x : JSON.stringify(x)).join(' '); out.push(s); console.log(s); };
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
const errors = []; const logs = [];
page.on('console', (m) => { const t = m.text(); logs.push(t); if (m.type() === 'error') errors.push(t); if (/panicked|LEVEL/.test(t)) console.log('  console:', t.slice(0, 200)); });
page.on('pageerror', (e) => errors.push(e.message));
const st = () => page.evaluate(() => window.__game.state());
const cmd = (c) => page.evaluate((c) => window.__game.cmd(c), c);
const until = async (pred, ms = 30000) => { const t0 = Date.now(); let x = await st(); while (!pred(x) && Date.now() - t0 < ms) { await sleep(150); x = await st(); } return x; };
const gameSecs = async (secs) => { const t0 = (await st()).gameTime; return until((x) => x.gameTime >= t0 + secs, 60000 + secs * 4000); };
const load = async (q) => {
  await page.goto(origin + q);
  for (const t0 = Date.now(); !(await page.evaluate(() => !!window.__gameState && !!window.__game));) { if (Date.now() - t0 > 120000) throw new Error('no start'); await sleep(250); }
  await gameSecs(0.3);
};
const idx = (x, n) => x.objs.findIndex((o) => o.n === n);
const resume = async () => { if ((await st()).paused) { await page.evaluate(() => window.__game.pause(false)); await until((q) => !q.paused, 8000); } };
const castAt = async (i, spell) => {
  await resume();
  await cmd(`face ${i}`); await cmd(`select ${spell}`);
  const a = await until((x) => x.spell === spell - 1 && x.aimObj === i && x.cooldown <= 0, 15000);
  await cmd('cast');
  const z = await until((x) => x.impacts > a.impacts && x.captionId > a.captionId, 30000);
  return z;
};
const obj = (x, n) => x.objs[idx(x, n)];
const hintText = () => page.evaluate(() => document.getElementById('hint')?.textContent || '');
const bench = () => page.evaluate(() => { try { return JSON.parse(window.__benchState); } catch { return null; } });
const pushBand = async (id) => { await page.evaluate((c) => { window.__gameIn = window.__gameIn || {}; (window.__gameIn.bench = window.__gameIn.bench || []).push(c); }, `band ${id}`); await sleep(800); };
const shot = async (n) => { if (shots) await page.screenshot({ path: `${shots}/${n}`, timeout: 180000 }); };

try {
  // P1: ?level=2 without debug, and `level` refused without debug
  await load('?low&level=2');
  let s = await until((x) => x.level === 2, 20000);
  log('P1 ?level=2 no debug:', { level: s.level, caption: s.caption, names: s.objs.map((o) => o.n), outcome: s.outcome, burning: s.burning, exitOpen: s.exitOpen });
  await cmd('level 1'); await gameSecs(0.5); s = await st();
  log('P1b level 1 without debug -> level', s.level);
  for (const q of ['?low&level=3', '?low&level=abc', '?low&level=']) { await load(q); s = await st(); log('P1c', q, 'level', s.level, 'burning', s.burning); }

  // P2: try to get past the stream without freezing
  await load('?low&debug&level=2');
  s = await until((x) => x.level === 2, 20000);
  await cmd('look 0 0');
  await cmd('move 0 1 4'); await gameSecs(4.5); s = await st();
  log('P2 walk into liquid stream: pos', s.pos);
  // jump spam while running forward
  await page.click('#game').catch(() => {});
  await cmd('tp 0 9'); await cmd('look 0 0');
  await cmd('move 0 1 3');
  for (let k = 0; k < 8; k++) { await page.keyboard.down('Space'); await sleep(120); await page.keyboard.up('Space'); await sleep(250); }
  await gameSecs(1); s = await st(); log('P2b jump spam into stream: pos', s.pos);
  // diagonal at the gap edge
  await cmd('tp 2.05 9'); await cmd('look -10 0'); await cmd('move 0.2 1 3'); await gameSecs(3.5); s = await st(); log('P2c diagonal at gap edge: pos', s.pos);
  // around the banks
  await cmd('tp 12 11'); await cmd('look 0 0'); await cmd('move 0 1 3'); await gameSecs(3.5); s = await st(); log('P2d rocks east: pos', s.pos);
  await cmd('tp 23.6 11'); await cmd('move 0 1 3'); await gameSecs(3.5); s = await st(); log('P2e east wall corner: pos', s.pos);
  // gate from north landing
  await cmd('tp 0 3'); await cmd('move 0 1 3'); await gameSecs(3.5); s = await st(); log('P2f gate blocks: pos', s.pos);
  await cmd('tp 2.3 3'); await cmd('move 0.1 1 3'); await gameSecs(3.5); s = await st(); log('P2g gate edge: pos', s.pos);
  await cmd('tp 23.6 -5'); await cmd('move 0 1 3'); await gameSecs(3.5); s = await st(); log('P2h ridge east corner: pos', s.pos);
  await cmd('move 0 0 0');

  // P3: partial freeze then fire (caption on a 1/3-frozen stream), per band
  await load('?low&debug&level=2'); s = await until((x) => x.level === 2, 20000);
  const ST = idx(s, 'stream'), GT = idx(s, 'ice gate'), LT = idx(s, 'lantern');
  await pushBand('sprout');
  s = await castAt(ST, 3); log('P3 sprout frost1:', obj(s, 'stream').ice, s.caption, s.capIcon, 'beads', s.fxBeads);
  s = await castAt(ST, 1); log('P3 sprout fire on 1/3 ice:', obj(s, 'stream').ice, s.caption, s.capIcon);
  s = await castAt(ST, 1); log('P3 sprout fire on liquid:', obj(s, 'stream').ice, s.caption, s.capIcon);
  s = await castAt(ST, 4); log('P3 sprout co2 stream:', s.caption, s.capIcon);
  s = await castAt(ST, 2); log('P3 sprout water stream:', s.caption, s.capIcon);
  for (const sp of [1, 2, 3, 4]) { s = await castAt(LT, sp); log('P3 sprout lantern spell', sp, ':', s.caption, '| icon', s.capIcon); }
  await pushBand('seedling');
  s = await castAt(LT, 2); log('P3 seedling water lantern:', s.caption);
  await pushBand('adept');
  s = await castAt(ST, 1); log('P3 adept fire liquid:', s.caption);

  // P4: freeze, walk out on the bridge, slide sideways, melt under the player
  await load('?low&debug&level=2'); s = await until((x) => x.level === 2, 20000);
  for (let k = 0; k < 3; k++) s = await castAt(ST, 3);
  log('P4 frozen:', obj(s, 'stream').ice, obj(s, 'stream').frozen, s.caption);
  await cmd('tp 0 6'); await gameSecs(0.3); await cmd('look 90 0'); await cmd('move 0 1 3'); await gameSecs(3.5); s = await st();
  log('P4 slide sideways on ice: pos', s.pos, 'onIce', s.onIce);
  const p0 = s.pos;
  // cast fire straight down at the stream from here
  await cmd('select 1'); await cmd('look 0 -60');
  const a = await until((x) => x.cooldown <= 0, 8000);
  await cmd('cast'); s = await until((x) => x.impacts > a.impacts, 20000); await gameSecs(0.5); s = await st();
  log('P4 fire on bridge under player: ice', obj(s, 'stream').ice, 'pos', s.pos, 'from', p0, 'caption', s.caption);
  await cmd('look 0 0'); await cmd('move 0 1 2'); await gameSecs(2.5); s = await st(); log('P4 after push, try walking -z: pos', s.pos);
  await cmd('move 0 -1 2'); await gameSecs(2.5); s = await st(); log('P4 after push, walk +z: pos', s.pos);

  // P5: stand in the middle of the bridge at the landing; fire from the other side
  await load('?low&debug&level=2'); s = await until((x) => x.level === 2, 20000);
  for (let k = 0; k < 3; k++) s = await castAt(ST, 3);
  await cmd('tp 0 5.9'); await gameSecs(0.3);
  s = await castAt(ST, 1); await gameSecs(0.3); s = await st();
  log('P5 middle of bridge, fire: pos', s.pos, 'ice', obj(s, 'stream').ice);

  // P6: melt gate, frost the puddle, and walk to the lantern; check banner, R, back arch
  await load('?low&debug&level=2'); s = await until((x) => x.level === 2, 20000);
  for (let k = 0; k < 3; k++) s = await castAt(ST, 3);
  await cmd('tp 0 2'); await gameSecs(0.3);
  for (let k = 0; k < 3; k++) s = await castAt(GT, 1);
  log('P6 gate:', obj(s, 'ice gate'), s.caption);
  s = await castAt(GT, 3); log('P6 frost on melted gate:', obj(s, 'ice gate'), s.caption);
  await cmd('look 0 0'); await cmd('move 0 1 6');
  s = await until((x) => x.outcome === 'won', 12000);
  log('P6 lantern reached: outcome', s.outcome, 'pos', s.pos, 'caption', s.caption);
  await gameSecs(1);
  log('P6 banner', await page.evaluate(() => ({ hidden: document.getElementById('banner').classList.contains('hidden'), big: document.querySelector('#banner .big').textContent, small: document.querySelector('#banner .small').textContent })));
  await shot('p6-won.png');
  s = await castAt(ST, 1); log('P6 cast after win: ice', obj(s, 'stream').ice);
  await page.keyboard.press('KeyR'); await gameSecs(0.5); s = await st();
  log('P6 R in level 2 -> level', s.level, 'outcome', s.outcome, 'pos', s.pos, 'stream ice', obj(s, 'stream')?.ice, 'gate wall', obj(s, 'ice gate')?.wall, s.caption);

  // P7: bench in level 2 and back; hint text
  await cmd('tp -3 14.5'); await gameSecs(0.5);
  log('P7 bench atBench in level 2 at pad:', (await bench())?.atBench);
  await page.mouse.click(640, 360); await until((x) => x.locked, 8000); await gameSecs(1);
  s = await st(); log('P7 hint in level 2 while locked:', JSON.stringify(await hintText()), 'locked', s.locked);
  await page.keyboard.press('Escape'); await sleep(500); await resume();
  await cmd('tp 0 21.8'); s = await until((x) => x.level === 1, 8000); await gameSecs(0.5); s = await st();
  log('P7 back arch -> level', s.level, 'burning', s.burning, 'exitOpen', s.exitOpen, 'pos', s.pos);
  await cmd('tp -3 14.5'); await gameSecs(0.5);
  log('P7 bench atBench in level 1 at pad:', (await bench())?.atBench);
  await cmd('tp 0 18');

  // P8: ping-pong while holding back
  await load('?low&debug'); s = await until((x) => x.level === 1 && x.burning === 3, 20000);
  for (const o of s.objs.filter((o) => o.burning)) await castAt(o.i, 2);
  s = await until((x) => x.outcome === 'won', 15000);
  await cmd('tp 0 19'); await cmd('look 0 0');
  const n0 = logs.filter((l) => /LEVEL \d/.test(l)).length;
  await cmd('move 0 -1 6'); await gameSecs(6.5);
  const switches = logs.filter((l) => /LEVEL \d/.test(l)).slice(n0);
  s = await st(); log('P8 holding back 6 s from z=19 after win: switches', switches.length, 'now level', s.level, 'pos', s.pos);

  // P9: switch levels with a projectile in flight and a held water stream
  await load('?low&debug'); s = await until((x) => x.level === 1 && x.burning === 3, 20000);
  const b0 = s.objs.filter((o) => o.burning)[0].i;
  await cmd(`face ${b0}`); await cmd('select 1'); await until((x) => x.cooldown <= 0, 8000);
  await cmd('cast'); await cmd('level 2'); await gameSecs(1.5); s = await st();
  log('P9 level switch mid-cast: level', s.level, 'objs', s.objs.length, 'errors', errors.length);
  await cmd('level 1'); await gameSecs(0.5);
  await cmd(`face ${b0}`); await cmd('select 2'); await until((x) => x.cooldown <= 0, 8000);
  await page.mouse.click(640, 360); await until((x) => x.locked, 8000);
  await page.mouse.down(); await sleep(400); await cmd('level 2'); await sleep(800); await page.mouse.up(); await gameSecs(1); s = await st();
  log('P9b level switch with water held: level', s.level, 'streams', s.streams, 'errors', errors.length);
  await page.keyboard.press('Escape'); await sleep(300); await resume();

  // P10: STATS level 1 vs level 2 (idle, low)
  const statsTail = (k) => logs.filter((l) => /STATS|CPUFRAME/.test(l)).slice(-k);
  await load('?low&debug'); await gameSecs(8); log('P10 L1', statsTail(2));
  await load('?low&debug&level=2'); await until((x) => x.level === 2); await gameSecs(8); log('P10 L2', statsTail(2));
  await shot('p10-l2-arrive.png');
} catch (e) { log('PROBE ERROR', String(e && e.stack || e)); }
log('console errors:', errors.length, errors.slice(0, 5));
log('panics:', logs.filter((l) => /panicked/.test(l)).slice(0, 3));
await browser.close(); server.close();
