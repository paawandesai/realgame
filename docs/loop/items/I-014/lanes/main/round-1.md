# I-014 lane main, round 1 (builder)

**Branch:** `loop/I-014`, rebased onto `origin/main` a94fc48 (I-012 merge) during the round. It was first cut from 9d030e0; the shared ref moved while I worked. No panel file changed upstream, so the rebase was clean.
**Lane paths touched:** `tests/e2e/panel/**`, `docs/loop/panel.md`, `docs/loop/items/I-014/**`. No client, `package.json`, `bars.md`, `checks.md` or `config.md` change.
**Build tested:** `crates/bevy-client/build-web.sh` at a94fc48 (`CARGO_TARGET_DIR=/home/claude/bevy-target`, wasm-bindgen from `/home/claude/bevy-target/tools/bin`). wasm 5,035,656 bytes gzip. Build output was not committed.
**Environment:** 4 cores, shared with other wave agents. Load average was 6 to 9 for runs st1 to st5 and 24 to 25 for the final run (st6). Software GPU, so no fps claim is made.

## What changed

| Breaker item (I-006 break-1) | Fix | Where |
|---|---|---|
| 1. Information barrier | The run dir now holds only `session.json` and `shots/`. `log.jsonl`, `daemon.log`, `daemon.out`, `start-error.json`, `operator.json` (token, `cpu_bench_ms`) and `result.json` live in the operator dir (`--operator-dir`, default `<run>.operator`). The two dirs may not nest. `start`, `observe --truth` and `debug` need `PANEL_ROLE=operator`, and the CLI checks that before it sends anything. The daemon answers truth and debug only with a random per-session token, so a raw post to the port gets `ok:false`. panel.md: the operator starts sessions; persona agents run with cwd in `mktemp -d` and get only the CLI path and run dir. | `barrier.mjs` (new), `panel.mjs`, `daemon.mjs`, panel.md "Information barrier" and recipe steps 3 to 8 |
| 2. B8 idle gaming | `minutes_played` is now **engaged** time. Engaged act = a non-`wait` act that changed the player's own state (moved ≥ 0.05 m, turned ≥ 0.5°, cast, changed spell, toggled pause or lock, or changed outcome); it gets full credit. Other non-wait acts get ≤ 0.5 s. A wait gets ≤ 3 s in total, and only after an engaged act. An observe counts only next to an engaged act. B8 also needs `engaged_acts ≥ 2 × minutes`. New measured fields: `panel_minutes`, `engaged_acts`, `stalled_turns`. | `measure.mjs engagement()`, `score.mjs` |
| 3. Run quality | `score` fails all bars with `reason:"run quality"` plus `problems` when a run has panics > 0, `stopped_because:"error"`, turns < 20, more than 10 % of turns stalled, or a non-default throttle (`--allow-throttle` lifts only the throttle check). Model-routing mismatches go under `warnings`. | `score.mjs runProblems()` |
| 4. Throttle never verified | At start the daemon times a fixed JS loop on the blank page, after the throttle is set and before the game loads (fastest of 7). The result goes to `operator.json`, the start log entry and `result.cpu_bench_ms`. B4 needs qa/9yo > 2 (`throttle_ok`, `throttle_ratio`). | `daemon.mjs cpuBench()`, `score.mjs` |
| 5. ttm unreachable by a natural input | ttm = the first act that holds a movement key and moved the player ≥ 0.05 m across that act, or leaves the player more than 0.5 m from spawn. Documented in panel.md. | `measure.mjs` |
| 6. ttc definition | The caption rule stays (no chem diff). panel.md and the code say why: chem flags are coarse and change on their own (spread, drying), which would credit a sky cast. | `measure.mjs`, panel.md |
| 7. No negative control | `observe` and `act` return `stalled` (frame counter did not advance within 2 s). It is logged and counted in `stalled_turns`. The selftest adds a stub page whose state never advances. | `daemon.mjs`, `selftest.mjs` |
| Nits | `report` refuses a log with no stop entry and no cap. The static server now sends MIME types for `.json` and `.png`. When the daemon is gone, the CLI says `daemon is gone (... idle exit ... or a crash)` instead of ECONNREFUSED, and an idle exit writes its reason to `daemon.log`. panel.md notes that selftest panel seconds are not representative. | `panel.mjs`, `daemon.mjs`, panel.md |

## Tests first

`harden.test.mjs` (U1 to U11) and fixture fields were written before any implementation. With a stub `barrier.mjs` that throws:

```
$ node --test tests/e2e/panel/
# tests 26
# pass 7
# fail 19
```

All 12 new tests failed (U1 has two). The 6 existing score tests and the schema test also failed, because the fixtures now carry `cpu_bench_ms` and `engaged_acts`, which the old schema rejected. The 7 measure and report tests passed.

## Results (real output)

Unit (`node --test tests/e2e/panel/`): **26/26 pass** (14 existing + 12 new).

| # | Test | Result |
|---|---|---|
| U1 | `observe --truth` / `debug` without `PANEL_ROLE` exit 1 naming `PANEL_ROLE`, fake daemon receives 0 requests; operator truth carries token `tok-123`; `start` refused without role | PASS |
| U2 | `authorize` refuses truth/debug with no, wrong or empty token | PASS |
| U3 | operator dir default `<run>.operator`; nested either way refused | PASS |
| U4 | 90 × `wait 10000`: `minutes_played` 0, `panel_minutes` 15, B8 false; with observes between: 0; 300 observes: 0 | PASS |
| U5 | 200 × (`observe`, `key Shift 10000`) unchanged state: 1.667 min | PASS |
| U6 | synthetic real-play log still 0.16 min, 2 engaged; long waits capped at 3 s | PASS |
| U7 | qa throttle 1 / panics 3 / error / turns 9 / stalled 20 → `run quality` naming persona and field; `--allow-throttle` lifts only throttle | PASS |
| U8 | qa/9yo bench 4 → B4 pass; 2 or missing → `throttle_ok:false` | PASS |
| U9 | `key w 1500` moving 0.3 m sets ttm 4.5; `keys w+Shift` 4.0; `Shift` or `wait` with the same displacement null; 0.01 m null | PASS |
| U10 | report without stop entry refused, no result.json | PASS |
| U11 | B8 fails at 31 engaged acts for 16 min, passes at 32 | PASS |

Selftest on the real a94fc48 build (`node tests/e2e/panel/selftest.mjs`): **47/47 pass** (st6, 9 min 44 s wall at load 25). The same code passed 47/47 on the 9d030e0 build twice (st4, st5).

Selected lines from st6:
```
PASS S1 start without PANEL_ROLE=operator is refused
PASS S1 run dir holds only session.json and shots/ session.json,shots
PASS S1 log.jsonl, daemon.log and operator.json are in the operator dir daemon.log,daemon.out,log.jsonl,operator.json
PASS S1 session.json carries no token and no operator path
PASS S2 persona observe --truth refused
PASS S2 raw post of a truth observe without the token refused observe --truth needs the operator token (operator.json in the operator dir)
PASS S2 raw post of a debug command without the token refused
PASS 7 time_to_move_s is the panel time of the first w act that moved ttm=7.6 first moving act panel_s=7.6
PASS S3 cpu_bench_ms qa / 9yo > 2 qa=83.8 ms 9yo=36.5 ms ratio=2.30
PASS S1 result.json is written to the operator dir .../st6/b-qa.operator/result.json
PASS S4 stub: observe returns stalled:true
PASS S4 stub: act returns ok with stalled:true
PASS S4 stub: measured.stalled_turns counts both turns stalled_turns=2 turns=2
PASS S4 observe on the real game: stalled false / act: stalled false
```

Idle probe on a real build (9yo, 6 × (`observe`, `wait 10000`), stop, report): `panel_minutes 1.3`, `minutes_played 0`, `engaged_acts 0`, `stalled_turns 0`.

Repo tests: `npm ci --silent && npm test`: **34/34 pass**.

## Changes to existing checks (none weakened)

- Selftest check 7 used to compare ttm with the *last* `w` act of the loop. Under the new ttm rule it compares with the *first* `w` act that moved the player. It is still an exact equality. On st2 the old comparison failed (7.6 vs 9.1) for exactly this reason.
- Selftest operator calls now pass `PANEL_ROLE=operator`, and persona calls run with the variable removed. The `daemon.log` path moved to the operator dir.
- Existing unit tests are unchanged. `fixtures.mjs` gained `cpu_bench_ms` (qa 400, others 100) and `engaged_acts: 100`, so a healthy fixture panel stays healthy.

## Known limits and risks

- **The throttle ratio depends on box load.** Fastest-of-7 JS timings gave qa/9yo ratios of 3.56, 3.70 and 4.04 at load 6 to 9, but 2.30 at load 25. The first version used a median of 3 on the loaded game page and gave 1.15 in run st3, which is why the benchmark moved to the blank page and takes the fastest run. On a heavily loaded box, `score` can fail B4 with `throttle_ok:false`. That is the intended signal (qa was not really slower), but the panel should run on a quiet box.
- The barrier keeps out accidents and casual peeking, not a model that sets out to break it. A persona runs as the same OS user, so it could set `PANEL_ROLE` or read the default sibling operator dir. panel.md says so and makes such a transcript invalid. Passing an `--operator-dir` inside the repo, away from `$T`, helps.
- `stalled` costs up to 2 s of wall time per observe (it waits for a new frame). Panel time is unaffected.
- A black screen with a running frame counter is still visible only to the persona.
