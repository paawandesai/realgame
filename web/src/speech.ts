// Read-aloud with the browser's own speech synthesis (no audio files, no
// network). Every line has a caption on screen and a separate speakable
// string with no symbols, so "2H2 + O2 -> 2H2O" is read as words.

import { words } from "./device";
import { icon, type IconName } from "./icons";

const NAMES: Record<string, [string, string]> = {
  // formula: [one, many]
  H2O: ["water", "water"],
  H2: ["hydrogen", "hydrogen"],
  O2: ["oxygen", "oxygen"],
  CO2: ["carbon dioxide", "carbon dioxide"],
  CH4: ["methane", "methane"],
  C: ["carbon", "carbon"],
  H: ["hydrogen atom", "hydrogen atoms"],
  O: ["oxygen atom", "oxygen atoms"],
};

const NUMBERS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];

function numberWord(n: number): string {
  return NUMBERS[n] ?? String(n);
}

/** Turns a formula term such as "2H2O" into words ("two water"). */
function termWords(coef: string, formula: string): string {
  const n = coef ? Number(coef) : 1;
  const name = NAMES[formula];
  if (name) return coef ? `${numberWord(n)} ${n === 1 ? name[0] : name[1]}` : name[0];
  // Unknown formula: spell it out letter by letter with numbers as words.
  const spelled = formula.replace(/([A-Z][a-z]?)(\d*)/g, (_, el: string, d: string) => ` ${el}${d ? ` ${numberWord(Number(d))}` : ""}`).trim();
  return coef ? `${numberWord(n)} ${spelled}` : spelled;
}

/**
 * A line safe to read aloud: arrows become "makes", formulas become names,
 * "+1 O" becomes "one oxygen atom", and symbols are dropped.
 */
export function speakable(text: string): string {
  let s = text
    .replace(/(\d)½/g, "$1 and a half ")
    .replace(/½/g, " half ")
    .replace(/×/g, " times ")
    .replace(/→|->/g, " makes ")
    .replace(/ΔH\s*=\s*/g, "energy ")
    .replace(/[−-](\d)/g, "minus $1")
    .replace(/\bkJ\b/g, "kilojoules")
    .replace(/°C/g, " degrees")
    // "+1 O" (leftover orbs).
    .replace(/\+(\d+)\s*([A-Z][a-z]?)\b/g, (_, n: string, el: string) => termWords(n, el));
  // Formula terms: an optional coefficient then element symbols with counts.
  s = s.replace(/\b(\d*)((?:[A-Z][a-z]?\d*)+)\b/g, (m, coef: string, f: string) => {
    // Plain words that start with a capital ("Boom", "Fire") are not formulas.
    if (!/\d/.test(f) && !(f in NAMES)) return m;
    if (/^[A-Z][a-z]+$/.test(f) && !(f in NAMES)) return m;
    return termWords(coef, f);
  });
  return s
    .replace(/ \+ /g, " plus ")
    .replace(/[₀-₉]/g, (d) => ` ${numberWord("₀₁₂₃₄₅₆₇₈₉".indexOf(d))}`)
    .replace(/[()[\]{}<>*_#|~^]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

type SpeechWin = Window & { __speechLog?: string[] };

/** How long a caption stays up. */
const CAPTION_MS = 9000;
/** A caption that waited behind a card shows for at least this long. */
const CAPTION_MIN_MS = 3000;
/** A caption that waited longer than this is old news and never shows. */
const CAPTION_STALE_MS = 8000;

export class Speech {
  private unlocked = false;
  private voice: SpeechSynthesisVoice | null = null;
  private lastCaption = "";
  private lastSpoken = "";
  private hideTimer = 0;
  enabled: boolean;
  /** Show the caption box. Hints still show when this is off. */
  captions = true;
  /** A picture for a caption (flame, ice, crystal, bubbles), shown beside
   * it for readers who are still learning; null for none. */
  pictureFor: ((caption: string) => IconName | null) | null = null;

  constructor(
    enabled: boolean,
    private readonly box: HTMLElement,
    private readonly text: HTMLElement,
    replay: HTMLButtonElement,
  ) {
    this.enabled = enabled;
    replay.addEventListener("click", () => this.replay());
    if (this.supported) {
      const pick = () => this.pickVoice();
      pick();
      window.speechSynthesis.addEventListener?.("voiceschanged", pick);
    }
  }

  private get supported(): boolean {
    return typeof window.speechSynthesis !== "undefined" && typeof SpeechSynthesisUtterance !== "undefined";
  }

  private pickVoice(): void {
    try {
      const voices = window.speechSynthesis.getVoices();
      const lang = (navigator.language || "en").slice(0, 2);
      // Prefer voices that run on the device (nothing is sent anywhere).
      const local = voices.filter((v) => v.localService);
      this.voice =
        local.find((v) => v.lang.startsWith(lang)) ?? local.find((v) => v.lang.startsWith("en")) ?? local[0] ?? null;
    } catch {
      this.voice = null;
    }
  }

  /** Call from a user gesture; browsers only speak after one. */
  unlock(): void {
    if (this.unlocked) return;
    this.unlocked = true;
    // Say the line that was on screen before the first tap.
    if (this.lastSpoken) this.speak(this.lastSpoken);
  }

  /** Another transient (a cast card, a Grimoire page) is on screen: the
   * caption waits and shows once it is gone, if it is still fresh. */
  private held = false;
  /** When the caption waiting (or showing) was said; 0 for none. */
  private captionAt = 0;

  /** Whether the caption box is on screen. */
  get showing(): boolean {
    return this.box.classList.contains("show");
  }

  /**
   * One message at a time: while `on`, the caption box stays hidden (a line
   * said meanwhile waits). When the hold ends, the latest line comes back
   * for the rest of its time, unless it is older than CAPTION_STALE_MS.
   */
  hold(on: boolean): void {
    if (on === this.held) return;
    this.held = on;
    if (on) {
      this.box.classList.remove("show");
      return;
    }
    const age = performance.now() - this.captionAt;
    if (this.captionAt > 0 && age < CAPTION_STALE_MS) this.showBox(CAPTION_MS - Math.min(age, CAPTION_MS - CAPTION_MIN_MS));
    else this.captionAt = 0;
  }

  /** A new player action: the caption about what happened before is stale
   * ("Too close!" must not outlive a fix), so it goes now. Replay still
   * says it again. */
  clear(): void {
    window.clearTimeout(this.hideTimer);
    this.captionAt = 0;
    this.box.classList.remove("show");
  }

  private showBox(ms: number): void {
    this.box.classList.add("show");
    window.clearTimeout(this.hideTimer);
    this.hideTimer = window.setTimeout(() => {
      this.box.classList.remove("show");
      this.captionAt = 0;
    }, ms);
  }

  /** Shows a caption and reads it aloud (if voice is on). */
  say(rawCaption: string, rawSpoken: string = speakable(rawCaption), force = false): void {
    // "Tap" on touch screens, "Click" with a mouse.
    const caption = words(rawCaption);
    const spoken = words(rawSpoken);
    this.lastCaption = caption;
    this.lastSpoken = spoken;
    if (this.unlocked) this.speak(spoken);
    if (!this.captions && !force) return;
    this.text.textContent = caption;
    const pic = this.pictureFor?.(caption) ?? null;
    let slot = this.box.querySelector<HTMLElement>(".mote-pic");
    if (pic) {
      if (!slot) {
        slot = document.createElement("span");
        slot.className = "mote-pic";
        slot.setAttribute("aria-hidden", "true");
        this.text.before(slot);
      }
      slot.replaceChildren(icon(pic));
    } else slot?.remove();
    this.captionAt = performance.now();
    if (this.held) {
      window.clearTimeout(this.hideTimer);
      return;
    }
    this.showBox(CAPTION_MS);
  }

  /** Reads a line aloud with no caption: for words already on screen in
   * their own place (the hint chip), so they never show twice. */
  sayOnly(rawCaption: string, rawSpoken: string = speakable(rawCaption)): void {
    const spoken = words(rawSpoken);
    this.lastCaption = words(rawCaption);
    this.lastSpoken = spoken;
    if (this.unlocked) this.speak(spoken);
  }

  replay(): void {
    if (!this.lastCaption) return;
    this.unlocked = true;
    this.say(this.lastCaption, this.lastSpoken, true);
  }

  stop(): void {
    if (this.supported) window.speechSynthesis.cancel();
  }

  private speak(line: string): void {
    if (!this.enabled || !this.supported || !line) return;
    try {
      const synth = window.speechSynthesis;
      synth.cancel();
      const u = new SpeechSynthesisUtterance(line);
      u.rate = 0.9;
      if (this.voice) {
        u.voice = this.voice;
        u.lang = this.voice.lang;
      }
      synth.speak(u);
      const w = window as SpeechWin;
      if (w.__speechLog) w.__speechLog.push(line);
    } catch {
      // Speech can fail on some systems; the caption is still on screen.
    }
  }
}
