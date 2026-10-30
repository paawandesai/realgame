#!/usr/bin/env node
// Persona panel harness CLI. Every command prints exactly one JSON object on
// stdout; on failure it prints {"ok":false,"error":"..."} and exits non-zero.
// A run dir holds all of one session's state, so sessions run in parallel.
// Protocol, persona prompts, rubric and scoring: docs/loop/panel.md.
//
//   node tests/e2e/panel/panel.mjs start   --run <dir> [--persona <id>] [--throttle <n>] [--query <q>]
//                                          [--vw 1280 --vh 720] [--base-url <url>] [--root <pkg-out>]
//                                          [--artifact <sha>] [--freeze 1|0]
//   node tests/e2e/panel/panel.mjs observe --run <dir> [--truth]
//   node tests/e2e/panel/panel.mjs act     --run <dir> <action> [args...]
//         key <Key> [ms] | keys <K1+K2> <ms> | move <dx> <dy> [steps] | drag <x1> <y1> <x2> <y2> [ms]
//         click <x> <y> [left|right] | hold <x> <y> <ms> | wheel <dy> | wait <ms>
//   node tests/e2e/panel/panel.mjs report  --run <dir> --file <answer.json> [--model <m>]
//   node tests/e2e/panel/panel.mjs stop    --run <dir>
//   node tests/e2e/panel/panel.mjs score   --runs <dir1> ... [--append <scoreboard.jsonl>] [--artifact <sha>]
import { spawn, execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync, mkdirSync, rmSync, openSync, appendFileSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { request as httpRequest } from 'node:http';
import { persona as personaInfo, PERSONAS } from './personas.mjs';
import { measure } from './measure.mjs';
import { validate, validateSelf } from './schema.mjs';
import { scorePanel } from './score.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../..');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const VALUE_FLAGS = new Set(['run', 'persona', 'throttle', 'query', 'vw', 'vh', 'base-url', 'root', 'artifact', 'file', 'append', 'freeze', 'model']);
function parse(argv) {
  const flags = {}; const pos = []; const runs = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--runs') { while (argv[i + 1] !== undefined && !argv[i + 1].startsWith('--')) runs.push(argv[++i]); continue; }
    if (a.startsWith('--')) {
      const k = a.slice(2);
      if (VALUE_FLAGS.has(k)) {
        if (argv[i + 1] === undefined) throw new Error(`--${k} needs a value`);
        flags[k] = argv[++i];
      } else flags[k] = true;
      continue;
    }
    pos.push(a);
  }
  return { flags, pos, runs };
}

const alive = (pid) => { try { process.kill(pid, 0); return true; } catch { return false; } };
const readJson = (p) => JSON.parse(readFileSync(p, 'utf8'));
const runDir = (flags) => { if (!flags.run) throw new Error('--run <dir> is required'); return resolve(flags.run); };

function session(dir) {
  const p = join(dir, 'session.json');
  if (!existsSync(p)) throw new Error(`no session in ${dir} (run start first)`);
  return readJson(p);
}

function post(port, msg, timeoutMs = 240000) {
  return new Promise((res, rej) => {
    const body = JSON.stringify(msg);
    const req = httpRequest({ host: '127.0.0.1', port, method: 'POST', path: '/', headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) } }, (r) => {
      let d = '';
      r.on('data', (c) => { d += c; });
      r.on('end', () => { try { res(JSON.parse(d)); } catch (e) { rej(e); } });
    });
    req.setTimeout(timeoutMs, () => req.destroy(new Error('daemon did not answer in time')));
    req.on('error', rej);
    req.end(body);
  });
}

async function cmdStart({ flags }) {
  const dir = runDir(flags);
  mkdirSync(dir, { recursive: true });
  const sp = join(dir, 'session.json');
  if (existsSync(sp)) {
    const s = readJson(sp);
    if (alive(s.pid)) throw new Error(`a session is already running in ${dir} (pid ${s.pid}); stop it first`);
    rmSync(sp, { force: true });
  }
  rmSync(join(dir, 'start-error.json'), { force: true });
  const id = flags.persona || '9yo';
  const p = personaInfo(id);
  const throttle = flags.throttle !== undefined ? +flags.throttle : p.throttle;
  if (!(throttle >= 1)) throw new Error('--throttle must be >= 1');
  let artifact = flags.artifact;
  if (!artifact) { try { artifact = execFileSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(); } catch { artifact = 'unknown'; } }
  if (existsSync(join(dir, 'log.jsonl'))) throw new Error(`${dir} already holds a session log; use a fresh run dir`);
  const args = [join(here, 'daemon.mjs'), '--run', dir, '--persona', id, '--throttle', String(throttle), '--query', flags.query || '',
    '--vw', String(flags.vw || 1280), '--vh', String(flags.vh || 720), '--artifact', artifact, '--freeze', flags.freeze === '0' ? '0' : '1'];
  if (flags['base-url']) args.push('--base-url', flags['base-url']);
  if (flags.root) args.push('--root', resolve(flags.root));
  const out = openSync(join(dir, 'daemon.out'), 'a');
  const child = spawn(process.execPath, args, { detached: true, stdio: ['ignore', out, out], env: process.env });
  child.unref();
  const t0 = Date.now();
  while (Date.now() - t0 < 200000) {
    if (existsSync(sp)) return { ok: true, run: dir, ...readJson(sp) };
    if (existsSync(join(dir, 'start-error.json'))) throw new Error(`start failed: ${readJson(join(dir, 'start-error.json')).error}`);
    if (!alive(child.pid)) throw new Error(`daemon exited during start (see ${join(dir, 'daemon.out')})`);
    await sleep(250);
  }
  try { process.kill(child.pid, 'SIGKILL'); } catch { /* gone */ }
  throw new Error('start timed out after 200 s waiting for window.__gameState');
}

async function cmdObserve({ flags }) {
  const dir = runDir(flags);
  return post(session(dir).port, { cmd: 'observe', truth: !!flags.truth });
}

async function cmdAct({ flags, pos }) {
  const dir = runDir(flags);
  const [action, ...args] = pos;
  if (!action) throw new Error('act needs an action');
  return post(session(dir).port, { cmd: 'act', action, args });
}

async function cmdDebug({ flags, pos }) {
  const dir = runDir(flags);
  const [action, ...args] = pos;
  return post(session(dir).port, { cmd: 'debug', action, args });
}

async function cmdStop({ flags }) {
  const dir = runDir(flags);
  const sp = join(dir, 'session.json');
  if (!existsSync(sp)) return { ok: true, stopped: false, note: 'no session running' };
  const s = readJson(sp);
  let reply = null;
  if (alive(s.pid)) {
    try { reply = await post(s.port, { cmd: 'stop' }, 20000); } catch { /* fall through to kill */ }
    const t0 = Date.now();
    while (alive(s.pid) && Date.now() - t0 < 10000) await sleep(100);
    if (alive(s.pid)) { try { process.kill(s.pid, 'SIGKILL'); } catch { /* gone */ } await sleep(300); }
  }
  rmSync(sp, { force: true });
  return { ok: true, stopped: true, pid: s.pid, port: s.port, turn: reply?.turn ?? null, panel_s: reply?.panel_s ?? null, cap: reply?.cap ?? null };
}

const SELF_KEYS = ['clarity', 'fun', 'blockers', 'would_play_again', 'best', 'worst', 'quotes'];
const MEASURED_KEYS = ['measured', 'time_to_move_s', 'time_to_cast_s', 'minutes_played', 'quests_completed', 'outcome', 'turns', 'console_errors', 'panics'];

function cmdReport({ flags }) {
  const dir = runDir(flags);
  if (!flags.file) throw new Error('--file <answer.json> is required');
  const logPath = join(dir, 'log.jsonl');
  if (!existsSync(logPath)) throw new Error(`no log.jsonl in ${dir}`);
  const log = readFileSync(logPath, 'utf8').trim().split('\n').filter(Boolean).map((l) => JSON.parse(l));
  const start = log.find((e) => e.cmd === 'start');
  let ans;
  try { ans = readJson(resolve(flags.file)); } catch (e) { throw new Error(`answer file: ${e.message}`); }
  const src = ans.self && typeof ans.self === 'object' ? ans.self : ans;
  const self = {};
  for (const k of SELF_KEYS) if (k in src) self[k] = src[k];
  const errs = validateSelf(self);
  if (errs.length) throw new Error(errs.join('; '));
  const warnings = MEASURED_KEYS.filter((k) => k in ans || k in src).map((k) => `warning: answer field "${k}" ignored; the harness measures it`);
  for (const w of warnings) process.stderr.write(`${w}\n`);
  const m = measure(log);
  const { cap, ...measured } = m;
  const stopped = cap || ans.stopped_because || 'quit';
  const result = {
    schema: 'panel-result/1',
    persona: start.persona,
    model: flags.model || ans.model || PERSONAS[start.persona]?.model || 'unknown',
    artifact_sha: start.artifact_sha,
    query: start.query,
    throttle: start.throttle,
    measured,
    self,
    stopped_because: stopped,
    ...(ans.quit_reason !== undefined ? { quit_reason: ans.quit_reason } : {}),
  };
  const rerr = validate(result);
  if (rerr.length) throw new Error(rerr.join('; '));
  writeFileSync(join(dir, 'result.json'), `${JSON.stringify(result, null, 2)}\n`);
  return { ok: true, result_path: join(dir, 'result.json'), result, warnings };
}

function cmdScore({ flags, runs }) {
  if (!runs.length) throw new Error('--runs <dir> ... is required');
  const results = []; const rejected = [];
  for (const d of runs) {
    const p = join(resolve(d), 'result.json');
    if (!existsSync(p)) { rejected.push({ run: d, error: 'missing result.json' }); continue; }
    let r;
    try { r = readJson(p); } catch (e) { rejected.push({ run: d, error: e.message }); continue; }
    const errs = validate(r);
    if (errs.length) { rejected.push({ run: d, error: errs.join('; ') }); continue; }
    results.push(r);
  }
  const line = scorePanel(results, { artifact: flags.artifact });
  if (flags.append) appendFileSync(resolve(flags.append), `${JSON.stringify(line)}\n`);
  return { ok: true, line, rejected, appended: flags.append ? resolve(flags.append) : null };
}

const COMMANDS = { start: cmdStart, observe: cmdObserve, act: cmdAct, debug: cmdDebug, report: cmdReport, stop: cmdStop, score: cmdScore };

(async () => {
  const [name, ...rest] = process.argv.slice(2);
  const fn = COMMANDS[name];
  if (!fn) throw new Error(`unknown command "${name}" (${Object.keys(COMMANDS).join(', ')})`);
  const out = await fn(parse(rest));
  process.stdout.write(`${JSON.stringify(out)}\n`);
  process.exitCode = out.ok === false ? (out.cap ? 3 : 1) : 0;
})().catch((e) => {
  process.stdout.write(`${JSON.stringify({ ok: false, error: e.message })}\n`);
  process.exitCode = 1;
});
