# I-035 round 1: Builder, all lanes (world, core, web, tester's golden section and perf --query)

Wave 13. Branch `loop/I-035`, worktree `.claude/worktrees/w13-I-035`. Built on loop/I-035 5b9d833 with
origin/loop/I-028 fed49a9 merged (74cb368), as directed. The refreshed line references are in spec.md,
"Refresh (wave 13 build)".

## What a player gets

Win the courtyard and the dark arch in the south wall lights up ice-blue. The banner says to walk to it,
and 3 s later the finder arrow points at it. Walk in and you are in a snowy valley:

- A stream crosses the valley. It is solid while liquid, so you stop on the bank.
- Frost freezes it a third per cast (0.34, 0.68, 1.00). Ice crusts grow in from both banks, and 4 beads
  (6 at full tier) skate out in white mist on every freezing hit.
- At 1.00 the stream is a slippery bridge and you cross it (`onIce`).
- Fire melts the ice gate in 3 hits.
- Walk up to the lantern: "You opened the Frozen Gate!"
- The back arch returns you to a fresh courtyard, whose exit stays open.

Wrong chemistry fails kindly, and every band gets its own lines. Screenshots are in `evidence/`:
`level2-arrive.png`, `level2-bridge-frozen.png`, `level2-gate-melting.png`, `level2-lantern.png`.

## Order of work

1. Merged I-028 and refreshed the line references (8032509).
2. Wrote the acceptance tests first on contract stubs (dcf70b9):
   - chem: tests 1 and 2, plus a level-1 guard;
   - level2: tests 3 and 4, including the bank push-out;
   - golden: `frozenGate` (tests 5-13, 16 entities, 18, `?level=2`) and the `I035_KEYS` key order (14);
   - `perf.mjs --query`.

   On the stubs, all 7 new Rust tests failed and the level-1 guard passed. Evidence:
   `evidence/tests-first-red.txt`.
3. Built the level in the three lanes (efe5cdb), then the bench guard line (5a0fa64, see below).
4. Fixes from the first golden runs (b58d112, 1ac36ac).

## Contract notes and decisions

- **Events.** `EventKind::Chilled` already existed, so it is reused. `EventKind::NoChange` is new (CO2 on
  `Stream`/`IceWall`). Its default caption is the Seedling CO2 line, so level 1's ice wall now says that
  line instead of `Smothered`'s blanket line. Frost on an `IceWall` now always emits `Chilled`; before,
  it refroze a partly melted wall to 1. No level-1 test depended on that.
- **Goal::Reach.** Turns off both `reseed` and the fire win (`live_outcome`). `World::new` stays `Fires`.
- **Bench guard.** bench.rs has exactly one changed line: `at_bench` adds `&& crate::bench_on()`, an atomic
  that main.rs sets false in level 2. The pad's meshes are hidden from main.rs: a Startup system ordered
  after `spawn_station` tags them `BenchPad`, which avoids touching I-028's spawn code.
- **Level switch.** `levels` despawns every `LevelRoot` entity, then builds the other level. `world::build`
  makes a new `Kit` each time; the old handles drop with their entities.
- **Ripple marker.** The ripple marker is a component, not a resource. A level-2 resource left one extra
  entity behind after a visit, because resources are entities in Bevy 0.20: 633 became 634. Test 18 caught
  it.
- **Exit arrow.** It uses the existing `fireBearing`/`fireOff` fields when nothing burns, and
  `#firefinder.exit` draws an arch instead of a flame. It waits 3 s of game time after the win, because
  I-025's golden test 18 ("nothing shows under the win banner") checks the first 2 frames. This is a
  judgement call between that test and spec step 0, so reviewers please check it.
- **Ripples.** They follow the Hand sway setting, which defaults off under prefers-reduced-motion. They
  are static while paused or once frozen.
- **Science.** `[S24]` comes from Wikipedia "Leidenfrost effect" (fetched 2026-11-01), which cites the
  Quéré review. The review could not be fetched: doi.org, annualreviews.org and crossref were refused by
  the egress proxy.
  - No fetched text supports drops gliding on *water*, so no line says "on the water".
  - The Seedling line "boils so fast it rides on a cushion of gas" was softened to "makes a cushion of its
    own gas and rides on it". The source says the vapour layer keeps the drop from boiling rapidly. The
    spec table and the tests were updated to the softened line.
  - New rows: C-049..C-072, P-020 (`STREAM_FREEZE`), and Not-claims rows for the arrival and objective
    lines, the banners, and the `freeze`/`melt` icon keys.
- **Golden edits outside the new section.** Besides the sanctioned `I035_KEYS` edit, two older checks
  asserted that nothing follows I-027's keys, which cannot hold together with test 14. Each now pins the
  four appended keys exactly, as I-027 did for its own keys (42cb448):
  - I-025 streams "fireInView is a boolean, the last state key";
  - I-027 progress "keys in order".

  Golden test 12: the section first stops test 10's leftover scripted walk (`move 0 0 0`). Otherwise
  the walk carried the player off the arch in the frame of the `tp`.
- **Running the section alone.** `ONLY=frozenGate` in golden.mjs runs just the section and the
  console-error check.
- **hud.js.** The pointer-lock block is untouched.

## Results

| check | result |
|---|---|
| `cargo fmt --all --check` (root) and `cargo fmt --check` (client) | clean |
| `cargo clippy --all-targets --locked -- -D warnings` (root, sim) | clean |
| `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` (client) | clean |
| `cargo test --locked` (root, sim) | 217 passed |
| native `cargo test --locked` (client; temp target dir, deleted after) | 81 passed (8 new I-035 tests) |
| native client clippy `--all-targets` | not clean, but only in files this item does not touch (bridge.rs dead fields on native, progress.rs, clock.rs double module); CI lints the client on wasm32 only |
| `npm test` | 82/82 (science lint included) |
| `npm run typecheck` | clean |
| `node tests/e2e/perf.mjs --self-test` | 50/50 (pageUrl query and isLevel2 tests added) |
| `build-web.sh` | ok; wasm brotli 3.427 MB vs 3.418 MB for the I-028 build (+0.009 MB) |
| golden `frozenGate` alone (`ONLY=frozenGate QUERY=?low`) | **16/16**, no console errors (`evidence/golden-frozenGate-alone.log`) |
| full golden run 1 (`QUERY=?low`) | 198/201: 3 failures (I-025 fireInView key position, I-025 test 18 win banner, I-027 progress key tail), all fixed in 1ac36ac; frozenGate 15/15 inside it, no console errors (`evidence/golden-full-run1.log`) |
| full golden run 2 (`QUERY=?low`, after 1ac36ac) | **200/201**. The one failure is `cast: a real click casts exactly once` (casts 1 -> 2 -> 2), a known wall-time flake. The three run-1 failures now pass; frozenGate 15/15 inside it; no console errors (`evidence/golden-full-run2.log`) |

### Perf (test 16), same build, low@4x, 5 interleaved pairs (`evidence/perf/`)

| | level 1 | level 2 (`--query level=2`) |
|---|---|---|
| CPU p50 per run (ms) | 27.7, 28.0, 29.8, 28.1, 28.7 | 29.4, 29.4, 26.5, 26.8, 29.2 |
| median CPU p50 | 28.1 | 29.2 |
| paired difference (l2 − l1) | | +1.7, +1.4, −3.3, −1.3, +0.5; median +0.5 ms; level 2 lower in 2 of 5 |
| median CPU p99 | 45.3 | 59.1 |
| median TTI (ms) | 5761 | 5488 (builds level 1, then level 2 on its first frame) |
| entities (golden STATS) | 633 | 458 |
| draw calls per frame (PERF_GL_COUNT, low@1x) | 46.6 | 25.7 |
| requested brotli | 3.632 MB | same build |

- **CPU p50.** The median paired difference is +0.5 ms. The pair differences run from −3.3 to +1.7 ms and
  split 2/3, about as wide as level 1's own run-to-run spread (2.1 ms). So level 2 is within the noise
  band, though not shown faster.
- **CPU p99** is higher in level 2: 59 vs 45 ms. The level-2 workload casts on the stream and gate, and
  each Frost hit adds beads and a mist burst. This is worth a look in break, but no bar is on p99 here.
- **Entities and draw calls** are well below level 1's.
- **Size.** Brotli grows by less than 0.01 MB.

## Not done / follow-ups

- F1 to F3 as filed in the spec.
- The arch has no picture label in level 1 (spec: none needed). Level 2's back arch carries a small
  glass-and-leaf greenhouse model on its lintel.
- The exit-arrow delay (3 s) is the reviewer's call against the I-025 banner rule.
