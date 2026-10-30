# I-024 spec critique (fresh context)

**Verdict: REVISE (not reject).** The item is the right thing and the Part T / Part P core is buildable. Six changes are needed before lanes start. Two are correctness (R's runtime material swap, the noise-versus-threshold rule). The rest tighten tests and trim scope.

Checked against origin/main 265a93e (`world.rs`, `main.rs`, `Cargo.toml`, `build-web.sh`, `perf.mjs`), `target.md`, `bars.md`, `areas.md`, `gameplay-standards.md`, and `I-019/spec.md` (lanes and contract).

## Right thing for kids 4-18 and teachers
- Yes. B2 (smooth at 4x CPU) and B3 (load <= 8 s) are the school-Chromebook bars. Nothing visible changes except under `?low`, where it is guarded by test 9. Download growth is explicitly allowed by target.md, and the spec protects load time rather than bytes. Good framing.
- GS walk is fine (5, 6, 12, 19 only; no new screens or text). Add GS-5 explicitly: the p99 note is "noted, not gated". Since GS-5 is P0 (no hitch > 50 ms), gate it: a kept candidate must not raise median `max`/p99 by more than 10%, or state why it is deferred.
- Part T (phases) is the most valuable piece for the next item and costs little. Keep it always-landing.

## Smallest version: cut
1. **Drop P3 (naga).** It is conditional on P2 "within 2 ms" and adds a third 20-minute clean build. Make it a follow-up.
2. **Make W optional or cut it.** The spec's own spike says wasm-opt saved < 1% size at z. binaryen is not installed and has to be fetched. W adds public contract (`WASM_OPT_LEVEL`, test 4) for a speculative gain. Worse, "flip the default to `WASM_OPT=1`" makes `npm run deploy` and CI depend on `wasm-opt` being present on every machine. If W stays: the flipped default must fall back to a warning and a plain build when `wasm-opt` is missing, and the CI/deploy host check is stated. My recommendation: cut W and test 4 to a follow-up; keep the box time for P and R.
3. Candidate count is the real cost: P1, P2, (P3), W1, W2, R, plus final, each with 6 serial quiet-box perf runs and clean builds of 272 to 1 259 s. State a time budget and a stop order (P1 first; build P2 only if P1 fails rule 1 by less than 5 points or passes), as I-015 did.

## Tests that can fail or cannot fail
- **Noise vs rule (must fix).** The spec's own fact says base alone spreads 30.4 to 48.2 ms across rounds. Rule 1 is an 8% effect from 3 pairs with RUNS=1. Test 2's second example shows one pair set where a bad median passes. With that noise the rule will both false-keep and false-revert. Fix: 5 rounds for CPU (alternating order, base first in rounds 1, 3, 5), keep "<= 0.92 and wins >= 4/5", or RUNS=3 per pair. Update `pairOrder` and the contract accordingly. Also add a base-vs-base run through `perf-pairs` once, logged, as the noise figure (R's test 9 already does this for pixels).
- **TTI rules 2 and 3 are under the noise floor.** Default@4x TTI is 10.5 to 11.7 s on a loaded box, which is a 1.2 s spread, against a +500 ms limit. Use +1 000 ms for default@4x, or +500 ms on the median of paired differences with at least 5 rounds. Keep low@4x at +500 ms only if the same base-vs-base run shows spread under 300 ms; otherwise state the measured limit.
- **Test 5 is vacuous if nothing is kept** ("red for the kept profile"). Run it on P1 whenever P1 is built. Also, the crate name is `realgame-bevy-client`, so the rustc line is `--crate-name realgame_bevy_client`, not `bevy_client`. Fix the text.
- **Test 5 size rule is wrong as a reject.** "Within 1% of either end means the override failed" can fire for a correct P1 if our crate's generics are small. The `-v` line check is the proof; make the size check informational.
- **Test 1.** The spec says 46 existing cases pass. I ran `--self-test` on a copy of main's `perf.mjs` and got 45 passed, 1 failed (likely a path-relative lint case when run outside the repo). The lane should record the baseline count from the repo root before editing, and say "N existing cases" with N measured.
- **Test 3.** The sum identity (fetch + compile_init + first_frame + to_interactive = tti_ms) holds by telescoping, so it cannot fail except for nulls. That is fine as a guard, but the real failure mode is a negative phase (e.g. `STARTUP` logged before the wasm `responseEnd` if the resource timing entry is for a cached or streaming-compiled fetch: with streaming instantiation, `responseEnd` and compile overlap). State that `compile_init_ms` is then overlap-biased and name that in the contract, otherwise the next item reads the wrong phase. Anchor the STARTUP regex (`\bSTARTUP t=`), as the test already implies.
- **Test 8 numbers are a guess.** 440 texParameteri / 2 = 220 texture binds = about 20 material bind-group changes per frame. Getting to <= 200 texParameteri means <= 9 changes, which needs the 10 toon materials to collapse to 1 *and* nothing else to change. The 1 300 / 200 thresholds may be unreachable even if the hypothesis is right. Make the hard check "texParameteri falls by >= 30% and draws/frame is unchanged within 5%", and give 1 300 / 200 as the expectation.

## Part R correctness problems (must fix)
1. **Runtime swap bug.** `world.rs:804` builds `StateMats` for plain objects with `normal: m.clone()`, where `m` can be a palette material. If a chemistry object's piece is switched to the shared white material plus vertex-colour mesh, and the look code later restores `mats.normal` (the original palette handle), the piece gets palette colour times vertex colour (colour squared, visibly darker). The spec says "static scenery" and "StateMats unchanged" but does not say how to tell static from object pieces. Required: only pieces with no chem `Obj` and no `StateMats` are swapped, and the spec names the decision point (recommend `Builder::piece` callers that pass no obj, or an explicit `Builder::scenery` flag), plus a test that every chem object's `StateMats.normal` is not a shared-low handle (a unit test on `world::build` is not feasible without an App; use a golden-free check: toggle a bush through Wet and back under `?low` and compare its pixel to start).
2. **Kit meshes.** The kit has four: cube, ball, cyl and cone (`world.rs:186, 301`; `tree` uses the cone with `leaf`). The spec lists three and says "at most 10 x 3 meshes". Say four (<= 40), or say leaf cones stay unshared and then test 8 expectations drop further.
3. **Where the colour map lives.** The 10 palette handles are locals inside `build`, and `piece(&mut self, mesh, mat, t)` has no access to `Assets<Mesh>` or `Assets<StandardMaterial>`. Specify that `Builder` gains a `low: Option<LowShare>` holding the shared handle, a `HashMap<(mesh id, colour key), Handle<Mesh>>` and a palette `(Handle id -> LinearRgba)` table, and that the mesh copies are made from `meshes.get(kit.mesh)` clone plus `insert_attribute(ATTRIBUTE_COLOR, vec![[r,g,b,a]; n])`. The `LOW_SHARED_MAT` count (`pieces`) needs the same hook.
4. **Pipeline variants.** Vertex-colour meshes compile a different pipeline key (`VERTEX_COLORS`) than the grass/wall meshes and the unshared glow ones. That adds at most one pipeline, which is fine, but say it so the lane checks `first_frame_ms` did not grow (rule 3 already catches TTI).
5. The pixel test ceiling of 1.0 per channel is loose for an operation that should be bit-identical (vertex colour is f32 linear times white base). Use <= 0.5 and say the expected value is about 0.

## Lanes buildable from the contract alone
- **measure:** yes. Contract has field names, verdict shape, exit codes and the two pure functions. Add `perf-pairs.mjs --self-test` exit code, the `rounds` default change (above), and what `perf-pairs` does with `valid:false` after re-run (exit 2 is stated; say re-run count, 1 per `perf.mjs`).
- **build:** yes after test 4 is decided. The profile text is given verbatim. One gap: P2 names crates (`glam`, `wgpu-core`, `wgpu-hal`); the lane should check each name exists in `Cargo.lock` since a `[profile.release.package.<name>]` for an absent crate only warns. Say "`cargo build` prints no `unused manifest key` warning" as an acceptance line.
- **render:** not yet. See "Part R correctness problems". With those added it is buildable.
- **Seam:** the orchestrator-only serial box is a good call. The seam breaker's three checks are testable.

## Overlap with I-019 (lanes)
Both touch `crates/bevy-client/src/world.rs` and `main.rs`; neither touches `Cargo.toml`, `perf.mjs`, `build-web.sh`, `golden.mjs` or `web/**` in common. Specifically:
- **world.rs:** I-019 rust lane moves `Look` to `cues.rs` (re-export), rewrites the `hot` / `burning` lines in `state_mats` (~line 322-326). I-024 R changes `build` (signature, Builder, palette at 477-487). Different hunks, so a textual merge should work, but both lanes "own" the file. State the merge order: **I-019 lands first** (it is user-visible, has a bigger contract), and I-024's render lane rebases onto it.
- **main.rs:** I-024's one change is the `world::build(...)` call at line 386. I-019 adds spawn code, state keys and a caption elsewhere. No hunk conflict expected, but the golden `clock and budget` check (key count) is I-019's concern only: I-024 adds no state keys. Say so.
- **Measurement base clash (real).** I-024 judges everything against base 265a93e. If I-019 merges first, its streams and extra effects change CPU and the GL counts, so test 8 thresholds and test 9's "base vs R" are no longer vs. the merged tree. Rule: lane evidence is measured on 265a93e builds (as specced, for isolation). **Test 11 must be re-run with base = the origin/main at merge time**, rebuilt once, and the final wasm includes both items. I-019's own test 21 (CPU <= base + 1.0 ms) uses a different base, so the two items' numbers must not be mixed. Record both bases in the evidence.
- **Visual guard.** I-019 changes the picture (Water default, hot bushes non-emissive, streams). I-024's test 9 compares R against base on identical code, so it is unaffected; the final combined run needs no new pixel test, but say the poses in test 9 must be captured before I-019's merges are included.
- The quiet box is shared: I-019 test 21 also needs it. Add I-019 to the serial schedule.

## Specific edits requested
1. Rules 1 to 3: 5 rounds for CPU and TTI, wins >= 4/5, TTI limit raised or justified by a base-vs-base spread; update `pairOrder`, test 2 and contract.
2. Cut P3 and (recommended) W with test 4, or add the `wasm-opt`-missing fallback and CI note.
3. Test 5: run on P1; fix crate name; size check informational; add "no unused manifest key" line.
4. Test 1: baseline case count measured from repo root.
5. Test 3 / contract: say `compile_init_ms` may overlap fetch under streaming compile; negative stays flagged.
6. Part R: 4 kit meshes; static-vs-object rule with the StateMats restore hazard and a Wet-and-back pixel check; Builder design sketch; test 8 as relative drop; test 9 ceiling 0.5.
7. GS-5: gate p99 / max, or state why deferred.
8. I-019 overlap: merge order, final test 11 re-based on merged origin/main, shared quiet box schedule.

## Planner response

Spec revised to revision 2 (`spec.md`). Each change, accepted or rejected:

- **GS-5 gate (p99/max):** accepted for p99 (new keep rule 5: median low@4x p99 <= 1.10 x base). Max is recorded, not gated: a single run's max is one frame and too noisy to gate.
- **Cut 1, drop P3:** accepted; moved to Out of scope as a follow-up.
- **Cut 2, Part W:** accepted (cut, not made optional); `WASM_OPT_LEVEL`, test 4 and test 7 removed (numbers kept as "cut"), `build-web.sh` leaves the build lane.
- **Cut 3, budget and stop order:** accepted; order noise -> P1 -> R -> P2 (only if P1 kept or CPU ratio <= 0.97 with rules 2-5 passing) -> final, max 5 verdicts and 8 h of quiet box, P2 dropped first.
- **Noise vs rule 1:** accepted; 5 rounds, base first in 1/3/5, wins >= 4/5, plus a logged base-vs-base `noise-pairs.json` run; `pairOrder`, test 2 and contract updated.
- **TTI rules 2/3 under noise:** accepted; both judged on median paired differences over 5 rounds; default@4x +1 000 ms; low@4x +500 ms unless the noise run's spread exceeds 300 ms, then the spread rounded up to 100 ms, capped at 1 000 ms (computed by `keepRule`, not by hand).
- **Test 5 vacuous / crate name:** accepted; runs on P1 whenever built, uses `realgame_bevy_client`.
- **Test 5 size reject:** accepted; size is informational, the `-v` lines are the proof.
- **Test 1 baseline count:** accepted; N measured from repo root before editing and recorded.
- **Test 3 overlap / anchored regex:** accepted; streaming-compile caveat in Part T and the contract, negative `compile_init_ms` flagged not failed, regex anchored with an extra `XSTARTUP` case.
- **Test 8 thresholds:** accepted; hard check is texParameteri <= 0.70 x base with draws within 5%; 1 300 / 200 kept as expectations.
- **R1 runtime swap bug:** accepted; swap is an end-of-`build` post-pass that skips every entity referenced by a `Target` (bodies, wall, puddle, flames, foam), plus a Wet-and-back pixel round trip in test 9. Chose the post-pass over a `Builder::scenery` flag because it does not depend on every call site being tagged correctly.
- **R2 four kit meshes:** accepted; cube, ball, cyl, cone (leaf cones included), at most 40 meshes.
- **R3 where the colour map lives:** accepted; `Builder.low: Option<LowShare>` with shared handle, palette `AssetId -> LinearRgba`, `(AssetId<Mesh>, colour bits)` cache, candidates list and `pieces` count; copies made in the post-pass from `meshes.get(..).clone()` + `insert_attribute(ATTRIBUTE_COLOR, ..)`.
- **R4 pipeline variant:** accepted; stated, with a check that median `first_frame_ms` grows by no more than 100 ms.
- **R5 pixel ceiling:** accepted; 0.5 per channel, expected about 0.
- **Measure lane contract gaps:** accepted; `--self-test` exit codes, `--rounds 5` default, one re-run per invalid run, `--noise` input, new verdict fields.
- **Build lane Cargo.lock / unused manifest key:** accepted; P2 names checked in `Cargo.lock` before building, and "no `unused manifest key` warning" is an acceptance line.
- **I-019 overlap:** accepted; I-019 merges first and I-024 render rebases; lane evidence stays on 265a93e; test 11 re-based on merge-time origin/main with both bases recorded; test 9 poses captured before I-019 is included; I-024 adds no state keys; I-019 test 21 joins the serial quiet-box schedule.
