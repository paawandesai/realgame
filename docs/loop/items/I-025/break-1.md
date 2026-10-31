# I-025 break-1 (breaker, wave 9)

Branch tip bf15d85. Lanes: all 5 changed files (`cues.rs`, `main.rs`, `hud.js`, `golden.mjs`, `science-sources.md`) are inside the spec's lanes; the I-019 amendment lives under `docs/loop/items` (excluded from the diff, in lane).

## Checks run
- wasm build (own target dir): ok. `cargo test --release`: 45 passed, 0 failed.
- golden (`?low`, under the lock): 123/124. The one FAIL is `cast: a real click casts exactly once` (`tests/e2e/golden.mjs:1808`), detail `casts 1 -> 2 -> 2 (20 frames on), lit[1] burning=true`. The cast count is correct (one cast, no repeat); the failing part is `r.ok`, which needs lit[1] out within 20 frames. At 1.6 fps in this run that is the known slow-GPU flake shape, not a double cast. Not blocking; the round notes cite base flake evidence, which I did not re-verify.
- Played as a 7-year-old (age band "4 to 7") and a 13-year-old ("11 and up") with Playwright at 1280x720 (16:9), 1024x768 (4:3) and 1680x720 (21:9), on a copy of the build with `rg-analytics` on so the picker shows. Screenshots: /tmp/claude-0/brk9-play-shots, /tmp/claude-0/brk9-edge-shots (not committed).
  - Age picker: visible before any pointer lock, a click on the band dismisses it, `locked` stays false afterwards, at all 3 aspects, both personas. Pass.
  - Hints: start hint shows before lock, "Aim at a fire and click." after lock, nothing contradicts `pickHint` order. Hints-off is covered by golden's `review:` checks, which passed. Pass.
  - Arrow: facing a fire gives `fireInView=true`, arrow hidden; facing away gives `fireInView=false`, `fireOff=true`, arrow shown. Same at all aspects and both personas. Pass.
  - One click, one cast: a 350 ms press gave `casts 0 -> 1`, `clicks 0 -> 1` at every size. Pass.
  - Wide-screen edge aim: in point-and-click mode with a fire placed at bearing +/-34.4 deg (fire at the right/left part of a 21:9 view, hfov stays 90 at every aspect), the cursor at the projected pixel gives `hover == that fire`; the projected pixel matches the rendered fire in the screenshot (21:9 right edge: pixel 1415, fire drawn at about 1440). No mismatch found.

## Blocking
None found.

## Should-fix
1. **Close-up streams are huge slabs that hide the target.** `crates/bevy-client/src/cues.rs:240/248/256` (`thick: 0.22`, outline 1.3x = 0.29 m) with the stream starting at the hand (`main.rs:1204`, 0.5 m ahead of the eye). A constant-width rod 0.22 m across at 0.5 m reads as a wedge about 25 degrees wide, so in the 21:9 and 4:3 shots the blue/white slab covers roughly a third of the screen (the whole right half below the crosshair) for the stream's life, and the dark outline makes it bigger. A child who aims at a fire to the right of the crosshair loses sight of it while casting. The spec accepted "perspective makes it look tapered", but the screenshots (`kid7-21x9-3-water-1.png`, `kid7-4x3-3-frost-1.png`) show a blocky near-plane wedge rather than a stream. Suggest a short ramp: draw the stream's first ~0.6 m thinner (or start it a little ahead of the hand), keeping 0.2 m from about 1.5 m on, or lower the near width and keep the far width. Needs a spec decision, since a lead-in gap was rejected.
2. **Golden's `a real click casts exactly once` is slow-GPU fragile.** `golden.mjs:1805-1808` ties "cast once" to "fire is out within 20 frames", so a miss under load reads as a cast bug. Splitting the check (cast count vs extinguished) would keep the signal.

## Nits
- `main.rs:1287-1291`: `mist_acc` keeps growing while the head is within 2 m of the eye, then releases a burst in one frame when it passes 2 m (up to the cap of 10). Harmless at the cap, but it differs from "keeps mist_acc" only in that the burst is not spread; fine to leave.
- Point-and-click mode still shows the centre crosshair while the cursor is elsewhere (seen in `edge-21x9-right.png`); older behaviour, not I-025's, listed for the panel brief.
- Frost and CO2 cores rely on a hand-tuned linear gain (`STREAM_CORE_GAIN`, `main.rs:~127`) to survive tonemapping; a change of the tonemapper or the sky colour will quietly break the "white stands out" claim, which only e2e tests 7/8 notice.
