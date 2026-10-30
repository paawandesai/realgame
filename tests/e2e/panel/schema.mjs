// A small JSON Schema (draft 2020-12) validator for the keywords that
// result.schema.json uses, so the harness needs no extra npm package. The
// schema file is the single source of truth: enums and ranges are read from it.
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
let cached = null;
export function loadSchema() {
  cached ??= JSON.parse(readFileSync(join(here, 'result.schema.json'), 'utf8'));
  return cached;
}

const typeOf = (v) => (v === null ? 'null' : Array.isArray(v) ? 'array' : Number.isInteger(v) ? 'integer' : typeof v);
const typeOk = (v, t) => { const a = typeOf(v); return a === t || (t === 'number' && a === 'integer'); };

// Returns a list of errors, each starting with the dotted path of the field.
export function check(v, s, path, out) {
  const at = path || '(root)';
  if ('const' in s && v !== s.const) { out.push(`${at}: expected ${JSON.stringify(s.const)}, got ${JSON.stringify(v)}`); return out; }
  if (s.enum && !s.enum.includes(v)) { out.push(`${at}: must be one of ${s.enum.join('|')}, got ${JSON.stringify(v)}`); return out; }
  if (s.type) {
    const ts = [].concat(s.type);
    if (!ts.some((t) => typeOk(v, t))) { out.push(`${at}: expected ${ts.join(' or ')}, got ${typeOf(v)}`); return out; }
  }
  if (typeof v === 'number') {
    if (s.minimum !== undefined && v < s.minimum) out.push(`${at}: ${v} is below the minimum ${s.minimum}`);
    if (s.maximum !== undefined && v > s.maximum) out.push(`${at}: ${v} is above the maximum ${s.maximum}`);
  }
  if (typeof v === 'string') {
    if (s.minLength !== undefined && v.length < s.minLength) out.push(`${at}: shorter than ${s.minLength}`);
    if (s.maxLength !== undefined && v.length > s.maxLength) out.push(`${at}: longer than ${s.maxLength}`);
  }
  if (Array.isArray(v)) {
    if (s.maxItems !== undefined && v.length > s.maxItems) out.push(`${at}: more than ${s.maxItems} items`);
    if (s.items) v.forEach((x, i) => check(x, s.items, `${path}[${i}]`, out));
  }
  if (typeOf(v) === 'object') {
    for (const k of s.required || []) if (!(k in v)) out.push(`${path ? `${path}.` : ''}${k}: required`);
    for (const [k, x] of Object.entries(v)) {
      const p = path ? `${path}.${k}` : k;
      if (s.properties?.[k]) check(x, s.properties[k], p, out);
      else if (s.additionalProperties === false) out.push(`${p}: not allowed`);
    }
  }
  return out;
}

export function validate(result) {
  return check(result, loadSchema(), '', []);
}

// The persona's self-report alone (the `self` part of the schema).
export function validateSelf(self) {
  return check(self, loadSchema().properties.self, 'self', []);
}
