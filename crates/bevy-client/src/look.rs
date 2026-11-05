//! The realistic world look (`?look=new`). Paawan, 2026-11-04: "a game
//! that is real to this real world"; references Teardown, Half-Life: Alyx,
//! Noita (docs/graphics/world-look-realistic.md).
//!
//! - **Real daylight:** `mood` picks a second set of moods (a near-white
//!   sun, blue sky fill, haze the colour of real air) and the sky shader
//!   draws a physical sky (Rayleigh and Mie scattering) for the sun's
//!   direction.
//! - **Scanned ground:** free CC0 photo scans (Poly Haven: grass, soil,
//!   rock, snow; `tools/textures/fetch.py`) as albedo and normal texture
//!   arrays on the smooth terrain. The browser decodes the JPEGs and builds
//!   the mip chain (`web/terrain-tex.js`); this module turns them into
//!   textures once they arrive.
//!
//! Off by default, so the shipped look stays as it is until Paawan picks
//! the new one after playing it.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDataOrder, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::view::{ColorGrading, ColorGradingGlobal};

use crate::mood::Mood;

/// Whether the realistic look is on (`?look=new`).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct NewLook(pub bool);

/// The ground textures, once made: albedo and normal arrays.
#[derive(Resource, Default)]
pub struct GroundTex {
    pub ready: bool,
}

pub fn plugin(app: &mut App) {
    let on = crate::param("look").is_some_and(|v| v == "new");
    app.insert_resource(NewLook(on))
        .init_resource::<GroundTex>();
    if on {
        app.add_systems(Update, load_ground);
    }
}

/// The decoded ground maps from `web/terrain-tex.js`: tile size, layers,
/// mip levels, and the albedo and normal bytes (RGBA8, layer-major, each
/// layer's mips from largest to smallest).
pub struct GroundMaps {
    pub size: u32,
    pub layers: u32,
    pub levels: u32,
    pub albedo: Vec<u8>,
    pub normal: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
fn take_maps() -> Option<GroundMaps> {
    use js_sys::{Reflect, Uint8Array};
    use wasm_bindgen::JsValue;
    let w = web_sys::window()?;
    let t = Reflect::get(&w, &JsValue::from_str("__terrainTex")).ok()?;
    if t.is_undefined() || t.is_null() {
        return None;
    }
    let num = |k: &str| Reflect::get(&t, &JsValue::from_str(k)).ok()?.as_f64();
    let bytes = |k: &str| -> Option<Vec<u8>> {
        let v = Reflect::get(&t, &JsValue::from_str(k)).ok()?;
        Some(Uint8Array::new(&v).to_vec())
    };
    let maps = GroundMaps {
        size: num("size")? as u32,
        layers: num("layers")? as u32,
        levels: num("levels")? as u32,
        albedo: bytes("albedo")?,
        normal: bytes("normal")?,
    };
    // Free the page's copy.
    let _ = Reflect::set(&w, &JsValue::from_str("__terrainTex"), &JsValue::NULL);
    Some(maps)
}

#[cfg(not(target_arch = "wasm32"))]
fn take_maps() -> Option<GroundMaps> {
    None
}

/// The bytes a layer-major RGBA8 array with `levels` mips takes.
pub fn array_bytes(size: u32, layers: u32, levels: u32) -> usize {
    let per_layer: usize = (0..levels)
        .map(|l| {
            let s = (size >> l).max(1) as usize;
            s * s * 4
        })
        .sum();
    per_layer * layers as usize
}

/// A texture array from decoded bytes, sampled with trilinear filtering
/// and repeat (the ground tiles).
pub fn array_image(m: &GroundMaps, data: Vec<u8>, srgb: bool) -> Image {
    let mut img = Image {
        data: Some(data),
        data_order: TextureDataOrder::LayerMajor,
        texture_descriptor: TextureDescriptor {
            label: Some("ground"),
            size: Extent3d {
                width: m.size,
                height: m.size,
                depth_or_array_layers: m.layers,
            },
            mip_level_count: m.levels,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: if srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        texture_view_descriptor: Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..default()
        }),
        asset_usage: RenderAssetUsages::RENDER_WORLD,
        ..default()
    };
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    img
}

/// Waits for the page's decoded ground maps, then hands them to the
/// smooth terrain's materials.
fn load_ground(
    mut ground: ResMut<GroundTex>,
    handles: Option<Res<crate::voxel_view::SmoothHandles>>,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<crate::voxel_tex::SmoothMat>>,
) {
    if ground.ready {
        return;
    }
    let Some(handles) = handles else {
        return;
    };
    let Some(m) = take_maps() else {
        return;
    };
    let want = array_bytes(m.size, m.layers, m.levels);
    if m.albedo.len() != want || m.normal.len() != want {
        warn!(
            "ground textures: expected {want} bytes, got {} and {}",
            m.albedo.len(),
            m.normal.len()
        );
        ground.ready = true;
        return;
    }
    let albedo = images.add(array_image(&m, m.albedo.clone(), true));
    let normal = images.add(array_image(&m, m.normal.clone(), false));
    for id in handles.0.iter() {
        if let Some(mut mat) = mats.get_mut(id) {
            mat.extension.ground_albedo = Some(albedo.clone());
            mat.extension.ground_normal = Some(normal.clone());
            mat.extension.ground.x = 1.0;
            // How many layers there are: the models' textures (bark,
            // leaves, planks, brick, metal) follow the ground's four.
            mat.extension.ground.y = m.layers as f32;
        }
    }
    info!("ground textures: {} layers of {} px", m.layers, m.size);
    ground.ready = true;
}

/// Applies the live mood's grade to the camera (called from
/// `mood::apply`, which owns the easing).
pub fn apply(m: &Mood, grades: &mut Query<&mut ColorGrading>) {
    for mut g in grades.iter_mut() {
        let want = ColorGradingGlobal {
            exposure: m.grade.x,
            post_saturation: m.grade.y,
            temperature: m.grade.z,
            ..default()
        };
        let cur = &g.global;
        if (cur.exposure - want.exposure).abs() > 0.005
            || (cur.post_saturation - want.post_saturation).abs() > 0.005
            || (cur.temperature - want.temperature).abs() > 0.005
        {
            g.global = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_bytes_counts_every_mip_of_every_layer() {
        // 4 x 4: 64 + 16 + 4 bytes a layer (4, 2, 1 px).
        assert_eq!(array_bytes(4, 2, 3), 2 * (64 + 16 + 4));
        assert_eq!(
            array_bytes(1024, 4, 11),
            4 * 4 * (0..11).map(|l| (1024usize >> l).pow(2)).sum::<usize>()
        );
    }

    #[test]
    fn the_array_image_is_a_mipped_srgb_array() {
        let m = GroundMaps {
            size: 4,
            layers: 2,
            levels: 3,
            albedo: vec![],
            normal: vec![],
        };
        let img = array_image(&m, vec![0; array_bytes(4, 2, 3)], true);
        assert_eq!(img.texture_descriptor.size.depth_or_array_layers, 2);
        assert_eq!(img.texture_descriptor.mip_level_count, 3);
        assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
    }
}
