// Source lint for B6 ("science is right"): every player-facing science string
// in the 3D client (its Rust, HTML and JS, and the player text in
// content/spells.json) must have a row in content/science-sources.md, every `_C`
// temperature constant in chem.rs must match its register value, and register
// rows must not go stale. The format and the claim heuristic are fixed by
// docs/loop/items/I-004/spec.md (Contract) and docs/loop/science-review.md.
//
// Environment (all optional):
//   SCIENCE_SCAN_ROOT       where the client tree is read from (default: repo root)
//   SCIENCE_SOURCES_FILE    the register (default: content/science-sources.md)
//   SCIENCE_SOURCES_VERBOSE 1 prints the inventory as `claim: <text>` lines and
//                           every string skipped as machine output as `skipped: <text>`
//
// The test never fails on `error` verdicts: those are a B6 score input and a
// backlog trigger, not a CI break.
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SCAN_ROOT = resolve(process.env.SCIENCE_SCAN_ROOT || REPO);
const SOURCES_FILE = resolve(process.env.SCIENCE_SOURCES_FILE || join(REPO, "content/science-sources.md"));
const VERBOSE = process.env.SCIENCE_SOURCES_VERBOSE === "1";
const CLIENT = "crates/bevy-client";
const CHEM = `${CLIENT}/src/chem.rs`;

// ---------------------------------------------------------------------------
// Normalization, skips and the claim test (Contract § Claim heuristic)
// ---------------------------------------------------------------------------

function normalize(s: string): string {
  return s
    .normalize("NFC")
    .replace(/[\u2080-\u2089]/g, (c) => String(c.charCodeAt(0) - 0x2080))
    .replace(/\u2212/g, "-")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Machine output, in exactly the two shapes the client uses: a JSON template
 * (starts with `{{"`) or a log line (`TAG key=...`: an upper-case tag, a space,
 * then a lower-case `key=`), and only in `.rs` files. Nothing else is skipped; a string merely containing
 * `{:` goes through the claim test like any other. See docs/loop/science-review.md
 * § Lint limits (c).
 */
const LOG_LINE = /^[A-Z][A-Z0-9_]{2,} [a-z_][a-z0-9_]*=/;
function isSkipped(s: string): boolean {
  return s.startsWith('{{"') || LOG_LINE.test(s);
}

const TEMPERATURE = /-?\d+(\.\d+)?\s*°\s*[CF]\b|\b\d+(\.\d+)?\s*K\b/;
const FORMULA = /\b(?:[A-Z][a-z]?\d*)+(?:\((?:s|l|g|aq)\))?/g;
const KEYWORD =
  /ignit|combust|burn|fuel|boil|melt|freez|froze|latent|oxygen|nitrogen|electroly|smother|heavier than air|\bheat|react|\brust|corro|acid|alkal|litmus|\bpH\b|oxidi|dissolv|evapor|condens|sublim|ca(?:ught|tch(?:es)?) fire/i;

/** Sentence shape: at least 3 tokens with a letter, ending in . ! ? or … once trailing quotes and brackets are trimmed. */
function isSentence(s: string): boolean {
  const words = s.split(/\s+/).filter((w) => /\p{L}/u.test(w));
  if (words.length < 3) return false;
  const end = s.replace(/["'`\u2018\u2019\u201c\u201d)\]}>»\s]+$/u, "");
  return /[.!?\u2026]$/.test(end);
}

function isClaim(s: string): boolean {
  if (TEMPERATURE.test(s) || KEYWORD.test(s) || isSentence(s)) return true;
  for (const m of s.matchAll(FORMULA)) {
    if (/\d/.test(m[0]) || /\((?:s|l|g|aq)\)$/.test(m[0])) return true;
  }
  return false;
}

class UnsupportedSyntax extends Error {}

// ---------------------------------------------------------------------------
// Rust: a small tokenizer (comments, char literals, lifetimes, strings)
// ---------------------------------------------------------------------------

type RustToken = { k: "str" | "ident" | "punct"; v: string; at: number; end: number };

function readRustString(src: string, start: number): { text: string; end: number } {
  let out = "";
  let j = start;
  for (;;) {
    const ch = src[j];
    if (ch === undefined) throw new UnsupportedSyntax(`unterminated Rust string at offset ${start}`);
    if (ch === '"') return { text: out, end: j + 1 };
    if (ch !== "\\") {
      out += ch;
      j++;
      continue;
    }
    const n = src[j + 1];
    const simple: Record<string, string> = { n: "\n", t: "\t", r: "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'" };
    if (n !== undefined && n in simple) {
      out += simple[n];
      j += 2;
    } else if (n === "x") {
      out += String.fromCharCode(parseInt(src.slice(j + 2, j + 4), 16));
      j += 4;
    } else if (n === "u" && src[j + 2] === "{") {
      const close = src.indexOf("}", j + 3);
      if (close < 0) throw new UnsupportedSyntax(`bad \\u escape at offset ${j}`);
      out += String.fromCodePoint(parseInt(src.slice(j + 3, close).replace(/_/g, ""), 16));
      j = close + 1;
    } else if (n === "\n" || n === "\r") {
      throw new UnsupportedSyntax(`Rust \\-newline string continuation at offset ${j} is not supported by the science lint`);
    } else {
      throw new UnsupportedSyntax(`unknown Rust escape \\${n ?? ""} at offset ${j}`);
    }
  }
}

function tokenizeRust(src: string): { tokens: RustToken[]; comments: Array<[number, number]> } {
  const tokens: RustToken[] = [];
  const comments: Array<[number, number]> = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i] as string;
    if (/\s/.test(c)) {
      i++;
    } else if (src.startsWith("//", i)) {
      const nl = src.indexOf("\n", i);
      const end = nl < 0 ? src.length : nl;
      comments.push([i, end]);
      i = end;
    } else if (src.startsWith("/*", i)) {
      let depth = 0;
      let j = i;
      do {
        if (src.startsWith("/*", j)) {
          depth++;
          j += 2;
        } else if (src.startsWith("*/", j)) {
          depth--;
          j += 2;
        } else if (j >= src.length) {
          throw new UnsupportedSyntax(`unterminated block comment at offset ${i}`);
        } else {
          j++;
        }
      } while (depth > 0);
      comments.push([i, j]);
      i = j;
    } else if (c === '"') {
      const s = readRustString(src, i + 1);
      tokens.push({ k: "str", v: s.text, at: i, end: s.end });
      i = s.end;
    } else if (c === "'") {
      if (src[i + 1] === "\\") {
        // Escaped char literal: '\\', '\'', '\n', '\x41', '\u{1F600}'.
        const close = src.indexOf("'", i + 3);
        if (close < 0 || close - i > 12) throw new UnsupportedSyntax(`bad char literal at offset ${i}`);
        tokens.push({ k: "punct", v: src.slice(i, close + 1), at: i, end: close + 1 });
        i = close + 1;
      } else {
        const cp = src.codePointAt(i + 1);
        const len = cp !== undefined && cp > 0xffff ? 2 : 1;
        if (cp !== undefined && src[i + 1] !== "\n" && src[i + 1 + len] === "'") {
          // Plain char literal, including '"'.
          tokens.push({ k: "punct", v: src.slice(i, i + 2 + len), at: i, end: i + 2 + len });
          i += 2 + len;
        } else {
          // Lifetime or label: 'static, 'a.
          tokens.push({ k: "punct", v: "'", at: i, end: i + 1 });
          i++;
        }
      }
    } else if (/[A-Za-z0-9_]/.test(c)) {
      let j = i;
      while (j < src.length && /[A-Za-z0-9_]/.test(src[j] as string)) j++;
      const id = src.slice(i, j);
      const next = src[j];
      if ((id === "r" || id === "br" || id === "cr") && (next === '"' || next === "#")) {
        throw new UnsupportedSyntax(`Rust raw string at offset ${i} is not supported by the science lint`);
      }
      if ((id === "b" || id === "c") && next === '"') {
        const s = readRustString(src, j + 1);
        tokens.push({ k: "str", v: s.text, at: i, end: s.end });
        i = s.end;
      } else {
        tokens.push({ k: "ident", v: id, at: i, end: j });
        i = j;
      }
    } else {
      tokens.push({ k: "punct", v: c, at: i, end: i + 1 });
      i++;
    }
  }
  return { tokens, comments };
}

/** Char ranges of `#[cfg(test)] mod <name> { ... }` blocks. */
function cfgTestModRanges(tokens: RustToken[]): Array<[number, number]> {
  const ranges: Array<[number, number]> = [];
  const seq = ["#", "[", "cfg", "(", "test", ")", "]"];
  for (let k = 0; k < tokens.length; k++) {
    if (!seq.every((v, d) => tokens[k + d]?.v === v)) continue;
    let m = k + seq.length;
    // Further attributes between #[cfg(test)] and `mod`.
    while (tokens[m]?.v === "#" && tokens[m + 1]?.v === "[") {
      let depth = 0;
      do {
        if (tokens[m]?.v === "[") depth++;
        else if (tokens[m]?.v === "]") depth--;
        m++;
      } while (depth > 0 && m < tokens.length);
    }
    if (tokens[m]?.v === "pub") m++;
    if (tokens[m]?.v !== "mod" || tokens[m + 1]?.k !== "ident" || tokens[m + 2]?.v !== "{") continue;
    let depth = 0;
    let e = m + 2;
    for (; e < tokens.length; e++) {
      const t = tokens[e] as RustToken;
      if (t.k !== "punct") continue;
      if (t.v === "{") depth++;
      else if (t.v === "}" && --depth === 0) break;
    }
    const last = tokens[Math.min(e, tokens.length - 1)] as RustToken;
    ranges.push([(tokens[k] as RustToken).at, last.end]);
    k = e;
  }
  return ranges;
}

const inside = (pos: number, ranges: Array<[number, number]>) => ranges.some(([a, b]) => pos >= a && pos < b);

/** String literals of a Rust file, escapes decoded, test modules dropped. */
function rustStrings(src: string): string[] {
  const { tokens } = tokenizeRust(src);
  const tests = cfgTestModRanges(tokens);
  return tokens.filter((t) => t.k === "str" && !inside(t.at, tests)).map((t) => t.v);
}

/** `[pub] const <NAME>_C: f32 = <v>;` in non-test, non-comment code. */
function rustTempConsts(src: string): Array<{ name: string; value: number }> {
  const { tokens, comments } = tokenizeRust(src);
  const skip = [...comments, ...cfgTestModRanges(tokens)];
  const re = /\b(?:pub(?:\([^)]*\))?\s+)?const\s+([A-Z][A-Z0-9_]*_C)\s*:\s*f(?:32|64)\s*=\s*(-?\s*[0-9][0-9_]*(?:\.[0-9_]*)?(?:[eE][-+]?[0-9]+)?)(?:_?f(?:32|64))?\s*;/g;
  const out: Array<{ name: string; value: number }> = [];
  for (const m of src.matchAll(re)) {
    if (inside(m.index, skip)) continue;
    out.push({ name: m[1] as string, value: Number((m[2] as string).replace(/[\s_]/g, "")) });
  }
  return out;
}

// ---------------------------------------------------------------------------
// HTML text nodes and JS string literals
// ---------------------------------------------------------------------------

function decodeEntities(s: string): string {
  const named: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0", minus: "\u2212" };
  return s.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (all, e: string) => {
    if (e[0] === "#") {
      const cp = e[1] === "x" || e[1] === "X" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
      return String.fromCodePoint(cp);
    }
    return named[e.toLowerCase()] ?? all;
  });
}

/** One string per HTML text node. Inline <script>/<style> bodies are refused. */
function htmlTexts(src: string): string[] {
  const out: string[] = [];
  let i = 0;
  let text = "";
  const flush = () => {
    if (text.trim()) out.push(decodeEntities(text));
    text = "";
  };
  while (i < src.length) {
    if (src.startsWith("<!--", i)) {
      flush();
      const e = src.indexOf("-->", i + 4);
      i = e < 0 ? src.length : e + 3;
    } else if (src[i] === "<" && /[A-Za-z/!?]/.test(src[i + 1] ?? "")) {
      flush();
      let j = i + 1;
      let quote = "";
      while (j < src.length) {
        const ch = src[j];
        if (quote) {
          if (ch === quote) quote = "";
        } else if (ch === '"' || ch === "'") {
          quote = ch;
        } else if (ch === ">") {
          break;
        }
        j++;
      }
      const tag = src.slice(i, j + 1);
      i = j + 1;
      const name = /^<([A-Za-z][A-Za-z0-9-]*)/.exec(tag)?.[1]?.toLowerCase();
      if ((name === "script" || name === "style") && !tag.endsWith("/>")) {
        const close = src.toLowerCase().indexOf(`</${name}`, i);
        const body = src.slice(i, close < 0 ? src.length : close);
        if (body.trim()) {
          throw new UnsupportedSyntax(`inline <${name}> body is not supported by the science lint`);
        }
        i = close < 0 ? src.length : close;
      }
    } else {
      text += src[i];
      i++;
    }
  }
  flush();
  return out;
}

function readJsEscape(src: string, j: number): { text: string; next: number } {
  const n = src[j + 1];
  const simple: Record<string, string> = { n: "\n", t: "\t", r: "\r", b: "\b", f: "\f", v: "\v", "0": "\0" };
  if (n === undefined) throw new UnsupportedSyntax("dangling escape");
  if (n in simple) return { text: simple[n] as string, next: j + 2 };
  if (n === "x") return { text: String.fromCharCode(parseInt(src.slice(j + 2, j + 4), 16)), next: j + 4 };
  if (n === "u" && src[j + 2] === "{") {
    const close = src.indexOf("}", j + 3);
    return { text: String.fromCodePoint(parseInt(src.slice(j + 3, close), 16)), next: close + 1 };
  }
  if (n === "u") return { text: String.fromCharCode(parseInt(src.slice(j + 2, j + 6), 16)), next: j + 6 };
  if (n === "\r" && src[j + 2] === "\n") return { text: "", next: j + 3 };
  if (n === "\n" || n === "\r") return { text: "", next: j + 2 };
  return { text: n, next: j + 2 };
}

/** Skips a raw `${ ... }` expression starting at the `{`; returns the index after `}`. */
function skipJsExpr(src: string, start: number): number {
  let depth = 0;
  let j = start;
  while (j < src.length) {
    const ch = src[j];
    if (ch === "{") depth++;
    else if (ch === "}") {
      depth--;
      if (depth === 0) return j + 1;
    } else if (ch === '"' || ch === "'") {
      j = readJsQuoted(src, j).next;
      continue;
    } else if (ch === "`") {
      j = readJsTemplate(src, j).next;
      continue;
    }
    j++;
  }
  throw new UnsupportedSyntax(`unterminated template expression at offset ${start}`);
}

function readJsQuoted(src: string, start: number): { text: string; next: number } {
  const q = src[start];
  let out = "";
  let j = start + 1;
  while (j < src.length) {
    const ch = src[j];
    if (ch === q) return { text: out, next: j + 1 };
    if (ch === "\n") break;
    if (ch === "\\") {
      const e = readJsEscape(src, j);
      out += e.text;
      j = e.next;
    } else {
      out += ch;
      j++;
    }
  }
  throw new UnsupportedSyntax(`unterminated JS string at offset ${start}`);
}

/** Template literal text with each `${...}` kept literally. */
function readJsTemplate(src: string, start: number): { text: string; next: number } {
  let out = "";
  let j = start + 1;
  while (j < src.length) {
    const ch = src[j];
    if (ch === "`") return { text: out, next: j + 1 };
    if (ch === "\\") {
      const e = readJsEscape(src, j);
      out += e.text;
      j = e.next;
    } else if (ch === "$" && src[j + 1] === "{") {
      const end = skipJsExpr(src, j + 1);
      out += src.slice(j, end);
      j = end;
    } else {
      out += ch;
      j++;
    }
  }
  throw new UnsupportedSyntax(`unterminated template literal at offset ${start}`);
}

function jsStrings(src: string): string[] {
  const out: string[] = [];
  let i = 0;
  let prev = ""; // last significant character, to tell a regex literal from division
  while (i < src.length) {
    const c = src[i] as string;
    if (/\s/.test(c)) {
      i++;
    } else if (src.startsWith("//", i)) {
      const nl = src.indexOf("\n", i);
      i = nl < 0 ? src.length : nl;
    } else if (src.startsWith("/*", i)) {
      const e = src.indexOf("*/", i + 2);
      if (e < 0) throw new UnsupportedSyntax(`unterminated block comment at offset ${i}`);
      i = e + 2;
    } else if (c === '"' || c === "'") {
      const s = readJsQuoted(src, i);
      out.push(s.text);
      i = s.next;
      prev = "a";
    } else if (c === "`") {
      const s = readJsTemplate(src, i);
      out.push(s.text);
      i = s.next;
      prev = "a";
    } else if (c === "/" && (prev === "" || /[(,=:[!&|?{};+\-*%<>~^]/.test(prev))) {
      // Regex literal: skip it (it holds no player-facing text).
      let j = i + 1;
      let cls = false;
      while (j < src.length && src[j] !== "\n") {
        const ch = src[j];
        if (ch === "\\") j++;
        else if (ch === "[") cls = true;
        else if (ch === "]") cls = false;
        else if (ch === "/" && !cls) break;
        j++;
      }
      if (src[j] !== "/") throw new UnsupportedSyntax(`unterminated regex literal at offset ${i}`);
      j++;
      while (/[a-z]/.test(src[j] ?? "")) j++;
      i = j;
      prev = "a";
    } else {
      prev = /[A-Za-z0-9_$)\]]/.test(c) ? "a" : c;
      i++;
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Scanning the client tree
// ---------------------------------------------------------------------------

type Found = { file: string; text: string };

function walk(dir: string, ext: string): string[] {
  if (!existsSync(dir)) return [];
  const out: string[] = [];
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p, ext));
    else if (name.endsWith(ext)) out.push(p);
  }
  return out;
}

const relTo = (root: string, abs: string) => abs.slice(root.length + 1).split("\\").join("/");

/** The spell data file (I-026). Only these keys of each `spells[]` entry are player text. */
const SPELLS_JSON = "content/spells.json";
const SPELL_TEXT_KEYS = ["name", "label", "hotbar_label", "cast_caption"] as const;

/** Player text in content/spells.json: string values of SPELL_TEXT_KEYS in each `spells[]` entry, nothing else. */
function spellsJsonTexts(src: string): string[] {
  let doc: unknown;
  try {
    doc = JSON.parse(src);
  } catch (e) {
    throw new UnsupportedSyntax(`invalid JSON: ${(e as Error).message}`);
  }
  const list = (doc as { spells?: unknown }).spells;
  if (!Array.isArray(list)) throw new UnsupportedSyntax("no spells[] array");
  const out: string[] = [];
  for (const spell of list) {
    if (spell === null || typeof spell !== "object") continue;
    for (const key of SPELL_TEXT_KEYS) {
      const v = (spell as Record<string, unknown>)[key];
      if (typeof v === "string") out.push(v);
    }
  }
  return out;
}

/** All extracted strings of one client file, normalized, with the machine-output skips split off. */
function extractFileAll(relPath: string, src: string): { kept: string[]; skipped: string[] } {
  const raw = relPath.endsWith(".rs")
    ? rustStrings(src)
    : relPath.endsWith(".html")
      ? htmlTexts(src)
      : relPath === SPELLS_JSON
        ? spellsJsonTexts(src)
        : jsStrings(src);
  const all = raw.map(normalize).filter((s) => s !== "");
  // Both skip shapes are Rust format strings (`{{` is Rust's escaped brace); .html, .js and spells.json strings are never skipped.
  if (!relPath.endsWith(".rs")) return { kept: all, skipped: [] };
  return { kept: all.filter((s) => !isSkipped(s)), skipped: all.filter(isSkipped) };
}

function extractFile(relPath: string, src: string): string[] {
  return extractFileAll(relPath, src).kept;
}

function scanClient(root: string): { perFile: Map<string, Set<string>>; found: Found[]; skipped: Found[] } {
  const files = [
    ...walk(join(root, CLIENT, "src"), ".rs"),
    ...walk(join(root, CLIENT, "web"), ".html"),
    ...walk(join(root, CLIENT, "web"), ".js"),
    ...(existsSync(join(root, SPELLS_JSON)) ? [join(root, SPELLS_JSON)] : []),
  ];
  const perFile = new Map<string, Set<string>>();
  const found: Found[] = [];
  const skipped: Found[] = [];
  for (const abs of files) {
    const file = relTo(root, abs);
    let strings: { kept: string[]; skipped: string[] };
    try {
      strings = extractFileAll(file, readFileSync(abs, "utf8"));
    } catch (e) {
      throw new UnsupportedSyntax(`${file}: ${(e as Error).message}`);
    }
    perFile.set(file, new Set(strings.kept));
    for (const text of strings.kept) if (isClaim(text)) found.push({ file, text });
    for (const text of strings.skipped) skipped.push({ file, text });
  }
  return { perFile, found, skipped };
}

// ---------------------------------------------------------------------------
// The register (Contract § content/science-sources.md format)
// ---------------------------------------------------------------------------

const VERDICTS = new Set(["correct", "simplified", "error"]);
const CLAIM_COLS = ["id", "claim", "where", "verdict", "note", "sources"];
const PARAM_COLS = ["id", "constant", "value", "unit", "where", "verdict", "note", "sources"];
const NOT_COLS = ["text", "where", "reason"];

type ClaimRow = { id: string; claim: string; norm: string; where: string[]; verdict: string; note: string; sources: string[] };
type ParamRow = { id: string; constant: string; value: number; where: string[]; verdict: string; note: string; sources: string[] };
type Register = {
  claims: ClaimRow[];
  params: ParamRow[];
  notClaims: Array<{ norm: string; where: string; reason: string }>;
  sources: Map<string, string>;
  errors: string[];
};

function splitRow(line: string): string[] {
  const cells: string[] = [];
  let cur = "";
  const body = line.trim().replace(/^\|/, "").replace(/\|$/, "");
  for (let i = 0; i < body.length; i++) {
    if (body[i] === "\\" && body[i + 1] === "|") {
      cur += "|";
      i++;
    } else if (body[i] === "|") {
      cells.push(cur.trim());
      cur = "";
    } else {
      cur += body[i];
    }
  }
  cells.push(cur.trim());
  return cells;
}

const unwrapCode = (s: string): string | null => {
  const m = /^(`+)\s?([\s\S]*?)\s?\1$/.exec(s);
  return m ? (m[2] as string) : null;
};

function parseNumber(cell: string): number {
  const t = normalize(cell).replace(/\s/g, "");
  return /^[-+]?\d+(\.\d+)?$/.test(t) ? Number(t) : Number.NaN;
}

function parseRegister(md: string): Register {
  const reg: Register = { claims: [], params: [], notClaims: [], sources: new Map(), errors: [] };
  const err = (line: number, msg: string) => reg.errors.push(`content/science-sources.md:${line}: ${msg}`);
  const ids = new Map<string, number>();
  const sourceRefs: Array<{ id: string; line: number; refs: string[] }> = [];
  const lines = md.split(/\r?\n/);
  let section = "";
  let tableLine = 0; // 0 = expecting header, 1 = expecting separator, 2 = rows
  const seen = new Set<string>();

  const sourcesCell = (cell: string, id: string, line: number): string[] => {
    if (!/^\[S\d+\](\s+\[S\d+\])*$/.test(cell)) {
      err(line, `${id}: sources cell must be one or more [S<n>] separated by spaces, got "${cell}"`);
      return [];
    }
    const refs = cell.match(/S\d+/g) ?? [];
    sourceRefs.push({ id, line, refs });
    return refs;
  };
  const common = (id: string, pattern: RegExp, verdict: string, note: string, line: number) => {
    if (!pattern.test(id)) err(line, `bad id "${id}" (expected ${pattern.source})`);
    if (ids.has(id)) err(line, `${id}: duplicate id (first used on line ${ids.get(id)})`);
    else ids.set(id, line);
    if (!VERDICTS.has(verdict)) err(line, `${id}: verdict "${verdict}" is not one of correct, simplified, error`);
    if (verdict !== "correct" && note === "") err(line, `${id}: verdict "${verdict}" needs a note`);
  };

  lines.forEach((text, idx) => {
    const line = idx + 1;
    const h = /^##\s+(.+?)\s*$/.exec(text);
    if (h) {
      section = (h[1] as string).toLowerCase();
      seen.add(section);
      tableLine = 0;
      return;
    }
    if (section === "sources") {
      if (/^\s*-\s*\[S/.test(text)) {
        const m = /^\s*-\s*\[(S\d+)\]\s+(\S.*)$/.exec(text);
        if (!m) return err(line, `malformed source entry "${text.trim()}"`);
        const id = m[1] as string;
        if (reg.sources.has(id)) err(line, `[${id}]: duplicate source id`);
        reg.sources.set(id, m[2] as string);
      }
      return;
    }
    if (!["claims", "parameters", "not claims"].includes(section) || !text.trim().startsWith("|")) return;
    const cells = splitRow(text);
    if (tableLine === 0) {
      const want = section === "claims" ? CLAIM_COLS : section === "parameters" ? PARAM_COLS : NOT_COLS;
      if (cells.join("|").toLowerCase() !== want.join("|")) {
        err(line, `## ${section} header must be | ${want.join(" | ")} |, got | ${cells.join(" | ")} |`);
      }
      tableLine = 1;
      return;
    }
    if (tableLine === 1 && cells.every((c) => /^:?-+:?$/.test(c))) {
      tableLine = 2;
      return;
    }
    tableLine = 2;
    if (section === "claims") {
      if (cells.length !== CLAIM_COLS.length) return err(line, `${cells[0] ?? "row"}: expected ${CLAIM_COLS.length} cells, got ${cells.length}`);
      const [id, claimCell, where, verdict, note, src] = cells as [string, string, string, string, string, string];
      common(id, /^C-\d{3}$/, verdict, note, line);
      const claim = unwrapCode(claimCell);
      if (claim === null || claim.trim() === "") err(line, `${id}: claim must be the in-game text wrapped in backticks`);
      const paths = where.split(",").map((p) => p.trim()).filter(Boolean);
      if (paths.length === 0) err(line, `${id}: where is empty`);
      const sources = sourcesCell(src, id, line);
      if (sources.includes("S0")) err(line, `${id}: [S0] (design choice) is not allowed in ## Claims`);
      reg.claims.push({ id, claim: claim ?? "", norm: normalize(claim ?? ""), where: paths, verdict, note, sources });
    } else if (section === "parameters") {
      if (cells.length !== PARAM_COLS.length) return err(line, `${cells[0] ?? "row"}: expected ${PARAM_COLS.length} cells, got ${cells.length}`);
      const [id, constCell, valueCell, , where, verdict, note, src] = cells as [string, string, string, string, string, string, string, string];
      common(id, /^P-\d{3}$/, verdict, note, line);
      const constant = unwrapCode(constCell);
      if (constant === null || constant.trim() === "") err(line, `${id}: constant must be wrapped in backticks`);
      const value = parseNumber(valueCell);
      if (Number.isNaN(value)) err(line, `${id}: value "${valueCell}" is not a number`);
      const paths = where.split(",").map((p) => p.trim()).filter(Boolean);
      if (paths.length === 0) err(line, `${id}: where is empty`);
      const sources = sourcesCell(src, id, line);
      if (sources.includes("S0") && !/^(gameplay tuning|scenario assumption)/.test(note)) {
        err(line, `${id}: [S0] needs a note starting with "gameplay tuning" or "scenario assumption"`);
      }
      reg.params.push({ id, constant: constant ?? "", value, where: paths, verdict, note, sources });
    } else {
      if (cells.length !== NOT_COLS.length) return err(line, `not-claim row: expected ${NOT_COLS.length} cells, got ${cells.length}`);
      const [textCell, where, reason] = cells as [string, string, string];
      const t = unwrapCode(textCell) ?? textCell;
      if (t.trim() === "") err(line, "not-claim row: text is empty");
      if (reason === "") err(line, `not-claim "${t}": reason is empty`);
      reg.notClaims.push({ norm: normalize(t), where, reason });
    }
  });

  for (const s of ["claims", "parameters", "sources"]) if (!seen.has(s)) err(0, `missing section ## ${s[0]?.toUpperCase()}${s.slice(1)}`);
  for (const r of sourceRefs) {
    for (const ref of r.refs) if (!reg.sources.has(ref)) err(r.line, `${r.id}: cites [${ref}], which is not defined in ## Sources`);
  }
  return reg;
}

// ---------------------------------------------------------------------------
// Load everything once; individual tests assert on the pieces.
// ---------------------------------------------------------------------------

let register: Register = { claims: [], params: [], notClaims: [], sources: new Map(), errors: [] };
let registerLoadError = "";
try {
  register = parseRegister(readFileSync(SOURCES_FILE, "utf8"));
} catch (e) {
  registerLoadError = `cannot read register ${SOURCES_FILE}: ${(e as Error).message}`;
}

let scan: ReturnType<typeof scanClient> = { perFile: new Map(), found: [], skipped: [] };
let scanError = "";
try {
  scan = scanClient(SCAN_ROOT);
} catch (e) {
  scanError = `science lint cannot parse the client: ${(e as Error).message}`;
}

const satisfied = new Set([...register.claims.map((r) => r.norm), ...register.notClaims.map((r) => r.norm)]);
const distinctClaims = [...new Set(scan.found.map((f) => f.text))].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
const unsourced = scan.found.filter((f) => !satisfied.has(f.text));
const notClaimSet = new Set(register.notClaims.map((r) => r.norm));

// Stale rows (C- rows only) and missing files.
const staleFailures: string[] = [];
let missingFiles = 0;
for (const row of register.claims) {
  for (const file of row.where) {
    if (!existsSync(join(SCAN_ROOT, file))) {
      missingFiles++;
      continue;
    }
    const strings = scan.perFile.get(file) ?? new Set(extractFile(file, readFileSync(join(SCAN_ROOT, file), "utf8")));
    if (!strings.has(row.norm)) staleFailures.push(`${row.id} is stale: ${file} no longer contains "${row.norm}"`);
  }
}

// _C constants pinned to the register.
const constFailures: string[] = [];
const chemPath = join(SCAN_ROOT, CHEM);
if (existsSync(chemPath)) {
  const consts = rustTempConsts(readFileSync(chemPath, "utf8"));
  const byName = new Map(register.params.map((p) => [p.constant.trim(), p]));
  for (const c of consts) {
    const row = byName.get(c.name);
    if (!row) constFailures.push(`${c.name} (${CHEM}, value ${c.value}) has no row in ## Parameters`);
    else if (Math.abs(row.value - c.value) > 1e-9) {
      constFailures.push(`${c.name}: ${CHEM} has ${c.value} but register ${row.id} has ${row.value}`);
    }
  }
  const names = new Set(consts.map((c) => c.name));
  for (const p of register.params) {
    const name = p.constant.trim();
    if (/^[A-Z][A-Z0-9_]*_C$/.test(name) && !names.has(name)) {
      constFailures.push(`${p.id} pins ${name}, but ${CHEM} has no such constant`);
    }
  }
}

if (VERBOSE) for (const t of distinctClaims) console.log(`claim: ${t}`);
if (VERBOSE) for (const f of scan.skipped) console.log(`skipped: ${f.text}`);
console.log(`science-sources: ${scan.skipped.length} skipped as machine output`);
if (missingFiles > 0) {
  console.log(`science-sources: warning: ${missingFiles} register file reference(s) not found under ${SCAN_ROOT} (client tree absent?); stale checks skipped for them`);
}
const clientClaims = distinctClaims.filter((t) => !notClaimSet.has(t)).length;
const sourcedClaims = distinctClaims.filter((t) => !notClaimSet.has(t) && satisfied.has(t)).length;
const errorVerdicts = [...register.claims, ...register.params].filter((r) => r.verdict === "error").length;
console.log(
  `science-sources: ${clientClaims} client claims, ${sourcedClaims} sourced, ${register.params.length} params, ${errorVerdicts} error verdicts, ${missingFiles} stale-file warnings`,
);

// ---------------------------------------------------------------------------
// The lint
// ---------------------------------------------------------------------------

test("science register is well-formed", () => {
  assert.equal(registerLoadError, "", registerLoadError);
  assert.deepEqual(register.errors, [], `register problems:\n  ${register.errors.join("\n  ")}`);
});

test("the client parses with the science extractor", () => {
  assert.equal(scanError, "", scanError);
});

test("every client science claim has a register row", () => {
  const msgs = unsourced.map((f) => `unsourced claim in ${f.file}: "${f.text}"`);
  assert.deepEqual(msgs, [], `add a ## Claims row (or a ## Not claims row) to content/science-sources.md:\n  ${msgs.join("\n  ")}`);
});

test("every _C temperature constant in chem.rs is pinned to its register value", () => {
  assert.deepEqual(constFailures, [], constFailures.join("\n"));
});

test("no register claim row is stale", () => {
  assert.deepEqual(staleFailures, [], staleFailures.join("\n"));
});

// ---------------------------------------------------------------------------
// Extractor fixtures (spec acceptance test 7)
// ---------------------------------------------------------------------------

const claimsOfRust = (src: string) => rustStrings(src).map(normalize).filter((s) => !isSkipped(s) && isClaim(s));

test("fixture: Rust comments are ignored, but // inside a string is kept", () => {
  const src = [
    "/// Liquid nitrogen boils at 77 K.",
    "pub const LN2_C: f32 = -196.0; // boils at 77 K",
    "/* CO2 is heavy /* nested */ still a comment \"H2O\" */",
    'fn f() -> &\'static str { "see http://x.org: CO2 is heavy" }',
  ].join("\n");
  assert.deepEqual(rustStrings(src), ["see http://x.org: CO2 is heavy"]);
  assert.deepEqual(claimsOfRust(src), ["see http://x.org: CO2 is heavy"]);
});

test("fixture: char literals '\"' and '\\\\' do not open or close a string", () => {
  const src = String.raw`fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }
fn g<'a>(x: &'a str) -> &'a str { let _q = '\''; "Ice melts at 0 °C." }`;
  assert.deepEqual(rustStrings(src), ["\\\\", '\\"', "Ice melts at 0 °C."]);
  assert.deepEqual(claimsOfRust(src), ["Ice melts at 0 °C."]);
});

test("fixture: strings inside #[cfg(test)] mod tests are ignored", () => {
  const src = `fn a() -> &'static str { "H2O boils" }
#[cfg(test)]
mod tests {
    use super::*;
    fn row() { let s = "}"; }
    #[test]
    fn t() { assert!(true, "refreezes flat, as a sheet, H2O(s)"); }
}
fn b() -> &'static str { "CO2 sinks" }`;
  assert.deepEqual(rustStrings(src), ["H2O boils", "CO2 sinks"]);
});

test("fixture: only {{\" JSON templates and TAG key= log lines are skipped as machine output", () => {
  const src = String.raw`fn f() {
    info!("IMPACT obj={} {:?} -> {:?}", 1, 2, 3);
    info!("STATS fps={:.1} burning={}", 1.0, 2);
    let j = format!("{{\"i\":{i},\"burning\":{}}}", 1);
    info!("Water is H2O: {:?}", 1);
    let m = format!("Ice melts at {:.0} °C", 0.0);
    info!("note: N2 boils {:?}", 1);
    let c = "Dry \"fuel\" catches fire fast.";
}`;
  const all = rustStrings(src).map(normalize);
  assert.deepEqual(all.filter(isSkipped), ["IMPACT obj={} {:?} -> {:?}", "STATS fps={:.1} burning={}", '{{"i":{i},"burning":{}}}']);
  for (const s of ["Water is H2O: {:?}", "Ice melts at {:.0} °C", "note: N2 boils {:?}"]) {
    assert.equal(isSkipped(s), false, s);
    assert.equal(isClaim(s), true, s);
  }
  assert.deepEqual(claimsOfRust(src), ["Water is H2O: {:?}", "Ice melts at {:.0} °C", "note: N2 boils {:?}", 'Dry "fuel" catches fire fast.']);
  // Skip-shaped prose stays invisible (Lint limits (c)); the same prose off the shape is flagged.
  assert.equal(isSkipped("STATS fps=1 copper turns green"), true);
  assert.equal(isSkipped("STATS copper turns green."), false);
  assert.equal(isClaim("STATS copper turns green."), true);
  assert.equal(isSkipped("stats fps=1 copper turns green."), false);
  // Skips apply to Rust strings only: a JS log line is kept (and judged by the claim test).
  assert.deepEqual(extractFileAll("web/boot.js", "console.log(`CPUFRAME n=${s.length} p50_ms=${q(0.5)}`);"), {
    kept: ["CPUFRAME n=${s.length} p50_ms=${q(0.5)}"],
    skipped: [],
  });
});

test("fixture: HTML text nodes are extracted one per node", () => {
  const html = `<!doctype html><html><head><meta content="a > b, CO2"><title>T</title><script type="module" src="./boot.js"></script></head>
<body><!-- N2 boils here --><button><span class="name">Fire</span><span class="formula">CH₄ + 2O₂ → CO₂ + 2H₂O</span></button>
<span class="formula">N₂(l) boils at &minus;196 °C</span><span>A &amp; B</span></body></html>`;
  assert.deepEqual(htmlTexts(html), ["T", "Fire", "CH₄ + 2O₂ → CO₂ + 2H₂O", "N₂(l) boils at −196 °C", "A & B"]);
  assert.throws(() => htmlTexts("<script>const a = 'CO2';</script>"), UnsupportedSyntax);
});

test("fixture: JS quoted and template literals are extracted, ${...} kept literally", () => {
  const js = [
    "// 'CO2 in a comment'",
    "const a = 'single H2O';",
    'const b = "double N2(l)";',
    "const c = `Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal}  ·  burning now: ${state.burning}`;",
    "const d = x / 2 / y; const r = /it's/g;",
    "const e = `${obj['n']} ok`;",
  ].join("\n");
  assert.deepEqual(jsStrings(js), [
    "single H2O",
    "double N2(l)",
    "Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal}  ·  burning now: ${state.burning}",
    "${obj['n']} ok",
  ]);
  assert.equal(
    normalize(jsStrings(js)[2] as string),
    "Fires put out: ${Math.min(state.extinguished, state.goal)} / ${state.goal} · burning now: ${state.burning}",
  );
});

test("fixture: normalization folds subscripts, U+2212 and whitespace", () => {
  assert.equal(normalize("CH₄ + 2O₂ → CO₂ + 2H₂O"), "CH4 + 2O2 → CO2 + 2H2O");
  assert.equal(normalize("\u2212196 °C"), "-196 °C");
  assert.equal(normalize("  a \n\t b  "), "a b");
});

test("fixture: the claim test", () => {
  for (const s of [
    "Fire",
    "ice wall",
    "steam vent",
    "Click to look around",
    "Frost",
    "Fires put out: 0 / 3",
    "Greenhouse saved!",
    "Loading…",
    "Click to play",
    "Paused",
    // Lint limits (a): no temperature, formula, keyword or sentence punctuation.
    "copper turns green",
  ]) {
    assert.equal(isClaim(normalize(s)), false, s);
  }
  for (const s of [
    "N₂(l) boils at −196 °C",
    "CO₂ displaces O₂",
    "cools below ignition",
    "Copper melts at 1085 °C.",
    "77 K",
    "H2O(s)",
    // Keyword-free claims from the I-004 break report, and the new keywords.
    "Iron rusts in water.",
    "Sodium reacts violently with water.",
    "Acids turn litmus red; pH 3.",
    "The greenhouse caught fire",
    "Salt dissolves",
    "Water evaporates",
    // Prose net: three words ending in sentence punctuation, trailing quotes and brackets trimmed.
    "Copper turns green.",
    "(Copper turns green!)",
  ]) {
    assert.equal(isClaim(normalize(s)), true, s);
  }
});

test("fixture: unsupported Rust shapes fail loudly", () => {
  assert.throws(() => rustStrings('fn f() { let s = r"raw CO2"; }'), UnsupportedSyntax);
  assert.throws(() => rustStrings('fn f() { let s = r#"raw"#; }'), UnsupportedSyntax);
  assert.throws(() => rustStrings('fn f() { let s = "a \\\n b"; }'), UnsupportedSyntax);
});

test("fixture: register parsing catches malformed rows", () => {
  const md = [
    "# R",
    "## Claims",
    "| id | claim | where | verdict | note | sources |",
    "|---|---|---|---|---|---|",
    "| C-001 | `a \\| b` | x.rs | correct |  | [S1] |",
    "| C-002 | `b` | x.rs | maybe |  | [S1] |",
    "| C-001 | `c` | x.rs | simplified |  | [S0] [S1] |",
    "| C-004 | `d` | x.rs | error |  |  |",
    "| C-005 | `e` | x.rs | correct |  | [S99] |",
    "## Parameters",
    "| id | constant | value | unit | where | verdict | note | sources |",
    "|---|---|---|---|---|---|---|---|",
    "| P-001 | `LN2_C` | \u2212196.0 | °C | chem.rs | correct |  | [S1] |",
    "| P-002 | `COOLING` | 0.05 | 1/s | chem.rs | correct | a guess | [S0] |",
    "| P-003 | `AMBIENT_C` | 20 | °C | chem.rs | correct | scenario assumption: a mild day | [S0] |",
    "## Sources",
    "- [S0] Design choice: gameplay tuning or scenario assumption, no physical claim.",
    "- [S1] Somewhere real.",
  ].join("\n");
  const r = parseRegister(md);
  assert.equal(r.claims[0]?.claim, "a | b");
  assert.equal(r.params[0]?.value, -196);
  const has = (re: RegExp) => assert.ok(r.errors.some((e) => re.test(e)), `${re} in\n${r.errors.join("\n")}`);
  has(/C-002: verdict "maybe"/);
  has(/C-001: duplicate id/);
  has(/C-001: verdict "simplified" needs a note/);
  has(/C-001: \[S0\] .* not allowed in ## Claims/);
  has(/C-004: verdict "error" needs a note/);
  has(/C-004: sources cell/);
  has(/C-005: cites \[S99\]/);
  has(/P-002: \[S0\] needs a note/);
  assert.ok(!r.errors.some((e) => /P-003|P-001/.test(e)), r.errors.join("\n"));
});

// ---------------------------------------------------------------------------
// content/spells.json scan (I-026 acceptance test 17)
// ---------------------------------------------------------------------------

/** A throwaway scan root holding only `content/spells.json`, for scanClient. */
function withSpellsFixture(spells: unknown, check: (root: string) => void): void {
  const root = mkdtempSync(join(tmpdir(), "science-spells-"));
  try {
    mkdirSync(join(root, "content"), { recursive: true });
    writeFileSync(join(root, "content/spells.json"), JSON.stringify(spells, null, 2));
    check(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const BAD = "Water boils at 50 °C.";
const fixtureSpell = (over: Record<string, unknown>) => ({
  id: "water",
  slot: 1,
  effect: "water",
  name: "Water",
  label: "H₂O absorbs heat",
  hotbar_label: "cools below ignition",
  cast_caption: "Water soaks up heat and cools things below their ignition temperature.",
  formula: "H2O(l)",
  tier_min: "sprout",
  reaction: { kind: "phase_change", id: "water_boiling", role: "absorbs_heat" },
  ...over,
});

test("fixture: an unregistered cast_caption in content/spells.json is reported as unsourced", () => {
  withSpellsFixture({ version: 1, spells: [fixtureSpell({ cast_caption: BAD })] }, (root) => {
    const s = scanClient(root);
    const hit = s.found.filter((f) => f.text === BAD);
    assert.deepEqual(hit, [{ file: "content/spells.json", text: BAD }]);
    assert.ok(!satisfied.has(BAD), "the fixture sentence must not be in the register");
  });
});

test("fixture: spells.json notes, ids and formulas are not player text", () => {
  withSpellsFixture(
    { version: 1, notes: BAD, spells: [fixtureSpell({ id: BAD, formula: BAD, effect: BAD })] },
    (root) => {
      const s = scanClient(root);
      assert.deepEqual(
        s.found.filter((f) => f.text === BAD),
        [],
      );
      // The four player-text keys are still read.
      assert.ok(s.perFile.get("content/spells.json")?.has("cools below ignition"));
    },
  );
});

test("the real content/spells.json is scanned and fully sourced", () => {
  const texts = scan.perFile.get("content/spells.json");
  assert.ok(texts, "content/spells.json was not scanned");
  assert.ok(texts.has(normalize("Liquid nitrogen boils at −196 °C (77 K), pulling heat out of anything it touches.")));
  const msgs = unsourced.filter((f) => f.file === "content/spells.json").map((f) => f.text);
  assert.deepEqual(msgs, []);
});
