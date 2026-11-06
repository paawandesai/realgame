# World look: realistic (`?look=new`)

Status: first pass, behind `?look=new`, for Paawan to play before anything merges. The default look is unchanged.

## Direction

Paawan, 2026-11-04 (23:48 UTC): "more than just badland style i want a game that is real to this real world". This replaces the stylised BADLAND sketches ("Formula world look", https://claude.ai/artifact/QG3fcrHJcEmmAobRLpNDxF) for the open world.

The reference targets:

- Noita and The Powder Toy for how matter behaves.
- Teardown for 3D fire, smoke and destruction.
- Half-Life: Alyx for close-up detail at the Bench.
- From Dust and Oxygen Not Included for fluids and heat.

Texture source: free CC0 photo scans (Paawan's choice on 2026-11-04).

## What the first pass does

| Part | How |
| --- | --- |
| Daylight | A second mood table in `mood.rs` (`area_mood_new`). Each region gets a near-white sun of about 5500 K and a blue sky fill. The haze is the colour of real air. There is no rim light and no tinted grade. The Marsh is overcast with grey-white mist. The relit Kiln is a real evening. |
| Sky | `sky.wesl` `physical_sky`: single scattering, Rayleigh plus Mie, over a plane-parallel air column, for the sun's direction. It settles into the fog colour at the horizon. There is no aurora. |
| Ground | Four Poly Haven CC0 scans: grass `sparse_grass`, soil `forest_ground_04`, rock `aerial_rocks_02`, snow `snow_field_aerial`. Each has albedo and an OpenGL normal map. They are stacked by `tools/textures/fetch.py` into `web/tex/` (3 MB of JPEG). The browser decodes them and builds the mips (`web/terrain-tex.js`), and `look.rs` makes the texture arrays. `voxel_smooth.wesl` picks layers from the vertex: snow is bright and grassy, grass is grassy and flat, soil is warm or dark ground, and rock is the rest plus steep slopes. Rock uses a triplanar projection; the others project from above. Each scan's mean colour is pulled toward a real albedo from the vertex colour, so marsh grass stays olive. Buildings, trees and glowing things are excluded (`voxel::smooth::MODEL_MARK`). |
| Water | No teal shimmer; only real reflection. |

## Speed

The speed fix applies to both looks. Measured with `tests/e2e/voxel-perf.mjs` (headless, `?low`, 4x CPU throttle), run serially on the same machine:

| | Before | After |
| --- | --- | --- |
| Median CPU frame | 46 ms | 36 ms |
| Worst stall at a stop | 300 ms | 73 ms |
| First playable | 15.8 s | 11.3 s |

The changes:

- Chunks wholly past the fog are hidden (`cull_far`).
- The whole valley is meshed while the game is paused (title, intro, menus), at 120 ms a frame.
- A chunk that overruns its budget is paid back over the next frames, at most 3 frames, instead of meshing at least one chunk every frame.

The cloud measures on SwiftShader, so these numbers are relative. A profile showed that most of the remaining frame is Bevy's render work (mesh bind groups, command submission), not game logic or meshing.

## Next

- **Fire, smoke and steam (Teardown).** Volumetric-looking smoke that darkens and spreads, flames coloured by temperature and emitter (from the Bench's show records: flame K, emitter, gas density, opacity and colour), and flickering light with scorch marks.
- **Trees and houses.** These are still the stylised low-poly models. Real-looking trees and buildings need models or scans.
- **High tier (WebGPU).** Ambient occlusion and volumetric light.
