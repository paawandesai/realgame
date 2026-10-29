// Screen switching for v3 (DESIGN_V3 §3, §12.5). Track C owns this file
// after Hour 0: the end screen's Trials / Lab / Today's Trial buttons, the
// Trials menu, and moving between the story, a Trial and the Lab.
//
// Hour-0 stub: it only tracks which screen is showing and announces changes
// on the bus. main.ts calls `mountScreens` once the game exists and
// `showScreen` from its existing start, story and end-screen paths.

import { emit, type Screen } from "./bus";
import type { WebGame } from "./pkg/realgame_sim.js";
import type { Tier } from "./types";

/** What screens need from main.ts. Grows as Track C wires screens in. */
export interface ScreenHost {
  tier: Tier;
  /** The one WebGame; `new_trial` / `new_lab` / `new_story` swap its level in place. */
  game: WebGame;
}

let current: Screen | null = null;
let host: ScreenHost | null = null;

/** Called once by main.ts after the game is created. */
export function mountScreens(h: ScreenHost): void {
  host = h;
}

export function screenHost(): ScreenHost | null {
  return host;
}

/** Shows `screen` (Hour 0: records it) and emits `screen:change` if it changed. */
export function showScreen(screen: Screen): void {
  if (screen === current) return;
  const prev = current;
  current = screen;
  emit("screen:change", { screen, prev });
}

export function currentScreen(): Screen | null {
  return current;
}
