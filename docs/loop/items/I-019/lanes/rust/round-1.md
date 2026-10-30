# I-019 rust lane, round 1

Branch `loop/I-019`, based on origin/main 265a93e. One builder ran the contract stub, the rust lane and the web lane in sequence.

## What changed (crates/bevy-client/src)

- **`cues.rs`** (new, Bevy-free): `fires_left`, `bearing_deg`, `off_screen`, `flight_secs` (speeds 32/30/120/20 m/s in `Spell::ALL` order, clamped to [0.03, 0.25] s; an unknown spell or a distance that is not positive and finite gives 0.03), `Look` (moved from `world.rs`, which re-exports it) and `look_emissive` (Hot 0.0, Burning 1.2, Frosty 0.25, others 0).
- **`main.rs`**
  - `fire` is in the command filter and the cast branch (`Some(&"cast") | Some(&"fire")`). `clicks` counts each `fire` in `incoming`, before the paused-queue clear. No second queue.
  - `Projectile.dur = cues::flight_secs(spell, dist)`.
  - `spawn_stream(material)` is one helper for Water, Fire and CO2 (`mats.glow[spell]`). The stream is a cone mesh from the hand (0.13 m thick) tapering to the moving head, which follows the projectile's `spell_path`. It lives until 0.2 s of game time after impact, and is drawn in at least 3 frames, so it shows at 1 to 4 fps too. Its clock counts only while unpaused, and a reset (epoch change) despawns it. Frost keeps its beam, which counts as a stream.
  - Water splash: 24 particles at size 0.38, plus 8 Steam particles when the target was burning. Other spells keep 14 at 0.22.
  - `Game.spell = 1` is set in `setup`. `reset()` still keeps the spell. New startup caption.
  - `nearFire`, `fireBearing`, `fireElev` and `fireOff` are computed every frame in `aim_and_cast`, from the live `settings.fov_y()` and the camera aspect, using `chem.bounds(i).center()`.
  - State keys appended after `dt` in contract order: `clicks, firesLeft, nearFire, fireBearing, fireElev, fireOff, streams, fxWater`.
- **`fx.rs`**: `Fx.water_live` counts live Water particles after each update (`fxWater`).
- **`world.rs`**: `hot` is now a scorched brown (0.4, 0.25, 0.13) with emissive `look_emissive(Hot) = 0`. `burning` and `frosty` take their strength from `look_emissive`, so Burning stays at 1.2.
- dt for streams comes from `clock::frame_dt` in `projectiles`. There is no `min(0.05` anywhere (the golden static check passes).

## Measured

- Unit tests, written first against the neutral stub, all failed (5/5 FAILED). After the change: `cues` 5/5 ok, `chem` 19 ok, `feel` 4 ok, `clock` 3 ok, `motion` 8 ok.
- `cargo fmt --check`: clean. `cargo clippy --target wasm32-unknown-unknown --locked -- -D warnings`: clean. Two type aliases (`Moving`, `StreamsAndBeams`) keep clippy's type-complexity lint quiet.
- `build-web.sh`: OK. The wasm is 5 043 310 bytes gzipped. Brotli-11 of `pkg-out` is 3 566 853 bytes in total (wasm 3 538 580). No base build was made, so the size delta for test 21 is still open for the tester.
- Golden (`QUERY=?low`, under the chromium lock, about 1 fps on this machine): **98/98 passed** (75 existing + 23 new). Relevant lines:
  - `clock: state JSON keeps the base keys … keys ok=true (n=48, last=fireOff,streams,fxWater) … parsed=202 errors=0 key changes=0`
  - `clock: no 0.05 s dt clamp left; every per-frame system uses clock::frame_dt`: PASS
  - Test 11: Water and CO2 from pose (−4, 14) to lit[0], 19.47 m: `g_i − g_c = 0.000 s`, `d = 0.25` (cast and impact in the same frame at dt 0.25).
  - Test 12: stream rows `[[131,1,1,1,24],[132,1,1,1,24],[133,1,1,1,16],[134,1,0,1,2],…]` (3 frames with a stream), `fxWater` 24 at impact. Pixel check: before 2909/2911, noise 2, after 30 894, need 5 678.

## Deviations

- **Test 11 pose.** `tp 0 -20` is inside the greenhouse (x −8..8, z −22..−12), so the test tries `(-4,14)→lit[0]`, `(4,14)→lit[1]`, `(-6,16)→lit[0]` and `(6,16)→lit[1]`, and uses the first where `aimObj` is the fire at 18 m or more. It logs the pose; on this seed that is `(-4,14)`.
- **Minimum stream frames.** The stream stays at least 3 frames as well as 0.2 s past impact. Without that, at the 0.25 s dt ceiling it would show in only 2 frames.
