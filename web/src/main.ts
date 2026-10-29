// Browser shell: start screen, input, the game loop and the HUD. All physics
// and chemistry run in the Rust simulation (crates/sim), compiled to
// WebAssembly.

import init, { WebGame, heat_lut, reaction_book } from "./pkg/realgame_sim.js";
import { Sfx } from "./audio";
import { BAR_SPELLS, HoverCard, parseBook, spellCardLines, type SpellDef } from "./book";
import { CARDS, SproutCards, type CardSpell } from "./cards";
import { FxLayer } from "./fx";
import { GlRenderer } from "./gl";
import { initDevice, isTouch, words } from "./device";
import { GoalMarkers } from "./goals";
import { Grimoire } from "./grimoire";
import { ObjectiveBanner } from "./objective";
import { PopupQueue } from "./popups";
import { icon, setIcon, type IconName } from "./icons";
import { loadPrefs, savePrefs, savedTier, storageGet, storageRemove, storageSet, isTier } from "./prefs";
import { FormulaRing, byId } from "./ring";
import { Speech, speakable } from "./speech";
import { SpellTray } from "./tray";
import { prettyFormula, type CastOutcome, type Fx, type GameState, type Goal, type Tier } from "./types";

const INPUT = { left: 1, right: 2, jump: 4, gather: 8 } as const;
/** Hotkeys 1-9: the five Grimoire spells, then spells the player typed. */
const MAX_SLOTS = 9;
/** The simulation always advances in fixed 60 Hz steps, whatever the display rate. */
const STEP_MS = 1000 / 60;
/** At most this many steps per frame (enough to hold full speed down to 30 fps);
 * anything beyond is dropped so a slow device slows the game down instead of
 * spiralling into ever longer frames. */
const MAX_STEPS_PER_FRAME = 2;
/** With no choice on the start screen, the youngest tier starts by itself. */
const AUTOSTART_MS = 8000;
/** Hint ladder: when each step (pulse, glowing card, full hint) shows after
 * no progress. Sprouts get help sooner. */
const HINT_IDLE_MS: Record<Tier, [number, number, number]> = {
  sprout: [3000, 6000, 12000],
  seedling: [8000, 16000, 24000],
  apprentice: [8000, 16000, 24000],
};
/** ...or straight to the full hint after this many misses in a row. */
const HINT_MISSES = 3;
const END_DELAY_MS = 2500;
/** The "new Grimoire page" card stays this long (or until the next input). */
const DISCOVER_MS = 1500;
/** The room-clear stamp. */
const ROOM_CLEAR_MS = 1900;

/** The spell each goal needs, written for each tier. */
const SPELLS: Record<Tier, Record<Goal["need"], string>> = {
  sprout: { water: "water", boom: "boom", split: "split" },
  seedling: { water: "H2O", boom: "2H2 + O2 -> 2H2O", split: "2H2O -> 2H2 + O2" },
  apprentice: { water: "H2O", boom: "2H2 + O2 -> 2H2O", split: "2H2O -> 2H2 + O2" },
};

/** Seedling: what the tile tray holds on entering each room. Room 2 starts
 * unbalanced on purpose (balancing it is the lesson), and room 3 the same
 * way for splitting water. */
/** The Seedling tray's suggested spell for each room's goal (rooms 1-4). */
const TRAY_STARTERS = ["H2O", "H2 + O2 -> H2O", "H2O -> H2 + O2", "H2O"];

/** Spells that work from the very first second (the suit starts charged). */
const STARTER_SLOTS = BAR_SPELLS.map((b) => b.formula);

/** What to aim at for each goal, in words and as a picture. */
const AIM_WORDS: Record<Goal["kind"], string> = { fire: "fire", ice: "ice", gas: "bubbles", brazier: "bowl" };
const GOAL_PICTURE: Record<Goal["kind"], IconName> = { fire: "fire", ice: "ice", gas: "bubbles", brazier: "bowl" };

/** The picture for a caption, from what it talks about (first match wins). */
const CAPTION_PICTURES: [RegExp, IconName][] = [
  [/\bcrystal|battery\b/i, "crystal"],
  [/\bice\b|\bmelt/i, "ice"],
  [/\bbubble|\bgas\b|\bsplit/i, "bubbles"],
  [/\bfire\b|\bflame|\bhedge|\bburn|\btorch/i, "fire"],
  [/\bbowl\b|\bbrazier/i, "bowl"],
  [/\bboom\b|\bbang\b/i, "boom"],
  [/\bwater\b|\bsplash/i, "drop"],
];
function capPicture(caption: string): IconName | null {
  return CAPTION_PICTURES.find(([re]) => re.test(caption))?.[1] ?? null;
}

const REALM_NAMES: Record<Tier, string> = {
  sprout: "Sprout",
  seedling: "Seedling",
  apprentice: "Apprentice",
};

/** Whether the page has had a user gesture (speech and audio need one). */
let gestured = false;
window.addEventListener("pointerdown", () => (gestured = true), { capture: true, once: true });
window.addEventListener("keydown", () => (gestured = true), { capture: true, once: true });

function speakNow(line: string): void {
  if (!gestured || typeof window.speechSynthesis === "undefined") return;
  try {
    window.speechSynthesis.cancel();
    const u = new SpeechSynthesisUtterance(line);
    u.rate = 0.9;
    window.speechSynthesis.speak(u);
  } catch {
    // No speech on this system; the words are on screen.
  }
}

/** Apprentice type-ahead: keys more than this far apart are separate actions. */
const TYPE_GAP_MS = 700;
/** How long a number key waits to see whether a formula follows ("2H2"). */
const HOTKEY_WAIT_MS = 250;
/** Words the spell box understands (see ALIASES in crates/sim/src/spell.rs). */
const SPELL_WORDS = ["water", "h20", "fire", "fireball", "methane", "boom", "pop", "split", "bubbles", "hydrogen", "oxygen", "cloud", "smother", "co2"];

/** Whether `text` (at least two keys) is how a spell word or formula begins. */
function startsSpell(text: string): boolean {
  const t = text.toLowerCase();
  if (t.length < 2) return false;
  // A coefficient, then an element this game uses (H, O, C, Cl) or a bracket.
  if (/^[1-9][hoc(]/.test(t)) return true;
  return SPELL_WORDS.some((w) => w.startsWith(t));
}

/** Sets an element's children, touching the DOM only if they changed (so a
 * focused button keeps its focus). */
function setChildren(el: HTMLElement, nodes: readonly Element[]): void {
  const now = el.children;
  if (now.length === nodes.length && nodes.every((n, i) => now[i] === n)) return;
  el.replaceChildren(...nodes);
}

/** The WebGL renderer is opt-in with ?gl=1 (and ?gl=0 always keeps Canvas2D). */
function wantWebGl(): boolean {
  return new URLSearchParams(window.location.search).get("gl") === "1";
}

async function boot(): Promise<void> {
  const wasm = await init();
  const saved = savedTier();
  if (saved) {
    start(saved, wasm.memory);
    return;
  }
  showStartScreen((tier) => start(tier, wasm.memory));
}

/**
 * Three picture buttons. With no choice and no activity for 8 s the game
 * starts by itself (as the last level picked on this device, else Sprout);
 * any pointer move, touch, focus or key press starts the wait over, so a
 * player who is still deciding is never whisked away.
 */
function showStartScreen(onChoose: (tier: Tier) => void): void {
  const screen = byId("start");
  screen.hidden = false;
  let chosen = false;
  const last = storageGet("lastTier");
  const fallback: Tier = isTier(last) ? last : "sprout";
  const choose = (tier: Tier, remember: boolean) => {
    if (chosen) return;
    chosen = true;
    window.clearInterval(timer);
    for (const [type, fn] of listeners) window.removeEventListener(type, fn, true);
    if (remember) {
      storageSet("tier", tier);
      storageSet("lastTier", tier);
    }
    onChoose(tier);
  };
  for (const pic of screen.querySelectorAll("[data-icon]")) setIcon(pic, pic.getAttribute("data-icon") as IconName);
  for (const b of document.querySelectorAll<HTMLButtonElement>("[data-tier]")) {
    const tier = b.dataset.tier;
    if (!isTier(tier)) continue;
    const name = REALM_NAMES[tier];
    if (isTier(last) && tier === last) {
      b.classList.add("last");
      const tag = document.createElement("span");
      tag.className = "age-last";
      tag.textContent = "Last time";
      b.append(tag);
    }
    b.addEventListener("focus", () => speakNow(name));
    b.addEventListener("pointerenter", () => speakNow(name));
    b.addEventListener("click", () => {
      speakNow(name);
      choose(tier, true);
    });
  }
  const fill = byId("autostart-fill");
  const text = byId("autostart-text");
  let began = performance.now();
  /** Activity puts the countdown back to the start. */
  const wake = () => {
    began = performance.now();
  };
  const listeners: [string, () => void][] = ["pointermove", "pointerdown", "touchstart", "keydown", "focusin", "wheel"].map((t) => [t, wake]);
  for (const [type, fn] of listeners) window.addEventListener(type, fn, true);
  const timer = window.setInterval(() => {
    const t = (performance.now() - began) / AUTOSTART_MS;
    fill.style.width = `${Math.min(100, t * 100)}%`;
    // Only a quiet screen counts down; while someone is choosing it just asks.
    text.textContent = t < 0.5 ? "Take your time" : `Starting as ${REALM_NAMES[fallback]} soon`;
    // Not remembered: the next visit asks again.
    if (t >= 1) choose(fallback, false);
  }, 100);
}

function start(tier: Tier, memory: WebAssembly.Memory): void {
  byId("start").hidden = true;
  byId("game").hidden = false;
  document.body.dataset.tier = tier;
  for (const pic of byId("rotate").querySelectorAll("[data-icon]")) setIcon(pic, pic.getAttribute("data-icon") as IconName);
  initDevice(() => renderTutorial());

  const prefs = loadPrefs(tier);
  const seed = (Math.random() * 1e9) >>> 0;
  const game = new WebGame(tier, seed);
  const w = game.width();
  const h = game.height();
  const canvas = byId<HTMLCanvasElement>("world");
  const stage = byId("stage");
  const ctx = canvas.getContext("2d", { alpha: false });
  if (!ctx) throw new Error("Canvas 2D is not available");

  const keys = new Set<string>();
  let lens = false;
  let lastCast = "";
  let castCount = 0;
  const slots: string[] = [...STARTER_SLOTS];
  /** What each typed hotkey spell is (its reaction, or the substance poured). */
  const typedKeys = new Map<string, string>();
  /** Two hotkeys with the same key are the same spell. */
  const slotKey = (f: string): string => {
    const def = BAR_SPELLS.find((b) => b.formula === f);
    if (def) return def.reaction ?? f;
    return typedKeys.get(f) ?? f;
  };
  /** Recent casts by id, so FX events can find their outcome and label. */
  const outcomes = new Map<number, CastOutcome>();
  const book = parseBook(reaction_book());
  const hover = new HoverCard(stage);
  const grimoire = new Grimoire(book, tier, hover);
  let state: GameState | null = null;
  let ended = false;
  let freePlay = false;
  let endTimer = 0;
  /** The room the HUD last saw the player in (-1: none yet). */
  let hudRoom = -1;
  let lastResult: CastOutcome | null = null;

  const replayBtn = byId<HTMLButtonElement>("replay");
  setIcon(replayBtn, "speaker");
  const speech = new Speech(prefs.voice, byId("mote"), byId("mote-text"), replayBtn);
  speech.captions = prefs.captions;
  // Sprout and Seedling captions carry a picture of what they are about.
  if (tier !== "apprentice") speech.pictureFor = capPicture;
  const sfx = new Sfx(tier);
  // Browsers only allow audio and speech after a gesture.
  const unlock = () => {
    sfx.unlock();
    speech.unlock();
  };
  window.addEventListener("pointerdown", unlock, { capture: true });
  window.addEventListener("keydown", unlock, { capture: true });

  const ring = new FormulaRing(tier, (formula) => requestCast(formula));
  // Optional WebGL renderer (?gl=1), on its own canvas over the 2D one (a
  // canvas can hold only one context). Input still lands on the 2D canvas
  // underneath; any failure keeps Canvas2D.
  let gl: GlRenderer | null = null;
  const glCanvas = document.createElement("canvas");
  glCanvas.id = "world-gl";
  glCanvas.setAttribute("aria-hidden", "true");
  if (wantWebGl()) {
    canvas.after(glCanvas);
    gl = GlRenderer.create(glCanvas, w, h, heat_lut());
    if (gl) game.set_cell_data(true);
    else glCanvas.remove();
  }
  document.body.dataset.renderer = gl ? "webgl" : "2d";
  const fx = new FxLayer(stage, gl ? [canvas, glCanvas] : [canvas], w, h, tier, (fix) => {
    if (tray) tray.set(fix);
    else {
      ring.set(fix);
      ring.toggle(true);
    }
  });
  fx.reducedOverride = prefs.reducedMotion;
  fx.glowLayer = gl === null;

  /** Back to Canvas2D for good (the browser dropped the WebGL context). */
  function dropWebGl(): void {
    gl = null;
    glCanvas.remove();
    game.set_cell_data(false);
    fx.glowLayer = true;
    document.body.dataset.renderer = "2d";
  }
  document.body.dataset.motion = prefs.reducedMotion ? "calm" : "full";

  const goals = new GoalMarkers(stage, byId("progress"), w, h, () => {
    const next = goals.next;
    if (next) autoWalk = { target: next.stand_x, stuck: 0, lastX: game.player_x() };
  });
  fx.follow(goals.layer);
  // The room's goal in one line: a banner on arrival, then a chip at the top.
  const objective = new ObjectiveBanner(byId("hud-pill"), byId("hud-banners"), tier);
  /** The Grimoire card, the room-clear stamp and the room score take turns. */
  const popups = new PopupQueue();
  // The cast-result bubble steers around whichever popup is up.
  fx.avoid = () => [
    popups.active,
    document.querySelector<HTMLElement>("#mote.show"),
    document.querySelector<HTMLElement>("#objective-banner:not(.shrink)"),
    document.querySelector<HTMLElement>("#goal-chip:not(.yield)"),
    document.querySelector<HTMLElement>("#hint-ghost"),
  ];
  objective.onAnnounce = (line) => speech.say(line, speakable(line), true);
  /** Room 1 is done: the Apprentice hint line switches to the current goal. */
  let room1Cleared = false;

  // Per-tier controls: picture cards, a tile tray, or the formula ring.
  let cards: SproutCards | null = null;
  let tray: SpellTray | null = null;
  if (tier === "sprout") {
    const root = byId("cards");
    root.hidden = false;
    cards = new SproutCards(root, canvas, (spell, drop) => {
      if (drop) game.set_aim(...toGrid(drop[0], drop[1]));
      requestCast(spell);
    });
    cards.unlock(["water"]);
  } else if (tier === "seedling") {
    const root = byId("tray");
    root.hidden = false;
    tray = new SpellTray(root, (t) => requestCast(t), (line) => speech.say(line));
    tray.suggest(1, TRAY_STARTERS[0]!);
    byId("jars-mini").hidden = false;
  } else {
    byId("hud-bottom").hidden = false;
    byId("jars-mini").hidden = false;
  }

  // The key list under the stage, for the tiers that cast with Enter.
  if (tier !== "apprentice") {
    const help = byId("help");
    const keysLine: [string, string[]][] = [
      ["Move", ["A", "D"]],
      ["Jump", ["Space"]],
      ["Point to aim", []],
      // A key picture, not a word: Seedling casts from the Cast button and
      // never needs the Apprentice "type, then Enter" words.
      ["Cast", ["↵"]],
      ["Gather", ["G"]],
      ["Grimoire", ["B"]],
    ];
    help.replaceChildren();
    keysLine.forEach(([what, ks], i) => {
      help.append(`${i > 0 ? " · " : ""}${what} `);
      for (const k of ks) {
        const kbd = document.createElement("kbd");
        kbd.textContent = k;
        help.append(kbd);
      }
    });
  }

  // ------------------------------------------------------------------ input

  const toGrid = (clientX: number, clientY: number): [number, number] => {
    const r = canvas.getBoundingClientRect();
    // The canvas shows the viewport; the camera's left edge turns it into world cells.
    return [
      game.view_x() + Math.floor(((clientX - r.left) / r.width) * w),
      Math.floor(((clientY - r.top) / r.height) * h),
    ];
  };

  // Pointing at the world only ever aims; casting is a separate action.
  // Apprentices aim by hovering; younger players aim with a deliberate tap,
  // so a passing mouse never steals soft auto-aim from them.
  let pointer: [number, number] | null = null;
  canvas.addEventListener("pointermove", (e) => {
    pointer = [e.clientX, e.clientY];
    const [x, y] = toGrid(e.clientX, e.clientY);
    if (tier === "apprentice") game.set_aim(x, y);
    else game.track_aim(x, y);
  });
  canvas.addEventListener("pointerdown", (e) => {
    pointer = [e.clientX, e.clientY];
    game.set_aim(...toGrid(e.clientX, e.clientY));
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());

  // A HUD button clicked with a mouse or finger hands focus back to the game,
  // so Space jumps and Enter casts instead of pressing that button again.
  // Keyboard users (detail 0) keep focus where they put it, and dialogs keep
  // theirs.
  document.addEventListener("click", (e) => {
    if (e.detail === 0 || !(e.target instanceof Element)) return;
    const b = e.target.closest("button");
    if (!b || b.closest("#settings, #grimoire-panel, #end, #start")) return;
    if (document.activeElement === b) b.blur();
  });

  /** Buttons, inputs and selects handle Enter and Space themselves. */
  const isControl = (t: EventTarget | null): boolean =>
    t instanceof HTMLElement && t.closest("button, input, select, textarea, [contenteditable]") !== null;

  /** Text boxes where every letter is part of a spell, not a hotkey. */
  const isTyping = (t: EventTarget | null): boolean =>
    t instanceof HTMLElement && t.closest("input, textarea, [contenteditable]") !== null;

  window.addEventListener("keydown", (e) => {
    const k = e.key.toLowerCase();
    if (k === "escape") {
      if (!menu.hidden && !trialConfirm.hidden) showTrialConfirm(false);
      else if (!menu.hidden) closeMenu();
      else if (!grimPanel.hidden) toggleGrimoire(false);
      else ring.toggle(false);
      return;
    }
    if (!menu.hidden || !byId("end").hidden) return;
    if ((k === "enter" || k === " ") && isControl(e.target)) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    // Apprentices type spells straight into the game; remember recent keys so
    // a word that starts with a game key ("water", "boom") or a formula that
    // starts with a digit ("2H2 + O2") still reaches the spell box whole.
    if (castInput && !e.repeat && e.key.length === 1) {
      const now = performance.now();
      if (now - typed.at > TYPE_GAP_MS) typed.keys = [];
      typed.at = now;
    }
    if (k === "l") {
      setLens(!lens);
      remember(e, () => setLens(!lens));
    } else if (k === "q" && tier !== "sprout") {
      // Q toggles the compact formula overlay. Never while typing a spell, and
      // never let the key itself land in the box the ring focuses.
      if (isTyping(e.target)) return;
      e.preventDefault();
      if (e.repeat) return;
      ring.toggle();
      remember(e, () => ring.toggle(false));
      return;
    } else if (k === "b" || k === "n") {
      const was = !grimPanel.hidden;
      toggleGrimoire();
      remember(e, () => toggleGrimoire(was));
    } else if (k === "tab" && castInput && !isControl(e.target)) {
      // Tab from the game (not from a button) picks the next Grimoire spell.
      e.preventDefault();
      cycleSpell(e.shiftKey ? -1 : 1);
      return;
    } else if (k === "enter") {
      e.preventDefault();
      if (e.repeat) return;
      // Apprentices type: Enter opens the spell box (already holding the last spell).
      if (castInput) focusCastBox();
      else requestCast();
      return;
    } else if (/^[1-9]$/.test(k) && !e.repeat) {
      const spell = tier === "sprout" ? CARDS[Number(k) - 1]?.spell : slots[Number(k) - 1];
      if (!spell || (tier === "sprout" && !cards?.element(spell as CardSpell))) return;
      if (castInput) {
        // A hotkey fires a moment later, unless it was the start of a formula.
        cancelHotkey();
        pendingHotkey = window.setTimeout(() => {
          pendingHotkey = 0;
          // Cast for real now: these keys can no longer start a formula.
          typed.keys = [];
          castSlot(spell);
        }, HOTKEY_WAIT_MS);
        remember(e, cancelHotkey);
        return;
      }
      castSlot(spell);
    } else if (k === " ") {
      e.preventDefault();
      remember(e, null);
    } else if (castInput && /^[a-z(]$/i.test(e.key) && !GAME_KEYS.has(k)) {
      // Starting to type a formula ("H2O", "CO2", "oxygen") anywhere opens the box.
      e.preventDefault();
      const lead = typeAheadLead(e.key);
      keys.clear();
      focusCastBox("");
      castInput.value = lead + e.key;
      return;
    } else remember(e, null);
    if (["a", "d", "arrowleft", "arrowright", " ", "w", "arrowup"].includes(k)) autoWalk = null;
    keys.add(k);
  });
  window.addEventListener("keyup", (e) => keys.delete(e.key.toLowerCase()));
  window.addEventListener("blur", () => keys.clear());

  /** Keys typed in quick succession, with how to undo what each one did. */
  const typed: { at: number; keys: { ch: string; undo: (() => void) | null }[] } = { at: 0, keys: [] };
  let pendingHotkey = 0;
  function cancelHotkey(): void {
    window.clearTimeout(pendingHotkey);
    pendingHotkey = 0;
  }
  function remember(e: KeyboardEvent, undo: (() => void) | null): void {
    if (castInput && !e.repeat && e.key.length === 1) typed.keys.push({ ch: e.key, undo });
  }
  /** The run of just-typed keys that, with `ch`, starts a spell word or a
   * formula; what those keys did (a hop, the Grimoire, a hotkey) is undone. */
  function typeAheadLead(ch: string): string {
    const list = typed.keys;
    typed.keys = [];
    for (let n = list.length; n > 0; n--) {
      const run = list.slice(-n);
      const lead = run.map((t) => t.ch).join("");
      if (startsSpell(lead + ch)) {
        for (const t of run.reverse()) t.undo?.();
        return lead;
      }
    }
    return "";
  }

  // On-screen move buttons (always shown to the younger tiers, on touch for all).
  const held = new Set<string>();
  const moveIcons: Record<string, IconName> = { left: "left", right: "right", jump: "jump" };
  for (const b of document.querySelectorAll<HTMLButtonElement>("#moves [data-key]")) {
    const key = b.dataset.key ?? "";
    const pic = moveIcons[key];
    if (pic) setIcon(b, pic);
    b.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      autoWalk = null;
      held.add(key);
    });
    const release = () => held.delete(key);
    b.addEventListener("pointerup", release);
    b.addEventListener("pointerleave", release);
    b.addEventListener("pointercancel", release);
    // Keyboard users: Enter on a move button takes a short step.
    b.addEventListener("click", (e) => {
      if (e.detail !== 0) return;
      held.add(key);
      window.setTimeout(() => held.delete(key), 250);
    });
  }

  byId("lens-btn").addEventListener("click", () => setLens(!lens));
  // The heat lens button has its own corner, apart from the spell row.
  byId("lens-btn").hidden = tier !== "apprentice";

  // Hold-to-gather buttons. A quick tap still counts: the press lasts at
  // least a quarter of a second so the simulation sees it.
  for (const b of document.querySelectorAll<HTMLButtonElement>("[data-hold]")) {
    const key = b.dataset.hold ?? "";
    setIcon(b, "scoop");
    let downAt = 0;
    b.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      downAt = performance.now();
      held.add(key);
    });
    const release = () => {
      const left = Math.max(0, 250 - (performance.now() - downAt));
      window.setTimeout(() => held.delete(key), left);
    };
    b.addEventListener("pointerup", release);
    b.addEventListener("pointercancel", release);
    b.addEventListener("click", (e) => {
      if (e.detail !== 0) return;
      downAt = performance.now();
      held.add(key);
      release();
    });
  }

  // Seedling on touch: Cast and a button that opens the tile tray as a drawer.
  const actTray = byId<HTMLButtonElement>("act-tray");
  setIcon(byId("act-cast"), "wand");
  if (tier === "seedling") byId("actions").hidden = false;
  const setTrayOpen = (open: boolean) => {
    document.body.classList.toggle("tray-open", open);
    actTray.setAttribute("aria-expanded", String(open));
  };
  actTray.addEventListener("click", () => setTrayOpen(!document.body.classList.contains("tray-open")));
  byId("act-cast").addEventListener("click", () => requestCast());
  // Apprentice on touch: the pencil opens the formula ring (tiles plus a text box).
  setIcon(byId("app-write"), "pencil");
  byId("app-write").addEventListener("click", () => ring.toggle());

  function setLens(on: boolean): void {
    lens = on;
    byId("lens-btn").setAttribute("aria-pressed", String(on));
    legend.hidden = !on;
    if (!on) for (const el of spotEls) el.hidden = true;
    lastSpotsAt = -Infinity;
  }

  // The heat lens legend: the same colours the lens draws (the sim's own
  // lookup table), on the same square-root temperature scale, with the
  // temperatures the game is about marked.
  const LUT = heat_lut();
  const TEMP_MIN = -60;
  const TEMP_SPAN = 2860;
  /** The legend's scale: piecewise, so the temperatures that matter (0 and
   * 100 °C) get room apart, while flames still fit on the right. */
  const LEGEND_SCALE: [number, number][] = [
    [-60, 0],
    [0, 0.14],
    [100, 0.36],
    [500, 0.6],
    [1000, 0.8],
    [2000, 1],
  ];
  const legendPos = (t: number): number => {
    for (let i = 1; i < LEGEND_SCALE.length; i++) {
      const [t0, p0] = LEGEND_SCALE[i - 1]!;
      const [t1, p1] = LEGEND_SCALE[i]!;
      if (t <= t1) return p0 + ((Math.max(t, t0) - t0) / (t1 - t0)) * (p1 - p0);
    }
    return 1;
  };
  const legendTemp = (pos: number): number => {
    for (let i = 1; i < LEGEND_SCALE.length; i++) {
      const [t0, p0] = LEGEND_SCALE[i - 1]!;
      const [t1, p1] = LEGEND_SCALE[i]!;
      if (pos <= p1) return t0 + ((pos - p0) / (p1 - p0)) * (t1 - t0);
    }
    return 2000;
  };
  /** The lens colour at a temperature, from the sim's lookup table (byte
   * index as `palette::temp_byte`). */
  const lutAt = (t: number): string => {
    const u = Math.min(1, Math.max(0, (t - TEMP_MIN) / TEMP_SPAN));
    const b = Math.round(Math.sqrt(u) * 255) * 4;
    return `rgb(${LUT[b]}, ${LUT[b + 1]}, ${LUT[b + 2]})`;
  };
  const legend = byId("lens-legend");
  {
    const title = document.createElement("span");
    title.className = "legend-title";
    title.textContent = tier === "sprout" ? "Hot and cold" : "Heat lens, °C";
    const bar = document.createElement("div");
    bar.className = "legend-bar";
    const stops: string[] = [];
    for (let i = 0; i <= 32; i++) stops.push(`${lutAt(legendTemp(i / 32))} ${((i / 32) * 100).toFixed(1)}%`);
    bar.style.background = `linear-gradient(to right, ${stops.join(", ")})`;
    const ticks = document.createElement("div");
    ticks.className = "legend-ticks";
    const marks: [number, string, string][] = [
      [0, "0", "ice melts"],
      [100, "100", "water boils"],
      [500, "500", ""],
      [1000, "1000+", ""],
    ];
    for (const [t, label, note] of marks) {
      const tick = document.createElement("span");
      tick.className = "legend-tick";
      tick.style.left = `${(legendPos(t) * 100).toFixed(1)}%`;
      tick.textContent = label;
      if (note && tier !== "sprout") {
        const n = document.createElement("span");
        n.className = "legend-note";
        n.textContent = note;
        tick.append(n);
      }
      ticks.append(tick);
    }
    legend.append(title, bar, ticks);
  }

  /** Labels on the hottest (and coldest) spots while the lens is on. */
  const spotEls: HTMLElement[] = [];
  let lastSpotsAt = -Infinity;
  function updateSpots(now: number): void {
    if (!lens || now - lastSpotsAt < 250) return;
    lastSpotsAt = now;
    const flat = game.hotspots();
    const kx = stage.clientWidth / w;
    const ky = stage.clientHeight / h;
    for (let i = 0; i * 3 < Math.max(flat.length, spotEls.length * 3); i++) {
      let el = spotEls[i];
      if (!el) {
        el = document.createElement("span");
        el.className = "lens-spot";
        goals.layer.append(el);
        spotEls.push(el);
      }
      if (i * 3 >= flat.length) {
        el.hidden = true;
        continue;
      }
      const x = flat[i * 3]!;
      const y = flat[i * 3 + 1]!;
      const t = flat[i * 3 + 2]!;
      el.hidden = false;
      el.classList.toggle("cold", t < 0);
      el.textContent = tier === "sprout" ? (t < 0 ? "Cold!" : "Hot!") : `${Math.round(t)} °C`;
      el.style.transform = `translate(${(x * kx).toFixed(1)}px, ${(y * ky).toFixed(1)}px)`;
    }
  }

  // ---------------------------------------------------- Apprentice spell box

  /** Keys the game uses itself; any other letter starts typing a spell. */
  const GAME_KEYS = new Set(["a", "d", "w", "g", "l", "q", "b", "n"]);
  const castInput = tier === "apprentice" ? byId<HTMLInputElement>("cast-input") : null;
  /** Where Tab is in the spell list (-1: not cycling yet). */
  let cycleAt = -1;
  const tutorial = byId("app-tutorial");
  let tutorialOpen = tier === "apprentice" && storageGet("tutorialDone") !== "1";
  tutorial.hidden = !tutorialOpen;
  /** The how-to tip: which spell to try for the goal in front of the
   * player now (the pill already says the goal itself, so the tip never
   * repeats it). Refreshed whenever the goal changes. */
  function renderTutorial(): void {
    const t = byId("app-tutorial-text");
    const need = goals.next?.need ?? "water";
    const code = document.createElement("code");
    code.textContent = need === "water" ? "water" : need;
    if (isTouch()) t.replaceChildren("Tap a spell below, or tap the pencil to write one (try ", code, ").");
    else t.replaceChildren("Type a formula or a word (try ", code, "), then press Enter.");
  }
  renderTutorial();
  const closeTutorial = () => {
    if (!tutorialOpen) return;
    tutorialOpen = false;
    tutorial.hidden = true;
    storageSet("tutorialDone", "1");
  };
  byId("tutorial-skip").addEventListener("click", () => {
    closeTutorial();
    castInput?.focus();
  });

  function focusCastBox(text: string = lastCast): void {
    if (!castInput) return;
    castInput.value = text;
    castInput.focus();
    castInput.select();
  }

  function cycleSpell(dir: number): void {
    if (!castInput) return;
    const list = slots.filter(Boolean);
    if (list.length === 0) return;
    if (cycleAt < 0) {
      const at = list.indexOf(castInput.value.trim());
      cycleAt = at >= 0 ? at : dir > 0 ? -1 : 0;
    }
    cycleAt = (cycleAt + dir + list.length) % list.length;
    focusCastBox(list[cycleAt]);
    renderSlots();
  }

  if (castInput) {
    castInput.addEventListener("keydown", (e) => {
      // Typing letters such as L or Q must not trigger game shortcuts.
      e.stopPropagation();
      if (e.key === "Enter") {
        e.preventDefault();
        if (e.repeat) return;
        const text = castInput.value.trim() || lastCast;
        if (!text) return;
        requestCast(text);
        castInput.value = "";
        castInput.blur();
      } else if (e.key === "Escape") {
        castInput.blur();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        if (lastCast) focusCastBox(lastCast);
      } else if (e.key === "Tab" && !e.shiftKey && castInput.value.trim() === "" ) {
        e.preventDefault();
        cycleSpell(1);
      } else if (e.key === "Tab" && slots.includes(castInput.value.trim())) {
        e.preventDefault();
        cycleSpell(e.shiftKey ? -1 : 1);
      }
    });
    castInput.addEventListener("input", () => (cycleAt = -1));
    castInput.addEventListener("blur", () => (cycleAt = -1));
    byId("cast-go").addEventListener("click", () => {
      const text = castInput.value.trim() || lastCast || slots[0] || "";
      if (text) requestCast(text);
      castInput.value = "";
    });
  }

  // ----------------------------------------------------- per-room challenge

  /** Apprentice: each room's casts, energy and wasted atoms (all from this run of the seed). */
  const challenge = { room: 0, casts: 0, kj: 0, kjIn: 0, wasted: 0 };
  const challengeEl = byId("challenge");
  /** How long the room score stays up (it also goes on the next room). */
  const SCORE_MS = 4000;
  function scoreCast(r: CastOutcome): void {
    if (tier !== "apprentice" || r.outcome === "unknown") return;
    challenge.casts++;
    // Heat given out and energy taken in are kept apart: they are different things.
    if (r.dh_kj < 0) challenge.kj -= r.dh_kj;
    else challenge.kjIn += r.dh_kj;
    for (const n of Object.values(r.leftovers)) challenge.wasted += n;
  }
  function roomCleared(room: number): void {
    if (tier !== "apprentice" || !prefs.challenge) {
      challenge.casts = challenge.kj = challenge.kjIn = challenge.wasted = 0;
      return;
    }
    const key = `best-room-${room}`;
    const best = Number(storageGet(key) ?? "0");
    const isBest = best === 0 || challenge.casts < best;
    if (isBest) storageSet(key, String(challenge.casts));
    challengeEl.textContent =
      `Room ${room + 1} clear · ${challenge.casts} cast${challenge.casts === 1 ? "" : "s"} · ` +
      `${challenge.kj.toFixed(1)} kJ heat released · ` +
      (challenge.kjIn > 0 ? `${challenge.kjIn.toFixed(1)} kJ energy in · ` : "") +
      `${challenge.wasted} atom${challenge.wasted === 1 ? "" : "s"} wasted` +
      (isBest ? " · best" : ` · best ${best} casts`);
    popups.push({ kind: "score", el: challengeEl, ms: SCORE_MS, dismissOnInput: true });
    challenge.casts = challenge.kj = challenge.kjIn = challenge.wasted = 0;
  }

  // ----------------------------------------------------------- Grimoire

  const bookBtn = byId<HTMLButtonElement>("book-btn");
  setIcon(bookBtn, "book");
  const grimPanel = byId("grimoire-panel");
  /** Pages found since the book was last open: they flip over when it opens. */
  const unseenPages = new Set<string>();
  function renderGrimoire(): void {
    const open = !grimPanel.hidden;
    grimoire.render(byId("grimoire-cards"), open ? unseenPages : new Set());
    if (open) unseenPages.clear();
    byId("grimoire-count").textContent =
      tier === "sprout" ? "" : `${grimoire.found.size} of ${grimoire.total} pages found`;
    bookBtn.dataset.count = String(grimoire.found.size);
  }
  function toggleGrimoire(open = grimPanel.hidden): void {
    grimPanel.hidden = !open;
    bookBtn.setAttribute("aria-expanded", String(open));
    if (open) {
      renderGrimoire();
      byId("grimoire-close").focus();
    } else hover.hide();
  }
  bookBtn.addEventListener("click", () => toggleGrimoire());
  byId("grimoire-close").addEventListener("click", () => {
    toggleGrimoire(false);
    bookBtn.focus();
  });
  renderGrimoire();

  /** Walking to the next goal by itself (the go arrow), hopping when blocked. */
  let autoWalk: { target: number; stuck: number; lastX: number } | null = null;

  function inputBits(): number {
    let bits = 0;
    if (keys.has("a") || keys.has("arrowleft") || held.has("left")) bits |= INPUT.left;
    if (keys.has("d") || keys.has("arrowright") || held.has("right")) bits |= INPUT.right;
    if (keys.has(" ") || keys.has("w") || keys.has("arrowup") || held.has("jump")) bits |= INPUT.jump;
    if (keys.has("g") || held.has("gather")) bits |= INPUT.gather;
    if (autoWalk && bits === 0) {
      const x = game.player_x();
      const cx = x + 3;
      if (Math.abs(cx - autoWalk.target) < 2) {
        autoWalk = null;
      } else {
        bits |= autoWalk.target > cx ? INPUT.right : INPUT.left;
        autoWalk.stuck = Math.abs(x - autoWalk.lastX) < 0.3 ? autoWalk.stuck + 1 : 0;
        autoWalk.lastX = x;
        if (autoWalk.stuck > 2 && autoWalk.stuck % 24 < 14) bits |= INPUT.jump;
        if (autoWalk.stuck > 600) autoWalk = null;
      }
    }
    return bits;
  }

  // ---------------------------------------------------------------- casting

  /**
   * The one way a spell gets cast: keyboard, Cast buttons, cards, tiles, quick
   * slots and the ring all come here. With no formula it casts what the
   * tray or ring holds (or the last spell).
   */
  function requestCast(formula?: string): void {
    let f = formula;
    if (f === undefined) {
      if (tray) f = tray.text();
      else if (ring.isOpen) f = ring.text();
      else f = lastCast || (tier === "sprout" ? "water" : slots[0]) || "";
    }
    f = f.trim();
    if (!f) {
      if (tier === "apprentice") ring.toggle(true);
      return;
    }
    doCast(f);
    ring.toggle(false);
  }

  /** A hotkey: a locked spell says where its reagent comes from instead. */
  function castSlot(f: string): void {
    const lock = tier === "apprentice" ? lockFor(f) : null;
    if (lock) {
      speech.say(lock.hint(), speakable(lock.hint()), true);
      return;
    }
    requestCast(f);
  }

  function doCast(formula: string): void {
    const result = JSON.parse(game.cast(formula)) as CastOutcome;
    lastResult = result;
    outcomes.set(result.id, result);
    if (outcomes.size > 32) outcomes.delete(outcomes.keys().next().value as number);
    // Events from the cast itself (charge, bolt, fizzle) show at once.
    handleFx(JSON.parse(game.take_fx()) as Fx[]);
    castCount++;
    // Apprentices read the floating label; the caption would only repeat it.
    if (tier !== "apprentice") speech.say(words(result.label).replace(/\n/g, " "), outcomeSpeech(result));
    hintOnCast(result);
    scoreCast(result);
    afterCastNudges(result);
    lastCastRoom = goals.next?.room ?? -1;
    if (tray) {
      // An unbalanced cast keeps the tray open with the number to change
      // glowing, so the fix is one tap away; anything else tucks it away.
      const unbalanced = result.imbalance.length > 0;
      if (!unbalanced) setTrayOpen(false);
      if (result.suggestion) tray.offerFix(result.suggestion);
      document.body.classList.toggle("cast-full", result.outcome === "full");
      window.setTimeout(() => document.body.classList.remove("cast-full"), 1200);
    }
    if (result.outcome === "full" || result.outcome === "partial") {
      lastCast = formula;
      closeTutorial();
      // A balanced spell the player typed joins the hotkeys after the five
      // Grimoire spells, in its simplest form; a multiple of a spell already
      // there (4H2 + 2O2 -> 4H2O) is the same spell, and a lopsided cast is
      // never kept.
      const key = result.reaction_id ?? result.canonical;
      if (tier === "apprentice" && result.outcome === "full" && result.canonical && key && !slots.some((f) => slotKey(f) === key)) {
        if (slots.length >= MAX_SLOTS) {
          const gone = slots.splice(STARTER_SLOTS.length, 1)[0];
          if (gone) typedKeys.delete(gone);
        }
        slots.push(result.canonical);
        typedKeys.set(result.canonical, key);
      }
      // Only a balanced spell finds its Grimoire page.
      if (result.first_discovery && result.reaction_id) foundPage(result.reaction_id);
    }
    renderSlots();
  }

  /** A reaction happened: a new Grimoire page flips face up the first time. */
  function foundPage(id: string): void {
    if (!grimoire.discover(id)) return;
    const host = byId("discover");
    grimoire.celebrate(host, id, fx.reducedMotion);
    popups.push({ kind: "discover", el: host, ms: DISCOVER_MS, dismissOnInput: true });
    unseenPages.add(id);
    renderGrimoire();
  }

  /** What gets read aloud for a cast: the label, plus a nudge on a miss. */
  function outcomeSpeech(r: CastOutcome): string {
    let line = words(r.label).replace(/\n/g, " ");
    if (r.outcome === "unknown" && tier === "sprout") line = "Hmm, that one is not a spell yet.";
    return speakable(line);
  }

  // ------------------------------------------------------------ hint ladder

  const hintGhost = byId("hint-ghost");
  const hintText = byId("hint-text");
  const hintUse = byId<HTMLButtonElement>("hint-use");
  const ghostHand = byId("ghost-hand");
  setIcon(ghostHand, "hand");
  const hint = { floor: 0, misses: 0, since: performance.now(), key: "", level: 0, nudged: false, ghostKey: "" };
  /** Boom is the answer right now (after the first bubbles in room 3, or
   * water thrown at the ice): its card glows at once, not after a wait.
   * Holds the kind of goal it's for, and ends when that goal does. */
  let nudgeFor: Goal["kind"] | null = null;
  /** The room whose goal the player last cast at (-1: none yet). */
  let lastCastRoom = -1;
  /** Casts since the goals last moved on; two in a row show a pointer. */
  let castsNoProgress = 0;
  let progressSig = "";
  const meters: number[] = [];
  let hintSpell = "";
  /** The kind of spell ("boom", "water", ...) that last worked, and the
   * room it was cast in: the hint never offers it straight back. */
  let lastWorked: { spell: string; room: number } | null = null;
  hintUse.addEventListener("click", () => {
    if (!hintSpell) return;
    if (tray) tray.set(hintSpell);
    else {
      ring.set(hintSpell);
      ring.toggle(true);
    }
  });

  function hintOnCast(r: CastOutcome): void {
    if (r.outcome === "full" || r.outcome === "partial") {
      hint.floor = 0;
      hint.misses = 0;
      lastWorked = { spell: r.spell, room: game.room() };
    } else {
      hint.misses++;
      hint.floor = Math.max(hint.floor, hint.level);
      if (hint.misses >= HINT_MISSES) hint.floor = 3;
    }
    hint.since = performance.now();
  }

  /** Room 3: the first bubbles point straight at Boom; and any two casts
   * that don't move the goal on bring up a pointer at the target. */
  function afterCastNudges(r: CastOutcome): void {
    const next = goals.next;
    const worked = r.outcome === "full" || r.outcome === "partial";
    if (worked && r.spell === "split" && next?.kind === "gas" && nudgeFor === null) {
      nudgeFor = "gas";
      speech.say(
        tier === "sprout"
          ? "Now light the bubbles! Tap boom!"
          : tier === "seedling"
            ? "Now light the bubbles with boom!"
            : "Now light the trapped gas: 2H2 + O2 -> 2H2O",
      );
    } else if (r.spell === "boom" && worked) nudgeFor = null;
    castsNoProgress++;
    if (castsNoProgress >= 2) {
      goals.pointAt = true;
      hint.floor = Math.max(hint.floor, 1);
    }
  }

  /** The goals moved on (a goal done, its next step, or a fire meter falling). */
  function trackProgress(list: Goal[]): void {
    const sig = list.map((g) => `${g.done}:${g.need}`).join(",");
    let moved = sig !== progressSig;
    progressSig = sig;
    list.forEach((g, i) => {
      const m = g.meter ?? 0;
      const before = meters[i] ?? m;
      if (m < before - 0.1) moved = true;
      meters[i] = moved || m > before ? m : before;
    });
    if (moved) {
      castsNoProgress = 0;
      goals.pointAt = false;
    }
  }

  function resetHints(): void {
    nudgeFor = null;
    lastCastRoom = -1;
    castsNoProgress = 0;
    goals.pointAt = false;
    hint.floor = 0;
    hint.misses = 0;
    hint.since = performance.now();
    hint.key = "";
  }

  function hintTick(now: number): void {
    const next = goals.next;
    const key = next ? `${next.room}:${next.need}:${goals.goVisible}` : "done";
    const goalChanged = key !== hint.key;
    if (goalChanged) {
      hint.key = key;
      // A new goal (or step): any spell is fair to suggest again.
      lastWorked = null;
      hint.floor = 0;
      hint.misses = 0;
      hint.since = now;
    }
    const steps = HINT_IDLE_MS[tier];
    // Time spent in the menu or on the end screen doesn't count.
    if (!menu.hidden || ended) hint.since = now - (hint.level > hint.floor ? steps[hint.level - 1]! : 0);
    const waited = now - hint.since;
    const idle = steps.filter((ms) => waited >= ms).length;
    let level = next ? Math.min(3, Math.max(hint.floor, idle)) : 0;
    // Bubbles made: go straight to the glowing Boom card.
    if (nudgeFor !== null && next?.kind !== nudgeFor) nudgeFor = null;
    const nudged = nudgeFor !== null;
    if (nudged && next) level = Math.max(level, 2);
    if (goalChanged || level !== hint.level || nudged !== hint.nudged) applyHint(level, next);
    placeGhostHand(now);
  }

  function applyHint(level: number, next: Goal | null): void {
    hint.level = level;
    hint.nudged = nudgeFor !== null;
    document.body.dataset.hint = String(level);
    goals.setHintLevel(level);
    const need: Goal["need"] | null = next ? (nudgeFor !== null ? "boom" : next.need) : null;
    // That exact spell just worked here and the goal is not done: the
    // advice is where to aim, not the same spell again.
    const repeat = need !== null && next !== null && lastWorked?.spell === need && lastWorked.room === next.room - 1;
    if (repeat && level >= 1) goals.pointAt = true;
    const spell = need && !repeat ? SPELLS[tier][need] : "";
    hintSpell = spell;
    const glow = level >= 2 && !goals.goVisible && need !== null && !repeat;
    cards?.setHint(glow ? (need as CardSpell) : null);
    tray?.setHint(glow ? spell : null);
    const slot = glow ? spell : "";
    if (slot !== slotHint) {
      slotHint = slot;
      renderSlots();
    }
    const showGhost = level >= 3 && next !== null;
    // What the chip would say now: it is rewritten whenever that changes
    // (a new goal, or walking vs casting), so it never goes stale.
    // Mote reads the chip aloud but shows no caption: the chip is the words.
    const ghostKey = `${next?.room}:${need}:${goals.goVisible}:${repeat}`;
    if (showGhost && (hintGhost.hidden || ghostKey !== hint.ghostKey)) {
      hint.ghostKey = ghostKey;
      if (goals.goVisible) {
        hintText.textContent = "Walk this way";
        hintUse.hidden = true;
        speech.sayOnly("Walk this way! Tap the arrow.");
      } else if (repeat && next) {
        const where = AIM_WORDS[next.kind];
        hintUse.hidden = true;
        if (tier === "sprout") {
          hintText.replaceChildren(icon(GOAL_PICTURE[next.kind]));
          speech.sayOnly(`Again! Aim at the ${where}!`);
        } else {
          hintText.textContent = `Again: aim right at the ${where}`;
          speech.sayOnly(`Again: aim right at the ${where}.`);
        }
      } else if (tier === "sprout") {
        const card = CARDS.find((c) => c.spell === need);
        hintText.replaceChildren(icon(card?.picture ?? "drop"));
        hintUse.hidden = true;
        speech.sayOnly(`Try the ${card?.name ?? "water drop"}!`);
      } else {
        hintText.textContent = `Try: ${prettyFormula(spell)}`;
        hintUse.hidden = false;
        speech.sayOnly(`Try: ${prettyFormula(spell)}`, speakable(`Try ${spell}`));
      }
    }
    hintGhost.hidden = !showGhost;
  }

  /** Sprout: a ghost hand taps the card to use (or the go arrow). */
  function placeGhostHand(now: number): void {
    if (!cards) return;
    const next = goals.next;
    const onboarding = castCount === 0;
    // A goal nobody has cast at yet: show which card, and where it goes.
    const freshGoal = next !== null && next.room !== lastCastRoom;
    const show = next !== null && !ended && (onboarding || hint.level >= 3 || nudgeFor !== null || freshGoal);
    let target: Element | null = null;
    if (show) {
      const need = nudgeFor !== null ? "boom" : next.need;
      // The card just worked: point at the target (the goal pointer), not the card.
      const repeat = lastWorked?.spell === need && lastWorked.room === next.room - 1 && !goals.goVisible;
      target = goals.goVisible ? document.getElementById("go-arrow") : repeat ? null : cards.element(need as CardSpell);
    }
    ghostHand.hidden = !target;
    if (!target) return;
    const r = target.getBoundingClientRect();
    const s = stage.getBoundingClientRect();
    let x = r.left - s.left + r.width / 2;
    let y = r.top - s.top + r.height / 2;
    const goal = goals.goVisible ? null : goals.nextOnStage(game.view_x());
    if (goal && !fx.reducedMotion) {
      // Tap the card, then glide from the card to the target and rest there.
      const t = (now % 2000) / 2000;
      const k = t < 0.25 ? 0 : t > 0.75 ? 1 : (t - 0.25) / 0.5;
      const ease = k * k * (3 - 2 * k);
      x += (goal[0] - x) * ease;
      y += (goal[1] - y) * ease;
      if (t < 0.25) y += Math.max(0, Math.sin(t * 4 * Math.PI)) * 8;
    } else if (!fx.reducedMotion) {
      y += Math.max(0, Math.sin(now / 250)) * 8;
    }
    ghostHand.style.left = `${x}px`;
    ghostHand.style.top = `${y}px`;
  }


  // -------------------------------------------------------------------- HUD

  /** Hover-card details for a hotkey spell, from the sourced reaction book. */
  const spellDef = (f: string): SpellDef =>
    BAR_SPELLS.find((b) => b.formula === f) ?? { formula: f, name: "Your spell", picture: "wand", reaction: null, note: "A spell you typed." };

  /** Where the reagent a spell needs comes from, while the suit has none. */
  const METHANE_HINT = () => `${isTouch() ? "Hold the scoop" : "Hold G"} in the bubbles at the marsh (room 4) to gather methane.`;
  const LOCKS: Record<string, { jar: string; hint: () => string }> = {
    methane_combustion: { jar: "CH4", hint: METHANE_HINT },
    CO2: { jar: "CO2", hint: () => "Catch CO₂ first: stand near a flame and your suit pulls it in." },
  };
  /** The lock on a hotkey spell, or null when it can be cast now. */
  function lockFor(f: string): { jar: string; hint: () => string } | null {
    const def = spellDef(f);
    const lock = LOCKS[def.reaction ?? def.formula];
    if (!lock || freePlay) return null;
    return (state?.jars[lock.jar] ?? 0) > 0 ? null : lock;
  }

  /** The hotkey spell the hint ladder points at (Apprentice). */
  let slotHint = "";
  /** The spell the room is ready for right now (room 3's trapped gas). */
  let readySpell = "";
  /** Room 3: once the trap is full, pulse the Boom card (tile, slot). */
  function updateBoomReady(here: Goal | null): void {
    const ready = here !== null && here.kind === "gas" && !here.done && (here.steps?.[1]?.done ?? false);
    const spell = ready ? SPELLS[tier].boom : "";
    if (spell === readySpell) return;
    readySpell = spell;
    cards?.setReady(ready ? "boom" : null);
    if (hint.level < 2) tray?.setHint(ready ? spell : null);
    renderSlots();
  }
  /** Slot buttons, reused across renders (one per hotkey, never more). */
  const slotButtons: HTMLButtonElement[] = [];
  const slotsEl = byId("slots");
  const moreBtn = byId<HTMLButtonElement>("slots-more");
  const moreList = byId("slots-more-list");
  const setMoreOpen = (open: boolean) => {
    moreList.hidden = !open;
    moreBtn.setAttribute("aria-expanded", String(open));
  };
  moreBtn.addEventListener("click", () => setMoreOpen(moreList.hidden));
  // Picking a spell from the list (or tapping elsewhere) closes it.
  document.addEventListener("pointerdown", (e) => {
    if (!moreList.hidden && e.target instanceof Node && !moreList.contains(e.target) && !moreBtn.contains(e.target)) setMoreOpen(false);
  });

  function slotButton(i: number): HTMLButtonElement {
    let b = slotButtons[i];
    if (b) return b;
    b = document.createElement("button");
    b.type = "button";
    b.className = "slot";
    const n = document.createElement("kbd");
    const pic = document.createElement("span");
    pic.className = "slot-pic";
    const label = document.createElement("span");
    label.className = "slot-name";
    const need = document.createElement("span");
    need.className = "slot-need";
    need.hidden = true;
    b.append(n, pic, label, need);
    const btn = b;
    b.addEventListener("click", () => {
      const f = btn.dataset.formula;
      setMoreOpen(false);
      if (!f) return;
      const lock = lockFor(f);
      if (lock) {
        // Locked: say where the reagent comes from instead of fizzling.
        speech.say(lock.hint(), speakable(lock.hint()), true);
        btn.classList.remove("shake");
        void btn.offsetWidth;
        btn.classList.add("shake");
        return;
      }
      requestCast(f);
    });
    hover.attach(b, () => {
      const f = btn.dataset.formula ?? "";
      const def = spellDef(f);
      const c = spellCardLines(def, book);
      if (def.name === "Your spell") c.title = prettyFormula(f);
      const lock = lockFor(f);
      if (lock) c.lines.unshift(`Locked: ${lock.hint()}`);
      return c;
    });
    slotButtons[i] = b;
    return b;
  }

  function renderSlots(): void {
    if (tier !== "apprentice") return;
    hover.hide();
    slots.forEach((f, i) => {
      const def = spellDef(f);
      const b = slotButton(i);
      b.classList.toggle("suggested", (i === 0 && castCount === 0) || (slotHint !== "" && f === slotHint) || (readySpell !== "" && f === readySpell));
      b.classList.toggle("armed", i === cycleAt);
      if (b.dataset.formula !== f) {
        b.dataset.formula = f;
        b.setAttribute("aria-label", `${i + 1}: ${def.name}, ${f}`);
        const [n, pic, label] = b.children as unknown as [HTMLElement, HTMLElement, HTMLElement];
        n.textContent = String(i + 1);
        pic.replaceChildren(icon(def.picture));
        label.textContent = def.reaction === null && def.name === "Your spell" ? prettyFormula(f) : def.name;
      }
      const lock = lockFor(f);
      const need = b.children[3] as HTMLElement;
      b.classList.toggle("locked", lock !== null);
      b.setAttribute("aria-disabled", String(lock !== null));
      need.hidden = lock === null;
      if (lock) {
        need.textContent = `needs ${prettyFormula(lock.jar)}`;
        b.title = lock.hint();
      } else b.removeAttribute("title");
    });
    // Spells dropped from the list (a restart) lose their buttons.
    for (const b of slotButtons.splice(slots.length)) b.remove();
    fitSlots();
  }

  /**
   * The hotbar never runs under another control: first the slots shrink to
   * a picture and a number, then the last ones move into a "+N" list.
   */
  function fitSlots(): void {
    if (tier !== "apprentice") return;
    setChildren(slotsEl, slotButtons);
    slotsEl.classList.remove("compact");
    moreBtn.hidden = true;
    const overflows = () => slotsEl.scrollWidth > slotsEl.clientWidth + 1;
    if (overflows()) slotsEl.classList.add("compact");
    const kept = [...slotButtons];
    const spill: HTMLButtonElement[] = [];
    while (overflows() && kept.length > 1) {
      spill.unshift(kept.pop()!);
      moreBtn.hidden = false;
      moreBtn.textContent = `+${spill.length}`;
      moreBtn.setAttribute("aria-label", `${spill.length} more spells`);
      setChildren(slotsEl, kept);
    }
    setChildren(moreList, spill);
    if (spill.length === 0) setMoreOpen(false);
  }
  window.addEventListener("resize", () => fitSlots());

  const describeTemp = (t: number): string => {
    if (tier === "apprentice") return `${Math.round(t)} °C`;
    if (t < 0) return "freezing";
    if (t < 35) return "cool";
    if (t < 100) return "warm";
    if (t < 500) return "hot";
    return "very hot";
  };

  let lastHudKey = "";
  const gatesSeen: boolean[] = [];
  function renderHud(s: GameState): void {
    for (const m of s.messages) speech.say(m);
    byId("readout").textContent = `${s.aim_substance} · ${describeTemp(s.aim_temp_c)}`;

    // Cards and tray pre-fills follow the room the player has reached.
    if (cards) {
      const met: CardSpell[] = ["water"];
      if (s.room >= 1 || freePlay) met.push("boom");
      if (s.room >= 2 || freePlay) met.push("split");
      if (s.room >= 3 || freePlay) met.push("cloud");
      cards.unlock(met);
      cards.update(s);
    }
    if (s.room !== hudRoom) {
      // A new room: last room's score and stamp go. (The Seedling tray's
      // suggestion follows the goal, not the room the player stands in, so
      // stepping to and fro over a room edge never rewrites it.)
      if (hudRoom >= 0) popups.drop("score", "room-clear");
      hudRoom = s.room;
    }
    // A gate opening ends that room's challenge.
    s.gates_open.forEach((open, i) => {
      if (open && gatesSeen[i] === false && !freePlay) showRoomClear(i);
      if (open && !gatesSeen[i] && !freePlay) roomCleared(i);
      gatesSeen[i] = open;
    });
    if (s.trial_complete && !ended && !freePlay && endTimer === 0) {
      endTimer = window.setTimeout(showEnd, END_DELAY_MS);
    }

    const key = JSON.stringify([s.jars, Math.round((s.crystal_kj / s.crystal_max_kj) * 20)]);
    if (key === lastHudKey) return;
    lastHudKey = key;
    renderJars(s);
    renderSlots();
  }

  // ---------------------------------------------------------- suit gauges

  /** Where each gauge's contents come from, for its tooltip. */
  const JAR_INFO: Record<string, { name: string; from: () => string }> = {
    H2O: {
      name: "Water",
      from: () =>
        `Your condenser collar pulls water out of the air: +1 every 1.5 s. ${isTouch() ? "Hold the scoop" : "Hold G"} by a pool to fill up at once.`,
    },
    H2: {
      name: "Hydrogen",
      from: () =>
        "Your wrist electrolyser splits water from the tank: 2 H₂O → 2 H₂ + O₂, using 571.7 kJ of ⚡ battery each time. The sun recharges ⚡ slowly; a crystal fast.",
    },
    O2: {
      name: "Oxygen",
      from: () => "Made with the hydrogen: every 2 H₂O split gives 2 H₂ and 1 O₂.",
    },
    CH4: { name: "Methane", from: () => METHANE_HINT() },
    CO2: {
      name: "Carbon dioxide",
      from: () => "Caught from flames near your hand: burning wood or methane makes CO₂, and your suit's scrubber pulls it in.",
    },
  };
  /** The last thing that filled each gauge ("caught from the flame"). */
  const CAUSE_WORDS: Record<string, string> = {
    condense: "from the air",
    split: "from the electrolyser",
    capture: "caught from the flame",
    gather: "gathered",
  };
  const lastCause = new Map<string, string>();
  const jarEls = new Map<string, { root: HTMLElement; fill: HTMLElement; label: HTMLElement }>();
  let chargeEl: HTMLElement | null = null;
  let shownJars: Record<string, number> | null = null;
  fx.onGain = (jar, _n, cause) => lastCause.set(jar, cause);

  function jarEl(formula: string): { root: HTMLElement; fill: HTMLElement; label: HTMLElement } {
    let el = jarEls.get(formula);
    if (el) return el;
    const root = document.createElement("div");
    root.className = "jar";
    root.dataset.jar = formula;
    root.tabIndex = 0;
    const glass = document.createElement("div");
    glass.className = "glass";
    const fill = document.createElement("div");
    fill.className = `fill f-${formula.toLowerCase()}`;
    glass.append(fill);
    const label = document.createElement("span");
    root.append(glass, label);
    hover.attach(root, () => {
      const info = JAR_INFO[formula];
      const n = state?.jars[formula] ?? 0;
      const cause = lastCause.get(formula);
      return {
        title: `${info?.name ?? formula} (${prettyFormula(formula)}): ${n}`,
        lines: [info?.from() ?? "", cause ? `Last refill: ${CAUSE_WORDS[cause] ?? cause}.` : ""],
        source: "",
      };
    });
    el = { root, fill, label };
    jarEls.set(formula, el);
    return el;
  }

  /** A "+n" (or "−n") that floats up from a gauge whenever it changes. */
  function gaugeTick(formula: string, delta: number): void {
    const el = jarEls.get(formula);
    if (!el || !el.root.isConnected) return;
    const t = document.createElement("span");
    t.className = `gauge-tick ${delta > 0 ? "up" : "down"}`;
    t.textContent = `${delta > 0 ? "+" : "−"}${Math.abs(delta)}`;
    const cause = delta > 0 ? lastCause.get(formula) : undefined;
    if (cause) t.dataset.cause = cause;
    el.root.append(t);
    el.root.classList.remove("pulse");
    void el.root.offsetWidth;
    el.root.classList.add("pulse");
    window.setTimeout(() => t.remove(), 1400);
  }

  function renderJars(s: GameState): void {
    const jars = byId("jars-mini");
    const shown: HTMLElement[] = [];
    for (const [formula, n] of Object.entries(s.jars)) {
      if (tier !== "apprentice" && n === 0 && (formula === "CH4" || formula === "CO2")) continue;
      const el = jarEl(formula);
      el.root.classList.toggle("empty", n === 0);
      el.fill.style.height = `${Math.min(100, (n / (s.jar_caps[formula] ?? 10)) * 100)}%`;
      el.label.textContent = `${prettyFormula(formula)} ${n}`;
      el.root.setAttribute("aria-label", `${JAR_INFO[formula]?.name ?? formula}: ${n}`);
      shown.push(el.root);
    }
    if (tier !== "sprout") {
      if (!chargeEl) {
        chargeEl = document.createElement("div");
        chargeEl.className = "jar charge";
      }
      // The battery and what is charging it right now: a crystal (fast) or
      // the solar strip (slow). Splitting water draws it down.
      const pct = Math.round((s.crystal_kj / s.crystal_max_kj) * 100);
      const crystal = s.charge_source === "crystal";
      chargeEl.textContent = `⚡ ${pct}% ${crystal ? "💎" : "☀"}`;
      chargeEl.dataset.source = s.charge_source;
      chargeEl.title = crystal
        ? "Suit battery, charging fast from the crystal. Splitting water uses 571.7 kJ per 2 H₂O."
        : "Suit battery, charging slowly from the solar strip. Splitting water uses 571.7 kJ per 2 H₂O.";
      chargeEl.setAttribute("aria-label", `Battery ${pct}%, charging from ${crystal ? "the crystal" : "the sun"}`);
      shown.push(chargeEl);
    }
    setChildren(jars, shown);
    if (shownJars) {
      // The player's own casts get a chip on each gauge; what the suit did by
      // itself (condenser, electrolyser, scrubber) is pooled into one slow chip.
      for (const [formula, n] of Object.entries(s.jars)) {
        const d = n - (shownJars[formula] ?? 0);
        const bg = s.background[formula] ?? 0;
        const mine = d - bg;
        if (mine !== 0) gaugeTick(formula, mine);
        if (bg !== 0) suitPending.set(formula, (suitPending.get(formula) ?? 0) + bg);
      }
    } else {
      for (const [formula, bg] of Object.entries(s.background)) if (bg !== 0) suitPending.set(formula, (suitPending.get(formula) ?? 0) + bg);
    }
    shownJars = { ...s.jars };
    flushSuitChip();
  }

  // ------------------------------------------------- the suit's own refills

  /** At most one background chip this often, so casts stay readable. */
  const SUIT_CHIP_MS = 3000;
  const suitPending = new Map<string, number>();
  let suitChipAt = -Infinity;
  let suitExplained = false;
  let suitTimer = 0;
  /** One soft chip for everything the suit did by itself since the last
   * one: "⚙ suit: +1 H₂O, +2 H₂, +1 O₂, −2 H₂O" merged per gauge. */
  function flushSuitChip(): void {
    const now = performance.now();
    const wait = suitChipAt + SUIT_CHIP_MS - now;
    if (wait > 0) {
      window.clearTimeout(suitTimer);
      suitTimer = window.setTimeout(flushSuitChip, wait + 10);
      return;
    }
    const parts = [...suitPending].filter(([, n]) => n !== 0);
    suitPending.clear();
    if (parts.length === 0) return;
    const jars = byId("jars-mini");
    if (!jars.isConnected) return;
    suitChipAt = now;
    const chip = document.createElement("span");
    chip.className = "suit-tick";
    chip.setAttribute("aria-hidden", "true");
    const gear = document.createElement("span");
    gear.className = "suit-icon";
    gear.textContent = "⚙";
    const words = document.createElement("span");
    words.textContent =
      tier === "sprout"
        ? "suit refill"
        : parts.map(([f, n]) => `${n > 0 ? "+" : "−"}${Math.abs(n)} ${prettyFormula(f)}`).join(", ");
    chip.append(gear, words);
    jars.append(chip);
    window.setTimeout(() => chip.remove(), 2600);
    if (!suitExplained) {
      suitExplained = true;
      const note = document.createElement("span");
      note.className = "suit-note";
      note.textContent =
        tier === "sprout"
          ? "⚙ Your suit fills itself slowly."
          : tier === "seedling"
            ? "⚙ Your suit refills itself: water from air, split by ⚡."
            : "⚙ Grey chips are your suit working by itself: the collar condenses water from the air, and the electrolyser splits it (2H₂O → 2H₂ + O₂) using ⚡ battery energy. Your casts show in colour.";
      jars.append(note);
      window.setTimeout(() => note.remove(), 7000);
    }
  }

  /** A gate opened: a big star stamp so it reads as done. One element,
   * reused for every room. */
  const roomClearEl = document.createElement("div");
  roomClearEl.id = "room-clear";
  roomClearEl.setAttribute("role", "status");
  roomClearEl.hidden = true;
  const roomClearText = document.createElement("span");
  roomClearEl.append(icon("star"), roomClearText);
  // Banners take turns in the top column, under the pill and hint rows.
  const hudBanners = byId("hud-banners");
  hudBanners.append(roomClearEl, byId("challenge"));
  function showRoomClear(room: number): void {
    if (room >= 3) return; // The last door has its own end screen.
    objective.holdFor(ROOM_CLEAR_MS);
    roomClearText.textContent = tier === "sprout" ? "Yay! Gate open!" : `Room ${room + 1} clear! Gate open`;
    popups.push({ kind: "room-clear", el: roomClearEl, ms: ROOM_CLEAR_MS, dismissOnInput: true });
  }

  // ------------------------------------------------------------- end screen

  function showEnd(): void {
    endTimer = 0;
    if (freePlay) return;
    ended = true;
    // Nothing from the last room shows through the end screen.
    popups.drop();
    grimoire.render(byId("grimoire"), new Set(grimoire.found));
    byId("end-sub").textContent =
      tier === "sprout"
        ? "You did it!"
        : `You passed the Alchemist's Trial: ${grimoire.found.size} of ${grimoire.total} Grimoire pages found.`;
    byId("end").hidden = false;
    speech.say("Breakthrough! You passed the trial!");
    byId("end-replay").focus();
  }

  byId("end-replay").addEventListener("click", () => restartTrial());
  byId("end-age").addEventListener("click", changeAge);
  byId("end-free").addEventListener("click", () => {
    freePlay = true;
    ended = false;
    byId("end").hidden = true;
    game.free_play();
    cards?.unlock(["water", "boom", "split", "cloud"]);
    speech.say("Free play! Every door is open.");
  });

  // ------------------------------------------------------------------- menu

  const menu = byId("settings");
  const gear = byId<HTMLButtonElement>("settings-btn");
  setIcon(gear, "gear");
  for (const pic of menu.querySelectorAll("[data-icon]")) setIcon(pic, pic.getAttribute("data-icon") as IconName);
  const menuBody = byId("menu-body");
  const menuMore = byId("menu-more");
  const trialConfirm = byId("trial-confirm");
  /** "More below" shows while the menu has rows scrolled out of sight. */
  const updateMenuCue = () => {
    menuMore.hidden = menuBody.scrollTop + menuBody.clientHeight >= menuBody.scrollHeight - 4;
  };
  menuBody.addEventListener("scroll", updateMenuCue, { passive: true });
  window.addEventListener("resize", () => {
    if (!menu.hidden) updateMenuCue();
  });
  menuMore.addEventListener("click", () => menuBody.scrollBy({ top: menuBody.clientHeight * 0.8, behavior: "smooth" }));
  const showTrialConfirm = (open: boolean) => {
    trialConfirm.hidden = !open;
    menu.classList.toggle("confirming", open);
    if (open) {
      speech.say(tier === "sprout" ? "Start all over again?" : "Start the whole trial again?");
      byId("trial-no").focus();
    } else byId("menu-trial").focus();
  };
  const openMenu = () => {
    menu.hidden = false;
    trialConfirm.hidden = true;
    menu.classList.remove("confirming");
    gear.setAttribute("aria-expanded", "true");
    menuBody.scrollTop = 0;
    updateMenuCue();
    byId("menu-age").focus();
  };
  const closeMenu = () => {
    menu.hidden = true;
    trialConfirm.hidden = true;
    gear.setAttribute("aria-expanded", "false");
    gear.focus();
  };
  gear.addEventListener("click", () => (menu.hidden ? openMenu() : closeMenu()));
  byId("settings-close").addEventListener("click", closeMenu);
  // Keep Tab inside the open menu.
  menu.addEventListener("keydown", (e) => {
    if (e.key !== "Tab") return;
    const scope = trialConfirm.hidden ? menu : trialConfirm;
    const items = [...scope.querySelectorAll<HTMLElement>("button, input, select")].filter((el) => el.offsetParent !== null);
    const first = items[0];
    const last = items[items.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first?.focus();
    }
  });
  byId("menu-age").addEventListener("click", changeAge);
  byId("menu-room").addEventListener("click", () => {
    game.restart_room();
    resetHints();
    autoWalk = null;
    // The room's suggested spell goes back in on the next goals poll.
    tray?.reset();
    closeMenu();
    speech.say(tier === "sprout" ? "Let's try again!" : "Room restarted.");
  });
  // Restarting the whole trial throws away every room: ask first, with a picture.
  byId("menu-trial").addEventListener("click", () => showTrialConfirm(true));
  byId("trial-no").addEventListener("click", () => showTrialConfirm(false));
  byId("trial-yes").addEventListener("click", () => {
    trialConfirm.hidden = true;
    restartTrial();
    closeMenu();
    speech.say(tier === "sprout" ? "All new! Let's go!" : "Trial restarted.");
  });

  const mute = byId<HTMLInputElement>("sound-mute");
  const master = byId<HTMLInputElement>("sound-master");
  const effects = byId<HTMLInputElement>("sound-sfx");
  mute.checked = sfx.settings.muted;
  master.value = String(Math.round(sfx.settings.master * 100));
  effects.value = String(Math.round(sfx.settings.sfx * 100));
  mute.addEventListener("change", () => sfx.set({ muted: mute.checked }));
  master.addEventListener("input", () => sfx.set({ master: Number(master.value) / 100 }));
  effects.addEventListener("input", () => sfx.set({ sfx: Number(effects.value) / 100 }));
  const voice = byId<HTMLInputElement>("voice");
  voice.checked = prefs.voice;
  voice.addEventListener("change", () => {
    prefs.voice = voice.checked;
    speech.enabled = voice.checked;
    if (!voice.checked) speech.stop();
    savePrefs(prefs);
  });
  const captionsBox = byId<HTMLInputElement>("captions");
  captionsBox.checked = prefs.captions;
  captionsBox.addEventListener("change", () => {
    prefs.captions = captionsBox.checked;
    speech.captions = captionsBox.checked;
    savePrefs(prefs);
  });
  const challengeBox = byId<HTMLInputElement>("challenge-on");
  challengeBox.checked = prefs.challenge;
  challengeBox.closest("label")!.hidden = tier !== "apprentice";
  challengeBox.addEventListener("change", () => {
    prefs.challenge = challengeBox.checked;
    if (!challengeBox.checked) popups.drop("score");
    savePrefs(prefs);
  });
  const motion = byId<HTMLInputElement>("motion");
  motion.checked = prefs.reducedMotion;
  motion.addEventListener("change", () => {
    prefs.reducedMotion = motion.checked;
    fx.reducedOverride = motion.checked;
    document.body.dataset.motion = motion.checked ? "calm" : "full";
    savePrefs(prefs);
  });
  const speedSelect = byId<HTMLSelectElement>("speed");
  speedSelect.value = String(prefs.speed);
  speedSelect.addEventListener("change", () => {
    prefs.speed = Number(speedSelect.value) || 1;
    savePrefs(prefs);
  });

  function changeAge(): void {
    storageRemove("tier");
    speech.stop();
    window.location.reload();
  }

  function restartTrial(): void {
    game.restart();
    window.clearTimeout(endTimer);
    endTimer = 0;
    ended = false;
    freePlay = false;
    byId("end").hidden = true;
    popups.drop();
    castCount = 0;
    lastCast = "";
    outcomes.clear();
    slots.splice(0, slots.length, ...STARTER_SLOTS);
    typedKeys.clear();
    renderSlots();
    hudRoom = -1;
    room1Cleared = false;
    renderTutorial();
    tray?.reset();
    tray?.suggest(1, TRAY_STARTERS[0]!);
    cards?.reset();
    cards?.unlock(["water"]);
    autoWalk = null;
    resetHints();
    state = null;
    lastHudKey = "";
    stateVersion = -1;
  }

  // ------------------------------------------------------------------- loop

  // The frame is drawn straight from WebAssembly memory. The view is rebuilt
  // only when memory grows (which detaches the old buffer) or the pointer moves.
  let frameImage: ImageData | null = null;
  let frameBuffer: ArrayBuffer | null = null;
  let framePtr = -1;
  function frameFor(ptr: number): ImageData {
    if (frameImage === null || frameBuffer !== memory.buffer || framePtr !== ptr) {
      frameBuffer = memory.buffer;
      framePtr = ptr;
      frameImage = new ImageData(new Uint8ClampedArray(memory.buffer, ptr, w * h * 4), w, h);
    }
    return frameImage;
  }

  /** A byte view into WebAssembly memory (cached until memory grows). */
  const byteViews = new Map<number, Uint8Array>();
  let byteViewsBuffer: ArrayBuffer | null = null;
  function bytesAt(ptr: number, len: number): Uint8Array {
    if (byteViewsBuffer !== memory.buffer) {
      byteViews.clear();
      byteViewsBuffer = memory.buffer;
    }
    let v = byteViews.get(ptr);
    if (!v || v.length !== len) {
      v = new Uint8Array(memory.buffer, ptr, len);
      byteViews.set(ptr, v);
    }
    return v;
  }

  /** Presentation events from the simulation: particles, flashes, labels. */
  function handleFx(events: Fx[]): void {
    if (events.length === 0) return;
    sfx.handle(events, outcomes);
    fx.handle(events, outcomes, performance.now());
    // Reactions seen in the world (the torch's wood, a lit methane vent) find pages too.
    for (const e of events) {
      if (e.kind === "discover" && e.reaction_id) foundPage(e.reaction_id);
      // Water bounced off the ice: point straight at Boom.
      if (e.kind === "frost" && goals.next?.kind === "ice") nudgeFor = "ice";
    }
  }

  let accumulator = 0;
  let lastTime = performance.now();
  let lastHudTime = -Infinity;
  let lastGoalsTime = -Infinity;
  let shownGoalLine = "";
  let stateVersion = -1;

  // One frame's error must never stop the game: the next frame is always
  // scheduled, and the first error is logged once.
  let frameErrorLogged = false;
  function loop(): void {
    try {
      frame();
    } catch (err) {
      if (!frameErrorLogged) {
        frameErrorLogged = true;
        console.error("frame failed; the game keeps running", err);
      }
    } finally {
      requestAnimationFrame(loop);
    }
  }

  function frame(): void {
    const now = performance.now();
    // Clamp long gaps (a hidden tab) so the game doesn't fast-forward.
    accumulator += Math.min(now - lastTime, 250) * prefs.speed;
    lastTime = now;
    let steps = 0;
    // Hit-stop: a big blast freezes the world for a few frames.
    if (fx.takeHitStop()) accumulator = 0;
    // The world waits while the menu or the end screen is open.
    if (!menu.hidden || ended) accumulator = 0;
    while (accumulator >= STEP_MS && steps < MAX_STEPS_PER_FRAME) {
      game.tick(inputBits());
      accumulator -= STEP_MS;
      steps++;
    }
    if (accumulator >= STEP_MS) accumulator = 0;
    if (steps > 0) handleFx(JSON.parse(game.take_fx()) as Fx[]);
    const tSim = performance.now();
    const alpha = accumulator / STEP_MS;
    if (gl?.lost) dropWebGl();
    if (gl) {
      // The GPU draws the lens itself, from the temperature bytes.
      const ptr = game.render(false, alpha);
      gl.draw({
        rgba: bytesAt(ptr, w * h * 4),
        data: bytesAt(game.cell_data(), w * h * 2),
        lens,
        time: now / 1000,
        motion: fx.reducedMotion ? 0 : 1,
        viewX: game.view_x(),
      });
    } else {
      ctx!.putImageData(frameFor(game.render(lens, alpha)), 0, 0);
      fx.setGlow(lens ? new Float32Array(0) : game.emissive());
    }
    fx.setGas(lens ? new Float32Array(0) : game.gas_bubbles());
    const tRender = performance.now();
    const viewX = game.view_x();
    fx.draw(now, viewX);
    const tFx = performance.now();
    if (pointer && steps > 0) game.track_aim(...toGrid(pointer[0], pointer[1]));
    // Goals change every tick; a few polls a second is plenty.
    if (now - lastGoalsTime >= 200) {
      lastGoalsTime = now;
      if (tray) byId("act-tray-text").textContent = prettyFormula(tray.text()) || words("Tap to build");
      const list = JSON.parse(game.goals()) as Goal[];
      goals.setGoals(list);
      trackProgress(list);
      const here = list[game.room()];
      const shownGoal = here && !here.done ? here : goals.next;
      objective.update(shownGoal);
      // One suggested spell per goal, swapped in once (never while the
      // player is editing it).
      if (tray && shownGoal && tray.suggest(shownGoal.room, TRAY_STARTERS[shownGoal.room - 1] ?? "H2O")) {
        actTray.classList.remove("new-spell");
        void actTray.offsetWidth;
        actTray.classList.add("new-spell");
      }
      updateBoomReady(here ?? null);
      if (!room1Cleared && list[0]?.done) room1Cleared = true;
      if (objective.line !== shownGoalLine) renderTutorial();
      shownGoalLine = objective.line;
    }
    framePortrait(viewX);
    layoutTopHud();
    updateSpots(now);
    goals.draw(now, viewX, game.player_x() + 3, fx.reducedMotion);
    updateLift(now, viewX);
    jumpHintTick(now, viewX);
    hintTick(now);
    // JSON is only built when something on the HUD actually changed.
    const version = game.state_version();
    if (state === null || (version !== stateVersion && now - lastHudTime >= 100)) {
      lastHudTime = now;
      stateVersion = version;
      state = JSON.parse(game.state()) as GameState;
      renderHud(state);
    }
    const end = performance.now();
    frameWork.push(end - now);
    if (frameWork.length > 120) frameWork.shift();
    frameParts[0] = frameParts[0]! * 0.95 + (tSim - now) * 0.05;
    frameParts[1] = frameParts[1]! * 0.95 + (tRender - tSim) * 0.05;
    frameParts[2] = frameParts[2]! * 0.95 + (tFx - tRender) * 0.05;
    frameParts[3] = frameParts[3]! * 0.95 + (end - tFx) * 0.05;
  }
  // Every tier: pushing against a wall too tall to step up (more than 2
  // cells) with no progress for 1.5 s brings up a bouncing jump arrow over
  // the player and pulses the jump button.
  const JUMP_HINT_MS = 1500;
  const blocked = { since: -1, x: 0 };
  const jumpHint = document.createElement("div");
  jumpHint.id = "jump-hint";
  jumpHint.setAttribute("aria-hidden", "true");
  jumpHint.hidden = true;
  jumpHint.append(icon("jump"));
  goals.layer.append(jumpHint);
  function jumpHintTick(now: number, viewX: number): void {
    const pushing =
      keys.has("a") || keys.has("arrowleft") || held.has("left") || keys.has("d") || keys.has("arrowright") || held.has("right");
    const jumping = keys.has(" ") || keys.has("w") || keys.has("arrowup") || held.has("jump");
    const x = game.player_x();
    if (!pushing || jumping || Math.abs(x - blocked.x) > 2) {
      blocked.since = pushing && !jumping ? now : -1;
      blocked.x = x;
    }
    const show = blocked.since >= 0 && now - blocked.since >= JUMP_HINT_MS;
    if (jumpHint.hidden === show) {
      jumpHint.hidden = !show;
      document.querySelector('#moves [data-key="jump"]')?.classList.toggle("hinting", show);
    }
    if (show) {
      const kx = stage.clientWidth / w;
      const ky = stage.clientHeight / h;
      jumpHint.style.transform = `translate(${((x + 3 - viewX) * kx).toFixed(1)}px, ${((game.player_y() - 4) * ky).toFixed(1)}px)`;
    }
  }

  // Sprout on an upright phone: the stage is wider than the screen (zoomed
  // in), so it slides sideways to keep the player near the middle.
  const portrait = window.matchMedia("(orientation: portrait) and (max-width: 700px)");
  let portraitLeft = 0;
  function framePortrait(viewX: number): void {
    if (tier !== "sprout" || !portrait.matches) {
      if (portraitLeft !== 0 || stage.style.left) {
        portraitLeft = 0;
        stage.style.left = "";
        stage.style.removeProperty("--off-left");
        stage.style.removeProperty("--off-right");
      }
      return;
    }
    const sw = stage.clientWidth;
    const vw = window.innerWidth;
    const px = (game.player_x() + 3 - viewX) * (sw / w);
    const target = Math.min(0, Math.max(vw - sw, vw / 2 - px));
    portraitLeft += (target - portraitLeft) * (fx.reducedMotion ? 1 : 0.15);
    if (Math.abs(target - portraitLeft) < 0.5) portraitLeft = target;
    stage.style.left = `${portraitLeft.toFixed(1)}px`;
    // How much of the stage is off each side of the screen, for edge arrows.
    stage.style.setProperty("--off-left", `${(-portraitLeft).toFixed(1)}px`);
    stage.style.setProperty("--off-right", `${(sw - vw + portraitLeft).toFixed(1)}px`);
  }

  // Camera framing: the world is exactly as tall as the stage, so when the
  // player drops into a pit behind a bottom HUD bar, the world layers slide
  // up (the strip uncovered at the bottom stays under the HUD).
  const LIFT_MARGIN_PX = 8;
  let hudRects: DOMRect[] = [];
  let hudRectsAt = -Infinity;
  function bottomHudRects(): DOMRect[] {
    const out: DOMRect[] = [];
    const add = (el: Element) => {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) out.push(r);
    };
    for (const id of ["hud-bottom", "tray"]) {
      const el = document.getElementById(id);
      if (el && !el.hidden) add(el);
    }
    for (const id of ["cards", "moves", "actions"]) {
      const el = document.getElementById(id);
      if (el && !el.hidden) for (const c of el.children) add(c);
    }
    return out;
  }
  function updateLift(now: number, viewX: number): void {
    if (now - hudRectsAt > 250) {
      hudRectsAt = now;
      hudRects = bottomHudRects();
    }
    const sr = canvas.getBoundingClientRect();
    // The canvas rect already includes the current lift; undo it.
    const top0 = sr.top + fx.liftPx;
    const kx = sr.width / w;
    const ky = sr.height / h;
    const pl = sr.left + (game.player_x() - viewX) * kx;
    const pr = pl + 6 * kx;
    const pb = top0 + (game.player_y() + 12) * ky + LIFT_MARGIN_PX;
    let need = 0;
    for (const r of hudRects) {
      if (r.right < pl - 8 || r.left > pr + 8) continue;
      if (r.top > top0 + sr.height || r.bottom < top0) continue;
      need = Math.max(need, pb - r.top);
    }
    const target = Math.min(Math.max(0, need), sr.height * 0.3);
    const lift = fx.liftPx + (target - fx.liftPx) * (target > fx.liftPx ? 0.35 : 0.12);
    fx.liftPx = Math.abs(lift - target) < 0.5 ? target : lift;
  }

  // ------------------------------------------------- one top message channel

  const hudTop = byId("hud-top");
  const moteEl = byId("mote");
  const jarsEl = byId("jars-mini");
  /**
   * The top column shows one thing at a time: while a banner (room clear,
   * the room's goal, the room score) is up, the pill and the tip step
   * aside. The column also drops below Mote's caption when a long caption
   * wraps into its way.
   */
  function layoutTopHud(): void {
    const bannerUp = popups.has("room-clear", "score");
    objective.setOthersUp(bannerUp);
    const anyBanner = bannerUp || objective.bannerUp;
    tutorial.classList.toggle("yield", anyBanner);
    let push = 0;
    const c = hudTop.getBoundingClientRect();
    const baseTop = c.top - (parseFloat(hudTop.style.marginTop) || 0);
    // Things in the top corners the column must not sit on: Mote's caption
    // (when showing) and the suit gauges.
    for (const el of [moteEl.classList.contains("show") ? moteEl : null, jarsEl]) {
      if (!el || el.hidden || getComputedStyle(el).position !== "absolute") continue;
      const m = el.getBoundingClientRect();
      if (m.width === 0) continue;
      // Overlaps where the column would be without a push?
      const across = m.left < c.right && c.left < m.right && m.top < baseTop + c.height && m.bottom > baseTop;
      if (across) push = Math.max(push, Math.ceil(m.bottom + 8 - baseTop));
    }
    const want = `${push}px`;
    if (hudTop.style.marginTop !== want) hudTop.style.marginTop = want;
  }

  /** How long each of the last frames took to compute (for automated checks). */
  const frameWork: number[] = [];
  /** Smoothed ms per frame for sim steps, world render, overlay, and HUD. */
  const frameParts = [0, 0, 0, 0];

  // Automated browser tests read the HUD state through this hook. It exposes
  // nothing that isn't already on screen and only exists with ?debug.
  if (new URLSearchParams(window.location.search).has("debug")) {
    (window as unknown as { realgame: unknown }).realgame = {
      state: () => state,
      viewX: () => game.view_x(),
      playerX: () => game.player_x(),
      playerY: () => game.player_y(),
      goals: () => JSON.parse(game.goals()) as Goal[],
      shake: () => ({ max: fx.maxShakePx, last: fx.lastShakePx }),
      cast: (f: string) => requestCast(f),
      hintLevel: () => hint.level,
      trayText: () => tray?.text() ?? null,
      lastOutcome: () => lastResult,
      frameMs: () => frameWork.reduce((a, b) => a + b, 0) / Math.max(1, frameWork.length),
      emissive: () => Array.from(game.emissive()),
      /** Heat-lens label spots: flat [x, y, °C] in viewport cells. */
      hotspots: () => Array.from(game.hotspots()),
      frameParts: () => [...frameParts],
      renderer: () => (gl ? "webgl" : "2d"),
      fxStats: () => fx.stats(),
      maxWavePx: () => fx.maxWavePx,
      /** The last impact point in page (client) pixels. */
      impactPx: () => {
        const p = fx.lastImpact;
        if (!p) return null;
        const r = canvas.getBoundingClientRect();
        return [r.left + ((p[0] - game.view_x()) / w) * r.width, r.top + (p[1] / h) * r.height];
      },
      room: () => game.room(),
      /** Opens every gate (as free play does), to reach later rooms quickly. */
      openGates: () => game.free_play(),
      aim: () => Array.from(game.aim()),
      /** Fills the hotbar with typed spells, as a long session would. */
      setSlots: (list: string[]) => {
        slots.splice(STARTER_SLOTS.length, slots.length, ...list.slice(0, MAX_SLOTS - STARTER_SLOTS.length));
        renderSlots();
      },
    };
  }

  renderSlots();
  requestAnimationFrame(loop);
}

boot().catch((err: unknown) => {
  const msg = document.createElement("p");
  msg.className = "error";
  msg.textContent = `The game could not start: ${String(err)}`;
  document.body.append(msg);
});
