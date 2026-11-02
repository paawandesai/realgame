// Rank and powers (Voxel mode): the HUD badge, the rank-up moment and the
// power key. Rust (powers.rs) owns the rules; this draws
// `window.__powersState` and sends commands on `window.__gameIn.powers`:
//   power        use the selected spell's power (the F key does the same)
//   found <id>   a cache found on an earlier visit (from this device's save)
// Found caches are remembered on this device under STORE_KEY.

export const STORE_KEY = 'rg.caches.v1';

const parse = (/** @type {string | undefined} */ raw) => {
  try { return raw ? JSON.parse(raw) : null; } catch { return null; }
};

/** @param {string} cmd */
const push = (cmd) => {
  const w = /** @type {any} */ (window);
  w.__gameIn = w.__gameIn || {};
  (w.__gameIn.powers = w.__gameIn.powers || []).push(cmd);
};

/** Ids of found caches saved on this device. */
export const loadFound = (/** @type {Storage | null} */ store) => {
  try {
    const v = JSON.parse((store && store.getItem(STORE_KEY)) || '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string' && /^[a-z]{1,16}$/.test(x)) : [];
  } catch { return []; }
};

/** The badge's progress bar: 0..1 of the way to the next rank. */
export const toNext = (/** @type {{points:number, nextAt:number|null}} */ st, /** @type {number} */ from) => {
  if (st.nextAt == null) return 1;
  const span = Math.max(1, st.nextAt - from);
  return Math.max(0, Math.min(1, (st.points - from) / span));
};

/** Where each rank starts (the same as RANKS in powers.rs). */
export const RANK_AT = [0, 2, 5, 9, 13];

/** @param {string} tag @param {string} cls @param {string} [text] */
const mk = (tag, cls, text) => {
  const el = document.createElement(tag);
  if (cls) el.className = cls;
  if (text !== undefined) el.textContent = text;
  return el;
};

const SOUND = ['flame_whoosh', 'water_splash', 'ice_crack', 'hiss'];

export function startRank() {
  const w = /** @type {any} */ (window);
  let store = /** @type {Storage | null} */ (null);
  try { store = w.localStorage; } catch { store = null; }
  let sfx = /** @type {any} */ (null);
  const play = (/** @type {string} */ n) => {
    if (sfx && sfx.play) { try { sfx.play(n); } catch { /* no sound */ } }
  };

  const badge = mk('div', 'rank-badge');
  badge.id = 'rank';
  badge.hidden = true;
  const name = mk('div', 'rank-name');
  const bar = mk('div', 'rank-bar');
  const fill = mk('i', '');
  bar.append(fill);
  const next = mk('div', 'rank-next');
  const power = mk('div', 'rank-power');
  badge.append(name, bar, next, power);
  const up = mk('div', 'rank-up');
  up.id = 'rank-up';
  up.setAttribute('role', 'status');
  up.setAttribute('aria-live', 'polite');
  up.hidden = true;
  document.body.append(badge, up);

  let sent = false;
  let ups = -1;
  let used = /** @type {number[] | null} */ (null);
  let found = -1;
  let upTimer = 0;
  const tick = () => {
    const s = parse(w.__gameState);
    const st = parse(w.__powersState);
    const on = !!s && s.mode === 'voxel' && !!st;
    badge.hidden = !on;
    if (on) {
      if (!sfx) {
        sfx = {};
        import('./sfx.js').then((m) => { sfx = m; }).catch(() => {});
      }
      if (!sent) {
        sent = true;
        for (const id of loadFound(store)) push(`found ${id}`);
      }
      name.textContent = `Rank: ${st.rankName}`;
      fill.style.width = `${Math.round(toNext(st, RANK_AT[st.rank] || 0) * 100)}%`;
      next.textContent = st.next ? `${st.points} / ${st.nextAt} to ${st.next}` : `${st.points} points: top rank`;
      const lock = st.unlocked ? (st.cd > 0 ? ` (${st.cd.toFixed(1)} s)` : '') : ' (locked)';
      power.textContent = `F: ${st.power}${lock}`;
      power.classList.toggle('rank-locked', !st.unlocked);
      // Save found caches on this device.
      if (st.found !== found) {
        if (found >= 0 && st.found > found) play('pickup');
        found = st.found;
        const ids = (st.caches || []).filter((c) => c.found).map((c) => c.id);
        try { if (store && ids.length) store.setItem(STORE_KEY, JSON.stringify(ids)); } catch { /* no saving */ }
      }
      // A power was used: its sound.
      if (used) {
        for (let i = 0; i < 4; i++) if ((st.used[i] || 0) > (used[i] || 0)) play(SOUND[i]);
      }
      used = st.used.slice();
      // The rank-up moment.
      if (ups >= 0 && st.rankUps > ups) {
        up.textContent = '';
        up.append(mk('div', 'rank-up-title', 'Rank up!'), mk('div', 'rank-up-name', st.rankName),
          mk('div', 'rank-up-small', st.next ? `Next: ${st.next} at ${st.nextAt} points` : 'The top rank'));
        up.hidden = false;
        up.classList.remove('rank-pop');
        void up.offsetWidth;
        up.classList.add('rank-pop');
        play('fanfare');
        clearTimeout(upTimer);
        upTimer = setTimeout(() => { up.hidden = true; }, 3500);
      }
      ups = st.rankUps;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
