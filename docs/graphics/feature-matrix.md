# Graphics feature matrix: Bevy 0.20.1 on WebGL2 and WebGPU

Graphics phase G0, measured 2026-11-04 with `crates/gfx-lab` (see its README). Each row loads the test scene with one feature on, in headless Chromium.

**How to read it.** WebGL2 ran on SwiftShader, a software GPU, so every frame costs about 250 ms. Read the frame times as cost relative to the base row, not as real fps. Headless WebGPU on SwiftShader lost its device at startup for every feature, so the WebGPU column comes from Bevy's own capability checks (compute shaders and storage textures), not from a run. Testers check it on real hardware in G5.

## Result

| Feature | WebGL2 (measured) | p50 vs base | WebGPU (expected) | Tier in the plan |
| --- | --- | --- | --- | --- |
| Base scene, ray-traced atoms | Runs; atoms render round with true depth | 1.00 (252 ms) | Runs | All |
| Colour grading | Runs | 0.99 | Runs | All |
| Clearcoat on atoms | Runs | 1.01 | Runs | Mid, High |
| Bloom | Runs | 1.42 | Runs | Mid, High |
| FXAA | Runs | 1.24 | Runs | All |
| SMAA | Runs | 1.25 | Runs | Optional |
| 4x MSAA | Runs (the game already ships it), too slow to sample on SwiftShader | n/a | Runs | High |
| Sun and point-light shadows | Runs | 1.15 | Runs | Mid, High |
| Screen-space transmission (glass) | Runs; refracts | 1.05 | Runs | Mid, High |
| Environment light, CPU-filtered (`envmap_cpu`) | **Runs and lights the scene** | 1.20 | Runs | All |
| Environment light, GPU-filtered (`GeneratedEnvironmentMapLight`) | **No effect**: needs compute shaders | 0.98 | Runs | High only |
| Distance fog | Runs | 1.35 | Runs | All |
| Depth of field | Runs, no visible blur | 1.00 | Runs | Not planned |
| Depth prepass | Runs, but WebGL logs incomplete-framebuffer errors | 1.04 | Runs | Mid, High (fog sheets) |
| SSAO | **Not loaded**: "max_storage_textures_per_shader_stage is less than 5" | 1.03 | Runs | High only |
| Contact shadows | **Fails**: shader can't `textureLoad` a depth texture in GLSL | n/a | Runs | High only |
| Screen-space reflections | **Fails**: never reached a frame | n/a | Runs | Not planned |
| Volumetric fog | **Fails**: same depth-texture error; the app quits | n/a | Runs (docs: "requires WebGPU") | High only |
| TAA | **Fails**: "a image was used with multiple samplers" in GLSL | n/a | Runs | High only |
| Order-independent transparency | Not loaded (no writable storage in fragment) | n/a | Runs | Not planned |
| Atmosphere | Not loaded (no compute shaders) | n/a | Runs | Not planned |

## What changes in the plan

1. **Environment light on Low and Mid uses the CPU-filtered route.** `GeneratedEnvironmentMapLight` silently does nothing on WebGL2. `crates/gfx-lab/src/envmap.rs` builds the diffuse and specular cubes in code at startup, under 0.3 MB of GPU memory with no image files, and it works on both APIs.
2. **The Low and Mid tiers lose SSAO, contact shadows, volumetric fog and TAA.** The plan already put these on High only, and the matrix confirms they can't run on WebGL2. AO on Low and Mid stays baked into vertex colours, and soft contact under atoms is a blob shadow.
3. **Glass transmission runs on WebGL2**, at about 5% extra cost on the software GPU. Mid keeps real refraction, as planned.
4. **The atom impostor works on WebGL2**: true depth, correct molecular geometry, lit by Bevy's PBR code with shadows received. G1 still needs a prepass and shadow shader for it, so atoms can cast shadows and work alongside the depth prepass.
5. **Any feature that needs a depth prepass is a risk on WebGL2.** The prepass logs framebuffer errors on SwiftShader. Mid's fog sheets should fade on analytic height, not on scene depth, until a real-GPU test shows the prepass is clean.

## Raw data

The runner writes `matrix.json` and one screenshot per run. The 2026-11-04 run used 60 warm-up and 180 sampled frames at 960 × 540. Its p50 frame times on SwiftShader, in ms: base 252, grading 251, clearcoat 256, dof 253, envmap_gen 247, ssao 259, depth_prepass 262, transmission 264, shadows 291, envmap_cpu 302, fxaa 313, smaa 315, distance_fog 340, bloom 358.
