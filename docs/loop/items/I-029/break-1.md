# I-029 break 1

Breaker, wave 13. Built `loop/I-029` at 78a55f9 (local branch; `origin/loop/I-029` did not exist, so the detached worktree was made from the local ref) on top of origin/main d476ac8. Wasm built with `build-web.sh` (gzip 5 105 512 B). I played it as a 6-year-old (Sprout, mouse only, spam and hold) and a 13-year-old (Apprentice, keyboard, trying to skip the build).

## Verdict

BLOCKING: the new `workedCasts` golden section is red at 78a55f9 (11 FAIL), so the item's own acceptance tests do not pass. My probes say the **product** behaves as the spec says in every one of those cases. The failures come from the test harness and from W18's timing at headless frame rates. The builder must fix the harness. No check may be weakened: the critic rules on W18 if its bar cannot be met headless.

## Runs

- `rustc --test worked.rs`: 6/6 pass. `test/bevy-worked.test.ts`: 8/8 pass.
- Golden, only `spellProgress` + `formulaBench` + `workedCasts` (a trimmed copy of `golden.mjs`, `QUERY=?low`, 1280x720): **106/117**.
  - `spellProgress`: all pass. `formulaBench`: all pass (no Bench regression).
  - `workedCasts`: W1-W6, W8, W9, W10, W11, W16, W19, W20, W21 pass. The 11 failures:

| check | log detail | cause (from my probes) |
|---|---|---|
| W7 impact on the clicked target | `hitKind=miss burning=true` | Harness. A real locked `mouse.down` in headless Chromium turns the view about 11 degrees before the click is read (yaw 57.8 -> 68.7, pitch -2.9 -> 8.0, aimObj 0 -> -1). Instant mode misses the same way (probe X1). Opened by `cmd('cast')`, the accepted worked cast lands `out` and puts out the fire (X2, X4). `work_aim` is right; the click aimed at nothing. The golden's own comment at `castAt` warns about this. |
| W12 overlay never calls request/exitPointerLock | `cast ok=false requestPointerLock=0 exitPointerLock=0` | Cascade. After W12's Escape, `ensureLocked` clicks the centre of `#resume`, which sits at y = 922 in a 720 px viewport: the pause card scrolls (`max-height: 92vh`). The click hits nothing, the page stays paused and unlocked, and every later check that needs the lock fails. On a fresh page the lock count is 0 over a full worked cast plus a right-press cancel (Y12). |
| W14 no spell switch while open | `opened=false` | Same cascade. Fresh page: keys 1/3/4, two wheel steps and `select 3` leave spell 1, and `select 3` works after cancel (Y14). |
| W15 reset closes it | `unpaused +99` | Same cascade (the pause card is still up). Fresh page: work null, panel gone and unpaused 4 frames after the `reset` command, counting the evaluate round trip (Y15). |
| W17 B opens the bench and closes the builder | `null fundefined` | Same cascade. Fresh page: the bench opens, `work === null`, casts +0 (Y17). |
| W13 tap opens the builder and lands on the tapped object | `no second fire found under the cursor` | Setup failure in the test. Fresh page in point mode: a tap opens the builder, the real-cursor build casts, `hitKind out`, and the fire goes out (Y13). |
| W18 x5 rows (sprout water, seedling water, seedling fire, apprentice water, apprentice fire) | `castFrames` 3/3/5/2/1; `ttcMs` 12178 / 6728 / 8636 / 15064 / 45768 | Bar not met headless. At 1.5-3 fps, `ttc - inputs x 150 <= 400 ms` cannot hold: each virtual-cursor click costs several evaluate round trips. `castFrames` 3-5 against the 2-frame bar needs a look. It may be the harness's "last input" timestamp or a real extra frame between `work submit` and the cast. Open frame CPU 6.8-11 ms and layout all pass. Copy the WORKEDTTC table into `evidence/ttc.md` once the section is green. |

The builder's note that the full golden "stands" does not reproduce for this section. The tester should rerun after the harness fix.

## Breaking it (what held)

Probes are scratch scripts in the breaker worktree (not committed). Each scenario uses a fresh page unless noted.

- **Stuck in the overlay:** none found.
  - Escape mid-build (Apprentice, mid-typing) and losing the lock mid-build both give the pause card, `work === null` and the panel removed. Resume never reopens it.
  - Every route out works: right press, the X button, Escape, B, reset and a correct build.
  - With `worked.js` missing (W19), nothing ever holds the world.
- **Cast without building:**
  - `castmode instant` and `?cast=instant` without `debug` are ignored, with a warning.
  - `select` and the digit keys are blocked while open.
  - A stale-id `work submit` is ignored.
  - Clicks and `cast` while open give +0.
  - The only skips are by design: `?debug`, a band-less page (W11) and the Sprout rule.
- **Lose XP:**
  - A wrong build gives 0 and changes nothing else (`XP_WORKED_WRONG = 0`).
  - Cancel, Escape, B and reset never touch progress.
  - A helped build earns 0, as the spec says.
- **World holds:**
  - `gameTime`, objects, position and yaw are frozen while open (W4).
  - A key held through the open is released cleanly (W5).
  - Holding the mouse button through the open casts 0. After release and a build, exactly +1 (S4).
- **Spam-click (6-year-old):**
  - 12 rapid presses on Make after a correct Sprout build give exactly 1 cast and +10 XP once. Extra submits are stale.
  - The later presses then open the **next** builder at once (see should-fix 3).
- **Mastered still asking:** no. W9: Mastered Water routes instant, casts on one click, +2 XP, and the hotbar shows a star.
- **Level 1 for Sprout:**
  - Fire is instant at Sprout (W10, F).
  - Water is a 4-click picture build, about 7 s estimated. The level-1 golden needs about 3 casts, so `N1 x estS` is about 21 s, under the 60 s tripwire.
  - My real-click 10-round Sprout run put out 0 fires, but that is the headless press-turn above. Opened by command, every accepted Water cast landed `out`.
  - I-035's level 2 (Frost worked, Fire instant at Sprout) stays winnable under the Sprout rule. I could not play it: I-035 is not merged.
- **Bench and spell progress:** both sections are fully green. B while the builder is open hands over to the bench cleanly.
- **13-year-old typing:**
  - `Hb` in `#bench-input` stays in the box (the bench ignores B in editable fields).
  - Tab reaches Make and scrolls it into view (S7).

## Should-fix (not blocking)

1. **The panel overflows and hides Make at text size 1.5/2 or on short screens.** The layout probe covered 3 viewports x 3 text sizes x 3 bands x 4 spells x New/Learning, before and after 2 wrong builds.
   - At the default text size, 1280x720, Seedling Fire New: scrollHeight 359 against 308, and Make clipped by 2 px (still clickable).
   - At text size 2: the Sprout Make is fully below the fold at 1280x720, 1366x768 and 1024x600. Seedling Fire New needs 922 px in a 308 px box.
   - At 1024x600, text size 1: the Apprentice Make is hidden for New spells.
   - It can be reached with the wheel or Tab. But a mouse-only 5-year-old under lock sees no Make button, and W18's layout bar checks only Learning at text size 1.
   - Fix options: put the actions row first or make it sticky; let the panel grow upward (smaller gap) when the content is tall; or scroll the focused or virtual-cursor element into view.
2. **`worked.js` is 14 594 B unminified.** The spec (B2/B3) promises under 12 KB. Trim it or correct the claim.
3. **Spam-clicking reopens the builder immediately.** After a correct Make, the extra presses of a 6-year-old open the next builder in the same burst. Consider a short grace (for example, ignore `fire` for about 300 ms after an accepted cast, or until the button is released) so the kid sees the spell land.
4. **Apprentice Fire is 24 typed characters on every cast** (estS 9.4 against the 10 s cap). Spell progress cannot reach Mastered without I-030, so a 13-year-old types the whole combustion equation for every Fire cast in a first session. This is a B5 chore risk for the panel and critic (the charged-casts fallback is already noted).
5. **Sprout has no path to XP shown in the field.** The "Learn it at the Formula Bench" line is hidden at Sprout (text), so a Sprout child builds Water forever for 0 XP unless they find the Bench. A picture cue (the bench icon on the New panel) would close that.
6. **The Practised toast is text at Sprout** ("Water: Practised!"), against the spec's own "Sprout sees pictures only".
7. **Harness fixes for the builder** (the blocking item):
   - `ensureLocked` should `scrollIntoView` `#resume` (or click it through the DOM) before clicking.
   - W7 should open the builder in a way that does not turn the view (or assert on `work_aim` against the aim at the press frame), as `castAt` does.
   - W13's target search needs a fix.
   - W18's per-input timing needs to tolerate harness round trips, or the critic rules.
8. **Pre-existing, not I-029:** the pause card's Resume is below the fold at 1280x720 (y = 922) and needs a scroll. Every Escape needs that scroll, worked cast or not. It is worth its own item.
9. **Merge overlap:** `aim_and_cast` reaches 14 parameters here and 15 with I-035's `level`. Both branches edit `control` and `aim_and_cast` in `main.rs`, so expect a manual merge.
