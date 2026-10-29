// Browser shell: start screen, input, the game loop and the HUD. All physics
// and chemistry run in the Rust simulation (crates/sim), compiled to
// WebAssembly.

import init, { WebGame } from "./pkg/realgame_sim.js";
import { Sfx } from "./audio";
import { CARDS, SproutCards, type CardSpell } from "./cards";
import { FxLayer } from "./fx";
import { GoalMarkers } from "./goals";
import { equationBalls, icon, setIcon, type IconName } from "./icons";
import { loadPrefs, savePrefs, savedTier, storageRemove, storageSet, isTier } from "./prefs";
import { FormulaRing, byId } from "./ring";
import { Speech, speakable } from "./speech";
import { SpellTray } from "./tray";
import { prettyFormula, type CastOutcome, type Fx, type GameState, type Goal, type Tier } from "./types";

const INPUT = { left: 1, right: 2, jump: 4, gather: 8 } as const;
const MAX_SLOTS = 5;
/** The simulation always advances in fixed 60 Hz steps, whatever the display rate. */
const STEP_MS = 1000 / 60;
/** At most this many steps per frame; anything beyond is dropped so a slow
 * device slows the game down instead of spiralling. */
const MAX_STEPS_PER_FRAME = 4;
/** With no choice on the start screen, the youngest tier starts by itself. */
const AUTOSTART_MS = 8000;
/** Hint ladder: each step after this long without progress... */
const HINT_IDLE_MS = 8000;
/** ...or straight to the full hint after this many misses in a row. */
const HINT_MISSES = 3;
const END_DELAY_MS = 2500;

/** The spell each goal needs, written for each tier. */
const SPELLS: Record<Tier, Record<Goal["need"], string>> = {
  sprout: { water: "water", boom: "boom", split: "split" },
  seedling: { water: "H2O", boom: "2H2 + O2 -> 2H2O", split: "2H2O -> 2H2 + O2" },
  apprentice: { water: "H2O", boom: "2H2 + O2 -> 2H2O", split: "2H2O -> 2H2 + O2" },
};

/** Spells that work from the very first second (the suit starts charged). */
const STARTER_SLOTS = ["H2O", "2H2 + O2 -> 2H2O", "2H2O -> 2H2 + O2"];

const REALM_NAMES: Record<Tier, string> = {
  sprout: "Sprout",
  seedling: "Seedling",
  apprentice: "Apprentice",
};

/** The Grimoire's pages, in the order the trial teaches them. */
const GRIMOIRE: { id: string; name: string; picture: IconName; equation: string }[] = [
  { id: "water_synthesis", name: "Boom", picture: "boom", equation: "2H2 + O2 -> 2H2O" },
  { id: "electrolysis_water", name: "Bubbles", picture: "bubbles", equation: "2H2O -> 2H2 + O2" },
  { id: "methane_combustion", name: "Fireball", picture: "fire", equation: "CH4 + 2O2 -> CO2 + 2H2O" },
  { id: "carbon_combustion", name: "Burning wood", picture: "fire", equation: "C + O2 -> CO2" },
];

interface NotebookEntry {
  id: string;
  equation: string;
  concept: string;
  realWorld: string;
  deltaH: number | null;
  source: string | null;
}

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
  /** Recent casts by id, so FX events can find their outcome and label. */
  const outcomes = new Map<number, CastOutcome>();
  const notebook = new Map<string, NotebookEntry>();
  let state: GameState | null = null;
  let ended = false;
  let freePlay = false;
  let endTimer = 0;
  let seedlingRoom2Filled = false;

  const replayBtn = byId<HTMLButtonElement>("replay");
  setIcon(replayBtn, "speaker");
  const speech = new Speech(prefs.voice, byId("mote"), byId("mote-text"), replayBtn);
  const sfx = new Sfx(tier);
  // Browsers only allow audio and speech after a gesture.
  const unlock = () => {
    sfx.unlock();
    speech.unlock();
  };
  window.addEventListener("pointerdown", unlock, { capture: true });
  window.addEventListener("keydown", unlock, { capture: true });

  const ring = new FormulaRing(tier, (formula) => requestCast(formula));
  const fx = new FxLayer(stage, [canvas], w, h, tier, (fix) => {
    if (tray) tray.set(fix);
    else {
      ring.set(fix);
      ring.toggle(true);
    }
  });
  fx.reducedOverride = prefs.reducedMotion;
  document.body.dataset.motion = prefs.reducedMotion ? "calm" : "full";

  const goals = new GoalMarkers(stage, byId("progress"), w, h, () => {
    const next = goals.next;
    if (next) autoWalk = { target: next.stand_x, stuck: 0, lastX: game.player_x() };
  });

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

  /** Buttons, inputs and selects handle Enter and Space themselves. */
  const isControl = (t: EventTarget | null): boolean =>
    t instanceof HTMLElement && t.closest("button, input, select, textarea, [contenteditable]") !== null;

  window.addEventListener("keydown", (e) => {
    const k = e.key.toLowerCase();
    if (k === "escape") {
      if (!menu.hidden) closeMenu();
      else ring.toggle(false);
      return;
    }
    if (!menu.hidden || !byId("end").hidden) return;
    if ((k === "enter" || k === " ") && isControl(e.target)) return;
    if (k === "l") setLens(!lens);
    else if (k === "q" && tier !== "sprout") ring.toggle();
    else if (k === "n") toggleNotebook();
    else if (k === "enter") {
      e.preventDefault();
      if (!e.repeat) requestCast();
    } else if (/^[1-5]$/.test(k) && !e.repeat) {
      const spell = tier === "sprout" ? CARDS[Number(k) - 1]?.spell : slots[Number(k) - 1];
      if (spell && (tier !== "sprout" || cards?.element(spell as CardSpell))) requestCast(spell);
    } else if (k === " ") e.preventDefault();
    if (["a", "d", "arrowleft", "arrowright", " ", "w", "arrowup"].includes(k)) autoWalk = null;
    keys.add(k);
  });
  window.addEventListener("keyup", (e) => keys.delete(e.key.toLowerCase()));
  window.addEventListener("blur", () => keys.clear());

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
      if (key === "ring") ring.toggle();
      else held.add(key);
    });
    const release = () => held.delete(key);
    b.addEventListener("pointerup", release);
    b.addEventListener("pointerleave", release);
    b.addEventListener("pointercancel", release);
    // Keyboard users: Enter on a move button takes a short step.
    b.addEventListener("click", (e) => {
      if (e.detail !== 0 || key === "ring") return;
      held.add(key);
      window.setTimeout(() => held.delete(key), 250);
    });
  }

  byId("lens-btn").addEventListener("click", () => setLens(!lens));
  byId("notebook-btn").addEventListener("click", toggleNotebook);
  byId("notebook-close").addEventListener("click", toggleNotebook);

  function setLens(on: boolean): void {
    lens = on;
    byId("lens-btn").setAttribute("aria-pressed", String(on));
  }

  function toggleNotebook(): void {
    const nb = byId("notebook");
    nb.hidden = !nb.hidden;
  }

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

  function doCast(formula: string): void {
    const result = JSON.parse(game.cast(formula)) as CastOutcome;
    outcomes.set(result.id, result);
    if (outcomes.size > 32) outcomes.delete(outcomes.keys().next().value as number);
    // Events from the cast itself (charge, bolt, fizzle) show at once.
    const events = JSON.parse(game.take_fx()) as Fx[];
    sfx.handle(events, outcomes);
    fx.handle(events, outcomes, performance.now());
    castCount++;
    speech.say(result.label, outcomeSpeech(result));
    hintOnCast(result);
    if (tray) {
      if (result.outcome === "partial" && result.suggestion) tray.offerFix(result.suggestion);
      document.body.classList.toggle("cast-full", result.outcome === "full");
      window.setTimeout(() => document.body.classList.remove("cast-full"), 1200);
    }
    if (result.outcome === "full" || result.outcome === "partial") {
      lastCast = formula;
      if (tier === "apprentice" && !slots.includes(formula)) {
        slots.unshift(formula);
        slots.length = Math.min(slots.length, MAX_SLOTS);
      }
      if (result.reaction_id && result.concept && !notebook.has(result.reaction_id)) {
        notebook.set(result.reaction_id, {
          id: result.reaction_id,
          equation: result.equation,
          concept: result.concept,
          realWorld: result.real_world ?? "",
          deltaH: result.delta_h_kj,
          source: result.source,
        });
        renderNotebook();
      }
    }
    renderSlots();
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
  const hint = { floor: 0, misses: 0, since: performance.now(), key: "", level: 0 };
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

  function resetHints(): void {
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
    // Time spent in the menu or on the end screen doesn't count.
    if (!menu.hidden || ended) hint.since = now - (hint.level - hint.floor) * HINT_IDLE_MS;
    const idle = Math.floor((now - hint.since) / HINT_IDLE_MS);
    const level = next ? Math.min(3, Math.max(hint.floor, idle)) : 0;
    if (level !== hint.level) applyHint(level, next);
    placeGhostHand(now);
  }

  function applyHint(level: number, next: Goal | null): void {
    hint.level = level;
    document.body.dataset.hint = String(level);
    goals.setHintLevel(level);
    const spell = next ? SPELLS[tier][next.need] : "";
    hintSpell = spell;
    cards?.setHint(level >= 2 && !goals.goVisible && next ? (next.need as CardSpell) : null);
    tray?.setHint(level >= 2 && !goals.goVisible ? spell : null);
    const showGhost = level >= 3 && next !== null;
    if (showGhost && hintGhost.hidden) {
      if (goals.goVisible) {
        hintText.textContent = "Walk this way";
        hintUse.hidden = true;
        speech.say("Walk this way! Tap the arrow.");
      } else if (tier === "sprout") {
        const card = CARDS.find((c) => c.spell === next.need);
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
    const show = next !== null && !ended && (onboarding || hint.level >= 3);
    let target: Element | null = null;
    if (show) {
      target = goals.goVisible ? document.getElementById("go-arrow") : cards.element(next.need as CardSpell);
    }
    ghostHand.hidden = !target;
    if (!target) return;
    const r = target.getBoundingClientRect();
    const s = stage.getBoundingClientRect();
    const tap = fx.reducedMotion ? 0 : Math.max(0, Math.sin(now / 250)) * 8;
    ghostHand.style.left = `${r.left - s.left + r.width / 2}px`;
    ghostHand.style.top = `${r.top - s.top + r.height / 2 + tap}px`;
  }

  // -------------------------------------------------------------------- HUD

  function renderSlots(): void {
    if (tier !== "apprentice") return;
    const el = byId("slots");
    el.replaceChildren();
    for (let i = 0; i < MAX_SLOTS; i++) {
      const f = slots[i];
      const b = document.createElement("button");
      b.type = "button";
      b.className = i === 0 && castCount === 0 ? "slot suggested" : "slot";
      b.disabled = !f;
      b.title = f ? prettyFormula(f) : "Empty slot";
      const n = document.createElement("kbd");
      n.textContent = String(i + 1);
      const label = document.createElement("span");
      label.textContent = f ? prettyFormula(f) : "";
      b.append(n, label);
      if (f) b.addEventListener("click", () => requestCast(f));
      el.append(b);
    }
  }

  function renderNotebook(): void {
    const list = byId("notebook-list");
    list.replaceChildren();
    byId("notebook-empty").hidden = notebook.size > 0;
    for (const e of notebook.values()) {
      const li = document.createElement("li");
      const eq = document.createElement("strong");
      eq.textContent = prettyFormula(e.equation);
      const c = document.createElement("p");
      c.textContent = e.concept;
      const rw = document.createElement("p");
      rw.className = "small";
      rw.textContent = e.realWorld;
      li.append(eq, c, rw);
      if (e.deltaH !== null) {
        const dh = document.createElement("p");
        dh.className = "small";
        dh.textContent = `ΔH = ${e.deltaH.toFixed(1)} kJ · ${e.source ?? ""}`;
        li.append(dh);
      }
      list.append(li);
    }
  }

  const describeTemp = (t: number): string => {
    if (tier === "apprentice") return `${Math.round(t)} °C`;
    if (t < 0) return "freezing";
    if (t < 35) return "cool";
    if (t < 100) return "warm";
    if (t < 500) return "hot";
    return "very hot";
  };

  let lastHudKey = "";
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
    if (tray && s.room === 1 && !seedlingRoom2Filled) {
      seedlingRoom2Filled = true;
      tray.set("H2 + O2 -> H2O");
    }
    if (s.trial_complete && !ended && !freePlay && endTimer === 0) {
      endTimer = window.setTimeout(showEnd, END_DELAY_MS);
    }

    const key = JSON.stringify([s.jars, Math.round((s.crystal_kj / s.crystal_max_kj) * 20)]);
    if (key === lastHudKey) return;
    lastHudKey = key;
    const jars = byId(tier === "apprentice" ? "jars" : "jars-mini");
    jars.replaceChildren();
    for (const [formula, n] of Object.entries(s.jars)) {
      if (tier !== "apprentice" && n === 0 && (formula === "CH4" || formula === "CO2")) continue;
      const jar = document.createElement("div");
      jar.className = "jar";
      jar.title = `${formula}: ${n}`;
      const fill = document.createElement("div");
      fill.className = `fill f-${formula.toLowerCase()}`;
      fill.style.height = `${Math.min(100, (n / (s.jar_caps[formula] ?? 10)) * 100)}%`;
      const glass = document.createElement("div");
      glass.className = "glass";
      glass.append(fill);
      const label = document.createElement("span");
      label.textContent = `${prettyFormula(formula)} ${n}`;
      jar.append(glass, label);
      jars.append(jar);
    }
    if (tier === "apprentice") {
      const charge = document.createElement("div");
      charge.className = "jar charge";
      charge.textContent = `⚡ ${Math.round((s.crystal_kj / s.crystal_max_kj) * 100)}%`;
      charge.title = "Crystal battery charge";
      jars.append(charge);
    }
  }

  // ------------------------------------------------------------- end screen

  function showEnd(): void {
    endTimer = 0;
    if (freePlay) return;
    ended = true;
    const list = byId("grimoire");
    list.replaceChildren();
    const found = new Set(state?.discovered ?? []);
    for (const g of GRIMOIRE) {
      const li = document.createElement("li");
      const has = found.has(g.id);
      li.className = has ? "found" : "silhouette";
      li.append(icon(g.picture));
      const name = document.createElement("strong");
      name.textContent = has ? g.name : "?";
      li.append(name);
      if (has) {
        const eq = document.createElement("span");
        eq.className = "grim-eq";
        if (tier === "sprout") eq.append(equationBalls(g.equation));
        else eq.textContent = prettyFormula(g.equation);
        li.append(eq);
      }
      list.append(li);
    }
    byId("end-sub").textContent =
      tier === "sprout" ? "You did it!" : `You passed the Alchemist's Trial: ${found.size} of ${GRIMOIRE.length} Grimoire pages found.`;
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
    if (tray) tray.set(room === 1 ? "H2 + O2 -> H2O" : "H2O");
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
    castCount = 0;
    lastCast = "";
    outcomes.clear();
    notebook.clear();
    renderNotebook();
    slots.splice(0, slots.length, ...STARTER_SLOTS);
    renderSlots();
    seedlingRoom2Filled = false;
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

  /** Presentation events from the simulation: particles, flashes, labels. */
  function handleFx(events: Fx[]): void {
    if (events.length === 0) return;
    sfx.handle(events, outcomes);
    fx.handle(events, outcomes, performance.now());
  }

  let accumulator = 0;
  let lastTime = performance.now();
  let lastHudTime = -Infinity;
  let lastGoalsTime = -Infinity;
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
    const alpha = accumulator / STEP_MS;
    ctx!.putImageData(frameFor(game.render(lens, alpha)), 0, 0);
    const viewX = game.view_x();
    fx.draw(now, viewX);
    if (pointer && steps > 0) game.track_aim(...toGrid(pointer[0], pointer[1]));
    // Goals change every tick; a few polls a second is plenty.
    if (now - lastGoalsTime >= 200) {
      lastGoalsTime = now;
      goals.setGoals(JSON.parse(game.goals()) as Goal[]);
    }
    goals.draw(now, viewX, game.player_x() + 3, fx.reducedMotion);
    hintTick(now);
    // JSON is only built when something on the HUD actually changed.
    const version = game.state_version();
    if (state === null || (version !== stateVersion && now - lastHudTime >= 100)) {
      lastHudTime = now;
      stateVersion = version;
      state = JSON.parse(game.state()) as GameState;
      renderHud(state);
    }
    requestAnimationFrame(loop);
  }

  // Automated browser tests read the HUD state through this hook. It exposes
  // nothing that isn't already on screen and only exists with ?debug.
  if (new URLSearchParams(window.location.search).has("debug")) {
    (window as unknown as { realgame: unknown }).realgame = {
      state: () => state,
      viewX: () => game.view_x(),
      playerX: () => game.player_x(),
      goals: () => JSON.parse(game.goals()) as Goal[],
      shake: () => ({ max: fx.maxShakePx, last: fx.lastShakePx }),
      cast: (f: string) => requestCast(f),
      hintLevel: () => hint.level,
      trayText: () => tray?.text() ?? null,
    };
  }

  renderSlots();
  renderNotebook();
  requestAnimationFrame(loop);
}

boot().catch((err: unknown) => {
  const msg = document.createElement("p");
  msg.className = "error";
  msg.textContent = `The game could not start: ${String(err)}`;
  document.body.append(msg);
});
