// Browser shell: input, the game loop and the HUD. All physics and chemistry
// run in the Rust simulation (crates/sim), compiled to WebAssembly.

import init, { WebGame } from "./pkg/realgame_sim.js";
import { trackSessionStart } from "./analytics";
import { FormulaRing, byId } from "./ring";
import { prettyFormula, type CastResult, type GameState, type Tier } from "./types";

const INPUT = { left: 1, right: 2, jump: 4, gather: 8 } as const;
const SLOW_MOTION_EVERY = 5; // While the ring is open, time runs at 1/5 speed.
const MAX_SLOTS = 5;

const REALM_NAMES: Record<Tier, string> = {
  sprout: "Sprout",
  seedling: "Seedling",
  apprentice: "Apprentice",
};

interface NotebookEntry {
  id: string;
  equation: string;
  concept: string;
  realWorld: string;
  deltaH: number | null;
  source: string | null;
}

function storageGet(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function storageSet(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Private windows and locked-down school profiles may block storage; the game works without it.
  }
}

async function boot(): Promise<void> {
  const wasm = await init();
  const saved = storageGet("tier") as Tier | null;
  if (saved && saved in REALM_NAMES) {
    start(saved, wasm.memory);
    return;
  }
  for (const b of document.querySelectorAll<HTMLButtonElement>("[data-tier]")) {
    b.addEventListener("click", () => {
      const tier = b.dataset.tier as Tier;
      storageSet("tier", tier);
      start(tier, wasm.memory);
    });
  }
}

function start(tier: Tier, memory: WebAssembly.Memory): void {
  trackSessionStart(tier);
  byId("start").hidden = true;
  byId("game").hidden = false;
  document.body.dataset.tier = tier;

  const game = new WebGame(tier, (Math.random() * 1e9) >>> 0);
  const w = game.width();
  const h = game.height();
  const canvas = byId<HTMLCanvasElement>("world");
  const ctx = canvas.getContext("2d", { alpha: false });
  if (!ctx) throw new Error("Canvas 2D is not available");
  const image = ctx.createImageData(w, h);

  const keys = new Set<string>();
  let gatherHeld = false;
  let lens = false;
  let frame = 0;
  let lastCast = "";
  const slots: string[] = [];
  const notebook = new Map<string, NotebookEntry>();
  let state: GameState | null = null;

  const ring = new FormulaRing(tier, (formula) => doCast(formula));

  // ------------------------------------------------------------------ input

  const toGrid = (clientX: number, clientY: number): [number, number] => {
    const r = canvas.getBoundingClientRect();
    return [Math.floor(((clientX - r.left) / r.width) * w), Math.floor(((clientY - r.top) / r.height) * h)];
  };

  canvas.addEventListener("pointermove", (e) => game.set_aim(...toGrid(e.clientX, e.clientY)));
  canvas.addEventListener("pointerdown", (e) => {
    game.set_aim(...toGrid(e.clientX, e.clientY));
    if (e.button === 2) {
      gatherHeld = true;
    } else if (e.button === 0) {
      const f = ring.isOpen ? ring.text() : lastCast;
      if (f) doCast(f);
      else ring.toggle(true);
    }
  });
  window.addEventListener("pointerup", (e) => {
    if (e.button === 2) gatherHeld = false;
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());

  window.addEventListener("keydown", (e) => {
    const k = e.key.toLowerCase();
    if (k === "tab") {
      e.preventDefault();
      setLens(true);
      return;
    }
    if (k === "q") ring.toggle();
    else if (k === "escape") ring.toggle(false);
    else if (k === "n") toggleNotebook();
    else if (k === "enter" && ring.isOpen) ring.cast();
    else if (/^[1-5]$/.test(k)) {
      const f = slots[Number(k) - 1];
      if (f) doCast(f);
    } else if (k === " ") e.preventDefault();
    keys.add(k);
  });
  window.addEventListener("keyup", (e) => {
    const k = e.key.toLowerCase();
    if (k === "tab") setLens(false);
    keys.delete(k);
  });
  window.addEventListener("blur", () => {
    keys.clear();
    gatherHeld = false;
  });

  // On-screen buttons for tablets.
  const touchHeld = new Set<string>();
  for (const b of document.querySelectorAll<HTMLButtonElement>("#touch [data-key]")) {
    const key = b.dataset.key ?? "";
    b.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      if (key === "ring") ring.toggle();
      else touchHeld.add(key);
    });
    const release = () => touchHeld.delete(key);
    b.addEventListener("pointerup", release);
    b.addEventListener("pointerleave", release);
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

  function inputBits(): number {
    let bits = 0;
    if (keys.has("a") || keys.has("arrowleft") || touchHeld.has("left")) bits |= INPUT.left;
    if (keys.has("d") || keys.has("arrowright") || touchHeld.has("right")) bits |= INPUT.right;
    if (keys.has(" ") || keys.has("w") || keys.has("arrowup") || touchHeld.has("jump")) bits |= INPUT.jump;
    if (gatherHeld || keys.has("g") || touchHeld.has("gather")) bits |= INPUT.gather;
    return bits;
  }

  // ---------------------------------------------------------------- casting

  function doCast(formula: string): void {
    const result = JSON.parse(game.cast(formula)) as CastResult;
    showToast(result);
    if (result.ok) {
      lastCast = formula;
      if (!slots.includes(formula)) {
        slots.unshift(formula);
        slots.length = Math.min(slots.length, MAX_SLOTS);
        renderSlots();
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
  }

  let toastTimer = 0;
  function showToast(r: CastResult): void {
    const toast = byId("toast");
    toast.replaceChildren();
    toast.className = r.ok ? "good" : r.kind === "fizzle" ? "fizzle" : "bad";
    if (r.equation) {
      const eq = document.createElement("div");
      eq.className = "toast-eq";
      eq.textContent = prettyFormula(r.equation);
      toast.append(eq);
    }
    const msg = document.createElement("div");
    msg.textContent = r.message;
    toast.append(msg);
    if (r.ok && r.delta_h_kj !== null) {
      const dh = document.createElement("div");
      dh.className = "small";
      const sign = r.delta_h_kj < 0 ? "released" : "absorbed";
      dh.textContent = `ΔH = ${r.delta_h_kj.toFixed(1)} kJ (heat ${sign}). Source: ${r.source ?? ""}`;
      toast.append(dh);
    }
    if (r.first_discovery) {
      const nb = document.createElement("div");
      nb.className = "small";
      nb.textContent = "New in your lab notebook!";
      toast.append(nb);
    }
    if (r.suggestion) {
      const fix = document.createElement("button");
      fix.type = "button";
      fix.textContent = `Fix it: ${prettyFormula(r.suggestion)}`;
      fix.addEventListener("click", () => {
        ring.set(r.suggestion ?? "");
        ring.toggle(true);
      });
      toast.append(fix);
    }
    toast.hidden = false;
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => (toast.hidden = true), r.suggestion ? 9000 : 5000);
  }

  // -------------------------------------------------------------------- HUD

  function renderSlots(): void {
    const el = byId("slots");
    el.replaceChildren();
    for (let i = 0; i < MAX_SLOTS; i++) {
      const f = slots[i];
      const b = document.createElement("button");
      b.type = "button";
      b.className = "slot";
      b.disabled = !f;
      b.title = f ? prettyFormula(f) : "Empty slot";
      const n = document.createElement("kbd");
      n.textContent = String(i + 1);
      const label = document.createElement("span");
      label.textContent = f ? prettyFormula(f) : "";
      b.append(n, label);
      if (f) b.addEventListener("click", () => doCast(f));
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

  let moteTimer = 0;
  function say(text: string): void {
    byId("mote-text").textContent = text;
    byId("mote").classList.add("show");
    window.clearTimeout(moteTimer);
    moteTimer = window.setTimeout(() => byId("mote").classList.remove("show"), 9000);
    // The youngest players hear everything read aloud.
    if (tier === "sprout" && "speechSynthesis" in window) {
      window.speechSynthesis.cancel();
      window.speechSynthesis.speak(new SpeechSynthesisUtterance(text));
    }
  }

  let lastHudKey = "";
  function renderHud(s: GameState): void {
    for (const m of s.messages) say(m);

    const done = s.objectives.filter((o) => o.done).length;
    const key = JSON.stringify([s.jars, s.charge, done, s.trial_complete]);
    byId("readout").textContent = `${s.aim_substance} · ${describeTemp(s.aim_temp_c)}`;
    if (key === lastHudKey) return;
    lastHudKey = key;

    byId("realm").textContent = s.trial_complete
      ? `${REALM_NAMES[tier]} · Breakthrough!`
      : `${REALM_NAMES[tier]} · Trial ${done}/${s.objectives.length}`;
    byId("realm-fill").style.width = `${(done / s.objectives.length) * 100}%`;

    const list = byId("objective-list");
    list.replaceChildren();
    for (const o of s.objectives) {
      const li = document.createElement("li");
      li.textContent = prettyFormula(o.text);
      if (o.done) li.className = "done";
      list.append(li);
    }

    const jars = byId("jars");
    jars.replaceChildren();
    for (const [formula, n] of Object.entries(s.jars)) {
      const jar = document.createElement("div");
      jar.className = "jar";
      jar.title = `${formula}: ${n}`;
      const fill = document.createElement("div");
      fill.className = `fill f-${formula.toLowerCase()}`;
      fill.style.height = `${Math.min(100, (n / 60) * 100)}%`;
      const glass = document.createElement("div");
      glass.className = "glass";
      glass.append(fill);
      const label = document.createElement("span");
      label.textContent = `${prettyFormula(formula)} ${n}`;
      jar.append(glass, label);
      jars.append(jar);
    }
    const charge = document.createElement("div");
    charge.className = "jar charge";
    charge.textContent = `⚡ ${s.charge}`;
    charge.title = "Electricity charge";
    jars.append(charge);
  }

  // ------------------------------------------------------------------- loop

  const pixels = () => new Uint8ClampedArray(memory.buffer, game.render(lens), w * h * 4);

  function loop(): void {
    frame++;
    if (!ring.isOpen || frame % SLOW_MOTION_EVERY === 0) game.tick(inputBits());
    image.data.set(pixels());
    ctx!.putImageData(image, 0, 0);
    if (frame % 6 === 0 || state === null) {
      state = JSON.parse(game.state()) as GameState;
      renderHud(state);
    }
    requestAnimationFrame(loop);
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
