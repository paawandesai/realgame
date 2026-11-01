# I-047 break-2 (Breaker, wave 12)

Tested: `loop/I-047` at 2f5b727, in a fresh detached worktree, with a fresh
`build-web.sh` build (wasm gzip 5 088 565 B). Every browser run held the
chromium lock, `?low`, 1280x720 unless noted. The base comparison swaps in
b2be429's `hud.js` (origin/main's) and changes nothing else (hud.js is copied
into `pkg-out`, not compiled).

**Verdict: do not merge.** T1, T2, T3, T4, T6 and T7 pass, but in this run the
fix makes headless real clicks worse than base, and it drops real first moves
in shapes the spec called "vanishingly unlikely". Golden is red (144/145) on
the check this item exists to fix, and the slow click passed only on its
retry. T5 is not met.

## Full golden on the branch: 144/145, exit 1

Log: `break2-golden.log` (check lines only).

| check | result | detail |
|---|---|---|
| `cast: a real click casts exactly once` | **FAIL** | `casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`. Same failure as the lane's full run: it cast, but the view turned off the fire. It is now 2 of 2 full runs on the fix |
| `streams: a slow click (0.35 s) casts once` | PASS **on the retry** | spans `span 0.302 max_dt 0.225 casts +2` then `span 0.240 max_dt 0.173 casts +1`. The spec needs a first-attempt pass |
| `clock: cast cadence keeps the 0.3 s cooldown in wall time` | PASS | |
| T1, T2, T3, T4, T6, T7 (`lock: ...`) | PASS | all Δ as the lane reported |
| hint gating (`feel: idle hint ...`, `review: Hints off ...`, `streams: pickHint ...`, `cast: the locked hint ...`), point-and-click, drag-to-look, `mouse look turns`, `aim turns while the fire button is held`, progress, `no console errors` | PASS | |

`npm test`: 73/73.

## Probe (`break2-probe.cjs`, log `break2-probe.log`): same script, fix then base

Locked by clicking `#resume`. Each case: `look 0 10`, 5 frames, the action,
10 frames, Δ against the pose before. 1 px = 0.0544° (T3: +300 clamps to
200 = 10.89°).

| case | fix | base |
|---|---|---|
| **A** `mouse.click(x,y)` to a new spot (its move, press and release land together), 6 clicks | **4/6 turned 10.89°/10.89°** (one 10.89/8.16) | 1/6 turned (21.77°, the old bug) |
| **A2** golden's real-click pattern: `mouse.move`, 2 frames, `mouse.click` at the same spot, 4 times | **3/4 turned 10.89°/10.89°** | **0/4 turned** |
| B1 down, up, flick (-150,-100) | PASS (8.16 / 5.44 as expected) | PASS |
| B2 down, flick (+200,+50), up | PASS | PASS |
| **B3** move (3,1), down, up, then (-3,-1) three times | one (-3,-1) lost: 0.16 / 0.05 instead of 0.33 / 0.11 | 0.33 / 0.11 |
| **B4** aim right (40,0), down, up, flick back (-40,0) (net 0) | **-2.18° yaw left over**: the flick back was dropped (matches `cdpPos`) | 0.00 |
| **B5** aim (120,80), press, first held move (-120,-80) (net 0) | **-6.53 / -4.35 left over**: dropped | 0.00 |
| **C** 400x300 window: down, up, genuine flick (-200,-150) | **0.00 / 0.00**: the whole flick dropped (matches the window centre) | 10.89 / 8.16 |
| D age picker (pkg-out served with `rg-analytics` on): shows, closes on a pick | PASS | PASS |

B cases are synthetic events (CDP moves cannot model a real locked mouse,
break-1 finding 1), dispatched in one `page.evaluate`, the same way T6 does.

## Findings

1. **Blocking: the fix breaks golden's real click more often than base.**
   This is the lane's own reported failure (move, press and release in one
   frame: Chromium sends one `-P` for all three, the rule drops it as "bogus",
   and the click's own `+P` turns the view 200 px). The lane saw it 1 time in 4.
   Here it happened 3 times in 4 in A2 and 4 times in 6 in A, against 0/4 and
   1/6 on base, and in the full golden again. The fix swaps one flaky
   real-click failure for a more frequent one. Of the lane's options, option
   1/2 (keep a `-P` while the `+P` it cancels is still owed) addresses exactly
   this. It is a spec decision.
2. **Blocking (T5): the slow click still needed its retry** (`casts +2` on the
   first attempt). The spec's acceptance says first attempt.
3. **Should fix: the `cdpPos` candidate drops real "move and come back"
   aiming.** For a real mouse, `cdpPos` is just the last down-right move. Aim
   right or down-right, click, and correct back: the first correction is
   dropped whenever it is the exact negation within 2 px. For small and medium
   moves this is common, not "well under 1 in 1 000" (B3, B4, B5). Each loss is
   one move (≤ 200 px, up to 10.89°). B5 is the cast-and-correct pattern while
   holding fire. Restrict `cdpPos` to the CDP shape (e.g. also require that
   the matching `+c` was followed by a kept `-c`, which is the same state the
   finding 1 fix needs), or drop it if pairing makes it redundant.
4. **Should fix: the window-centre candidate eats a full-speed flick in small
   windows.** In a 400x300 window the centre is (200,150), and the ±200 clamp
   makes a fast up-left flick read (-200,-150) or close to it, so the first
   flick after a click counts for nothing (C). Small iframes and windows
   embedded in a classroom portal hit this. The candidate exists for T2's
   synthetic case: skip it when it is within 2 px of the clamp on either axis,
   or use it only when no `edgePos` or `cdpPos` is known.

## Unaffected (checked)

- Unlocked point-and-click and drag-to-look: `movement()` only calls
  `dropBogus` under lock, and golden passes them.
- Touch: `pointerType !== 'mouse'` goes to `tap` and never locks. Keyboard and
  pad look are in Rust, not touched.
- Age picker: shows and closes on a pick on the fix and on base (probe D).
- Hint gating: all golden hint checks pass.
- No native debug dir was created. `bench.js`, `bench.rs`, `style.css` and
  golden's `formulaBench` were not touched.
