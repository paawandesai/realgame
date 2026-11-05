//! Environment light with no image files (plan "Materials and lighting").
//!
//! Two routes, so the matrix can compare them:
//! - `envmap_gen`: a cube map of the lab drawn in code, filtered on the GPU
//!   by Bevy's `GeneratedEnvironmentMapLight` (needs compute shaders).
//! - `envmap_cpu`: the same lab, "pre-filtered" on the CPU at startup into
//!   a small diffuse cube and a specular cube with a mip per roughness step,
//!   then used as a plain `EnvironmentMapLight` (should run on WebGL2).
//!
//! The lab is a smooth analytic function (dark walls, a warm window, a cool
//! ceiling glow), so blurring it is just blending each direction towards its
//! cosine-weighted average; no convolution is needed.

use bevy::asset::RenderAssetUsages;
use bevy::light::EnvironmentMapLight;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

/// Direction of each cube face texel (wgpu face order +X, -X, +Y, -Y, +Z, -Z).
fn face_dir(face: usize, u: f32, v: f32) -> Vec3 {
    match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    }
    .normalize()
}

/// The lab's radiance in a direction (linear RGB, roughly cd/m² / 1000).
fn lab(d: Vec3) -> Vec3 {
    let wall = Vec3::new(0.05, 0.04, 0.035);
    // Window: up and to the left-front, warm daylight.
    let win_dir = Vec3::new(-0.6, 0.55, 0.6).normalize();
    let w = d.dot(win_dir);
    let window = Vec3::new(1.0, 0.85, 0.65) * 6.0 * smooth(0.93, 0.97, w);
    // Ceiling: a faint cool glow; floor: darker.
    let ceiling = Vec3::new(0.12, 0.16, 0.22) * smooth(0.2, 0.9, d.y);
    let floor = Vec3::new(-0.02, -0.02, -0.02) * smooth(0.0, -0.8, d.y).max(0.0);
    (wall + window + ceiling + floor).max(Vec3::ZERO)
}

/// The lab blurred to roughness `r` (0 sharp, 1 fully diffuse).
fn lab_blurred(d: Vec3, r: f32) -> Vec3 {
    lab(d).lerp(lab_diffuse(d), (r * 1.2).min(1.0))
}

/// Cosine-weighted average of the lab around `d`, from a fixed set of
/// sample directions.
fn lab_diffuse(d: Vec3) -> Vec3 {
    let mut sum = Vec3::ZERO;
    let mut wsum = 0.0;
    for i in 0..256 {
        // Fibonacci sphere.
        let y = 1.0 - (i as f32 + 0.5) / 128.0;
        let rr = (1.0 - y * y).max(0.0).sqrt();
        let a = i as f32 * 2.399_963;
        let s = Vec3::new(a.cos() * rr, y, a.sin() * rr);
        let w = s.dot(d).max(0.0);
        sum += lab(s) * w;
        wsum += w;
    }
    sum / wsum.max(1e-4)
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// f32 to IEEE half (no NaN handling needed for these values).
fn f16_bits(x: f32) -> u16 {
    let b = x.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp <= 0 {
        return sign;
    }
    if exp >= 31 {
        return sign | 0x7c00;
    }
    sign | ((exp as u16) << 10) | ((man >> 13) as u16)
}

fn push_texel(out: &mut Vec<u8>, c: Vec3) {
    for v in [c.x, c.y, c.z, 1.0] {
        out.extend_from_slice(&f16_bits(v).to_le_bytes());
    }
}

/// A cube image with `mips` levels; `f(dir, level)` gives each texel.
fn cube(size: u32, mips: u32, f: impl Fn(Vec3, u32) -> Vec3) -> Image {
    let mut data = Vec::new();
    // Layer-major: every mip of face 0, then face 1, and so on.
    for face in 0..6 {
        for level in 0..mips {
            let s = (size >> level).max(1);
            for y in 0..s {
                for x in 0..s {
                    let u = (x as f32 + 0.5) / s as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / s as f32 * 2.0 - 1.0;
                    push_texel(&mut data, f(face_dir(face, u, v), level));
                }
            }
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        vec![0; (size * size * 6 * 8) as usize],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(data);
    img.texture_descriptor.mip_level_count = mips;
    img.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    img
}

/// The sharp lab cube (one mip), for `GeneratedEnvironmentMapLight`.
pub fn sky_cubemap(images: &mut Assets<Image>, size: u32) -> Handle<Image> {
    images.add(cube(size, 1, |d, _| lab(d)))
}

/// CPU-filtered diffuse and specular cubes for a plain `EnvironmentMapLight`.
pub fn cpu_environment(images: &mut Assets<Image>) -> EnvironmentMapLight {
    let spec_size: u32 = 64;
    let mips = spec_size.ilog2() + 1;
    let specular = cube(spec_size, mips, |d, level| {
        lab_blurred(d, level as f32 / (mips - 1) as f32)
    });
    let diffuse = cube(16, 1, |d, _| lab_diffuse(d));
    EnvironmentMapLight {
        diffuse_map: images.add(diffuse),
        specular_map: images.add(specular),
        intensity: 900.0,
        rotation: Quat::IDENTITY,
        affects_lightmapped_mesh_diffuse: true,
    }
}
