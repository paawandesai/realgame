//! Graphics G0 spike (docs/graphics/feature-matrix.md).
//!
//! One small Bench scene: a dark wood bench, a glass chamber and three
//! molecules (H2O, CH4, CO2) at real bond lengths and angles, drawn with the
//! ray-traced atom impostors that G1 will use. Each Bevy rendering feature is
//! switched on from the URL (`?f=shadows,transmission,ssao`) so a headless
//! browser can load the page once per feature on WebGL2 and on WebGPU and
//! record whether it runs, what it logs and what a frame costs on the CPU.
//!
//! The result is published for the test script as `window.__gfxlab`.
//! Nothing here is used by the game; it has its own Cargo workspace.

mod atoms;
mod envmap;
mod report;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::smaa::Smaa;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{
    CascadeShadowConfigBuilder, FogVolume, GeneratedEnvironmentMapLight, GlobalAmbientLight,
    VolumetricFog, VolumetricLight,
};
use bevy::log::LogPlugin;
use bevy::pbr::{
    ContactShadows, DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion, ScreenSpaceReflections,
    ScreenSpaceTransmission, ScreenSpaceTransmissionQuality,
};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, Msaa};

/// The features this run switched on (`?f=a,b,c`).
#[derive(Resource, Clone, Default)]
pub struct Features(pub Vec<String>);

impl Features {
    pub fn on(&self, name: &str) -> bool {
        self.0.iter().any(|f| f == name)
    }
}

/// Every feature the spike knows, in the order the matrix lists them.
pub const ALL: &[&str] = &[
    "base",
    "msaa",
    "fxaa",
    "smaa",
    "taa",
    "bloom",
    "grading",
    "shadows",
    "transmission",
    "clearcoat",
    "envmap_cpu",
    "envmap_gen",
    "depth_prepass",
    "ssao",
    "contact_shadows",
    "ssr",
    "volfog",
    "dof",
    "distance_fog",
];

fn features_from_url() -> Features {
    #[cfg(target_arch = "wasm32")]
    {
        let search = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        let list = search
            .trim_start_matches('?')
            .split('&')
            .find_map(|kv| kv.strip_prefix("f="))
            .unwrap_or("base");
        return Features(list.split(',').map(|s| s.to_string()).collect());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let list = std::env::args()
            .find_map(|a| a.strip_prefix("--f=").map(str::to_string))
            .unwrap_or_else(|| "base".into());
        Features(list.split(',').map(|s| s.to_string()).collect())
    }
}

fn main() {
    let features = features_from_url();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Formula gfx-lab".into(),
                        canvas: Some("#lab".into()),
                        fit_canvas_to_parent: true,
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(LogPlugin {
                    // Keep every warning: which plugin refuses to run on
                    // which API is exactly what this spike records.
                    filter: "warn,realgame_gfx_lab=info,wgpu_core=warn,wgpu_hal=warn".into(),
                    ..default()
                }),
        )
        .insert_resource(features)
        .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.75, 0.8, 1.0),
            brightness: 120.0,
            affects_lightmapped_meshes: true,
        })
        .add_plugins((atoms::plugin, report::plugin))
        .add_systems(Startup, setup)
        .add_systems(Update, orbit)
        .run();
}

#[derive(Component)]
struct LabCam;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    f: Res<Features>,
) {
    // Room: a dark back wall and floor, so the chamber is the brightest thing.
    let wall = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.15, 0.13),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 0.1, 6.0))),
        MeshMaterial3d(wall.clone()),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 4.0, 0.1))),
        MeshMaterial3d(wall),
        Transform::from_xyz(0.0, 2.0, -2.0),
    ));

    // Bench: dark oak top on two legs.
    let oak = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.2, 0.12),
        perceptual_roughness: 0.55,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.4, 0.08, 1.0))),
        MeshMaterial3d(oak.clone()),
        Transform::from_xyz(0.0, 0.9, 0.0),
    ));
    for x in [-1.05, 1.05] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.1, 0.86, 0.8))),
            MeshMaterial3d(oak.clone()),
            Transform::from_xyz(x, 0.43, 0.0),
        ));
    }

    // Brass base ring and the glass chamber.
    let brass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.6, 0.3),
        metallic: 1.0,
        perceptual_roughness: 0.3,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.3, 0.05))),
        MeshMaterial3d(brass),
        Transform::from_xyz(0.0, 0.965, 0.0),
    ));
    let glass = if f.on("transmission") {
        StandardMaterial {
            base_color: Color::srgb(0.92, 1.0, 0.98),
            specular_transmission: 0.95,
            diffuse_transmission: 0.0,
            thickness: 0.004,
            ior: 1.5,
            perceptual_roughness: 0.04,
            ..default()
        }
    } else {
        // The Low-tier fallback: alpha-blended glass with a Fresnel-ish edge
        // from the low roughness, no refraction.
        StandardMaterial {
            base_color: Color::srgba(0.85, 1.0, 0.97, 0.14),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.05,
            reflectance: 0.6,
            ..default()
        }
    };
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.28, 0.6).mesh().resolution(48))),
        MeshMaterial3d(materials.add(glass)),
        Transform::from_xyz(0.0, 1.29, 0.0),
    ));

    // Molecules: water inside the chamber, methane and CO2 on the bench.
    atoms::spawn_molecule(&mut commands, &atoms::water(), Vec3::new(0.0, 1.3, 0.0));
    atoms::spawn_molecule(
        &mut commands,
        &atoms::methane(),
        Vec3::new(-0.75, 1.1, 0.15),
    );
    atoms::spawn_molecule(&mut commands, &atoms::co2(), Vec3::new(0.75, 1.1, 0.15));

    // Window key light.
    let mut sun = commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            color: Color::srgb(1.0, 0.9, 0.78),
            shadow_maps_enabled: f.on("shadows"),
            ..default()
        },
        Transform::from_xyz(-2.0, 3.0, 2.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            minimum_distance: 0.1,
            maximum_distance: 8.0,
            first_cascade_far_bound: 8.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
    if f.on("volfog") {
        sun.insert(VolumetricLight);
    }
    // Cool chamber light under the glass.
    commands.spawn((
        PointLight {
            color: Color::srgb(0.4, 1.0, 0.9),
            intensity: 2_000.0,
            range: 3.0,
            shadow_maps_enabled: f.on("shadows"),
            ..default()
        },
        Transform::from_xyz(0.0, 1.05, 0.0),
    ));

    // Camera.
    let mut cam = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.55, 2.2).looking_at(Vec3::new(0.0, 1.15, 0.0), Vec3::Y),
        Tonemapping::SomewhatBoringDisplayTransform,
        LabCam,
    ));
    cam.insert(if f.on("msaa") {
        Msaa::Sample4
    } else {
        Msaa::Off
    });
    let hdr = f.on("bloom") || f.on("ssao") || f.on("ssr") || f.on("taa") || f.on("dof");
    if hdr {
        cam.insert(Hdr);
    }
    if f.on("fxaa") {
        cam.insert(Fxaa::default());
    }
    if f.on("smaa") {
        cam.insert(Smaa::default());
    }
    if f.on("taa") {
        cam.insert(TemporalAntiAliasing::default());
    }
    if f.on("bloom") {
        cam.insert(Bloom::NATURAL);
    }
    if f.on("grading") {
        cam.insert(ColorGrading {
            global: ColorGradingGlobal {
                exposure: 0.2,
                post_saturation: 0.9,
                ..default()
            },
            ..default()
        });
    }
    if f.on("transmission") {
        cam.insert(ScreenSpaceTransmission {
            steps: 1,
            quality: ScreenSpaceTransmissionQuality::Medium,
        });
    }
    if f.on("depth_prepass") {
        cam.insert(DepthPrepass);
    }
    if f.on("ssao") {
        cam.insert(ScreenSpaceAmbientOcclusion::default());
    }
    if f.on("contact_shadows") {
        cam.insert(ContactShadows::default());
    }
    if f.on("ssr") {
        cam.insert(ScreenSpaceReflections::default());
    }
    if f.on("dof") {
        cam.insert(DepthOfField {
            focal_distance: 1.2,
            ..default()
        });
    }
    if f.on("distance_fog") {
        cam.insert(DistanceFog {
            color: Color::srgb(0.08, 0.09, 0.12),
            falloff: FogFalloff::Linear {
                start: 3.0,
                end: 9.0,
            },
            ..default()
        });
    }
    if f.on("volfog") {
        cam.insert(VolumetricFog::default());
    }
    if f.on("envmap_cpu") {
        let env = envmap::cpu_environment(&mut images);
        cam.insert(env);
    }
    if f.on("envmap_gen") {
        let cube = envmap::sky_cubemap(&mut images, 64);
        cam.insert(GeneratedEnvironmentMapLight {
            environment_map: cube,
            intensity: 900.0,
            ..default()
        });
    }
    if f.on("volfog") {
        commands.spawn((
            FogVolume::default(),
            Transform::from_scale(Vec3::splat(8.0)),
        ));
    }
}

/// A slow orbit, so TAA and the frame-time sample see a moving camera.
fn orbit(time: Res<Time>, mut cam: Query<&mut Transform, With<LabCam>>) {
    let a = (time.elapsed_secs() * 0.2).sin() * 0.35;
    for mut t in &mut cam {
        *t = Transform::from_xyz(2.2 * a.sin(), 1.55, 2.2 * a.cos())
            .looking_at(Vec3::new(0.0, 1.15, 0.0), Vec3::Y);
    }
}
