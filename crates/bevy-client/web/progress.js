// Spell progress, saved on this device (docs/loop/items/I-027/spec.md).
//
// The game owns the rules (crates/bevy-client/src/progress.rs) and publishes
// `progRev` and `progSave` in window.__gameState. This module only tells the
// game today's local day and the saved progress at boot, and writes
// `progSave` to localStorage when `progRev` grows (never per frame). Nothing
// leaves the device, and it does not depend on the analytics switch. Every
// storage access is wrapped, so blocked or corrupt storage costs only saving.

export const STORE_KEY = 'rg.progress.v1';
export const MAX_SAVE = 512;
export const SAVE_RE = /^p1(\|[a-z0-9:.]+)*$/;
// The same budget as analytics.js's POLL_MS.
const POLL_MS = 500;

/** @param {unknown} s @returns {s is string} */
const valid = (s) => typeof s === 'string' && s.length <= MAX_SAVE && SAVE_RE.test(s);

/**
 * Days since 1970-01-01 in the device's time zone.
 * @param {Date} date
 * @returns {number}
 */
export function localDay(date) {
  return Math.floor((date.getTime() - date.getTimezoneOffset() * 60000) / 86400000);
}

/**
 * The stored save if it is valid, else null. Never deletes or rewrites it.
 * @param {{ getItem(k: string): string | null }} storage
 * @returns {string | null}
 */
export function readSave(storage) {
  try {
    const s = storage.getItem(STORE_KEY);
    return valid(s) ? s : null;
  } catch {
    return null;
  }
}

/**
 * Writes `progSave` once each time `progRev` grows. Never throws.
 * @param {{ storage: { setItem(k: string, v: string): void }, getState: () => any }} opts
 * @returns {{ poll(): void }}
 */
export function createSaver({ storage, getState }) {
  let last = 0;
  return {
    poll() {
      try {
        const s = getState();
        if (!s || typeof s.progRev !== 'number' || s.progRev <= last) return;
        if (!valid(s.progSave)) return;
        storage.setItem(STORE_KEY, s.progSave);
        last = s.progRev;
      } catch {
        // Storage blocked: progress lasts for this page only.
      }
    },
  };
}

/**
 * Boot: send the day and the save to the game, then keep saving.
 * @param {any} [win] the page's window (a fake one in tests)
 */
export function startProgress(win = globalThis.window) {
  try {
    if (!win) return;
    let storage = null;
    try { storage = win.localStorage; } catch { storage = null; }
    const D = win.Date || Date;
    const send = (/** @type {string} */ c) => { try { win.__game?.cmd(c); } catch { /* no game hook */ } };
    let sentDay = localDay(new D());
    send(`prog day ${sentDay}`);
    const save = storage ? readSave(storage) : null;
    if (save) send(`prog load ${save}`);
    const saver = createSaver({
      storage: storage || { setItem() { throw new Error('no storage'); } },
      getState: () => {
        const raw = win.__gameState;
        return typeof raw === 'string' ? JSON.parse(raw) : raw;
      },
    });
    // The day is sent again once local midnight passes (I-030; I-027's
    // should-fix), so keep-it-sharp and day counts see the new day.
    const poll = () => {
      try {
        const today = localDay(new D());
        if (today !== sentDay) { sentDay = today; send(`prog day ${today}`); }
      } catch { /* no clock */ }
      saver.poll();
    };
    win.setInterval?.(poll, POLL_MS);
    win.document?.addEventListener?.('visibilitychange', () => {
      if (win.document.visibilityState === 'hidden') poll();
    });
    win.addEventListener?.('pagehide', poll);
  } catch {
    // Saving must never break the game.
  }
}
