# I-022 spec critique

**Verdict: revise (small).** Right thing, close to the smallest version, and most tests can fail on a3e50b1. One Rust-lane statement is wrong against the code, one e2e test is timing-flaky, and a few gaps need a sentence each. No cut of scope is needed.

Read: spec.md, I-019 spec.md, and target.md, bars.md, areas.md, gameplay-standards.md from origin/main. Feasibility checked against `origin/main` source (hud.js, age.js, main.rs, analytics.ts, golden.mjs, analytics-e2e.mjs).

## Right thing for kids 4-18 and teachers: yes
- It closes the six should-fix findings of two breaker reports and nothing else. The first-person rule is untouched.
- Hints off, Aim help off and View width are teacher and classroom controls (GS-1, 10, 24, 26). The defaults are unchanged, so a 4 to 8 year old on defaults sees no difference. The start line and the point-mode note stay with Hints off, so the mouse-only path (GS-11) is still explained.
- The wrong-player-count fix (B7) is a real error in the number the teacher reads. Data collected is unchanged, so it does not touch the human-only "change what data is collected" rule. Say so in one line in the PR, because the dashboard's meaning of `players` changes.
- 4:3 projectors are real in classrooms, and the 21:9 113 degrees breaks the GS-1 ceiling. Worth doing.

## Smallest version: acceptable
Six small hunks, each tied to a named finding. Splitting would cost more than it saves (two lanes, one shared build). Keep it as one item. Do not add anything.

## Must fix before dispatch
1. **Rust lane text is wrong about the camera block.** The block at main.rs ~757-767 is inside `control`, which already takes `mut settings: ResMut<Settings>`. So "Its `settings` parameter becomes `ResMut<Settings>`" is a no-op. The real problem: `control` has neither a `Window` nor a `Camera` in scope. Its camera query is `Query<(&mut Transform, &mut Projection), With<FpsCam>>`, and `logical_viewport_size` lives on `Camera`. The lane cannot write "the same expression `aim_and_cast` uses" without new parameters. Fix the spec by choosing one:
   - (a) add `&Camera` to the `control` camera query tuple and `window: Single<&Window, With<PrimaryWindow>>` for the fallback (adjust the `#[allow(clippy::too_many_arguments)]` and the non-wasm `cursor` `Single<..., With<PrimaryWindow>>` conflict check: two `Single`/Query on `Window` is fine, but the lane must build native to confirm), or
   - (b) write `settings.aspect` in `aim_and_cast` and make that parameter `ResMut<Settings>`, since that system already computes the aspect. It runs every frame, so `control`'s projection reads it one frame late, which the spec already tolerates ("within 10 frames").
   Recommend (b): fewer new parameters, one aspect expression, and the ray and projection come from the same write. Add: `Settings` is written every frame, and nothing uses `is_changed` (checked: no hits), so that is safe.
2. **Test 13 is timing-flaky as written.** A fixed 3 s delay on `age.js` races golden-style boot. `firstState` can take longer than 3 s on cloud software GL, so the picker appears before the click and the precondition fails, which is a false failure. Replace the fixed delay with a held route: the `page.route` handler awaits a promise that the test resolves right after it has confirmed `pointerLockElement?.id === 'game'` (or after a 60 s cap, which then fails the check). This also removes the wall-clock wait.
3. **Test 13/14 click path needs `#loading` gone.** `#loading` is a fixed full-screen overlay. State that the click happens after `firstState` and that `#loading` is hidden (check `#loading` hidden or `elementFromPoint` is the canvas), else the click lands on the overlay and no lock is taken. If boot hides `#loading` only after the first frame, say so in the precondition.
4. **Playwright version claim.** "Playwright 1.63" is not checked in the spec and `node_modules` is not in this checkout. `page.clock.setFixedTime` needs 1.45 or later. Have the web lane read the pinned version in `package.json` first, and if it is older, say the test is blocked rather than rewriting it. Also confirm that `setFixedTime` moves `Date` for `dailyId` only and that `performance.now()` based `dur` is not affected (session_end needs `dur`).

## Tests: can they fail?
- 1 to 3, 4, 6: yes, they fail on a3e50b1 (functions missing, id random, `players.today` 1).
- 7: fails today. Note that `#hint` may be unrelated-visible due to `note`; the test starts from `fresh()` so `note === ''`. Say that `fresh()` clears `note`, or the test could pass-by-accident on a leftover note.
- 9: relies on a near-miss search that "fails if none found". Fine, but state the fallback pose is fixed (`tp 4 2`, `bushes[5]`) so the lane does not guess.
- 10: the 21:9 case needs a 1680x720 canvas. Confirm the page resizes the canvas on `setViewportSize` (the existing layout check at golden.mjs:826 already does, so the harness supports it). Also: existing check at golden.mjs:473 and 934 run at 1280x720 only, so no existing check changes. Good.
- 12: see item 4 above.
- 14: also assert `#age` stays outside the four inerted elements (it is a sibling in index.html, so the pick buttons stay clickable). One line.
- 18 (perf guard): fine. It adds no script change.

## Gaps (one sentence each)
- **hintAllowed edge:** with Hints off, a locked first-time player with `casts === 0` sees no coaching line. That is the intent, but state that the objective line and the pause controls table remain (GS-24), so the reviewer does not read it as a blocker.
- **funnel after midnight:** `players` now counts `session_start` only, while `funnel` still counts `quest_complete` by `day||daily_id`. A quest completed after midnight can show on a day with 0 players. It is listed out of scope; add one line to the Problem section so the dashboard reader is not surprised, and file the follow-up in the backlog.
- **`dayBands` zero row:** the spec says it "must still return a 0 row where the shape needs one". Name the existing test that proves it ("stats aggregate players, bands...") and add that assertion to test 6 for a band with no session_start that day.
- **Picker listener leak:** the late-lock `pointerlockchange` listener is removed on pick, but if `startAgePicker` returns false (opt-out) nothing is added. Say that explicitly so the lane does not add it earlier.

## Overlap with I-019: none that blocks
- `main.rs`: I-022 touches `Settings`, the camera block / `aim_and_cast` lines 875 and 879. I-019 touches the line-662 filter, the cast branch (~904), `Projectile.dur`, and the format string. Hunks are disjoint. If critique item 1(b) is taken, I-022 also edits the `aim_and_cast` signature line (`settings: Res` to `ResMut`), which is next to I-019's `flight_secs` edit in the same function. That is a textual-conflict risk, not a semantic one. Mention it in the "Clear of I-019" table.
- `hud.js`: `setHint` is the only shared function. I-019 rewrites hint priority and banner suppression. The spec already says the merged `setHint` must route through `hintAllowed`. Add a test-side guard to the web lane: I-019's `Aim at a fire and click.` locked hint must also vanish with Hints off (the test 7 design, which never reads wording, already covers that).
- `golden.mjs`: both add one function and one call line. Keep the order-free rule. Fine.
- `fov` semantics: I-019's `off_screen` takes `hfov` from the live `fov` and aspect. After I-022 it is exact. I-019's `hfovOf(v)` helper uses the viewport aspect, so it agrees. No change needed, but the I-019 lane should be told the `fov` field now follows aspect (the contract here says so).
- No lane in I-022 owns anything I-019's lanes own, except the shared files above.

## Lanes buildable from the contract alone: yes, after fixes 1 to 4
The rust and web lanes share no new symbol. The seam (tests 9 and 10 need both) is named. Lane hygiene test 17 is checkable by `git diff --name-only`.

## Required changes (summary)
1. Rewrite the Rust-lane camera-block sentence per item 1 (recommend writing `aspect` in `aim_and_cast`).
2. Test 13: held route released by the test, not a 3 s timer; state the `#loading` precondition.
3. Verify Playwright version for `page.clock` before building test 12.
4. Add the five one-line gaps above (hints and objective, funnel note, dayBands zero row assertion, listener only when the picker shows, `#age` not inerted).
5. Add the `aim_and_cast` signature line to the I-019 overlap table if 1(b) is used.

## Planner response

Each claim was checked against a3e50b1 before it was accepted. All changes are accepted. Spec revised to Revision 2.

1. **Rust camera block (item 1): accept (b).** Confirmed: `control` already takes `ResMut<Settings>` and has no `Camera` or `Window` in scope, while `aim_and_cast` takes `Res<Settings>` and already computes `aspect`. `aim_and_cast` now takes `ResMut<Settings>` and writes `settings.aspect`. `control` is not edited, a one-frame lag is allowed, and `is_changed` is unused, so the per-frame write is safe.
2. **Test 13 held route (item 2): accept.** A route released by the test, with a 60 s cap that fails the check, removes the boot-time race and the wall-clock wait.
3. **`#loading` precondition (item 3): accept.** The click now waits for `#loading.hidden` (set at `hud.js:547`) and for `elementFromPoint(centre)` to be `#game`.
4. **Playwright version (item 4): accept, with a correction.** `playwright` is not in `package.json`. It resolves to the global install, 1.63.0 here, so the lane checks `require('playwright/package.json').version` and reports test 12 as blocked if it is below 1.45. `dur` uses `performance.now()` (`analytics.js:268`), which `setFixedTime` does not fake, and the test checks that `dur` is valid.
5. **Test 7 `note` (tests section): accept, reworded.** `fresh()` does not clear `note`. A successful lock does (`hud.js:187`, `241`), and the spec now says that. A leftover note would make the test fail, not pass by accident.
6. **Test 9 fixed pose: accept.** The spec now fixes `tp 4 2` and `bushes[5]`.
7. **Test 10 viewport and existing checks: accept, no change needed.** The harness already resizes (golden.mjs:826).
8. **Test 14 `#age` not inert: accept.** Added `#age.inert === false` and `closest('[inert]') === null`.
9. **Gap, hints and objective: accept.** One line in Smallest version 1 (GS-24: objective and controls table stay).
10. **Gap, funnel after midnight: accept.** One line added to Problem, plus the PR note on the meaning of `players`. The backlog follow-up is for the loop's backlog step to file (the planner step edits spec.md only). It goes with I-007 N-2.
11. **Gap, dayBands zero row: accept.** A 0-row assertion was added to test 6, next to the existing guard test.
12. **Gap, picker listener only when shown: accept.** The spec says that nothing is added when `startAgePicker` returns false.
13. **I-019 overlap table, `aim_and_cast` signature line: accept.** The table now names it as a textual-only conflict risk. The test 7 note covers I-019's new locked hint.
