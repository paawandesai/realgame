// The information barrier between a persona and the harness (I-014,
// docs/loop/panel.md "Information barrier"). A persona sees only its run dir
// (session.json + shots/); the operator dir holds the truth log, the daemon
// logs, the token and result.json. Truth and debug need the operator role on
// the CLI side and the token on the daemon side.
import { resolve, relative, isAbsolute, join } from 'node:path';
import { readFileSync, existsSync } from 'node:fs';
import { timingSafeEqual } from 'node:crypto';

export const ROLE_ENV = 'PANEL_ROLE';
export const isOperator = (env = process.env) => env[ROLE_ENV] === 'operator';

export function requireOperator(what, env = process.env) {
  if (!isOperator(env)) throw new Error(`${what} is an operator command: set ${ROLE_ENV}=operator (personas use only observe and act)`);
}

const inside = (child, parent) => {
  const r = relative(parent, child);
  return r === '' || (!r.startsWith('..') && !isAbsolute(r));
};

// The run dir (persona-visible) and the operator dir (default: a sibling
// "<run>.operator"). Neither may contain the other.
export function layout(run, opDir) {
  const r = resolve(run);
  const op = resolve(opDir || `${r}.operator`);
  if (inside(op, r) || inside(r, op)) throw new Error(`the operator dir must be outside the run dir (run ${r}, operator dir ${op})`);
  return { run: r, op };
}

export function readToken(op) {
  const p = join(op, 'operator.json');
  if (!existsSync(p)) return null;
  try { return JSON.parse(readFileSync(p, 'utf8')).token || null; } catch { return null; }
}

const same = (a, b) => {
  const x = Buffer.from(String(a)); const y = Buffer.from(String(b));
  return x.length === y.length && timingSafeEqual(x, y);
};

// Daemon side: a truth observe or a debug command needs the session token.
export function authorize(msg, token) {
  const needs = msg.cmd === 'debug' || (msg.cmd === 'observe' && !!msg.truth);
  if (!needs) return;
  if (!token || !msg.token || !same(msg.token, token)) {
    throw new Error(`${msg.cmd === 'debug' ? 'debug' : 'observe --truth'} needs the operator token (operator.json in the operator dir)`);
  }
}
