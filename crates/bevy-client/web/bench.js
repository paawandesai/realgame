// The Formula Bench's builder (I-028) and the host of the one Bench (I-060).
// See docs/loop/items/I-028/spec.md (sections 1-3 and Contract) and
// docs/loop/items/I-060/spec.md.
//
// Loaded by boot.js with a guarded dynamic import, so a missing or broken
// bench.js costs only the bench, never the boot. The game drains
// window.__gameIn.bench (one command string each: `band <tier>`, `open`,
// `pick <slot>`, `learn <codex id>`, `try <slot> <text…>`, `close`) and
// writes window.__benchState (a JSON string) only when it changes.
// crates/sim's bench::judge is the only judge: nothing here decides right or
// wrong. The balance scale is display arithmetic (atom counts per side),
// never a verdict.
//
// The pure helpers and mountBuilder below build a formula at a band's level;
// the one Bench's Spells tab (bench-spells.js) and the worked casts
// (worked.js) mount them. startBench (at the end) opens the Chemistry Bench
// over the world: the B key, E at a Bench and the #benchbtn button, only
// while no other aria-modal dialog is showing (the one modal gate). While it
// is open the game is paused, the mouse is free, the page behind it is inert,
// and every keydown/keypress on this page is swallowed in the window capture
// phase before winit or hud.js can see it (keyup always passes, so a key held
// when the bench opened is released in the game).

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
export const ICONS = /** @type {Record<string, string>} */ ({ fire: 'fire', water: 'water', frost: 'frost', co2: 'foam' });
export const STAGE_WORD = /** @type {Record<string, string>} */ ({ unknown: 'New', learning: 'Learning', practised: 'Practised', mastered: 'Mastered' });
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
 * when every element matches, tilt = sign(right total - left total). The beam
 * is never drawn level unless it is level (round 2, break B1): when the totals
 * are equal but an element differs, it tilts toward the pan with more of the
 * first mismatched element in recipe order.
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
  let tilt = Math.sign(total(right) - total(left));
  if (!level && tilt === 0) {
    const el = order.find((e) => left[e] !== right[e]);
    tilt = el === undefined ? 0 : Math.sign((right[el] || 0) - (left[el] || 0));
  }
  return { left, right, level, tilt };
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
  // A verdict without diffs (never from the judge since round 2) falls back
  // to the line that names no counts, so the reason is never blank.
  const askCounts = `Not yet. Which atoms does ${name} have, and how many?`;
  switch (r.code) {
    case 'ok':
      return b === 'sprout' ? `You made ${name}!` : `${formula} is right! You are learning ${name}.`;
    case 'empty':
      return b === 'sprout' ? 'Tap the balls to build it.' : b === 'seedling' ? 'Tap the tiles to build it.' : 'Write a formula first.';
    case 'atoms_off':
      if (b === 'sprout') return 'Look again. Count the balls in the picture.';
      if ((b === 'seedling' || tries >= 3) && diffs.length) return counts();
      return askCounts;
    case 'other_spell':
      return `That makes ${other}. This one is ${name}.`;
    case 'unbalanced':
      if (b === 'seedling' || !diffs.length) return 'The scale is not level. Atoms are never made or lost, so both sides need the same atoms.';
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
      // P10: a one-substance spell needs no arrow, and lower-case symbols
      // are the commonest slip, so say both plainly.
      return /->|→/.test(me.formula || '')
        ? 'Use element symbols (capital first letter), numbers, + and an arrow ->.'
        : 'Write element symbols and numbers, like NaCl. Each symbol starts with a capital letter.';
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

/** Reading lines aloud is off unless the saved settings say `voice: true` (Paawan's walkthrough). */
function voiceOn() {
  try {
    const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) || 'null');
    return !!(s && typeof s === 'object' && s.voice === true);
  } catch {
    return false;
  }
}

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
export function speak(text) {
  try {
    if (typeof window === 'undefined' || !text) return;
    const syn = /** @type {any} */ (window).speechSynthesis;
    if (!syn || typeof syn.speak !== 'function' || !soundOn() || !voiceOn()) return;
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
export function wakeAudio() {
  try {
    if (audio) { if (audio.state === 'suspended') audio.resume().catch(() => {}); return; }
    const AC = window.AudioContext || /** @type {any} */ (window).webkitAudioContext;
    if (AC) audio = new AC();
  } catch {
    audio = null;
  }
}
/** A short rising tone for a correct build (peak gain at most PEAK, sound on only). */
export function rise() {
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
      // Per element "C 1" with a mark when the two pans differ (the same
      // text as before: the mark is drawn by CSS, never a colour alone).
      /** @param {HTMLElement} box @param {Record<string, number>} p @param {Record<string, number>} q */
      const counts = (box, p, q) => {
        /** @type {Array<HTMLElement | string>} */
        const parts = [];
        order.forEach((el, k) => {
          if (k > 0) parts.push(', ');
          const span = mk(doc, 'span', 'bench-pan-el', `${el} ${p[el] || 0}`);
          span.setAttribute('data-el', el);
          span.setAttribute('data-ok', String((p[el] || 0) === (q[el] || 0)));
          parts.push(span);
        });
        box.replaceChildren(...parts);
      };
      counts(L.count, s.left, s.right);
      counts(R.count, s.right, s.left);
      // "more atoms" only when the totals really differ (B1: equal totals
      // with different elements tilt the beam but neither pan has more).
      /** @param {Record<string, number>} p */
      const sum = (p) => Object.values(p).reduce((a, b) => a + b, 0);
      const more = Math.sign(sum(s.right) - sum(s.left));
      L.more.textContent = more < 0 ? 'more atoms' : '';
      R.more.textContent = more > 0 ? 'more atoms' : '';
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
// The one Bench (I-060): B opens the Chemistry Bench over the world
// ---------------------------------------------------------------------------
//
// The world's B key (and E at a Bench, and the #benchbtn button) shows the
// Chemistry Bench page (bench-free.html?mode=story) in a same-origin iframe
// over the world. Its Spells tab (bench-spells.js) learns and practises the
// spells through this page's `window.__gameIn.bench` queue, which bench.rs
// drains and judges (judge_with and the shared codex) as before; only the
// UI moved. This module keeps the rest of the old Bench's contract: the one
// modal gate, the paused world, the freed pointer lock (re-locked on close,
// or on the next click after Escape), the page behind made inert, every key
// swallowed while open, the band sent to the game, the bench chip and the
// button's rings. The iframe is made on the first open and kept (hidden), so
// the Bench reopens where it was.
//
// Test hook: window.__bench = { state(), ui(), open(), close(), cmd(s),
// frame(), band(), setBand(b), fixed(), ready() }.

/** The four bands (Tier ids); `bandOf` above keeps the builder's three. */
const BANDS4 = ['sprout', 'seedling', 'apprentice', 'adept'];
/** @param {unknown} b */
const band4 = (b) => (typeof b === 'string' && BANDS4.includes(b) ? b : null);
const FRAME_SRC = 'bench-free.html?mode=story';

let started = false;

export function startBench() {
  if (started || typeof window === 'undefined' || typeof document === 'undefined') return;
  started = true;
  const w = /** @type {any} */ (window);
  if (!w.__gameIn) w.__gameIn = {};
  if (!Array.isArray(w.__gameIn.bench)) w.__gameIn.bench = [];
  const push = (/** @type {string} */ c) => {
    const gi = w.__gameIn;
    if (!gi) return;
    if (!Array.isArray(gi.bench)) gi.bench = [];
    gi.bench.push(c);
  };

  // ---- the band: the URL, else the age picker, else a pick in the Bench ----
  /** @type {string | null} */
  let picked = null;
  /** @type {string | null} */
  let chosen = null;
  /** @type {string | null} */
  let sentBand = null;
  const urlBand = () => {
    try { return band4(new URLSearchParams(location.search).get('band')); } catch { return null; }
  };
  const resolveBand = () => {
    const b = urlBand() || chosen || picked;
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
  root.setAttribute('aria-label', 'Chemistry Bench');
  root.hidden = true;
  // Shown until the Bench page says it is ready (or if it never does).
  const wait = mk(document, 'div', 'bench-wait');
  wait.append(mk(document, 'p', 'bench-wait-line', 'Setting up the Bench…'));
  const waitBack = mk(document, 'button', 'bench-close', 'Back to the world');
  waitBack.setAttribute('type', 'button');
  wait.append(waitBack);
  root.append(wait);
  document.body.append(root);
  /** @type {HTMLIFrameElement | null} */
  let frame = null;
  let ready = false;
  const ensureFrame = () => {
    if (frame) return frame;
    frame = /** @type {HTMLIFrameElement} */ (document.createElement('iframe'));
    frame.id = 'bench-frame';
    frame.title = 'Chemistry Bench';
    frame.src = FRAME_SRC;
    root.append(frame);
    return frame;
  };
  /** The Bench page's story API, once it has started. */
  const story = () => { try { return /** @type {any} */ (frame?.contentWindow)?.__benchStory || null; } catch { return null; } };

  // The HUD button: a flask and the key cap, top right.
  const btn = mk(document, 'button');
  btn.id = 'benchbtn';
  btn.setAttribute('type', 'button');
  btn.setAttribute('aria-label', 'Chemistry Bench (B)');
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
      tr.append(mk(document, 'td', '', 'Chemistry Bench'), mk(document, 'td', '', 'B'));
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

  // ---- open and close ----
  const ui = { open: false };
  let wasLocked = false;
  let wasPaused = false;
  /** @type {Array<[HTMLElement, boolean]>} */
  let behind = [];
  const focusIn = () => {
    if (ready && frame) {
      try { frame.focus(); frame.contentWindow?.focus(); } catch { /* the frame takes it on click */ }
    } else waitBack.focus();
  };
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
    // The pause card is shown by the pause above; while the bench is open
    // it stays out of sight, and comes back on close if the game is still
    // paused (hud.js is not edited).
    document.getElementById('pause')?.classList.add('bench-under');
    ui.open = true;
    root.hidden = false;
    ensureFrame();
    wait.hidden = ready;
    resolveBand();
    push('open');
    try { story()?.opened(); } catch { /* the page catches up when it starts */ }
    focusIn();
    return true;
  };

  // A lock the click before B asked for can land after the Bench opened (the
  // HUD then unpauses): while the Bench is open the mouse stays free and the
  // world stays paused. Checked on the event and on a timer, since headless
  // Chromium does not always send the event.
  const keepFree = () => {
    if (!ui.open || !document.pointerLockElement) return;
    try { document.exitPointerLock(); } catch { /* nothing to free */ }
    try { w.__game && w.__game.pause(true); } catch { /* no HUD */ }
  };
  document.addEventListener('pointerlockchange', keepFree);
  setInterval(keepFree, 200);

  /** @param {string} via */
  const closeBench = (via) => {
    if (!ui.open) return false;
    ui.open = false;
    push('close');
    try { story()?.closed(); } catch { /* nothing to stop */ }
    root.hidden = true;
    for (const [el, was] of behind) el.inert = was;
    behind = [];
    document.getElementById('pause')?.classList.remove('bench-under');
    btn.inert = false;
    try { w.__game && w.__game.pause(wasPaused); } catch { /* no HUD */ }
    if (wasLocked && via !== 'escape') {
      try { w.__game && w.__game.lock(); } catch { /* hud's note covers a refusal */ }
    }
    document.getElementById('game')?.focus();
    return true;
  };

  waitBack.addEventListener('click', () => closeBench('click'));
  btn.addEventListener('click', () => { wakeAudio(); openBench('button'); });

  // ---- keys: window capture, keydown and keypress only (never keyup) ----
  // Keys typed inside the Bench page go to its own document and never reach
  // the game; these catch the ones that land on this page.
  const editable = (/** @type {EventTarget | null} */ t) => {
    const el = /** @type {HTMLElement | null} */ (t);
    return !!el && !!el.tagName && (/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable);
  };
  const onKey = (/** @type {KeyboardEvent} */ e) => {
    if (!ui.open) {
      // P10: E (the game's use key) opens it too while standing at a Bench.
      if (e.type === 'keydown' && (e.code === 'KeyB' || (e.code === 'KeyE' && atBench())) && !e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey && !editable(e.target) && !gateBlocked()) {
        e.stopImmediatePropagation();
        e.preventDefault();
        wakeAudio();
        openBench('key');
      }
      return;
    }
    e.stopImmediatePropagation();
    if (e.type !== 'keydown') return;
    if (e.key === 'Escape') { e.preventDefault(); closeBench('escape'); return; }
    const t = /** @type {any} */ (e.target);
    if (!t || typeof t.closest !== 'function' || !t.closest('#bench')) { e.preventDefault(); focusIn(); }
  };
  window.addEventListener('keydown', onKey, true);
  window.addEventListener('keypress', onKey, true);
  // Focus stays inside the bench while it is open.
  document.addEventListener('focusin', (e) => {
    if (!ui.open) return;
    const t = /** @type {any} */ (e.target);
    if (!t || typeof t.closest !== 'function' || !t.closest('#bench')) focusIn();
  }, true);

  // ---- the age picker's pick (age.js is not edited) ----
  for (const b of document.querySelectorAll('#age button[data-band]')) {
    b.addEventListener('click', () => {
      picked = band4(/** @type {HTMLElement} */ (b).dataset.band);
      resolveBand();
    });
  }

  // ---- the per-frame poll: the published view and the button's classes ----
  /** @type {{ atBench?: boolean }} */
  let rust = {};
  let rustJson = '';
  const atBench = () => !!rust.atBench;
  const cls = { near: false, hint: false, locked: false };
  let hintDone = false;
  const setCls = (/** @type {'near' | 'hint' | 'locked'} */ k, /** @type {boolean} */ on) => {
    if (cls[k] === on) return;
    cls[k] = on;
    btn.classList.toggle(k, on);
  };
  // Next to a Bench, a chip says what B does there (the first time, what to
  // learn), so the first objective's "learn Water" has an obvious next key.
  const chip = document.createElement('div');
  chip.id = 'bench-chip';
  chip.className = 'late-chip';
  chip.setAttribute('role', 'status');
  chip.hidden = true;
  document.body.append(chip);
  const poll = () => {
    try {
      const js = w.__benchState;
      if (typeof js === 'string' && js !== rustJson) {
        rustJson = js;
        try { rust = JSON.parse(js); } catch { /* keep the last good view */ }
      }
      setCls('near', atBench());
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
      const chipOn = atBench() && !ui.open && !document.body.classList.contains('title-open');
      const chipText = hintDone ? 'E or B: open the Chemistry Bench' : 'Press E to learn Water at the Chemistry Bench';
      if (chip.hidden === chipOn) chip.hidden = !chipOn;
      if (chipOn && chip.textContent !== chipText) chip.textContent = chipText;
    } catch {
      // The poll must never stop.
    }
    requestAnimationFrame(poll);
  };
  requestAnimationFrame(poll);

  // ---- the API the Bench page (story mode) and tests use ----
  /** @type {any} */
  const hook = {
    state: () => { try { return JSON.parse(w.__benchState); } catch { return null; } },
    ui: () => ({ open: ui.open, band: urlBand() || chosen || picked, ready, view: !ready ? 'wait' : (() => { try { return frame && frame.contentDocument && frame.contentDocument.getElementById('bandAsk')?.hidden === false ? 'chooser' : 'bench'; } catch { return 'bench'; } })() }),
    open: () => openBench('api'),
    close: (/** @type {string | undefined} */ via) => closeBench(via === 'escape' ? 'escape' : 'api'),
    cmd: (/** @type {string} */ s) => push(String(s)),
    frame: () => frame,
    /** The band the world knows (null: not asked yet). */
    band: () => urlBand() || chosen || picked,
    /** A band picked inside the Bench; a class link's ?band= wins. */
    setBand: (/** @type {string} */ b) => { if (band4(b) && !urlBand()) { chosen = b; resolveBand(); } },
    fixed: () => !!urlBand(),
    ready: () => { ready = true; wait.hidden = true; if (ui.open) { try { story()?.opened(); } catch { /* none */ } focusIn(); } },
  };
  w.__bench = hook;

  resolveBand();
}
