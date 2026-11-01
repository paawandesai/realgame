# I-029 round 2 (builder, wave 13)

This round follows break 1 and the arbiter's round-2 ruling (`critique.md`, "Round 2 ruling"; 6fe78cd). Branch `loop/I-029`.

## Result

- **Full golden** (`BASE_URL= QUERY=?low`): **262/263**. The one failure is `cast: a real click casts exactly once` (`casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`), which is on the list of known wall-time flakes. Every other check passed:

  | section | checks |
  |---|---|
  | workedCasts | 55 |
  | frozenGate | 15 |
  | bench | 54 |
  | progress | 12 |

  The log ends with `no console errors`.
- **Sections only** (`ONLY=workedCasts,frozenGate,formulaBench`): 123/125. The two failures were W13 and the new R2b check, both from harness setup. They were fixed in 4e6045a. A rerun of setup, W13 and R2b alone passed 3/3. The full run above includes those fixes.
- `cargo fmt --check` passes in the root and in the client.
- `cargo clippy -D warnings` passes for the root (`--all-targets`) and for the wasm client (both dev, as in CI, and release).
- `cargo test --locked` passes: 217.
- `rustc --edition 2021 --test src/chem.rs` passes 24/24. `rustc --test src/worked.rs` passes 7/7, including the new `grace_after_accept`.
- `npm run typecheck` is clean. `npm test` passes 96/96, including `test/bevy-worked.test.ts` at 11/11.
- `build-web.sh` builds the wasm at 5 126 268 B gzip. `worked.js` is now 11 957 B.

## Step (a): merge origin/main (I-035 Frozen Gate)

The merge is 1a91d20. Two files had conflicts:
- **`main.rs`.** All four conflicted hunks keep both sides:
  - imports: `realgame_sim::bench` and I-035's `world::{ArchDoor, ArchMats, …, LevelRoot}`;
  - the `Level`, `BenchPad` and `BENCH_ON` items;
  - the `Game` tail: `work`, `work_aim` and `cap_icon`;
  - the `aim_and_cast` parameters: `wk: WorkCtx` and `level: Res<Level>`. That makes 15, under Bevy's 16.

  Merged cleanly:
  - I-035's level switching (`levels`, `reset(.., level)`) and the exit-arch finder;
  - I-029's routes.

  `reset` rebuilds `Game`, so a level switch also closes an open builder. `paused` is recomputed every frame from `web.paused || work.is_open()`.
- **`golden.mjs`.** `workedCasts` and then `frozenGate` are both called, before `no console errors`. Both functions are kept.

`ONLY=` now takes a comma-separated list of `frozenGate`, `spellProgress`, `formulaBench` and `workedCasts`. Before this it took `frozenGate` only.

**Handoff check** (spec, "Overlap and merge order"): `worked: handoff: level 2 at Sprout (castmode worked): Fire routes instant, three instant Fire casts melt the gate, and the level is won`. It loads `?level=2&band=sprout` with `castmode worked`, checks that `routes[0] === 'instant'`, and makes three Fire casts at the gate. Each cast must land within 2 frames with `work` null on every frame. Then the gate must be gone, and walking to the lantern must win the level. It passed in both runs.

## Step (b): harness fixes (listed, as the ruling asks)

All of these are in `tests/e2e/golden.mjs`, `workedCasts`. None weakens an existing check.
1. **`ensureLocked`** scrolls `#resume` into view before it reads the button's rect and clicks it. In break 1 the button was at y = 922 in a 720 px viewport, and that caused the W12, W14, W15 and W17 cascade. The product fix (8, below) does the same thing on its own.
2. **A keydown recorder that runs first.** `worked.js` stops keydowns in its own window-capture listener, which registers at boot, before `record()`. So an `addInitScript` adds a keydown recorder ahead of it on every page loaded after the section starts. This lets W18 anchor on the Apprentice Enter.
   - `record()` adds its own keydown listener only on pages without the init script: W1, W11 and W19, which use new contexts.
   - The init script stays on the main page for the rest of the run. It only appends to `window.__wcPress`.
3. **Recorder rows carry `aimObj`.** This is condition (a) of the W7 ruling.
4. **W7 (arbiter r2).**
   - The builder opens with `cmd('cast')`, not a real locked press.
   - A new check, `worked: W7 (arbiter r2) the builder opened by cast shows on a frame aimed at the target`, requires the first recorder row where `#worked` shows to have `aimObj === target`.
   - The Make press is still a real virtual-cursor press (`vclick`), and the 2-frame and 30-frame checks are anchored on it.
   - W12's lock count still opens with a real `lclick`.

   The reason for opening by command: a real locked press in headless Chromium turns the view about 11 degrees before the click is read. That tests the browser, not the game. W3, W5, W12, W18 and `cast: a real click casts exactly once` still cover real presses.
5. **W13 target search** (setup only):
   - The three fires are at (-13.5, -3), (13.5, -3) and (0, -5). Seen from z = 5.5 they are about 58 degrees apart, so no two fit in the 90-degree view. That was break 1's "no second fire found". My probe confirmed it: facing each fire, no other burning bush ever came under the cursor.
   - The search now stands at z = 18 first, where they are about 33 degrees apart, and falls back to z = 5.5.
   - Each probe waits 2 game frames for `hover`, instead of a 450 ms wall-clock wait.
   - The check itself is unchanged.
6. **W18 (arbiter r2).**
   - `castFrames` counts from the Make press to the first row with `casts` +1. The Make press is the last capture-phase left `pointerdown`, or `keydown` Enter, at or before that row.
   - A new field, `sameFrame`, is ANDed into the row check: the cast row must be the first row with `work === null` after the press.
   - The `ttcMs - inputs x PACE <= 400` bar is removed. `ttcMs` is still printed, and the table is in `evidence/ttc.md`.
   - The `lastInput` round trip before each input is gone.
   - The bars `openFrames <= 2`, CPU <= 50 ms, layout, `estS <= 10`, "Sprout Water is 4 inputs" and `N1 x estS <= 60` are unchanged.

   Every row read `castFrames` 1 and `sameFrame` true.
7. **Grace waits.** `workedCast()` and W10 wait 0.5 game seconds after a cast before the next deliberate click (`afterGrace`). Without this, the new 0.4 s grace (should-fix 3) would swallow that click.
8. **New checks:**
   - **R2a** (should-fix 3): a real click right after an accepted Make casts nothing more and opens no builder for 10 frames. After the pause, a click opens the next builder.
   - **R2b** (should-fix 1): Make is inside the panel's visible box, inside the viewport, and on top at its centre (`elementFromPoint`), and the virtual cursor's press reaches the game. It covers six cases:

     | band | spell | viewport | text size |
     |---|---|---|---|
     | Sprout | Water | 1280x720 | 2 |
     | Sprout | Water | 1024x600 | 2 |
     | Apprentice | Water New | 1024x600 | 1 |
     | Apprentice | Fire New | 1024x600 | 1 |
     | Seedling | Fire New | 1280x720 | 1 |
     | Seedling | Fire New | 1280x720 | 2 |

     The text size is set after the lock. At 200 % the hotbar wraps over the canvas centre, where `ensureLocked` clicks. That hotbar overlap is pre-existing and is not this item's.
   - **Handoff:** see step (a).

Node test (arbiter r2, item 4): `14: Make and Enter send work submit synchronously`.
- It replaces `setTimeout`, `setInterval`, `queueMicrotask` and `requestAnimationFrame` with recorders.
- A virtual-cursor press on Make must have sent `work submit 3 H2O(l)` by the time `press` returns. So must Enter in the Apprentice input, by the time `key` returns.
- Nothing may be deferred.

## Step (c): should-fixes

1. **Make always visible.**
   - `#worked .bench-actions` is `position: sticky; bottom: -10px` with the panel's background, so Make stays at the panel's bottom edge when the content is taller than the box.
   - In `worked.js`, pushing the virtual cursor past the panel's top or bottom scrolls the panel. A mouse-only child under lock can still reach every piece.
   - Evidence, measured by a probe:

     | case | before | after |
     |---|---|---|
     | Sprout, text size 2, 1280x720 | Make at y 688-752 in a panel ending at 712 (hidden) | 640-704 (in view) |
     | Apprentice New, 1024x600 | Make at 547-603 in a panel ending at 592 (hidden) | 528-584 (in view) |

     Screenshots: `evidence/r2-before-*.png` and `evidence/r2-after-*.png`. The "before" pages use the merged commit's `hud.js`, `style.css` and `worked.js` on the same wasm.
   - Node test 16 covers the edge scroll. R2b covers the layout headless.
2. **`worked.js` is 11 957 B** (was 14 594 B). The size came from shorter comments, fewer JSDoc typedefs and an `el(tag, cls, text, attrs)` helper. The behaviour is unchanged, and node tests 8-13 still pass.
3. **Spam-click pause.** `worked::Grace` (`GRACE_SECS = 0.4`):
   - After an accepted worked cast, `fire` and `tap` commands are dropped for 0.4 game seconds, and held fire does not repeat.
   - A press that began during the pause stays ignored while it is held. Clicks are still counted in `clicks`.
   - The cast queue is cleared on accept.
   - Tests: the Rust unit test `grace_after_accept` and golden R2a.
   - Nothing changes for instant casts.
4. Apprentice Fire, 24 characters: not changed. It is left to the critic, as break 1 said.
5. **Bench cue at Sprout.** A New panel at Sprout shows a picture of the Formula Bench (a table with a flask, drawn in CSS, `role="img"`, `aria-label="Formula Bench"`) in the header, in place of the text line. Node test 15.
6. **The Practised toast at Sprout shows pictures only:** the spell's icon and ◑. The words "Water: Practised!" move to its `aria-label`. Other bands are unchanged, and W8 still reads "Practised". Node test 15.
7. Harness: step (b).
8. **The pause card's Resume** is scrolled into view whenever the card shows: `showPause` sets the card's `scrollTop = 0`, then calls `#resume.scrollIntoView({ block: 'nearest' })`. The recent-messages list is capped at `max-height: 24vh` with its own scroll. `hud.js`'s pointer-lock block (I-047) is not touched. The change is in `showPause`.
9. Merge: step (a).

## Files

`crates/bevy-client/src/main.rs`, `crates/bevy-client/src/worked.rs`, `crates/bevy-client/web/worked.js`, `crates/bevy-client/web/style.css`, `crates/bevy-client/web/hud.js` (`showPause` only), `test/bevy-worked.test.ts`, `tests/e2e/golden.mjs`, `docs/loop/items/I-029/evidence/{ttc.md, r2-*.png}` and this file. The untracked `tests/e2e/wc-only*.tmp.mjs` files are deleted, and so is `/home/claude/bevy-target/debug`, which the dev wasm clippy run created.

## Open for the critic

- Should-fix 4: Apprentice Fire is 24 typed characters per cast, with estS 9.4 against the 10 s cap.
- At text size 200 % the hotbar wraps over the middle of the screen. This is pre-existing and not I-029's, and it is worth its own item.
- A single real extra frame between `work submit` and the cast would still pass the W18 bar of 2. The ruling accepts this.
