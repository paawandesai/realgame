//! The block world's generated texture (I-052; the art is I-054's
//! `atlas_gen`): made at startup from the world seed, no downloads,
//! nearest-neighbour, no mipmaps (WebGL2-safe).
//!
//! Each block face shows its material's tile (`atlas_gen::tile_for`: grass
//! and snow have side tiles, logs have rings on their ends). The mesher's
//! UVs are in blocks, so one greedy-merged quad spans several blocks; the
//! chunk material (`VoxMat`, a `StandardMaterial` with a small fragment
//! shader, `voxel_atlas.wesl`) wraps each block's UV into the tile whose
//! origin rides in UV_1. Vertex colours carry only the AO light. The glow
//! atlas is the emissive texture, so crystal cores, runes, torches and the
//! hearth light up under bloom.

use crate::atlas_gen;
use bevy::asset::{uuid_handle, RenderAssetUsages};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::{Shader, ShaderRef};

/// The chunk material: StandardMaterial plus the tile-wrapping shader.
pub type VoxMat = ExtendedMaterial<StandardMaterial, AtlasTiles>;

/// The extension: no data of its own, only the fragment shader.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct AtlasTiles {}

const SHADER: Handle<Shader> = uuid_handle!("5f2b7a64-1c0e-4c1f-9a3d-52a7b31e0954");

impl MaterialExtension for AtlasTiles {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHADER)
    }
}

/// Registers the chunk material and its shader (voxel mode only).
pub fn register(app: &mut App) {
    app.world_mut()
        .resource_mut::<Assets<Shader>>()
        .insert(
            SHADER.id(),
            Shader::from_wesl(
                include_str!("voxel_atlas.wesl"),
                "realgame/voxel_atlas.wesl",
            ),
        )
        .ok();
    app.add_plugins(MaterialPlugin::<VoxMat>::default());
}

fn image(size: u32, data: Vec<u8>) -> Image {
    let mut img = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..ImageSamplerDescriptor::default()
    });
    img
}

/// UV_1 for a vertex of material `mat` on a face with normal `n`: the
/// origin of its tile in the atlas.
pub fn tile_origin(mat: u8, n: [f32; 3]) -> [f32; 2] {
    let face = if n[1] > 0.5 {
        atlas_gen::Face::Top
    } else if n[1] < -0.5 {
        atlas_gen::Face::Bottom
    } else {
        atlas_gen::Face::Side
    };
    let r = atlas_gen::uv_rect(atlas_gen::tile_for(mat, face));
    [r[0], r[1]]
}

/// UV_0 for a vertex at `p` on a face with normal `n`, in blocks: top and
/// bottom faces map x and z; side faces map their horizontal axis and -y,
/// so a side tile's top row is at the block's top (the grass lip).
pub fn block_uv(p: [f32; 3], n: [f32; 3]) -> [f32; 2] {
    if n[1].abs() > 0.5 {
        [p[0], p[2]]
    } else if n[0].abs() > 0.5 {
        [p[2], -p[1]]
    } else {
        [p[0], -p[1]]
    }
}

/// The two chunk materials: `opaque` (lit, rough) and `clear` (water, ice,
/// glass, crystal, steam: alpha blend, both sides).
pub struct ChunkMats {
    pub opaque: Handle<VoxMat>,
    pub clear: Handle<VoxMat>,
}

pub fn materials(images: &mut Assets<Image>, mats: &mut Assets<VoxMat>, seed: u64) -> ChunkMats {
    let a = atlas_gen::generate_atlas(seed);
    let base = images.add(image(a.size, a.rgba));
    let glow = images.add(image(a.size, a.glow));
    let mut make = |clear: bool| {
        mats.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(base.clone()),
                emissive: LinearRgba::rgb(2.0, 2.0, 2.0),
                emissive_texture: Some(glow.clone()),
                perceptual_roughness: if clear { 0.15 } else { 0.9 },
                reflectance: if clear { 0.5 } else { 0.2 },
                alpha_mode: if clear {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                cull_mode: if clear {
                    None
                } else {
                    Some(bevy::render::render_resource::Face::Back)
                },
                double_sided: clear,
                ..default()
            },
            extension: AtlasTiles {},
        })
    };
    ChunkMats {
        opaque: make(false),
        clear: make(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::mat;

    #[test]
    fn faces_pick_their_tiles() {
        let up = [0.0, 1.0, 0.0];
        let side = [1.0, 0.0, 0.0];
        assert_eq!(tile_origin(mat::GRASS, up), {
            let r = atlas_gen::uv_rect(mat::GRASS);
            [r[0], r[1]]
        });
        let r = atlas_gen::uv_rect(atlas_gen::GRASS_SIDE);
        assert_eq!(tile_origin(mat::GRASS, side), [r[0], r[1]]);
    }

    #[test]
    fn side_uvs_run_down_from_the_top() {
        // On a side face, v grows downward: the top edge of a block is v = -top.
        let a = block_uv([3.0, 35.0, 7.0], [0.0, 0.0, 1.0]);
        let b = block_uv([3.0, 34.5, 7.0], [0.0, 0.0, 1.0]);
        assert!(b[1] > a[1]);
        assert_eq!(block_uv([2.0, 9.0, 5.0], [0.0, 1.0, 0.0]), [2.0, 5.0]);
    }
}
