# I-006 lane main, round 1 (builder)

**Branch:** `loop/I-006`, built first on `origin/claude/fps-prototype` 877131f (the spec's base), then **rebased onto 5a8f5fe** ("Fix pointer lock: a left click always locks; add drag-to-look fallback"), which landed on PR #13 during the round. PR #13 is not merged. Merges after I-001.
**Lane paths touched:** `tests/e2e/panel/**`, `docs/loop/panel.md`, `docs/loop/items/I-006/**`. No client, `package.json`, `checks.md` or `config.md` change.
**Builds tested:** `crates/bevy-client/build-web.sh` (`CARGO_TARGET_DIR=/home/claude/bevy-target`, wasm-bindgen 0.2.129 from `/home/claude/bevy-target/tools/bin`). At 877131f: release in 2m51s, wasm 17,655,634 bytes raw, 5,032,661 gzip. At 5a8f5fe: wasm 5,034,647 gzip. Build output was not committed.
**Stale check (spec):** 5a8f5fe changes only `hud.js`, `index.html` and `golden.mjs`. No state field, HUD id or console line the harness uses changed, so `hud.mjs` needed no change. The selftest was rerun on the 5a8f5fe build (31/31, below). The first run on 877131f was 30/30.
**Environment while measuring:** 4 cores, 15 GB memory, load average 10 to 12 (other wave agents running Chromium at the same time). The game ran at about 0.7 to 3 fps (golden's STATS: `fps_p50=0.6` to `1.3`). No fps or perf claim is made here.

## What was built

| File | Role |
|---|---|
| `tests/e2e/panel/panel.mjs` | CLI: `start`, `observe [--truth]`, `act`, `report`, `stop`, `score` (and `debug` for the selftest). One JSON object per call; exit 1 on failure, 3 when a cap is hit. |
| `tests/e2e/panel/daemon.mjs` | Detached session daemon: one Chromium page, a static server for `pkg-out`, a 127.0.0.1 API on a random port, `session.json`, `log.jsonl`, `shots/NNNN.png`. Serialises commands. |
| `tests/e2e/panel/hud.mjs` | The only place that names HUD ids, the state global and console line formats (spec "stale check"). `readView` returns visible text only. |
| `tests/e2e/panel/personas.mjs` | 5 personas: reading cost per observe (6yo 4 s, 9yo 3 s, 13yo 2 s, teacher 3 s, qa 2 s), default throttle (qa 4), model routing; caps (400 turns / 20 panel min). |
| `tests/e2e/panel/clock.mjs` | Panel clock (deterministic from the action log). |
| `tests/e2e/panel/measure.mjs` | `time_to_move_s`, `time_to_cast_s`, `minutes_played`, `quests_completed`, outcome, turns, console errors, panics, all from `log.jsonl`. |
| `tests/e2e/panel/result.schema.json` + `schema.mjs` | Draft 2020-12 schema `panel-result/1`, and a small validator driven by that file (no new npm package). |
| `tests/e2e/panel/score.mjs` | The panel line for `scoreboard.jsonl` (B4/B5/B8, incomplete and mismatched panels). |
| `tests/e2e/panel/*.test.mjs`, `index.js`, `fixtures.mjs` | Unit tests (no browser). `index.js` exists because Node 22 resolves the directory argument of `node --test tests/e2e/panel/` as a module; it imports the test files. |
| `tests/e2e/panel/selftest.mjs` | Scripted robot over the real CLI and a real build (tests 6 to 12). |
| `docs/loop/panel.md` | Protocol: run recipe, commands, panel clock, game clock, measured fields, five persona prompts, rubric with anchors, schema mirror, scoring, known harness facts. |

## Tests first

The unit tests were written before any module existed. First run:

```
$ node --test tests/e2e/panel/
# Error [ERR_MODULE_NOT_FOUND]: Cannot find module '.../tests/e2e/panel/score.mjs' imported from .../score.test.mjs
# pass 0
# fail 1
```

## Results (real output)

| # | Test | Result | Measured |
|---|---|---|---|
| 1 | Scoring B4 | PASS | spec example `B4.pass=true`, `median_clarity=8`; ttc 31 / null / control blocker each false; text blocker true |
| 2 | Scoring B5, B8 | PASS | fun [8,8,8,9,6] pass; [8,8,8,9,5] fail `min_fun=5`; [7,7,8,9,9] pass median 8; [7,7,7,9,9] fail; 9yo 14.9 min false, 15.0 true |
| 3 | Incomplete panel | PASS | 4 results, five `9yo`, mismatched sha/query all `pass:false` with the right reason; `panel-result/0` listed in `rejected` with `schema: expected "panel-result/1"` |
| 4 | `report` validation | PASS | clarity 11, fun 0, missing blockers, kind "other" each exit 1 naming the field; an answer's own `measured`/`time_to_cast_s` ignored with a stderr warning, harness values written |
| 5 | Panel clock | PASS | 3 + 1.5 + 2 + 0.1 + 3 = **9.6**, identical on replay |
| 6 | observe | PASS | PNG 366,104 bytes with PNG signature; all view keys; no `objs`/`aimObj`/`hover`/`tick`/`ghTemp` without `--truth`; `truth.objs` length 14 with it |
| 7 | move | PASS | first `key w 1500` moved **1.00 m**; `time_to_move_s = 9.1` = that act's panel time; a session with only `wait` has `time_to_move_s: null` |
| 8 | look and cast | PASS | click on sky: `pointer_locked=true`; `move 200 0`: yaw 0 → 334.8 (25.2°) **via real** (no synthetic fallback needed in new headless); sky cast at pitch 47.6: casts 0→1, impacts 0→1, `time_to_cast_s` stays null; `key r` restart (real input), debug face to burning bush 6, `Digit2`, click centre: casts 0→1, bush burning → false (wet 1, caption "Fire out! Water cooled it below its ignition temperature."), `time_to_cast_s = 36.1` = the casting click's panel time (877131f run: bush 0, 47.8) |
| 9 | Real input only | PASS | `act cmd reset`, `act face 3`, `act teleport 1 2` exit 1 (`unknown action ... real input only`); `debug face 3` refused without `PANEL_SELFTEST`; next observe ok (turn 2); the game-command hook appears only at `daemon.mjs:276` (debug face) and `daemon.mjs:384` (reset in start) |
| 10 | Throttle | PASS | qa: `session.json` `throttle:4`, `cdp_throttle_rate:4`, daemon.log `CDP Emulation.setCPUThrottlingRate rate=4`, result `throttle:4`; 9yo: `throttle:1`, no CDP call |
| 11 | Parallel and cleanup | PASS | qa and 9yo sessions started and observed concurrently, each turn 1 with its own screenshot; after stop both pids dead and ports refuse connections; second stop exits 0 |
| 12 | Errors surface | PASS | injected `console.error("x")` → `console_errors_new: 1` on the next act, `measured.console_errors: 1` |
| 13 | One real haiku persona | **BLOCKED** | This builder runs as a workflow subagent with no tool to spawn another agent, so no fresh-context haiku run was possible. Not marked passed; no scripted substitute counted. The loop should run it as the first step after merge (one haiku agent, 6yo block from panel.md, run dir under `.loop-artifacts/I-006/`, copy `result.json` and five shots to `evidence/`). |
| 14 | Existing smoke unaffected | **FAIL (pre-existing, environment)** / scope PASS | `git diff --stat origin/claude/fps-prototype` shows only `tests/e2e/panel/`, `docs/loop/panel.md`, `docs/loop/items/I-006/`. `golden.mjs` is untouched, but `QUERY=?low node tests/e2e/golden.mjs` exits 2 on this box: after the pointer-lock click the headless-shell renderer was OOM-killed (`Target crashed`; dmesg `Killed process ... (chrome-headless) anon-rss:6375768kB`). Control: an uncommitted temp copy of golden.mjs whose only change was `chromium.launch({ channel: 'chromium' })` passed **28/28** (exit 0) on the same build and box minutes later (pointer lock `locked=true`, mouse look yaw 0 → 334.8, WASD 2.16 m, win banner, no console errors). So the failure is the headless shell in this cloud, not this branch. After the rebase onto 5a8f5fe, unmodified golden.mjs crashed the same way (`PASS boots`, `PASS three fires at start`, then `page.evaluate: Target crashed`, exit 2). |

Selftest summary lines: `30/30 passed` on 877131f; `31/31 passed` on 5a8f5fe (one check added: R restarts the courtyard). Two intermediate 5a8f5fe runs were 28/30 and 16/19; their causes are findings 7 and 8, both fixed before the 31/31 run. Unit tests: `# tests 14 # pass 14 # fail 0`. Full selftest log and two results in `evidence/selftest/`.

Wall time per daemon command during the selftest (contended box): act mean 8.1 s (max 18.6 s), observe mean 3.3 s (max 6.8 s), start about 6 to 10 s.

## Findings

1. **Headless shell + pointer lock = OOM.** With Playwright's default `chromium.launch()` (chrome-headless-shell 1243), a click that requests pointer lock stalls frames (STATS windows stretch to 20 to 35 s) and the renderer grows past 4 to 9 GB until the memory cgroup kills it, within about a minute. WebAssembly memory stays at 33.2 MB, so the growth is in the browser, not the game heap. Key presses and plain mouse moves do not trigger it. With `channel: 'chromium'` (new headless, 153.0.8010.12) the same click locks (`locked=true`) and stays stable. The daemon uses the new headless and falls back to the shell. The dmesg log shows the same kills on other agents' Chromium runs, so this probably affects the I-001 smoke and perf runs too: **follow-up for the smoke/perf owners: launch with `{ channel: 'chromium' }`** (outside this lane).
2. **Game clock between turns.** Without intervention, the game keeps running in wall time while a model thinks (often tens of seconds per turn), so the greenhouse would burn down after a few turns and B5/B8 would measure model latency. The daemon pauses page JavaScript between commands (CDP `Debugger.pause`, resumed for each observe/act; `--freeze 0` turns it off). Measured: across 30 s pauses the frame counter moved by 2 (resume overhead only), and pointer lock survived the pause. Side effect: STATS/CPUFRAME inside a panel session include the pauses; panel.md says they are not perf data.
3. **The game runs slower than real time in the cloud.** At about 1 fps Bevy's per-frame clamp (`delta.min(0.05)` for projectiles, 0.25 s virtual max delta for the fixed step) makes a 1500 ms `key w` walk about 1 m and a water projectile take several wall seconds to land. Panel time counts input duration, so cloud `time_to_move_s`/`time_to_cast_s` are pessimistic. On the 10 s / 30 s B4 thresholds this can matter: in the selftest, a robot that knew exactly what to do needed 9.1 panel s to move 0.5 m (with a 3 s read before). Recorded for the scoring step; the harness does not correct for it.
4. **Every cast writes a caption** as it leaves the hand (`game.say(spell.cast_caption())`), so a caption change alone does not show that a cast landed. `measure.landed` subtracts one caption per cast and ignores lone spontaneous captions ("Heat from a neighbour…", "Oh no!"); unit tests pin the sky-cast case.
5. **Clicks under pointer lock are sent in place.** A move before the press sends a stray `movementX`, which turned the view by 161° in an early run on the headless shell. Unlocked clicks move first and wait 2 frames (wall time) so the game's hover sees the cursor before the press; otherwise a click on the sky could be read as a tap on whatever was under the old cursor (seen once in an early run).
7. **Half-written `session.json`.** In one parallel start, `start` read `session.json` while the daemon was still writing it (`Unexpected end of JSON input`), though the daemon was healthy. The daemon now writes to a temp file and renames it (same for `start-error.json`).
8. **The fire can win during a slow scripted run.** The game runs during each act, so on a ~1 fps box the selftest's sky-cast wait loop let the fires reach the greenhouse (`outcome: lost`). After that, `face` and casts did nothing useful. The selftest now presses R (real input) and checks for a fresh courtyard before the bush cast. A persona can lose the same way, which is real play and is recorded as such.
9. The view exposes the cursor shape (`pointer` over targets when unlocked) and whether the crosshair is lit, because a player sees both; no game-state field is exposed.

## Needs (outside this lane)
- Smoke/perf harnesses: switch to new headless (`channel: 'chromium'`), finding 1.
- Loop scoring step: run test 13 (haiku 6yo) right after merge, then the first five-persona panel; update checks.md level 5 to the panel.md recipe.
- Client-web area (optional): a panel would benefit from the game's own pause while unfocused, which would let the harness drop the debugger freeze.
