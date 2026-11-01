# I-053: synthesised sound library for the block world

**based_on:** origin/main 5662cd7. **Lane:** wave-15-mac. **Owns only:** `crates/bevy-client/web/sfx.js`, `test/sfx.test.ts`, `docs/loop/items/I-053/**` (no hud.js, music.js, index.html or Rust).

**Why.** The Game Week assignment requires every asset to be generated for the game; Paawan approved free synthesis in code. This gives the block world one sound library with a consistent warm, soft, kid-friendly palette.

## What it is

- `sfx.js`: a Web Audio API module, no samples and no files. Every sound is built from oscillators, a code-made noise buffer (and a code-made crackle loop), biquad filters and gain envelopes.
- API: `play(name, opts) -> handle | null` (`opts`: `volume` 0..1, `pitch` multiplier, `material` for `dig`/`step`, `duration` for held sounds), `setVolume(v)` (clamped 0..1; NaN keeps it), `names`, `unlock()`. `createSfx({ makeContext })` builds an independent instance (tests, dev pages).
- The AudioContext is created on the first user gesture (pointerdown, keydown or touchstart); `play` before that is a silent no-op, as is an unknown name.
- Master chain: master volume → warm low-pass (6 kHz) → limiter (threshold −12 dB, ratio 20) → 0.5 trim (−6 dBFS) → speakers.
- Sounds (29): water_charge (rising, held, `stop()` on release), water_splash, hiss, steam, flame_whoosh, fire_crackle (loop, `stop()`), frost_spray, ice_crack, ice_drip, co2_puff, h2_pop, methane_whoomp, glass_ting, dig_{dirt,stone,sand,wood,ice} (and `dig` + `material`), place_block, step_{grass,stone,sand,snow,wood} (and `step` + `material`), pickup, bench_success, bench_wrong, gate_open, fanfare.

## Checks

- `test/sfx.test.ts` (node:test, the repo's runner; a mocked AudioContext): every name exists and plays from generated sources only; unknown names and plays before the first gesture are no-ops; volume is clamped; the limiter, warm low-pass and −6 dBFS trim are in place; no envelope exceeds unity before the master.
- `sfx-preview.html`: a button per sound and "Measure all" (an AnalyserNode on the output records each sound's peak).

## Not done here

Wiring the sounds into the game (hud.js / the block world's events) belongs to the lanes that own those files.
