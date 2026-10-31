# I-046 rust lane, round 1

Branch `loop/I-046` from origin/main 76b5f13 (stream code identical to e70df08).

## First step: callers of `stream_style` / `StreamStyle`

`grep -n 'stream_style\|StreamStyle' crates/bevy-client/src` on 76b5f13: only
`cues.rs` (the struct, the function and the I-025 unit test) and `main.rs` stream
code (`Stream.style`, `stream_transform`, `spawn_stream`, the `SpellMats` build).
No other caller, so the lane proceeded.

## Red first (stub `lead 0`, `lead_min 1`, I-025 values thick 0.22 / outline 1.3; stub `vm_hand` returns `VM_HAND`)

| test | stub result | message |
|---|---|---|
| U1 `stream_taper_and_thickness_stay_in_bounds` | FAIL | `spell 0: lead 0` |
| U2 `stream_width_is_attached_thin_then_full` | FAIL | `w0 <= 0.25·thick` fails (w0 = thick) |
| U3 `stream_split_adds_up` | pass | satisfiable by the stub, as critique.md noted; kept as the structural guard |
| U4 `stream_never_looks_wider_than_a_fifth_radian` | FAIL | `spell 0: depth 2: 0.44 rad` (Water 0.57) |
| U5 `vm_hand_turns_with_the_gauntlet` | FAIL | `vfov 30` (rigid-turn distance to the gem) |

## Green

`lead 0.8`, `lead_min 0.15`, `thick 0.18` for all four styles, `outline_scale 1.2`
for Water, Frost and CO2; `feel::vm_hand` = `Rx(vm_tilt_deg)·VM_HAND`, exactly
`VM_HAND` when the tilt is 0.

- `rustc --edition 2021 --test src/cues.rs`: 12 passed (I-025's
  `stream_styles_stand_out_and_tell_the_truth` unedited and green).
- `rustc --edition 2021 --test src/feel.rs`: 13 passed (no existing test edited).
- `cargo test --locked` (native, bevy-client): 57 passed.
- `cargo fmt --check`: clean. `cargo clippy --release --target wasm32-unknown-unknown -- -D warnings`: clean.
- `build-web.sh`: ok, wasm gzip 5 088 082 B (base build of 76b5f13: 5 084 227 B, +3.9 KB).

### U4: widest the stream can look (rad, outline included; hand at (0.2, -0.15, -0.5), target on axis)

| depth | base Water (0.22 x 1.3, no taper) | Water/Frost/CO2 now | Fire now |
|---|---|---|---|
| 2 m | 0.572 | 0.168 | 0.140 |
| 5 m | 0.572 | 0.166 | 0.139 |
| 10 m | 0.572 | 0.166 | 0.138 |
| 25 m | 0.572 | 0.166 | 0.138 |
| 60 m | 0.572 | 0.166 | 0.138 |

Limit 0.20.

## Structure (`main.rs`)

- `Stream` is an unmeshed root (`Transform::IDENTITY`, `Visibility`) holding the
  `Entity` ids of two `StreamPart` children: the **lead** (new shared
  `ConicalFrustum` radius 0.5 / 0.075, height 1, resolution 10, in
  `SpellMats.lead`) and the **rod** (`kit.cyl`). Each part has the same outline
  child rule as before (inverted hull, `outline_scale` in x/z).
- `stream_transforms` places both from `cues::stream_split`; a zero-length part
  is scaled to zero (as the old rod was on its first frame).
- `projectiles` writes the part transforms through a new `StreamParts` query
  (`With<StreamPart>, Without<Stream>, Without<Projectile>`, disjoint from both
  `Moving` queries). Paused streams are skipped, so parts freeze. Despawning the
  root despawns parts and outlines (reset, epoch, `STREAM_AFTER`).
- `streams` still counts `With<Stream>`, one per live cast. Mist still uses
  `from` and `head`. No `kit` mesh is mutated.
- Entities per stream: Water/Frost/CO2 2 -> 5 (+3: root, lead, lead outline),
  Fire 1 -> 3. One new mesh.
- `aim_and_cast`: `hand = eye + rot * feel::vm_hand(settings.fov_y())`; the
  projectile ball keeps the same start.
