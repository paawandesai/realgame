// The Mastery trial and keep-it-sharp (I-030; docs/loop/items/I-030/spec.md,
// wave 14 revision). Drawn inside the Formula Bench (bench.js owns the modal,
// the keys and the focus trap); this file draws `window.__trialState.trial`
// and sends `trial`/`sharp`/`try`/`answer`/`again`/`endtrial` through
// ctx.push. Rust (src/trial.rs, src/bench.rs) judges every build and every
// answer: the right choice is not in the page until the player has answered.
//
// The trial: build the spell 3 times from memory (no recipe, no scale), each
// correct build filling a star pip; misses never cost. Then one "what
// happens?" question; a wrong answer shows the right one and why, then "Ask
// me again". Keep it sharp: one build from memory on a later day.

import { mountBuilder, reasonFor, pretty, speak, rise } from './bench.js';

/** @typedef {{ id: string, label: string, icon: string }} Choice */
/** @typedef {{ id: string, prompt: string, choices: Choice[], answer: string | null, why: string | null }} Question */
/** @typedef {{ spell: number, slot: number, kind: string, phase: string, built: number, need: number, misses: number, asks: number, right: boolean | null, mastered: boolean, q: Question | null }} Trial */
/** @typedef {{ v?: number, sharp?: number[], trial?: Trial | null }} TrialState */
/** @typedef {{ slot: number, id: string, name: string, formula: string, stage: string, prompt: string }} SpellCard */
/** @typedef {{ n?: number, slot: number, ok?: boolean, code: string, diffs?: Array<[string, number, number]> }} Result */
/** @typedef {{ band?: string, spells?: SpellCard[], pick?: number, recipe?: any[], result?: Result }} BenchView */
/** @typedef {{ key?: string, builder?: { destroy: () => void } | null, feedback?: HTMLElement | null, shownN?: number, sentAt?: number, answered?: boolean, spoken?: string, rose?: string }} Mem */
/** @typedef {{ push: (c: string) => void, band?: string | null, close?: () => void, mem?: Mem }} Ctx */

const ICON_CLASS = /** @type {Record<string, string>} */ ({ fire: 'fire', water: 'water', frost: 'frost', co2: 'foam' });
const IDS = ['fire', 'water', 'frost', 'co2'];

// The Sprout pictures (ICONS in src/trial.rs): small flat drawings, each with
// at least two shapes. Colour is never the only cue: each card has a label.
const FLAME = '<path d="M 32 10 C 40 22 46 28 44 38 C 42 47 22 47 20 38 C 18 28 26 24 32 10 Z" fill="#ff7a1a"/><path d="M 32 24 C 36 31 38 34 37 39 C 36 43 28 43 27 39 C 26 34 29 31 32 24 Z" fill="#ffd23a"/>';
const LOGS = '<rect x="12" y="46" width="40" height="7" rx="3" fill="#8a5a2b" transform="rotate(-10 32 50)"/><rect x="12" y="46" width="40" height="7" rx="3" fill="#6e4520" transform="rotate(10 32 50)"/>';
const svg = (/** @type {string} */ inner) => `<svg viewBox="0 0 64 64" width="96" height="96" aria-hidden="true">${inner}</svg>`;
/** @type {Record<string, string>} */
export const PICTURES = {
  fire: svg(`${LOGS}${FLAME}`),
  fire_out: svg(`${LOGS}<path d="M 24 40 C 20 32 30 30 26 22" stroke="#9aa3b5" stroke-width="3" fill="none"/><path d="M 36 40 C 32 30 42 28 38 18" stroke="#9aa3b5" stroke-width="3" fill="none"/><circle cx="18" cy="16" r="3" fill="#4aa8ff"/><circle cx="46" cy="12" r="3" fill="#4aa8ff"/>`),
  fire_big: svg(`${LOGS}<path d="M 32 2 C 46 18 56 28 52 42 C 48 54 16 54 12 42 C 8 28 20 20 32 2 Z" fill="#ff5a1a"/><path d="M 32 18 C 40 28 44 34 42 42 C 40 50 24 50 22 42 C 20 34 26 28 32 18 Z" fill="#ffd23a"/>`),
  ice: svg('<rect x="14" y="16" width="36" height="36" rx="4" fill="#bfe8ff" stroke="#5fb8ee" stroke-width="3"/><path d="M 20 24 L 30 22" stroke="#ffffff" stroke-width="3" stroke-linecap="round"/><path d="M 20 30 L 26 29" stroke="#ffffff" stroke-width="3" stroke-linecap="round"/>'),
  balloon_big: svg('<ellipse cx="32" cy="26" rx="22" ry="24" fill="#ff4f7a"/><polygon points="29,50 35,50 32,55" fill="#d93a62"/><path d="M 32 55 C 28 59 36 61 32 64" stroke="#555" stroke-width="2" fill="none"/>'),
  balloon_small: svg('<ellipse cx="32" cy="40" rx="8" ry="9" fill="#ff4f7a"/><polygon points="30,49 34,49 32,52" fill="#d93a62"/><path d="M 32 52 C 29 56 35 58 32 62" stroke="#555" stroke-width="2" fill="none"/><path d="M 14 18 L 20 24 M 50 18 L 44 24 M 32 8 L 32 16" stroke="#9aa3b5" stroke-width="3" stroke-linecap="round"/>'),
  candle_out: svg('<rect x="24" y="28" width="16" height="28" rx="2" fill="#f4efe2" stroke="#c9bfa5" stroke-width="2"/><path d="M 32 28 L 32 22" stroke="#333" stroke-width="2"/><path d="M 32 20 C 28 14 36 12 32 6" stroke="#9aa3b5" stroke-width="3" fill="none"/>'),
};

/** @param {unknown} b */
function bandOf(b) {
  if (b === 'adept') return 'apprentice';
  return b === 'sprout' || b === 'seedling' || b === 'apprentice' ? b : 'apprentice';
}

/**
 * The line over the builder (Contract strings).
 * @param {string} band
 * @param {number} built correct builds so far (0 for keep it sharp)
 * @param {string} kind 'master' or 'sharp'
 * @param {string} spellName the shown name, for example 'Water'
 * @param {boolean} [reaction] a balancing build (Fire); defaults to the name
 */
export function lineFor(band, built, kind, spellName, reaction) {
  const name = spellName;
  if (kind !== 'sharp' && built === 1) return 'Again!';
  if (kind !== 'sharp' && built === 2) return 'Last one!';
  const b = bandOf(band);
  const fire = reaction === undefined ? /^fire$/i.test(String(spellName || '')) : reaction;
  if (b === 'sprout') return `Make ${name} from memory!`;
  if (b === 'seedling') return fire ? `Balance ${name} without the scale.` : `Build ${name} from memory. No recipe this time.`;
  return `Write ${name} from memory.`;
}

/**
 * The three star pips: on for each correct build.
 * @param {number} built
 * @returns {boolean[]}
 */
export function pipStates(built) {
  const n = Number(built) || 0;
  return [n >= 1, n >= 2, n >= 3];
}

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
 * @param {Document} doc
 * @param {string} cls
 * @param {string} text
 * @param {() => void} onClick
 */
function button(doc, cls, text, onClick) {
  const b = mk(doc, 'button', cls, text);
  b.setAttribute('type', 'button');
  b.addEventListener('click', onClick);
  return b;
}

/**
 * Draws the open trial into `body`. Call it on every change of the trial or
 * the bench view; it keeps the builder (and what the player has tapped) until
 * the trial moves on. `ctx.mem` holds that memory between calls (made on
 * first use); the bench resets it when it clears its body.
 * @param {HTMLElement} body
 * @param {TrialState} trialState
 * @param {BenchView} benchView
 * @param {Ctx} ctx
 */
export function renderTrial(body, trialState, benchView, ctx) {
  const t = trialState && trialState.trial;
  if (!t || !body) return;
  const doc = /** @type {Document} */ (body.ownerDocument);
  const mem = ctx.mem || (ctx.mem = {});
  const v = benchView || {};
  const band = bandOf(ctx.band || v.band);
  const card = (v.spells || []).find((s) => s.slot === t.slot)
    || { slot: t.slot, id: IDS[t.spell] || '', name: '', formula: '', stage: '', prompt: '' };
  const name = pretty(card.name);
  const recipe = v.pick === t.slot && Array.isArray(v.recipe) && v.recipe.length ? v.recipe : null;
  const needRecipe = band !== 'apprentice';
  const sharp = t.kind === 'sharp';
  const q = t.q;
  const key = [t.slot, t.kind, t.phase, t.built, t.asks, String(t.right), band, needRecipe && recipe ? 'r' : '-', q ? q.answer : ''].join('|');
  const push = (/** @type {string} */ c) => { try { ctx.push(c); } catch { /* no game */ } };
  const back = () => push('endtrial');
  const result = v.result;

  if (mem.key !== key) {
    if (mem.builder) { try { mem.builder.destroy(); } catch { /* gone */ } }
    mem.builder = null;
    mem.feedback = null;
    mem.answered = false;
    mem.key = key;
    // A result shown before this view (the build that moved it on) is old.
    mem.shownN = Math.max(mem.shownN || 0, result && typeof result.n === 'number' ? result.n : 0);
    body.replaceChildren();
    const wrap = mk(doc, 'div', `trial band-${band}`);
    wrap.setAttribute('data-phase', t.phase);
    wrap.setAttribute('data-kind', t.kind);
    const head = mk(doc, 'div', 'trial-head');
    const icon = mk(doc, 'span', `icon ${ICON_CLASS[card.id] || 'water'} trial-icon`);
    icon.setAttribute('aria-hidden', 'true');
    head.append(icon, mk(doc, 'h2', 'trial-title', sharp ? `Keep ${name} sharp` : `Master ${name}`));
    if (!sharp) {
      const pips = mk(doc, 'div', 'trial-pips');
      pips.setAttribute('role', 'img');
      pips.setAttribute('aria-label', `${t.built} of 3`);
      for (const on of pipStates(t.built)) {
        const pip = mk(doc, 'span', 'trial-pip', '★');
        pip.setAttribute('data-on', String(on));
        pip.setAttribute('aria-hidden', 'true');
        pips.append(pip);
      }
      head.append(pips);
    }
    wrap.append(head);
    body.append(wrap);

    if (t.phase === 'build') {
      const line = lineFor(band, t.built, t.kind, name, card.id === 'fire');
      const lineEl = mk(doc, 'p', 'trial-line', line);
      lineEl.setAttribute('aria-live', 'polite');
      const mount = mk(doc, 'div', 'trial-mount');
      const feedback = mk(doc, 'div', 'trial-feedback');
      feedback.setAttribute('aria-live', 'polite');
      mem.feedback = feedback;
      const row = mk(doc, 'div', 'bench-actions trial-nav');
      row.append(button(doc, 'bench-back trial-back', 'All spells', back));
      wrap.append(lineEl, mount, feedback, row);
      if (needRecipe && !recipe) {
        mount.append(mk(doc, 'p', 'bench-wait', 'Getting the bench ready'));
        mem.key = '';
      } else {
        const mountIt = () => {
          if (mem.builder) { try { mem.builder.destroy(); } catch { /* gone */ } }
          mem.builder = mountBuilder(mount, {
            band,
            spell: card,
            recipe,
            prompt: '',
            unaided: true,
            onMake: (text) => {
              // One try in flight at a time (a double-clicked Make sends one);
              // the guard lapses after 2 s so a lost result never locks it.
              const now = Date.now();
              if (mem.sentAt && now - mem.sentAt < 2000) return;
              mem.sentAt = now;
              push(`try ${t.slot} ${text}`.trimEnd());
            },
          });
        };
        mountIt();
        mem.sentAt = 0;
        /** @type {any} */ (mem).remount = mountIt;
        if (mem.spoken !== key && band === 'sprout') { mem.spoken = key; speak(line); }
        if (t.built > 0 && mem.rose !== key) { mem.rose = key; rise(); }
      }
    } else if (t.phase === 'ask' || t.phase === 'answered') {
      if (!q) return;
      const box = mk(doc, 'div', 'trial-q');
      const said = band === 'sprout' ? `What happens? ${q.prompt}` : q.prompt;
      if (band === 'sprout') {
        const lead = mk(doc, 'h3', 'trial-lead', 'What happens?');
        const scene = mk(doc, 'div', 'trial-scene');
        const sceneIcon = mk(doc, 'span', `icon ${ICON_CLASS[card.id] || 'water'}`);
        sceneIcon.setAttribute('aria-hidden', 'true');
        scene.append(sceneIcon);
        box.append(lead, scene);
      }
      const promptRow = mk(doc, 'div', 'trial-prompt-row');
      promptRow.append(mk(doc, 'p', 'trial-prompt', q.prompt));
      const say = button(doc, 'bench-say trial-say', '', () => speak(said));
      say.setAttribute('aria-label', 'Say it again');
      const glyph = mk(doc, 'span', 'bench-say-glyph');
      glyph.setAttribute('aria-hidden', 'true');
      say.append(glyph);
      promptRow.append(say);
      box.append(promptRow);
      const answered = t.phase === 'answered';
      if (answered && t.right === false) {
        const wrong = mk(doc, 'p', 'trial-wrong');
        const cross = mk(doc, 'span', 'bench-mark cross');
        cross.setAttribute('aria-hidden', 'true');
        wrong.append(cross, mk(doc, 'span', '', 'Not quite. It is this one:'));
        box.append(wrong);
      }
      const grid = mk(doc, 'div', `trial-choices${band === 'sprout' ? ' pictures' : ''}`);
      for (const ch of q.choices || []) {
        const b = mk(doc, 'button', `trial-choice${band === 'sprout' ? '' : ' text'}`);
        b.setAttribute('type', 'button');
        b.setAttribute('data-choice', ch.id);
        if (band === 'sprout' && ch.icon && PICTURES[ch.icon]) {
          const pic = mk(doc, 'span', 'trial-pic');
          pic.innerHTML = PICTURES[ch.icon] || '';
          b.append(pic);
        }
        b.append(mk(doc, 'span', 'trial-label', ch.label));
        if (answered) {
          /** @type {HTMLButtonElement} */ (b).disabled = true;
          if (q.answer === ch.id) b.setAttribute('data-correct', 'true');
        } else {
          b.addEventListener('click', () => {
            if (mem.answered) return;
            mem.answered = true;
            push(`answer ${ch.id}`);
          });
          if (band === 'sprout') {
            b.addEventListener('focus', () => speak(ch.label));
          }
        }
        grid.append(b);
      }
      box.append(grid);
      if (answered && q.why) box.append(mk(doc, 'p', 'trial-why', q.why));
      const row = mk(doc, 'div', 'bench-actions trial-nav');
      if (answered && t.right === false) row.append(button(doc, 'trial-again', 'Ask me again', () => push('again')));
      row.append(button(doc, 'bench-back trial-back', 'All spells', back));
      box.append(row);
      wrap.append(box);
      if (mem.spoken !== key) {
        mem.spoken = key;
        if (!answered) speak(said);
        else if (q.why) speak(`Not quite. It is this one. ${q.why}`);
      }
    } else {
      // Done.
      const done = mk(doc, 'div', 'trial-done');
      done.setAttribute('data-kind', t.kind);
      done.setAttribute('data-mastered', String(!!t.mastered));
      let line = '';
      if (sharp) {
        const tick = mk(doc, 'span', 'bench-mark tick big');
        tick.setAttribute('aria-hidden', 'true');
        line = 'Still sharp! +10 XP.';
        done.append(tick, mk(doc, 'p', 'trial-sharp-line', line));
      } else {
        const art = mk(doc, 'div', 'trial-art');
        const big = mk(doc, 'span', `icon ${ICON_CLASS[card.id] || 'water'} trial-done-icon`);
        big.setAttribute('aria-hidden', 'true');
        const star = mk(doc, 'span', 'trial-star', '★');
        star.setAttribute('aria-hidden', 'true');
        art.append(big, star);
        done.append(art);
        if (q && q.why) {
          const why = mk(doc, 'p', 'trial-why');
          const tick = mk(doc, 'span', 'bench-mark tick');
          tick.setAttribute('aria-hidden', 'true');
          why.append(tick, mk(doc, 'span', '', q.why));
          done.append(why);
        }
        line = t.mastered ? `${name} mastered! One click casts it now.` : '';
        if (line) done.append(mk(doc, 'p', 'trial-mastered', line));
      }
      const row = mk(doc, 'div', 'bench-actions trial-nav');
      row.append(
        button(doc, 'bench-leave trial-leave', 'Back to the game', () => { try { ctx.close?.(); } catch { /* bench gone */ } }),
        button(doc, 'bench-back trial-back', 'All spells', back),
      );
      done.append(row);
      wrap.append(done);
      if (mem.rose !== key) { mem.rose = key; rise(); }
      if (mem.spoken !== key && line) { mem.spoken = key; speak(line); }
    }
    const open = [...body.querySelectorAll('.trial-choice')].find((c) => !(/** @type {HTMLButtonElement} */ (c).disabled));
    const first = /** @type {HTMLElement | null} */ (body.querySelector('#bench-input') || open || body.querySelector('.trial-again') || body.querySelector('[data-token]') || body.querySelector('button'));
    try { first?.focus?.(); } catch { /* not focusable */ }
  }

  // A wrong build in the build phase: the band's kind reason (nothing lost).
  if (t.phase === 'build' && mem.feedback && result && typeof result.n === 'number' && result.n > (mem.shownN || 0) && result.slot === t.slot) {
    mem.shownN = result.n;
    mem.sentAt = 0;
    if (!result.ok) showMiss(doc, mem, band, result, card, recipe, sharp, name);
  }
}

/**
 * @param {Document} doc
 * @param {Mem} mem
 * @param {string} band
 * @param {Result} r
 * @param {SpellCard} card
 * @param {any[] | null} recipe
 * @param {boolean} sharp
 * @param {string} name
 */
function showMiss(doc, mem, band, r, card, recipe, sharp, name) {
  const box = mk(doc, 'div', 'bench-fizzle trial-miss');
  const mark = mk(doc, 'span', 'bench-mark cross');
  mark.setAttribute('aria-hidden', 'true');
  /** @type {Record<number, { name: string, formula: string }>} */
  const names = { [card.slot]: { name, formula: card.formula } };
  let line = '';
  try { line = reasonFor(band, /** @type {any} */ (r), names, 1); } catch { line = ''; }
  box.append(mark, mk(doc, 'p', 'bench-reason', line));
  if (band === 'sprout' && r.code === 'atoms_off' && recipe) {
    // The picture diff: per element, the balls built against the balls wanted.
    /** @type {Record<string, [number, number]>} */
    const rows = {};
    for (const term of recipe) for (const [el, n] of term.atoms || []) rows[el] = [n, n];
    for (const [el, have, want] of r.diffs || []) rows[el] = [have, want];
    const list = mk(doc, 'div', 'bench-diffs');
    for (const el of Object.keys(rows)) {
      const [have, want] = rows[el] || [0, 0];
      const row = mk(doc, 'div', 'bench-diff');
      row.setAttribute('data-el', el);
      row.setAttribute('data-ok', String(have === want));
      const balls = (/** @type {number} */ n, /** @type {string} */ cls) => {
        const s = mk(doc, 'span', cls);
        for (let k = 0; k < n; k++) {
          const b = mk(doc, 'span', `bench-ball el-${el}`, el);
          b.setAttribute('data-el', el);
          s.append(b);
        }
        return s;
      };
      const m = mk(doc, 'span', `bench-mark ${have === want ? 'tick' : 'cross'}`);
      m.setAttribute('aria-hidden', 'true');
      row.append(balls(have, 'bench-diff-have'), balls(want, 'bench-diff-want'), m);
      list.append(row);
    }
    box.append(list);
  }
  let said = line;
  if (sharp) {
    const stays = `Not this time. ${name} stays mastered.`;
    box.append(mk(doc, 'p', 'trial-stays', stays));
    box.append(button(doc, 'trial-retry', 'Try again', () => {
      const again = /** @type {any} */ (mem).remount;
      if (typeof again === 'function') again();
      if (mem.feedback) mem.feedback.replaceChildren();
    }));
    said = `${line} ${stays}`;
  }
  mem.feedback?.replaceChildren(box);
  speak(said);
}
