# I-019 spec critique

**Verdict: revise.** The direction is right and most tests can fail on 3dbd332. The item is bigger than the smallest version, one test is wrong by arithmetic, and the contract misses one Rust site the `fire` command must touch.

Read: target.md, bars.md, areas.md, spec.md. `gameplay-standards.md` is not in `docs/loop/` on this branch (it exists only under `.claude/worktrees/wf_*/docs/loop/`), so I used the worktree copy for the GS items. The primary checkout also has no `crates/bevy-client/src` at its HEAD, so I checked feasibility with `git show 3dbd332:...`. Both are planner hygiene issues; the spec's GS walk is consistent with the worktree copy.

## Right thing for the players: yes

- The first-person rule is respected: latched click, a stream from the gauntlet, a crosshair that separates fire from bush, and an arrow to the last fire.
- The problem matches B4. Nine blockers from 5 personas, all traced to causes visible in the code: lost clicks, 0.6 s flights, Fire as the default spell, a counter that does not match the fires, and no way to find a fire.
- The chemistry rules are left alone. Water as the default does not remove the spell choice (GS-30), and the spec says so.
- The prediction (blockers 9 down to at most 2, clarity +1) is falsifiable by the paired panel run.

## Smallest version: not yet. Cut these

1. **Cut the CO2 sinking cloud (item 4).** That covers `P::Co2`, `CO2_SINK`, the `fxLive` and `co2Y` fields, the P-016 science row, test 13 and the `fx.rs` assertion in test 20.
   - It serves GS-32 (a P1, and "improved" is not a requirement), not "a 6-year-old sees the click work".
   - It brings a B6 science row with an invented tuning constant, and the only test that checks it is flaky (see below).
   - Keep the CO2 stream as a pale entity in the existing foam colour. Tell the planner to file "CO2 cloud that sinks" as a separate item.
   - `fxLive` can shrink to `fxWater` (a u32), or the splash check can use the existing `impacts` plus the one count test 12 needs.
2. **Defer `#pausebtn`.** Esc-while-locked and `KeyP` fix the reported "Esc did not work". The button is a P1 for point-mode and touch players, and the target is "WASD + mouse". It adds CSS, a propagation rule, hint and banner rules, and half of test 17. If it stays, say why in the GS walk. I recommend deferring it with one backlog line.
3. **Fire and CO2 streams reuse the Water code.** The spec already implies this. State it in the Rust lane: one `spawn_stream(material)`, not three paths.
4. Everything else (latch, flight cap, Water default, one counter, finder, `fire` ring, hint priority, Esc/P) is the minimum for the stated goal. Keep it.

## Can every test fail? Problems

- **Test 13 is wrong by arithmetic and by frame rate.** It asserts that `co2Y` falls at least 0.1 m in 10 frames.
  - At dt = 0.05 and gravity −0.8, 10 frames give 0.5·0.8·0.5² = 0.10 m, which is exactly the threshold.
  - At a normal 16 ms frame, 10 frames is 0.16 s and a 0.01 m fall.
  - The mean over live particles also shifts as old ones expire or hit the floor.
  - This goes away with the cut above. If the cloud stays, make the test time-based on game seconds (a `gameTime` field) and assert a drop for a fixed 0.5 game seconds against the floor-clamped mean, with a margin of at least 2×.
- **Test 12 pixel check can be flaky.**
  - A screenshot takes wall-time. The stream lives about 0.45 game seconds, which can be only a few frames of wall time on the software GPU.
  - Mitigation: have the test take the "before" shot twice to measure noise. Pass if the delta exceeds the noise by 0.3%. Keep "3 attempts", and log all attempts.
  - Consider capturing in the frame `streams` first goes ≥ 1 by pausing the sim there (`__game.pause(true)`), then taking the screenshot. That removes the race.
- **Test 16 turn direction is under-specified.**
  - It says `turn ±150 0` "toward the arrow". Positive bearing means right, but in `player.rs` forward is `(-sin yaw, -cos yaw)`, so a positive yaw turns left.
  - Say which `turn` sign moves the view right (read it from `player.rs` and the `turn` handler). Otherwise the web lane guesses wrong and the loop never terminates.
  - Also say what happens near |bearing| ≈ 180, where the side flips each frame. The test reads state and DOM in one `evaluate`, so it should only assert the class equals the sign of the `fireBearing` read in that same `evaluate`. Write that down.
- **Test 11:** its `K = max(10, ceil(0.25/dtMean)+2)` is sound (the game clock is clamped to 0.05, so at least 5 frames). Courtyard HALF = 24 m, so an 18 m pose exists. Name the pose in the spec, for example `tp 0 -20` toward a fire at about z = −2, or say "the builder picks and logs it" and accept that.
- **Test 8 and test 10 use synthetic events.** State that the `hud.js` pointerdown handler must not gate on `isTrusted`. A short line in the web lane is enough.
- Tests 7, 8, 9, 14, 15, 17 and 18 all name a fact that is false on 3dbd332. Good.
- **Test 19** (GS-6 flicker) is a guard, not a failing test today. That is acceptable for a P0 guard but label it as such. The two-scenario version is enough; drop the "6 clicks in 2 s" case if the 20-frame pair check already covers Water.

## Buildable from the contract alone? Almost. Fix these

- **Missing Rust site.** In `main.rs` (3dbd332) the command filter is `c.starts_with("tap") || c == "cast"` (line 662). `fire` is dropped there unless added. `aim_and_cast` also matches `a.first() == Some(&"cast")`. Add both to the contract: "`fire` is added to the line-662 filter and to the cast branch in `aim_and_cast`".
- **`clicks` semantics.** It counts commands received, but the queue is cleared while paused. Say whether `clicks` counts before that clear (as the spec reads) so that a `clicks` +1 and `casts` +0 while paused is legal.
- **Contract stub.** It says `fire` is "an alias of `cast`" in the stub, but the web lane's test 8 needs the latch behaviour. That is fine, because the queue already exists and an alias is the real implementation. Say so, to stop the Rust lane from re-deriving it.
- **`flight_secs(spell: u8, ...)` but item 2 also says "`cues.rs` has its own copy of `Spell`".** Pick `u8` (the contract does) and delete the other option.
- **I-007 dependency.** Test 15, 16 and 8 rely on I-007 fields (`hitKind`, `fov`, the `on` class). Add to the lane table that the web lane's golden code is written against I-007's merged `golden.mjs` helpers (`face`, `castAt`, the pose), and that the planner refreshes `based_on` before dispatch (already stated).
- Lane ownership is clean. `world.rs` and `cues.rs` are both Rust lane, so moving `Look` into `cues.rs` is safe. `science-sources.md` rows are web lane only. `bridge.rs` is in the Rust lane list but not in `areas.md`'s client-world row. It is in client-core, so say which lane owns the doc comment or drop it.

## Smaller notes

- The counter can read, for example, "Fires to put out: 7" while the win needs 3. It is honest and test 14 pins it. Make sure the panel brief tells personas this is intended, so it is not reported as a bug.
- Hot bushes lose their emissive. Say in the Rust lane that `Burning` keeps its emissive unchanged, so a fire is still the brightest thing in the courtyard.
- B2 and B3 guards (test 21) are reasonable. Cutting the CO2 cloud lowers the risk.

## Specific changes requested

1. Remove item 4, P-016, `co2Y`, `fxLive[7]`, test 13, and the `CO2_SINK` assertion in test 20. Keep a pale CO2 stream. File a follow-up.
2. Defer `#pausebtn` and its half of test 17 (and the `#pausebtn` hidden check in test 18). File a follow-up.
3. Add the `fire` filter and `aim_and_cast` match to the contract.
4. Fix test 16: state the `turn` sign convention and assert only same-read consistency.
5. Make test 12's pixel check noise-calibrated, or capture it paused on the first stream frame.
6. Remove the "own copy of `Spell`" option from item 2.
7. Say that the `pointerdown` handler must not check `isTrusted`.
8. Note that `gameplay-standards.md` is missing from this branch's `docs/loop/` and the primary checkout lacks the crate sources.

## Planner response

Spec revised to revision 2. All 8 requested changes are accepted, along with the inline notes.

1. **Accept.** Removed item 4, P-016, `co2Y`, `fxLive[8]`, test 13 and the `CO2_SINK` lint assertion. CO2 keeps a stream in the existing foam material. `fxLive` shrinks to `fxWater`. "CO2 cloud that sinks" is listed as a follow-up in Out of scope. Reason: GS-32 is a P1 and the cloud does not serve the one-click goal.
2. **Accept.** Deferred `#pausebtn`, and removed it from test 17, test 18, the contract, the lanes and the GS walk (GS-7/11/22 rewritten). It is a follow-up. Reason: the target is WASD + mouse, and Esc/P fix the reported blocker.
3. **Accept.** The contract stub now makes the real `fire` change: the line-662 filter, `Some(&"cast") | Some(&"fire")` in `aim_and_cast`, and `clicks` incremented before the paused-queue clear (line 847). Reason: verified at 3dbd332, where `fire` would otherwise be dropped.
4. **Accept.** The facts now state the turn sign: `turn dx` does `yaw -= dx·k`, so a positive `dx` turns right (verified in `player.rs`). Test 16 steps `turn +150` on `right` and `turn -150` on `left`, and asserts only same-`evaluate` consistency. Reason: removes the guess and the ±180 flicker failure.
5. **Accept (both).** Test 12 takes two before shots for noise, pauses on the first `streams ≥ 1` frame via a rAF watcher, and requires a delta of noise + 0.3%. It keeps 3 logged attempts. Reason: removes the wall-time race.
6. **Accept.** `flight_secs(spell: u8, ...)` only, and the "own copy of `Spell`" option is deleted. Reason: matches the contract.
7. **Accept.** The e2e preamble and the web lane both say the `pointerdown` handler must not gate on `isTrusted`. Reason: tests 8 and 10 use synthetic events.
8. **Accept.** The revision note at the top records that the primary checkout (661b9ba) lacks `gameplay-standards.md` and the crate sources, and that the facts come from `git show 3dbd332:`. Reason: hygiene, and builders must branch from real origin/main.

Inline notes:
- **Test 11 pose:** accepted. `tp 0 -20` facing the counted fire nearest z = −2, with a logged fallback.
- **Test 19:** accepted. Labelled as a guard, and the "6 clicks in 2 s" case is dropped.
- **I-007 helper dependency:** accepted. Added to the e2e preamble and the web lane.
- **`bridge.rs`:** accepted. Dropped from the Rust lane, because it is not edited.
- **`Burning` emissive unchanged:** accepted. Added to item 8 and unit test 5.
- **Panel brief note on "Fires to put out: 7":** accepted. Added under Out of scope.
- **Stream code:** accepted. One `spawn_stream(material)` is stated in item 3 and in the Rust lane.

No rejections.
