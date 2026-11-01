//! The block world's generated texture (I-052, Lane A; spec section 5.9):
//! made at startup, no downloads, nearest-neighbour, no mipmaps
//! (WebGL2-safe).
//!
//! The style is "toon block": the colour of each block comes from the
//! mesher's vertex colours (`realgame_voxel::mesh::TINT`, the courtyard's
//! palette, times the AO), and this one greyscale tile adds the thin darker
//! border and a little seeded speckle. The tile repeats once per block,
//! because the mesher's UVs are in blocks, so greedy-merged quads still show
//! every block's edge.
//!
//! Round 1 uses one shared tile for every material. Per-material detail
//! (wood rings, ice streaks, crystal facets, the carved runestone) needs the
//! 16 x 16 atlas of section 5.9 and a small material extension that wraps
//! the UVs inside a tile (a merged quad cannot repeat one tile of an atlas
//! with a plain `StandardMaterial`); that is the Wednesday polish item.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Pixels along each edge of the tile.
pub const TILE: u32 = 16;

/// A small integer hash in 0..1 (no `sin`).
fn hash(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x27d4_eb2d) ^ y.wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65535.0
}

/// The tile's brightness at pixel (x, y): a 1 px darker border, a faint
/// bevel just inside it, and speckle.
pub fn tile_value(x: u32, y: u32) -> f32 {
    let edge = x == 0 || y == 0 || x == TILE - 1 || y == TILE - 1;
    if edge {
        return 0.62;
    }
    let bevel = if x == 1 || y == 1 {
        0.05
    } else if x == TILE - 2 || y == TILE - 2 {
        -0.05
    } else {
        0.0
    };
    0.9 + bevel + 0.08 * (hash(x, y) - 0.5) + 0.05 * (hash(x / 4, y / 4 + 17) - 0.5)
}

/// The tile as a repeating, nearest-sampled sRGB image.
pub fn tile(images: &mut Assets<Image>) -> Handle<Image> {
    let mut data = Vec::with_capacity((TILE * TILE * 4) as usize);
    for y in 0..TILE {
        for x in 0..TILE {
            let v = (tile_value(x, y).clamp(0.0, 1.0) * 255.0) as u8;
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: TILE,
            height: TILE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..ImageSamplerDescriptor::default()
    });
    images.add(img)
}

/// The two chunk materials: `opaque` (lit, rough) and `clear` (water, ice,
/// glass, crystal: alpha blend, both sides).
pub struct ChunkMats {
    pub opaque: Handle<StandardMaterial>,
    pub clear: Handle<StandardMaterial>,
}

pub fn materials(images: &mut Assets<Image>, mats: &mut Assets<StandardMaterial>) -> ChunkMats {
    let tex = tile(images);
    let opaque = mats.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(tex.clone()),
        perceptual_roughness: 0.9,
        reflectance: 0.2,
        ..default()
    });
    let clear = mats.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(tex),
        perceptual_roughness: 0.15,
        reflectance: 0.5,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    ChunkMats { opaque, clear }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_border_is_darker_than_the_middle() {
        let mid = tile_value(TILE / 2, TILE / 2);
        assert!(tile_value(0, 5) < mid - 0.15);
        assert!(tile_value(5, TILE - 1) < mid - 0.15);
        for y in 0..TILE {
            for x in 0..TILE {
                assert!((0.0..=1.0).contains(&tile_value(x, y)));
            }
        }
    }
}
