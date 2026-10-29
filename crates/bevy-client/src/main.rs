//! Spike: Alchemist's Trial's current 2D simulation, rendered in 3D by Bevy.
//!
//! The simulation (`realgame-sim`) is unchanged and steps at its fixed 60 Hz
//! in `FixedUpdate`. Every rendered frame, the cells around the camera become
//! a thin voxel slab (see `slab.rs`): lit solids, glowing flames with bloom,
//! translucent gases. A perspective camera follows the player.
//!
//! Keys: A/D or arrows walk, Space jumps, 1 water, 2 boom, 3 split, 4 fire
//! (each aims at the room's goal first). On the web, `?auto` walks and casts
//! by itself so a headless browser can measure a busy scene.

mod slab;

use bevy::asset::{AssetMetaCheck, RenderAssetUsages};
#[cfg(feature = "bloom")]
use bevy::camera::Hdr;
#[cfg(not(feature = "bloom"))]
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::GlobalAmbientLight;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::time::Instant;
#[cfg(feature = "bloom")]
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use realgame_sim::game::{
    Game, Tier, HEIGHT, INPUT_JUMP, INPUT_LEFT, INPUT_RIGHT, PLAYER_H, PLAYER_W, VIEW_W, WORLD_W,
};
use slab::Slab;

/// Cells drawn either side of the 320-cell view, so the perspective camera
/// never sees the slab end.
const MARGIN: usize = 48;
/// Point lights placed at the hottest spots on screen.
const FIRE_LIGHTS: usize = 4;

#[derive(Resource)]
struct Sim {
    game: Game,
    input: u32,
    jump: bool,
    auto: bool,
    /// `?frozen`: the sim does not step (measures rendering cost alone).
    frozen: bool,
    /// Slowest single tick, total tick time and tick count since the last report.
    sim_ms: f32,
    sim_total_ms: f32,
    ticks_run: u32,
}

#[derive(Resource)]
struct SlabMeshes {
    slab: Slab,
    solid: Handle<Mesh>,
    glow: Handle<Mesh>,
    gas: Handle<Mesh>,
}

#[derive(Resource, Default)]
struct Stats {
    frames: u64,
    window_start: Option<Instant>,
    window_frames: u32,
    worst_ms: f32,
    last: Option<Instant>,
    build_ms: f32,
    upload_ms: f32,
}

#[derive(Component)]
struct PlayerBody;

#[derive(Component)]
struct FireLight(usize);

fn main() {
    let window = Window {
        title: "Alchemist's Trial 3D spike".into(),
        canvas: Some("#game".into()),
        fit_canvas_to_parent: true,
        prevent_default_event_handling: true,
        resolution: (1280, 720).into(),
        ..default()
    };
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(window),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb_u8(52, 78, 128)))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 400.0,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(Sim {
            game: Game::new(Tier::parse("apprentice"), 1),
            input: 0,
            jump: false,
            auto: auto_mode(),
            frozen: has_flag("frozen"),
            sim_ms: 0.0,
            sim_total_ms: 0.0,
            ticks_run: 0,
        })
        .init_resource::<Stats>()
        .add_systems(Startup, setup)
        .add_systems(Update, read_input)
        .add_systems(FixedUpdate, step_sim)
        .add_systems(
            Update,
            (rebuild_slab, follow_player, place_lights, report)
                .chain()
                .after(read_input),
        )
        .run();
}

/// Whether the page URL (web) or the command line (native) has `flag`.
#[cfg(target_arch = "wasm32")]
fn has_flag(flag: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|s| s.contains(flag))
}

#[cfg(not(target_arch = "wasm32"))]
fn has_flag(flag: &str) -> bool {
    std::env::args().any(|a| a == format!("--{flag}"))
}

fn auto_mode() -> bool {
    has_flag("auto")
}

/// Milliseconds since the page started loading (web) or since start (native).
fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            * 1000.0
    }
}

/// The WebAssembly linear memory in MB (0 on native).
fn wasm_memory_mb() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        core::arch::wasm32::memory_size(0) as f32 * 65536.0 / 1_048_576.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

fn empty_mesh() -> Mesh {
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    m.insert_indices(Indices::U32(Vec::new()));
    m
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("STARTUP t={:.0}ms", now_ms());
    let solid = meshes.add(empty_mesh());
    let glow = meshes.add(empty_mesh());
    let gas = meshes.add(empty_mesh());

    // Vertex colours multiply the white base colour.
    let solid_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        ..default()
    });
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(6.0, 2.4, 0.4),
        unlit: true,
        ..default()
    });
    let gas_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(solid.clone()),
        MeshMaterial3d(solid_mat),
        Transform::default(),
    ));
    commands.spawn((
        Mesh3d(glow.clone()),
        MeshMaterial3d(glow_mat),
        Transform::default(),
    ));
    commands.spawn((
        Mesh3d(gas.clone()),
        MeshMaterial3d(gas_mat),
        Transform::default(),
    ));
    commands.insert_resource(SlabMeshes {
        slab: Slab::default(),
        solid,
        glow,
        gas,
    });

    // Back wall: a dim stone panel behind the slab so the depth reads.
    let wall = meshes.add(Cuboid::new(
        WORLD_W as f32 + 400.0,
        HEIGHT as f32 + 200.0,
        1.0,
    ));
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(40, 48, 72),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(wall),
        MeshMaterial3d(wall_mat),
        Transform::from_xyz(
            WORLD_W as f32 / 2.0,
            HEIGHT as f32 / 2.0,
            -slab::SOLID_DEPTH - 6.0,
        ),
    ));

    // The player: a box the size of the collision box.
    let body = meshes.add(Cuboid::new(PLAYER_W, PLAYER_H, 4.0));
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(240, 228, 66),
        emissive: LinearRgba::rgb(0.3, 0.28, 0.05),
        ..default()
    });
    commands.spawn((
        Mesh3d(body),
        MeshMaterial3d(body_mat),
        Transform::default(),
        PlayerBody,
    ));

    // Sun from the upper left and in front, no shadow maps (cheap on Chromebooks).
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-1.0, 2.0, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // `?lite`: the "Low" tier. No HDR, bloom or fire point lights.
    let lite = has_flag("lite");
    let lights = if lite { 0 } else { FIRE_LIGHTS };
    for k in 0..lights {
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.6, 0.2),
                intensity: 0.0,
                range: 90.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 10.0),
            FireLight(k),
        ));
    }

    let mut camera = commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 40f32.to_radians(),
            far: 2000.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 100.0, 230.0),
    ));
    #[cfg(feature = "bloom")]
    if !lite {
        camera.insert((Hdr, Bloom::NATURAL));
    }
    #[cfg(not(feature = "bloom"))]
    camera.insert(Tonemapping::Reinhard);
    info!("tier={}", if lite { "lite" } else { "full" });
}

fn read_input(keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<Sim>) {
    let mut input = 0;
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        input |= INPUT_LEFT;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        input |= INPUT_RIGHT;
    }
    if keys.pressed(KeyCode::Space) {
        input |= INPUT_JUMP;
    }
    if keys.just_pressed(KeyCode::Space) {
        sim.jump = true;
    }
    sim.input = input;
    for (key, spell) in [
        (KeyCode::Digit1, "water"),
        (KeyCode::Digit2, "boom"),
        (KeyCode::Digit3, "split"),
        (KeyCode::Digit4, "fire"),
    ] {
        if keys.just_pressed(key) {
            sim.game.aim_at_goal();
            let out = sim.game.cast(spell);
            info!("cast {spell}: {:?}", out.outcome);
        }
    }
}

/// Fixed 60 Hz, like the browser build. `?auto` walks right in bursts and
/// casts the four spells in turn at the goal, to keep the scene busy.
fn step_sim(mut sim: ResMut<Sim>) {
    if sim.frozen {
        return;
    }
    let t0 = Instant::now();
    let mut input = sim.input;
    if sim.jump {
        sim.game.press_jump();
        sim.jump = false;
    }
    if sim.auto {
        let tick = sim.game.ticks;
        if tick % 240 < 90 {
            input |= INPUT_RIGHT;
        }
        if tick % 50 == 25 {
            let spell = ["fire", "boom", "water", "split"][(tick / 50 % 4) as usize];
            sim.game.aim_at_goal();
            sim.game.cast(spell);
        }
    }
    sim.game.tick(input);
    let ms = t0.elapsed().as_secs_f32() * 1000.0;
    sim.sim_ms = sim.sim_ms.max(ms);
    sim.sim_total_ms += ms;
    sim.ticks_run += 1;
}

fn rebuild_slab(
    sim: Res<Sim>,
    mut slab: ResMut<SlabMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut stats: ResMut<Stats>,
) {
    // Frozen: the grid never changes, so after the first frames the meshes
    // stay as they are (this measures Bevy's own per-frame floor).
    if sim.frozen && stats.frames > 5 {
        return;
    }
    let t0 = Instant::now();
    let left = sim.game.view_x.round() as usize;
    let x0 = left.saturating_sub(MARGIN);
    let x1 = (left + VIEW_W + MARGIN).min(WORLD_W);
    let SlabMeshes {
        slab: s,
        solid,
        glow,
        gas,
    } = &mut *slab;
    s.build(&sim.game.grid, x0, x1);
    let t1 = Instant::now();
    for (h, data) in [(&*solid, &s.solid), (&*glow, &s.glow), (&*gas, &s.gas)] {
        if let Some(mut m) = meshes.get_mut(h) {
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions.clone());
            m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals.clone());
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colors.clone());
            m.insert_indices(Indices::U32(data.indices.clone()));
        }
    }
    let build = (t1 - t0).as_secs_f32() * 1000.0;
    let upload = t1.elapsed().as_secs_f32() * 1000.0;
    stats.build_ms = stats.build_ms.max(build);
    stats.upload_ms = stats.upload_ms.max(upload);
}

fn follow_player(
    sim: Res<Sim>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    mut body: Query<&mut Transform, (With<PlayerBody>, Without<Camera3d>)>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    let p = &sim.game.player;
    let a = fixed.overstep_fraction();
    let px = p.prev_x + (p.x - p.prev_x) * a + PLAYER_W / 2.0;
    let py = p.prev_y + (p.y - p.prev_y) * a + PLAYER_H / 2.0;
    let world = Vec3::new(px, HEIGHT as f32 - py, 0.0);
    if let Ok(mut t) = body.single_mut() {
        t.translation = world;
    }
    if let Ok(mut t) = cam.single_mut() {
        // In front and above, looking down about 15 degrees: the floor's top
        // faces and the slab's depth both show, with no sideways skew.
        let want = Vec3::new(world.x + 30.0, world.y + 70.0, 260.0);
        let k = 1.0 - (-6.0 * time.delta_secs()).exp();
        t.translation = t.translation.lerp(want, k);
        let look = Vec3::new(t.translation.x, t.translation.y - 60.0, 0.0);
        t.look_at(look, Vec3::Y);
    }
}

fn place_lights(sim: Res<Sim>, mut lights: Query<(&mut PointLight, &mut Transform, &FireLight)>) {
    let spots = sim
        .game
        .hotspots(sim.game.view_x.round() as i32, FIRE_LIGHTS);
    for (mut light, mut t, FireLight(k)) in &mut lights {
        if let Some(&(x, y, temp)) = spots.get(*k) {
            light.intensity = (temp / 1000.0).clamp(0.3, 3.0) * 2.0e7;
            t.translation = Vec3::new(x as f32, (HEIGHT as i32 - y) as f32, 14.0);
        } else {
            light.intensity = 0.0;
        }
    }
}

/// Logs a line every 2 seconds for the measurement script to parse.
fn report(mut stats: ResMut<Stats>, mut sim: ResMut<Sim>, slab: Res<SlabMeshes>) {
    let now = Instant::now();
    stats.frames += 1;
    if stats.frames == 3 {
        info!(
            "FIRST_FRAME t={:.0}ms quads={}",
            now_ms(),
            slab.slab.quads()
        );
    }
    if let Some(last) = stats.last {
        let dt = (now - last).as_secs_f32() * 1000.0;
        stats.worst_ms = stats.worst_ms.max(dt);
    }
    stats.last = Some(now);
    let start = *stats.window_start.get_or_insert(now);
    stats.window_frames += 1;
    let span = (now - start).as_secs_f32();
    if span >= 2.0 {
        info!(
            "STATS fps={:.1} avg_ms={:.2} worst_ms={:.1} sim_ms_per_frame={:.2} ticks_per_frame={:.2} sim_max_ms={:.2} build_max_ms={:.2} upload_max_ms={:.2} quads={} solid={} glow={} gas={} player_x={:.0} wasm_mem_mb={:.1}",
            stats.window_frames as f32 / span,
            span * 1000.0 / stats.window_frames as f32,
            stats.worst_ms,
            sim.sim_total_ms / stats.window_frames as f32,
            sim.ticks_run as f32 / stats.window_frames as f32,
            sim.sim_ms,
            stats.build_ms,
            stats.upload_ms,
            slab.slab.quads(),
            slab.slab.solid.quads(),
            slab.slab.glow.quads(),
            slab.slab.gas.quads(),
            sim.game.player.x,
            wasm_memory_mb(),
        );
        stats.window_start = Some(now);
        stats.window_frames = 0;
        stats.worst_ms = 0.0;
        stats.build_ms = 0.0;
        stats.upload_ms = 0.0;
        sim.sim_ms = 0.0;
        sim.sim_total_ms = 0.0;
        sim.ticks_run = 0;
    }
}
