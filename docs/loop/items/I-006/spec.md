# I-006: Persona panel harness (observe/act CLI + panel protocol)

**based_on:** `origin/claude/fps-prototype` 877131fe4aa8293b033d44502cb20c83ab5dec16 (PR #13, not merged). `origin/main` a7b48bd for the loop docs. The lane builds on `claude/fps-prototype`, because the client and `tests/e2e/golden.mjs` exist only there.
**Track:** measurement. **Moves:** B4, B5, B8, from "no way to measure" to "measurable". **No bar value is claimed at merge.** The first real panel run is a separate scoring step done by the loop, not by this item.
**Contract:** none. No shared file is touched. `package.json` is not edited: Playwright is loaded the way `golden.mjs` loads it (`createRequire(import.meta.url)('playwright')`, preinstalled).

## Facts this spec relies on (from 877131f)
- The build is `bash crates/bevy-client/build-web.sh`, with output in `crates/bevy-client/web/pkg-out`. Use `CARGO_TARGET_DIR=/home/claude/bevy-target` and the wasm-bindgen CLI under `/home/claude/bevy-target/tools`.
- `window.__game.state()` returns the parsed `window.__gameState` (spell, yaw, pitch, locked, hover, aimObj, casts, frame, tick, objs[] with `n`/`burning`, burning, extinguished, outcome `playing|won|lost`, caption, captionId, ghTemp, ghDanger). `window.__game.cmd(text)` takes the test commands `select`, `cast`, `tap`, `look`, `turn`, `face`, `tp`, `move`, `tick` and `reset`.
- The HUD DOM ids are `#objective`, `#obj-line`, `#warning`, `#crosshair`, `#hint`, `#caption`, `#hotbar` (slots carry `.sel`), `#banner` (`.big`/`.small`, `.hidden`), `#pause` (`#resume`, `#pointmode`) and `#loading`.
- Headless Chromium may report `movementX = 0` under pointer lock. `golden.mjs` falls back to dispatching a synthetic `mousemove` with `movementX` on `document`, which is the same handler a real mouse feeds.
- The console prints `STATS ...` and `CPUFRAME n= p50_ms= ...` every 2 s, and `FIRST_FRAME t=...ms` once.
- The cloud uses a software GPU, so frames are slow and fps means nothing there. Waits are keyed to game state or wall time, never to frame counts a persona would have to reason about.

**Stale check:** if PR #13 merges with different names for the state fields, HUD ids or console lines listed above, the builder updates the harness's selectors in one place (`tests/e2e/panel/hud.mjs`) and notes the change in the round notes.

## Problem
Three of the eight bars (B4 every age can play, B5 fun, B8 enough to play) are judged by a persona panel. `checks.md` level 5 names "I-006 harness" as the way to run it, and there is no such harness. Nothing lets a model play the 3D artifact one turn at a time. Nothing measures time-to-move and time-to-cast the same way on every run. There are no persona prompts, no rubric and no output schema, so `scoreboard.jsonl` has no panel lines. I-007 (feel) and I-008 (content) both name the first panel run as their evidence, and that run cannot happen yet.

The harness has to survive a model's way of playing. Each persona turn is a separate tool call, often tens of seconds apart, so the browser must stay alive between calls. The persona must see only what a player sees (screenshot and visible HUD text), never the game's internal state. Otherwise a 6-year-old persona "finds" a target by reading `objs[]`, and B4 measures nothing.

## Smallest version
One lane. Files: `tests/e2e/panel/**` and `docs/loop/panel.md`.

### 1. CLI: `node tests/e2e/panel/panel.mjs <command> --run <dir> [...]`
Every command prints exactly one JSON object on stdout. It exits 0 on success, and on failure it exits non-zero and prints `{"ok":false,"error":"..."}`. `<dir>` holds all of a session's state, so five sessions can run in parallel with five dirs.

- `start --run <dir> [--persona <id>] [--throttle <n>] [--query <q>] [--vw 1280 --vh 720] [--base-url <url>]`
  Spawns a detached daemon that holds one Chromium page and, unless `--base-url` is given, a static server for `web/pkg-out` (same MIME map as golden). The daemon listens on 127.0.0.1 on a random port and writes `<dir>/session.json` (`pid`, `port`, `persona`, `throttle`, `query`, `started_at`). `--throttle n` applies CDP `Emulation.setCPUThrottlingRate {rate:n}`. `start` returns once `window.__gameState` exists (timeout 180 s), or fails. The persona `qa` defaults to `--throttle 4`, and the other personas default to 1.
- `observe --run <dir> [--truth]`
  Saves `<dir>/shots/NNNN.png` (NNNN is the turn number) and returns the **view**:
  `{ok, turn, panel_s, screenshot, hud:{objective, warning, hint, caption, banner, pause, loading, hotbar:{selected, slots:[label...]}, crosshair_on, cursor}, pointer_locked}`.
  The HUD text fields contain only text that is visible: an element that is hidden, or has zero opacity or zero size, gives `null`. `--truth` adds a `truth` object (the raw `__game.state()`, plus the last `CPUFRAME` and `STATS` lines). The persona instructions in `panel.md` never use `--truth`. The runner and the selftest use it.
- `act --run <dir> <action> [args]`. Actions are real input only:
  - `key <Key> [ms]`: hold one key for `ms` (default 100). Keys are Playwright names (`w`, `a`, `s`, `d`, `ArrowUp`...`ArrowRight`, `Space`, `Shift`, `Escape`, `Digit1` to `Digit9`, `r`).
  - `keys <K1+K2...> <ms>`: hold a chord, for example `w+Shift 2000`.
  - `move <dx> <dy> [steps]`: relative mouse move. When the pointer is locked and the real move gives no yaw change, the harness uses golden's synthetic `movementX`/`movementY` fallback and reports `"via":"synthetic"`.
  - `drag <x1> <y1> <x2> <y2> [ms]`: press, move and release.
  - `click <x> <y> [left|right]`, and `hold <x> <y> <ms>` (hold the mouse button to keep casting). The coordinates are in viewport CSS px.
  - `wheel <dy>`.
  - `wait <ms>`, at most 10000.
  `act` never calls `__game.cmd`. It returns `{ok, turn, panel_s, action, via, console_errors_new}`.
- `report --run <dir> --file <persona-answer.json>`: validates the persona's self-report against the schema (section 3), merges it with the measured fields, and writes `<dir>/result.json`. An invalid self-report exits non-zero and names the field.
- `stop --run <dir>`: closes the browser and the server, kills the daemon and removes `session.json`. It is idempotent.
- `score --runs <dir1> ... [--append <scoreboard.jsonl>] [--artifact <sha>]`: reads each `result.json`, computes the panel line (section 4), prints it, and appends it to the given file only if `--append` is passed.

### 2. Measured fields (the harness measures these, the persona never reports them)
The daemon samples truth after every act and every observe, and logs each turn to `<dir>/log.jsonl` (`turn`, `cmd`, `panel_s`, the view, and the truth summary: pos/yaw/casts/outcome/burning/extinguished).
- **Panel clock (`panel_s`).** Wall time on a model's turn includes model latency, which a child does not have, and the software GPU slows everything down. So time is counted as **panel time**: the sum of each act's input duration (key/hold/drag ms, `wait` ms, 100 ms for a click or move), plus a fixed reading cost per `observe` (per persona: 6yo 4 s, 9yo 3 s, 13yo 2 s, teacher 3 s, qa 2 s, stored in `personas.mjs`). The rule is written in `panel.md`. It is deterministic for a given action log.
- `time_to_move_s`: the panel time at the end of the first act after which the player's position has moved more than 0.5 m from the spawn point. It is `null` if that never happens.
- `time_to_cast_s`: the panel time at the end of the first act after which `casts` increased **and** some object's chemistry changed (burning, extinguished, wet, ice, or a caption change caused by the cast). This means a cast that landed, not a click into the sky. It is `null` if that never happens.
- `minutes_played`: the panel time at `stop`, or at the turn cap, divided by 60.
- `quests_completed`, `outcome`, `turns`, `console_errors`, `panics` (from the console text `panicked`), `throttle`.

### 3. `docs/loop/panel.md` (protocol)
- **Run recipe.** Build, then one fresh-context agent per persona, then `start`, the turn loop (observe, decide, act), `stop`, `report`, and `score`. Model routing comes from `config.md` (haiku for 6yo/9yo, sonnet for 13yo/teacher, opus for qa). Use the same artifact sha and query (`?low` in the cloud) for all five. Cap: 400 turns or 20 panel minutes, whichever comes first. (The cap must let a 9yo reach the 15 panel minutes B8 needs: at about 3 s per observe plus a short act, 150 turns is only about 11 minutes, so B8 could never pass.) A persona may stop earlier by saying it would quit, and its reason is recorded.
- **Five persona prompts**, each a self-contained block. Each sets the age-appropriate reading level, patience, what the persona knows about games, and the rule "you only know what the screenshot and the HUD text show you; never guess hidden state; say what you are trying to do before each act". Prices are never discussed (target.md). The 6yo reads only short words and prefers clicking things. The QA persona hunts for blockers, soft-locks and console errors, and plays on 4x throttle.
- **Rubric** (the persona's self-report, asked after `stop`): `clarity` 1–10 and `fun` 1–10, each with a one-sentence anchor scale written into the doc (1 = "I didn't know what to do / I wanted to stop", 10 = "I always knew what to do / I didn't want to stop"); `blockers` (a list of `{kind: control|goal|bug|perf|text, what, turn}`, where `control` means the persona could not move, look or cast as intended); `would_play_again` (yes/no/maybe); `best` and `worst` (one sentence each, in the persona's voice); `quotes` (at most 3 short verbatim lines from its turn notes).
- **Result schema** `tests/e2e/panel/result.schema.json` (JSON Schema draft 2020-12), mirrored in the doc: `{schema:"panel-result/1", persona, model, artifact_sha, query, throttle, measured:{time_to_move_s, time_to_cast_s, minutes_played, quests_completed, outcome, turns, console_errors, panics}, self:{clarity, fun, blockers, would_play_again, best, worst, quotes}, stopped_because: "quit|turn_cap|time_cap|blocked|error"}`.
- **Scoring rules** for B4/B5/B8, quoted from `bars.md` and turned into the formula that `score` uses (section 4). It states that panel scores are "panel", never "player" data.

### 4. Panel line for `scoreboard.jsonl` (written by `score`)
`{"ts", "kind":"panel", "source":"panel", "artifact_sha", "query", "personas":[...5 ids], "per_persona":{id:{ttm, ttc, clarity, fun, minutes, blockers_control}}, "B4":{"pass", "median_clarity", "all_move_10s", "all_cast_30s", "control_blockers"}, "B5":{"pass", "median_fun", "min_fun"}, "B8":{"pass", "minutes_9yo", "quests_completed_9yo"}}`
- B4 passes when every persona has `time_to_move_s` ≤ 10 and `time_to_cast_s` ≤ 30 (a `null` fails), the median clarity is ≥ 8, and no persona reports a blocker of kind `control`.
- B5 passes when the median fun is ≥ 8 and every persona's fun is ≥ 6.
- B8 passes when the 9yo's `minutes_played` is ≥ 15. The content conditions of B8 (≥ 3 quests, 6 spells, a free-play area, a skill tree) are recorded as `"content_check":"manual"`, because this harness cannot see them.
- If fewer than 5 valid results are given, every bar gets `"pass":false, "reason":"incomplete panel"`.

## Acceptance tests (each can fail)
Unit tests run with `node --test tests/e2e/panel/` and need no browser. The selftest runs with `node tests/e2e/panel/selftest.mjs` after a build, and it is a scripted robot, not a model. It may use `observe --truth` and one debug-only `face` command through a `PANEL_SELFTEST=1` path in the daemon that `act` refuses otherwise (test 9 checks that).

1. **Scoring, B4.** Five results with ttm [3,5,8,9,10], ttc [10,12,20,25,30], clarity [8,8,9,7,10] and no control blockers → `B4.pass=true`, `median_clarity=8`. Change one ttc to 31 → false. Change one ttc to `null` → false. Add one `{kind:"control"}` blocker → false. Add a `{kind:"text"}` blocker instead → still true.
2. **Scoring, B5 and B8.** Fun [8,8,8,9,6] → pass. Fun [8,8,8,9,5] → fail (`min_fun=5`). Fun [7,7,8,9,9] → pass (median 8, min 7; this case pins the median as the middle of 5 sorted values). Fun [7,7,7,9,9] → fail (median 7). 9yo minutes 14.9 → `B8.pass=false`. 15.0 → true.
3. **Incomplete panel.** Four results → every bar has `pass:false` and `reason:"incomplete panel"`. A result with `schema:"panel-result/0"` is rejected by name. Five results that do not carry the five distinct persona ids (for example five `9yo`), or that differ in `artifact_sha` or `query`, give `pass:false` with `reason:"incomplete panel"` or `"mismatched runs"`.
4. **Schema validation (`report`).** A clarity of 11, a fun of 0, a missing `blockers`, or a blocker kind `"other"` each exit non-zero and name the field. A valid answer writes `result.json`, and `measured` comes from the log, not from the answer: if the answer carries its own `measured` or `time_to_cast_s`, the harness's values win and a warning is printed.
5. **Panel clock.** A synthetic log: observe (9yo, 3 s), `key w 1500`, `wait 2000`, `click 640 360`, observe → `panel_s` = 3 + 1.5 + 2 + 0.1 + 3 = 9.6. The same log replayed gives the same value.
6. **Selftest, observe.** After `start`, `observe` writes a PNG file larger than 10 KB whose first 8 bytes are the PNG signature. The view has every key listed in section 1. Without `--truth`, the JSON text contains none of `objs`, `aimObj`, `hover`, `tick` or `ghTemp`. With `--truth` it contains `truth.objs`.
7. **Selftest, move.** `act key w 1500` (or more, up to 6000 on the software GPU, in steps of 1500) moves the truth position more than 0.5 m, and `result.measured.time_to_move_s` is the panel time of that act, not `null`. With no movement keys pressed (only `wait`), `time_to_move_s` stays `null`.
8. **Selftest, look and cast.** `act click` on empty sky locks the pointer (or the result says `pointer_locked:false` and the test continues in tap mode). `act move 200 0` changes yaw by more than 5°, with `via` either `real` or `synthetic`. After the debug `face` toward a burning bush and `act key Digit2`, then `act click` at the canvas centre (or `hold` 600 ms), `casts` increases and the bush's burning state changes, and `time_to_cast_s` is set. A click into the sky with no target in front increases `casts` but does **not** set `time_to_cast_s`.
9. **Real input only.** `act cmd reset`, `act face 3` and any unknown action exit non-zero with an error, and the session is still alive (a following `observe` succeeds). `grep -n "__game.cmd" tests/e2e/panel/*.mjs` matches only the `PANEL_SELFTEST` debug path and the `reset` used by `start`.
10. **Throttle.** `start --persona qa` writes `throttle:4` to `session.json` and to the result, and the CDP call is made (logged). `start --persona 9yo` writes `throttle:1`.
11. **Parallel and cleanup.** Two sessions with different `--run` dirs run `observe` at the same time, and each gets its own screenshot and turn counter. After `stop` on both, neither daemon pid is alive, neither port accepts connections, and `stop` a second time exits 0.
12. **Errors surface.** A page that logs `console.error("x")` (injected through `page.evaluate` in the selftest) shows up as `console_errors_new: 1` on the next act, and in `measured.console_errors`.
13. **Protocol smoke with one real model.** One fresh-context haiku agent plays the 6yo persona exactly as `panel.md` instructs it (no `--truth`) for at least 20 turns or until it quits. Its `result.json` validates against the schema, and its turn log shows it used only `observe` and `act`. The run dir goes under `.loop-artifacts/I-006/`, and its `result.json` and five screenshots are copied to `docs/loop/items/I-006/evidence/`. If the lane cannot spawn a haiku agent, it records test 13 as `blocked` in the round notes; it is never marked passed on a scripted run. This test checks that a model can drive the harness. Its scores are **not** a bar reading and are not appended to `scoreboard.jsonl`.
14. **Existing smoke unaffected.** `QUERY=?low node tests/e2e/golden.mjs` still passes, and `git diff --stat` shows changes only under `tests/e2e/panel/` and `docs/loop/panel.md` (plus this item's dir).

## Out of scope
- Running the actual five-persona panel and writing B4/B5/B8 values to `scoreboard.jsonl` (loop scoring step, after merge).
- Any client change: no new HUD ids, no state fields, no touch controls. If the harness needs a fact the view cannot see, it goes into `needs`/the round notes as a follow-up for the client-web area.
- Real children, teachers or playtests (human-only, target.md).
- Real-GPU / Mac runs. The panel is cloud-only. Perf numbers stay in the `perf` check (B2/B3).
- Editing `checks.md`, `config.md`, `bars.md` or `package.json`. The loop updates `checks.md`'s level-5 command to the `panel.md` recipe when it merges this item.
- Aggregating across nights, plateau logic, or dashboards.

## Lanes

| lane | owns | depends on | done when |
|------|------|------------|-----------|
| panel | `tests/e2e/panel/**`, `docs/loop/panel.md` | built `crates/bevy-client/web/pkg-out` from 877131f (read-only) | tests 1–14 pass (13 may be `blocked`, named as such), round notes in `docs/loop/items/I-006/lanes/panel/round-N.md` |
