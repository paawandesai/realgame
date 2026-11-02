// The valley's later puzzles and the ending (I-052, late lane; spec sections
// 4-8). Rust (late.rs) owns the rules; this draws `window.__lateState` and
// sends commands on `window.__gameIn.late`:
//   use               E, next to the rig, the crate, the Kiln door or the
//                     Invention Bench
//   tie <gas item>    the balloon chooser at the crate (h2_balloon, o2_flask)
//   invent <items>|<text>   the Invention Bench
//   keep_exploring    closes the ending card
// The chooser, the Invention Bench and the ending card follow the Formula
// Bench's modal rules (aria-modal, the page behind inert, Escape closes, the
// canvas gets the focus back). Only in the block world.

const parse = (/** @type {string | undefined} */ raw) => {
  try { return raw ? JSON.parse(raw) : null; } catch { return null; }
};

/** @param {string} cmd */
const push = (cmd) => {
  const w = /** @type {any} */ (window);
  w.__gameIn = w.__gameIn || {};
  (w.__gameIn.late = w.__gameIn.late || []).push(cmd);
};

/** What E does next to each thing. */
export const PROMPTS = {
  rig: 'E: use the electrolysis rig',
  crate: 'E: the balloon crate',
  door: 'E: open the Kiln door',
  invention: 'E: use the Invention Bench',
};

/** The five puzzles the ending lists. */
export const PUZZLES = [
  'Put out the workshop fire',
  'Freeze the river ford',
  'Smother the marsh gas',
  'Lift the Kiln Spark with hydrogen',
  'Invent a reaction to relight the Kiln',
];

/** m:ss for the time played. */
export const clock = (/** @type {number} */ s) => {
  const t = Math.max(0, Math.round(s));
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, '0')}`;
};

/** Pretty chemistry: digits after a letter become subscripts. */
export const chem = (/** @type {string} */ s) => s.replace(/([A-Za-z)])(\d+)/g, (_, a, d) => a + [...d].map((c) => '₀₁₂₃₄₅₆₇₈₉'[+c]).join(''));

/** @param {string} tag @param {string} cls @param {string} [text] */
const mk = (tag, cls, text) => {
  const el = document.createElement(tag);
  if (cls) el.className = cls;
  if (text !== undefined) el.textContent = text;
  return el;
};

export function startLate() {
  const w = /** @type {any} */ (window);
  let sfx = /** @type {any} */ (null);
  const play = (/** @type {string} */ n) => {
    if (sfx) { try { sfx.play(n); } catch { /* no sound */ } }
  };

  // The prompt chip and the message line.
  const chip = mk('div', 'late-chip');
  chip.id = 'late-chip';
  chip.hidden = true;
  const say = mk('div', 'late-say');
  say.id = 'late-say';
  say.setAttribute('role', 'status');
  say.setAttribute('aria-live', 'polite');
  say.hidden = true;
  document.body.append(chip, say);

  /** A modal dialog with a card. */
  const modal = (/** @type {string} */ id, /** @type {string} */ label) => {
    const root = mk('div', 'late-modal');
    root.id = id;
    root.hidden = true;
    root.setAttribute('role', 'dialog');
    root.setAttribute('aria-modal', 'true');
    root.setAttribute('aria-label', label);
    const card = mk('div', 'late-card');
    root.append(card);
    document.body.append(root);
    return { root, card };
  };

  /** @type {Array<[HTMLElement, boolean]>} */
  let behind = [];
  let wasPaused = false;
  /** @type {HTMLElement | null} */
  let shown = null;
  const otherModal = () => [...document.querySelectorAll('[aria-modal="true"]')]
    .some((el) => el !== shown && !(/** @type {HTMLElement} */ (el).hidden) && /** @type {HTMLElement} */ (el).offsetParent !== null);
  const show = (/** @type {HTMLElement} */ root, /** @type {HTMLElement | null} */ focus) => {
    if (shown === root) return true;
    if (shown || otherModal()) return false;
    shown = root;
    wasPaused = !!(w.__gameIn && w.__gameIn.paused);
    try { if (document.pointerLockElement) document.exitPointerLock(); } catch { /* nothing to free */ }
    try { w.__game && w.__game.pause(true); } catch { /* no HUD */ }
    behind = /** @type {Array<[HTMLElement, boolean]>} */ (['stage', 'hud', 'loading', 'pause']
      .map((id) => document.getElementById(id))
      .filter(Boolean)
      .map((el) => [el, /** @type {HTMLElement} */ (el).inert]));
    for (const [el] of behind) el.inert = true;
    document.getElementById('pause')?.classList.add('bench-under');
    root.hidden = false;
    focus?.focus();
    return true;
  };
  const hide = () => {
    if (!shown) return;
    shown.hidden = true;
    shown = null;
    for (const [el, was] of behind) el.inert = was;
    behind = [];
    document.getElementById('pause')?.classList.remove('bench-under');
    try { w.__game && w.__game.pause(wasPaused); } catch { /* no HUD */ }
    document.getElementById('game')?.focus();
  };

  // The balloon chooser.
  const gas = modal('late-gas', 'Fill the balloon');
  gas.card.append(mk('h2', '', 'Fill the balloon'), mk('p', 'late-help', 'Which gas will lift the crate up to the Kiln Spark? A gas lifts only if it is lighter than air.'));
  const gasRow = mk('div', 'late-row');
  const gasClose = /** @type {HTMLButtonElement} */ (mk('button', 'late-close', 'Not now'));
  gasClose.type = 'button';
  gas.card.append(gasRow, gasClose);
  gasClose.addEventListener('click', hide);
  const GAS_LABEL = { h2_balloon: 'Hydrogen, H₂', o2_flask: 'Oxygen, O₂' };
  const openGas = (/** @type {any} */ st) => {
    gasRow.replaceChildren();
    /** @type {HTMLButtonElement | null} */
    let first = null;
    for (const g of st.gases || []) {
      const b = /** @type {HTMLButtonElement} */ (mk('button', `late-pick late-gas-${g}`, /** @type {any} */ (GAS_LABEL)[g] || g));
      b.type = 'button';
      b.dataset.item = g;
      b.addEventListener('click', () => { push(`tie ${g}`); play('pickup'); hide(); });
      gasRow.append(b);
      first = first || b;
    }
    show(gas.root, first);
  };

  // The Invention Bench.
  const inv = modal('late-invent', 'Invention Bench');
  inv.card.append(mk('h2', '', 'Invention Bench'), mk('p', 'late-help', 'Put up to three things from your pack on the bench. The Kiln needs a reaction that burns hotter than 1700 °C.'));
  const invRow = mk('div', 'late-row late-items');
  const typedWrap = mk('label', 'late-typed');
  typedWrap.append(mk('span', '', 'Or type the reaction'));
  const typed = /** @type {HTMLInputElement} */ (mk('input', 'late-text'));
  typed.type = 'text';
  typed.placeholder = 'C + O2 -> CO2';
  typed.autocomplete = 'off';
  typed.spellcheck = false;
  typedWrap.append(typed);
  const invTry = /** @type {HTMLButtonElement} */ (mk('button', 'late-try', 'Try it'));
  invTry.type = 'button';
  const invOut = mk('p', 'late-result');
  invOut.setAttribute('role', 'status');
  invOut.setAttribute('aria-live', 'polite');
  const invClose = /** @type {HTMLButtonElement} */ (mk('button', 'late-close', 'Close'));
  invClose.type = 'button';
  inv.card.append(invRow, typedWrap, invTry, invOut, invClose);
  invClose.addEventListener('click', hide);
  /** @type {Set<string>} */
  const chosen = new Set();
  const drawBench = (/** @type {any} */ st) => {
    invRow.replaceChildren();
    for (const it of st.bench || []) {
      const b = /** @type {HTMLButtonElement} */ (mk('button', 'late-pick late-item', chem(it.label)));
      b.type = 'button';
      b.dataset.item = it.item;
      b.setAttribute('aria-pressed', String(chosen.has(it.item)));
      b.addEventListener('click', () => {
        if (chosen.has(it.item)) chosen.delete(it.item);
        else if (chosen.size < 3) chosen.add(it.item);
        b.setAttribute('aria-pressed', String(chosen.has(it.item)));
      });
      invRow.append(b);
    }
    if (!(st.bench || []).length) invRow.append(mk('p', 'late-help', 'Your pack has nothing to react yet. A fuel (coal, wood or an H₂ balloon) and an O₂ flask would do.'));
    typedWrap.hidden = !(st.band === 'apprentice' || st.band === 'adept');
  };
  let triedSeen = 0;
  invTry.addEventListener('click', () => {
    const text = typedWrap.hidden ? '' : typed.value.trim();
    push(`invent ${[...chosen].join(',')}|${text}`);
  });
  const openBench = (/** @type {any} */ st) => {
    chosen.clear();
    typed.value = '';
    invOut.textContent = '';
    drawBench(st);
    show(inv.root, /** @type {HTMLElement | null} */ (invRow.querySelector('button')) || invTry);
  };

  // The ending card.
  const end = modal('ending', 'You relit the Great Kiln');
  end.root.classList.add('late-ending');
  const endTitle = mk('h2', 'late-end-title', 'You relit the Great Kiln!');
  const endTime = mk('p', 'late-end-time');
  const endList = mk('ul', 'late-end-list');
  for (const p of PUZZLES) endList.append(mk('li', '', p));
  const endSpells = mk('p', 'late-end-spells');
  const keep = /** @type {HTMLButtonElement} */ (mk('button', 'late-keep', 'Keep exploring'));
  keep.type = 'button';
  const again = /** @type {HTMLButtonElement} */ (mk('button', 'late-again', 'Play again'));
  again.type = 'button';
  const endRow = mk('div', 'late-row');
  endRow.append(keep, again);
  end.card.append(endTitle, endTime, mk('p', 'late-help', 'Puzzles solved:'), endList, endSpells, endRow);
  keep.addEventListener('click', () => { push('keep_exploring'); hide(); });
  again.addEventListener('click', () => { location.reload(); });

  const editable = (/** @type {EventTarget | null} */ t) => {
    const el = /** @type {HTMLElement | null} */ (t);
    return !!el && !!el.tagName && (/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable);
  };
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (shown) {
      if (e.code === 'Escape' && shown !== end.root) {
        e.preventDefault();
        e.stopImmediatePropagation();
        hide();
      } else if (e.code === 'Enter' && shown === inv.root && editable(e.target)) {
        e.preventDefault();
        invTry.click();
      } else if (!editable(e.target) && /^Digit[1-3]$/.test(e.code) && shown === gas.root) {
        const b = /** @type {HTMLButtonElement | undefined} */ (gasRow.querySelectorAll('button')[+e.code.slice(5) - 1]);
        if (b) { e.preventDefault(); b.click(); }
      }
      if (!editable(e.target)) e.stopImmediatePropagation();
      return;
    }
    if (e.code !== 'KeyE' || e.repeat || editable(e.target) || otherModal()) return;
    const s = parse(w.__gameState);
    const st = parse(w.__lateState);
    if (!s || s.mode !== 'voxel' || s.paused || !st || !st.near) return;
    e.preventDefault();
    if (st.near === 'crate' && st.crate && st.crate.state === 'floor' && (st.gases || []).length) openGas(st);
    else push('use');
  }, true);

  let sayN = 0;
  let sayTimer = 0;
  let openSeen = -1;
  let cardSeen = false;
  let lastKey = '';
  const tick = () => {
    const s = parse(w.__gameState);
    const st = parse(w.__lateState);
    const on = !!s && s.mode === 'voxel' && !!st;
    if (on && !sfx) {
      sfx = {};
      import('./sfx.js').then((m) => { sfx = m; }).catch(() => {});
    }
    if (on) {
      const prompt = st.near && !shown ? /** @type {any} */ (PROMPTS)[st.near] : '';
      chip.hidden = !prompt;
      if (prompt && chip.textContent !== prompt) chip.textContent = prompt;
      if (st.sayN !== sayN) {
        if (sayN && st.say) {
          say.textContent = st.say;
          say.hidden = false;
          clearTimeout(sayTimer);
          sayTimer = setTimeout(() => { say.hidden = true; }, 6000);
        }
        sayN = st.sayN;
      }
      if (openSeen < 0) openSeen = st.openBench;
      if (st.openBench !== openSeen) {
        openSeen = st.openBench;
        openBench(st);
      }
      if (shown === inv.root) {
        const key = JSON.stringify([st.bench, st.band]);
        if (key !== lastKey) { lastKey = key; drawBench(st); }
        if (st.tried && st.tried.n !== triedSeen) {
          triedSeen = st.tried.n;
          invOut.textContent = chem(st.tried.reason || '');
          invOut.className = `late-result ${st.tried.ok ? 'ok' : 'no'}`;
          play(st.tried.ok ? 'fanfare' : 'hiss');
        }
      } else if (st.tried) {
        triedSeen = st.tried.n;
      }
      if (st.card && !cardSeen) {
        cardSeen = true;
        if (shown) hide();
        endTime.textContent = `Time played: ${clock(st.played)}`;
        const spells = (s.spellsLearnt || []).length;
        endSpells.textContent = spells ? `Spells learnt: ${spells}` : '';
        play('fanfare');
        // A moment to see the Kiln roar before the card.
        setTimeout(() => show(end.root, keep), 1500);
      }
      if (!st.card && shown === end.root) hide();
    } else {
      chip.hidden = true;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
