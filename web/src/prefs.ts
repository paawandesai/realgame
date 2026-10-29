// Per-device preferences in localStorage. Every access is wrapped: private
// windows and locked-down school profiles can block storage, and the game
// must work the same without it (it just forgets between visits).

import type { Tier } from "./types";

export function storageGet(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function storageSet(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Blocked storage: the setting lasts for this visit only.
  }
}

export function storageRemove(key: string): void {
  try {
    window.localStorage.removeItem(key);
  } catch {
    // Nothing to remove when storage is blocked.
  }
}

export const TIERS: readonly Tier[] = ["sprout", "seedling", "apprentice"];
export const SPEEDS = [0.5, 0.75, 1] as const;

export function isTier(v: unknown): v is Tier {
  return typeof v === "string" && (TIERS as readonly string[]).includes(v);
}

export function savedTier(): Tier | null {
  const t = storageGet("tier");
  return isTier(t) ? t : null;
}

export interface Prefs {
  /** Read lines aloud. Defaults on for the two younger tiers. */
  voice: boolean;
  /** Calmer screen: no shake or hit-stop, fewer particles. */
  reducedMotion: boolean;
  /** Game speed: 0.5, 0.75 or 1. */
  speed: number;
  /** Mentor captions on screen. Off by default for Apprentices, who get labels. */
  captions: boolean;
  /** Apprentice per-room score (casts, kJ, atoms wasted). */
  challenge: boolean;
}

export function loadPrefs(tier: Tier): Prefs {
  const voice = storageGet("voice");
  const motion = storageGet("reducedMotion");
  const speed = Number(storageGet("speed"));
  const captions = storageGet("captions");
  const challenge = storageGet("challenge");
  let systemReduced = false;
  try {
    systemReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    // Very old browsers: assume full motion.
  }
  return {
    voice: voice === null ? tier !== "apprentice" : voice === "on",
    reducedMotion: motion === null ? systemReduced : motion === "on",
    speed: (SPEEDS as readonly number[]).includes(speed) ? speed : 1,
    captions: captions === null ? tier !== "apprentice" : captions === "on",
    challenge: challenge !== "off",
  };
}

export function savePrefs(p: Prefs): void {
  storageSet("voice", p.voice ? "on" : "off");
  storageSet("reducedMotion", p.reducedMotion ? "on" : "off");
  storageSet("speed", String(p.speed));
  storageSet("captions", p.captions ? "on" : "off");
  storageSet("challenge", p.challenge ? "on" : "off");
}
