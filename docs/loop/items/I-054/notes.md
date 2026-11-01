# I-054: generated block atlas (wave-15-mac, Paawan's Mac)

Base: `origin/loop/I-052` at 3315e28. Branch: `loop/I-054`. Owned: `crates/bevy-client/src/atlas_gen.rs`, `docs/loop/items/I-054/**`.

## What it is

`atlas_gen::generate_atlas(seed) -> Atlas` draws the whole block atlas in code: 256 x 256 RGBA8, 16 x 16 tiles of 16 px. `generate_atlas_sized(seed, 32)` draws the same tiles at 32 px (512 x 512). The output is deterministic from the seed.

- **Tile order:** tile `n` is the material whose `atlas` index in `content/materials.json` is `n`. On loop/I-052 that index equals the row id (0 air .. 26 gas_vent). After those rows come four extra face tiles:
  - 27 `GRASS_SIDE` (soil with a grass lip)
  - 28 `LOG_TOP` (rings)
  - 29 `SNOW_SIDE`
  - 30 `FURNACE` (the front of `valley::mat` row 27; that row is not in materials.json yet)
- **`tile_for(mat, Face::{Top, Bottom, Side})`:**
  - grass and snow sides use their side tiles
  - grass and snow bottoms use dirt
  - log ends use the rings
  - material 27 maps to the Furnace tile
  - everything else uses its own row's tile
- **`uv_rect(tile)`:** returns `[u0, v0, u1, v1]`, with v pointing down.
- **`Atlas::glow`:** an emissive texture of the same size. It is black except where a tile glows:
  - rune strokes
  - the crystal's bright core and facet edges
  - torch flame
  - hearth and Furnace fire
  - Invention Bench circuits
  - the gas vent's faint green
  - the Bench's teal grid, at a low level

## Style

One warm palette, built from the courtyard's existing colours:
- warm stone with a lavender shadow
- soft greens
- terracotta brick
- glowing teal crystal and runes
- amber torch and hearth light

Each tile is drawn the same way:
- a few flat value bands (toon shading)
- gentle tileable value noise, quantised through a 4 x 4 ordered dither
- a thin darker rim
- one small seeded detail per material: pebbles, strata, bark grooves, rings, plank seams and nails, brick courses, ore clusters, ice streaks, wave crests, a mirrored rune glyph, crystal facets around a core

It is not photographic. Nothing is copied; every pixel comes from the functions in this file.

Clear tiles have their own alpha:
- ice 0.8
- water 0.74
- steam 0.25-0.55
- glass 0.22, with a 0.85 frame
- crystal 0.88-1
- torch 0 around the stick and flame

Opaque rows are fully opaque, and only air is fully clear.

## Previews

These are written by the ignored test `cargo test --release i054_preview -- --ignored`. Set `SEED=n` for another world; the default seed is 2026.
- `atlas-preview.png`: the 31 tiles at 8x on a checkerboard, so clear tiles show.
- `blocks-preview.png`: a small isometric patch of blocks, with toon face lighting, a tree, bench, kiln, runestones, crystals, snow and water.
- `atlas-256.png` and `atlas-glow-256.png`: the raw atlas and its emissive texture.

## Tests (`cargo test i054`)

1. One tile per material row, at its `atlas` index (27 rows, plus 4 extras = 31 of 256 slots).
2. The same seed gives the same atlas, and a different seed gives a different one.
3. Only air is fully clear, and every opaque row is fully opaque.
4. Each tile's alpha-weighted average colour is within 40/255 per channel of its palette base (3 seeds).
5. Glowing tiles glow, and plain ones (dirt, stone, grass, water, snow, grass side) do not.
6. A 32 px atlas draws the same tiles: averages within 24/255.
7. The face mapping and UV rectangles.

## How Lane A calls it (voxel_tex.rs)

```rust
let a = crate::atlas_gen::generate_atlas(world_seed);
let image = |data| Image::new(
    Extent3d { width: a.size, height: a.size, depth_or_array_layers: 1 },
    TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD,
);
let mut base = image(a.rgba.clone());
base.sampler = ImageSampler::nearest(); // no mipmaps (WebGL2-safe)
let mut glow = image(a.glow.clone());
glow.sampler = ImageSampler::nearest();
// opaque pass: base_color_texture = base, emissive_texture = glow,
// emissive = LinearRgba::rgb(2.0, 2.0, 2.0) (bloom picks up the cores),
// alpha_mode = Mask(0.5); clear pass: same textures, AlphaMode::Blend.
```

In the mesher, the tile for each quad is `atlas_gen::tile_for(mat, face)`, and its UVs come from `uv_rect`. With greedy meshing, repeat the tile per block: pass the tile rectangle as vertex data and wrap with `fract` in the shader. Otherwise, emit one quad per block face for the textured faces. The torch tile is meant for a crossed-quad sprite, not a cube.

## Deviations

- **`main.rs`:** one line, `#[allow(dead_code)] mod atlas_gen;`, so the module compiles and its tests run. Lane A owns main.rs; please keep the line when you merge, or move it into voxel_tex.
- **The Furnace (valley row 27)** has a tile (30), but it has no row in materials.json yet. Whoever adds the row should set `"atlas": 30`, or I can remap.
