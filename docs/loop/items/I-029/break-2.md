# I-029 break 2

Breaker, wave 13, round 2. I built `origin/loop/I-029` at 877b7e8 in a fresh detached worktree (`/tmp/claude-0/brk13b-I-029`, now removed). It contains origin/main through cdd719e (I-035 Frozen Gate). I did not write this code. `build-web.sh` built the wasm at 5 126 333 B gzip, and `worked.js` is 11 957 B. I played it as a 5-year-old (Sprout, mouse only, spamming and holding the button) and as a 13-year-old (Apprentice, keyboard, trying to skip the build).

## Verdict

**BLOCKING: none.** The item's own sections are green and so is every section near it. I found no way to get stuck, cast without building, lose XP or double-cast. Levels 1 and 2 can be won in worked mode at every band I tried. Every should-fix from break 1 is done or has a reason given. What is left is small and goes to the critic (below).

## Runs

| run | result |
|---|---|
| `ONLY=workedCasts,frozenGate,formulaBench,spellProgress` (`QUERY=?low`, 1280x720) | **137/137**: workedCasts 55, frozenGate 15, bench 54, progress 12, plus `no console errors` |
| `rustc --edition 2021 --test crates/bevy-client/src/chem.rs` | 24/24 |
| `rustc --test crates/bevy-client/src/worked.rs` | 7/7 |
| `test/bevy-worked.test.ts` | 11/11 |

I did not rerun the full golden. The builder's 262/263 stands, with one failure on the known flake list (`cast: a real click casts exactly once`). I found nothing on this branch that touches the other sections.

Detail lines worth keeping:
- W18: every row reads `openFrames` 1, `castFrames` 1 and `sameFrame` true. `estS` is 7 / 4.6 / 3.7 / 3.45 / 9.4.
- N1 = 3, which gives 21 s at Sprout and 13.8 s at Seedling.
- R2b: all 6 cases ok, and Make was pressed in each.
- The handoff check (level 2 at Sprout) is won.

## Harness review (do the checks still test what they say?)

The round-2 harness changes match the r2 ruling. None of them weakens a check.
- **`ensureLocked` scrolls `#resume` into view.** The product now does the same scroll itself (`showPause`). My pause probe pressed Escape without the harness scroll at 4 viewports x 3 text sizes. `#resume` was always in view and on top: at 1280x720 its bottom is at y 691-692, and at 1024x600 at y 576.
- **W7 opens by `cmd('cast')`.** The new check (a), "shown on a frame aimed at the target", passes with `aimObj 0` on the first row where the builder shows. The Make press is still a real virtual-cursor press (`press f211 -> cast f212`), and the impact puts out the fire for +10 XP.
- **W18 counts game frames from the Make press, and the cast row must also be the accept row.** I read the code: `makeP` is the last left `pointerdown` or Enter at or before the cast row. A deferred cast would fail `sameFrame`.
- **R2a really tests the pause** and does not pass by accident. I worried that its second click might still land on the open overlay, where it would be a stale `work submit` and would pass even without the grace. So I replayed R2a twice with press-time overlay state. Both times the second press came 1 frame after the cast with the overlay already gone (`press f34 overlayOpen=false`, `press f85 overlayOpen=false`). Casts +1, and no builder opened.
- **The W13 search from z = 18** is setup only. The check itself is unchanged.
- **R2b** sets `--ui` after the lock, because at 200 % the hotbar covers the canvas centre. I confirmed this is real and that it is not this item's: with the text size saved at 2 before the page loads, the centre click hits the hotbar and the page never locks. See Pre-existing.

## Breaking it

All probes ran in a throwaway copy of `golden.mjs`, inside `workedCasts`, using its own helpers. Each one used a fresh page.

**Make hidden** (the full matrix: 3 bands x 4 spells x 4 viewports (1280x720, 1024x600, 1366x768, 1024x768) x text size 1 / 1.5 / 2, New and after 2 wrong builds):
- In the real flow, where the text size is set before the builder opens (it can only be changed on the pause card, and opening the pause card cancels the builder), Make was in the panel, in the viewport and on top in every case.
- Spot checks at text size 2, with screenshots:

  | case | Make (px) | panel `scrollTop` |
  |---|---|---|
  | Seedling Water, 1280x720 | 638-694 | 213 |
  | Seedling Fire, 1280x720 | 648-704 | 600 |
  | Apprentice Water, 1280x720 | 610-666 | 238 |
  | Seedling Water, 1024x600 | 518-574 | 273 |

- But Make is in view because the panel opens scrolled down (focus goes to the first piece), not because of the sticky row. See should-fix 1.

**Stuck in the overlay:** none. Right press, ✕, Escape, B and a correct build all close it, and the sections cover each of these. Holding the left button down on Make for 4 s gives casts +1, and the only builder ids seen are `[1, null]`: nothing reopens and nothing repeats, because a press on the overlay never sets `input.fire`.

**Double casts / spam (6-year-old):**
- Make followed by 12 rapid clicks (30 ms down, 30 ms up) gives casts 0 -> 1.
- The next builder opened 0.53 game seconds after the cast. That is after the 0.4 s grace, as designed.
- A click 0.5 game seconds after a cast opens the next builder. That is correct: the grace has ended.

**Cast without building / XP loss:** nothing new. Break 1's results still hold, and W6, W8, W12, W14 and W15 all pass.

**Level 1 in worked mode, Water only, real builds:**

| band | outcome | casts | builds | wall time (headless) |
|---|---|---|---|---|
| Sprout | won | 3 | 3 | 96 s |
| Seedling | won | 3 | 3 | 93 s |

**Level 2 (Frozen Gate) in worked mode:**

| band | routes | result |
|---|---|---|
| Sprout | Fire instant | won (handoff check) |
| Seedling | all worked | 3 worked Fire builds melt the gate, won |
| Apprentice | all worked | 3 typed `CH4 + 2O2 -> CO2 + 2H2O` builds melt the gate, won |

**Merge with I-035:** `frozenGate` is 15/15 on the merged branch, and the hud.js diff touches only `showPause`. The pointer-lock block (I-047) is not touched.

## Should-fix (not blocking)

1. **The sticky Make row does not do what its comment says** (`style.css`: "Make never leaves the panel's view").
   - `.bench-actions` is sticky inside `.bench-build`, so it can never rise above the top of `.bench-build`. At text size 2, the head, the "New spell" line and the Copy line above `.bench-build` are taller than the panel.
   - If the panel is scrolled to the top, Make is clipped at the bottom:

     | case | Make (px) | panel bottom |
     |---|---|---|
     | Seedling Fire, 1280x720, `scrollTop` 36 | 692-748 | 712 |
     | Seedling Water, 1024x600 | 610-666 | 592 |
     | Apprentice Water, 1280x720, `scrollTop` 0 | 674-730 | 712 |

   - This happens when a child pushes the virtual cursor up to read the Copy line, or when the browser is zoomed while the builder is open. Moving the cursor back down to Make scrolls the panel again (edge scroll), so it can be recovered.
   - Fix: make the panel a flex column with the actions row outside the scrolling area, or put the sticky row directly under `#worked`.
2. **At text size 2 the panel opens scrolled past its head.**
   - New spells at Seedling and Apprentice: the name, the ✕ button and the "Copy: …" line, which is the answer a New spell gives, start above the visible area. In the Seedling Fire screenshot only part of the scale shows.
   - They can be reached by pushing the cursor up, but a child may never see the formula they are meant to copy.
   - Fix: keep the Copy line next to the pieces, or scroll the first piece into view with `block: 'nearest'` and keep the Copy line just above it.
3. **Apprentice Fire is still 24 typed characters per cast** (`estS` 9.4 against the 10 s cap). Same as break 1, item 4. It is for the critic.

## Pre-existing (not I-029)

- **At text size 200 % the hotbar wraps over the centre of the screen.** A click in the middle hits the hotbar, so a player who saved text size 2 cannot take the pointer lock by clicking the centre. Seen headless at 1280x720. The builder noted this too, and it is worth its own item.

## Cleanup

The probe file (`tests/e2e/wc-probe.tmp.mjs`), the `node_modules` symlink and the breaker worktree are removed. I created no `/home/claude/bevy-target/debug`.
