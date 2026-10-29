// Which kind of pointer the player is using, so instructions say "tap" on a
// touch screen and "click" with a mouse, and touch controls show when needed.
// Never mentions particular mouse buttons.

let touch = false;

/** True when the player last used (or the device mainly uses) a finger or pen. */
export function isTouch(): boolean {
  return touch;
}

/** Watches the pointer type and keeps `body[data-input]` up to date. */
export function initDevice(onChange: () => void = () => {}): void {
  try {
    touch = window.matchMedia("(pointer: coarse)").matches;
  } catch {
    touch = false;
  }
  document.body.dataset.input = touch ? "touch" : "mouse";
  window.addEventListener(
    "pointerdown",
    (e) => {
      const now = e.pointerType === "touch" || e.pointerType === "pen";
      if (now === touch) return;
      touch = now;
      document.body.dataset.input = touch ? "touch" : "mouse";
      onChange();
    },
    { capture: true },
  );
}

/** Rewrites an instruction for the current device ("Tap" or "Click"). */
export function words(text: string): string {
  if (touch)
    return text
      .replace(/\bHold G\b/g, "Hold the scoop")
      .replace(/\bhold G\b/g, "hold the scoop")
      .replace(/\bpress Enter\b/g, "tap Go")
      .replace(/\bClick\b/g, "Tap")
      .replace(/\bclick\b/g, "tap");
  return text.replace(/\bTap\b/g, "Click").replace(/\btap\b/g, "click");
}
