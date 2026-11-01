// Worked casts (I-029): the field overlay. Before a spell is mastered a click
// opens a small builder (I-028's mountBuilder) below the crosshair and the
// world holds; a correct build casts. Rust owns every decision: this module
// only draws `window.__workState` and sends `work …` commands, and Rust judges
// every build with the bench's judge. See docs/loop/items/I-029/spec.md,
// section 6.
//
// Loaded by boot.js with a guarded dynamic import. Until `work ready` is sent
// (startWorked, below) every route is instant, so a missing or broken
// worked.js (or bench.js) leaves today's game: nothing can hold the world
// with no panel to close.
//
// While the panel is open:
// - under pointer lock, locked mouse moves drive a virtual cursor (hud.js
//   forwards them) and a left press clicks what is under it; a right press
//   cancels. Moves of 200 px or more are ignored (hud.js clamps a locked move
//   to 200 px, which is how a bogus post-click jump arrives).
// - every keydown but Escape and B is stopped in the window capture phase, so
//   winit and hud.js never see it; only Tab is prevented, so typing still
//   reaches the formula box. keyup always passes.
// - the bench wins: if it opens, the panel cancels itself.
// The overlay never requests or releases the pointer lock.

import { mountBuilder, reasonFor, pretty } from './bench.js';

/** @typedef {{ mountBuilder: typeof mountBuilder, reasonFor: typeof reasonFor, pretty: typeof pretty }} BenchApi */
/** @typedef {{ slot: number, id: string, name: string, formula: string, stage: string, prompt: string }} Card */
/** @typedef {{ n?: number, slot: number, ok?: boolean, code: string }} Result */
/** @typedef {{ band?: string, spells?: Card[], recipe?: any[], result?: Result }} View */
/** @typedef {{ id: number, spell: number, route: string, wrong: number, help: boolean, view: View | null }} Work */
/** @typedef {{ band?: string | null, stages?: string[], work?: Work | null }} WorkState */

const SHAPE = /** @type {Record<string, string>} */ ({ unknown: '○', learning: '◔', practised: '◑', mastered: '★' });
const WORD = /** @type {Record<string, string>} */ ({ unknown: 'New', learning: 'Learning', practised: 'Practised', mastered: 'Mastered' });
const ICON = /** @type {Record<string, string>} */ ({ fire: 'fire', water: 'water', frost: 'frost', co2: 'foam' });
// Hotbar names by spell index, for the toast when no view named the spell yet.
const NAMES = ['Fire', 'Water', 'Frost', 'CO₂'];
// hud.js clamps a locked move to this; a move this big is a jump, not a hand.
const JUMP = 200;
const TOAST_MS = 4000;

/**
 * @param {{ doc: Document, win: Window, cmd: (c: string) => void, bench: BenchApi }} opts
 */
export function createWorked({ doc, win, cmd, bench }) {
  const w = /** @type {any} */ (win);
  /** @type {HTMLElement | null} */ let panel = null;
  /** @type {HTMLElement | null} */ let copyLine = null;
  /** @type {HTMLElement | null} */ let reasonBox = null;
  /** @type {HTMLElement | null} */ let vcursor = null;
  /** @type {{ destroy: () => void } | null} */ let builder = null;
  let openId = 0;
  let band = '';
  let lastN = 0;
  let cancelSent = 0;
  /** @type {View | null} */ let view = null;
  /** @type {Record<number, string>} */ const names = {};
  /** @type {string[] | null} */ let prevStages = null;
  let vx = 0;
  let vy = 0;
  /** @type {any} */ let toastTimer = 0;

  const el = (/** @type {string} */ tag, /** @type {string} */ cls, /** @type {string} */ text = '') => {
    const e = doc.createElement(tag);
    if (cls) e.className = cls;
    if (text) e.textContent = text;
    return e;
  };
  const send = (/** @type {string} */ c) => { try { cmd(c); } catch { /* no game */ } };

  const showCopy = () => {
    if (!copyLine || !view) return;
    const card = (view.spells || [])[0];
    copyLine.textContent = `Copy: ${bench.pretty(card ? card.formula : '')}`;
    copyLine.hidden = false;
  };

  const placeCursor = () => {
    if (!vcursor) return;
    vcursor.style.left = `${vx}px`;
    vcursor.style.top = `${vy}px`;
  };

  const close = () => {
    if (builder) { try { builder.destroy(); } catch { /* already gone */ } }
    builder = null;
    if (panel) panel.remove();
    panel = null;
    copyLine = null;
    reasonBox = null;
    vcursor = null;
    view = null;
    openId = 0;
    lastN = 0;
    try { /** @type {HTMLElement | null} */ (doc.getElementById('game'))?.focus(); } catch { /* no canvas */ }
  };

  const focusFirst = () => {
    if (!panel) return;
    const input = /** @type {HTMLElement | null} */ (panel.querySelector('#bench-input'));
    const first = input || /** @type {HTMLElement | null} */ (panel.querySelector('[data-token]')) || /** @type {HTMLElement | null} */ (panel.querySelector('button'));
    try { first?.focus(); } catch { /* not focusable */ }
  };

  /** @param {Work} work */
  const open = (work) => {
    close();
    const v = /** @type {View} */ (work.view);
    view = v;
    openId = work.id;
    band = String(v.band || 'apprentice');
    const card = (v.spells || [])[0] || { slot: work.spell + 1, id: '', name: '', formula: '', stage: 'unknown', prompt: '' };
    names[work.spell] = card.name;
    const stage = card.stage || 'unknown';
    const p = el('div', '');
    p.id = 'worked';
    p.setAttribute('role', 'dialog');
    p.setAttribute('aria-label', card.name);
    p.setAttribute('data-band', band);
    const head = el('div', 'worked-head');
    const icon = el('span', `icon ${ICON[card.id] || 'water'} worked-icon`);
    icon.setAttribute('aria-hidden', 'true');
    const x = el('button', 'worked-close', '✕');
    x.setAttribute('type', 'button');
    x.setAttribute('aria-label', 'Cancel');
    x.addEventListener('click', () => api.cancel());
    head.append(icon, el('span', 'worked-name', bench.pretty(card.name)), el('span', 'worked-stage', WORD[stage] || ''), x);
    p.append(head);
    // Sprout reads pictures only (GS-21): no sentence lines there.
    if (stage === 'unknown' && band !== 'sprout') {
      p.append(el('p', 'worked-new', 'New spell: copy it. Learn it at the Formula Bench (B) to earn XP.'));
    }
    copyLine = el('p', 'worked-copy');
    copyLine.hidden = true;
    p.append(copyLine);
    const mount = el('div', 'worked-mount');
    p.append(mount);
    reasonBox = el('div', 'worked-reason');
    p.append(reasonBox);
    vcursor = el('span', 'vcursor');
    vcursor.setAttribute('aria-hidden', 'true');
    vcursor.hidden = !doc.pointerLockElement;
    p.append(vcursor);
    doc.body.append(p);
    panel = p;
    const id = work.id;
    try {
      builder = bench.mountBuilder(mount, {
        band,
        spell: card,
        recipe: Array.isArray(v.recipe) ? v.recipe : null,
        prompt: card.prompt,
        onMake: (/** @type {string} */ text) => { if (openId === id) send(`work submit ${id} ${text}`); },
      });
    } catch { builder = null; }
    if ((stage === 'unknown' || work.help) && band !== 'sprout') showCopy();
    const r = p.getBoundingClientRect();
    vx = r.left + r.width / 2;
    vy = r.top + r.height / 2;
    placeCursor();
    focusFirst();
  };

  /** @param {Result | undefined} result */
  const showResult = (result) => {
    if (!result || !reasonBox || !view || result.ok) return;
    const n = Number(result.n) || 0;
    if (n <= lastN) return;
    lastN = n;
    /** @type {Record<number, { name: string, formula: string }>} */
    const byslot = {};
    for (const c of view.spells || []) byslot[c.slot] = { name: c.name, formula: c.formula };
    let text = '';
    try { text = bench.reasonFor(band, /** @type {any} */ (result), byslot, n); } catch { text = ''; }
    const mark = el('span', 'bench-mark cross');
    mark.setAttribute('aria-hidden', 'true');
    reasonBox.replaceChildren(mark, el('span', 'worked-reason-text', text));
    try { w.__game && typeof w.__game.sound === 'function' && w.__game.sound('miss'); } catch { /* no sound */ }
  };

  const badges = (/** @type {string[]} */ stages) => {
    const slots = /** @type {HTMLElement[]} */ ([...doc.querySelectorAll('.slot')]);
    slots.forEach((slot, i) => {
      const st = stages[i] || 'unknown';
      let b = /** @type {HTMLElement | null} */ (slot.querySelector('span.stage'));
      if (!b) {
        b = el('span', 'stage');
        slot.append(b);
      }
      if (b.getAttribute('data-stage') === st) return;
      b.setAttribute('data-stage', st);
      b.textContent = SHAPE[st] || SHAPE.unknown || '';
      b.setAttribute('aria-label', WORD[st] || '');
      b.setAttribute('title', WORD[st] || '');
    });
  };

  const toast = (/** @type {number} */ i) => {
    let t = doc.getElementById('worked-toast');
    if (!t) {
      t = el('div', '');
      t.id = 'worked-toast';
      t.setAttribute('role', 'status');
      doc.body.append(t);
    }
    const shape = el('span', 'worked-toast-shape', SHAPE.practised);
    shape.setAttribute('aria-hidden', 'true');
    t.replaceChildren(shape, el('span', '', `${bench.pretty(names[i] || NAMES[i] || '')}: Practised!`));
    t.hidden = false;
    const tt = t;
    try {
      win.clearTimeout(toastTimer);
      toastTimer = win.setTimeout(() => { tt.hidden = true; }, TOAST_MS);
    } catch { /* stays */ }
  };

  const api = {
    /** @param {WorkState | null | undefined} ws */
    update(ws) {
      if (!ws || typeof ws !== 'object') return;
      const stages = Array.isArray(ws.stages) ? ws.stages : null;
      if (stages) {
        badges(stages);
        if (prevStages) stages.forEach((s, i) => { if (s === 'practised' && prevStages && prevStages[i] === 'learning') toast(i); });
        prevStages = stages.slice();
      }
      const work = ws.work || null;
      if (!work) { if (panel) close(); return; }
      if (work.id !== openId) {
        if (!work.view) return;
        open(work);
      } else if (work.view) {
        view = work.view;
      }
      if (work.help && band !== 'sprout' && copyLine && copyLine.hidden) showCopy();
      if (view) showResult(view.result);
      // The bench wins: it opened (B or its button), so this worked cast ends.
      let benchOpen = false;
      try { benchOpen = !!(w.__bench && w.__bench.ui().open); } catch { benchOpen = false; }
      if (benchOpen && cancelSent !== openId) { cancelSent = openId; send(`work cancel ${openId}`); }
    },
    isOpen() { return !!panel; },
    /** @param {number} dx @param {number} dy */
    move(dx, dy) {
      if (!panel || !vcursor) return false;
      if (Math.abs(dx) >= JUMP || Math.abs(dy) >= JUMP) return true;
      const r = panel.getBoundingClientRect();
      vx = Math.min(r.left + r.width, Math.max(r.left, vx + (Number(dx) || 0)));
      vy = Math.min(r.top + r.height, Math.max(r.top, vy + (Number(dy) || 0)));
      vcursor.hidden = false;
      placeCursor();
      return true;
    },
    /** @param {number} button */
    press(button) {
      if (!panel) return false;
      if (button === 2) { api.cancel(); return true; }
      if (button !== 0) return true;
      const hit = /** @type {HTMLElement | null} */ (doc.elementFromPoint(vx, vy));
      const target = hit && typeof hit.closest === 'function' ? /** @type {HTMLElement | null} */ (hit.closest('button')) : null;
      if (target && panel && (target === panel || target.closest('#worked'))) {
        try { target.focus(); } catch { /* not focusable */ }
        target.click();
      }
      return true;
    },
    /** @param {KeyboardEvent} e */
    key(e) {
      if (!panel || !e || e.type !== 'keydown') return false;
      if (e.key === 'Escape' || e.code === 'Escape' || e.code === 'KeyB') return false;
      e.stopImmediatePropagation();
      const t = /** @type {any} */ (e.target);
      if (e.key === 'Tab') {
        e.preventDefault();
        moveFocus(e.shiftKey ? -1 : 1);
        return true;
      }
      const inInput = !!t && t.tagName === 'INPUT';
      if (inInput && e.key === 'Enter') {
        if (typeof t._benchMake === 'function') t._benchMake();
        return true;
      }
      if (String(e.key || '').startsWith('Arrow') && !inInput) {
        if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && t && typeof t._benchStep === 'function') {
          t._benchStep(e.key === 'ArrowUp' ? 1 : -1);
        } else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
          moveFocus(e.key === 'ArrowRight' ? 1 : -1);
        }
      }
      return true;
    },
    cancel() { if (panel && openId) send(`work cancel ${openId}`); },
  };

  const moveFocus = (/** @type {number} */ d) => {
    if (!panel) return;
    const list = /** @type {HTMLElement[]} */ ([...panel.querySelectorAll('button, input')]);
    if (!list.length) return;
    const i = list.indexOf(/** @type {HTMLElement} */ (doc.activeElement));
    const next = list[i < 0 ? 0 : (i + d + list.length) % list.length];
    try { next?.focus(); } catch { /* not focusable */ }
  };

  return api;
}

export function startWorked() {
  const w = /** @type {any} */ (window);
  const game = w.__game;
  if (!game || typeof game.overlay !== 'function' || typeof game.cmd !== 'function') return;
  const api = createWorked({ doc: document, win: window, cmd: game.cmd, bench: { mountBuilder, reasonFor, pretty } });
  game.overlay(api);
  window.addEventListener('keydown', (e) => { api.key(e); }, true);
  let raw = '';
  /** @type {WorkState | null} */ let ws = null;
  const tick = () => {
    const now = w.__workState;
    if (typeof now === 'string' && now !== raw) {
      raw = now;
      try { ws = JSON.parse(now); } catch { ws = null; }
      api.update(ws);
    } else if (api.isOpen()) {
      api.update(ws);
    }
    window.requestAnimationFrame(tick);
  };
  window.requestAnimationFrame(tick);
  game.cmd('work ready');
}
