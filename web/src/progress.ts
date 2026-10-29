// v3 save data (DESIGN_V3 §6). Track C owns this file after Hour 0.
//
// One localStorage key, `alch.v3`, about 4 KB at most, every access in
// try/catch: with storage blocked (school machines, private windows) play
// carries on in memory and the end screen offers a progress code instead.
// No PII, no free text, no cross-session identifier: only card and Trial
// ids the game itself defines, star bits, solution signatures and dates.
//
// Schema (version 3):
//   {
//     v: 3,
//     cards:  { [cardId]:  { found: boolean, stars: number, seer: boolean } },
//     trials: { [trialId]: { stars: number, sigs: number[] } },
//     lab:    { unlocked: string[] },
//     days:   string[]            // local "YYYY-MM-DD" a daily was played
//   }
// `stars` is a bitmask (1 = star 1, 2 = star 2, 4 = star 3), because stars
// can be earned in any order. `sigs` holds distinct `TrialResult.sig`s.

export interface CardSave {
  found: boolean;
  stars: number;
  seer: boolean;
}

export interface TrialSave {
  stars: number;
  sigs: number[];
}

export interface SaveV3 {
  v: 3;
  cards: Record<string, CardSave>;
  trials: Record<string, TrialSave>;
  lab: { unlocked: string[] };
  days: string[];
}

export type Star = 1 | 2 | 3;

export const SAVE_KEY = "alch.v3";
/** Keep the save small: stored JSON over this is not written. */
export const MAX_SAVE_BYTES = 4096;
const MAX_SIGS = 8;
const MAX_DAYS = 366;

function fresh(): SaveV3 {
  return { v: 3, cards: {}, trials: {}, lab: { unlocked: [] }, days: [] };
}

let data: SaveV3 = fresh();
let loaded = false;
let storageWorks = true;

function isObj(x: unknown): x is Record<string, unknown> {
  return typeof x === "object" && x !== null && !Array.isArray(x);
}

/** Validates a parsed save, dropping anything malformed. */
export function sanitize(raw: unknown): SaveV3 {
  const out = fresh();
  if (!isObj(raw) || raw.v !== 3) return out;
  if (isObj(raw.cards)) {
    for (const [id, c] of Object.entries(raw.cards)) {
      if (!isObj(c)) continue;
      out.cards[id] = { found: c.found === true, stars: Number(c.stars) & 7, seer: c.seer === true };
    }
  }
  if (isObj(raw.trials)) {
    for (const [id, t] of Object.entries(raw.trials)) {
      if (!isObj(t)) continue;
      const sigs = Array.isArray(t.sigs) ? t.sigs.filter((s): s is number => Number.isInteger(s)).slice(0, MAX_SIGS) : [];
      out.trials[id] = { stars: Number(t.stars) & 7, sigs };
    }
  }
  if (isObj(raw.lab) && Array.isArray(raw.lab.unlocked)) {
    out.lab.unlocked = raw.lab.unlocked.filter((s): s is string => typeof s === "string");
  }
  if (Array.isArray(raw.days)) {
    out.days = raw.days.filter((d): d is string => typeof d === "string" && /^\d{4}-\d{2}-\d{2}$/.test(d)).slice(-MAX_DAYS);
  }
  return out;
}

/** Reads the save (once; later calls return the in-memory copy). */
export function load(): SaveV3 {
  if (loaded) return data;
  loaded = true;
  try {
    const raw = window.localStorage.getItem(SAVE_KEY);
    data = raw ? sanitize(JSON.parse(raw)) : fresh();
  } catch {
    storageWorks = false;
    data = fresh();
  }
  return data;
}

function save(): void {
  try {
    const json = JSON.stringify(data);
    if (json.length > MAX_SAVE_BYTES) return;
    window.localStorage.setItem(SAVE_KEY, json);
  } catch {
    storageWorks = false;
  }
}

/** False once storage has thrown: the end screen should offer `exportCode()`. */
export function storageOk(): boolean {
  load();
  return storageWorks;
}

function card(id: string): CardSave {
  return (load().cards[id] ??= { found: false, stars: 0, seer: false });
}

/** A card's reaction fired. True the first time. */
export function markFound(id: string): boolean {
  const c = card(id);
  if (c.found) return false;
  c.found = true;
  c.stars |= 1;
  save();
  return true;
}

/** Awards a star on a card or a Trial. True if it was new. */
export function addStar(kind: "card" | "trial", id: string, star: Star): boolean {
  const bit = 1 << (star - 1);
  const rec = kind === "card" ? card(id) : (load().trials[id] ??= { stars: 0, sigs: [] });
  if (rec.stars & bit) return false;
  rec.stars |= bit;
  save();
  return true;
}

/** A correct Predict on this card's reaction (the cosmetic "Seer" eye). */
export function markSeer(id: string): boolean {
  const c = card(id);
  if (c.seer) return false;
  c.seer = true;
  save();
  return true;
}

/** Records a solved Trial's solution signature; returns how many distinct
 * signatures that Trial now has (2 or more earns star 3). */
export function recordSig(trialId: string, sig: number): number {
  const t = (load().trials[trialId] ??= { stars: 0, sigs: [] });
  if (!t.sigs.includes(sig) && t.sigs.length < MAX_SIGS) {
    t.sigs.push(sig);
    save();
  }
  return t.sigs.length;
}

/** Today's Trial was played on local date `day` ("YYYY-MM-DD"). Never a streak. */
export function markDay(day: string): void {
  const d = load().days;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day) || d.includes(day)) return;
  d.push(day);
  if (d.length > MAX_DAYS) d.shift();
  save();
}

/** A 12-character progress code (base32 of the bitfields plus a CRC8) for
 * storage-blocked play. Hour-0 stub: Track C implements it. */
export function exportCode(): string {
  return "";
}

/** Restores progress from `exportCode()` output. Hour-0 stub: always false. */
export function importCode(code: string): boolean {
  void code;
  return false;
}

/** Test seam: forget the in-memory copy so the next call reloads. */
export function resetForTests(): void {
  data = fresh();
  loaded = false;
  storageWorks = true;
}
