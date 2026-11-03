// Game feel (submission P7): footsteps per ground type, a landing thud,
// spell landings heard from where they land (left/right and distance),
// and a soft confirm on every UI button (P13). Reads `__gameState`'s
// voxel keys `stepMat`, `stride`, `lands`, `landV`, `hits`, `hitAt`,
// `hitSpell` (steps.rs). All sounds are synthesised in sfx.js. Only acts in
// the block world.

/** Metres walked per footstep (two steps per stride at a walk). */
export const STEP_M = 1.6;
/** Spell landing sounds by spell index (Fire, Water, Frost, CO2). */
export const HIT_SOUND = ['flame_whoosh', 'water_splash', 'ice_crack', 'hiss'];
/** Beyond this (m) a landing is barely heard. */
const HEAR_M = 45;

/**
 * Where a sound at `at` sits for a listener at `cam` facing `yawDeg`
 * (the game's yaw: forward is (-sin, -cos) in x, z): `pan` -1..1 and a
 * `volume` that falls with distance.
 * @param {[number, number, number]} cam @param {number} yawDeg @param {[number, number, number]} at
 */
export function place(cam, yawDeg, at) {
  const dx = at[0] - cam[0], dz = at[2] - cam[2];
  const d = Math.hypot(dx, at[1] - cam[1], dz);
  const flat = Math.hypot(dx, dz);
  const y = (yawDeg * Math.PI) / 180;
  // Right is (cos, -sin).
  const side = flat > 1e-3 ? (dx * Math.cos(y) - dz * Math.sin(y)) / flat : 0;
  const pan = Math.max(-1, Math.min(1, side * 0.85));
  const volume = Math.max(0.12, Math.min(1, 1 - d / HEAR_M));
  return { pan, volume };
}

/** How many footsteps fall between stride `from` and `to` (metres). @param {number} from @param {number} to */
export function stepsBetween(from, to) {
  if (!(to > from)) return 0;
  return Math.floor(to / STEP_M) - Math.floor(from / STEP_M);
}

/** A landing's volume for a fall speed (m/s). */
export const landVolume = (/** @type {number} */ v) => Math.max(0.35, Math.min(1, (v - 3) / 8));

export function startFeel() {
  const w = /** @type {any} */ (window);
  /** @type {any} */
  let sfx = null;
  let loading = false;
  const load = () => {
    if (sfx || loading) return;
    loading = true;
    import('./sfx.js').then((m) => { sfx = m; }).catch(() => {});
  };
  const play = (/** @type {string} */ n, /** @type {any} */ o) => { if (sfx) { try { sfx.play(n, o); } catch { /* no sound */ } } };

  // Every UI button gets the same soft confirm (not the canvas, not keys).
  document.addEventListener('click', (e) => {
    const t = /** @type {Element | null} */ (e.target instanceof Element ? e.target.closest('button') : null);
    if (t && !(/** @type {HTMLButtonElement} */ (t).disabled)) { load(); play('ui_confirm', { volume: 0.7 }); }
  }, true);

  /** @type {any} */
  let prev = null;
  const tick = () => {
    try {
      const raw = w.__gameState;
      const s = typeof raw === 'string' ? JSON.parse(raw) : null;
      if (s && s.mode === 'voxel' && typeof s.stride === 'number') {
        load();
        if (prev && !s.paused) {
          const n = Math.min(2, stepsBetween(prev.stride, s.stride));
          if (n > 0 && s.stepMat) play('step', { material: s.stepMat, volume: 0.55, pitch: 0.95 + Math.random() * 0.1 });
          if (s.lands > prev.lands) play('land', { material: s.stepMat || 'grass', volume: landVolume(s.landV) });
          if (s.hits > prev.hits && Array.isArray(s.hitAt) && Array.isArray(s.cam)) {
            const name = HIT_SOUND[s.hitSpell] || 'water_splash';
            const at = place(s.cam, s.yaw, s.hitAt);
            play(name, { pan: at.pan, volume: at.volume * 0.8 });
          }
        }
        prev = s;
      }
    } catch {
      // Feel must never stop the game.
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
