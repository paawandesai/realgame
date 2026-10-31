# I-028 web lane, round 1

Branch `loop/I-028-cw` (the client lane is commit c1e0f2e on the same branch). Web commits: 49f815e
(bench.js, style.css, node tests, register rows), 85dcf76 (golden tests 12-28b, analytics-e2e test 29,
a `fake` fix), plus this note and its evidence.

Paths touched, all from the web lane's row in the Lanes table: `crates/bevy-client/web/bench.js`,
`crates/bevy-client/web/style.css` (one appended block), `test/bench.test.ts` (new),
`tests/e2e/golden.mjs` (`formulaBench` body only, appended after the stub's test 12b; 0 deleted
lines), `tests/e2e/analytics-e2e.mjs` (new section 15 appended after section 14 (the picker's
lock/inert checks), before the `catch`), `content/science-sources.md` (rows only),
`test/science-sources.test.ts` (the three `prompt_*` keys in `SPELL_TEXT_KEYS` only). `hud.js`,
`index.html`, `age.js`, `analytics.js`, `progress.js`, `boot.js` and `build-web.sh` are unchanged
beyond the stub.

## Built, in order

1. **Tests 8-11, 11b first (red).** `test/bench.test.ts`, 9 tests. Against the stub `bench.js`
   (swapped in from HEAD): **9/9 fail**. With the bodies: 9/9 pass. 11b uses a minimal DOM shim
   inside the test (compound selectors, descendant selectors, click/input/keydown dispatch). There is
   no `linkedom`, so no dependency was added and no golden 28c was needed.
2. **Pure exports:** `composeSprout`, `composeSeedling`, `composeEquation`, `scaleOf`, `reasonFor`
   (every Contract string exactly, one line per diff joined by `\n`, `(n/a)` codes give the
   Apprentice string, Seedling `${formula}` without its state), `bandFrom`, plus `pretty` (subscripts
   and an arrow, state kept or stripped).
3. **Modal:** `#bench` (`role="dialog"`, `aria-modal="true"`, z-index 900). Opening remembers
   `wasLocked`/`wasPaused`, calls `exitPointerLock` and `__game.pause(true)`, and makes `#stage`, `#hud`,
   `#loading` and `#pause` inert, remembering each one's previous value (`#benchbtn` is inert too
   while the bench is open). It then focuses the first control and pushes `open`. Closing (Close or
   Escape, also from the text box) pushes `close` and restores each remembered `inert`. It then calls
   `pause(wasPaused)`, calls `__game.lock()` if the game was locked and the close was not Escape, and
   focuses the canvas.
4. **Key capture:** window-capture `keydown` and `keypress` only (never `keyup`). While open, every
   key stops there. Tab and Shift+Tab trap focus inside the bench. Arrow keys move focus, except that
   ArrowUp/ArrowDown step a focused tile or coefficient and arrows inside the text box move the
   caret. Enter in the text box sends the build. Printable keys are never `preventDefault`ed. A
   capture `focusin` keeps focus inside. While closed, only `KeyB` is read (no modifiers, not in an
   editable field) and only through the modal gate.
5. **One modal gate:** any `[aria-modal="true"]` other than `#bench` that has no `hidden`
   attribute, no `.hidden` class and a non-zero rect blocks both `B` and `#benchbtn`.
6. **Band:** URL, then the age picker's click (`bench.js` listens on `#age button[data-band]`), then
   the chooser. The `band <tier>` command is pushed every time the resolved band changes, open or
   closed. It is never stored. **Deviation (small):** a choice made in the bench's own chooser (first
   ask or "Change age") wins for the rest of the page load, even over `?band=`. Without that, "Change
   age" would do nothing on a class link. `bandFrom` itself keeps the spec's order, as test 11 checks.
7. **Chooser** (clones the age picker's three SVGs, 168 px buttons, the question spoken when it shows
   and each label spoken on focus or hover), **list** (cards with the hotbar icon classes, a stage
   badge with a shape and a word), and the **three build views** through `mountBuilder`:
   - Sprout: a recipe of balls grouped by element, a tray, four 104 px atom cards, Undo and Make.
   - Seedling, one substance: the formula with subscripts and 0-4 count tiles with a tick on a match.
   - Seedling Fire: coefficient tiles (1-4) under the balance scale.
   - Apprentice: a text box (maxlength 60, autocomplete off, spellcheck off) with a live preview.
8. **Fizzle panel:** the reason. Sprout `atoms_off` adds a picture diff (`.bench-diff[data-el][data-ok]`
   with tick or cross shapes, counts from the sim's `diffs`). Seedling `unbalanced` adds
   per-element `left/right` rows. Apprentice gets counts only from the third wrong try on that
   spell in this bench session (the counter resets on open). The build stays as it was.
9. **Done panel:** a 160 px icon with one 300 ms pop, a tick shape, the band's `ok` line, the
   Apprentice "Simplest form" and coefficient lines, the Learning badge when `learned`, a rising
   three-note tone (peak 0.15, sound on only), and the line spoken. Buttons: "Build another" (pushes
   `open`, back to the list) and "Back to the game".
10. **Speech guard:** `speechSynthesis` only. Sound counts as on unless the saved settings hold
    `sound: false` (an absent or unreadable key reads as on). `cancel()` runs before each line. It
    does not check `getVoices()`: speaking with no voices is a native no-op.
11. **`#benchbtn`:** a CSS flask and a `B` key cap, 72 px, top right, z-index 800 (above `#pause`,
    so it can be clicked while paused). It has `.locked` (`pointer-events: none`) while the pointer
    is locked, `.near` from `atBench`, and `.hint` after the first cast until any spell is past
    Unknown. All three are read in one `requestAnimationFrame` poll that writes a class only on a
    change. `__gameState` is not parsed for this: two regexes run, and only until the ring is gone
    for good.
12. **Controls row** `Formula Bench | B`, inserted after "Read the last message again" in the
    pause card's table.
13. **`window.__bench`** = `{ state, ui, open, close, cmd }`, plus `fake(view)` under `?debug` only.
14. **The Fire scale was built last, in full; no fallback was needed.** The pans read
    `C 1, H 4, O 2` / `C 1, H 2, O 3`. The beam tilts by 4° per side (`--tilt`, 200 ms, none under
    reduced motion). The heavier pan says "more atoms". A level beam shows a tick and "Level", and
    the scale gets `data-level="true"`.

## Checks

| check | result |
|---|---|
| `npm test` | 82/82 (73 before + 9 in `test/bench.test.ts`) |
| `tsc -p tsconfig.json` (checkJs reaches `bench.js` through the test) | clean |
| science lint, red step (keys added, rows not yet) | 32 unsourced: the 11 prompts and 21 `bench.js` strings |
| science lint, after the rows | green: 42 client claims, 42 sourced, 0 stale |
| register rows | claims C-040..C-048 (two "cold" nitrogen prompts [S1], two methane prompts [S7][S4], the conservation reason [S7], three Apprentice state prompts [S2]/[S1]/[S5], the coefficient line [S7]); not-claims rows for the other 4 prompts and 19 reason/UI strings, plus a `more atoms` row with the atoms-not-mass note |
| `node --check` golden, analytics-e2e | ok (analytics-e2e test 29 not run here: the tester runs it after the shared build) |

## Golden (QUERY=?low, under the Chromium lock): one full run on this branch, sim still the stub

Full run (`evidence/web/golden-full-stub.txt`): **159/169**. Every existing check passes except
`cast: a real click casts exactly once` (`casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`).
That is the same slow-GPU shape the contract builder recorded on the stub, at about 2.7 fps, and
this branch touches no cast code. `no console errors` passes.

The bench section has 30 checks: **21 pass, 9 fail.** All 9 failures need the sim lane's `learnable`,
`prompt`, `recipe` or `judge`. On the stub the list has no cards and every `try` comes back `parse`.

| spec test | check | on this branch (stub sim) |
|---|---|---|
| 12b | a missing bench.js does not stop the boot | PASS |
| 12 | closed shape after boot | PASS |
| 13 | pad on screen from the start pose (bearing −40.6°, half view 45°) | PASS |
| 13 | standing on the pad lights `near`, never opens, lock kept, not paused (30 frames) | PASS |
| 13 | no collider pushes the player off the pad | PASS |
| 13 | leaving clears `atBench` and `near` | PASS |
| 14 | B opens it from a locked game (lock freed, paused, on top, the four inert) | PASS |
| 14 | Escape closes, unpauses, canvas focus | PASS |
| 14 | a held W is released (pos unchanged 30 frames after close and re-lock) | PASS |
| 15 | opened from the pause card, closing returns to it | PASS |
| 16 | first open asks with three 168 px pictures with SVGs | PASS |
| 16 | choosing 8 to 10 lists the four spells | FAIL (stub: no cards) |
| 16 | reopened on the same load it does not ask | PASS |
| 16 | Change age shows the chooser | PASS |
| 16 | after a reload it asks again; no storage key holds the band | PASS |
| 17, 21, 22, 23 | Sprout list (Water, Frost, CO₂, 120 px) | FAIL (stub) |
| 17, 21, 22, 23 | the rest of the Sprout flow | FAIL (stub: aborts at the first card) |
| 18 | Seedling Fire on the scale | FAIL (stub) |
| 19 | Apprentice typing, key leak, Escape in the box, third-try counts | FAIL (stub) |
| 20 | keyboard only | FAIL (stub) |
| 20 | Escape closes, canvas focus | PASS |
| 24, 25 | sizes, text, contrast | FAIL (stub: needs cards for the build views) |
| 26 | animations once, none under reduced motion, static rings (uses `__bench.fake`) | PASS |
| 27 | one modal gate (`#age` and a stand-in `aria-modal` div) | PASS |
| 28 | `#benchbtn` 72 px, labelled, opens; locked: `.locked`, `pointer-events: none` | PASS |
| 28a | ring after the first cast; controls row `Formula Bench | B` | PASS |
| 28a | learning removes the ring for good | FAIL (stub; the run's catch reports it as "the bench button") |
| 28b | `band seedling` reaches Rust with the bench closed (exact JSON) | PASS |
| 28b | `?band=apprentice` reaches Rust within 2 s of the first state (1716 ms) | PASS |
| 28b | Sprout `pick 1` ignored; `try 1 …` is `not_for_band` | FAIL (pick part passes; stub judge says `parse`) |

Before the full run, the `formulaBench` section ran on its own twice through a scratchpad harness
(not a full golden), using the same function text extracted from `golden.mjs`. The first run, at
20/30, found two test bugs. Test 26 read a live `CSSStyleDeclaration` after the panel was removed,
and `brief(undefined)` threw. It also found one `bench.js` bug: `__bench.fake` did not re-render the
build view for a pick that was already set. The second run was 22/31 with only the stub-dependent
failures left (`evidence/web/formulaBench-only-stub.log`).

**Recorded as expected:** tests 16 (list), 17-25, 28a (ring removal) and 28b (`not_for_band`) pass
only after the sim lane merges. Test 12b and every check above that does not need a card or a
verdict pass now.

## Evidence

`evidence/web/` holds:
- `a-chooser.png`, the chooser;
- `b-list.png`, the list through `fake`, showing all four badge shapes;
- `c-fire.png`, the Seedling Fire scale;
- `d-fizzle.png`, the `unbalanced` fizzle with diff rows;
- `e-done.png`, the done panel with the Learning badge;
- `f-pad.png`, standing on the pad with `#benchbtn.near`;
- `golden-full-stub.txt`, the PASS/FAIL lines of the full run;
- `formulaBench-only-stub.log`.

## Notes for the seam breaker and the tester

- `try` text is `try <slot> <composed text>` with trailing whitespace trimmed. An empty build sends
  `try <slot>`, which Rust reads as `""` and judges `empty`.
- `bench.js` sends `open` again for "Build another" and "All spells", to clear `pick` and `result`.
  It never sends `adept`.
- Test 29 (analytics-e2e section 15) still needs a run in the built site.
