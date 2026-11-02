// Worked casts (I-029, spec section 6): the field overlay over I-028's
// mountBuilder. Rust decides and judges; this draws `__workState` and sends
// `work …` commands; until `work ready` every route is instant. Under lock a
// virtual cursor clicks (200 px jumps ignored; past the top or bottom it
// scrolls a tall panel); keydowns but Escape and B stop (only Tab prevented).
// It never touches the pointer lock.

import { mountBuilder, reasonFor, pretty } from './bench.js';

/** @typedef {{ n?: number, slot: number, ok?: boolean, code: string }} Result */
/** @typedef {{ band?: string, spells?: any[], recipe?: any[], result?: Result }} View */
/** @typedef {{ id: number, spell: number, wrong: number, help: boolean, view: View | null }} Work */
/** @typedef {{ stages?: string[], work?: Work | null }} WorkState */

/** @type {Record<string, string>} */
const SHAPE = { unknown: '○', learning: '◔', practised: '◑', mastered: '★' };
/** @type {Record<string, string>} */
const WORD = { unknown: 'New', learning: 'Learning', practised: 'Practised', mastered: 'Mastered' };
// CO₂ uses the foam icon.
const icon = (/** @type {string} */ id) => `icon ${id === 'co2' ? 'foam' : id || 'water'} worked-icon`;
const IDS = ['fire', 'water', 'frost', 'co2'];
const NAMES = ['Fire', 'Water', 'Frost', 'CO₂'];
const JUMP = 200;

/**
 * @param {{ doc: Document, win: Window, cmd: (c: string) => void, bench: { mountBuilder: typeof mountBuilder, reasonFor: typeof reasonFor, pretty: typeof pretty } }} o
 */
export function createWorked({ doc, win, cmd, bench }) {
  const w = /** @type {any} */ (win);
  let [panel, copyLine, reasonBox, vcursor, builder, prev] = /** @type {any[]} */ ([]);
  /** @type {View | null} */ let view = null;
  /** @type {Record<number, string>} */ const names = {};
  let openId = 0, band = '', lastN = 0, cancelSent = 0, vx = 0, vy = 0, timer = 0;
  // The mousemove fallback moves the cursor until hud.js forwards a real move.
  let moveWorks = false;

  /** @returns {any} */
  const el = (/** @type {string} */ tag, cls = '', text = '', /** @type {Record<string, string>} */ at = {}) => {
    const e = doc.createElement(tag);
    if (cls) e.className = cls;
    if (text) e.textContent = text;
    for (const k in at) e.setAttribute(k, at[k] || '');
    return e;
  };
  const H = { 'aria-hidden': 'true' };
  const send = (/** @type {string} */ c) => { try { cmd(c); } catch { /* no game */ } };
  const sprout = () => band === 'sprout';
  const place = () => { if (vcursor) { vcursor.style.left = `${vx}px`; vcursor.style.top = `${vy}px`; } };

  const showCopy = () => {
    if (!copyLine || !view) return;
    const card = (view.spells || [])[0];
    copyLine.textContent = `Copy: ${bench.pretty(card ? card.formula : '')}`;
    copyLine.hidden = false;
  };

  const close = () => {
    try { builder?.destroy(); } catch { /* gone */ }
    panel?.remove();
    panel = copyLine = reasonBox = vcursor = builder = view = null;
    openId = lastN = 0;
    try { doc.getElementById('game')?.focus(); } catch { /* no canvas */ }
  };

  /** @param {Work} work */
  const open = (work) => {
    close();
    const v = /** @type {View} */ (work.view);
    view = v;
    const id = openId = work.id;
    band = String(v.band || 'apprentice');
    const card = (v.spells || [])[0] || { slot: work.spell + 1, id: '', name: '', formula: '', stage: 'unknown', prompt: '' };
    names[work.spell] = card.name;
    const stage = card.stage || 'unknown';
    const p = el('div', '', '', { id: 'worked', role: 'dialog', 'aria-label': card.name, 'data-band': band });
    const head = el('div', 'worked-head');
    const x = el('button', 'worked-close', '✕', { type: 'button', 'aria-label': 'Cancel' });
    x.addEventListener('click', () => api.cancel());
    head.append(el('span', icon(card.id), '', H), el('span', 'worked-name', bench.pretty(card.name)), el('span', 'worked-stage', WORD[stage] || ''));
    p.append(head);
    // Sprout sees pictures only (GS-21): a bench picture, not the line.
    const isNew = stage === 'unknown';
    if (isNew && sprout()) head.append(el('span', 'worked-bench', '', { role: 'img', 'aria-label': 'Formula Bench' }));
    head.append(x);
    if (isNew && !sprout()) p.append(el('p', 'worked-new', 'New spell: copy it. Learn it at the Formula Bench (B) to earn XP.'));
    copyLine = el('p', 'worked-copy');
    copyLine.hidden = true;
    const mount = el('div', 'worked-mount');
    reasonBox = el('div', 'worked-reason');
    vcursor = el('span', 'vcursor', '', H);
    vcursor.hidden = !doc.pointerLockElement;
    p.append(copyLine, mount, reasonBox, vcursor);
    doc.body.append(panel = p);
    // Practised (I-052 round 2): the spell is known well, so its worked
    // cast is one step: the formula is shown and one Cast press confirms it
    // (the game still judges the formula it receives).
    if (stage === 'practised' && card.formula) {
      const sure = el('button', 'worked-confirm', '', { type: 'button' });
      sure.append(el('span', icon(card.id), '', H), el('span', '', sprout() ? '✓' : `Cast ${bench.pretty(card.name)}`));
      if (!sprout()) mount.append(el('p', 'worked-known', `You know this one: ${bench.pretty(card.formula)}`));
      mount.append(sure);
      sure.addEventListener('click', () => { if (openId === id) send(`work submit ${id} ${card.formula}`); });
      const r0 = p.getBoundingClientRect();
      vx = r0.left + r0.width / 2;
      vy = r0.top + r0.height / 2;
      place();
      try { sure.focus(); } catch { /* none */ }
      return;
    }
    try {
      builder = bench.mountBuilder(mount, {
        band,
        spell: card,
        recipe: Array.isArray(v.recipe) ? v.recipe : null,
        prompt: card.prompt,
        onMake: (/** @type {string} */ t) => { if (openId === id) send(`work submit ${id} ${t}`); },
      });
    } catch { builder = null; }
    if ((isNew || work.help) && !sprout()) showCopy();
    const r = p.getBoundingClientRect();
    vx = r.left + r.width / 2;
    vy = r.top + r.height / 2;
    place();
    try { (p.querySelector('#bench-input') || p.querySelector('[data-token]') || x)?.focus(); } catch { /* none */ }
  };

  /** @param {Result | undefined} res */
  const showResult = (res) => {
    if (!res || !reasonBox || !view || res.ok) return;
    const n = Number(res.n) || 0;
    if (n <= lastN) return;
    lastN = n;
    /** @type {Record<number, { name: string, formula: string }>} */
    const byslot = {};
    for (const c of view.spells || []) byslot[c.slot] = { name: c.name, formula: c.formula };
    let text = '';
    try { text = bench.reasonFor(band, /** @type {any} */ (res), byslot, n); } catch { /* none */ }
    reasonBox.replaceChildren(el('span', 'bench-mark cross', '', H), el('span', 'worked-reason-text', text));
    try { w.__game.sound('miss'); } catch { /* no sound */ }
  };

  const badges = (/** @type {string[]} */ stages) => {
    doc.querySelectorAll('.slot').forEach((slot, i) => {
      const st = stages[i] || 'unknown';
      let b = /** @type {any} */ (slot.querySelector('span.stage'));
      if (!b) slot.append(b = el('span', 'stage'));
      if (b.getAttribute('data-stage') === st) return;
      b.setAttribute('data-stage', st);
      b.textContent = SHAPE[st] || SHAPE.unknown || '';
      b.setAttribute('aria-label', WORD[st] || '');
      b.setAttribute('title', WORD[st] || '');
    });
  };

  // "Water: Practised!" for 4 s (Sprout: pictures, the words a label).
  const toast = (/** @type {number} */ i) => {
    /** @type {any} */
    let t = doc.getElementById('worked-toast');
    if (!t) doc.body.append(t = el('div', '', '', { id: 'worked-toast', role: 'status' }));
    const words = `${bench.pretty(names[i] || NAMES[i] || '')}: Practised!`;
    const shape = el('span', 'worked-toast-shape', SHAPE.practised, H);
    if (sprout()) {
      t.replaceChildren(el('span', icon(IDS[i] || ''), '', H), shape);
      t.setAttribute('aria-label', words);
    } else {
      t.replaceChildren(shape, el('span', '', words));
      t.removeAttribute('aria-label');
    }
    t.hidden = false;
    try {
      win.clearTimeout(timer);
      timer = win.setTimeout(() => { t.hidden = true; }, 4000);
    } catch { /* stays */ }
  };

  const moveFocus = (/** @type {number} */ d) => {
    const list = panel ? [...panel.querySelectorAll('button, input')] : [];
    if (!list.length) return;
    const i = list.indexOf(doc.activeElement);
    try { list[i < 0 ? 0 : (i + d + list.length) % list.length]?.focus(); } catch { /* not focusable */ }
  };

  const api = {
    /** @param {WorkState | null | undefined} ws */
    update(ws) {
      if (!ws || typeof ws !== 'object') return;
      const stages = Array.isArray(ws.stages) ? ws.stages : null;
      if (stages) {
        badges(stages);
        stages.forEach((s, i) => { if (s === 'practised' && prev && prev[i] === 'learning') toast(i); });
        prev = stages.slice();
      }
      const work = ws.work || null;
      if (!work) { if (panel) close(); return; }
      if (work.id !== openId) {
        if (!work.view) return;
        open(work);
      } else if (work.view) view = work.view;
      if (work.help && !sprout() && copyLine?.hidden) showCopy();
      if (view) showResult(view.result);
      let benchOpen = false;
      try { benchOpen = !!w.__bench.ui().open; } catch { /* no bench */ }
      if (benchOpen && cancelSent !== openId) { cancelSent = openId; send(`work cancel ${openId}`); }
    },
    isOpen() { return !!panel; },
    /** @param {number} dx @param {number} dy @param {boolean} [fallback] */
    move(dx, dy, fallback = false) {
      if (!panel || !vcursor) return false;
      if (fallback && moveWorks) return true;
      if (!fallback && (dx || dy)) moveWorks = true;
      if (Math.abs(dx) >= JUMP || Math.abs(dy) >= JUMP) return true;
      const r = panel.getBoundingClientRect();
      const ny = vy + (Number(dy) || 0);
      // Past the top or bottom: scroll a tall panel, so every piece is reachable.
      const over = ny > r.bottom ? ny - r.bottom : ny < r.top ? ny - r.top : 0;
      if (over) panel.scrollTop = (Number(panel.scrollTop) || 0) + over;
      vx = Math.min(r.right, Math.max(r.left, vx + (Number(dx) || 0)));
      vy = Math.min(r.bottom, Math.max(r.top, ny));
      vcursor.hidden = false;
      place();
      return true;
    },
    /** @param {number} button */
    press(button) {
      if (!panel) return false;
      if (button === 2) api.cancel();
      if (button !== 0) return true;
      const hit = /** @type {any} */ (doc.elementFromPoint(vx, vy));
      const b = hit?.closest ? hit.closest('button') : null;
      if (b && b.closest('#worked')) {
        try { b.focus(); } catch { /* not focusable */ }
        b.click();
      }
      return true;
    },
    /** @param {KeyboardEvent} e */
    key(e) {
      if (!panel || !e || e.type !== 'keydown') return false;
      if (e.key === 'Escape' || e.code === 'Escape' || e.code === 'KeyB') return false;
      e.stopImmediatePropagation();
      const t = /** @type {any} */ (e.target);
      const k = String(e.key || '');
      const inInput = t?.tagName === 'INPUT';
      if (k === 'Tab') {
        e.preventDefault();
        moveFocus(e.shiftKey ? -1 : 1);
      } else if (inInput && k === 'Enter') {
        t._benchMake?.();
      } else if (!inInput && (k === 'ArrowUp' || k === 'ArrowDown')) {
        t?._benchStep?.(k === 'ArrowUp' ? 1 : -1);
      } else if (!inInput && (k === 'ArrowLeft' || k === 'ArrowRight')) {
        moveFocus(k === 'ArrowRight' ? 1 : -1);
      }
      return true;
    },
    cancel() { if (panel && openId) send(`work cancel ${openId}`); },
  };
  return api;
}

export function startWorked() {
  const w = /** @type {any} */ (window);
  const game = w.__game;
  if (typeof game?.overlay !== 'function' || typeof game.cmd !== 'function') return;
  const api = createWorked({ doc: document, win: window, cmd: game.cmd, bench: { mountBuilder, reasonFor, pretty } });
  game.overlay(api);
  window.addEventListener('keydown', (e) => { api.key(e); }, true);
  // Where locked pointermove has no movement (headless Chromium): mousemove.
  const clamp = (/** @type {number} */ v) => (Number.isFinite(v) ? Math.max(-JUMP, Math.min(JUMP, v)) : 0);
  document.addEventListener('mousemove', (e) => {
    if (api.isOpen() && document.pointerLockElement) api.move(clamp(e.movementX), clamp(e.movementY), true);
  });
  let raw = '';
  /** @type {WorkState | null} */ let ws = null;
  const tick = () => {
    const now = w.__workState;
    if (typeof now === 'string' && now !== raw) {
      raw = now;
      try { ws = JSON.parse(now); } catch { ws = null; }
      api.update(ws);
    } else if (api.isOpen()) api.update(ws);
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  game.cmd('work ready');
}
