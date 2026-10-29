// The Grimoire: one card per reaction this level can make, face down (a
// silhouette) until the player first makes it. Found pages are remembered in
// localStorage on this device only; if storage is blocked the book still
// works for the visit.

import { GRIMOIRE_PAGES, formatDh, type Book, type HoverCard } from "./book";
import { equationBalls, icon } from "./icons";
import { storageGet, storageSet } from "./prefs";
import { prettyFormula, type Tier } from "./types";

const KEY = "grimoire";
const CONFETTI = 28;
const CONFETTI_COLORS = ["#e69f00", "#56b4e9", "#009e73", "#f0e442", "#0072b2", "#d55e00", "#cc79a7"];

export class Grimoire {
  readonly found = new Set<string>();
  private popTimer = 0;

  constructor(
    private readonly book: Book,
    private readonly tier: Tier,
    private readonly hover: HoverCard,
  ) {
    try {
      const saved = JSON.parse(storageGet(KEY) ?? "[]") as unknown;
      if (Array.isArray(saved)) {
        for (const id of saved) if (typeof id === "string" && GRIMOIRE_PAGES.some((p) => p.id === id)) this.found.add(id);
      }
    } catch {
      // A damaged entry just means an empty book.
    }
  }

  get total(): number {
    return GRIMOIRE_PAGES.length;
  }

  /** Records a page. Returns true the first time it is ever found. */
  discover(id: string): boolean {
    if (!GRIMOIRE_PAGES.some((p) => p.id === id) || this.found.has(id)) return false;
    this.found.add(id);
    storageSet(KEY, JSON.stringify([...this.found]));
    return true;
  }

  /** Fills a list with every page: face up if found, a silhouette if not. */
  render(list: HTMLElement, flip: ReadonlySet<string> = new Set()): void {
    list.replaceChildren(...GRIMOIRE_PAGES.map((p) => this.card(p.id, flip.has(p.id))));
  }

  /** The "new page" moment: the card flips face up in the middle of the screen, with confetti. */
  celebrate(host: HTMLElement, id: string, calm: boolean): void {
    const title = document.createElement("strong");
    title.className = "discover-title";
    title.textContent = this.tier === "sprout" ? "New card!" : "New Grimoire page!";
    const card = this.card(id, true);
    const parts: HTMLElement[] = [title, card];
    if (!calm) {
      const burst = document.createElement("div");
      burst.className = "confetti";
      burst.setAttribute("aria-hidden", "true");
      for (let i = 0; i < CONFETTI; i++) {
        const c = document.createElement("span");
        const angle = (i / CONFETTI) * Math.PI * 2;
        const dist = 120 + (i % 5) * 30;
        c.style.setProperty("--dx", `${Math.cos(angle) * dist}px`);
        c.style.setProperty("--dy", `${Math.sin(angle) * dist - 40}px`);
        c.style.setProperty("--rot", `${(i * 47) % 360}deg`);
        c.style.background = CONFETTI_COLORS[i % CONFETTI_COLORS.length]!;
        burst.append(c);
      }
      parts.push(burst);
    }
    host.replaceChildren(...parts);
    host.hidden = false;
    window.clearTimeout(this.popTimer);
    this.popTimer = window.setTimeout(() => (host.hidden = true), 3200);
  }

  private card(id: string, flip: boolean): HTMLLIElement {
    const page = GRIMOIRE_PAGES.find((p) => p.id === id);
    const r = this.book.byId.get(id);
    const li = document.createElement("li");
    li.className = "grim-card";
    li.dataset.id = id;
    const has = this.found.has(id);
    li.classList.add(has ? "found" : "silhouette");
    if (flip && has) li.classList.add("flip");
    const pic = document.createElement("span");
    pic.className = "grim-pic";
    if (page) pic.append(icon(page.picture));
    const name = document.createElement("strong");
    name.className = "grim-name";
    li.append(pic, name);
    if (!has || !r || !page) {
      name.textContent = "?";
      li.setAttribute("aria-label", "Page not found yet");
      return li;
    }
    name.textContent = page.name;
    const eq = document.createElement("span");
    eq.className = "grim-eq";
    if (this.tier === "sprout") eq.append(equationBalls(r.equation));
    else eq.textContent = prettyFormula(r.equation);
    const dh = document.createElement("span");
    dh.className = `grim-dh ${r.delta_h_kj < 0 ? "exo" : "endo"}`;
    dh.textContent =
      this.tier === "sprout"
        ? r.delta_h_kj < 0
          ? "Gets hot!"
          : "Needs power"
        : `ΔH ${formatDh(r.delta_h_kj)}${r.delta_h_kj < 0 ? " · heat out" : " · energy in"}`;
    const words = document.createElement("span");
    words.className = "grim-words";
    const src = document.createElement("span");
    src.className = "grim-src";
    src.textContent = r.source.split(",")[0] ?? r.source;
    if (this.tier === "sprout") {
      words.textContent = r.short_sprout;
      li.append(eq, dh, words);
    } else if (this.tier === "seedling") {
      // The face keeps it short; the numbers and their source wait behind
      // "Why?" (tap the card, or press Enter on it).
      words.textContent = r.short_seedling;
      const why = document.createElement("span");
      why.className = "grim-why";
      why.textContent = "Why?";
      const back = document.createElement("span");
      back.className = "grim-back";
      const concept = document.createElement("span");
      concept.className = "grim-words";
      concept.textContent = r.concept;
      back.append(dh, concept, src);
      li.append(eq, words, why, back);
      li.setAttribute("aria-expanded", "false");
      const turn = () => {
        const open = li.classList.toggle("show-back");
        li.setAttribute("aria-expanded", String(open));
        why.textContent = open ? "Back" : "Why?";
      };
      li.addEventListener("click", turn);
      li.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          e.stopPropagation();
          turn();
        }
      });
    } else {
      words.textContent = r.concept;
      li.append(eq, dh, words, src);
    }
    li.tabIndex = 0;
    li.setAttribute("aria-label", `${page.name}: ${r.equation}`);
    // Seedling cards turn over to show the same details, so no pop-up too.
    if (this.tier === "seedling") return li;
    this.hover.attach(li, () => ({
      title: `${page.name}: ${prettyFormula(r.equation)}`,
      lines: [`ΔH = ${formatDh(r.delta_h_kj)}`, r.concept, r.real_world],
      source: r.source,
    }));
    return li;
  }
}
