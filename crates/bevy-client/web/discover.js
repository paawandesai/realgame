// Discovered spells in the block world (I-053, docs/loop/items/I-053/spec.md).
//
// Rust (discover.rs) owns the rules. This module loads the shared save
// (web/formula-save.js) into the game through `window.__gameIn.disc`,
// writes the game's `save` back when its `rev` grows, reloads when the
// Chemistry Bench page saves a spell in another tab, and shows a
// discovered spell's name on its hotbar slot. Only in the block world.

import { addFound, loadCommands, readFound, readSpells, writeSpells } from './formula-save.js';

const POLL_MS = 400;

/** @param {string | undefined} raw */
const parse = (raw) => {
  try { return raw ? JSON.parse(raw) : null; } catch { return null; }
};

/**
 * Hotbar slot labels: a discovered spell's name over the shipped spell's,
 * and the shipped text back when it is switched off. Sprout sees no formula.
 * @param {Document} doc
 * @param {any} st `__discState`
 */
export function relabel(doc, st) {
  const slots = [...doc.querySelectorAll('.slot[data-spell]')];
  slots.forEach((el, i) => {
    if (i > 3) return;
    const name = /** @type {HTMLElement | null} */ (el.querySelector('.name'));
    const sub = /** @type {HTMLElement | null} */ (el.querySelector('.formula'));
    const e = /** @type {HTMLElement} */ (el);
    if (name && e.dataset.baseName === undefined) e.dataset.baseName = name.textContent || '';
    if (sub && e.dataset.baseSub === undefined) e.dataset.baseSub = sub.textContent || '';
    const disc = st && Array.isArray(st.on) && st.on[i] && st.names && st.names[i] ? String(st.names[i]) : '';
    e.classList.toggle('disc-slot', !!disc);
    if (disc) e.dataset.kind = (st.kinds && st.kinds[i]) || '';
    else delete e.dataset.kind;
    if (name) name.textContent = disc || e.dataset.baseName || '';
    if (sub) sub.textContent = disc ? `press ${i + 1} to switch` : e.dataset.baseSub || '';
  });
}

/** @param {any} [win] */
export function startDiscover(win = globalThis.window) {
  try {
    if (!win) return;
    /** @type {any} */
    let storage = null;
    try { storage = win.localStorage; } catch { storage = null; }
    const push = (/** @type {string} */ c) => {
      win.__gameIn = win.__gameIn || {};
      (win.__gameIn.disc = win.__gameIn.disc || []).push(c);
    };
    const load = () => {
      if (!storage) return;
      for (const c of loadCommands(readSpells(storage), readFound(storage))) push(c);
    };
    let started = false;
    let lastRev = 0;
    let lastLabels = '';
    let fireFound = false;
    const poll = () => {
      try {
        const gs = parse(win.__gameState);
        if (!gs || gs.mode !== 'voxel') return;
        if (!started) { started = true; load(); }
        const st = parse(win.__discState);
        if (!st) return;
        if (storage && typeof st.rev === 'number' && st.rev > lastRev && st.loaded) {
          lastRev = st.rev;
          writeSpells(storage, st.save);
        }
        const labels = JSON.stringify([st.on, st.names, st.kinds]);
        if (labels !== lastLabels) { lastLabels = labels; relabel(win.document, st); }
        // Finds flow both ways: Fire learned in the world is methane burning.
        if (storage && !fireFound) {
          const ws = parse(win.__workState);
          if (ws && Array.isArray(ws.stages) && ws.stages[0] && ws.stages[0] !== 'unknown') {
            fireFound = true;
            addFound(storage, ['methane_burns']);
          }
        }
      } catch { /* the game keeps running without saving */ }
    };
    win.setInterval?.(poll, POLL_MS);
    // The Bench page saved a spell in another tab: load it.
    win.addEventListener?.('storage', (/** @type {StorageEvent} */ e) => {
      if (started && (e.key === 'formula.spells.v1' || e.key === 'formula.bench.found')) load();
    });
  } catch {
    // Discoveries must never break the game.
  }
}
