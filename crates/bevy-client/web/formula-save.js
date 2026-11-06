// One save for the world and the Chemistry Bench page (I-053, step 6).
//
// Both pages are served from the same origin, so they share localStorage.
// This module is the only code that reads or writes the shared keys:
//
//   formula.bench.found  JSON array of codex ids found on the Bench (the
//                        Bench page's existing key; the world adds the
//                        reactions it confirms, so finds flow both ways).
//   formula.spells.v1    JSON {v:1, saved:{id: builds}, slots:[id|null x4],
//                        quests:[id]}: discoveries saved as spells, how many
//                        times each was built (mastery: 2 builds, then
//                        instant), which spell slot each sits on, and the
//                        chemistry quests done.
//
// Shipped-spell progress stays in `rg.progress.v1` (web/progress.js) and the
// Bench page's band in `formula.bench.band`; neither is shared (the world asks
// the age band each visit, for privacy). Every access is wrapped: blocked or
// corrupt storage costs only saving.

export const FOUND_KEY = 'formula.bench.found';
export const SPELLS_KEY = 'formula.spells.v1';
/** Builds before a discovered spell casts instantly (crates/chem discover::MASTERY_BUILDS). */
export const MASTERY_BUILDS = 2;
const ID_RE = /^[a-z0-9_]{1,48}$/;
const MAX_IDS = 400;

/** @typedef {{ v: 1, saved: Record<string, number>, slots: (string | null)[], quests: string[] }} SpellSave */

/** @param {unknown} v @returns {string[]} */
const ids = (v) => (Array.isArray(v) ? [...new Set(v.filter((x) => typeof x === 'string' && ID_RE.test(x)))].slice(0, MAX_IDS) : []);

/** @returns {SpellSave} */
export const emptySave = () => ({ v: 1, saved: {}, slots: [null, null, null, null], quests: [] });

/**
 * A spell save from any parsed value: unknown fields dropped, ids checked,
 * builds clamped to 0..255.
 * @param {any} raw
 * @returns {SpellSave}
 */
export function cleanSave(raw) {
  const s = emptySave();
  if (!raw || typeof raw !== 'object') return s;
  if (raw.saved && typeof raw.saved === 'object') {
    for (const [k, n] of Object.entries(raw.saved).slice(0, MAX_IDS)) {
      if (ID_RE.test(k)) s.saved[k] = Math.max(0, Math.min(255, Math.floor(Number(n) || 0)));
    }
  }
  if (Array.isArray(raw.slots)) {
    s.slots = [0, 1, 2, 3].map((i) => (typeof raw.slots[i] === 'string' && ID_RE.test(raw.slots[i]) && raw.slots[i] in s.saved ? raw.slots[i] : null));
  }
  s.quests = ids(raw.quests);
  return s;
}

/**
 * Two saves merged: every saved spell from either, the larger build count,
 * every quest from either, and `b`'s slots (the newer writer's).
 * @param {SpellSave} a
 * @param {SpellSave} b
 * @returns {SpellSave}
 */
export function mergeSaves(a, b) {
  const out = emptySave();
  for (const s of [a, b]) for (const [k, n] of Object.entries(s.saved)) out.saved[k] = Math.max(out.saved[k] ?? 0, n);
  out.slots = b.slots.map((x) => (x && x in out.saved ? x : null));
  out.quests = [...new Set([...a.quests, ...b.quests])];
  return out;
}

/** @param {any} storage @param {string} key */
const read = (storage, key) => {
  try { return JSON.parse(storage.getItem(key) || 'null'); } catch { return null; }
};
/** @param {any} storage @param {string} key @param {unknown} v */
const write = (storage, key, v) => {
  try { storage.setItem(key, JSON.stringify(v)); return true; } catch { return false; }
};

/** @param {any} storage @returns {string[]} */
export const readFound = (storage) => ids(read(storage, FOUND_KEY));

/** Adds codex ids to the Bench's finds. @param {any} storage @param {string[]} add */
export function addFound(storage, add) {
  const now = readFound(storage);
  const next = ids([...now, ...add]);
  if (next.length !== now.length) write(storage, FOUND_KEY, next);
  return next;
}

/** @param {any} storage @returns {SpellSave} */
export const readSpells = (storage) => cleanSave(read(storage, SPELLS_KEY));

/**
 * Saves a found reaction as a spell (0 builds; kept if already saved).
 * @param {any} storage @param {string} id
 * @returns {boolean} written
 */
export function saveSpell(storage, id) {
  if (!ID_RE.test(id)) return false;
  const s = readSpells(storage);
  if (!(id in s.saved)) s.saved[id] = 0;
  return write(storage, SPELLS_KEY, s);
}

/**
 * Writes the world's view of the save, merged with what is stored (the Bench
 * page may have saved a spell meanwhile).
 * @param {any} storage @param {any} fromGame
 */
export function writeSpells(storage, fromGame) {
  return write(storage, SPELLS_KEY, mergeSaves(readSpells(storage), cleanSave(fromGame)));
}

/**
 * The commands that load a save into the game (`window.__gameIn.disc`).
 * @param {SpellSave} s @param {string[]} found
 * @returns {string[]}
 */
export function loadCommands(s, found) {
  const saved = Object.entries(s.saved).map(([k, n]) => `${k}:${n}`).join(',');
  return [
    `found ${found.join(',') || '-'}`,
    `saved ${saved || '-'}`,
    `slots ${s.slots.map((x) => x || '-').join(',')}`,
    `quests ${s.quests.join(',') || '-'}`,
  ];
}
