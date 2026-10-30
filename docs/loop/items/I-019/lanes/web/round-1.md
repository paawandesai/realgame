# I-019 web lane, round 1

Branch `loop/I-019`, based on origin/main 265a93e. One builder did both lanes; the e2e tests were written before the implementation.

## What changed

- **`web/hud.js`**
  - A locked left `pointerdown` sends `cmd('fire')` and still sets `input.fire` for holds. There is no `isTrusted` check.
  - `#obj-line` is a text node `Fires to put out: N` plus `.fire-pips`, one `.fire-pip` per fire up to 8, then `.fire-more` (a plus drawn by CSS, so the text stays `Fires to put out: N`). A win still reads `Greenhouse saved!`.
  - `#firefinder` (a chevron and a flame, no text, 72×64 px) sits at the left or right edge. Its class is `right` iff `fireBearing > 0`. It is shown only when `playing && fireOff && nearFire >= 0 && !paused`.
  - `#crosshair.fire` is set when locked and `objs[aimObj].burning`. `canvas.hover-fire` is set when the hovered target is burning.
  - Escape while locked, and `KeyP`, call `exitPointerLock()` and `showPause(true)` in the same handler. P while paused acts like Resume. The pause table gains the row `Pause | Esc or P`, added with `createElement`.
  - Hints: nothing while a banner shows. The locked hint is `Aim at a fire and click.` while `extinguished === 0`.
  - Test hooks: `#hud[data-frame]` and `__game.hudState()` (the state the HUD last drew).
- **`web/style.css`**: `canvas.hover` uses `cursor: crosshair` and `hover-fire` uses `pointer`. `#crosshair.fire span` gets an outer orange ring drawn with `box-shadow`. Styles for the pips and the finder. The finder pulses at scale 1 to 1.1 over 1 s, alternating, so 0.5 Hz, with `animation: none` under reduced motion.
- **`content/science-sources.md`**: three not-claims rows, `Aim at a fire and click.`, `Fires to put out: ${state.firesLeft}` and the new startup caption. These replace the old objective template, the glowing-target hint and the "…Press 2 for Water." caption.
- **`tests/e2e/golden.mjs`**
  - `castingYouCanSee(...)` holds tests 7–12 and 14–19 and is called after `feelAndStandards`, before `no console errors`.
  - The clock section's key-order check now expects `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]`. This is the one sanctioned edit to an existing check.

## Measured (golden, `QUERY=?low`, under the chromium lock)

- Run 1: 90/98. All 75 existing checks passed. The 8 new failures were all test-side: on this machine the game runs at about 1 fps, and its animation-frame callback can run after the HUD's, so a read that waited for `#hud[data-frame] === state.frame` used up its retries. The values in those failures were already correct. The fix reads `__game.hudState()`, the state the DOM was drawn from. Test 9 now takes its aim after the pointer move has landed. Test 15 restarts the 45 s idle clock with a pause and resume, because at 1 fps the idle hint (which ranks above the locked hint) was due.
- Run 2: **98/98 passed**, including:
  - `cast: one synthetic click casts once and puts the fire out … clicks 0 -> 1, casts 0 -> 1, … hitKind=out, firesLeft 3 -> 2, frames 1`
  - `cast: a real click casts exactly once casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=false`; hold: `casts +4 over 1.632 game s`
  - `cast: a click casts and shows a stream within 2 frames cast after 1 frames, stream after 1 frames`
  - `cast: a fire behind the player shows the arrow on its side … bearing=-122.2 … w 78.3`, then `11 steps, consistent=true, bearing=-32.4 fireOff=false`
  - `cast: Esc pauses and frees the mouse under automation` and `P pauses and P again resumes; … row=true`
  - `cast: the crosshair adds a fire ring only on a burning target … withFire "rgba(0,0,0,0.75) 0 0 0 2px, rgb(255,122,26) 0 0 0 5px, …" without "none"`
  - `cast: point mode marks a fire … fire: [hover hover-fire]; bush: [hover]`
  - `cast: nothing shows under the lose banner` / `… win banner`: hint "", finder hidden, no nudge
  - `cast: the stream and splash do not flash the screen`: Water 2.1 %, CO2 1.9 %
  - `no console errors`: PASS
- `npm test`: 50 pass, 0 fail (`science-sources: 33 client claims, 33 sourced`).

## Deviations

- **Hint priority.** The spec puts `note` above the idle hint. That order fails I-007's existing check `feel: idle hint appears after 45 s`, because the point-and-click note is still set from I-007 test 13 when that check runs. Existing checks must not be weakened, so the order here is: banner, idle hint, note, locked hint, start hint. No new test depends on note-over-idle.
- **Test 12 pixel check.** `__game.pause(true)` shows the pause card over the stream, so the test hides `#pause` for the screenshot only; the game stays paused.
- **Not run:** test 21 (perf.mjs paired runs and brotli delta against a base build), which belongs to the tester.
