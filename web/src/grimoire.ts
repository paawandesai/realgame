// The Grimoire: one card per reaction this level can make, face down (a
// silhouette) until the player first makes it. Found pages are remembered in
// localStorage on this device only; if storage is blocked the book still
// works for the visit.

import { BAR_SPELLS, GRIMOIRE_PAGES, earnablePages, formatDh, stateNote, type Book, type GrimoirePage, type HoverCard } from "./book";
import { equationBalls, icon } from "./icons";
import { storageGet, storageSet } from "./prefs";
import { prettyFormula, SEEDLING_SAFETY, type Tier } from "./types";

const KEY = "grimoire";
const CONFETTI = 20;
const CONFETTI_COLORS = ["#e69f00", "#56b4e9", "#009e73", "#f0e442", "#0072b2", "#d55e00", "#cc79a7"];

export class Grimoire {
  readonly found = new Set<string>();

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

  /** Pages this tier can earn in this trial. */
  get total(): number {
    return earnablePages(this.tier).length;
  }

  /** Earnable pages found so far. */
  get earned(): number {
    return earnablePages(this.tier).filter((p) => this.found.has(p.id)).length;
  }

  /** Records a page. Returns true the first time it is ever found. */
  discover(id: string): boolean {
    const page = GRIMOIRE_PAGES.find((p) => p.id === id);
    if (!page || page.soon || this.found.has(id)) return false;
    this.found.add(id);
    storageSet(KEY, JSON.stringify([...this.found]));
    return true;
  }

  /** A challenge earned a "coming soon" page: it becomes a full page.
   * Returns true the first time. */
  unlock(id: string): boolean {
    const page = GRIMOIRE_PAGES.find((p) => p.id === id);
    if (!page || this.found.has(id)) return false;
    this.found.add(id);
    storageSet(KEY, JSON.stringify([...this.found]));
    return true;
  }

  /**
   * Fills a list with the pages: face up if found, a silhouette if not,
   * then the "coming soon" page. `cleared` holds the rooms (1-4) already
   * done, so no face-down page sends the player back to one. On the end
   * screen Sprout sees only the cards earned and one "more soon" card.
   */
  render(list: HTMLElement, flip: ReadonlySet<string> = new Set(), cleared: ReadonlySet<number> = new Set(), end = false): void {
    let pages = GRIMOIRE_PAGES.filter((p) => p.soon || p.tiers.includes(this.tier) || this.found.has(p.id));
    if (end && this.tier === "sprout") pages = pages.filter((p) => p.soon || this.found.has(p.id));
    list.replaceChildren(...pages.map((p) => this.card(p, flip.has(p.id), cleared)));
  }

  /**
   * The "new page" moment: a compact copy of the card pops face up in the
   * popup corner, with a little confetti. Fills `host`; the caller shows and
   * hides it (one popup at a time).
   */
  celebrate(host: HTMLElement, id: string, calm: boolean): void {
    const title = document.createElement("strong");
    title.className = "discover-title";
    title.textContent = this.tier === "sprout" ? "New card!" : "New Grimoire page!";
    const page = GRIMOIRE_PAGES.find((p) => p.id === id);
    if (!page) return;
    const card = this.card(page, true);
    card.classList.add("compact");
    card.removeAttribute("tabindex");
    const parts: HTMLElement[] = [title, card];
    if (!calm) {
      const burst = document.createElement("div");
      burst.className = "confetti";
      burst.setAttribute("aria-hidden", "true");
      for (let i = 0; i < CONFETTI; i++) {
        const c = document.createElement("span");
        const angle = (i / CONFETTI) * Math.PI * 2;
        const dist = 50 + (i % 5) * 14;
        c.style.setProperty("--dx", `${Math.cos(angle) * dist}px`);
        c.style.setProperty("--dy", `${Math.sin(angle) * dist - 20}px`);
        c.style.setProperty("--rot", `${(i * 47) % 360}deg`);
        c.style.background = CONFETTI_COLORS[i % CONFETTI_COLORS.length]!;
        burst.append(c);
      }
      parts.push(burst);
    }
    // Replaced, never added to: the popup holds one card at a time.
    host.replaceChildren(...parts);
  }

  /** A dangerous reaction's real-world note: words for readers, a
   * "Stand back!" picture for Sprout. */
  private safety(note: string | undefined): HTMLElement | null {
    if (!note) return null;
    const el = document.createElement("span");
    el.className = "grim-safety";
    if (this.tier === "sprout") {
      // Pictures only: the boom and an arrow pointing away from it.
      el.append(icon("boom"), icon("left"));
      el.setAttribute("aria-label", "Stand back");
    }
    else {
      const words = document.createElement("span");
      words.textContent = this.tier === "seedling" ? SEEDLING_SAFETY : prettyFormula(note);
      el.append(icon("warning"), words);
    }
    return el;
  }

  /**
   * The long words (why it happens, the source, the safety note) wait
   * behind a "More" button, so a card stays short and the book fits the
   * screen. Sprout reads none of it: its parts show as they are.
   */
  private more(li: HTMLElement, parts: (HTMLElement | null)[]): void {
    const body = parts.filter((p): p is HTMLElement => p !== null);
    if (body.length === 0) return;
    if (this.tier === "sprout") {
      li.append(...body);
      return;
    }
    const box = document.createElement("span");
    box.className = "grim-more-body";
    box.append(...body);
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "grim-more";
    btn.textContent = "More";
    btn.setAttribute("aria-expanded", "false");
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const open = li.classList.toggle("show-more");
      btn.setAttribute("aria-expanded", String(open));
      btn.textContent = open ? "Less" : "More";
    });
    li.append(btn, box);
  }

  /** A found pour page (water, carbon dioxide): no reaction, so no ΔH. */
  private pourCard(li: HTMLLIElement, name: HTMLElement, page: GrimoirePage): HTMLLIElement {
    const formula = page.formula ?? "";
    const spell = BAR_SPELLS.find((s) => s.formula === formula);
    name.textContent = page.name;
    const eq = document.createElement("span");
    eq.className = "grim-eq";
    if (this.tier === "sprout") eq.append(equationBalls(formula));
    else eq.textContent = prettyFormula(formula);
    const words = document.createElement("span");
    words.className = "grim-words";
    words.textContent = this.tier === "apprentice" ? (spell?.note ?? page.short ?? "") : (page.short ?? "");
    li.append(eq, words);
    const phase = page.phase ? this.book.phases.get(page.phase) : undefined;
    if (phase && this.tier !== "sprout") {
      // A phase change from the sourced table: energy in per mole, no reaction.
      const dh = document.createElement("span");
      dh.className = "grim-dh endo";
      dh.textContent = `ΔH +${phase.kj.toFixed(2)} kJ/mol · energy in, at ${phase.c} °C`;
      li.append(dh);
      if (this.tier === "apprentice") {
        const src = document.createElement("span");
        src.className = "grim-src";
        src.textContent = phase.source;
        li.append(src);
      }
    }
    if (this.tier === "apprentice" && spell?.noteSource) {
      const src = document.createElement("span");
      src.className = "grim-src";
      src.textContent = spell.noteSource;
      li.append(src);
    }
    li.tabIndex = 0;
    li.setAttribute("aria-label", `${page.name}: ${formula}`);
    return li;
  }

  private card(page: GrimoirePage, flip: boolean, cleared: ReadonlySet<number> = new Set()): HTMLLIElement {
    const id = page.id;
    const r = this.book.byId.get(id);
    const li = document.createElement("li");
    li.className = "grim-card";
    li.dataset.id = id;
    // A "coming soon" page a free-play challenge unlocked is a full page.
    const has = this.found.has(id);
    li.classList.add(has ? "found" : "silhouette");
    if (page.soon && !has) li.classList.add("soon");
    if (flip && has) li.classList.add("flip");
    const pic = document.createElement("span");
    pic.className = "grim-pic";
    pic.append(icon(page.picture));
    const name = document.createElement("strong");
    name.className = "grim-name";
    li.append(pic, name);
    if (page.soon && !has) {
      // Sprout: a clock picture (later), no words.
      if (this.tier === "sprout") name.append(icon("clock"));
      else name.textContent = page.name;
      if (this.tier !== "sprout") {
        const soon = document.createElement("span");
        soon.className = "grim-where";
        soon.textContent = prettyFormula(page.soon);
        li.append(soon);
      }
      li.setAttribute("aria-label", this.tier === "sprout" ? "More cards soon" : `${page.name}: ${page.soon}`);
      return li;
    }
    if (!has || (!r && !page.formula)) {
      name.textContent = "?";
      // Never a dead end: every face-down page says how to earn it, and
      // never sends the player back to a room already cleared.
      const where = this.tier === "sprout" ? page.whereSprout : cleared.has(page.room) ? page.again : page.where;
      const hint = document.createElement("span");
      hint.className = "grim-where";
      hint.textContent = this.tier === "sprout" ? where : prettyFormula(where);
      li.append(hint);
      this.more(li, [this.safety(r?.safety)]);
      li.setAttribute("aria-label", `Page not found yet. ${where}${r?.safety && this.tier !== "sprout" ? ` ${r.safety}` : ""}`);
      return li;
    }
    if (!r) return this.pourCard(li, name, page);
    name.textContent = page.name;
    const eq = document.createElement("span");
    eq.className = "grim-eq";
    if (this.tier === "sprout") eq.append(equationBalls(r.equation));
    else if (this.tier === "apprentice") eq.textContent = prettyFormula(r.equation_states ?? r.equation);
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
      this.more(li, [this.safety(r.safety)]);
    } else if (this.tier === "seedling") {
      // The face keeps it short; the numbers, why it happens, the source
      // and the safety note wait behind "More".
      words.textContent = r.short_seedling;
      const concept = document.createElement("span");
      concept.className = "grim-words";
      concept.textContent = r.concept;
      li.append(eq, words);
      this.more(li, [dh, concept, src, this.safety(r.safety)]);
    } else {
      words.textContent = r.concept;
      li.append(eq, dh);
      const note = stateNote(r.id, this.book);
      let n: HTMLElement | null = null;
      if (note) {
        n = document.createElement("span");
        n.className = "grim-note";
        n.textContent = note;
      }
      this.more(li, [words, n, src, this.safety(r.safety)]);
    }
    li.tabIndex = 0;
    li.setAttribute("aria-label", `${page.name}: ${r.equation}${r.safety && this.tier !== "sprout" ? `. ${r.safety}` : ""}`);
    // Seedling cards show the same details behind "More", so no pop-up too.
    if (this.tier === "seedling") return li;
    this.hover.attach(li, () => ({
      title: `${page.name}: ${prettyFormula(r.equation_states ?? r.equation)}`,
      // The steam note is already printed on the page itself: one copy only.
      lines: [`ΔH = ${formatDh(r.delta_h_kj)}`, r.concept, r.real_world],
      source: stateNote(r.id, this.book) && this.book.vaporNote ? `${r.source}; ${this.book.vaporNote.source}` : r.source,
    }));
    return li;
  }
}
