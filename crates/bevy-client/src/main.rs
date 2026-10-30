//! Alchemist's Trial: first-person 3D prototype. Real chemistry is magic.
//!
//! One small courtyard. Walk with WASD, look with the mouse, and cast four
//! spells from a gauntlet: fire (combustion), water (cooling), frost (liquid
//! nitrogen) and foam (CO2). Objective: "Save the Greenhouse", put out three
//! fires before they spread to it.
//!
//! Modules: `chem` is the deterministic rules engine (no Bevy), `world`
//! builds the courtyard and its colliders, `player` is the kinematic
//! character (its integrator is `motion`), `clock` the game clock, `fx` the
//! particle pool, `bridge` the link to the page's HUD.

mod bridge;
mod chem;
mod clock;
mod feel;
mod fx;
mod motion;
mod player;
mod world;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::asset::AssetMetaCheck;
#[cfg(feature = "bloom")]
use bevy::camera::Hdr;
#[cfg(not(feature = "bloom"))]
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::{
    CascadeShadowConfigBuilder, GlobalAmbientLight, NotShadowCaster, NotShadowReceiver,
};
use bevy::log::LogPlugin;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::platform::time::Instant;
#[cfg(feature = "bloom")]
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::PrimaryWindow;
use bridge::WebIn;
use chem::{EventKind, Kind, Outcome, Spell};
use fx::{Fx, P};
use player::Player;
use world::{Chem, Colliders, Kit};

/// Counts live heap bytes and the largest single allocation, so STATS can
/// tell a leak (live bytes keep rising) from a one-off peak.
mod heap {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    pub static LIVE: AtomicUsize = AtomicUsize::new(0);
    pub static PEAK: AtomicUsize = AtomicUsize::new(0);
    pub static BIGGEST: AtomicUsize = AtomicUsize::new(0);
    pub struct Counting;
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            let n = LIVE.fetch_add(l.size(), Relaxed) + l.size();
            PEAK.fetch_max(n, Relaxed);
            BIGGEST.fetch_max(l.size(), Relaxed);
            unsafe { System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            LIVE.fetch_sub(l.size(), Relaxed);
            unsafe { System.dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
            if new > l.size() {
                let n = LIVE.fetch_add(new - l.size(), Relaxed) + new - l.size();
                PEAK.fetch_max(n, Relaxed);
                BIGGEST.fetch_max(new, Relaxed);
            } else {
                LIVE.fetch_sub(l.size() - new, Relaxed);
            }
            unsafe { System.realloc(p, l, new) }
        }
    }
    #[global_allocator]
    static A: Counting = Counting;
    pub fn mb(a: &AtomicUsize) -> f32 {
        a.load(Relaxed) as f32 / 1_048_576.0
    }
}

/// Settings from the URL (`?sens=1.5&assist=0&low`) or the command line.
/// The pause menu's settings (web/hud.js) change `sens`, `hfov`, `sway` and
/// `assist` at runtime with `set` commands.
#[derive(Resource, Debug, Clone)]
struct Settings {
    sens: f32,
    /// Aim-assist cone, radians (0 = off).
    assist: f32,
    /// Horizontal field of view at 16:9, degrees ("View width").
    hfov: f32,
    /// The gauntlet's walk bob ("Hand sway"). The camera never bobs.
    sway: bool,
    low: bool,
    /// Anti-aliasing: 4x MSAA and/or FXAA (`?aa=msaa|fxaa|both|off`).
    msaa: bool,
    fxaa: bool,
    /// `?debug`: enables the test commands (tp, tick, look, turn, face,
    /// move) on `window.__game.cmd`.
    debug: bool,
}

impl Settings {
    /// Vertical field of view, degrees.
    fn fov_y(&self) -> f32 {
        feel::vfov_from_h(self.hfov)
    }
}

/// The aim-assist cone when it is on.
const ASSIST: f32 = 4.0 * std::f32::consts::PI / 180.0;
/// Seconds between casts.
const COOLDOWN: f32 = 0.3;

#[derive(Clone, Copy, Debug, Default)]
struct Aim {
    point: Vec3,
    obj: Option<usize>,
    assisted: bool,
}

#[derive(Resource, Default)]
struct Game {
    spell: usize,
    cooldown: f32,
    caption: String,
    caption_id: u32,
    caption_age: f32,
    aim: Aim,
    hover: Option<usize>,
    recoil: f32,
    casts: u32,
    impacts: u32,
    paused: bool,
    locked: bool,
    events: Vec<chem::Event>,
    /// Holds the movement keys down from a test command: (right, fwd, secs).
    scripted_move: Option<(Vec2, f32)>,
    /// `cast`/`tap` commands waiting for the cooldown, with their age in
    /// seconds. Dropped after 0.3 s, and cleared by a reset (a new Game).
    cast_queue: Vec<(String, f32)>,
    /// Bumped by every reset: spells and beams from before it vanish
    /// instead of landing in the new courtyard.
    epoch: u32,
    /// What the last impact did (for the HUD's hit marker and sound), and
    /// where it landed on screen as canvas fractions (-1 when off screen).
    hit_kind: Option<chem::HitKind>,
    hit_x: f32,
    hit_y: f32,
    /// Camera and viewmodel translations this frame, for the tests.
    cam: Vec3,
    vm: Vec3,
    /// Game seconds played since the last reset: this frame's dt is added
    /// unless paused (`gameTime` in the state).
    game_time: f32,
    /// This frame's clamped dt (`clock::frame_dt`).
    dt: f32,
}

impl Game {
    fn say(&mut self, s: &str) {
        self.caption = s.to_string();
        self.caption_id += 1;
        self.caption_age = 0.0;
    }
}

#[derive(Component)]
struct FpsCam;
#[derive(Component)]
struct Viewmodel;
#[derive(Component)]
struct SpellGlow;
#[derive(Component)]
struct FireLight;

#[derive(Component)]
struct Projectile {
    spell: Spell,
    from: Vec3,
    to: Vec3,
    t: f32,
    dur: f32,
    obj: Option<usize>,
    epoch: u32,
}

#[derive(Component)]
struct Beam {
    life: f32,
    epoch: u32,
}

#[derive(Resource)]
struct SpellMats {
    glow: [Handle<StandardMaterial>; 4],
}

#[derive(Resource, Default)]
struct Stats {
    frames: u64,
    window_start: Option<Instant>,
    last: Option<Instant>,
    dts: Vec<f32>,
    cpu: Vec<f32>,
    frame_start: Option<Instant>,
}

fn main() {
    let settings = read_settings();
    let window = Window {
        title: "Alchemist's Trial: first-person prototype".into(),
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
                })
                // Bevy 0.20 always adds the SSAO, OIT, atmosphere and
                // background motion-vector plugins, and each warns at startup
                // that WebGL2 cannot run it. The game uses none of them, so
                // their logs are kept to errors only.
                .set(LogPlugin {
                    filter: format!(
                        "{},bevy_pbr::ssao=error,bevy_pbr::atmosphere=error,bevy_core_pipeline::oit=error,bevy_core_pipeline::prepass=error",
                        bevy::log::DEFAULT_FILTER
                    ),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.55, 0.78, 0.98)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.85, 0.9, 1.0),
            brightness: 700.0,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(Time::<Fixed>::from_seconds(chem::DT as f64))
        // One ceiling for the whole game: the fixed-step chemistry (driven by
        // the virtual clock) and every per-frame system (`clock::frame_dt`)
        // advance at most MAX_FRAME_DT of game time per frame.
        .insert_resource({
            let mut t = Time::<Virtual>::default();
            t.set_max_delta(std::time::Duration::from_secs_f32(clock::MAX_FRAME_DT));
            t
        })
        .insert_resource(settings)
        .init_resource::<Stats>()
        .init_resource::<Game>()
        .init_resource::<WebIn>()
        .init_resource::<Player>()
        .add_systems(Startup, setup)
        .add_systems(First, frame_start)
        .add_systems(FixedUpdate, chem_tick)
        .add_systems(
            Update,
            (
                control,
                aim_and_cast,
                projectiles,
                react,
                sync_visuals,
                viewmodel,
                fx::update_fx,
                publish,
            )
                .chain(),
        )
        .add_systems(Last, report)
        .run();
}

/// Query parameters on the web; `--key=value` arguments natively.
fn param(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    let q: String = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    #[cfg(not(target_arch = "wasm32"))]
    let q: String = std::env::args()
        .skip(1)
        .map(|a| a.trim_start_matches("--").to_string())
        .collect::<Vec<_>>()
        .join("&");
    q.trim_start_matches('?').split('&').find_map(|kv| {
        let mut it = kv.splitn(2, '=');
        (it.next()? == key).then(|| it.next().unwrap_or("").to_string())
    })
}

fn read_settings() -> Settings {
    // Provisional: the HUD sends the effective settings (URL, then saved,
    // then default) as soon as the first frame is published.
    let sens = param("sens")
        .and_then(|s| s.parse::<f32>().ok())
        .and_then(feel::clamp_sens)
        .unwrap_or(1.0);
    // Aim assist is on unless `?assist=0` (or `off`/`false`).
    let assist_on = !matches!(param("assist").as_deref(), Some("0" | "off" | "false"));
    let low = param("low").is_some_and(|v| !matches!(v.as_str(), "0" | "false"));
    // Full tier: 4x MSAA plus FXAA (FXAA also catches what MSAA misses, such
    // as edges inside the HDR and bloom path). Low tier: no anti-aliasing by
    // default (one full-screen pass fewer, I-015); `?low&aa=fxaa` turns FXAA
    // back on.
    let (msaa, fxaa) = match param("aa").as_deref() {
        Some("msaa") => (true, false),
        Some("fxaa") => (false, true),
        Some("both") => (true, true),
        Some("off" | "0") => (false, false),
        _ => (!low, !low),
    };
    Settings {
        sens,
        assist: if assist_on { ASSIST } else { 0.0 },
        hfov: feel::DEFAULT_HFOV,
        sway: true,
        low,
        msaa,
        fxaa,
        debug: param("debug").is_some_and(|v| !matches!(v.as_str(), "0" | "false")),
    }
}

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

fn spell_color(s: Spell) -> Color {
    match s {
        Spell::Fire => Color::srgb(1.0, 0.45, 0.1),
        Spell::Water => Color::srgb(0.2, 0.55, 1.0),
        Spell::Frost => Color::srgb(0.6, 0.95, 1.0),
        Spell::Foam => Color::srgb(0.92, 0.95, 1.0),
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<Settings>,
    mut game: ResMut<Game>,
) {
    info!("STARTUP t={:.0}ms", now_ms());
    let (kit, colliders, chem) =
        world::build(&mut commands, &mut meshes, &mut materials, &mut images);
    commands.insert_resource(colliders);
    commands.insert_resource(chem);
    fx::setup_fx(
        &mut commands,
        &mut meshes,
        &mut materials,
        if settings.low { 160 } else { 320 },
    );

    // Sun: warm, from the south-west, with one shadow cascade (full tier).
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            color: Color::srgb(1.0, 0.95, 0.85),
            shadow_maps_enabled: !settings.low,
            ..default()
        },
        Transform::from_xyz(-0.6, 1.0, 0.45).looking_at(Vec3::ZERO, Vec3::Y),
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            minimum_distance: 0.1,
            maximum_distance: 50.0,
            first_cascade_far_bound: 50.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
    if !settings.low {
        for _ in 0..3 {
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.55, 0.2),
                    intensity: 0.0,
                    range: 12.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::default(),
                FireLight,
            ));
        }
    }

    let glow: [Handle<StandardMaterial>; 4] = Spell::ALL.map(|s| {
        let l = spell_color(s).to_linear();
        materials.add(StandardMaterial {
            base_color: spell_color(s),
            emissive: LinearRgba::rgb(l.red * 5.0, l.green * 5.0, l.blue * 5.0),
            ..default()
        })
    });
    commands.insert_resource(SpellMats { glow: glow.clone() });

    let mut camera = commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: settings.fov_y().to_radians(),
            near: 0.05,
            far: 250.0,
            ..default()
        }),
        Transform::from_xyz(0.0, player::EYE, 18.0),
        DistanceFog {
            color: Color::srgb(0.7, 0.85, 1.0),
            directional_light_color: Color::NONE,
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Linear {
                start: 35.0,
                end: 110.0,
            },
        },
        FpsCam,
    ));
    #[cfg(feature = "bloom")]
    if !settings.low {
        camera.insert((Hdr, Bloom::NATURAL));
    }
    #[cfg(not(feature = "bloom"))]
    camera.insert(Tonemapping::Reinhard);
    camera.insert(if settings.msaa {
        Msaa::Sample4
    } else {
        Msaa::Off
    });
    if settings.fxaa {
        camera.insert(Fxaa::default());
    }

    // The gauntlet viewmodel: bronze forearm, cuff, hand and fingers, with a
    // glowing gem and rune bands in the selected spell's colour.
    let bronze = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.52, 0.28),
        metallic: 0.6,
        perceptual_roughness: 0.35,
        ..default()
    });
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.35),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });
    let cube = kit.cube.clone();
    let ball = kit.ball.clone();
    let vm = |t: Transform, m: &Handle<StandardMaterial>, mesh: &Handle<Mesh>| {
        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(m.clone()),
            t,
            NotShadowCaster,
            NotShadowReceiver,
        )
    };
    let cam_id = camera.id();
    let root = commands
        .spawn((
            Transform::from_xyz(0.21, -0.18, -0.42).with_scale(Vec3::splat(0.55)),
            Visibility::Inherited,
            Viewmodel,
        ))
        .with_children(|c| {
            let s = |x, y, z| Vec3::new(x, y, z);
            c.spawn(vm(
                Transform::from_xyz(0.0, 0.0, 0.14).with_scale(s(0.1, 0.1, 0.32)),
                &bronze,
                &cube,
            ));
            c.spawn(vm(
                Transform::from_xyz(0.0, 0.0, -0.03).with_scale(s(0.13, 0.13, 0.06)),
                &gold,
                &cube,
            ));
            c.spawn(vm(
                Transform::from_xyz(0.0, 0.01, -0.12).with_scale(s(0.11, 0.06, 0.12)),
                &bronze,
                &cube,
            ));
            for k in 0..4 {
                let x = -0.039 + k as f32 * 0.026;
                c.spawn(vm(
                    Transform::from_xyz(x, 0.005, -0.21)
                        .with_rotation(Quat::from_rotation_x(-0.35))
                        .with_scale(s(0.02, 0.025, 0.08)),
                    &gold,
                    &cube,
                ));
            }
            c.spawn((
                vm(
                    Transform::from_xyz(0.0, 0.05, -0.11).with_scale(Vec3::splat(0.05)),
                    &glow[0],
                    &ball,
                ),
                SpellGlow,
            ));
            for z in [0.06, 0.2] {
                c.spawn((
                    vm(
                        Transform::from_xyz(0.0, 0.052, z).with_scale(s(0.104, 0.01, 0.02)),
                        &glow[0],
                        &cube,
                    ),
                    SpellGlow,
                ));
            }
        })
        .id();
    commands.entity(cam_id).add_child(root);
    game.say("Save the Greenhouse! Put out 3 fires before they spread. Press 2 for Water.");
    info!(
        "tier={} sens={} cm360@800cpi={:.1} assist={} msaa={} fxaa={}",
        if settings.low { "low" } else { "full" },
        settings.sens,
        feel::cm_per_360(feel::LOOK_PER_PX, settings.sens, 800.0),
        settings.assist > 0.0,
        settings.msaa,
        settings.fxaa
    );
}

fn frame_start(mut stats: ResMut<Stats>) {
    stats.frame_start = Some(Instant::now());
}

fn chem_tick(mut chem: ResMut<Chem>, mut game: ResMut<Game>) {
    if game.paused || chem.world.outcome() != Outcome::Playing {
        return;
    }
    let mut ev = Vec::new();
    chem.world.step(&mut ev);
    game.events.extend(ev);
}

/// Reads the HUD bridge, keys and mouse, runs test commands, moves the player
/// and places the camera.
#[allow(clippy::too_many_arguments)]
fn control(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut settings: ResMut<Settings>,
    mut web: ResMut<WebIn>,
    mut game: ResMut<Game>,
    mut player: ResMut<Player>,
    mut chem: ResMut<Chem>,
    colliders: Res<Colliders>,
    mut cam: Query<(&mut Transform, &mut Projection), With<FpsCam>>,
    #[cfg(not(target_arch = "wasm32"))] mut cursor: Single<
        &mut bevy::window::CursorOptions,
        With<PrimaryWindow>,
    >,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    bridge::poll(&mut web);
    #[cfg(target_arch = "wasm32")]
    {
        game.locked = web.locked;
        game.paused = web.paused;
        let _ = (&motion, &mouse);
        player.look(web.dx, web.dy, settings.sens);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use bevy::window::CursorGrabMode;
        if mouse.just_pressed(MouseButton::Left) && !game.locked {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            game.locked = true;
        }
        if keys.just_pressed(KeyCode::Escape) {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
            game.locked = false;
        }
        if game.locked {
            player.look(motion.delta.x, motion.delta.y, settings.sens);
        }
    }
    // The game clock: frozen while paused, back to 0 on a reset (below).
    if !game.paused {
        game.game_time += dt;
    }

    // Spell selection: 1-4 or the wheel.
    for (k, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .into_iter()
    .enumerate()
    {
        if keys.just_pressed(key) {
            game.spell = k;
        }
    }
    // On the web the wheel arrives as a `select` command from hud.js.
    if cfg!(not(target_arch = "wasm32")) && game.locked && scroll.delta.y != 0.0 {
        let step = if scroll.delta.y < 0.0 { 1 } else { 3 };
        game.spell = (game.spell + step) % 4;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        reset(&mut chem, &mut game, &mut player);
    }

    let mut cmd_move = None;
    let cmds = std::mem::take(&mut web.cmds);
    for c in &cmds {
        let a: Vec<&str> = c.split_whitespace().collect();
        // Numbers only: NaN and infinities read as 0.
        let f = |i: usize| {
            a.get(i)
                .and_then(|s| s.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(0.0)
        };
        let name = a.first().copied();
        // Test commands that move the player or the clock need `?debug`.
        let debug_only = matches!(
            name,
            Some("look" | "turn" | "face" | "tp" | "move" | "tick")
        );
        if debug_only && !settings.debug {
            warn!("command {c:?} needs ?debug");
            continue;
        }
        match name {
            Some("select") => game.spell = (f(1) as usize).clamp(1, 4) - 1,
            Some("look") => {
                player.yaw = f(1).to_radians();
                player.pitch = f(2).clamp(-89.0, 89.0).to_radians();
            }
            Some("turn") => player.look(f(1), f(2), settings.sens),
            Some("face") => {
                let i = f(1) as usize;
                if i < chem.targets.len() {
                    let c = chem.bounds(i).center();
                    player.face(c);
                }
            }
            Some("tp") => {
                let edge = world::HALF - player::RADIUS;
                player.pos = Vec3::new(f(1).clamp(-edge, edge), 0.0, f(2).clamp(-edge, edge));
                player.vel = Vec3::ZERO;
            }
            Some("move") => {
                let w = Vec2::new(f(1), f(2)).clamp_length_max(1.0);
                cmd_move = Some((w, f(3).clamp(0.0, 10.0)));
            }
            Some("tick") => {
                // At most 20 000 steps (11 minutes of game time) per command.
                let mut ev = Vec::new();
                for _ in 0..f(1).clamp(0.0, 20_000.0) as usize {
                    if chem.world.outcome() == Outcome::Playing {
                        chem.world.step(&mut ev);
                    }
                }
                game.events.extend(ev);
            }
            Some("reset") => reset(&mut chem, &mut game, &mut player),
            // Settings from the pause menu (not debug-only).
            Some("set") => {
                let num = || a.get(2).and_then(|s| s.parse::<f32>().ok());
                let on = matches!(a.get(2).copied(), Some("1" | "on" | "true"));
                match a.get(1).copied() {
                    Some("sens") => {
                        if let Some(v) = num().and_then(feel::clamp_sens) {
                            settings.sens = v;
                        }
                    }
                    Some("hfov") => {
                        if let Some(v) = num().and_then(feel::clamp_hfov) {
                            settings.hfov = v;
                        }
                    }
                    Some("sway") => settings.sway = on,
                    Some("assist") => settings.assist = if on { ASSIST } else { 0.0 },
                    _ => warn!("{c:?} is not a known setting"),
                }
            }
            _ => {}
        }
    }
    web.cmds = cmds
        .into_iter()
        .filter(|c| c.starts_with("tap") || c == "cast")
        .collect();
    if let Some(m) = cmd_move {
        game.scripted_move = Some(m);
    }
    game.dt = dt;

    // Movement.
    let mut wish = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        wish.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        wish.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        wish.x += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        wish.x -= 1.0;
    }
    if let Some((w, secs)) = game.scripted_move {
        wish += w;
        game.scripted_move = (secs > dt).then_some((w, secs - dt));
    }
    if !game.paused {
        // Built once per frame; the player substeps against them.
        let ice: Vec<world::Aabb> = (0..chem.targets.len())
            .filter(|&i| {
                let o = &chem.world.objs[i];
                o.kind == Kind::Puddle && o.frozen()
            })
            .map(|i| chem.bounds(i))
            .collect();
        let ice = player::boxes(&ice);
        let solids = player::boxes(colliders.solid(&chem.world));
        player.step(
            player::MoveInput {
                wish,
                sprint: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
                jump: keys.pressed(KeyCode::Space),
            },
            dt,
            &solids,
            &ice,
        );
    }
    if let Ok((mut t, mut proj)) = cam.single_mut() {
        t.translation = player.eye();
        t.rotation = player.rotation();
        game.cam = t.translation;
        let fov = settings.fov_y().to_radians();
        if matches!(&*proj, Projection::Perspective(p) if (p.fov - fov).abs() > 1e-6) {
            if let Projection::Perspective(p) = &mut *proj {
                p.fov = fov;
            }
        }
    }
}

fn reset(chem: &mut Chem, game: &mut Game, player: &mut Player) {
    chem.world = chem.initial.clone();
    for t in &mut chem.targets {
        t.last = None;
    }
    *player = Player::default();
    let spell = game.spell;
    *game = Game {
        spell,
        locked: game.locked,
        paused: game.paused,
        epoch: game.epoch.wrapping_add(1),
        ..default()
    };
    game.say("Save the Greenhouse! Put out 3 fires before they spread.");
}

/// Best target along a ray: what it hits, or (with assist) the target
/// whose edge is within the assist cone of the ray and in plain view. The
/// cone is `assist` radians up to 20 m and narrows to nothing at 60 m
/// (`feel::assist_cone`). Assist bends only the cast, never the view.
fn pick(chem: &Chem, colliders: &Colliders, origin: Vec3, dir: Vec3, assist: f32) -> Aim {
    let (t, hit) = world::raycast(chem, colliders, origin, dir, 120.0);
    let point = origin + dir * t;
    if hit.is_some() || assist <= 0.0 {
        return Aim {
            point,
            obj: hit,
            assisted: false,
        };
    }
    let mut best: Option<(f32, usize, Vec3)> = None;
    for i in 0..chem.targets.len() {
        if chem.world.objs[i].kind == Kind::Greenhouse {
            continue;
        }
        let b = chem.bounds(i);
        let c = b.center();
        let v = c - origin;
        let dist = v.length();
        if dist < 0.5 || dist > 60.0 {
            continue;
        }
        let half = (b.max - b.min) * 0.5;
        let radius = half.x.min(half.z).max(half.y.min(half.x.max(half.z))) * 0.7;
        let ang = dir.angle_between(v / dist) - (radius / dist).min(1.0).asin();
        if ang > feel::assist_cone(assist, dist) || best.is_some_and(|(a, _, _)| a <= ang) {
            continue;
        }
        let (t2, h2) = world::raycast(chem, colliders, origin, v / dist, dist + 1.0);
        if h2 == Some(i) || t2 >= dist - radius {
            best = Some((ang, i, c));
        }
    }
    match best {
        Some((_, i, c)) => Aim {
            point: c,
            obj: Some(i),
            assisted: true,
        },
        None => Aim {
            point,
            obj: None,
            assisted: false,
        },
    }
}

/// Aims, and casts on click (pointer locked) or tap (pointer free).
#[allow(clippy::too_many_arguments)]
fn aim_and_cast(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    mut web: ResMut<WebIn>,
    mut game: ResMut<Game>,
    player: Res<Player>,
    chem: Res<Chem>,
    colliders: Res<Colliders>,
    kit: Res<Kit>,
    mats: Res<SpellMats>,
    window: Single<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<FpsCam>>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    game.cooldown -= dt;
    game.caption_age += dt;
    game.recoil = (game.recoil - dt * 6.0).max(0.0);
    let eye = player.eye();
    game.aim = pick(&chem, &colliders, eye, player.forward(), settings.assist);

    // Pointer free: what is under the cursor (kids can just click things).
    // The cursor arrives as a fraction of the canvas (0..1 each way), and
    // the ray is built from the player's current view, not the camera's
    // transform from the last frame or the canvas's pixel size (which mixes
    // logical, physical and render-scaled pixels).
    let aspect = cam
        .single()
        .ok()
        .and_then(|(c, _)| c.logical_viewport_size())
        .map_or(window.width() / window.height().max(1.0), |s| {
            s.x / s.y.max(1.0)
        });
    let rot = player.rotation();
    let fov_y = settings.fov_y();
    let ray_at = |f: Vec2| -> Option<Vec3> {
        feel::ray_dir(f.x, f.y, fov_y, aspect).map(|v| rot * Vec3::from_array(v))
    };
    let tap_assist = settings.assist.max(3f32.to_radians()) * 1.5;
    game.hover = None;
    if !game.locked {
        #[cfg(target_arch = "wasm32")]
        let cursor = web.pointer.map(|(x, y)| Vec2::new(x, y));
        #[cfg(not(target_arch = "wasm32"))]
        let cursor = window
            .cursor_position()
            .map(|p| p / Vec2::new(window.width(), window.height()).max(Vec2::ONE));
        if let Some(dir) = cursor.and_then(ray_at) {
            game.hover = pick(&chem, &colliders, eye, dir, tap_assist).obj;
        }
    }

    let mut casts: Vec<Aim> = Vec::new();
    // `cast` and `tap` obey the same cooldown and pause as the mouse. While
    // paused they are dropped; during the cooldown they wait at most 0.3 s
    // (so a stale click never fires later), and one runs per cooldown (so
    // tap spam cannot fire an unlimited stream of spells).
    let incoming = std::mem::take(&mut web.cmds);
    if game.paused {
        game.cast_queue.clear();
    } else {
        game.cast_queue
            .extend(incoming.into_iter().map(|c| (c, 0.0)));
        for q in &mut game.cast_queue {
            q.1 += dt;
        }
        game.cast_queue.retain(|q| q.1 <= 0.3 + dt);
        game.cast_queue.truncate(8);
    }
    let cmds: Vec<String> = if game.cooldown <= 0.0 && !game.cast_queue.is_empty() {
        vec![game.cast_queue.remove(0).0]
    } else {
        Vec::new()
    };
    for c in cmds {
        let a: Vec<&str> = c.split_whitespace().collect();
        if a.first() == Some(&"cast") {
            casts.push(game.aim);
        } else if a.first() == Some(&"tap") && a.len() == 3 {
            let p = Vec2::new(a[1].parse().unwrap_or(0.0), a[2].parse().unwrap_or(0.0));
            if let Some(dir) = ray_at(p) {
                casts.push(pick(&chem, &colliders, eye, dir, tap_assist));
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let firing = {
        let _ = &mouse;
        web.fire
    };
    #[cfg(not(target_arch = "wasm32"))]
    let firing = mouse.pressed(MouseButton::Left);
    if casts.is_empty() && game.locked && !game.paused && firing && game.cooldown <= 0.0 {
        casts.push(game.aim);
    }
    let ended = chem.world.outcome() != Outcome::Playing;
    for aim in casts {
        if ended {
            break;
        }
        let spell = Spell::ALL[game.spell];
        // If the cooldown ran out during this frame, the part of the frame
        // past zero counts toward the next one, so held fire keeps one cast
        // per 0.3 s of game time at any frame rate (at 4 fps, a cast every
        // 0.25 or 0.5 s rather than every 0.5 s). A cooldown that ran out on
        // an earlier frame carries nothing.
        let carry = if game.cooldown + dt > 0.0 {
            game.cooldown.min(0.0)
        } else {
            0.0
        };
        game.cooldown = COOLDOWN + carry;
        game.recoil = 1.0;
        game.casts += 1;
        game.say(spell.cast_caption());
        let hand = eye + player.rotation() * Vec3::new(0.2, -0.15, -0.5);
        let dist = hand.distance(aim.point);
        if spell == Spell::Frost {
            // A beam of liquid nitrogen: lands almost at once.
            let mid = (hand + aim.point) * 0.5;
            commands.spawn((
                Mesh3d(kit.cube.clone()),
                MeshMaterial3d(mats.glow[2].clone()),
                Transform::from_translation(mid)
                    .looking_at(aim.point, Vec3::Y)
                    .with_scale(Vec3::new(0.07, 0.07, dist)),
                NotShadowCaster,
                Beam {
                    life: 0.25,
                    epoch: game.epoch,
                },
            ));
        }
        let (speed, size) = match spell {
            Spell::Fire => (32.0, 0.32),
            Spell::Water => (30.0, 0.26),
            Spell::Frost => (120.0, 0.12),
            Spell::Foam => (20.0, 0.4),
        };
        commands.spawn((
            Mesh3d(kit.ball.clone()),
            MeshMaterial3d(mats.glow[game.spell].clone()),
            Transform::from_translation(hand).with_scale(Vec3::splat(size)),
            NotShadowCaster,
            Projectile {
                spell,
                from: hand,
                to: aim.point,
                t: 0.0,
                dur: (dist / speed).max(0.03),
                obj: aim.obj,
                epoch: game.epoch,
            },
        ));
    }
}

/// Moves projectiles and beams; applies spells on arrival.
#[allow(clippy::too_many_arguments)]
fn projectiles(
    mut commands: Commands,
    time: Res<Time>,
    player: Res<Player>,
    mut game: ResMut<Game>,
    mut chem: ResMut<Chem>,
    mut fx: ResMut<Fx>,
    cam: Query<&Camera, With<FpsCam>>,
    mut q: Query<(Entity, &mut Projectile, &mut Transform), Without<Beam>>,
    mut beams: Query<(Entity, &mut Beam, &mut Transform), Without<Projectile>>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    if fx.epoch != game.epoch {
        // A reset: clear the old courtyard's smoke and sparks too.
        fx.epoch = game.epoch;
        fx.clear = true;
    }
    for (e, mut b, mut t) in &mut beams {
        b.life -= dt;
        t.scale.x *= 0.85;
        t.scale.y *= 0.85;
        if b.life <= 0.0 || b.epoch != game.epoch {
            commands.entity(e).despawn();
        }
    }
    if game.paused {
        return;
    }
    for (e, mut p, mut t) in &mut q {
        if p.epoch != game.epoch {
            // Cast before a reset: it never lands.
            commands.entity(e).despawn();
            continue;
        }
        p.t += dt;
        let a = (p.t / p.dur).min(1.0);
        let pos =
            p.from.lerp(p.to, a) + Vec3::Y * (a * (1.0 - a) * 0.6 * p.from.distance(p.to) * 0.05);
        t.translation = pos;
        // A short trail.
        let trail = match p.spell {
            Spell::Fire => P::Fire,
            Spell::Water => P::Water,
            Spell::Frost => P::Frost,
            Spell::Foam => P::Foam,
        };
        if fx.rand() < 0.7 {
            let v = fx.rand3() * 0.4;
            fx.emit(trail, pos, v, 0.12, 0.35);
        }
        if a < 1.0 {
            continue;
        }
        commands.entity(e).despawn();
        game.impacts += 1;
        // Where it landed on screen, from this frame's view (the camera's
        // GlobalTransform is a frame old until transforms propagate).
        let view = GlobalTransform::from(
            Transform::from_translation(player.eye()).with_rotation(player.rotation()),
        );
        (game.hit_x, game.hit_y) = cam
            .single()
            .ok()
            .and_then(|c| {
                let size = c.logical_viewport_size()?;
                let px = c.world_to_viewport(&view, p.to).ok()?;
                let f = px / size.max(Vec2::ONE);
                ((0.0..=1.0).contains(&f.x) && (0.0..=1.0).contains(&f.y)).then_some((f.x, f.y))
            })
            .unwrap_or((-1.0, -1.0));
        game.hit_kind = Some(chem::HitKind::Miss);
        let burst = match p.spell {
            Spell::Fire => P::Fire,
            Spell::Water => P::Water,
            Spell::Frost => P::Frost,
            Spell::Foam => P::Foam,
        };
        fx.burst(burst, p.to, 14, 3.0, 0.22);
        if p.spell == Spell::Fire {
            fx.burst(P::Spark, p.to, 6, 5.0, 0.07);
        }
        if let Some(i) = p.obj {
            let mut ev = Vec::new();
            chem.world.apply(i, p.spell, &mut ev);
            game.hit_kind = Some(chem::hit_kind(&ev));
            // Spell results always get a caption.
            if let Some(last) = ev.last() {
                game.say(last.kind.caption());
            }
            for e in &ev {
                info!("IMPACT obj={} {:?} -> {:?}", e.obj, p.spell, e.kind);
            }
            game.events.extend(ev.into_iter().map(|mut e| {
                // Mark as already captioned.
                e.obj |= 1 << 30;
                e
            }));
        }
    }
}

/// Turns chemistry events into particles and captions.
fn react(mut game: ResMut<Game>, chem: Res<Chem>, mut fx: ResMut<Fx>) {
    let events = std::mem::take(&mut game.events);
    for e in events {
        let captioned = e.obj & (1 << 30) != 0;
        let i = e.obj & !(1 << 30);
        let c = chem.bounds(i).center();
        let top = Vec3::new(c.x, chem.bounds(i).max.y, c.z);
        match e.kind {
            EventKind::Extinguished(_) => {
                fx.burst(P::Steam, top, 18, 2.0, 0.5);
                fx.burst(P::Smoke, top, 8, 1.5, 0.4);
            }
            EventKind::Melted => {
                fx.burst(P::Water, top, 16, 3.0, 0.2);
                fx.burst(P::Steam, top, 10, 1.5, 0.5);
            }
            EventKind::Froze | EventKind::Chilled => fx.burst(P::Frost, top, 20, 2.0, 0.4),
            EventKind::Steam | EventKind::Fizzle(chem::Fizzle::Wet) => {
                fx.burst(P::Steam, top, 14, 1.5, 0.5)
            }
            EventKind::Ignited => fx.burst(P::Fire, top, 16, 2.5, 0.35),
            EventKind::BurntOut => fx.burst(P::Smoke, top, 14, 1.2, 0.5),
            _ => {}
        }
        if e.kind == EventKind::Reseeded {
            // Always said: the player must know where the new fire is.
            game.say(e.kind.caption());
            continue;
        }
        if !captioned
            && matches!(
                e.kind,
                EventKind::Ignited | EventKind::Melted | EventKind::BurntOut
            )
            && game.caption_age > 1.5
        {
            let who = chem.targets[i].name;
            let line = match e.kind {
                EventKind::Ignited if who == "greenhouse" => {
                    "Oh no! Heat from the fire reached the greenhouse. Press R to try again.".to_string()
                }
                EventKind::Ignited => format!(
                    "Heat from a neighbour raised the {who} to its ignition temperature: it caught fire!"
                ),
                k => k.caption().to_string(),
            };
            game.say(&line);
        }
    }
    match chem.world.outcome() {
        Outcome::Won if !game.caption.starts_with("You saved") => {
            game.say(
                "You saved the greenhouse! Water, frost and CO₂ each stop fire in their own way.",
            );
        }
        _ => {}
    }
}

/// Parts of a target's look: placement, visibility and (if any) material.
type SceneParts = (
    &'static mut Transform,
    &'static mut Visibility,
    Option<&'static mut MeshMaterial3d<StandardMaterial>>,
);

/// Shows each object's state: materials, flames, foam, ice, emitters, lights.
#[allow(clippy::too_many_arguments)]
fn sync_visuals(
    time: Res<Time>,
    settings: Res<Settings>,
    player: Res<Player>,
    game: Res<Game>,
    kit: Res<Kit>,
    mut chem: ResMut<Chem>,
    mut fx: ResMut<Fx>,
    mut q: Query<SceneParts, (Without<FireLight>, Without<fx::Particle>)>,
    mut lights: Query<(&mut PointLight, &mut Transform), With<FireLight>>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    let now = time.elapsed_secs();
    let rate = if settings.low { 0.45 } else { 1.0 } * if game.paused { 0.0 } else { 1.0 };
    let Chem { world, targets, .. } = &mut *chem;
    let mut fires: Vec<(f32, Vec3)> = Vec::new();
    for (i, tgt) in targets.iter_mut().enumerate() {
        let o = &world.objs[i];
        let look = world::look_of(o);
        if tgt.last != Some(look) {
            tgt.last = Some(look);
            for &e in &tgt.bodies {
                if let Ok((_, _, Some(mut m))) = q.get_mut(e) {
                    m.0 = tgt.mats.get(look);
                }
            }
        }
        for (k, &e) in tgt.flames.iter().enumerate() {
            if let Ok((mut t, mut v, _)) = q.get_mut(e) {
                *v = if o.burning {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                if o.burning {
                    let f = now * 11.0 + i as f32 * 1.7 + k as f32 * 2.3;
                    let base = if k == 0 { 1.0 } else { 0.7 };
                    let w = base * (1.0 + 0.12 * f.sin());
                    let h = base * 1.6 * (1.0 + 0.2 * (f * 1.3).cos());
                    let sz = match o.kind {
                        Kind::Greenhouse => 1.3,
                        Kind::DryHedge => 1.1,
                        _ => 1.25,
                    };
                    t.scale = Vec3::new(w * sz, h * sz, w * sz);
                }
            }
        }
        if let Some(e) = tgt.foam {
            if let Ok((mut t, mut v, _)) = q.get_mut(e) {
                *v = if o.smothered > 0.0 {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                let k = (o.smothered / 4.0).clamp(0.2, 1.0);
                let base = if o.kind == Kind::DryHedge {
                    Vec3::new(1.8, 1.6, 12.4)
                } else {
                    Vec3::new(2.1, 1.5, 2.1)
                };
                t.scale = base * Vec3::new(k.sqrt(), k, k.sqrt());
            }
        }
        if let Some(e) = tgt.wall {
            if let Ok((mut t, mut v, _)) = q.get_mut(e) {
                *v = if o.wall {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                let h = 3.2 * (0.2 + 0.8 * o.ice);
                t.scale.y = h;
                t.translation.y = h / 2.0;
            }
        }
        if let Some(e) = tgt.puddle {
            if let Ok((_, mut v, Some(mut m))) = q.get_mut(e) {
                *v = if o.wall {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
                let want = if o.frozen() { &kit.ice } else { &kit.water };
                if m.0 != *want {
                    m.0 = want.clone();
                }
            }
        }

        // Emitters.
        let bb = if tgt.melted_aabb.is_some() && !o.wall {
            tgt.melted_aabb.unwrap_or(tgt.aabb)
        } else {
            tgt.aabb
        };
        let c = bb.center();
        let half = (bb.max - bb.min) * 0.5;
        let spot = |fx: &mut Fx| {
            Vec3::new(
                c.x + (fx.rand() - 0.5) * 1.6 * half.x,
                bb.max.y - 0.2,
                c.z + (fx.rand() - 0.5) * 1.6 * half.z,
            )
        };
        let area = (half.x * half.z).sqrt().clamp(1.0, 3.0);
        if o.burning {
            fires.push((player.pos.distance(c), Vec3::new(c.x, bb.max.y + 0.8, c.z)));
            if fx.rand() < 30.0 * area * rate * dt {
                let p = spot(&mut fx);
                let v = Vec3::new(0.0, 1.5 + fx.rand() * 1.5, 0.0) + fx.rand3() * 0.3;
                fx.emit(P::Fire, p, v, 0.35, 0.8);
            }
            if fx.rand() < 5.0 * rate * dt {
                let p = spot(&mut fx) + Vec3::Y * 1.2;
                fx.emit(P::Smoke, p, Vec3::Y * 1.5, 0.5, 2.0);
            }
        }
        let steaming = (o.kind == Kind::SteamVent && o.temp > 60.0)
            || (o.wet > 0.0 && o.temp >= 99.0 && !o.burning);
        if steaming && fx.rand() < 14.0 * rate * dt {
            let p = spot(&mut fx);
            let v = Vec3::new(0.0, 2.5, 0.0) + fx.rand3() * 0.4;
            fx.emit(P::Steam, p, v, 0.5, 1.6);
        }
        if (o.temp < -60.0 || (o.frozen() && o.temp < -100.0)) && fx.rand() < 8.0 * rate * dt {
            let p = spot(&mut fx);
            let v = fx.rand3() * 0.3 + Vec3::Y * 0.2;
            fx.emit(P::Frost, p, v, 0.35, 1.4);
        }
        if o.smothered > 0.0 && fx.rand() < 3.0 * rate * dt {
            let p = spot(&mut fx) + Vec3::Y * 0.3;
            fx.emit(P::Foam, p, Vec3::Y * 0.6, 0.15, 0.8);
        }
    }
    fires.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (k, (mut l, mut t)) in lights.iter_mut().enumerate() {
        if let Some(&(_, p)) = fires.get(k) {
            l.intensity = 120_000.0 * (1.0 + 0.15 * (now * 13.0 + k as f32).sin());
            t.translation = p;
        } else {
            l.intensity = 0.0;
        }
    }
}

/// The gauntlet: spell colour, recoil kick and a little walk bob ("Hand
/// sway", off in the settings: then the bob is exactly zero). The recoil
/// kick stays: it is the response to the player's own click.
fn viewmodel(
    mut game: ResMut<Game>,
    settings: Res<Settings>,
    player: Res<Player>,
    mats: Res<SpellMats>,
    mut root: Query<&mut Transform, With<Viewmodel>>,
    mut glows: Query<&mut MeshMaterial3d<StandardMaterial>, With<SpellGlow>>,
) {
    for mut m in &mut glows {
        if m.0 != mats.glow[game.spell] {
            m.0 = mats.glow[game.spell].clone();
        }
    }
    if let Ok(mut t) = root.single_mut() {
        let r = game.recoil * game.recoil;
        let speed = player.vel.xz().length().min(9.0) / 9.0;
        let bob = player.stride * 1.6;
        let (bx, by) = if settings.sway {
            (
                bob.sin() * 0.01 * speed,
                (bob * 2.0).cos().abs() * -0.008 * speed,
            )
        } else {
            (0.0, 0.0)
        };
        t.translation = Vec3::new(0.21 + bx, -0.18 + by + r * 0.015, -0.42 + r * 0.05);
        t.rotation = Quat::from_rotation_x(r * 0.35) * Quat::from_rotation_y(0.1);
        game.vm = t.translation;
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Writes the state the HUD and tests read.
fn publish(
    mut frame: Local<u64>,
    mut last_hit: Local<(u32, u64)>,
    game: Res<Game>,
    chem: Res<Chem>,
    player: Res<Player>,
    settings: Res<Settings>,
) {
    *frame += 1;
    // The frame in which the latest impact landed.
    if last_hit.0 != game.impacts {
        *last_hit = (game.impacts, *frame);
    }
    let w = &chem.world;
    let outcome = match w.outcome() {
        Outcome::Playing => "playing",
        Outcome::Won => "won",
        Outcome::Lost => "lost",
    };
    let objs: Vec<String> = w
        .objs
        .iter()
        .enumerate()
        .map(|(i, o)| {
            format!(
                "{{\"i\":{i},\"n\":\"{}\",\"burning\":{},\"temp\":{:.0},\"wet\":{:.2},\"ice\":{:.2},\"wall\":{},\"frozen\":{},\"smothered\":{:.1},\"fuel\":{:.1}}}",
                chem.targets[i].name, o.burning, o.temp, o.wet, o.ice, o.wall, o.frozen(), o.smothered, o.fuel
            )
        })
        .collect();
    let opt = |o: Option<usize>| o.map_or(-1, |v| v as i64);
    let risk = w.greenhouse_risk().unwrap_or((chem::AMBIENT_C, false));
    let json = format!(
        "{{\"frame\":{},\"tick\":{},\"locked\":{},\"paused\":{},\"spell\":{},\"spellName\":\"{}\",\"formula\":\"{}\",\"aimObj\":{},\"assisted\":{},\"hover\":{},\"extinguished\":{},\"goal\":{},\"burning\":{},\"ghTemp\":{:.0},\"ghDanger\":{},\"outcome\":\"{outcome}\",\"caption\":\"{}\",\"captionId\":{},\"casts\":{},\"impacts\":{},\"pos\":[{:.3},{:.3},{:.3}],\"yaw\":{:.2},\"pitch\":{:.2},\"onIce\":{},\"low\":{},\"sens\":{},\"assist\":{},\"fov\":{:.2},\"sway\":{},\"cam\":[{:.3},{:.3},{:.3}],\"vm\":[{:.4},{:.4},{:.4}],\"cooldown\":{:.3},\"cooldownMax\":{},\"hitKind\":\"{}\",\"hitX\":{:.4},\"hitY\":{:.4},\"hitFrame\":{},\"objs\":[{}],\"gameTime\":{:.3},\"dt\":{:.4}}}",
        *frame,
        w.ticks,
        game.locked,
        game.paused,
        game.spell,
        Spell::ALL[game.spell].name(),
        Spell::ALL[game.spell].formula(),
        opt(game.aim.obj),
        game.aim.assisted,
        opt(game.hover),
        w.extinguished,
        chem::GOAL_FIRES,
        w.burning(),
        risk.0,
        risk.1,
        esc(&game.caption),
        game.caption_id,
        game.casts,
        game.impacts,
        player.pos.x,
        player.pos.y,
        player.pos.z,
        player.yaw.to_degrees(),
        player.pitch.to_degrees(),
        player.on_ice,
        settings.low,
        settings.sens,
        settings.assist > 0.0,
        settings.fov_y(),
        settings.sway,
        game.cam.x,
        game.cam.y,
        game.cam.z,
        game.vm.x,
        game.vm.y,
        game.vm.z,
        game.cooldown.max(0.0),
        COOLDOWN,
        game.hit_kind.map_or("none", chem::HitKind::name),
        game.hit_x,
        game.hit_y,
        last_hit.1,
        objs.join(","),
        game.game_time,
        game.dt
    );
    bridge::publish(&json);
}

fn pct(v: &mut [f32], p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    v[((v.len() - 1) as f32 * p).round() as usize]
}

/// Logs a STATS line every 2 seconds.
fn report(
    mut stats: ResMut<Stats>,
    entities: Query<Entity>,
    chem: Res<Chem>,
    game: Res<Game>,
    raw: Res<Messages<bevy::winit::RawWinitWindowEvent>>,
) {
    let now = Instant::now();
    stats.frames += 1;
    if stats.frames == 3 {
        info!("FIRST_FRAME t={:.0}ms", now_ms());
    }
    if let Some(s) = stats.frame_start {
        let cpu = (now - s).as_secs_f32() * 1000.0;
        stats.cpu.push(cpu);
    }
    if let Some(last) = stats.last {
        let dt = (now - last).as_secs_f32() * 1000.0;
        stats.dts.push(dt);
    }
    stats.last = Some(now);
    let start = *stats.window_start.get_or_insert(now);
    let span = (now - start).as_secs_f32();
    if span >= 2.0 && !stats.dts.is_empty() {
        let n = stats.dts.len();
        let mut dts = std::mem::take(&mut stats.dts);
        let mut cpu = std::mem::take(&mut stats.cpu);
        let avg = dts.iter().sum::<f32>() / n as f32;
        let p50 = pct(&mut dts, 0.5);
        let p95 = pct(&mut dts, 0.95);
        let worst = pct(&mut dts, 1.0);
        let cpu50 = pct(&mut cpu, 0.5);
        let cpu95 = pct(&mut cpu, 0.95);
        info!(
            "STATS fps={:.1} fps_p50={:.1} fps_p5={:.1} avg_ms={:.2} p50_ms={:.2} p95_ms={:.2} worst_ms={:.1} update_ms_p50={:.2} update_ms_p95={:.2} entities={} burning={} extinguished={} wasm_mem_mb={:.1} heap_live_mb={:.1} heap_peak_mb={:.1} biggest_alloc_mb={:.1} winit_events={}",
            n as f32 / span,
            1000.0 / p50.max(0.01),
            1000.0 / p95.max(0.01),
            avg,
            p50,
            p95,
            worst,
            cpu50,
            cpu95,
            entities.iter().count(),
            chem.world.burning(),
            chem.world.extinguished,
            wasm_memory_mb(),
            heap::mb(&heap::LIVE),
            heap::mb(&heap::PEAK),
            heap::mb(&heap::BIGGEST),
            raw.len(),
        );
        let _ = &game;

        stats.window_start = Some(now);
    }
}
