// The Formula Bench (I-028): build a spell's formula at your age band's
// level. See docs/loop/items/I-028/spec.md (sections 1-3 and Contract).
//
// Loaded by boot.js with a guarded dynamic import, so a missing or broken
// bench.js costs only the bench, never the boot. The game drains
// window.__gameIn.bench (one command string each: `band <tier>`, `open`,
// `pick <slot>`, `try <slot> <text…>`, `close`) and writes
// window.__benchState (a JSON string) only when it changes. crates/sim's
// bench::judge is the only judge: nothing here decides right or wrong. The
// balance scale is display arithmetic (atom counts per side), never a verdict.
//
// The bench never opens by itself: only the B key and the #benchbtn button
// open it, and only while no other aria-modal dialog is showing (the one
// modal gate). While it is open the game is paused, the mouse is free, the
// page behind it is inert, and every keydown/keypress is swallowed in the
// window capture phase before winit or hud.js can see it (keyup always
// passes, so a key held when the bench opened is released in the game).
//
// Test hook: window.__bench = { state(), ui(), open(), close(), cmd(s) },
// plus fake(view) under ?debug only.

/** @typedef {{ side: number, coef: number, formula: string, atoms: Array<[string, number]>, state?: string | null }} RecipeTerm */
/** @typedef {{ slot: number, id: string, name: string, formula: string, stage: string, prompt: string }} SpellCard */
/** @typedef {{ n?: number, slot: number, ok?: boolean, code: string, diffs?: Array<[string, number, number]>, other?: number, bad?: string, simplest?: string, coef?: number, learned?: boolean, stage?: string }} Result */
/** @typedef {{ v?: number, atBench?: boolean, open?: boolean, band?: string, spells?: SpellCard[], pick?: number, recipe?: RecipeTerm[], result?: Result }} View */
/** @typedef {Record<string | number, { name: string, formula: string }>} Names */
/** @typedef {{ band: string, spell: SpellCard, recipe: RecipeTerm[] | null, prompt?: string, onMake: (text: string) => void }} BuilderOpts */

const BANDS = ['sprout', 'seedling', 'apprentice'];
// The four atom cards and tiles, with the CPK-style colours a drawing uses
// (a convention, not a claim); the letter is always drawn too (GS-19).
const ELEMENTS = ['H', 'O', 'C', 'N'];
const ICONS = /** @type {Record<string, string>} */ ({ fire: 'fire', water: 'water', frost: 'frost', co2: 'foam' });
const STAGE_WORD = /** @type {Record<string, string>} */ ({ unknown: 'New', learning: 'Learning', practised: 'Practised', mastered: 'Mastered' });
const STATE_WORD = /** @type {Record<string, string>} */ ({ l: 'a liquid', g: 'a gas', s: 'a solid', aq: 'a solution in water' });
const SETTINGS_KEY = 'alchemist.settings.v1';
// Loudest any tone gets, as hud.js.
const PEAK = 0.15;
const SUB = '₀₁₂₃₄₅₆₇₈₉';

// ---------------------------------------------------------------------------
// Pure helpers (exported ones are tested in test/bench.test.ts)
// ---------------------------------------------------------------------------

/** @param {unknown} b */
function bandOf(b) {
  if (b === 'adept') return 'apprentice';
  return typeof b === 'string' && BANDS.includes(b) ? b : null;
}

/**
 * A formula for reading: subscripts for atom counts, an arrow for `->`;
 * state symbols kept or stripped.
 * @param {string} text
 * @param {boolean} [keepState]
 */
export function pretty(text, keepState = true) {
  let s = String(text || '');
  if (!keepState) s = s.replace(/\((?:s|l|g|aq)\)/g, '');
  s = s.replace(/->/g, '→');
  let out = '';
  for (let i = 0; i < s.length; i++) {
    const c = s[i] ?? '';
    const prev = out[out.length - 1] ?? '';
    if (c >= '0' && c <= '9' && (/[A-Za-z)]/.test(prev) || SUB.includes(prev))) out += SUB[+c];
    else out += c;
  }
  return out;
}

/**
 * The elements in formula text order across the recipe (first appearance).
 * @param {RecipeTerm[] | null | undefined} recipe
 */
function recipeOrder(recipe) {
  /** @type {string[]} */
  const out = [];
  for (const t of recipe || []) for (const [el] of t.atoms || []) if (!out.includes(el)) out.push(el);
  return out;
}

/**
 * Text for per-element counts: recipe elements first, in recipe order, then
 * others alphabetically; a count of 1 is written as nothing.
 * @param {RecipeTerm[] | null | undefined} recipe
 * @param {Record<string, number>} counts
 */
function formulaOf(recipe, counts) {
  const order = recipeOrder(recipe);
  const rest = Object.keys(counts).filter((el) => !order.includes(el)).sort();
  let out = '';
  for (const el of [...order, ...rest]) {
    const n = counts[el] || 0;
    if (n > 0) out += n === 1 ? el : `${el}${n}`;
  }
  return out;
}

/**
 * Sprout: the text for the atoms placed in the tray, for example
 * ['H','O','H'] with the water recipe -> 'H2O'.
 * @param {string[]} placed
 * @param {RecipeTerm[] | null} recipe
 */
export function composeSprout(placed, recipe) {
  /** @type {Record<string, number>} */
  const counts = {};
  for (const el of placed || []) counts[el] = (counts[el] || 0) + 1;
  return formulaOf(recipe, counts);
}

/**
 * Seedling, one substance: the text for the tile counts, {H:2,O:1} -> 'H2O'.
 * @param {RecipeTerm[] | null} recipe
 * @param {Record<string, number>} counts
 */
export function composeSeedling(recipe, counts) {
  return formulaOf(recipe, counts || {});
}

/**
 * A reaction: the equation for the coefficient tiles, for example Fire
 * [1,2,1,2] -> 'CH4 + 2O2 -> CO2 + 2H2O'.
 * @param {RecipeTerm[]} recipe
 * @param {number[]} coefs
 */
export function composeEquation(recipe, coefs) {
  /** @param {number} side */
  const part = (side) => recipe
    .map((t, i) => ({ t, c: coefs[i] ?? 1 }))
    .filter(({ t }) => t.side === side)
    .map(({ t, c }) => `${c > 1 ? c : ''}${t.formula}`)
    .join(' + ');
  return `${part(0)} -> ${part(1)}`;
}

/**
 * The balance scale: atom counts per pan (before and after the arrow), level
 * when every element matches, tilt = sign(right total - left total).
 * @param {RecipeTerm[]} recipe
 * @param {number[]} coefs
 */
export function scaleOf(recipe, coefs) {
  const order = recipeOrder(recipe);
  /** @type {Record<string, number>} */
  const left = {};
  /** @type {Record<string, number>} */
  const right = {};
  for (const el of order) { left[el] = 0; right[el] = 0; }
  recipe.forEach((t, i) => {
    const c = coefs[i] ?? 1;
    const pan = t.side === 0 ? left : right;
    for (const [el, n] of t.atoms) pan[el] = (pan[el] || 0) + c * n;
  });
  const total = (/** @type {Record<string, number>} */ p) => Object.values(p).reduce((a, b) => a + b, 0);
  const level = order.every((el) => left[el] === right[el]);
  return { left, right, level, tilt: Math.sign(total(right) - total(left)) };
}

/**
 * The band's reason line for a judged result (Contract, "Reason strings").
 * Lines for several diffs are joined with a newline.
 * @param {string} band
 * @param {Result} result
 * @param {Names} names
 * @param {number} tries wrong tries on this spell in this bench session, this one included
 */
export function reasonFor(band, result, names, tries) {
  const b = bandOf(band) || 'apprentice';
  const r = result || /** @type {Result} */ ({ code: 'parse', slot: 0 });
  const me = names[r.slot] || { name: '', formula: '' };
  const name = me.name;
  const other = (names[r.other ?? 0] || { name: '' }).name;
  const formula = pretty(me.formula, b !== 'seedling');
  const diffs = r.diffs || [];
  const bad = r.bad || '';
  const stateWord = STATE_WORD[bad] || 'a liquid';
  const counts = () => diffs.map(([el, have, want]) => `Count again: ${el} should be ${want}. You have ${have}.`).join('\n');
  switch (r.code) {
    case 'ok':
      return b === 'sprout' ? `You made ${name}!` : `${formula} is right! You are learning ${name}.`;
    case 'empty':
      return b === 'sprout' ? 'Tap the balls to build it.' : b === 'seedling' ? 'Tap the tiles to build it.' : 'Write a formula first.';
    case 'atoms_off':
      if (b === 'sprout') return 'Look again. Count the balls in the picture.';
      if (b === 'seedling' || tries >= 3) return counts();
      return `Not yet. Which atoms does ${name} have, and how many?`;
    case 'other_spell':
      return `That makes ${other}. This one is ${name}.`;
    case 'unbalanced':
      if (b === 'seedling') return 'The scale is not level. Atoms are never made or lost, so both sides need the same atoms.';
      return diffs.map(([el, left, right]) => `Not balanced yet: ${el}: ${left} before, ${right} after.`).join('\n');
    case 'state_missing':
      return 'Right atoms. Now add its state: (s), (l) or (g).';
    case 'wrong_state':
      return `Right atoms. This spell needs it as ${stateWord}.`;
    case 'not_single':
      return 'This spell is one substance. Write just its formula.';
    case 'no_products':
      return 'Add an arrow and what it makes.';
    case 'wrong_species':
      return 'Those are not the substances in this reaction. Check what burns and what it makes.';
    case 'element':
      return `"${bad}" is not an element symbol.`;
    case 'not_for_band':
      return 'Pick a spell from the list.';
    case 'parse':
    default:
      return 'Use element symbols, numbers, + and an arrow.';
  }
}

/**
 * The age band: ?band= in the URL (adept reads as apprentice), else the age
 * picker's pick on this page load, else the bench chooser's; else null (ask).
 * @param {string} search
 * @param {string | null | undefined} picked
 * @param {string | null | undefined} chosen
 */
export function bandFrom(search, picked, chosen) {
  let url = null;
  try { url = new URLSearchParams(search || '').get('band'); } catch { url = null; }
  return bandOf(url) || bandOf(picked) || bandOf(chosen) || null;
}

// ---------------------------------------------------------------------------
// Speech and sound
// ---------------------------------------------------------------------------

/** Sound is on unless the saved settings say `sound: false` (absent or unreadable = on, as hud.js). */
function soundOn() {
  try {
    const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) || 'null');
    if (s && typeof s === 'object' && s.sound === false) return false;
  } catch {
    // Unreadable settings read as the default: sound on.
  }
  return true;
}

/**
 * Speaks a line (cancelling any line still playing) when the speech API
 * exists and sound is on. With no voices it is a silent no-op: every view
 * also works from its pictures and on-screen text.
 * @param {string} text
 */
function speak(text) {
  try {
    if (typeof window === 'undefined' || !text) return;
    const syn = /** @type {any} */ (window).speechSynthesis;
    if (!syn || typeof syn.speak !== 'function' || !soundOn()) return;
    if (typeof syn.cancel === 'function') syn.cancel();
    const U = /** @type {any} */ (window).SpeechSynthesisUtterance;
    syn.speak(typeof U === 'function' ? new U(text) : { text });
  } catch {
    // Speech is a help, never a requirement.
  }
}

/** @type {AudioContext | null} */
let audio = null;
/** Starts (or resumes) the audio context; call from a click or key handler. */
function wakeAudio() {
  try {
    if (audio) { if (audio.state === 'suspended') audio.resume().catch(() => {}); return; }
    const AC = window.AudioContext || /** @type {any} */ (window).webkitAudioContext;
    if (AC) audio = new AC();
  } catch {
    audio = null;
  }
}
/** A short rising tone for a correct build (peak gain at most PEAK, sound on only). */
function rise() {
  if (!audio || !soundOn()) return;
  try {
    const t0 = audio.currentTime + 0.01;
    [[523, 0], [659, 0.12], [784, 0.24]].forEach(([f, at]) => {
      if (!audio || f === undefined || at === undefined) return;
      const o = audio.createOscillator();
      const g = audio.createGain();
      o.type = 'triangle';
      o.frequency.setValueAtTime(f, t0 + at);
      g.gain.setValueAtTime(0.0001, t0 + at);
      g.gain.exponentialRampToValueAtTime(PEAK, t0 + at + 0.02);
      g.gain.exponentialRampToValueAtTime(0.0001, t0 + at + 0.14);
      o.connect(g).connect(audio.destination);
      o.start(t0 + at);
      o.stop(t0 + at + 0.16);
    });
  } catch {
    // No sound: the icon and the words still say it.
  }
}

// ---------------------------------------------------------------------------
// The build view on its own: mountBuilder (also used by I-029 and I-030)
// ---------------------------------------------------------------------------

/**
 * @param {Document} doc
 * @param {string} tag
 * @param {string} [cls]
 * @param {string} [text]
 */
function mk(doc, tag, cls, text) {
  const el = /** @type {HTMLElement} */ (doc.createElement(tag));
  if (cls) el.className = cls;
  if (text !== undefined) el.textContent = text;
  return el;
}

/**
 * A ball with its letter inside (H white, O red, C dark grey, N blue).
 * @param {Document} doc
 * @param {string} el
 */
function ball(doc, el) {
  const b = mk(doc, 'span', `bench-ball el-${el}`, el);
  b.setAttribute('data-el', el);
  return b;
}

/**
 * The build view for one spell at one band, without the list, the chooser,
 * the modal handling, the focus trap or inert. `recipe` may be null (no
 * recipe card). `onMake(text)` gets the composed text, the same text the
 * bench sends in `try`. Every placeable piece carries `data-token`.
 * @param {HTMLElement} root
 * @param {BuilderOpts} opts
 * @returns {{ destroy: () => void }}
 */
export function mountBuilder(root, opts) {
  const doc = /** @type {Document} */ (root.ownerDocument);
  const band = bandOf(opts.band) || 'apprentice';
  const recipe = Array.isArray(opts.recipe) && opts.recipe.length ? opts.recipe : null;
  const spell = opts.spell || { slot: 0, id: '', name: '', formula: '', stage: 'unknown', prompt: '' };
  const make = (/** @type {string} */ text) => { try { opts.onMake(text); } catch { /* the caller's problem */ } };
  const reaction = !!recipe && recipe.some((t) => t.side === 1);

  root.replaceChildren();
  const wrap = mk(doc, 'div', `bench-build band-${band}`);
  const head = mk(doc, 'div', 'bench-build-head');
  const icon = mk(doc, 'span', `icon ${ICONS[spell.id] || 'water'} bench-build-icon`);
  icon.setAttribute('aria-hidden', 'true');
  head.append(icon, mk(doc, 'h2', 'bench-build-name', pretty(spell.name)));
  wrap.append(head);
  if (opts.prompt) {
    const row = mk(doc, 'div', 'bench-prompt-row');
    const say = mk(doc, 'button', 'bench-say');
    say.setAttribute('type', 'button');
    say.setAttribute('aria-label', 'Say it again');
    const glyph = mk(doc, 'span', 'bench-say-glyph');
    glyph.setAttribute('aria-hidden', 'true');
    say.append(glyph);
    const promptText = opts.prompt;
    say.addEventListener('click', () => speak(promptText));
    row.append(mk(doc, 'p', 'bench-prompt', promptText), say);
    wrap.append(row);
  }

  if (band === 'sprout') {
    // Recipe card: the balls the spell needs, grouped per element.
    if (recipe) {
      const card = mk(doc, 'div', 'bench-recipe');
      card.setAttribute('aria-label', 'Recipe');
      for (const t of recipe) {
        for (const [el, n] of t.atoms) {
          const group = mk(doc, 'span', 'bench-group');
          for (let k = 0; k < n * Math.max(1, t.coef || 1); k++) group.append(ball(doc, el));
          card.append(group);
        }
      }
      wrap.append(card);
    }
    /** @type {string[]} */
    const placed = [];
    const tray = mk(doc, 'div', 'bench-tray');
    tray.setAttribute('aria-label', 'Your build');
    const drawTray = () => {
      tray.replaceChildren(...placed.map((el) => ball(doc, el)));
      tray.setAttribute('data-count', String(placed.length));
    };
    drawTray();
    const cards = mk(doc, 'div', 'bench-atoms');
    for (const el of ELEMENTS) {
      const b = mk(doc, 'button', 'bench-atom');
      b.setAttribute('type', 'button');
      b.setAttribute('data-el', el);
      b.setAttribute('data-token', el);
      b.setAttribute('aria-label', `Add ${el}`);
      b.append(ball(doc, el));
      b.addEventListener('click', () => { if (placed.length < 12) { placed.push(el); drawTray(); } });
      cards.append(b);
    }
    const undo = mk(doc, 'button', 'bench-undo', 'Undo');
    undo.setAttribute('type', 'button');
    undo.addEventListener('click', () => { placed.pop(); drawTray(); });
    const makeBtn = mk(doc, 'button', 'bench-make', 'Make');
    makeBtn.setAttribute('type', 'button');
    makeBtn.addEventListener('click', () => make(composeSprout(placed, recipe)));
    const actions = mk(doc, 'div', 'bench-actions');
    actions.append(undo, makeBtn);
    wrap.append(tray, cards, actions);
  } else if (band === 'seedling' && reaction && recipe) {
    // Fire: coefficient tiles under a balance scale of atom counts.
    const coefs = recipe.map(() => 1);
    const scale = mk(doc, 'div', 'bench-scale');
    const beam = mk(doc, 'div', 'bench-beam');
    const pivot = mk(doc, 'div', 'bench-pivot');
    pivot.setAttribute('aria-hidden', 'true');
    /** @param {string} cls @param {string} label */
    const pan = (cls, label) => {
      const p = mk(doc, 'div', `bench-pan ${cls}`);
      const count = mk(doc, 'div', 'bench-pan-count');
      const more = mk(doc, 'div', 'bench-pan-more', '');
      p.append(mk(doc, 'div', 'bench-pan-label', label), count, more);
      return { p, count, more };
    };
    const L = pan('left', 'Before the arrow');
    const R = pan('right', 'After the arrow');
    const levelMark = mk(doc, 'div', 'bench-level');
    beam.append(L.p, R.p);
    scale.append(beam, pivot, levelMark);
    const row = mk(doc, 'div', 'bench-equation');
    /** @type {Array<{ step: (d: number) => void }>} */
    const tiles = [];
    const drawScale = () => {
      const s = scaleOf(recipe, coefs);
      const order = recipeOrder(recipe);
      /** @param {Record<string, number>} p */
      const text = (p) => order.map((el) => `${el} ${p[el] || 0}`).join(', ');
      L.count.textContent = text(s.left);
      R.count.textContent = text(s.right);
      L.more.textContent = s.tilt < 0 ? 'more atoms' : '';
      R.more.textContent = s.tilt > 0 ? 'more atoms' : '';
      scale.setAttribute('data-level', String(s.level));
      scale.setAttribute('data-tilt', String(s.tilt));
      beam.style.setProperty('--tilt', String(s.tilt));
      levelMark.textContent = s.level ? 'Level' : '';
    };
    recipe.forEach((t, i) => {
      if (i > 0) row.append(mk(doc, 'span', 'bench-op', recipe[i - 1]?.side !== t.side ? '→' : '+'));
      const term = mk(doc, 'span', 'bench-term');
      const c = mk(doc, 'button', 'bench-coef', '1');
      c.setAttribute('type', 'button');
      c.setAttribute('data-term', String(i));
      c.setAttribute('data-token', `coef:${i}`);
      const label = () => c.setAttribute('aria-label', `${coefs[i]} ${t.formula}`);
      const step = (/** @type {number} */ d) => {
        coefs[i] = (((coefs[i] ?? 1) - 1 + d + 4) % 4) + 1;
        c.textContent = String(coefs[i]);
        label();
        drawScale();
      };
      label();
      c.addEventListener('click', () => step(1));
      c.addEventListener('keydown', (/** @type {any} */ e) => {
        if (e.key === 'ArrowUp' || e.key === 'ArrowDown') { e.preventDefault(); step(e.key === 'ArrowUp' ? 1 : -1); }
      });
      /** @type {any} */ (c)._benchStep = step;
      tiles.push({ step });
      term.append(c, mk(doc, 'span', 'bench-mol', pretty(t.formula, false)));
      row.append(term);
    });
    drawScale();
    const makeBtn = mk(doc, 'button', 'bench-make', 'Make');
    makeBtn.setAttribute('type', 'button');
    makeBtn.addEventListener('click', () => make(composeEquation(recipe, coefs)));
    const actions = mk(doc, 'div', 'bench-actions');
    actions.append(makeBtn);
    wrap.append(scale, row, actions);
  } else if (band === 'seedling') {
    // One substance: the formula with subscripts, and a count tile per element.
    /** @type {Record<string, number>} */
    const want = {};
    if (recipe) {
      for (const t of recipe) for (const [el, n] of t.atoms) want[el] = (want[el] || 0) + n;
      const card = mk(doc, 'div', 'bench-recipe bench-formula', pretty(recipe.map((t) => t.formula).join(' + '), false));
      card.setAttribute('aria-label', 'Recipe');
      wrap.append(card);
    }
    /** @type {Record<string, number>} */
    const counts = {};
    const grid = mk(doc, 'div', 'bench-tiles');
    for (const el of ELEMENTS) {
      counts[el] = 0;
      const b = mk(doc, 'button', 'bench-tile');
      b.setAttribute('type', 'button');
      b.setAttribute('data-el', el);
      b.setAttribute('data-token', el);
      const letter = ball(doc, el);
      const num = mk(doc, 'span', 'bench-tile-count', '0');
      const tick = mk(doc, 'span', 'bench-tick');
      tick.setAttribute('aria-hidden', 'true');
      b.append(letter, num, tick);
      const draw = () => {
        num.textContent = String(counts[el]);
        b.setAttribute('aria-label', `${el}: ${counts[el]}`);
        b.setAttribute('data-match', String(!!recipe && (counts[el] || 0) === (want[el] || 0) && (counts[el] || 0) > 0));
      };
      const step = (/** @type {number} */ d) => { counts[el] = (((counts[el] || 0) + d) % 5 + 5) % 5; draw(); };
      draw();
      b.addEventListener('click', () => step(1));
      b.addEventListener('keydown', (/** @type {any} */ e) => {
        if (e.key === 'ArrowUp' || e.key === 'ArrowDown') { e.preventDefault(); step(e.key === 'ArrowUp' ? 1 : -1); }
      });
      /** @type {any} */ (b)._benchStep = step;
      grid.append(b);
    }
    const makeBtn = mk(doc, 'button', 'bench-make', 'Make');
    makeBtn.setAttribute('type', 'button');
    makeBtn.addEventListener('click', () => make(composeSeedling(recipe, counts)));
    const actions = mk(doc, 'div', 'bench-actions');
    actions.append(makeBtn);
    wrap.append(grid, actions);
  } else {
    // Apprentice: type the formula; a live preview shows subscripts.
    const box = mk(doc, 'div', 'bench-typed');
    const input = /** @type {HTMLInputElement} */ (mk(doc, 'input', 'bench-input'));
    input.setAttribute('id', 'bench-input');
    input.setAttribute('type', 'text');
    input.setAttribute('maxlength', '60');
    input.setAttribute('autocomplete', 'off');
    input.setAttribute('spellcheck', 'false');
    input.setAttribute('aria-label', 'Formula');
    input.setAttribute('data-token', 'text');
    const preview = mk(doc, 'div', 'bench-preview', '');
    preview.setAttribute('aria-hidden', 'true');
    const send = () => make(String(input.value || '').trim().replace(/\s+/g, ' '));
    input.addEventListener('input', () => { preview.textContent = pretty(String(input.value || '')); });
    input.addEventListener('keydown', (/** @type {any} */ e) => { if (e.key === 'Enter') { e.preventDefault(); send(); } });
    /** @type {any} */ (input)._benchMake = send;
    const makeBtn = mk(doc, 'button', 'bench-make', 'Make');
    makeBtn.setAttribute('type', 'button');
    makeBtn.addEventListener('click', send);
    box.append(input, preview);
    const actions = mk(doc, 'div', 'bench-actions');
    actions.append(makeBtn);
    wrap.append(box, actions);
  }
  root.append(wrap);
  return { destroy: () => root.replaceChildren() };
}

// ---------------------------------------------------------------------------
// The bench: modal, chooser, list, build, done and fizzle
// ---------------------------------------------------------------------------

let started = false;

export function startBench() {
  if (started || typeof window === 'undefined' || typeof document === 'undefined') return;
  started = true;
  const w = /** @type {any} */ (window);
  if (!w.__gameIn) w.__gameIn = {};
  if (!Array.isArray(w.__gameIn.bench)) w.__gameIn.bench = [];
  const debug = new URLSearchParams(location.search).has('debug');
  const push = (/** @type {string} */ c) => {
    const gi = w.__gameIn;
    if (!gi) return;
    if (!Array.isArray(gi.bench)) gi.bench = [];
    gi.bench.push(c);
  };

  // ---- state ----
  /** @type {View} */
  let rust = { v: 1, atBench: false, open: false };
  let rustJson = '';
  /** @type {View | null} */
  let fakeView = null;
  /** @type {string | null} */
  let picked = null;
  /** @type {string | null} */
  let chosen = null;
  let explicit = false;
  /** @type {string | null} */
  let sentBand = null;
  const ui = {
    open: false,
    /** @type {string | null} */
    band: null,
    /** @type {'chooser' | 'list' | 'build' | 'done' | 'fizzle'} */
    view: /** @type {'chooser' | 'list' | 'build' | 'done' | 'fizzle'} */ ('chooser'),
    pick: 0,
  };
  let wasLocked = false;
  let wasPaused = false;
  /** @type {Array<[HTMLElement, boolean]>} */
  let behind = [];
  let shownN = 0;
  let awaiting = false;
  /** @type {Record<number, number>} */
  let misses = {};
  /** @type {Names} */
  const names = {};
  for (const b of document.querySelectorAll('#hotbar .slot[data-spell]')) {
    const slot = +(/** @type {HTMLElement} */ (b).dataset.spell || 0);
    const n = b.querySelector('.name');
    if (slot && n) names[slot] = { name: n.textContent || '', formula: '' };
  }

  const view = () => fakeView || rust;
  const resolveBand = () => {
    const b = explicit ? bandOf(chosen) : bandFrom(location.search, picked, chosen);
    ui.band = b;
    if (b && b !== sentBand) {
      push(`band ${b}`);
      sentBand = b;
    }
    return b;
  };

  // ---- DOM ----
  const root = mk(document, 'div');
  root.id = 'bench';
  root.setAttribute('role', 'dialog');
  root.setAttribute('aria-modal', 'true');
  root.setAttribute('aria-labelledby', 'bench-title');
  root.hidden = true;
  const panel = mk(document, 'div', 'bench-panel');
  const header = mk(document, 'div', 'bench-head');
  const title = mk(document, 'h1', 'bench-title', 'Formula Bench');
  title.id = 'bench-title';
  const change = mk(document, 'button', 'bench-change', 'Change age');
  change.setAttribute('type', 'button');
  const closeBtn = mk(document, 'button', 'bench-close', 'Close');
  closeBtn.setAttribute('type', 'button');
  header.append(title, change, closeBtn);
  const body = mk(document, 'div', 'bench-body');
  panel.append(header, body);
  root.append(panel);
  document.body.append(root);

  // The HUD button: a flask and the key cap, top right.
  const btn = mk(document, 'button');
  btn.id = 'benchbtn';
  btn.setAttribute('type', 'button');
  btn.setAttribute('aria-label', 'Formula Bench (B)');
  const flask = mk(document, 'span', 'bench-flask');
  flask.setAttribute('aria-hidden', 'true');
  const key = mk(document, 'span', 'bench-key', 'B');
  key.setAttribute('aria-hidden', 'true');
  btn.append(flask, key);
  document.body.append(btn);

  // The controls list on the pause card gains the bench key (GS-24).
  try {
    const table = document.querySelector('#pause .card table');
    if (table) {
      const tr = mk(document, 'tr');
      tr.append(mk(document, 'td', '', 'Formula Bench'), mk(document, 'td', '', 'B'));
      const rows = [...table.querySelectorAll('tr')];
      const after = rows.find((r) => (r.querySelector('td')?.textContent || '').trim() === 'Read the last message again');
      if (after) after.after(tr);
      else (/** @type {HTMLTableElement} */ (table).tBodies[0] || table).append(tr);
    }
  } catch {
    // No controls table: the button still shows the key.
  }

  // ---- the one modal gate ----
  const gateBlocked = () => {
    for (const el of document.querySelectorAll('[aria-modal="true"]')) {
      if (el === root) continue;
      const e = /** @type {HTMLElement} */ (el);
      if (e.hidden || e.classList.contains('hidden')) continue;
      const r = e.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) return true;
    }
    return false;
  };

  // ---- focus ----
  const focusables = () => /** @type {HTMLElement[]} */ ([...root.querySelectorAll('button, input, [tabindex]:not([tabindex="-1"])')])
    .filter((el) => !(/** @type {HTMLButtonElement} */ (el).disabled) && el.getClientRects().length > 0);
  const focusFirst = () => {
    const input = /** @type {HTMLElement | null} */ (root.querySelector('#bench-input'));
    const first = input || body.querySelector('button') || focusables()[0];
    /** @type {HTMLElement | null} */ (first)?.focus();
  };
  const moveFocus = (/** @type {number} */ d) => {
    const list = focusables();
    if (!list.length) return;
    const i = list.indexOf(/** @type {HTMLElement} */ (document.activeElement));
    const next = list[i < 0 ? 0 : (i + d + list.length) % list.length];
    next?.focus();
  };

  // ---- rendering ----
  /** @type {{ destroy: () => void } | null} */
  let builder = null;
  let builderKey = '';
  let listKey = '';
  /** @type {HTMLElement | null} */
  let feedback = null;

  const clearBody = () => {
    if (builder) builder.destroy();
    builder = null;
    builderKey = '';
    listKey = '';
    feedback = null;
    body.replaceChildren();
  };

  const noteNames = (/** @type {SpellCard[] | undefined} */ spells) => {
    for (const s of spells || []) names[s.slot] = { name: pretty(s.name), formula: s.formula };
  };

  /** @param {string} stage */
  const badge = (stage) => {
    const b = mk(document, 'span', 'bench-stage');
    b.setAttribute('data-stage', stage);
    const shape = mk(document, 'span', 'bench-stage-shape');
    shape.setAttribute('aria-hidden', 'true');
    b.append(shape, mk(document, 'span', 'bench-stage-word', STAGE_WORD[stage] || 'New'));
    return b;
  };

  const showChooser = () => {
    clearBody();
    ui.view = 'chooser';
    const q = 'How old are you?';
    const wrap = mk(document, 'div', 'bench-chooser');
    wrap.append(mk(document, 'h2', 'bench-question', q));
    const row = mk(document, 'div', 'bench-bands');
    const labels = /** @type {Record<string, [string, string]>} */ ({
      sprout: ['4 to 7', '4 to 7 years old'],
      seedling: ['8 to 10', '8 to 10 years old'],
      apprentice: ['11 and up', '11 years old and up'],
    });
    for (const b of BANDS) {
      const [label, aria] = labels[b] || ['', ''];
      const btnB = mk(document, 'button', 'bench-band');
      btnB.setAttribute('type', 'button');
      btnB.setAttribute('data-band', b);
      btnB.setAttribute('aria-label', aria);
      const svg = document.querySelector(`#age button[data-band="${b}"] svg`);
      if (svg) btnB.append(svg.cloneNode(true));
      btnB.append(mk(document, 'span', 'bench-band-label', label));
      btnB.addEventListener('focus', () => speak(label));
      btnB.addEventListener('pointerenter', () => speak(label));
      btnB.addEventListener('click', () => {
        chosen = b;
        explicit = true;
        resolveBand();
        showList();
      });
      row.append(btnB);
    }
    wrap.append(row);
    body.append(wrap);
    if (ui.open) focusFirst();
    speak(q);
  };

  const showList = () => {
    const v = view();
    const spells = (v.band === ui.band || fakeView) ? v.spells || [] : [];
    noteNames(spells);
    const keyNow = `list|${ui.band}|${JSON.stringify(spells)}`;
    if (ui.view === 'list' && listKey === keyNow) return;
    const had = /** @type {HTMLElement | null} */ (document.activeElement)?.closest?.('.bench-spell');
    const hadSlot = had ? had.getAttribute('data-slot') : null;
    clearBody();
    ui.view = 'list';
    ui.pick = 0;
    listKey = keyNow;
    const wrap = mk(document, 'div', `bench-list band-${ui.band}`);
    wrap.append(mk(document, 'h2', 'bench-list-title', 'Pick a spell'));
    const grid = mk(document, 'div', 'bench-cards');
    for (const s of spells) {
      const card = mk(document, 'button', 'bench-spell');
      card.setAttribute('type', 'button');
      card.setAttribute('data-slot', String(s.slot));
      card.setAttribute('aria-label', `${pretty(s.name)}: ${STAGE_WORD[s.stage] || 'New'}`);
      const icon = mk(document, 'span', `icon ${ICONS[s.id] || 'water'} bench-spell-icon`);
      icon.setAttribute('aria-hidden', 'true');
      card.append(icon, mk(document, 'span', 'bench-spell-name', pretty(s.name)), badge(s.stage));
      card.addEventListener('click', () => pickSpell(s.slot));
      grid.append(card);
    }
    wrap.append(grid);
    body.append(wrap);
    if (ui.open) {
      const again = hadSlot ? /** @type {HTMLElement | null} */ (body.querySelector(`.bench-spell[data-slot="${hadSlot}"]`)) : null;
      if (again) again.focus(); else focusFirst();
    }
  };

  const pickSpell = (/** @type {number} */ slot) => {
    ui.pick = slot;
    ui.view = 'build';
    awaiting = false;
    push(`pick ${slot}`);
    const card = (view().spells || []).find((s) => s.slot === slot);
    if (card) speak(card.prompt);
    showBuild();
  };

  const showBuild = () => {
    const v = view();
    const card = (v.spells || []).find((s) => s.slot === ui.pick);
    if (!card) return;
    const band = ui.band || 'apprentice';
    const needRecipe = band !== 'apprentice';
    const recipe = v.pick === ui.pick && v.recipe ? v.recipe : null;
    // Sprout and Seedling wait one frame for the recipe the game publishes.
    if (needRecipe && !recipe && !fakeView) {
      if (!builderKey) {
        clearBody();
        body.append(mk(document, 'p', 'bench-wait', 'Getting the bench ready'));
        builderKey = 'wait';
      }
      return;
    }
    const keyNow = `build|${band}|${ui.pick}|${JSON.stringify(recipe)}`;
    if (builderKey === keyNow) return;
    clearBody();
    builderKey = keyNow;
    const wrap = mk(document, 'div', 'bench-buildwrap');
    const back = mk(document, 'button', 'bench-back', 'All spells');
    back.setAttribute('type', 'button');
    back.addEventListener('click', () => { push('open'); showList(); });
    const mount = mk(document, 'div', 'bench-mount');
    feedback = mk(document, 'div', 'bench-feedback');
    feedback.setAttribute('aria-live', 'polite');
    wrap.append(back, mount, feedback);
    body.append(wrap);
    builder = mountBuilder(mount, {
      band,
      spell: card,
      recipe,
      prompt: card.prompt,
      onMake: (text) => {
        wakeAudio();
        awaiting = true;
        push(`try ${ui.pick} ${text}`.trimEnd());
      },
    });
    if (ui.open) focusFirst();
  };

  /** @param {Result} r */
  const showFizzle = (r) => {
    misses[r.slot] = (misses[r.slot] || 0) + 1;
    ui.view = 'fizzle';
    if (!feedback) return;
    const band = ui.band || 'apprentice';
    const line = reasonFor(band, r, names, misses[r.slot] || 0);
    const box = mk(document, 'div', 'bench-fizzle');
    const mark = mk(document, 'span', 'bench-mark cross');
    mark.setAttribute('aria-hidden', 'true');
    box.append(mark, mk(document, 'p', 'bench-reason', line));
    const diffs = r.diffs || [];
    if (band === 'sprout' && r.code === 'atoms_off') {
      // A picture diff: per element, the balls built against the balls wanted.
      const recipe = view().recipe || [];
      /** @type {Record<string, [number, number]>} */
      const rows = {};
      for (const t of recipe) for (const [el, n] of t.atoms) rows[el] = [n, n];
      for (const [el, have, want] of diffs) rows[el] = [have, want];
      const order = [...recipeOrder(recipe), ...Object.keys(rows).filter((el) => !recipeOrder(recipe).includes(el)).sort()];
      const list = mk(document, 'div', 'bench-diffs');
      for (const el of order) {
        const [have, want] = rows[el] || [0, 0];
        const okEl = have === want;
        const row = mk(document, 'div', 'bench-diff');
        row.setAttribute('data-el', el);
        row.setAttribute('data-ok', String(okEl));
        const haveBox = mk(document, 'span', 'bench-diff-have');
        for (let k = 0; k < have; k++) haveBox.append(ball(document, el));
        const wantBox = mk(document, 'span', 'bench-diff-want');
        for (let k = 0; k < want; k++) wantBox.append(ball(document, el));
        const m = mk(document, 'span', `bench-mark ${okEl ? 'tick' : 'cross'}`);
        m.setAttribute('aria-hidden', 'true');
        row.append(haveBox, wantBox, m);
        list.append(row);
      }
      box.append(list);
    } else if (band === 'seedling' && r.code === 'unbalanced') {
      const list = mk(document, 'div', 'bench-diffs');
      for (const [el, left, right] of diffs) {
        const row = mk(document, 'div', 'bench-diff', `${el}: ${left} before, ${right} after`);
        row.setAttribute('data-el', el);
        row.setAttribute('data-ok', 'false');
        list.append(row);
      }
      box.append(list);
    }
    feedback.replaceChildren(box);
    speak(line);
  };

  /** @param {Result} r */
  const showDone = (r) => {
    const v = view();
    const card = (v.spells || []).find((s) => s.slot === r.slot);
    const band = ui.band || 'apprentice';
    clearBody();
    ui.view = 'done';
    const line = reasonFor(band, { ...r, code: 'ok' }, names, 0);
    const wrap = mk(document, 'div', 'bench-done');
    const icon = mk(document, 'span', `icon ${ICONS[card ? card.id : ''] || 'water'} bench-done-icon`);
    icon.setAttribute('aria-hidden', 'true');
    const tick = mk(document, 'span', 'bench-mark tick big');
    tick.setAttribute('aria-hidden', 'true');
    wrap.append(icon, tick, mk(document, 'p', 'bench-done-line', line));
    if (band === 'apprentice') {
      if (r.simplest) wrap.append(mk(document, 'p', 'bench-done-extra', `Simplest form: ${pretty(r.simplest)}`));
      const coef = r.coef || 0;
      const formula = pretty((names[r.slot] || { formula: '' }).formula);
      if (coef > 1) wrap.append(mk(document, 'p', 'bench-done-extra', `A ${coef} in front would mean ${coef} molecules. One molecule is ${formula}.`));
    }
    if (r.learned) wrap.append(badge(r.stage || 'learning'));
    const row = mk(document, 'div', 'bench-actions');
    const again = mk(document, 'button', 'bench-again', 'Build another');
    again.setAttribute('type', 'button');
    again.addEventListener('click', () => { push('open'); showList(); });
    const leave = mk(document, 'button', 'bench-leave', 'Back to the game');
    leave.setAttribute('type', 'button');
    leave.addEventListener('click', () => closeBench('click'));
    row.append(again, leave);
    wrap.append(row);
    body.append(wrap);
    if (ui.open) again.focus();
    rise();
    speak(line);
  };

  // Called on every change of the published view (and of a fake view).
  const sync = () => {
    if (!ui.open) return;
    const v = view();
    noteNames(v.spells);
    if (!ui.band || ui.view === 'chooser') return;
    const r = v.result;
    if (r && typeof r.n === 'number' && r.n > shownN && (ui.view === 'build' || ui.view === 'fizzle') && r.slot === ui.pick && (awaiting || fakeView)) {
      shownN = r.n;
      awaiting = false;
      if (r.ok) showDone(r);
      else showFizzle(r);
      return;
    }
    if (ui.view === 'list') showList();
    else if (ui.view === 'build') showBuild();
  };

  // ---- open and close ----
  /** @param {string} via */
  const openBench = (via) => {
    if (ui.open || gateBlocked()) return false;
    void via;
    const canvas = document.getElementById('game');
    wasLocked = !!document.pointerLockElement && document.pointerLockElement === canvas;
    wasPaused = !!(w.__gameIn && w.__gameIn.paused);
    try { if (document.pointerLockElement) document.exitPointerLock(); } catch { /* nothing to free */ }
    try { w.__game && w.__game.pause(true); } catch { /* no HUD */ }
    behind = /** @type {Array<[HTMLElement, boolean]>} */ (['stage', 'hud', 'loading', 'pause']
      .map((id) => document.getElementById(id))
      .filter(Boolean)
      .map((el) => [el, /** @type {HTMLElement} */ (el).inert]));
    for (const [el] of behind) el.inert = true;
    btn.inert = true;
    ui.open = true;
    misses = {};
    awaiting = false;
    root.hidden = false;
    resolveBand();
    push('open');
    if (ui.band) { ui.view = 'list'; listKey = ''; showList(); } else showChooser();
    focusFirst();
    return true;
  };

  /** @param {string} via */
  const closeBench = (via) => {
    if (!ui.open) return false;
    ui.open = false;
    push('close');
    clearBody();
    root.hidden = true;
    for (const [el, was] of behind) el.inert = was;
    behind = [];
    btn.inert = false;
    fakeView = null;
    try { w.__game && w.__game.pause(wasPaused); } catch { /* no HUD */ }
    if (wasLocked && via !== 'escape') {
      try { w.__game && w.__game.lock(); } catch { /* hud's note covers a refusal */ }
    }
    document.getElementById('game')?.focus();
    return true;
  };

  closeBtn.addEventListener('click', () => closeBench('click'));
  change.addEventListener('click', () => { if (ui.open) showChooser(); });
  btn.addEventListener('click', () => { wakeAudio(); openBench('button'); });

  // ---- keys: window capture, keydown and keypress only (never keyup) ----
  const editable = (/** @type {EventTarget | null} */ t) => {
    const el = /** @type {HTMLElement | null} */ (t);
    return !!el && !!el.tagName && (/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable);
  };
  const onKey = (/** @type {KeyboardEvent} */ e) => {
    if (!ui.open) {
      if (e.type === 'keydown' && e.code === 'KeyB' && !e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey && !editable(e.target) && !gateBlocked()) {
        e.stopImmediatePropagation();
        e.preventDefault();
        wakeAudio();
        openBench('key');
      }
      return;
    }
    e.stopImmediatePropagation();
    if (e.type !== 'keydown') return;
    const t = /** @type {any} */ (e.target);
    if (!t || typeof t.closest !== 'function' || !t.closest('#bench')) {
      if (e.key !== 'Escape') { e.preventDefault(); focusFirst(); return; }
    }
    if (e.key === 'Escape') { e.preventDefault(); closeBench('escape'); return; }
    if (e.key === 'Tab') { e.preventDefault(); moveFocus(e.shiftKey ? -1 : 1); return; }
    const inInput = !!t && t.tagName === 'INPUT';
    if (inInput && e.key === 'Enter') { e.preventDefault(); if (typeof t._benchMake === 'function') t._benchMake(); return; }
    if (String(e.key || '').startsWith('Arrow')) {
      if (inInput) return;
      if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && typeof t._benchStep === 'function') {
        e.preventDefault();
        t._benchStep(e.key === 'ArrowUp' ? 1 : -1);
        return;
      }
      e.preventDefault();
      moveFocus(e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : -1);
    }
  };
  window.addEventListener('keydown', onKey, true);
  window.addEventListener('keypress', onKey, true);
  // Focus stays inside the bench while it is open.
  document.addEventListener('focusin', (e) => {
    if (!ui.open) return;
    const t = /** @type {any} */ (e.target);
    if (!t || typeof t.closest !== 'function' || !t.closest('#bench')) focusFirst();
  }, true);

  // ---- the age picker's pick (age.js is not edited) ----
  for (const b of document.querySelectorAll('#age button[data-band]')) {
    b.addEventListener('click', () => {
      picked = /** @type {HTMLElement} */ (b).dataset.band || null;
      resolveBand();
    });
  }

  // ---- the per-frame poll: the published view and the button's classes ----
  const cls = { near: false, hint: false, locked: false };
  let hintDone = false;
  const setCls = (/** @type {'near' | 'hint' | 'locked'} */ k, /** @type {boolean} */ on) => {
    if (cls[k] === on) return;
    cls[k] = on;
    btn.classList.toggle(k, on);
  };
  const poll = () => {
    try {
      const js = w.__benchState;
      if (typeof js === 'string' && js !== rustJson) {
        rustJson = js;
        try { rust = JSON.parse(js); } catch { /* keep the last good view */ }
        sync();
      }
      setCls('near', !!rust.atBench);
      setCls('locked', !!document.pointerLockElement);
      // The attention ring: after the first cast, until any spell is past Unknown.
      if (!hintDone) {
        const gs = w.__gameState;
        if (typeof gs === 'string') {
          if (/"st":"(?:learning|practised|mastered)"/.test(gs)) hintDone = true;
          const m = /"casts":(\d+)/.exec(gs);
          setCls('hint', !hintDone && !!m && +(m[1] || 0) >= 1);
        }
      } else setCls('hint', false);
    } catch {
      // The poll must never stop.
    }
    requestAnimationFrame(poll);
  };
  requestAnimationFrame(poll);

  // ---- test hook ----
  /** @type {any} */
  const hook = {
    state: () => { try { return JSON.parse(w.__benchState); } catch { return null; } },
    ui: () => ({ open: ui.open, band: ui.band, view: ui.view, pick: ui.pick }),
    open: () => openBench('api'),
    close: () => closeBench('api'),
    cmd: (/** @type {string} */ s) => push(String(s)),
  };
  if (debug) {
    // Renders a given BenchView as if the game had published it (it does
    // not write __benchState), so the panels can be built against the stub.
    hook.fake = (/** @type {View | null} */ v) => {
      if (!v) { fakeView = null; sync(); return; }
      if (!ui.open) openBench('fake');
      fakeView = v;
      if (v.band && bandOf(v.band)) ui.band = bandOf(v.band);
      noteNames(v.spells);
      const same = v.pick === ui.pick && (ui.view === 'build' || ui.view === 'fizzle');
      if (v.pick && !same) { ui.pick = v.pick; ui.view = 'build'; builderKey = ''; showBuild(); }
      else if (!v.pick && ui.view !== 'chooser') showList();
      sync();
    };
  }
  w.__bench = hook;

  resolveBand();
}
