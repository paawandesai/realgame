# Formula art bible, v2 (draft)

Status: draft from graphics phase G0. It replaces `docs/art-bible.md` after Thursday's submission ships; until then the submission keeps v1. Plan: [Formula: graphics upgrade plan](https://claude.ai/artifact/CVmLpTpyZDSEUQmF23bGwz).

Every visual is checked against this page. An asset is done only when a separate critic has compared it with this page and with an in-game capture at play scale (the lecture's creator, critic, in-game check loop). Captures come from cloud runs only.

## Direction in one line

The world is a mood, painted in depth layers where only science glows. Look closer, at the Bench or where a spell lands, and the picture switches to a precise, real-looking view of the molecules.

## Two layers

| | World layer | Close-up layer |
| --- | --- | --- |
| Where | The open world, the valley, dungeons | The Bench chamber, spell-hit lenses, the codex |
| Look | BADLAND-like: limited palette, silhouettes, layered fog, glow | Real: glossy atoms, true geometry, clear glass |
| Detail | Low; big shapes, few colours | High; every bond length and angle is real |
| Truth | Stylised on purpose | Never stylised in a way that misleads |

## World layer

- **Three depth bands.** Near: dark silhouettes with a thin rim light. Middle: the playable space, in the region's palette at normal contrast. Far: flat painted ridges and sky that fade into fog.
- **Palette.** Five colours per region, plus science teal `#40FFD9`. No fully saturated primaries outside spells.
- **Fog.** Height fog pools in valleys and over water; a second, region-coloured distance fog; drifting fog sheets between the bands (Mid and High tiers).
- **Glow is earned.** Only science glows: runes, crystals, spell light, the Bench. Emissive stays under the bloom ceiling, so glow reads as light, not white blobs.
- **Readability first.** A 6-year-old must tell the path, the goal and anything usable apart at a glance. Interactables keep a lit edge, even inside a silhouette.

### Region palettes (first pass)

| Region | Ground / near | Middle | Far / fog | Accent light |
| --- | --- | --- | --- | --- |
| Village | `#2B1E1A` umber | `#C9874F` clay, `#D8B26E` ochre | `#5E6F8E` dusk blue | Warm sun `#FFD8A0` |
| Frozen Fields | `#1E2A36` slate | `#CFE8F2` ice, `#EEF2F5` snow | `#8FA8C4` blue haze | Low cold sun `#E8F4FF` |
| Marsh | `#1F2416` peat | `#6B6F3A` olive, `#4A3B57` bruise | `#A6AE6A` yellow-green fog | Sickly glow `#C8E07A` |
| Crystal Cave | `#07090C` black | `#0E3B44` deep teal | `#071A1F` | Crystal `#40FFD9`, torch `#FF9A4A` |
| Kiln | `#2A1610` soot | `#D99A6C` clay | `#6A3A2A` ember haze | Hearth `#FF8A2B` |

## Close-up layer

- **Atoms.** Spheres in CPK colours: H `#EDEDED`, C `#38383D`, N `#3050F8`, O `#E61F1A` (and the standard CPK table for the rest). Ball-and-stick radius is a quarter of the van der Waals radius; space-filling uses the full radius.
- **Scale.** 1 Å is 8 cm at the bench. Bond lengths and angles come from the chemistry engine and are never adjusted for looks.
- **Bonds.** Cylinders split in half, each half in its atom's colour. Double and triple bonds are two or three thinner parallel cylinders.
- **Material.** Glossy plastic: roughness about 0.3, a clearcoat highlight on Mid and High, a faint rim so atoms read against the dark room.
- **Events.** Stretching bonds thin and warm toward orange; a broken bond throws two short sparks; a forming bond grows from both atoms with a teal glow. Energy released is a warm pulse, energy taken in a cool pulse.
- **Glass.** The chamber is near-clear glass with a faint teal tint, index 1.5, refracting on Mid and High and alpha-blended with a bright edge on Low.

## The lab

The lab uses the world's painted-depth idea, so the Bench never feels like a different game: a warm, darker room, a window key light from the side, brass as the only metal, and the chamber as the brightest thing in view.

## Light and exposure

- Fixed exposure per area (lab, valley, cave), never auto-exposure, so colours stay stable for teaching.
- The shader tonemapper (no LUT file), then colour grading per region, eased as you walk between regions.

## Never

- Photo textures, pixel tiles or hard voxel edges.
- A molecule shape, colour or size changed so it looks better but teaches something false.
- Glow on anything that is not science.
