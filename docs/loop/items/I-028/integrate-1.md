# I-028 integration, round 1 (wave 12)

Branch `loop/I-028` (was the contract head 230de1b on dc1d336). The integrator merged the sim lane, then the
client+web lane, both `--no-ff`, as the order requires.

## Lanes merged

| lane | branch, head | state when handed over |
|---|---|---|
| sim | `loop/I-028-sim`, 4239d04 | **Not finished by the lane builder.** Tests 1-7 (dca7530) were committed. The implementation was only in its worktree and nothing was pushed. The integrator finished the lane in that worktree: one clippy fix (`type_complexity` on `coefs`, now a `Side<'a>` alias); `fmt`, both-feature clippy and tests re-run; commit 9789c6b; lane notes `lanes/sim/round-1.md` (4239d04); push; worktree removed. No behaviour beyond what the lane builder wrote. |
| client + web | `loop/I-028-cw`, 0f9a352 | Done (golden 159/169 on the stub; the 9 bench failures needed the sim). |

Merges: 528d324 (sim), b8cf7fe (cw). **No conflicts** (the lanes own disjoint paths).

Spec conflict recorded by the sim lane: the `Prompt` rule runs before the reference checks. That reorders I-026 test 4i, so its setup now gives fire a `prompt_sprout`, and a new assertion pins the order. See `lanes/sim/round-1.md`.

## Checks on the merged tree (b8cf7fe)

| check | result |
|---|---|
| root `cargo test --locked` | 217 passed, 0 failed |
| `cargo test -p realgame-sim --no-default-features --locked` (sim lane head) | 61 passed |
| `cargo fmt --all --check` (root and client) | clean |
| root sim clippy `-D warnings`, both feature sets (sim lane head) | clean |
| client `cargo test --locked` (native) | 73 passed |
| wasm `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings` | clean |
| `npm test` | 82/82 (science lint: 42 client claims, 42 sourced) |
| `build-web.sh` | ok; `pkg-out` holds `bench.js`; wasm gzip 5,102,307 B |

## Golden: one full run (`BASE_URL= QUERY=?low`, under the Chromium lock)

`evidence/integrate/golden-full-1.txt`: **184/186**, 21:21 to 22:01 UTC.

| FAIL | kind |
|---|---|
| `clock: cast cadence keeps the 0.3 s cooldown in wall time` (`casts=8 need 9 expectedGame=3.750 frames=16`) | known wall-time flake |
| `bench: keyboard only reaches Water and builds it (focus never leaves the bench)` (`reached=true inside=false`, build `ok`, learned) | **integration bug, fixed in e3dc0af** |

`cast: a real click casts exactly once` passed this time. 46 of the 47 bench checks (tests 12-28b) passed, including every check that failed on the stub (16 list, 17-25, 28a ring removal, 28b `not_for_band`).

### Fix e3dc0af (web lane path, `bench.js` + 1 line in its `style.css` block)

Picking a spell from the list removes the focused card. Sprout and Seedling then show "Getting the bench ready" until the game publishes the recipe on a later frame. That wait view has nothing focusable, so focus fell back to `<body>`, and the test's per-key "focus is inside `#bench`" probe caught it. On the stub this test never got that far. Now the wait line takes focus (`tabindex=-1`, so it is not tabbable; no outline). `npm test` and tsc stay green.

Verification: the `formulaBench` section alone (the current golden's `formulaBench` on the golden harness, with a per-key focus diagnostic), with the fixed `bench.js`/`style.css` copied into `pkg-out` (the wasm is unchanged): **48/48** (`evidence/integrate/formulaBench-only-after-fix.txt`). No second full golden was run (one was budgeted). The fix only touches the open bench's list-to-build step.

**Golden verdict:** no non-flake failure remains after the fix. The full run itself had one non-flake failure, now fixed and verified by the section run.

## Test 29 (analytics-e2e), built site

`web/dist` was assembled by `scripts/build-site.sh`'s steps from the integrated `pkg-out` (b8cf7fe), without rebuilding the wasm. Run: `SKIP_BUILD=1 node tests/e2e/analytics-e2e.mjs` under the lock: **62/62** (`evidence/integrate/analytics-e2e-1.txt`).

- `bench: a picker pick (8 to 10) opens the bench on the Seedling list, no chooser` `{"open":true,"chooser":0,"cards":4}`
- `bench: with DNT (no picker) the bench asks with its chooser` `{"open":true,"chooser":3,"cards":0}`
- `bench run: 0 console errors and 0 page errors`

All earlier sections, the D1 row checks included, still pass.

## Test 31 (perf guard)

See `evidence/perf.md` and `evidence/perf-pairs.json`. Median low@4x CPU p50: base 28.2 ms, cand 27.4 ms. The bar is base + 1.0 ms, so it **passes**. perf-pairs exited 1 on its own keep rule, which the spec says to ignore. Brotli of `pkg-out` grew 22.3 KB against a 40 KB limit: **passes**.

## Hygiene

- `cues.rs`, `feel.rs` and `main.rs` stream code: untouched by the merge and the fix.
- The base worktree and the copied `pkg-out` in the scratchpad were removed. So were `web/dist` and the sim lane worktree.
- `/home/claude/bevy-target/debug` (5.4 GB after the client native tests) was deleted. No cargo process was running at the time.

## Not done here

The seam breaker, test 32 (golden count) and the science reviewer are left to their own roles.
