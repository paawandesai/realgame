# I-020: The first 30 seconds: a walk-and-cast coach, a safe start, and a click that hits the fire

**Status: blocked on merge order, not on missing work.** I-007 (#24), I-005 (#25) and I-019 (#29) have merged. Dispatch this item after **I-022** (building) and **I-025** (being specced) have merged, because both edit the exact lines this item hooks into (see **After** and "Overlap with I-022, I-024, I-025" under Contract). Revision 2 follows the first critique; the roadmap-v2 refresh is described under "Refresh (roadmap v2)" at the end; **revision 3** follows the roadmap-v2 critique (see `critique.md`, "Planner response", and "Revision 3" at the end). The reading-level captions were split out to **I-021**, and revision 3 also moves `event_lines` and the caption snapshot there.

**Moves:** B4 (every age can play), the move part. Today 0 of 5 personas pass it. The 6yo, 9yo and teacher have `time_to_move_s: null`, the 13yo took 11.8 s and qa took 19.5 s. All 5 already land a cast within 30 s (the 6yo at 21.9 s), and that must hold.
**Expected at the first paired panel after merge** (same seed and query, before and after I-020; no persona gets `?band`, so every persona plays the default real-player path). **These predictions hold only for panels whose artifact comes before I-027, I-028 and I-029**, where a cast is instant. After I-029 a first Water cast becomes a worked cast with 3-10 s of builder time, so B4's "lands a cast within 30 s" is measured on that route and belongs to I-029. A later panel's cast-clock regression is not evidence against this item.
- At least 4 of 5 personas move within 10 s, and the 6yo, 9yo and teacher all do. The target is 5 of 5; qa at 4x throttle is the risk.
- All 5 still land a cast within 30 s, and the 6yo does it within 20 s.
- No persona reports "I clicked the fire and nothing happened" (the 6yo's control blocker).
- Every rise in `burning` in the panel transcripts is followed by a spread line in `history` (6yo turn 28 and 9yo turn 29 saw the fires go from 1 to 2 with no caption).

B4 will still fail after this item: clarity ≥ 8 and zero text blockers need I-021, and zero control blockers also need I-025. **No bar is reported as moved at merge.** The movement is judged at the next panel run. B1 (test 12), B2/B3 (test 13) and B6 (test 11) must not regress. Roadmap v2 (target.md "Hard rule" and "Core loop"): this item is the first-person WASD-and-look onboarding the hard rule asks for; it adds no crafting, mastery or worked-cast behaviour (B9, B10, B11 are untouched).

**based_on:** origin/main `c4832d2e86c2408443124960b130fc6208a89aab` (2026-10-30, roadmap v2, #31). `git diff a0e2ecb..c4832d2 -- crates tests functions test` is empty, so every code citation below is the code I-019 left at a0e2ecb. The code was read with `git show c4832d2:<path>`: `crates/bevy-client/src/{main.rs,player.rs,chem.rs}`, `crates/bevy-client/web/{hud.js,boot.js,index.html}`, `crates/bevy-client/build-web.sh`, `scripts/build-site.sh`, `tests/e2e/{golden.mjs,perf.mjs,analytics-e2e.mjs}`, `content/science-sources.md`, and `docs/loop/{target.md,bars.md,areas.md,gameplay-standards.md,backlog.md}`. The panel evidence is `.loop-artifacts/panel/2026-10-30/*` at artifact 3dbd332 (nightly-1); no panel has run since. The I-022, I-024 and I-025 facts come from their specs in `docs/loop/items/`. Line numbers: `main.rs` is `crates/bevy-client/src/main.rs`, `hud.js` is `crates/bevy-client/web/hud.js`, `golden` is `tests/e2e/golden.mjs`.

**After:** I-022 and I-025 merged. Builders branch from origin/main after the later of the two. Before dispatch the planner:
1. moves `based_on` to that commit and re-checks the stale-check list below, and re-reads the `setHint`/`hintAllowed` block, `pickHint` and its call in `frame()`, the golden key-order lines, and the `projectiles` landing block for their new line numbers;
2. records in `round-1.md`'s header golden's check count on that commit (the critique read 98 at a0e2ecb) and the exact key-list order on origin/main, which the contract lane extends (test 12).

There is no planner-written input file. Revision 3 moved the caption snapshot to I-021.

If I-025 is delayed and this item goes first, I-025 rebases onto it instead: its `pickHint` call goes inside this item's `coachText || …` expression, and its `I025_KEYS` go before `I020_KEYS` only if it merges first (keys are appended in merge order). The planner states which order applies when it dispatches.

**Stale check.** The planner refreshes this spec before building if any of these differ:
- I-019's hint priority after I-025 is not banner > idle > note > locked > start, the hint is not chosen by one expression passed to `setHint` in `frame()`, or `setHint` is not the only writer of `#hint`;
- I-022's `hintAllowed(text)` gate is not `settings.hints || !text || text === note` inside `setHint`;
- `Game.spell = 1` at boot, the `firesLeft` counter, `#firefinder`, or the rule that new DOM is built from `hud.js`;
- the fire counter or any HUD element outside `#caption` shows a countdown, a timer or a "spreading" cue that does not come from `burning` changing (while the world is held, such a cue would warn about a paused threat);
- I-007's `--ui` text scale, `IDLE_HINT` / `nudge`, or the `cast_queue` path;
- the `__gameState` fields `pos`, `casts`, `impacts`, `caption`, `captionId`, `hover`, `aimObj`, `spell`, `locked`, `paused`, `outcome` or `tick`, or golden's key-order check (`golden:1045-1047`) stops being an exact list;
- `perf.mjs` `pageWorkload` no longer begins with a `tp` more than 0.4 m from spawn followed by a cast that lands on a named object (test 13 relies on this to end the hold during warm-up);
- `golden.mjs` or `analytics-e2e.mjs` start waiting in real time (not `tick N`) for a spread, a lose or any chem change on a page before its first landed cast (test 12 relies on this).

## Facts from c4832d2 this spec relies on

- **Nothing asks the player to walk.** The player spawns at `(0, 0, 18)` (`Player::default`, `player.rs:39-47`). Every burning bush can be hit from spawn: `pick` (`main.rs:899`) casts a 120 m ray, and the assist cone narrows to nothing at 60 m. The unlocked start hint is `START_HINT` = `Click to look around. Esc shows the controls and point-and-click mode.` (`hud.js:33`); the boot caption is `Save the Greenhouse! Put out 3 fires before they spread. Water is ready: aim at a fire and click.` (`main.rs:636-638`). W A S D appears only in the pause table. All three personas with `time_to_move: null` won without pressing a movement key.
- **The panel's move rule** (`tests/e2e/panel/measure.mjs`, unchanged since 3dbd332): a movement-key act that moves the player at least 0.05 m, or any act that leaves the player more than 0.5 m from spawn.
- **Keyboard focus.** Bevy/winit reads keys on the canvas (`#game`, `tabindex="0"`). `hud.js` focuses it only in `lock()` (`hud.js:192`), on a canvas `pointerdown` (`hud.js:364`), on Point-and-click (`hud.js:431`) and on a hotbar press (`hud.js:434`). Nothing focuses it before the first click, so a W pressed before any click may not reach the game. golden focuses it with `page.evaluate` before pressing `w` (`golden:145`).
- **A refused lock makes the first click do nothing, and later clicks go to the crosshair.**
  - A canvas left `pointerdown` while unlocked (not point mode) calls `lock()` (`hud.js:363-376`). `pointerup` (`hud.js:377-394`) then does nothing unless `lockDenied` is already true; if so it sends `cmd('cast')`, or sets `castIfRefused` when a request is still `pending` (`hud.js:167-168`), and the refusal branch of `lock()` then sends `cmd('cast')` (`hud.js:198-199`).
  - On the very first refusal `lockDenied` is set only inside that refusal branch (`hud.js:206-208`), so the first click is dropped whenever the refusal settles after `pointerup`. Every cast from a refused-lock click goes to the **crosshair**, never to the cursor. The 6yo clicked the fire at (308, 340): nothing happened, and later clicks went to the crosshair.
- **Headless Chromium grants the lock.** golden's check `a click over a target locks the pointer and does not cast` (`golden:118`) passes on the e2e runner.
- **Spread is often not captioned.** In `react()` (`main.rs:1292-1349`), an uncaptioned `Ignited` (spread from a neighbour) is said only when `game.caption_age > 1.5` (`main.rs:1320-1339`). Every cast resets `caption_age` through `game.say(spell.cast_caption())` (`main.rs:1098`), so while the player is casting, spread is silent. `Reseeded` is always said (`main.rs:1316-1319`).
- **The world clock** is `chem_tick` in `FixedUpdate` (`main.rs:344`, body `main.rs:654-661`). It returns early only while paused or after the outcome is decided. The debug `tick N` command (`main.rs:782-791`) calls `chem.world.step` directly. `publish` reads the outcome from `w.outcome()` (`main.rs:1562`). Spell results (`chem.world.apply`, `main.rs:1271-1287`) do not go through `chem_tick`.
- **Commands.** `control()` (`main.rs:666-876`) runs `__game.cmd` commands; `look`, `turn`, `face`, `tp`, `move` and `tick` need `?debug` (`main.rs:751-758`). `reset` is `main.rs:792`, `reset()` is `main.rs:878-893`. Unknown commands fall through `_ => {}`; afterwards only `tap…`, `cast` and `fire` are kept for `aim_and_cast` (`main.rs:816-819`). Movement runs at `main.rs:822-864`; the camera block is `main.rs:865-874`.
- **Landing.** In `projectiles` (`main.rs:1164-1290`), a projectile that arrives increments `impacts` (`main.rs:1236`); when it carries `obj: Some(i)` (`main.rs:1271`) the spell is applied.
- **State and golden's key check.** The `__gameState` format string is `main.rs:1580-1581`, its last argument `fx.water_live` at `main.rs:1634`. golden pins the keys exactly: `keys.length === BASE_KEYS.length + 2 + I019_KEYS.length` and the order `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS]` (`golden:1045-1047`). Any new key fails `clock: state JSON keeps the base keys in order…` (`golden:1214`) unless that list is extended. I-025 extends it with `I025_KEYS = ['fireInView']`.
- **golden.mjs** (read for test 12): `three fires at start` (`golden:95`), a click that locks (`golden:118`), mouse look, `WASD moves` (more than 1.5 m, `golden:151`), jump, spell keys, and `left click casts while locked` at pitch 25° (`golden:171`; it is not aimed at an object). Then `fire spreads to neighbours` with `tick 2400` (`golden:193-195`), the lose with `tick 9000`, then `reset`. The first cast that lands on an object is the Fire cast on the dry hedge (`golden:236`); every later check on that page, including `feelAndStandards`, `castingYouCanSee` and I-022's `reviewFollowups`, runs after it. The second page in `feelAndStandards` (`golden:940-967`) checks only `sens`, sway, the crosshair animation and `casts > 0`. No step waits in real time for chem changes before a landed cast.
- **analytics-e2e.mjs** teleports with `tp 0 5.5` and lands Water casts on each fire before waiting for the win (`analytics-e2e.mjs:214-224`).
- **perf.mjs** (read for test 13): `pageWorkload` (`perf.mjs:505`) starts with `tp 0 5.5`, `face <dry hedge>`, `select 1`, `cast` (`perf.mjs:532-533`): 12.5 m from spawn and a cast that lands on an object, so the hold ends in warm-up, before sampling. (With `PERF_NO_CAST=1` it would not end; test 13 does not use that mode.) I-024 adds fields to the run JSON and a `perf-pairs.mjs` driver but does not change `pageWorkload`.
- **Web build.** `build-web.sh:21` copies `index.html style.css boot.js hud.js favicon.svg` into `pkg-out`; `hud.js` imports nothing today. `scripts/build-site.sh:28` copies all of `pkg-out`, so a module added to the `cp` line reaches the site too.
- **The science lint** (`test/science-sources.test.ts`, unchanged since 3dbd332) scans the `.rs` files under `crates/bevy-client/src` and the `.html` and `.js` files under `crates/bevy-client/web`. It flags any sentence-shaped string, formula or temperature that has no row in `content/science-sources.md`. Game instructions go in its **Not claims** table (for example `Aim at a fire and click.` and `IDLE_HINT`); science claims get a `C-` row. The last `C-` id at c4832d2 is C-039.

## Relied on from merged and in-flight items (their contracts; not rebuilt here)

- **I-019 (merged).** Water is selected at boot. The fire counter (`firesLeft`, `#obj-line`, `.fire-pip`), the nearest-fire arrow (`#firefinder`), the spell streams, `#crosshair.fire`, `AIM_HINT`, `P` and `#pausebtn`, the latched click (`fire` command), the boot caption, and `__game.hudState()` all belong to I-019. New DOM is built from `hud.js` with `createElement`.
- **I-007 (merged).** The settings block (including `hints`), the `--ui` text scale, `IDLE_HINT` with the Water-slot `nudge`, the hit markers and the cast cooldown.
- **I-022 (building).** `setHint(text)` writes `hintAllowed(text) ? text : ''`, with `hintAllowed = (text) => settings.hints || !text || text === note`. Every hint that reaches `#hint` through `setHint` is hidden when Hints is off, except notes.
- **I-025 (being specced).** The hint in `frame()` becomes `setHint(pickHint({ playing, idle, note, locked, extinguished, casts }))`, order banner > idle > note > locked > start, unchanged from today. New state key `fireInView`, appended after `fxWater`, and `I025_KEYS` in golden's key check.

**This item does not change** any of the following: casting visuals, the fire counter, the arrow, the crosshair, the cast and click path while locked, any caption text, `pickHint`, `setHint`, `hintAllowed`, the I-007 hints, or settings.

## Problem

In the nightly panel, 3 of 5 personas (6yo, 9yo, teacher) never pressed a movement key, and the other 2 took 11.8 s and 19.5 s. Nothing on screen asks the player to walk, and the quest can be won from where the player starts. The 6yo's first click on a fire did nothing, because the lock was refused and the click was dropped. Later clicks went to the crosshair, not to the fire under the cursor. When fire spread (1 burning became 2), the caption that explains it was suppressed, because the player had just cast. Fires also spread while a new player is still learning the controls, so the first area is not a safe space (GS-23).

## Smallest version

### 1. The coach beat (Rust: `crates/bevy-client/src/onboard.rs`, new, Bevy-free)

- `pub enum Step { Walk, Cast, Done }`, and `pub struct Onboard { spawn: [f32; 2], moved: bool, landed: bool }` with `pub const MOVE_M: f32 = 0.4`.
- `Onboard::new(spawn_xz)` starts with `moved = false` and `landed = false`.
- `fn observe_pos(&mut self, xz: [f32; 2])` sets `moved` once the horizontal distance from `spawn` is at least `MOVE_M`. It never unsets it.
- `fn observe_landed(&mut self)` sets `landed`.
- `fn step(&self) -> Step` returns `Walk` if `!moved`, else `Cast` if `!landed`, else `Done`. The order is fixed: a player who casts first is still asked to walk.
- `fn holds_world(&self) -> bool` is `step() != Done`.
- `fn restart(&mut self)` clears both flags.

**Wiring in `main.rs`:**
- `mod onboard;` next to the other `mod` lines (`main.rs:15-23`). An `Onboard` resource is inserted next to `.init_resource::<Player>()` (`main.rs:341`), with `Player::default().pos` as `[x, z]`.
- `control()` calls `observe_pos([player.pos.x, player.pos.z])` every frame, after the movement block (after `main.rs:864`) and before the camera block (`main.rs:865`). That point is after any `tp`/`move` command too.
- `projectiles` calls `observe_landed()` inside the `if let Some(i) = p.obj` block (`main.rs:1271`), whatever the spell or result.
- **World hold.** `chem_tick` (`main.rs:654`) also returns early while `onboard.holds_world()`. Fires, heat spread, greenhouse heating and drying all wait until the player has walked and landed one cast. A new player therefore cannot lose while learning (GS-23, GS-25). Spell results still apply while held (they do not go through `chem_tick`). The debug `tick N` command still steps the world while held, so golden's spread and lose checks keep working.
- **The hold is silent on purpose.** Nothing tells the player that the world is waiting. A player who never walks keeps a frozen world, which is safe, and the coach is already asking for the one action that ends the hold. No countdown or "spreading" cue may show while held (stale check, test 8).
- **No reset of the beat.** `reset()` (R, `cmd reset`, and I-005's post-pick reset) does **not** touch `Onboard`. The beat runs once per page load. A reset before the first walk therefore leaves the world held.
- **Replay.** The new command `onboard restart`, allowed without `?debug`, is a new arm in the `control()` command `match` (next to `reset`, `main.rs:792`). It calls `Onboard::restart()` and then `reset()`. This is the "tutorial on demand" that GS-23 asks for. After I-029 this still holds without extra code. `reset` rebuilds `Game`, which closes an open builder (I-029 G13), and the replay button sits in the pause card, which cancels the builder when it opens (I-029 `showPause`).

### 2. Spread is always said (Rust: `crates/bevy-client/src/lines.rs`, new, Bevy-free)

- `pub fn spread_line_due(is_spread_ignite: bool, already_captioned: bool, caption_age: f32, is_melted_or_burnt_out: bool) -> bool`. It returns true for a spread ignite that has not been captioned, **whatever `caption_age` is**. For `Melted` and `BurntOut` it keeps today's rule (uncaptioned and `caption_age > 1.5`). Every other case returns false, so `react()` keeps its current handling for them. `lines.rs` holds only this function; it is Bevy-free and does not need `crate::chem`. `mod lines;` goes next to `mod onboard;`.
- **`react()` changes by one condition.** In the uncaptioned-event block (`main.rs:1320-1339`), the `game.caption_age > 1.5` test becomes a call to `spread_line_due(...)` with the event's kind. Nothing else in `react()` moves: the particle `match`, `Reseeded`, the greenhouse line, the spread `format!` and the win line stay where they are, with their text unchanged.
- **Every caption string stays byte-identical.** No `&'static str` or `format!` template in `chem.rs` or `main.rs` changes text. The spread template that test 11 matches is, at c4832d2, `Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!` (per I-021's caption table). The planner re-reads it at dispatch.
- **Moved to I-021 (revision 3).** `event_lines` (pulling the whole event-line decision out of `react()`), `caption-strings.txt` and its byte-identical Rust test. They do not move B4's move half, and the world hold means spread cannot happen before the first landed cast anyway. I-021 is the item that keys captions, so it needs them.

### 3. The coach on screen (web: `crates/bevy-client/web/coach.js`, new; wired in `hud.js`)

- **`coachFor(state, now, doneAt)`** returns `{ step, text, icon, slot } | null`.
  - `step` is `walk`, `pick`, `cast` or `done`.
  - `pick` is `onboard === 'cast'` with `spell === 0` (Fire is selected, and fire cannot put out fire).
  - `done` lasts 2500 ms after `onboard` first reads `done` (`doneAt`), then the function returns `null`.
  - It returns `null` while `paused`, while `state.work` is truthy (I-029's builder is open; the key does not exist before I-029, so this is a no-op until then), whenever `outcome !== 'playing'`, and before the first state. (A player can win before walking, because spell results apply while held; the banner then wins over the coach.)
  - The steps live in an exported `STEPS` list, so I-029 can add a `work` step with its own builder coach copy without rewriting `coachFor`. This item writes no builder copy.
  - `slot` is 2 for `pick` and `null` otherwise.
- **Pinned coach texts.** There is one plain text per step. It is written for the middle reader, and the icon carries it for pre-readers. I-021 adds the sprout variants.

| step | text | icon |
|------|------|------|
| walk | `Press W to walk.` | `key-w` |
| pick | `Fire will not put out fire. Pick Water: press 2.` | `slot` |
| cast | `Aim at a fire and click to put it out.` | `mouse-left` |
| done | `Nice! Now put out every fire.` | `star` |

The walk step says "Press", not "Hold" (GS-9: no held keys). 0.4 m is about one short press of W, so a 4-year-old using one finger can tap it.

- **Reading rule for coach text** (test 7 enforces it; GS-21): at most 14 words, no word longer than 11 letters, no chemical formula or subscript, at most one temperature. Words are split on whitespace, with punctuation stripped.
- **`mountCoach(doc, canvas, send)`** (exported from `coach.js`) is the testable seam. It builds all of the coach DOM below with `doc.createElement`, and the replay button. It returns `{ update(state, now, hintsOn), replayButton }`. `update` sets the `#coach` class and visibility, the `.slot.coach` class, and the first-state focus, and returns the coach text (or `''`) for `hud.js` to wrap. `hud.js` does only three things: imports `coach.js`, calls `mountCoach(document, canvas, cmd)` once, and changes the hint line to `setHint(coach.update(state, now, settings.hints) || pickHint({...}))`.
- **DOM** (built by `mountCoach` with `createElement`; `index.html` is not edited):
  - **`#coach`**, bottom centre, above the hotbar. Its class is `step-<step>` and it is hidden when `coachFor` is `null`. It holds a large icon (at least 64 px) and nothing else:
    - `key-w` is a keycap with "W" and an up arrow;
    - `mouse-left` is a mouse with the left button filled;
    - `slot` is an arrow pointing at the Water slot;
    - `star` is a star.
  - **The coach text goes into `#hint` through `setHint`**, so the panel's HUD reader sees it with no harness change. The one hint line in `frame()` becomes `setHint(coachText || pickHint({...}))`, where `coachText` is what `coach.update(...)` returns (the text of `coachFor(...)` or `''`). Because `coachFor` is `null` under a banner and `pickHint` returns `''` there, the order is banner, coach, idle, `note`, locked, start. `pickHint`, `setHint` and `hintAllowed` are not edited.
  - **Hints off.** I-022's gate sits inside `setHint`, so with Hints unticked the coach **text** is hidden like every other coaching hint. The `#coach` **icon** ignores `hintsOn` and stays, so the tutorial still shows what to press. (Revision 2 said the coach ignores Hints; that would need an edit to I-022's `hintAllowed`, so it is dropped.) **The limit:** the pick text is the only science line in the first minute. With Hints off it is hidden, and the `slot` icon alone can say "pick Water" but not "fire cannot put out fire". So GS-30 is met in the `pick` step only while Hints is on (the default), which is consistent with I-022.
  - The text is at least 24 px times `--ui`, and has at least 4.5:1 contrast on the panel background (GS-18).
  - In the `pick` step, the Water slot (`.slot[data-spell="2"]`) gets class `coach`: a thick outline plus an arrow shape, not colour alone (GS-19).
  - `#coach` and `#hint` must not cover the crosshair, and must not overlap `#firefinder`, `#objective` or `#hotbar` at 1366×768 and at 1920×1080 (test 9 checks the bounding boxes).
  - **Motion.** Only `done` animates: one scale pop from 1.0 to 1.15 to 1.0 over 300 ms. It does not flash (GS-6). Under `prefers-reduced-motion: reduce` there is no animation.
- **Replay button.** The pause card gets a button **`#coach-replay`**, "Play the tutorial again". It is at least 40 px tall (GS-22) and is reachable by Tab plus Enter (GS-13). `mountCoach` creates it and wires its click to `send('onboard restart')` and `canvas.focus()`. `hud.js` appends `coach.replayButton` to the pause card and adds `showPause(false)` (`hud.js:215`) to the same click.
- **Focus.** The first time `update` sees a state, `mountCoach` calls `canvas.focus()`, unless `doc` has a visible `#age` or the state is paused. W then works before any click. `hud.js`'s first-state branch (`hud.js:627`) is not edited. The I-005 picker already refocuses the canvas after a pick (after I-022, it restores `inert` first).

### 4. A click on a fire casts even when the lock is refused (web: `hud.js` pointer code, `hud.js:167-168`, `190-211`, `363-394`)

- At an unlocked left `pointerdown` (not point mode), record `{x, y}` and whether `state.hover >= 0`.
- A short click (moved 6 px or less) whose lock request is refused now casts **once**, whether the refusal settles before or after `pointerup`, and including the very first click on the page. It casts **at the cursor**, `cmd('tap x y')`, when a target was hovered at `pointerdown`, and otherwise `cmd('cast')` at the crosshair. (Today the first refused click is dropped, and later ones cast at the crosshair; see Facts.) The "Click again to lock the mouse." refusal after Esc keeps its current behaviour.
- When the lock is **granted**, nothing changes: the click locks and does not cast. The golden check `a click over a target locks the pointer and does not cast` stays.
- `tap` already exists and is allowed without `?debug`.

## Acceptance tests

- **Rust:** `cargo test --manifest-path crates/bevy-client/Cargo.toml --locked`.
- **Web unit:** `node --test crates/bevy-client/web/`.
- **E2E:** `node tests/e2e/onboard.mjs <pkg-out dir>`. It serves the built `crates/bevy-client/web/pkg-out` and drives headless Chromium at 1280×720 with `?low`. It reads `window.__game.state()`, `window.__game.hudState()` and the DOM. "Within N frames" means N increments of `state.frame`. Objects are found by `objs[].n` and `burning`, never by a fixed index. The e2e file is first run, and its red state first counts, once all three lanes have merged.

1. **`Onboard`** (core).
   - `new([0,18])` gives `Walk` and `holds_world()` true.
   - `observe_pos([0, 17.7])` (0.3 m) gives `Walk`.
   - `observe_pos([0, 17.59])` (0.41 m) gives `Cast`.
   - `observe_pos([0, 18])` (back at spawn) still gives `Cast`.
   - `observe_landed()` gives `Done`, and `holds_world()` is false.
   - A fresh `Onboard` with `observe_landed()` first gives `Walk`, and a later `observe_pos` 1 m away gives `Done`.
   - `restart()` gives `Walk` with `holds_world()` true.
2. **Spread rule** (core).
   - `spread_line_due(true, false, 0.0, false)` is true.
   - `spread_line_due(true, true, 9.0, false)` is false.
   - `spread_line_due(false, false, 0.5, true)` is false and `spread_line_due(false, false, 2.0, true)` is true (Melted and BurntOut keep the 1.5 s rule).
   - **Mutation:** putting `caption_age > 1.5` back into the spread branch of `spread_line_due` must fail the first assertion. The e2e half of the guard is test 11.
3. *(Moved to I-021 in revision 3: the caption-strings snapshot test.)*
4. **`coachFor` and the pinned table** (web).
   - `{onboard:'walk', outcome:'playing'}` gives step `walk`, icon `key-w`, and text `Press W to walk.`
   - `{onboard:'cast', spell:0, outcome:'playing'}` gives `pick` with `slot: 2`.
   - `{onboard:'cast', spell:1, outcome:'playing'}` gives `cast`.
   - `{onboard:'done', outcome:'playing'}` at `now = doneAt + 2499` gives `done`, and at `doneAt + 2501` gives `null`.
   - `{onboard:'walk', paused:true}` gives `null`.
   - `{onboard:'walk', outcome:'lost'}` and `{onboard:'done', outcome:'won'}` at `now = doneAt` give `null`.
   - `{onboard:'cast', spell:2, outcome:'playing', work:{id:1}}` gives `null` (I-029's builder is open).
   - For all 4 rows, `COACH[step].text` and `.icon` equal the pinned table (was test 5).
5. *(Folded into test 4 in revision 3.)*
6. **`mountCoach`** (web, fake DOM). `mountCoach(fakeDoc, fakeCanvas, send)`:
   - creates `#coach` and `#coach-replay` through `fakeDoc.createElement`;
   - calls `fakeCanvas.focus()` on the first `update` when `fakeDoc` has no visible `#age`, and does not call it when `#age` is visible or the state is paused;
   - after a click on `replayButton`, `send` has received `onboard restart`;
   - with `update({onboard:'walk', outcome:'playing'}, now, false)` (Hints off), `#coach` still has class `step-walk` and is not hidden, and the call returns the walk text (the gate is `setHint`'s, not the coach's);
   - with `update({onboard:'cast', spell:0, outcome:'playing'}, now, true)`, the fake `.slot[data-spell="2"]` gets class `coach`.
7. **Reading rule** (web). Every `COACH` text passes the §3 rule. **Mutations:** each of these must fail the test: making the walk text `Press W to walk forward immediately.` (a 12-letter word), and making the pick text `Fire will not put out fire. Pick H₂O: press 2.` (a formula).
8. **E2E world hold** (contract, `?debug`).
   - On a fresh page with no input for 5 s: `onboard` is `"walk"`, `held` is true, `tick` is unchanged, `burning` is 3, and every `objs[].temp` is unchanged. No visible text in `#objective`, `#firefinder` or `#obj-line` (the elements I-019 owns) matches `/spread|[0-9]+ ?s\b|hurry/i`. The check is scoped to those three so that later HUD text (a pause-card time, an I-029 builder timer) cannot break it.
   - **Reset before any walk:** `cmd('reset')` leaves `onboard` `"walk"` and `held` true, and `tick` stays unchanged for 2 s.
   - `cmd('move 0 1 0.6')` gives `onboard` `"cast"` once `pos` has moved at least 0.4 m, and `tick` is still unchanged.
   - `face <a burning bush>`, `select 2`, `cast` gives `onboard` `"done"` and `held` false within 10 frames of `impacts` rising. `tick` then rises within 60 frames.
   - On another fresh page, `cmd('tick 2400')` while held raises `tick` by 2400 (debug still bypasses).
   - After `done`, `cmd('reset')` leaves `onboard` `"done"`, and `tick` rises after the reset.
   - `cmd('onboard restart')` gives `onboard` `"walk"`, `held` true, `burning` 3, `casts` 0, and then a `tick` that stays unchanged for 3 s.
9. **E2E coach and focus, as a new player would play** (contract, **no `?debug`**, default settings so Hints is on). The steps run in this order, and nothing clicks before step 2 has passed:
   1. At the first state, `document.activeElement` is `#game`, `#coach` has class `step-walk` and is visible, and `#hint` text is `Press W to walk.` **Overlap:** at 1366×768 and then at 1920×1080 (`page.setViewportSize`, then back to 1280×720), the bounding boxes of `#coach` and `#hint` intersect none of `#firefinder`, `#objective`, `#hotbar` or `#crosshair`.
   2. `page.keyboard.down('w')`, wait 1500 ms, `up`: `pos` has moved at least 0.4 m from spawn. Within 2 frames of `onboard` becoming `"cast"`, `#coach` has `step-cast`, or `step-pick` if `spell` is 0.
   3. Press `1` (Fire): within 2 frames `#coach` has `step-pick`, `.slot[data-spell="2"]` has class `coach`, and `#hint` is the pick text. Press `2`: back to `step-cast`, and the class `coach` is gone.
   4. Sweep the mouse across the canvas at y = 0.5 × height in 16 px steps, stopping where `state.hover` is a burning object F, and click there. **The lock must be granted:** `locked` is true within 30 frames, and `casts` is unchanged (the golden rule). Otherwise the test fails with "lock refused in headless; the refused path is test 10".
   5. Steer with synthetic `pointermove` events carrying `movementX`, as golden does, until `aimObj === F`. Then do one `page.mouse.down()` and `up()`.
   - Pass when `onboard` is `"done"` within 30 s of the first state, `#coach` shows `step-done` and is hidden within 3500 ms after that, and `#hint` then equals what `__game.pickHint` returns for the drawn state (I-025's priority).
   - **Mutation:** removing the `canvas.focus()` call in `mountCoach` must fail step 1 (`#game` is not active) and step 2 (W does not move the player, because nothing has clicked yet).
10. **E2E click-to-target when the lock is refused** (contract, `?debug` only for `face` and `turn`).
    - An init script makes `HTMLCanvasElement.prototype.requestPointerLock` return a rejected promise.
    - Fresh page, then `face <burning bush B>`, then `turn 150 0`, so B is off-centre and `aimObj !== B`.
    - Sweep the mouse as in test 9 until `hover === B`, then do one `page.mouse.click`. This is the first click on the page.
    - Within 60 frames, `casts` rose by exactly 1 and `objs[B].burning` is false (Water is selected at boot, from I-019).
    - **Mutation:** reverting §4, so the first refused click casts nothing, must fail this test.
11. **E2E spread is always explained, and science register** (contract and web).
    - **Spread** (contract, `?debug`). Walk and land one cast as in test 8, so the hold ends. Wait until `cooldown` is 0. Then `face <non-burning bush>`, `select 2`, and send `cast` and `tick 2400` in the same frame (one `page.evaluate` that calls `cmd` twice), so the cast caption resets `caption_age` in the same frame the spread events are produced. Once `burning` has risen above its value before the tick, `#history` (pause open) contains a line that matches the spread template pinned in §2, with `{who}` read as `.+` (the planner confirms the text at dispatch). **Mutation:** restoring the `caption_age > 1.5` condition for spread must fail this check.
    - **Register** (web lane). `npm test` passes. The pick text (`Fire will not put out fire.` is a science statement) gets its own new `C-` row (C-040 at c4832d2, or the next free id) with a verdict and sources. The lint matches the string itself, so I-019's cooling row cannot stand in for it, but the new row may cite the same sources. The other three coach texts and the replay label, if the lint flags them, are game instructions and go in the **Not claims** table with a reason, as `AIM_HINT` and `IDLE_HINT` do. `git diff <based_on>... -- content/science-sources.md` adds rows and removes none.
12. **B1 not regressed.** After `build-web.sh`, `QUERY=?low node tests/e2e/golden.mjs` passes 3 out of 3 runs, run alone, with 0 console errors. **It reports the same number of checks** as the dispatch base, which the planner records in `round-1.md` (the critique read 98 at a0e2ecb), so a check that is silently skipped shows up. `golden.mjs` changes **only** by the `I020_KEYS` addition at the key-order check (`golden:1045-1047` at c4832d2). It adds `const I020_KEYS = ['onboard', 'held'];`, appends `...I020_KEYS` as the last element of the key list as it stands on origin/main at dispatch (for example `[...BASE_KEYS, 'gameTime', 'dt', ...I019_KEYS, ...I025_KEYS, ...I020_KEYS]` if only I-025 is ahead), and grows `keys.length` by `I020_KEYS.length`. The list order follows merge order, and the planner states it at dispatch. The check's name and its other conditions stay. The checks read for this claim are listed under "Facts" (spread and lose use `tick N`, and the first landed cast comes before any real-time chem wait). `tests/e2e/analytics-e2e.mjs` passes unchanged.
13. **B2/B3 guard.** `node tests/e2e/perf.mjs` at 4x throttle with `?low`, with `PERF_NO_CAST` unset.
    - **Gated:** Brotli total is at most +25 KB over the based_on artifact. The number is deterministic. Every run's `workload_ok` is true. The run covers the world after `Done`, because the workload's first step teleports 12.5 m and lands a cast during warm-up (see Facts).
    - **Recorded, not gated:** CPU frame p50 and TTI from one interleaved pair of runs (this artifact against the based_on artifact). I-024's `perf-pairs.mjs` may drive the pair if it has merged. I-024 measured a base-against-base CPU spread of 30.4 to 48.2 ms, so a 1 ms gate on one run would fail at random. This item adds one per-frame distance check and a few KB of JS, so the full paired protocol stays with I-024.
    - Both artifacts are built from the same `Cargo.toml` profile. If I-024 changed it after `based_on`, the planner moves `based_on` first.
    - The contract lane records the Brotli delta, the `workload_ok` values and the recorded medians in `round-1.md`.
14. **Scope.** `git diff --name-only <based_on>...` lists only the paths in the Lanes table below, plus `docs/loop/items/I-020/**`. Nothing under `crates/sim/`, `functions/`, `tests/e2e/perf.mjs`, `tests/e2e/analytics-e2e.mjs`, `tests/e2e/panel/`, `crates/bevy-client/web/age.js` or `crates/bevy-client/web/analytics.js` changes, `crates/bevy-client/web/index.html` does not change, and the only change to `golden.mjs` is the `I020_KEYS` addition (any other key list, including I-027's, comes from the rebase and is not this item's diff).

## Gameplay standards touched

Source: `docs/loop/gameplay-standards.md` at origin/main c4832d2 (unchanged since 3dbd332).

| GS | P | How |
|----|---|-----|
| GS-6 | P0 | The coach has no flashing. Only `done` makes one 300 ms scale pop, and reduced motion turns it off. |
| GS-11 | P0 | The beat needs one key (W) plus a click. The click now works on the fire under the cursor even when the lock is refused (§4, test 10). |
| GS-13 | P1 | `#coach-replay` works with Tab and Enter. |
| GS-17 | P0 | `done` shows a big star and a short success line. I-007 and I-019 supply the hit marker and the cheer. |
| GS-18 | P0 | Coach text is at least 24 px times `--ui` with 4.5:1 contrast. The breaker measures it. |
| GS-19 | P0 | The coach icons differ by shape. The Water-slot cue is an outline plus an arrow, not colour alone. |
| GS-21 | P0 | Every coach step has an icon that shows the action, so a pre-reader sees a W keycap or a mouse. The coach text follows the plain-reader rule (test 7). Nothing auto-dismisses before the player acts: walk, pick and cast stay up until they are done. Reading levels for captions are I-021. |
| GS-22 | P1 | `#coach-replay` is at least 40 px tall. |
| GS-23 | P0 | Move and cast are learned by doing. The world is held, so the first area cannot be lost. The tutorial can be replayed from pause. |
| GS-24 | P0 | The objective line stays as it is (I-019 owns it). The coach adds a step-specific action line, which Hints off hides like other coaching hints (I-022); the icon stays. The idle hint is I-007's. |
| GS-25 | P1 | No time pressure while learning: fire waits for the player's first walk and first landed cast, and no countdown shows while held. |
| GS-27 | P0 | The coach copy is friendly and neutral for every age. |
| GS-28 | P0 | No lure or auto-advance. The coach ends into normal play, and replay is opt-in. |
| GS-9 | P0 | The walk step asks for a press, not a held key (`Press W to walk.`); 0.4 m is about one short press. |
| GS-30 | P0 | The `pick` step keeps the science choice visible: fire cannot put out fire, so pick Water. Water at boot is I-019's choice. **Met only while Hints is on**: with Hints off the text is hidden and the icon cannot carry "not fire" (see §3). |

GS-1 to GS-5, GS-7, GS-8, GS-10, GS-12, GS-14 to GS-16, GS-20, GS-26, GS-29, GS-31 and GS-32 are not touched. The item makes no change to the camera, the locked cast path, feedback FX, audio, settings or caption text.

## Out of scope

- **Reading-level captions, labels and the sprout coach variants: all of I-021.**
- Casting visuals, the fire counter, the nearest-fire arrow, `#crosshair.fire`, the locked hint, the boot caption and pausing (I-019); stream thickness, the arrow-in-view rule, one cast per click and the hint order (I-025).
- The Hints gate (I-022). Sensitivity, FOV, the hit markers, the idle hint and settings (I-007).
- Walk speed and frame time under throttle. The 13yo and qa called walking slow; that is B2 work (I-024).
- A world-space marker on the ground for the walk step. Voice-over or spoken coach lines (no audio pipeline yet).
- A visible "world is waiting" cue (the hold is silent on purpose; see §1).
- Coaching the roadmap-v2 core loop. **I-029 owns the coach while a worked cast is being built**: once the first click on an Unknown spell opens the builder, `coachFor` returns `null` (§3), and I-029 adds a `work` step to `STEPS` with its own builder coach copy, or rewrites the `cast` text if it no longer matches. The guided first Formula Bench visit (I-028 lists it as out of scope) has no owner yet; see Hand-off. A second tutorial for the other spells is later content work.
- `event_lines`, the caption-strings snapshot and its Rust test (moved to I-021 in revision 3).

## Contract

**New `__gameState` fields** (existing fields unchanged), appended as the last keys, after I-025's `fireInView`:

| field | type | meaning |
|-------|------|---------|
| `onboard` | `"walk"`, `"cast"` or `"done"` | `Onboard::step()` |
| `held` | bool | `Onboard::holds_world()`: `chem_tick` is skipping |

Both lanes copy this sample fragment for fake states and assertions (other keys omitted):

```json
{"spell":2,"paused":false,"outcome":"playing","onboard":"walk","held":true}
```

and, after the walk, `"onboard":"cast","held":true`; after the first landed cast, `"onboard":"done","held":false`. Only these three `onboard` strings exist.

golden's key-order check gains `I020_KEYS = ['onboard', 'held']`, appended last in the list as it stands on origin/main at dispatch (merge order; test 12). This is the only `golden.mjs` edit.

**Commands** (`__game.cmd`, allowed without `?debug`): `onboard restart` calls `Onboard::restart()`, then `reset()`. `tick N` (debug) steps the world even while it is held. `reset` does not touch `Onboard`. The header comment of `hud.js` (`hud.js:5-16`) lists `onboard restart`.

**Rust modules.** `crates/bevy-client/src/onboard.rs` and `crates/bevy-client/src/lines.rs` (only `spread_line_due`), with the signatures in §1 and §2. Both are Bevy-free. `main.rs` adds `mod onboard; mod lines;` next to the other `mod` lines (there is no `lib.rs`). Both files fall under the `client-core` area; adding them to that row of `docs/loop/areas.md` is a planner change outside this diff (I-021 also expects `lines.rs` there).

**Web module** (ES module, no DOM access at import; `hud.js` imports it statically; `build-web.sh:21`'s `cp` line adds `web/coach.js`): `coach.js` exports `COACH` (the pinned table), `STEPS`, `coachFor(state, now, doneAt)` and `mountCoach(doc, canvas, send)` returning `{ update(state, now, hintsOn), replayButton }`. `coachFor` returns `null` while `state.paused` or `state.work` is truthy. I-029 may extend `STEPS` with a `work` step and override that `null`; this item leaves the hook and writes no builder copy.

**DOM:**
- `#coach.step-{walk|pick|cast|done}`, with the coach text in `#hint` through `setHint`;
- `.slot.coach`;
- `#coach-replay` in the pause card.

All of it is created by `mountCoach`. `index.html` is unchanged.

### Overlap with I-022, I-024, I-025

| file | this item's hunks (c4832d2 lines) | other items' hunks | rule |
|------|------|------|------|
| `hud.js` | header comment 5-16; pointer code 167-168, 190-211, 363-394; the one hint line in `frame()` (today 666-671, after I-025 the `setHint(pickHint({...}))` call); one `mountCoach(...)` call and appending `replayButton` to the pause card with `showPause(false)` on its click; one `import` line | **I-022:** `setHint` and the new `hintAllowed` (441-444). **I-025:** `pickHint` (moved out of 665-671), its call, `findOn` (673), `window.__game.pickHint` (230-238) | Merge order I-022, I-025, I-020. This item wraps I-025's call (`coachText \|\| pickHint(...)`) and never edits `pickHint`, `setHint` or `hintAllowed`. If the merged code has no single expression to wrap, the builder stops and reports it in `round-1.md` |
| `main.rs` | `mod` lines 15-23; resource at 341; `chem_tick` 654-661; `control()`: one `match` arm near 792 and one call between 864 and 865; `projectiles`: one call inside 1271-1287; `react()`: one condition in 1320-1339; format string 1580-1581 and args after 1634 | **I-022:** `Settings` 91-113, `read_settings` 402-412, `aim_and_cast` 952, 980, 983, 987. **I-024:** `world::build` call 464. **I-025:** `Game` fields, `Beam`/stream code 124, 209-277, `setup` 507-516, `aim_and_cast` after 1017 and 1036-1136, `projectiles` beam loop 1182-1189 and mist, format string and args (`fireInView`) | No shared lines except the format string, where keys are appended in merge order. `control`'s camera block (865-874) is not edited |
| `golden.mjs` | key-order lines 1045-1047 | **I-025:** same lines (`I025_KEYS`) and 1617, plus a new function. **I-022:** `reviewFollowups` and its call | This item's edit goes on top of I-025's at the same lines; nothing else |
| `build-web.sh` | `cp` line 21 | none (I-024 does not touch it) | — |
| `content/science-sources.md` | new rows only | I-022, I-025: none planned | Add rows; remove none |

**Later items.** I-021 builds on this item's `COACH`, `coachFor` and `lines.rs` (`spread_line_due`), and now adds `event_lines` and the caption snapshot itself. I-026 (spells from `crates/sim` data) keeps caption strings byte-identical; I-021's snapshot test guards it, and I-021 already goes before I-026. I-027 appends its own `__gameState` and golden keys; whichever of I-027 and this item merges second rebases its key list onto the other's (test 12). I-029 holds the world with `paused` while its builder is open and this item holds it with `Onboard`. The two compose (either one holds), I-029 G15's "`select 2`, `cast`, `casts` +1 within 2 frames" is not broken by the hold, and `coachFor` stays `null` while the builder is open.

## Lanes

The three lanes build against the contract independently. Web tests use fake state objects and a fake DOM, and core tests are Rust unit tests. The contract lane's e2e is red until the core and web lanes have merged, and is first run then. Merge order is core, then web, then contract.

| lane | owns | builds |
|------|------|--------|
| core | `crates/bevy-client/src/main.rs`, `crates/bevy-client/src/onboard.rs`, `crates/bevy-client/src/lines.rs` | §1 and §2, tests 1 and 2, the new state fields, `onboard restart`, the world hold, the one-condition spread fix |
| web | `crates/bevy-client/web/hud.js`, `style.css`, `coach.js`, `*.test.mjs`, `crates/bevy-client/build-web.sh` (the `cp` line only), `content/science-sources.md` | §3 and §4, tests 4, 6, 7 and the register half of 11. `mountCoach` (coach DOM, icons, replay button, first-state focus). Click-to-target on a refused lock |
| contract | `tests/e2e/onboard.mjs`, `tests/e2e/golden.mjs` (key-order lines only) | Tests 8 to 10 and the spread half of 11. The `I020_KEYS` edit. Runs 12 and 13 and records them in `round-1.md` |

**Shared files.** `main.rs` and `hud.js` are also edited by I-022 and I-025, which merge first (see the overlap table). These lanes rebase onto them and keep to the hunks named above.

## Hand-off to I-021

The reading-level work split out of revision 1 is specced and critiqued as **I-021** (`docs/loop/items/I-021/spec.md`), which goes after this item. This item keeps no part of it: no `?band`, no caption keys, no sprout or apprentice text, no panel plumbing. I-021 relies on this item for `COACH`, `coachFor(state, now, doneAt)` and `lines.rs` (`spread_line_due`). **Revision 3 moved `event_lines`, `caption-strings.txt` and its Rust test to I-021.**

**Planner follow-ups outside this diff (revision 3):**
1. **I-021 refresh.** I-021's spec assumes that I-020 ships `event_lines`, `caption-strings.txt` and the snapshot test (its dispatch step 2, the I-020 bullets under "Relied on", `event_lines_keyed`, and tests 3 and 6). I-021 must now build these itself: the planner writes the snapshot at I-021's dispatch, and `event_lines_keyed` becomes the primary function. Its other uses of this item (`COACH`, `coachFor`, `spread_line_due`, the `#coach` icon under Hints off) are unchanged, apart from the walk text, which is now `Press W to walk.`
2. **I-029 Overlap table.** Add a row for `crates/bevy-client/web/coach.js`. I-029 owns the coach copy while the builder is open (a `work` step in `STEPS`, or new `cast` text), because `coachFor` returns `null` while `state.work` is set.
3. **Backlog row: guided first Formula Bench visit (GS-23).** I-028 lists it as out of scope, and no item owns it. Without it nobody teaches *learn*, the first step of the core loop. File it to run after I-028, and say that it moves B4 and B9.

## Refresh (roadmap v2)

Refreshed against origin/main c4832d2 (I-019 merged in #29, wave-7 state #30, roadmap v2 #31). Scope did not grow. Each change:

1. **based_on** 3dbd332 → c4832d2. The code is I-019's (a0e2ecb); #31 touched only `docs/loop/*.md`.
2. **Status and After.** I-007, I-005 and I-019 are merged, so the old blockers are gone. The new gate is merge order: I-022 (Hints gate in `setHint`) and I-025 (`pickHint`, `fireInView`, golden key list) go first, with a stated fallback if I-025 is late.
3. **Collision with I-022's hint gating, resolved without touching its hunk.** The coach text goes through `setHint`, so `hintAllowed` hides it when Hints is off; the `#coach` icon ignores Hints and stays. Revision 2's "the coach ignores I-007's hints setting" is dropped, because keeping it would mean editing I-022's `hintAllowed`. Test 6 gains one assertion (icon stays with Hints off); test 9 states that it runs with default settings.
4. **Hint priority wiring.** "Coach outranks every hint except the banner" is now one wrapped expression, `setHint(coachText || pickHint({...}))`, on top of I-025's `pickHint`. To make that hold, `coachFor` returns `null` whenever `outcome !== 'playing'`. Revision 2's exception "except during `done`" is removed, because a player can win before walking (spell results apply while held) and the `done` line would then sit under the win banner. Test 4 gains the `{onboard:'done', outcome:'won'}` row. Test 9's last check compares `#hint` with `__game.pickHint` instead of describing I-019's order.
5. **golden key-order check.** Revision 2 said `golden.mjs` is unchanged, but golden pins the exact key list (`golden:1045-1047`), so the new `onboard` and `held` keys would fail `clock: state JSON keeps the base keys in order…`. The contract lane now appends `I020_KEYS` after I-025's `I025_KEYS`, the same sanctioned edit I-019 and I-025 make. Tests 12 and 14 and the lane table say so.
6. **Refused-lock facts.** I-019 added `castIfRefused` (`hud.js:167-168`, `198-199`). The first refused click is still dropped and every refused-lock cast still goes to the crosshair, so §4 stands. Its wording now covers a refusal that settles before or after `pointerup`, and keeps the post-Esc "Click again" path as it is.
7. **Line numbers and paths** updated throughout to c4832d2: `chem_tick` 654-661, `control` 666-876 (movement to 864, camera 865-874), command filter 751-758 and 816-819, `tick` 782-791, `reset` 792 and 878-893, `projectiles` landing 1236 and 1271, `react` 1292-1349 (spread block 1320-1339), format string 1580-1581, `Player::default` `player.rs:39-47`, `hud.js` pointer code 167-211 and 363-394, `setHint` 441-444, first state 627, hint block 659-671, `build-web.sh:21`.
8. **Caption snapshot sources** gain the boot caption in `setup` (`main.rs:636-638`), which I-019 added and I-021 counts as a caption.
9. **Test 8 regex fix.** The boot caption says "before they spread", so the "no spreading cue" check now excludes `#caption` (it was already a caption on 3dbd332, so revision 2's check would have failed on day one).
10. **Test 11.** The spread run sends `cast` and `tick 2400` in one `page.evaluate` after `cooldown` is 0, so both land in the same frame as the old wording intended. The register half follows the register's practice: the pick text gets a `C-` row (C-040 or next free), and the plain instructions go in **Not claims**, as I-019's `AIM_HINT` and I-007's `IDLE_HINT` do (revision 2 asked for `C-` rows for all of them).
11. **Test 13** notes that I-024's `perf-pairs.mjs` may drive the interleaved runs and that both artifacts must share one `Cargo.toml` profile (I-024 may change it).
12. **Trimmed for I-021.** The hand-off section no longer repeats I-021's critique requirements (they are I-021's own spec now). "I-021 adds sprout and apprentice variants" became "sprout variants" (I-021 rev 2 cut the apprentice coach text).
13. **Roadmap v2 check.** The item is first-person WASD onboarding, which fits the hard rule, and adds nothing to the core loop. Coaching the Formula Bench, worked casts and mastery is out of scope (I-028 to I-030). I-026 is noted as going after this item. Walk speed now points to I-024, not I-015 (parked).
14. **Scope list** adds `analytics-e2e.mjs` and `analytics.js` (I-022 edits them) to the must-not-change paths, and states that `golden.mjs` changes only at the key-order lines.

## Revision 3 (roadmap-v2 critique)

Follows `critique.md` ("Planner response"). Scope shrank.

1. **The cast step under the new loop.** `coachFor` returns `null` while `state.work` is set, and it exports `STEPS` so I-029 can add a `work` step. I-029 owns the builder coach copy. Replay after I-029 is covered by `reset` and by pause, which both close the builder.
2. **Missing owner for the first Formula Bench visit.** Listed as a planner follow-up in Hand-off.
3. **Smaller.** `event_lines`, `caption-strings.txt` and test 3 moved to I-021. `lines.rs` holds only `spread_line_due`, and `react()` changes by one condition. Test 11 matches the spread template pinned in §2. There is no planner-written input file.
4. **Testable web seam.** `mountCoach(doc, canvas, send)` in `coach.js` owns the coach DOM, the replay button and the first-state focus. Test 6 targets it. `hud.js` keeps an import, a call and the hint wrap.
5. **Walk text** `Press W to walk.` (GS-9).
6. **Expected block** limited to artifacts that predate I-027..I-029.
7. **Tests.**
   - Test 4 absorbs test 5 and gains a `work` row.
   - Test 8's text check is scoped to `#objective`, `#firefinder` and `#obj-line`.
   - Test 9 gains the overlap bounding boxes and the `canvas.focus()` mutation.
   - Test 12 checks the check count and appends to origin/main's key list in merge order.
   - Test 13 gates Brotli and `workload_ok`, and records CPU and TTI without gating them (I-024's 30-48 ms base spread).
   - Test 14 reads "only the `I020_KEYS` addition".
8. **Science.** The pick line keeps its own `C-` row (the lint matches strings). The GS-30 limit under Hints off is stated.
9. **Contract.** It gains a sample `__gameState` fragment and the rule that golden's key list follows merge order.
