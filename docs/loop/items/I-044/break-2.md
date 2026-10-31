# I-044 break, round 2 (wave 10)

Verdict: no blocking findings. Round-1 Blocking 1 is fixed.

Build: `build-web.sh` exit 0 (wasm-bindgen from /home/claude/bevy-target/tools/bin), wasm gzip 5 044 585 B (matches the builder). Unit tests (plain rustc): feel 12, cues 8, chem 19, clock 3, motion 8, all pass. Full golden under the lock (`QUERY=?low`): **127/127**, one run (the box ran at about 1 fps during it, and no frame-time check flaked). Diff since break-1 touches only feel.rs, the two main.rs sites, golden test 7, and lane docs; hud.js unchanged.

## Round-1 findings
- Blocking 1 (gem hidden behind the wrist ring): fixed. The rest pose now orbits about the eye (translation and Rx tilt together, `feel::vm_rest`), so the gem keeps its clearance from the ring. Test 7 reads vm equal to the expected orbit pose and passes: 1680x720 View 70 gem 11.72 deg, blue 128 (round 1: 0), mirror 0; View 75 blue 136; 960x720 View 70 blue 105. The pixel box is the same 15x15 centred on the gem, not loosened. Test 8 reads vm exactly (0.21, -0.18, -0.42).
- Should-fix 2 (`vm_root_y(58.72)` -0.17992): gone, the default pose is exact with tilt 0.
- Should-fix 1: honoured (box unchanged, geometry half stricter).
- Should-fix 3 (16:9 at View 70 moves; stream start not lifted): still true, recorded in the lane docs; keep "stream starts at the lifted hand" on the backlog.
- Should-fix 4 (I-019 flake): passed in my run.
- Test 6 controls and test 9: pass again (controls 92.6, 61.9, 105.2 and 132.5 px away, all miss; no aim-hint flash).

## Should-fix
1. The spec's Contract still says only root y moves and test 7's formula uses `vm + Ry(0.1)·...`. Update the spec to the orbit (`g = vm + Rx(tilt)·Ry(0.1)·(0, 0.0275, -0.0605)`) and note that unit tests 1 and 4 are restated for `vm_rest`, since `vm_root_y` no longer exists.
2. Test 7 recomputes tilt from the same clamp formula as the Rust code, so its geometry half is partly a restatement. The pixel half is the independent check and it passes; fine to leave.
3. The stream start (main.rs hand (0.2, -0.15, -0.5)) is not tilted with the viewmodel, so at 21:9 / View 70 it starts about 5 deg below the gem. Unchanged named follow-up.

## Not verified
Age picker: the builder's picker-check (site-like copy, lock) was not re-run by me; the diff does not touch age.js, analytics.js or the picker markup.
