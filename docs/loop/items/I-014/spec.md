# I-014: harden the persona panel harness

**Source:** `docs/loop/items/I-006/break-1-main.md` (should-fix 1 to 7 and the nits).
**Lane paths:** `tests/e2e/panel/**`, `docs/loop/panel.md`, `docs/loop/items/I-014/**`. No client, `package.json`, `checks.md`, `bars.md` or `config.md` change.

## Decisions

1. **Information barrier, in code.**
   - Two directories per session. The **run dir** (`--run`) is the only one a persona is told about; it holds `session.json` (pid, port, persona, query, viewport: no truth, no operator path) and `shots/`. The **operator dir** (`--operator-dir`, default `<run>.operator`, a sibling, never inside the run dir) holds `log.jsonl`, `daemon.log`, `daemon.out`, `operator.json` (secret token, CPU benchmark), `start-error.json` and `result.json`.
   - `observe --truth` and `debug` are refused by the CLI unless `PANEL_ROLE=operator` is set, before anything is sent to the daemon.
   - The daemon answers a `truth` observe or a `debug` command only when the request carries the token from `operator.json`; `debug` also still needs `PANEL_SELFTEST=1` at start. A persona that posts `{"cmd":"observe","truth":true}` straight to the port gets `ok:false`.
   - `start` is an operator command too (`PANEL_ROLE=operator`); the persona gets only `observe` and `act` (and may call `stop`). `report` reads `<operator dir>/log.jsonl` and writes `result.json` next to it (it falls back to `<run>/log.jsonl` for an I-006-layout run dir); `score --runs` accepts either dir.
   - panel.md: the operator runs `start`, `stop`, `report`; each persona agent runs with its cwd in a fresh temp dir outside the repo (`mktemp -d`), is given only the absolute CLI path and its run dir (also in that temp dir), and writes `answer.json` there.
2. **B8 `minutes_played` counts engaged time only** (`measure.mjs activeMs`). An act is *engaged* when it is real input other than `wait` and the player's state changed across it: moved >= 0.05 m, yaw or pitch changed >= 0.5 degrees, casts rose, the selected spell changed, pause or pointer lock toggled, or the outcome changed. Credit: an engaged act counts its full input time; a non-`wait` act with no state change counts at most 0.5 s; a `wait` counts only right after an engaged act, at most 3 s in total per engaged act; an `observe` counts its reading cost only when the act before or after it is engaged. `measured.panel_minutes` keeps raw panel time (caps still use it); `measured.engaged_acts` is new. B8 also needs `engaged_acts >= 2 * minutes_played`.
3. **Run quality in `score`.** The panel fails every bar with `reason:"run quality"` and a `problems` list when any persona has: `panics > 0`; `stopped_because:"error"`; `turns < 20`; a throttle other than its persona default (qa 4, others 1) unless `score --allow-throttle`; more than 10 % of turns stalled.
4. **Throttle is verified.** At start the daemon times a fixed JavaScript loop on the blank page (`performance.now`, fastest of 7) after setting the CDP throttle and before the game loads and stores `cpu_bench_ms` in `operator.json`; `report` copies it into the result. `score` fails B4 with `throttle_ok:false` unless qa's `cpu_bench_ms` / 9yo's is > 2 (a missing value fails).
5. **`time_to_move_s`** is the panel time at the end of the first act that either (a) holds a movement key (`w a s d`, arrows) and displaced the player by >= 0.05 m across that act, or (b) leaves the player > 0.5 m from spawn. The cloud's frame-clamped motion makes 0.5 m need about 1.5 s of `w`; (a) makes a natural hold count.
6. **`time_to_cast_s` keeps the caption rule** (impacts rose plus a spell-result caption). Chemistry flags are coarse (a heated bush that does not ignite changes no flag) and they also change on their own (spread, drying), so a chem diff would add false positives with a sky cast. panel.md says this and why.
7. **Stall detection.** `observe` and `act` return `stalled:true` when the page's frame counter did not advance (2 s wall budget after the input). It is a harness flag, not game state. The log records it and `measured.stalled_turns` counts it. Selftest adds a negative control: a stub page whose state never advances must give `stalled:true` on observe and act, and refuse nothing else.
8. **Nits.** `report` refuses a log with no `stop` entry and no cap. The static server gets `.json` and `.png` MIME types. When the daemon is gone, `observe`/`act` say `daemon is gone` and point to `daemon.log` in the operator dir instead of printing ECONNREFUSED; an idle exit is written to `daemon.log`. `score` lists model-routing mismatches (qa not opus, etc.) under `warnings` (not a failure: models are assigned by the operator).

## Acceptance tests (each can fail)

Unit (`node --test tests/e2e/panel/`, no browser):

- U1 `observe --truth` without `PANEL_ROLE=operator` exits 1 with an error naming `PANEL_ROLE`, and a fake daemon behind `session.json` receives no request. `debug` likewise.
- U2 `authorize()` refuses `truth` and `debug` without the right token and allows plain `observe`/`act`.
- U3 `layout(run)` puts the operator dir outside the run dir (not equal, not inside), and an `--operator-dir` inside the run dir is refused.
- U4 Idle probe: a log of 90 x `wait 10000` (with or without observes in between) gives `minutes_played` < 0.5 and B8 fails; 300 bare observes give 0.
- U5 A log of 400 alternating `observe` + `key Shift 10000` with no state change gives `minutes_played` <= 200/60.
- U6 The existing synthetic log still gives `minutes_played` 0.16 (all of it engaged).
- U7 `score` with a qa throttle 1, or panics 3, or `stopped_because:"error"`, or turns 9, fails all bars with `reason:"run quality"` and names the persona and field; `--allow-throttle` lifts only the throttle problem.
- U8 `score` fails B4 when qa/9yo `cpu_bench_ms` ratio <= 2 or is missing; passes at 4.
- U9 `time_to_move_s` is set by a `key w 1500` that moved 0.3 m (under 0.5 m from spawn), and not by a `key Shift` that moved 0.3 m or a `wait`.
- U10 `report` refuses a log without a stop entry.
- U11 B8 fails when `engaged_acts < 2 * minutes_played`.

Selftest (`PANEL_ROLE=operator node tests/e2e/panel/selftest.mjs`, real build):

- S1 After `start`, the run dir holds only `session.json` and `shots/`; `log.jsonl` and `operator.json` are in the operator dir; `session.json` has no token.
- S2 A persona-role `observe --truth` exits non-zero; a raw HTTP post of `{"cmd":"observe","truth":true}` without the token gets `ok:false`; with the operator role it works.
- S3 `operator.json` has `cpu_bench_ms` for qa and 9yo and qa/9yo > 2.
- S4 Stub page: `observe` and `act` return `stalled:true`; on the real game they return `stalled:false`.
- S5 The existing selftest checks still pass (check 7 now compares with the first `w` act that moved).
