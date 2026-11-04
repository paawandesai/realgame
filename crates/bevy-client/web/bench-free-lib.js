// Pure helpers for the Chemistry Bench free play page (bench-free.js):
// formula parsing and equation sides. No DOM, so tests import it. Element and
// substance names live in content/chem/names.json (served by the engine).

/**
 * Atom counts of a formula such as `Ca(OH)2` or `CH3COOH`; null when it
 * isn't one, or uses a symbol outside `elements`.
 * @param {string} f
 * @param {(el: string) => boolean} isElement
 * @returns {Record<string, number> | null}
 */
export function parseFormula(f, isElement) {
  const t = f.replace(/\s/g, '');
  if (!/^[A-Za-z0-9()]+$/.test(t)) return null;
  /** @type {Record<string, number>[]} */
  const stack = [{}];
  let i = 0;
  const at = (/** @type {number} */ k) => t.charAt(k);
  const num = () => { let s = ''; while (i < t.length && /\d/.test(at(i))) s += at(i++); return s ? Number(s) : 1; };
  const top = () => /** @type {Record<string, number>} */ (stack[stack.length - 1]);
  while (i < t.length) {
    const ch = at(i);
    if (ch === '(') { stack.push({}); i++; }
    else if (ch === ')') {
      i++;
      const g = stack.length > 1 ? stack.pop() : undefined;
      if (!g) return null;
      const n = num();
      const into = top();
      for (const [e, k] of Object.entries(g)) into[e] = (into[e] ?? 0) + k * n;
    } else if (/[A-Z]/.test(ch)) {
      let el = ch; i++;
      if (i < t.length && /[a-z]/.test(at(i))) el += at(i++);
      if (!isElement(el)) return null;
      const into = top();
      into[el] = (into[el] ?? 0) + num();
    } else return null;
  }
  const only = stack.length === 1 ? stack[0] : undefined;
  return only && Object.keys(only).length ? only : null;
}

/**
 * Hill order: C, then H, then the rest A to Z (all A to Z without carbon).
 * @param {Record<string, number>} counts
 */
export function hill(counts) {
  const n = (/** @type {string} */ k) => counts[k] ?? 0;
  const keys = Object.keys(counts).filter((k) => n(k) > 0);
  const order = n('C') ? ['C', ...(n('H') ? ['H'] : []), ...keys.filter((k) => k !== 'C' && k !== 'H').sort()] : keys.sort();
  return order.map((k) => k + (n(k) > 1 ? n(k) : '')).join('');
}

/**
 * Splits one side of an equation into formula -> coefficient.
 * @param {string} s
 */
export function sideOf(s) {
  /** @type {Record<string, number>} */
  const m = {};
  for (const term of s.split('+')) {
    const x = term.trim().match(/^(\d*)\s*(.+)$/);
    const f = x?.[2];
    if (x && f) m[f] = (m[f] ?? 0) + (x[1] ? Number(x[1]) : 1);
  }
  return m;
}

/**
 * `2H2 + O2` from formula -> coefficient.
 * @param {Record<string, number>} side
 */
export function sideText(side) {
  return Object.entries(side).map(([f, n]) => (n > 1 ? n : '') + f).join(' + ');
}
