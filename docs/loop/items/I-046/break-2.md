# I-046 break, round 2 (wave 11, fresh breaker)

Note: there is no `break-1.md` on loop/I-046 or on disk. The round-1 blocking findings were taken from the two round-1 lane reports and the web round-2 report: (B1) G1 P0 red, (B2) G6 (a) red by 0.4 px, (B3) nothing had been run in a browser / golden red, plus the rust should-fix (hand end thinner than `stream_width(0)` on short streams).

Build at 71fd152: `build-web.sh` ok. `rustc --test cues.rs`: 13 passed. `rustc --test feel.rs`: 13 passed. Full golden (`QUERY=?low`, under the lock): **130/133**, log `brk2-golden.log`.

## Blocking

1. **G1 P0 is still red, identical to round 1.** "close streams: the targeted fire at the crosshair stays visible (Aim help on)" at 1680x720 Water: warm before 2487, after 0, ratio 0.00, `burning=false`, stream mask in the square 3484. The frozen frame is the first frame after the cast, and with Aim help on the cast extinguishes the targeted fire on that frame, so no stream shape can pass. Round 2 did not touch it (web round-2 says "needs a spec decision"). The item cannot merge with a red new check: either the spec redefines P0 (freeze one frame before impact, or use a target the spell cannot put out) or the check goes.
2. **G6 (a) is still red, identical to round 1.** 21:9, View width 70, Water: farthest stream pixel (1330,576) is 25.6 px from Hs' (1308,589), limit 25.2. (b) and (c) pass. The cause (gem silhouette crossing the 0.03*VH band edge) is not stream-width tunable, so this is a threshold/definition problem, not a code regression. Same decision needed: the band or limit must be re-derived, or (a) dropped in favour of (b)+(c).

## Verified fixed

- Rust should-fix (short streams thinner than `stream_width(0)`): `stream_parts` and U6 are correct on read, U6 passes for spells 0..=255 and lengths 0..2 m. No regression found in `stream_transforms` (zero-length parts still become `none`).
- G6 constants now come from `feel.rs` and fail loudly if unreadable; the check read them fine (tilt 5.90 deg).
- A real golden run exists now: every pre-existing check passed, including I-025 tests 7, 8, 10, I-019 stream checks, review hint and cursor-ray checks. G1 P1/P2 and G2 pass at both aspects, G3 passes, G6 (b) (c) pass.

## Should-fix

- `cast: that real click puts lit[1] out` (G5 second half) failed in my run (aim object at click 3, lit[1] = 3, never out in 2.0 game s, 7 frames); it passed in round-1's full run and failed in round-2's. Two failures of three runs means the split check is flaky on this box (the click carries view movement under pointer lock, per web round 1). Not caused by the stream change, but it makes the golden non-deterministic: fix the click setup (test 9 above the split) or mark it a known flake before merge.
- A golden run takes about 1.5 h here; the owed 3/3 full runs were not done by this breaker (one run only).
- Perf (B3) not run by this breaker.

## Verdict

Not mergeable as is: two new checks (G1 P0, G6 (a)) are red on the branch for reasons the round-2 lane left to a spec decision, so break-1 blockers B1 and B2 are not fixed. The stream geometry itself (taper, thickness, hand start, short-stream width) looks correct.
