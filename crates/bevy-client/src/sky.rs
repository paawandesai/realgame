//! The smooth world's sky (voxel mode): a big sphere around the camera with
//! a procedural gradient, sun and drifting clouds (`sky.wesl`); no assets.

use bevy::asset::uuid_handle;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::{Shader, ShaderRef};

const SHADER: Handle<Shader> = uuid_handle!("3a8e6f12-7c4d-4b9a-b1e2-6d0f5a7c9e21");
/// Inside the camera's far plane (400 m in voxel mode).
const RADIUS: f32 = 360.0;

/// The sky's material.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct SkyMat {
    /// `low.x` is 1 with `?low` (one-octave clouds, a still aurora).
    #[uniform(0)]
    pub low: Vec4,
}

impl Material for SkyMat {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHADER)
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Seen from inside.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(Component)]
struct SkyDome;

/// Registers the sky (voxel mode only).
pub fn plugin(app: &mut App) {
    app.world_mut()
        .resource_mut::<Assets<Shader>>()
        .insert(
            SHADER.id(),
            Shader::from_wesl(include_str!("sky.wesl"), "realgame/sky.wesl"),
        )
        .ok();
    app.add_plugins(MaterialPlugin::<SkyMat>::default())
        .add_systems(Startup, setup)
        .add_systems(PostUpdate, follow.before(TransformSystems::Propagate));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<SkyMat>>,
    settings: Res<crate::Settings>,
) {
    let low = if settings.low { 1.0 } else { 0.0 };
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(RADIUS).mesh().uv(32, 16))),
        MeshMaterial3d(mats.add(SkyMat {
            low: Vec4::new(low, 0.0, 0.0, 0.0),
        })),
        Transform::default(),
        NotShadowCaster,
        NotShadowReceiver,
        SkyDome,
        bevy::camera::visibility::NoFrustumCulling,
    ));
}

fn follow(
    cam: Query<&Transform, (With<Camera3d>, Without<SkyDome>)>,
    mut sky: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(c) = cam.iter().next() else {
        return;
    };
    for mut t in &mut sky {
        t.translation = c.translation;
    }
}
