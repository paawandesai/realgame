// @ts-check
// The age picker: three pictures with a number under each ("4 to 7", "8 to 10",
// "11 and up"), shown once per page load before play, only when analytics is
// on (the rg-analytics switch) and the browser has not opted out (DNT or GPC).
// The choice is deliberately not stored: on shared classroom devices a
// remembered band would be counted for the next child. See docs/ANALYTICS.md.
//
// The markup is in index.html (hidden). Styles are set here through the
// CSSOM, which the page's CSP (style-src 'self') allows, so style.css is not
// touched. The WASM keeps loading behind the picker; after a pick the game is
// reset once, so time spent on the picker costs no quest time.
import { analyticsOn, optedOut, start } from './analytics.js';

/**
 * @param {HTMLElement | SVGElement} el
 * @param {Record<string, string>} props
 */
function css(el, props) {
  for (const [k, v] of Object.entries(props)) el.style.setProperty(k, v);
}

/** Calls the game's reset once its first state has arrived (R2: a pick before boot is queued). */
function resetWhenReady() {
  const w = /** @type {any} */ (window);
  const go = () => {
    try {
      w.__game.cmd('reset');
    } catch {
      // The game is not there: nothing to reset.
    }
  };
  if (w.__gameState && w.__game) {
    go();
    return;
  }
  const timer = setInterval(() => {
    if (w.__gameState && w.__game) {
      clearInterval(timer);
      go();
    }
  }, 100);
}

function style(/** @type {HTMLElement} */ root) {
  css(root, {
    position: 'fixed',
    inset: '0',
    'z-index': '1000',
    display: 'flex',
    'align-items': 'center',
    'justify-content': 'center',
    'overflow-y': 'auto',
    padding: '16px',
    'box-sizing': 'border-box',
    background: 'rgb(12, 20, 44)',
    color: '#ffffff',
    'font-family': 'system-ui, -apple-system, "Segoe UI", sans-serif',
  });
  const card = /** @type {HTMLElement | null} */ (root.querySelector('.age-card'));
  if (card) css(card, { 'text-align': 'center', 'max-width': '100%', margin: 'auto' });
  const title = /** @type {HTMLElement | null} */ (root.querySelector('h1'));
  if (title) css(title, { margin: '0 0 24px', 'font-size': '32px', 'font-weight': '700', 'line-height': '1.2' });
  const row = /** @type {HTMLElement | null} */ (root.querySelector('.age-choices'));
  if (row) css(row, { display: 'flex', 'flex-wrap': 'wrap', gap: '20px', 'justify-content': 'center' });
  root.querySelectorAll('.age-choice').forEach((b) => {
    const btn = /** @type {HTMLElement} */ (b);
    css(btn, {
      display: 'flex',
      'flex-direction': 'column',
      'align-items': 'center',
      'justify-content': 'flex-end',
      gap: '10px',
      'min-width': '168px',
      'min-height': '168px',
      padding: '16px 18px',
      'box-sizing': 'border-box',
      border: '3px solid #ffffff',
      'border-radius': '18px',
      background: '#fff8e7',
      color: '#1d2733',
      font: 'inherit',
      cursor: 'pointer',
    });
    const svg = btn.querySelector('svg');
    if (svg) css(svg, { width: '112px', height: '112px', display: 'block' });
    const label = /** @type {HTMLElement | null} */ (btn.querySelector('.age-label'));
    if (label) css(label, { 'font-size': '24px', 'font-weight': '700', 'line-height': '1.2', 'white-space': 'nowrap' });
    // A thick focus ring, the picker's only visual change (GS-6: no animation).
    btn.addEventListener('focus', () => css(btn, { outline: '5px solid #ffd23f', 'outline-offset': '4px' }));
    btn.addEventListener('blur', () => css(btn, { outline: 'none' }));
  });
}

/**
 * Shows the picker if analytics is on and the browser has not opted out.
 * Returns true when it was shown.
 */
export function startAgePicker() {
  try {
    if (!analyticsOn(document) || optedOut(/** @type {any} */ (navigator))) return false;
    const root = document.getElementById('age');
    if (!root) return false;
    const buttons = /** @type {HTMLButtonElement[]} */ ([...root.querySelectorAll('button[data-band]')]);
    if (buttons.length !== 3) return false;
    style(root);
    root.hidden = false;

    const move = (/** @type {number} */ step) => {
      const i = buttons.indexOf(/** @type {HTMLButtonElement} */ (document.activeElement));
      const next = buttons[(i < 0 ? 0 : i + step + buttons.length) % buttons.length];
      next?.focus();
    };
    // Modal: keys never reach the game or the HUD behind it (Esc, R, WASD...).
    // Tab and the arrow keys move between the three pictures; Enter and Space
    // keep their default action (pressing the focused button).
    const onKey = (/** @type {KeyboardEvent} */ e) => {
      e.stopImmediatePropagation();
      if (e.type !== 'keydown') return;
      if (e.key === 'Tab') {
        e.preventDefault();
        move(e.shiftKey ? -1 : 1);
      } else if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
        e.preventDefault();
        move(1);
      } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        e.preventDefault();
        move(-1);
      } else if (!buttons.includes(/** @type {HTMLButtonElement} */ (document.activeElement))) {
        buttons[0]?.focus();
      }
    };
    // Focus stays on the picker while it is open: the game focuses its canvas
    // when it finishes booting, which may happen behind the picker.
    /** @type {HTMLButtonElement | undefined} */
    let lastFocused = buttons[0];
    const onFocusIn = (/** @type {FocusEvent} */ e) => {
      const t = /** @type {HTMLButtonElement} */ (e.target);
      if (buttons.includes(t)) lastFocused = t;
      else (lastFocused ?? buttons[0])?.focus();
    };
    const swallow = (/** @type {Event} */ e) => e.stopPropagation();
    const pointerEvents = ['pointerdown', 'pointerup', 'pointermove', 'mousedown', 'mouseup', 'click', 'wheel', 'contextmenu'];
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('keyup', onKey, true);
    document.addEventListener('focusin', onFocusIn, true);
    for (const t of pointerEvents) root.addEventListener(t, swallow);

    let picked = false;
    for (const b of buttons) {
      b.addEventListener('click', () => {
        if (picked) return;
        picked = true;
        window.removeEventListener('keydown', onKey, true);
        window.removeEventListener('keyup', onKey, true);
        document.removeEventListener('focusin', onFocusIn, true);
        root.hidden = true;
        css(root, { display: 'none' });
        start(b.dataset.band ?? '');
        resetWhenReady();
        document.getElementById('game')?.focus();
      });
    }
    buttons[0]?.focus();
    return true;
  } catch {
    return false;
  }
}
