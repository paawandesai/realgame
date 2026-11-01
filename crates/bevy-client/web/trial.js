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
export const PICTURES = {} || {
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
// CONTRACT STUB (I-030): the exports exist; nothing is drawn yet.
/** @param {string} band @param {number} built @param {string} kind @param {string} spellName @param {boolean} [reaction] */
export function lineFor(band, built, kind, spellName, reaction) { void band; void built; void kind; void spellName; void reaction; return ''; }
/** @param {number} built @returns {boolean[]} */
export function pipStates(built) { void built; return []; }
/** @param {HTMLElement} body @param {TrialState} trialState @param {BenchView} benchView @param {Ctx} ctx */
export function renderTrial(body, trialState, benchView, ctx) { void body; void trialState; void benchView; void ctx; void mountBuilder; void reasonFor; void pretty; void speak; void rise; void IDS; void ICON_CLASS; }
