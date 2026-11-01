// Worked casts (I-029): the field overlay. Before a spell is mastered a click
// opens a small builder (I-028's mountBuilder) below the crosshair and the
// world holds; a correct build casts. Rust owns every decision: this module
// only draws `window.__workState` and sends `work …` commands.
// See docs/loop/items/I-029/spec.md, section 6.

/**
 * @param {{ doc?: Document, win?: Window, cmd?: (c: string) => void, bench?: unknown }} _opts
 */
export function createWorked(_opts) {
  return {
    /** @param {unknown} _ws */
    update(_ws) {},
    isOpen() { return false; },
    /** @param {number} _dx @param {number} _dy */
    move(_dx, _dy) { return false; },
    /** @param {number} _button */
    press(_button) { return false; },
    /** @param {KeyboardEvent} _e */
    key(_e) { return false; },
    cancel() {},
  };
}

export function startWorked() {}
