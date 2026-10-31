# I-025 contract stub, round 1

Commit `e37a8f5` on `loop/I-025` (base origin/main 05d43a5).

- `cues.rs`: appended `StreamStyle`, `stream_style` (neutral: thick 0.13, core = spell colour, outline = core, scale 1.0, mist 0), real `rel_luminance`/`contrast_ratio`, `fire_in_view` = false, `HOLD_DELAY` = 0.5, `held_secs` = 0, `hold_repeats` = false. No Bevy types; `rustc --test` builds it alone. Not wired into `main.rs`.
- `main.rs`: `Game.fire_in_view` (false) and the state key `fireInView` appended after `fxWater` (format string and args only).
- `golden.mjs`: the two sanctioned edits. Key-order check takes `I025_KEYS = ['fireInView']`; I-019 test 16's `found` is `!!h.ff && !h.ff.vis && h.s.fireInView === true && (h.s.fireOff === true || |fireBearing| <= hfov/2)`, and its detail now logs `fireInView` and why the arrow hid (`nearest fire in view` / `another fire in view`). Names and other conditions unchanged.
- The stub build compiles with 8 dead-code warnings (the unused stub API); the lanes remove them.
