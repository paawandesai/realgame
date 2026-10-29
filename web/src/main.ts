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
const TRAY_STARTERS = ["H2O", "H2 + O2 -> H2O", "H2O -> H2 + O2", "H2O"];

/** Spells that work from the very first second (the suit starts charged). */
const STARTER_SLOTS = BAR_SPELLS.map((b) => b.formula);

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

/** Three picture buttons; with no choice in 8 s the game starts as Sprout. */
function showStartScreen(onChoose: (tier: Tier) => void): void {
  const screen = byId("start");
  screen.hidden = false;
  let chosen = false;
  const choose = (tier: Tier, remember: boolean) => {
    if (chosen) return;
    chosen = true;
    window.clearInterval(timer);
    if (remember) storageSet("tier", tier);
    onChoose(tier);
  };
  for (const b of document.querySelectorAll<HTMLButtonElement>("[data-tier]")) {
    const tier = b.dataset.tier;
    if (!isTier(tier)) continue;
    const pic = b.querySelector(".age-pic");
    if (pic) setIcon(pic, pic.getAttribute("data-icon") as IconName);
    const name = REALM_NAMES[tier];
    b.addEventListener("focus", () => speakNow(name));
    b.addEventListener("pointerenter", () => speakNow(name));
    b.addEventListener("click", () => {
      speakNow(name);
      choose(tier, true);
    });
  }
  const began = performance.now();
  const fill = byId("autostart-fill");
  const timer = window.setInterval(() => {
    const t = (performance.now() - began) / AUTOSTART_MS;
    fill.style.width = `${Math.min(100, t * 100)}%`;
    // Not remembered: the next visit asks again.
    if (t >= 1) choose("sprout", false);
  }, 100);
}

function start(tier: Tier, memory: WebAssembly.Memory): void {
  byId("start").hidden = true;
  byId("game").hidden = false;
  document.body.dataset.tier = tier;
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
  const objective = new ObjectiveBanner(stage, tier);
  /** The Grimoire card, the room-clear stamp and the room score take turns. */
  const popups = new PopupQueue();
  // The cast-result bubble steers around whichever popup is up.
  fx.avoid = () => [
    popups.active,
    document.querySelector<HTMLElement>("#mote.show"),
    document.querySelector<HTMLElement>("#objective-banner:not(.shrink)"),
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
    tray.set("H2O");
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
      ["Cast", ["Enter"]],
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
      if (!menu.hidden) closeMenu();
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
  function renderTutorial(): void {
    const t = byId("app-tutorial-text");
    if (room1Cleared && objective.line) {
      t.textContent = `Goal: ${objective.line}`;
      return;
    }
    const code = document.createElement("code");
    code.textContent = "water";
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
    popups.push({ kind: "score", el: challengeEl, ms: SCORE_MS });
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
    if (tier !== "apprentice") speech.say(result.label, outcomeSpeech(result));
    hintOnCast(result);
    scoreCast(result);
    afterCastNudges(result);
    lastCastRoom = goals.next?.room ?? -1;
    if (tray) {
      setTrayOpen(false);
      if (result.outcome === "partial" && result.suggestion) tray.offerFix(result.suggestion);
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
    let line = r.label;
    if (r.outcome === "unknown" && tier === "sprout") line = "Hmm, that one is not a spell yet.";
    if (r.outcome === "fizzle" && tier === "sprout") line = `${r.label} Wait a moment for your suit to fill up.`;
    return speakable(line);
  }

  // ------------------------------------------------------------ hint ladder

  const hintGhost = byId("hint-ghost");
  const hintText = byId("hint-text");
  const hintUse = byId<HTMLButtonElement>("hint-use");
  const ghostHand = byId("ghost-hand");
  setIcon(ghostHand, "hand");
  const hint = { floor: 0, misses: 0, since: performance.now(), key: "", level: 0, nudged: false };
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
    if (key !== hint.key) {
      hint.key = key;
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
    if (level !== hint.level || nudged !== hint.nudged) applyHint(level, next);
    placeGhostHand(now);
  }

  function applyHint(level: number, next: Goal | null): void {
    hint.level = level;
    hint.nudged = nudgeFor !== null;
    document.body.dataset.hint = String(level);
    goals.setHintLevel(level);
    const need: Goal["need"] | null = next ? (nudgeFor !== null ? "boom" : next.need) : null;
    const spell = need ? SPELLS[tier][need] : "";
    hintSpell = spell;
    const glow = level >= 2 && !goals.goVisible && need !== null;
    cards?.setHint(glow ? (need as CardSpell) : null);
    tray?.setHint(glow ? spell : null);
    const slot = glow ? spell : "";
    if (slot !== slotHint) {
      slotHint = slot;
      renderSlots();
    }
    const showGhost = level >= 3 && next !== null;
    if (showGhost && hintGhost.hidden) {
      if (goals.goVisible) {
        hintText.textContent = "Walk this way";
        hintUse.hidden = true;
        speech.say("Walk this way! Tap the arrow.");
      } else if (tier === "sprout") {
        const card = CARDS.find((c) => c.spell === need);
        hintText.replaceChildren(icon(card?.picture ?? "drop"));
        hintUse.hidden = true;
        speech.say(`Try the ${card?.name ?? "water drop"}!`);
      } else {
        hintText.textContent = `Try: ${prettyFormula(spell)}`;
        hintUse.hidden = false;
        speech.say(`Try: ${prettyFormula(spell)}`, speakable(`Try ${spell}`));
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
      target = goals.goVisible ? document.getElementById("go-arrow") : cards.element(need as CardSpell);
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
  const METHANE_HINT = () => `Gather methane from the marsh bubbles in room 4: ${isTouch() ? "hold the scoop" : "hold G"}.`;
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
      b.classList.toggle("suggested", (i === 0 && castCount === 0) || (slotHint !== "" && f === slotHint));
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
      // A new room: last room's score and stamp go, and the Seedling tray
      // starts over with this room's starter spell.
      if (hudRoom >= 0) popups.drop("score", "room-clear");
      hudRoom = s.room;
      tray?.set(TRAY_STARTERS[s.room] ?? "H2O");
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
      from: () => "Your wrist electrolyser splits water from the tank: 2 H₂O → 2 H₂ + O₂. Slow on solar power, fast beside a crystal.",
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
    if (tier === "apprentice") {
      if (!chargeEl) {
        chargeEl = document.createElement("div");
        chargeEl.className = "jar charge";
        chargeEl.title = "Crystal battery charge";
      }
      chargeEl.textContent = `⚡ ${Math.round((s.crystal_kj / s.crystal_max_kj) * 100)}%`;
      shown.push(chargeEl);
    }
    setChildren(jars, shown);
    if (shownJars) {
      for (const [formula, n] of Object.entries(s.jars)) {
        const d = n - (shownJars[formula] ?? 0);
        if (d !== 0) gaugeTick(formula, d);
      }
    }
    shownJars = { ...s.jars };
  }

  /** A gate opened: a big star stamp so it reads as done. One element,
   * reused for every room. */
  const roomClearEl = document.createElement("div");
  roomClearEl.id = "room-clear";
  roomClearEl.setAttribute("role", "status");
  roomClearEl.hidden = true;
  const roomClearText = document.createElement("span");
  roomClearEl.append(icon("star"), roomClearText);
  stage.append(roomClearEl);
  function showRoomClear(room: number): void {
    if (room >= 3) return; // The last door has its own end screen.
    objective.holdFor(ROOM_CLEAR_MS);
    roomClearText.textContent = tier === "sprout" ? "Yay! Gate open!" : `Room ${room + 1} clear! Gate open`;
    popups.push({ kind: "room-clear", el: roomClearEl, ms: ROOM_CLEAR_MS });
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
  const openMenu = () => {
    menu.hidden = false;
    gear.setAttribute("aria-expanded", "true");
    byId("menu-age").focus();
  };
  const closeMenu = () => {
    menu.hidden = true;
    gear.setAttribute("aria-expanded", "false");
    gear.focus();
  };
  gear.addEventListener("click", () => (menu.hidden ? openMenu() : closeMenu()));
  byId("settings-close").addEventListener("click", closeMenu);
  // Keep Tab inside the open menu.
  menu.addEventListener("keydown", (e) => {
    if (e.key !== "Tab") return;
    const items = [...menu.querySelectorAll<HTMLElement>("button, input, select")].filter((el) => !el.hidden);
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
    const room = game.restart_room();
    resetHints();
    autoWalk = null;
    tray?.set(TRAY_STARTERS[room] ?? "H2O");
    closeMenu();
    speech.say(tier === "sprout" ? "Let's try again!" : "Room restarted.");
  });
  byId("menu-trial").addEventListener("click", () => {
    restartTrial();
    closeMenu();
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
    tray?.set("H2O");
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

  function loop(): void {
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
      objective.update(here && !here.done ? here : goals.next);
      if (!room1Cleared && list[0]?.done) {
        room1Cleared = true;
        renderTutorial();
      } else if (room1Cleared && objective.line !== shownGoalLine) renderTutorial();
      shownGoalLine = objective.line;
    }
    goals.draw(now, viewX, game.player_x() + 3, fx.reducedMotion);
    updateLift(now, viewX);
    if (jumpHint) jumpHintTick(now, viewX);
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
    requestAnimationFrame(loop);
  }
  // Sprout: pushing against a ledge or wall with no progress brings up a
  // bouncing jump arrow over the player and pulses the jump button.
  const JUMP_HINT_MS = 1500;
  let jumpHint: HTMLElement | null = null;
  const blocked = { since: -1, x: 0 };
  if (tier === "sprout") {
    jumpHint = document.createElement("div");
    jumpHint.id = "jump-hint";
    jumpHint.setAttribute("aria-hidden", "true");
    jumpHint.hidden = true;
    jumpHint.append(icon("jump"));
    goals.layer.append(jumpHint);
  }
  function jumpHintTick(now: number, viewX: number): void {
    if (!jumpHint) return;
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
