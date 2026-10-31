# I-044 break, round 1 (wave 10)

Build: CARGO_TARGET_DIR=/home/claude/bevy-target, `build-web.sh` exit 0, wasm gzip 5 044 475 B. Unit tests (plain rustc): feel 11, cues 8, chem 19, clock 3, motion 8, all pass. Full golden under the lock (`QUERY=?low`): 2 FAIL (below), everything else PASS. Changed files are all inside the spec's lanes (feel.rs, the two main.rs sites, hud.js `hintAllowed`, golden.mjs `reviewFollowups`); no stray edits.

## Blocking

1. **The item's own test 7 fails, and the failure is a real player-visible defect, not a test bug.** `review: the gauntlet's spell gem stays on screen at 21:9` FAILs at 1680x720 View width 70 (blue pixels at G: 0, mirror 0) and is marginal at View width 75 (45 of 225; mirror 0). Reproduced on my own run: `vm=[0.21,-0.127,-0.42]`, gem 11.71 deg below centre, G=(1350,609). Geometry passes; pixels fail. Cause (crates/bevy-client/src/feel.rs:130 `vm_root_y`, wired at crates/bevy-client/src/main.rs:647 and :1640): lifting only the root's y moves the gem up but the gold wrist-ring child (local z -0.03, top at local y 0.065) stays in front of it, and the gem's centre is only unoccluded at 16.7 deg or more below centre. At 11.7 deg the ring's edge is about 7 px above G, so the screenshot (`/tmp/claude-0/i044/gem-1680-70.png`) shows just the top sliver of a dome above a gold block. The "which spell is ready" cue is therefore still mostly hidden at 21:9 / View width 70, the exact case the item exists to fix. The spec's Contract (only root y moves) makes this unfixable inside the lanes, so it needs a planner decision. Options recorded by the builder (`lanes/web/round-1.md`): (1) orbit the whole viewmodel about the eye (translation plus equal Rx tilt) so the gem looks as it does today; (2) move the gem or ring in the child list; (3) change test 7's pixel box to the visible dome. Option 1 is the only one that keeps the default look and unbreaks the cue. Until one is picked the branch cannot merge with a green golden.

## Should-fix

1. golden.mjs (test 7 block, about line 2516, `blueIn` 15x15 box): once the fix lands, do not loosen the box to pass; centre it on the visible dome or the check stops meaning "gem unoccluded".
2. `vm_root_y(58.72)` is -0.17992, not -0.18 (feel.rs:132, true default gem angle 17.608 deg). Test 8 (+-0.002) passes; the "nothing moves" seam is off by about 0.17 px at 1280x720. Harmless; acknowledge in the spec.
3. 16:9 at View width 70 also moves now (gem 16.5 deg, root y -0.1698), so "default 16:9 unchanged" holds only at View width 90. Stream start (main.rs:1204 hand) is not lifted: at 21:9 / 70 it starts about 5 deg below the gem. Already a named follow-up; keep it on the backlog.
4. `cast: a real click casts exactly once` failed in my run (casts 1 -> 2 -> 2) at fps 1.5 on a loaded box; the known I-019 wall-time flake the builder also recorded. Not caused by this diff, but spec test 10's "3/3 green" has not been shown.

## Nit

1. Test 6 controls: (ii) moved 61.9 px at 960x720 and (i) 92.6 px, above the 25 px floor, and both miss, so test 6 can fail. No change needed.

## Play-through (7-year-old and 13-year-old; 16:9 1280x720, 4:3 960x720, 21:9 1680x720)

- Hints: with Hints off, Esc after lock shows only the start line, never "Aim at a fire and click." (test 9 PASS; builder recorded it failing on the base on try 1). The `hintAllowed` change (hud.js:463) is correct.
- Aim at the edge of a wide screen: with Aim help off at 960x720 and 1680x720, a target at about (0.75W, 0.30H) is hit at the crosshair-projected pixel (P=(728,234) and (1264,236)) and both wrong-ray controls miss. The cursor ray matches the view off-centre at 4:3 and 21:9.
- Age picker: index.html ships `rg-analytics` "off", so the picker does not appear in pkg-out; forcing it on via a served copy still showed no picker (headless; possibly the DNT/GPC gate). This diff does not touch age.js, analytics.js or the picker markup, so I could not verify "picker works before pointer lock" this round. Screenshots: `/tmp/claude-0/brk10-I044-shots/`.
- 21:9 default look (View width 90): hotbar and gauntlet do not overlap and the gem is fine. The defect is only the lifted case in Blocking 1.
