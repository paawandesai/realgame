# I-007 break 1 (main) - BREAKER

Branch loop/I-007 @ 870ce29 vs origin/main. Built the wasm release (build-web.sh), ran unit tests, `npm test`, and `QUERY=?low node tests/e2e/golden.mjs`.

## Results
- Lane: PASS. Changed files outside docs/loop/items: content/science-sources.md, crates/bevy-client/src/{chem,feel,main,player}.rs, web/hud.js, web/style.css, tests/e2e/golden.mjs. All inside the lane. golden.mjs has zero deleted lines (no existing check edited or loosened).
- feel.rs 4/4, chem.rs 15/15, `npm test` 34/34.
- Golden: 67/67 passed on the second run (existing 36 checks plus the "feel and standards" section, no console errors). The first run, under load average about 21 from other agents' browsers, failed the old `WASD moves` check (1.40 m against 1.5 m within the 20 s wait) and then crashed the page target. The check passed on rerun at 2.15 m, and nothing in the diff touches movement (player.rs changes only the LOOK_PER_PX source), so I call it load flake, not a regression.

## GS P0 claims
- GS-1 PASS: boot fov 58.72 (90 deg at 16:9). Slider 70-110 live. `tap` ray uses the live FOV (ray_dir).
- GS-3 PASS: cam equals eye at all samples; vm spread is exactly 0 with sway off. Recoil kick remains and is cast feedback.
- GS-4 PASS: yaw and pitch unchanged over idle. Assist only bends the cast.
- GS-8 PASS: 100 px gives 5.44 deg at sens 1 (21 cm/360 at 800 CPI); the slider is 0.25-3.00 and sens 2 gives double.
- GS-10 PASS for the locked-aim path: runtime toggle without reload, distance falloff. See SF-2 for the point-and-click path.
- GS-12 PASS: turn, cast and set sens all land within 1-2 frames. Input feel is otherwise unchanged apart from the intended slower default.
- GS-15, GS-17, GS-19 PASS: hit/fizzle/out/miss differ in shape and in sound. The out marker is 96 px with a 3-note cheer, peak gain 0.15. Muted state still shows markers.
- GS-18 PASS: all HUD/pause text at least 18 px at 1366x768 and 1920x1080, 200% doubles, contrast at least 4.5:1 on the panel.
- GS-21 PASS: the idle hint is short and the Water slot glows, so a pre-reader gets a cue.
- GS-24 PASS (idle hint at 45.9 s, not before 40 s), but see SF-1 for the Hints toggle.
- GS-6: marker and glow are static or 1 Hz. Frame-to-frame change 2.3% and 0 flashes in the burst.
- Science register: PASS. The one new sentence has a non-claim row, it states no science, and the reading level is low.

## Blocking
None.

## Should-fix
- SF-1 (GS-24 "players can turn hints down", GS-26). crates/bevy-client/web/hud.js:532 and the `h` computation around 573-585. The `#hints` checkbox ("Hints") only gates the 45 s idle hint and the Water glow. The ordinary onboarding hints are untouched: "Aim at a glowing target...", "Click to look around...", "Point and click: click a glowing target...". The golden check output itself shows the point-and-click hint on screen right after Hints is switched off. A student who unticks "Hints" still gets hints, so the label misleads. Either gate `h` on `settings.hints` (keep "Click to look around" while the pointer is unlocked and nothing has been cast) or rename the control to "Idle hint".
- SF-2 (GS-10 toggle). crates/bevy-client/src/main.rs:879. `tap_assist = settings.assist.max(3 deg) * 1.5` means that with "Aim help" off, point-and-click taps still get a 4.5 deg cone (and hover uses it as well). The checkbox is therefore not a full off switch for the click path. Test 10 covers only the locked path. Use 0 (or only the 1.5x factor on the setting) when assist is off, or document that the click path always helps.
- SF-3 (GS-1/GS-2 honesty). crates/bevy-client/src/feel.rs:27 and Settings::fov_y in main.rs. "View width 90 deg" is true only at 16:9. Bevy's fov is vertical and fixed at 9/16, so on 4:3 (older school projectors) or 21:9 the real horizontal angle differs (about 74 deg at 4:3), and "View width" is mislabelled. Derive the vertical angle from the live canvas aspect, or state that the value is for 16:9.

## Nit
- N-1 main.rs:292 and hud.js:50 (and feel::clamp_sens). `?sens=` is now clamped to 0.25-3 (was 0.05-10), so existing links like `?sens=0.1` or `?sens=5` change silently. Acceptable per spec, but not called out in the README or docs.
- N-2 hud.js:470. Turning Hints back on does not restart the idle clock, so the hint can appear at once if the player was idle while it was off. Reset `idleSince` when the box is re-ticked.
- N-3 hud.js:195-200 (first state). The game boots with the provisional URL values (fov 90, sway on) and the stored settings arrive on the first HUD frame, so a saved sway=off player gets one or a few frames of bob and a saved 110 deg gets a few frames at 90. Cosmetic.
- N-4 style.css:65. `.slot { min-width: 190px }` with a max-width 760 override. At 200% the hotbar wraps to several rows and covers more of the view. It stays inside the viewport as the test requires, but it is large for small laptops. A P2 follow-up could collapse the formula at 200%.
