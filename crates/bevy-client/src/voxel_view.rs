//! The block world's view (I-052, Lane A): chunk entities and the
//! time-sliced mesh queue, the generated texture, break and place with the
//! Pick, the 1-9 hotbar, walking on blocks, and `?biome=` spawns.
//! Registered only in `Mode::Voxel`.
//!
//! Shared state other lanes use:
//! - `VoxWorld`: the block world. Lane B's matter step reads and writes it
//!   through `realgame_voxel::World::{get, set}`; every `set` marks chunks
//!   dirty and this view remeshes them (the contract rule).
//! - `crate::Pack`: gathering adds to it and placing takes from it; Lane C's
//!   story owns the pack UI, crafting and the `pack` state key.
//! - `crate::PlayerEvent`: `Dug`, `Placed`, `EnteredBiome` and `InBox` are
//!   sent from here.

use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::time::Instant;
use bevy::prelude::*;
use realgame_sim::valley::Item;
use realgame_voxel::{self as vox, materials, smooth, Biome, Cell, RayMode, World, CHUNK};

use crate::motion::{self, Body, Box3};
use crate::player::{MoveInput, Player};
use crate::{bridge::WebIn, clock, Game, PlayerEvent, Settings};

/// This plugin's values for the voxel-mode state keys (spec section 8);
/// `publish` formats them. Values, not JSON fragments (except `probe`, a
/// debug answer that `publish` copies as is).
#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `biome`: the id of the biome the player is in.
    pub biome: &'static str,
    /// `hot`: the selected hotbar slot, 1-9.
    pub hot: u8,
    /// `lookCell`: `[x, y, z, mat]` under the crosshair, or none.
    pub look_cell: Option<[i32; 4]>,
    /// `digT`: dig progress 0..1.
    pub dig_t: f32,
    /// `chunksMeshed`: chunks with a current, non-empty mesh.
    pub chunks_meshed: u32,
    /// `meshQ`: chunks waiting in the mesh queue.
    pub mesh_q: u32,
    /// `probe` (debug): the last `probe x y z` answer as JSON, with a
    /// sequence number `n`.
    pub probe: Option<String>,
    /// CPU milliseconds spent meshing this frame (logged for E8).
    pub mesh_ms: f32,
}

/// The block world (spec section 5).
#[derive(Resource)]
pub struct VoxWorld(pub World);

/// The smooth look's three materials (opaque, clear, water), for `mood`.
#[derive(Resource)]
pub struct SmoothHandles(pub Vec<Handle<crate::voxel_tex::SmoothMat>>);

/// How far the Pick and placing reach, metres from the eye.
pub const REACH: f32 = 5.0;
/// Said when a bound item cannot be placed as a block.
pub const UNPLACEABLE_LINE: &str = "That can't be placed. Use it at the Invention Bench.";
/// How far a spell's aim looks for a block.
pub const SPELL_RANGE: f32 = 120.0;
/// The pause after a block breaks, while `digT` shows 1 (so a held button
/// does not tunnel straight on into the next block).
const DIG_REST: f32 = 0.25;
/// Walking speeds (the same as `player.rs`).
const WALK: f32 = 5.0;
const SPRINT: f32 = 8.5;
/// The most the body steps up without a jump (`tuning::STEP_UP`).
const STEP: f32 = crate::tuning::STEP_UP;
/// The first caption in the Village (said again when the intro closes).
pub const WELCOME_LINE: &str =
    "Welcome to the valley! Walk with WASD. Press 5 for the Pick and hold the mouse button to dig.";
/// The caption for a protected block (spec section 4).
pub const PROTECTED_LINE: &str = "This old runestone will not budge. Try chemistry.";

/// Chunk meshes: the opaque and clear entities of each chunk, the mesh
/// queue, and the materials.
#[derive(Resource)]
struct Chunks {
    /// Each chunk's pass entities: opaque, clear, (smooth) water, and the
    /// props near and far (`PROP_LOD_M`).
    ents: Vec<[Option<Entity>; 5]>,
    /// Each chunk's props level of detail as last set (`Some(true)` far),
    /// `None` when a props entity is new and needs setting.
    far: Vec<Option<bool>>,
    /// Waiting to be meshed (index order; sorted by distance when picked).
    queue: Vec<usize>,
    queued: Vec<bool>,
    opaque: Handle<crate::voxel_tex::VoxMat>,
    clear: Handle<crate::voxel_tex::VoxMat>,
    /// The smooth look's materials; `None` with `?blocks` (the block
    /// atlas look, kept for comparison).
    smooth: Option<crate::voxel_tex::SmoothMats>,
    /// The spawn ring (3 x 2 x 3 chunks around the start) is meshed.
    ring_done: bool,
}

/// What the Pick is doing.
#[derive(Resource, Default)]
struct Dig {
    /// The block being dug, and whether it continues without a held button
    /// (a click without pointer lock, or a tap).
    cell: Option<[i32; 3]>,
    auto: bool,
    t: f32,
    /// Seconds left of the pause after a break.
    rest: f32,
    /// The protected caption was said for this press.
    told: bool,
}

/// Walking state: the camera's step-up smoothing.
#[derive(Resource, Default)]
pub struct Walk {
    /// Metres the camera sits below the eye after a step up; eases to 0.
    pub lag: f32,
    /// Walk on the blocks themselves (`?blocks`), not the smooth surface.
    pub blocks: bool,
}

/// Where the player was, for biome and stamp entry events.
#[derive(Resource, Default)]
struct Whereabouts {
    biome: Option<Biome>,
    stamps: Vec<&'static str>,
}

/// Registers the view (Voxel mode only).
pub fn plugin(app: &mut App) {
    let biome = app
        .world()
        .get_resource::<crate::StartBiome>()
        .map_or(Biome::Village, |b| b.0);
    let seed = crate::param("seed")
        .filter(|_| crate::param("debug").is_some())
        .and_then(|s| parse_seed(&s))
        .unwrap_or(vox::DEFAULT_SEED);
    crate::voxel_tex::register(app);
    crate::sky::plugin(app);
    let t = Instant::now();
    let world = vox::generate(seed);
    info!(
        "VOXEL generate seed={seed:#x} {:.0} ms",
        t.elapsed().as_secs_f32() * 1000.0
    );
    app.insert_resource(Keys {
        biome: biome.id(),
        hot: 2,
        ..default()
    })
    .insert_resource(VoxWorld(world))
    .init_resource::<crate::Pack>()
    .init_resource::<Dig>()
    .insert_resource(Walk {
        blocks: crate::param("blocks").is_some(),
        ..default()
    })
    .init_resource::<Whereabouts>()
    .add_systems(Startup, setup.after(crate::setup))
    .add_systems(PostStartup, hide_courtyard)
    .add_systems(
        Update,
        (tools, mesh_chunks, prop_lod)
            .chain()
            .after(crate::control)
            .before(crate::aim_and_cast),
    );
}

fn parse_seed(s: &str) -> Option<u64> {
    match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<crate::voxel_tex::VoxMat>>,
    mut smooth_mats: ResMut<Assets<crate::voxel_tex::SmoothMat>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_mats: ResMut<Assets<StandardMaterial>>,
    world: Res<VoxWorld>,
    start: Res<crate::StartBiome>,
    mut player: ResMut<Player>,
    mut game: ResMut<Game>,
    settings: Res<Settings>,
) {
    let m = crate::voxel_tex::materials(&mut images, &mut mats, world.0.seed);
    let n = world.0.chunk_count();
    let smooth = crate::param("blocks")
        .is_none()
        .then(|| crate::voxel_tex::smooth_materials(&mut smooth_mats, settings.low));
    if let Some(s) = smooth.as_ref() {
        commands.insert_resource(SmoothHandles(vec![
            s.opaque.clone(),
            s.clear.clone(),
            s.water.clone(),
        ]));
    }
    if smooth.is_some() {
        let opaque = smooth.as_ref().map(|m| m.opaque.clone());
        if let Some(rock) = opaque.clone() {
            // The far mountains around the valley (outside the cells).
            commands.spawn((
                Mesh3d(meshes.add(to_smooth_mesh(&vox::props::backdrop(world.0.seed), false))),
                MeshMaterial3d(rock),
                Transform::IDENTITY,
                NotShadowCaster,
            ));
        }
        spawn_props(&mut commands, &mut meshes, &mut std_mats, opaque);
    }
    let mut chunks = Chunks {
        ents: vec![[None; 5]; n],
        far: vec![None; n],
        queue: Vec::new(),
        queued: vec![false; n],
        opaque: m.opaque,
        clear: m.clear,
        smooth,
        ring_done: false,
    };
    // Everything is dirty after generation: queue it all.
    for c in 0..n {
        chunks.queue.push(c);
        chunks.queued[c] = true;
    }
    commands.insert_resource(chunks);
    let (pos, yaw) = vox::spawn_for(&world.0, start.0);
    *player = Player {
        pos: Vec3::from_array(pos),
        yaw,
        ..Player::default()
    };
    game.say(match start.0 {
        Biome::Village => WELCOME_LINE,
        _ => "Welcome to the valley! Press 5 for the Pick and hold the mouse button to dig.",
    });
}

/// The smooth look's fixed props: the Great Kiln's glowing rune bands
/// round its dome and a glowing ring on its chimney (the dome itself is
/// the smooth mesh of its bricks).
fn spawn_props(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    clay: Option<Handle<crate::voxel_tex::SmoothMat>>,
) {
    if let Some(clay) = clay {
        // A round clay dome and chimney over the Kiln's brick cells (the
        // cells stay underneath for the puzzle and for collision).
        for mesh in [kiln_dome(), kiln_chimney()] {
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(clay.clone()),
                Transform::IDENTITY,
            ));
        }
    }
    let rune = mats.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.9, 0.8),
        // Thin rune lines, under the bloom ceiling (B4 judges: the first
        // bands read as screen-wide strips).
        emissive: LinearRgba::rgb(0.3, 1.6, 1.35),
        unlit: true,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    // Round the clay dome (`kiln_dome`: centre y 40.8, half-axes 8.0 x 9.3
    // x 5.1), a hair outside it.
    // Above the doorway's arch (`models::kiln_frame`, top y 45.2).
    for (y, h) in [(45.75f32, 0.13f32), (46.15, 0.06)] {
        let k = (1.0 - ((y - 40.8) / 9.3).powi(2)).sqrt();
        let band = ring_band([128.0, y, 169.0], 8.0 * k + 0.06, 5.1 * k + 0.06, h);
        commands.spawn((
            Mesh3d(meshes.add(band)),
            MeshMaterial3d(rune.clone()),
            Transform::IDENTITY,
            NotShadowCaster,
        ));
    }
    let top = ring_band([128.5, 59.6, 169.5], 2.08, 2.08, 0.07);
    commands.spawn((
        Mesh3d(meshes.add(top)),
        MeshMaterial3d(rune),
        Transform::IDENTITY,
        NotShadowCaster,
    ));
}

/// Vertex streams for the smooth material: colour (rgb, AO 1) and UV_0
/// (no glow, not grassy).
fn clay_mesh(pos: Vec<[f32; 3]>, nrm: Vec<[f32; 3]>, idx: Vec<u32>) -> Mesh {
    let look = smooth::look_of(vox::mat::BRICK);
    let n = pos.len();
    // B4 round 3 (the Kiln read as one flat orange): clay courses about a
    // metre high that alternate a little in tone, a darker, damp foot,
    // and soot round the chimney's mouth.
    let colors: Vec<[f32; 4]> = pos
        .iter()
        .map(|p| {
            let course = (p[1] / 0.95).floor() as i32;
            let tone = if course.rem_euclid(2) == 0 { 1.0 } else { 0.9 };
            let foot = 0.8 + 0.2 * ((p[1] - 40.8) / 2.0).clamp(0.0, 1.0);
            let soot = ((p[1] - 56.5) / 3.5).clamp(0.0, 1.0) * 0.7;
            let k = tone * foot;
            let c = [look.rgb[0] * k, look.rgb[1] * k, look.rgb[2] * k];
            let ch = [0.07, 0.055, 0.045];
            [
                c[0] + (ch[0] - c[0]) * soot,
                c[1] + (ch[1] - c[1]) * soot,
                c[2] + (ch[2] - c[2]) * soot,
                1.0,
            ]
        })
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; n])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(idx))
}

/// The Kiln's dome: half an ellipsoid just outside its brick shell
/// (`gen::stamps`, P5), with the runestone door left open on the south.
fn kiln_dome() -> Mesh {
    let (c, r) = ([128.0f32, 40.8, 169.0], [8.0f32, 9.3, 5.1]);
    const AZ: u32 = 64;
    const EL: u32 = 16;
    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    for j in 0..=EL {
        let phi = j as f32 / EL as f32 * std::f32::consts::FRAC_PI_2;
        for i in 0..=AZ {
            let th = i as f32 / AZ as f32 * std::f32::consts::TAU;
            let u = Vec3::new(phi.cos() * th.cos(), phi.sin(), phi.cos() * th.sin());
            pos.push([c[0] + r[0] * u.x, c[1] + r[1] * u.y, c[2] + r[2] * u.z]);
            nrm.push(
                Vec3::new(u.x / r[0], u.y / r[1], u.z / r[2])
                    .normalize()
                    .to_array(),
            );
        }
    }
    let mut idx = Vec::new();
    let door = |p: [f32; 3]| (126.8..130.2).contains(&p[0]) && p[1] < 44.2 && p[2] > 169.0;
    for j in 0..EL {
        for i in 0..AZ {
            let a = j * (AZ + 1) + i;
            let (b, cc, d) = (a + 1, a + AZ + 1, a + AZ + 2);
            let mid = [0, 1, 2].map(|k| (pos[a as usize][k] + pos[d as usize][k]) / 2.0);
            if door(mid) {
                continue;
            }
            idx.extend_from_slice(&[a, cc, b, b, cc, d]);
        }
    }
    clay_mesh(pos, nrm, idx)
}

/// The Kiln's chimney: a round stack over its square brick one, open at
/// the top.
fn kiln_chimney() -> Mesh {
    let (c, r, y0, y1) = ([128.5f32, 169.5], 2.05f32, 46.0f32, 60.0f32);
    const AZ: u32 = 32;
    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=AZ {
        let th = i as f32 / AZ as f32 * std::f32::consts::TAU;
        let (s, co) = th.sin_cos();
        // A slight taper towards the top.
        for (y, rr) in [(y0, r * 1.08), (y1, r)] {
            pos.push([c[0] + rr * co, y, c[1] + rr * s]);
            nrm.push([co, 0.04, s]);
        }
        if i < AZ {
            let b = i * 2;
            idx.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
        }
    }
    clay_mesh(pos, nrm, idx)
}

/// A vertical ribbon `h` tall round the ellipse with centre `c` and
/// half-axes `rx`, `rz`, facing out.
fn ring_band(c: [f32; 3], rx: f32, rz: f32, h: f32) -> Mesh {
    const N: u32 = 96;
    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=N {
        let a = i as f32 / N as f32 * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        let n = Vec3::new(co / rx, 0.0, s / rz).normalize();
        for y in [c[1] - h / 2.0, c[1] + h / 2.0] {
            pos.push([c[0] + rx * co, y, c[2] + rz * s]);
            nrm.push(n.to_array());
        }
        if i < N {
            let b = i * 2;
            idx.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
    .with_inserted_indices(Indices::U32(idx))
}

/// The block world replaces the courtyard (spec section 3.2): its entities
/// go, while `Chem` and `Colliders` stay as inert resources for the shared
/// systems that still read them.
type CourtyardParts = Or<(With<crate::world::LevelRoot>, With<crate::BenchPad>)>;

fn hide_courtyard(mut commands: Commands, q: Query<Entity, CourtyardParts>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ---------------------------------------------------------------------------
// Walking.
// ---------------------------------------------------------------------------

/// One frame of walking: `Player::step`'s rules (`motion`), with the
/// world's built blocks and ice around the player as boxes, a step up of
/// at most `STEP` without jumping, and the world's edge as the bound.
///
/// Natural ground (`smooth::Kind::Soft`) is walked as it is drawn: the feet
/// stand on the smooth surface (`vox::ground`), so ramps that the drawing
/// makes from terrace steps are walked up, and the body never hovers over
/// or sinks into what is shown. Natural ground higher than a step, or above
/// the knees, still stops the body. With `?blocks` every solid block is a
/// box, as before.
pub fn walk(player: &mut Player, walk: &mut Walk, world: &World, input: MoveInput, dt: f32) {
    let MoveInput { wish, sprint, jump } = input;
    let fwd = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let right = Vec3::new(player.yaw.cos(), 0.0, -player.yaw.sin());
    let speed = if sprint { SPRINT } else { WALK };
    let wish = wish.clamp_length_max(1.0);
    let target = (right * wish.x + fwd * wish.y) * speed;
    let body = Body {
        pos: player.pos.to_array(),
        vel: player.vel.to_array(),
        grounded: player.grounded,
        on_ice: player.on_ice,
    };
    let smooth = !walk.blocks;
    if smooth {
        if let Some(y) = lift_out(world, player.pos) {
            player.pos.y = y;
            player.vel.y = player.vel.y.max(0.0);
            player.grounded = true;
        }
    }
    let body = Body {
        pos: player.pos.to_array(),
        ..body
    };
    // Every block the body could touch this frame, step up included.
    let reach = speed.max(player.vel.length()) * dt + 1.2;
    let p = player.pos;
    let r = motion::RADIUS + reach;
    let fall = (player.vel.y.abs() * dt).max(1.0);
    let aabb = [
        p.x - r,
        p.y - fall,
        p.z - r,
        p.x + r,
        p.y + motion::HEIGHT + STEP + reach,
        p.z + r,
    ];
    let ice: Vec<Box3> = vox::ice_boxes(world, aabb).iter().map(box3).collect();
    // Natural ground is a box only above the knees (overhangs, ceilings and
    // the upper part of a wall); below that the smooth surface holds it.
    let soft = |b: &[f32; 6]| {
        smooth::kind(world.get([b[0] as i32, b[1] as i32, b[2] as i32]).mat) == smooth::Kind::Soft
    };
    let mut solids: Vec<Box3> = vox::solid_boxes(world, aabb)
        .iter()
        .filter(|b| !smooth || !soft(b) || b[1] >= p.y + 1.0 - 1e-3)
        .map(box3)
        .collect();
    solids.extend_from_slice(&ice);
    // The blocks themselves, without this frame's floor box.
    let blocks = solids.len();
    let body = match unwedge(&solids, &body) {
        Some(pos) => Body {
            pos,
            vel: [body.vel[0], body.vel[1].min(0.0), body.vel[2]],
            ..body
        },
        None => body,
    };
    let p = Vec3::from_array(body.pos);
    // The drawn ground under the feet, as a floor box for this frame.
    let ground_at = |x: f32, z: f32, y: f32| vox::ground(world, x, z, y + STEP, y - 3.0);
    if smooth {
        if let vox::Ground::Floor(g) = ground_at(p.x, p.z, p.y) {
            let top = g.min(p.y);
            let w = motion::RADIUS + 0.5;
            solids.push(Box3 {
                min: [p.x - w, top - 0.5, p.z - w],
                max: [p.x + w, top, p.z + w],
            });
        }
    }
    const BOUND: f32 = 1.0e5;
    // Space jumps (`tuning::JUMP_HEIGHT`, enough to get onto one block);
    // moving never does.
    let mut a = body;
    if jump && a.grounded {
        a.vel[1] = crate::tuning::jump_speed();
        a.grounded = false;
    }
    motion::advance(
        &mut a,
        [target.x, target.z],
        false,
        dt,
        &solids,
        &ice,
        BOUND,
    );
    let moved =
        |b: &Body| ((b.pos[0] - body.pos[0]).powi(2) + (b.pos[2] - body.pos[2]).powi(2)).sqrt();
    // Blocked means slower than the same frame with nothing in the way
    // (not slower than the wished speed: starting from rest is slow too,
    // and must not try a step up).
    let mut free = body;
    motion::advance(&mut free, [target.x, target.z], false, dt, &[], &[], BOUND);
    let want = moved(&free);
    if body.grounded && !jump && want > 1e-4 && moved(&a) < want * 0.5 {
        let mut b = body;
        b.pos[1] += STEP;
        // Pressed against the step, the body's speed into it is already 0
        // (the last frame's collision), so the raised try walks at the
        // wished speed; otherwise a stopped player could never step up.
        b.vel[0] = target.x;
        b.vel[2] = target.z;
        let room = !solids.iter().any(|s| s.overlaps(&body_box(&b)));
        if room {
            motion::advance(
                &mut b,
                [target.x, target.z],
                false,
                dt,
                &solids,
                &ice,
                BOUND,
            );
            if moved(&b) > moved(&a) + 0.02 {
                walk.lag -= b.pos[1] - a.pos[1];
                a = b;
            }
        }
    }
    if smooth {
        stand_on_surface(world, &body, &mut a, jump, dt, walk, &solids[..blocks]);
    }
    // The world's edge (the protected mountains stop the player first).
    let lim = |v: f32, hi: i32| v.clamp(motion::RADIUS, hi as f32 - motion::RADIUS);
    a.pos[0] = lim(a.pos[0], vox::WORLD[0]);
    a.pos[2] = lim(a.pos[2], vox::WORLD[2]);
    let before = player.pos;
    player.pos = Vec3::from_array(a.pos);
    player.vel = Vec3::from_array(a.vel);
    player.grounded = a.grounded;
    player.on_ice = a.on_ice;
    player.stride += (player.pos - before).xz().length();
    // The camera catches up after a step: fast, but never a pop.
    walk.lag = (walk.lag * (1.0 - (dt * 14.0).min(1.0))).clamp(-STEP, 0.0);
    if walk.lag > -0.002 {
        walk.lag = 0.0;
    }
}

/// Feet deeper than a step inside natural ground (a `tp` into a hill, or
/// ground set where the player stands): the drawn surface of the nearest
/// open space above them, so the body is never left inside the ground
/// (where no floor holds it and it would fall through the hill).
fn lift_out(world: &World, p: Vec3) -> Option<f32> {
    let cell = [
        p.x.floor() as i32,
        (p.y + STEP).floor() as i32,
        p.z.floor() as i32,
    ];
    if smooth::kind(world.get(cell).mat) != smooth::Kind::Soft
        || vox::ground(world, p.x, p.z, p.y + STEP, p.y - 3.0) != vox::Ground::Inside
    {
        return None;
    }
    for k in 1..=48 {
        match vox::ground(world, p.x, p.z, p.y + STEP + k as f32, p.y) {
            vox::Ground::Floor(g) => return Some(g),
            vox::Ground::Inside => continue,
            vox::Ground::None => return None,
        }
    }
    None
}

/// `?blocks`: the old block look and walking (read once).
pub fn blocks_mode() -> bool {
    static B: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *B.get_or_init(|| crate::param("blocks").is_some())
}

/// The first block a ray from `origin` hits, as drawn. On smooth ground a
/// ramp is drawn over part of the air cell above a terrace step, so a ray
/// that meets that drawn ramp hits the ground block under it (a face up),
/// not whatever lies behind (I-052 r3). With `blocks` it is the plain
/// block raycast.
pub fn pick(
    world: &World,
    origin: Vec3,
    dir: Vec3,
    max: f32,
    mode: RayMode,
    blocks: bool,
) -> Option<vox::Hit> {
    let hit = vox::raycast(world, origin.to_array(), dir.to_array(), max, mode);
    if blocks || dir.length_squared() < 1e-12 {
        return hit;
    }
    let d = dir.normalize();
    let end = hit.map_or(max, |h| h.t);
    let mut t = 0.0;
    while t < end {
        let q = origin + d * t;
        let c = q.floor().as_ivec3().to_array();
        if world.get(c).mat == vox::mat::AIR {
            let below = [c[0], c[1] - 1, c[2]];
            let soft = |p: [i32; 3]| smooth::kind(world.get(p).mat) == smooth::Kind::Soft;
            // A ramp is drawn over an air cell only beside a step up (a
            // ground block level with it), so flat ground costs nothing.
            let step = [[1, 0], [-1, 0], [0, 1], [0, -1]]
                .iter()
                .any(|d| soft([c[0] + d[0], c[1], c[2] + d[1]]));
            if step && soft(below) {
                if let vox::Ground::Floor(g) = vox::ground(world, q.x, q.z, q.y + 0.02, q.y - 1.5) {
                    if g >= q.y - 0.02 {
                        return Some(vox::Hit {
                            cell: below,
                            normal: [0, 1, 0],
                            t,
                        });
                    }
                }
            }
        }
        t += 0.1;
    }
    hit
}

/// How high (m) the drawn bank's lip at the edge of a sheet of ice may
/// rise for W to walk up onto it.
const ICE_CLIMB: f32 = 0.8;

/// Feet on or within about a metre of the top of a sheet of ice (the
/// bank's lip beside it).
fn by_ice(world: &World, p: [f32; 3]) -> bool {
    let y = (p[1] - 0.5).floor() as i32;
    (-1..=1).any(|dx| {
        (-1..=1).any(|dz| {
            let c = [
                (p[0] + dx as f32 * 0.9).floor() as i32,
                y,
                (p[2] + dz as f32 * 0.9).floor() as i32,
            ];
            world.get(c).mat == vox::mat::ICE
                || world.get([c[0], c[1] - 1, c[2]]).mat == vox::mat::ICE
        })
    })
}

/// The steepest drawn ground W walks up (rise over run, about 50 degrees).
const MAX_SLOPE: f32 = 1.2;

/// The smooth ground's half of a walking frame: natural ground that rises
/// more than a step (or a wall of it) stops the move, axis by axis so the
/// body slides along it; then the feet are set on the drawn surface when
/// they are below it, or kept on it walking downhill.
fn stand_on_surface(
    world: &World,
    from: &Body,
    a: &mut Body,
    jump: bool,
    dt: f32,
    walk: &mut Walk,
    solids: &[Box3],
) {
    let y = a.pos[1];
    let (x0, z0) = (from.pos[0], from.pos[2]);
    let (x1, z1) = (a.pos[0], a.pos[2]);
    // Ice is drawn as a flat sheet, so the drawn bank beside it rises in a
    // short lip from the bed below the ice: off the ice W walks up that
    // lip, as it would a ramp (I-052 r3: the frozen ford).
    let climb = if from.grounded && by_ice(world, from.pos) {
        ICE_CLIMB
    } else {
        STEP
    };
    // Natural ground in the way: the drawn surface at the centre or the
    // body's leading edge is more than a step up. Built blocks are left to
    // their boxes.
    let natural = |px: f32, pz: f32| {
        let (cx, cz) = (px.floor() as i32, pz.floor() as i32);
        let lo = y.floor() as i32;
        !(lo..=(y + climb).floor() as i32).any(|cy| {
            matches!(
                smooth::kind(world.get([cx, cy, cz]).mat),
                smooth::Kind::Hard | smooth::Kind::Organic | smooth::Kind::Clear
            )
        })
    };
    let blocked = |x: f32, z: f32| -> bool {
        let (dx, dz) = (x - x0, z - z0);
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            return false;
        }
        let lead = motion::RADIUS * 0.9 / len;
        [(x, z), (x + dx * lead, z + dz * lead)]
            .iter()
            .any(|&(px, pz)| {
                natural(px, pz)
                    && vox::ground(world, px, pz, y + climb, y - 1.5) == vox::Ground::Inside
            })
    };
    if blocked(x1, z1) {
        // Keep as much of the move as is clear (a long frame moves far),
        // then try sliding along each axis.
        let mut t = 1.0f32;
        for _ in 0..4 {
            t *= 0.5;
            if !blocked(x0 + (x1 - x0) * t, z0 + (z1 - z0) * t) {
                break;
            }
        }
        if blocked(x0 + (x1 - x0) * t, z0 + (z1 - z0) * t) {
            t = 0.0;
        }
        let (xt, zt) = (x0 + (x1 - x0) * t, z0 + (z1 - z0) * t);
        if !blocked(x1, zt) {
            a.pos[0] = x1;
            a.pos[2] = zt;
            a.vel[2] = 0.0;
        } else if !blocked(xt, z1) {
            a.pos[0] = xt;
            a.pos[2] = z1;
            a.vel[0] = 0.0;
        } else {
            a.pos[0] = xt;
            a.pos[2] = zt;
            a.vel[0] = 0.0;
            a.vel[2] = 0.0;
        }
        // The height came from the move to (x1, z1): where the body is held
        // back to, it must not be inside a block's box (feet that dropped
        // past a bank's edge, set back over the ice box they left).
        if solids.iter().any(|s| s.overlaps(&body_box(a))) {
            a.pos[1] = from.pos[1];
            a.vel[1] = a.vel[1].max(0.0);
            a.grounded = from.grounded;
        }
    }
    let speed = (a.vel[0] * a.vel[0] + a.vel[2] * a.vel[2]).sqrt();
    let snap = 0.3 + speed * dt * 1.2;
    if let vox::Ground::Floor(g) = vox::ground(world, a.pos[0], a.pos[2], y + climb, y - 3.0) {
        let d = ((a.pos[0] - x0).powi(2) + (a.pos[2] - z0).powi(2)).sqrt();
        let off_ice = climb > STEP && g - y <= climb;
        if g > y && from.grounded && !jump && !off_ice && g - y > d * MAX_SLOPE + 0.004 {
            // Steeper than a walkable slope (the face of a wall): stay put.
            a.pos[0] = x0;
            a.pos[2] = z0;
            a.vel[0] = 0.0;
            a.vel[2] = 0.0;
        } else if g > y && from.grounded && !headroom(world, a.pos[0], a.pos[2], g) {
            // No room to stand up there (a low ceiling over the rise; Paawan's
            // playtest: the cave wedged the head into the rock): stay put.
            a.pos[0] = x0;
            a.pos[2] = z0;
            a.vel[0] = 0.0;
            a.vel[2] = 0.0;
        } else if g > y {
            // Up onto the surface (a ramp, or landing on it).
            if g - y > 0.05 {
                walk.lag -= g - y;
            }
            a.pos[1] = g;
            if a.vel[1] <= 0.0 {
                a.vel[1] = 0.0;
                a.grounded = true;
            }
        } else if from.grounded && !jump && a.vel[1] <= 0.0 && y - g < snap && {
            // Never down into a block's box (a bevel is drawn a little
            // below a built block's top; the box holds the feet there).
            // `solids` leaves out the frame's floor box, which is where
            // the feet were, not where they are going.
            let mut b = *a;
            b.pos[1] = g;
            !solids.iter().any(|s| s.overlaps(&body_box(&b)))
        } {
            // Down a slope: stay on it rather than hop off each bump.
            a.pos[1] = g;
            a.vel[1] = 0.0;
            a.grounded = true;
        }
    }
}

/// Whether a body standing at `(x, y, z)` has its height clear of solid
/// cells (any material) in its own column.
fn headroom(world: &World, x: f32, z: f32, y: f32) -> bool {
    let (cx, cz) = (x.floor() as i32, z.floor() as i32);
    let lo = (y + 0.25).floor() as i32;
    let hi = (y + motion::HEIGHT - 0.05).floor() as i32;
    !(lo..=hi).any(|cy| vox::materials::flags(world.get([cx, cy, cz]).mat).solid)
}

/// A body whose box is inside solid blocks (wedged against a ceiling after
/// a step, or set there by a `tp`): the nearest free place a little below,
/// beside or above it, so it never hangs frozen in the rock.
fn unwedge(solids: &[Box3], b: &Body) -> Option<[f32; 3]> {
    let inside = |p: [f32; 3]| {
        let t = Body { pos: p, ..*b };
        solids.iter().any(|s| s.overlaps(&body_box(&t)))
    };
    if !inside(b.pos) {
        return None;
    }
    let [x, y, z] = b.pos;
    let mut tries: Vec<[f32; 3]> = Vec::new();
    for k in 1..=8 {
        let d = k as f32 * 0.125;
        tries.push([x, y - d, z]);
        for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            tries.push([x + dx * d, y, z + dz * d]);
            tries.push([x + dx * d, y - d, z + dz * d]);
        }
    }
    for k in 1..=6 {
        tries.push([x, y + k as f32 * 0.125, z]);
    }
    tries.into_iter().find(|&p| !inside(p))
}

fn box3(b: &[f32; 6]) -> Box3 {
    Box3 {
        min: [b[0], b[1], b[2]],
        max: [b[3], b[4], b[5]],
    }
}

fn body_box(b: &Body) -> Box3 {
    let p = b.pos;
    Box3 {
        min: [p[0] - motion::RADIUS, p[1], p[2] - motion::RADIUS],
        max: [
            p[0] + motion::RADIUS,
            p[1] + motion::HEIGHT,
            p[2] + motion::RADIUS,
        ],
    }
}

// ---------------------------------------------------------------------------
// Debug commands (`?debug`): tp, face, setcell, probe (spec section 8).
// ---------------------------------------------------------------------------

/// Runs one voxel-mode debug command; false if `a` is not one of them.
pub fn debug_cmd(
    a: &[&str],
    world: &mut World,
    player: &mut Player,
    keys: &mut Keys,
    mut matter: Option<&mut realgame_matter::World>,
    probe_n: &mut u32,
) -> bool {
    let f = |i: usize| {
        a.get(i)
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|v| v.is_finite())
    };
    let cell = || -> Option<[i32; 3]> { Some([f(1)? as i32, f(2)? as i32, f(3)? as i32]) };
    match a.first().copied() {
        Some("tp") => {
            if let (Some(x), Some(y), Some(z)) = (f(1), f(2), f(3)) {
                let w = vox::WORLD;
                player.pos = Vec3::new(
                    x.clamp(0.5, w[0] as f32 - 0.5),
                    y.clamp(0.0, w[1] as f32),
                    z.clamp(0.5, w[2] as f32 - 0.5),
                );
                player.vel = Vec3::ZERO;
            } else {
                warn!("voxel tp needs x y z");
            }
            true
        }
        Some("face") => {
            if let (Some(x), Some(y), Some(z)) = (f(1), f(2), f(3)) {
                player.face(Vec3::new(x, y, z));
            }
            true
        }
        Some("setcell") => {
            let m = a.get(4).and_then(|s| {
                materials::by_name(s).or_else(|| {
                    s.parse::<u8>()
                        .ok()
                        .filter(|&m| (m as usize) < materials::table().len())
                })
            });
            match (cell(), m) {
                (Some(p), Some(m)) => {
                    let mut c = Cell::new(m);
                    if materials::flags(m).phase == materials::Phase::Liquid {
                        c.set_level(8);
                    }
                    world.set(p, c);
                    if let Some(m) = matter.as_deref_mut() {
                        m.touch(world, p);
                    }
                }
                _ => warn!("setcell x y z <material>"),
            }
            true
        }
        Some("probe") => {
            if let Some(p) = cell() {
                *probe_n += 1;
                let c = world.get(p);
                let fl = materials::flags(c.mat);
                // The matter sim holds the live temperature and gas (an
                // untouched cell reads its material's resting temperature).
                let (temp_c, co2) = match matter.as_deref() {
                    Some(m) => (m.temp_c(world, p), format!("{:.3}", m.gas_at(p)[0])),
                    None => (c.temp_c(), "null".to_string()),
                };
                keys.probe = Some(format!(
                    "{{\"n\":{},\"at\":[{},{},{}],\"mat\":\"{}\",\"level\":{},\"state\":{},\"tempC\":{:.2},\"co2\":{co2},\"solid\":{},\"protected\":{}}}",
                    probe_n,
                    p[0],
                    p[1],
                    p[2],
                    materials::name(c.mat),
                    c.level(),
                    c.state(),
                    temp_c,
                    fl.solid,
                    !world.can_break(p),
                ));
            }
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// The Pick, placing and the hotbar.
// ---------------------------------------------------------------------------

/// The pack item behind hotbar key `hot` (6-9), if it is a block.
fn held_block(pack: &realgame_sim::valley::Pack, hot: u8) -> Option<u8> {
    let slot = *pack.hot.get(usize::from(hot.checked_sub(6)?))?;
    match pack.slots.get(usize::from(slot?))?.as_ref()?.item {
        Item::Block(m) => Some(m),
        // A torch is placed as a torch block (Paawan's playtest: a bound
        // torch could not be used).
        Item::Torch => Some(vox::mat::TORCH),
        _ => None,
    }
}

/// The pack item a placed block `m` uses up.
fn placed_item(m: u8) -> Item {
    if m == vox::mat::TORCH {
        Item::Torch
    } else {
        Item::Block(m)
    }
}

/// Binds a free key 6-9 to the slot holding `item`, unless one already is.
fn auto_bind(pack: &mut realgame_sim::valley::Pack, item: Item) {
    let holds = |s: Option<u8>, pack: &realgame_sim::valley::Pack| {
        s.and_then(|s| pack.slots.get(usize::from(s)).copied().flatten())
            .is_some_and(|st| st.item == item)
    };
    if pack.hot.iter().any(|&s| holds(s, pack)) {
        return;
    }
    let Some(slot) = pack
        .slots
        .iter()
        .position(|s| s.is_some_and(|st| st.item == item))
    else {
        return;
    };
    let free = (0..4).find(|&k| {
        pack.hot[k]
            .and_then(|s| pack.slots.get(usize::from(s)).copied().flatten())
            .is_none()
    });
    if let Some(k) = free {
        pack.hot[k] = Some(slot as u8);
    }
}

/// The ray through the canvas point `f` (fractions, 0..1 each way).
fn tap_ray(player: &Player, settings: &Settings, f: Vec2) -> Option<Vec3> {
    let fov_y = crate::feel::vfov_for(settings.hfov, settings.aspect);
    crate::feel::ray_dir(f.x, f.y, fov_y, settings.aspect)
        .map(|v| player.rotation() * Vec3::from_array(v))
}

/// The look cell, then the hotbar tools: the Pick digs (held while the
/// pointer is locked; a click without the lock or a tap digs on by itself),
/// and a block on keys 6-9 is placed on the face looked at. Clicks with a
/// spell selected (keys 1-4) are left for `aim_and_cast`.
#[allow(clippy::too_many_arguments)]
fn tools(
    time: Res<Time>,
    settings: Res<Settings>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut web: ResMut<WebIn>,
    mut game: ResMut<Game>,
    player: Res<Player>,
    mut world: ResMut<VoxWorld>,
    mut keys: ResMut<Keys>,
    mut pack: ResMut<crate::Pack>,
    mut dig: ResMut<Dig>,
    mut whereabouts: ResMut<Whereabouts>,
    mut events: MessageWriter<PlayerEvent>,
    mut matter: Option<ResMut<crate::blockchem::Matter>>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    let world = &mut world.0;
    let eye = player.eye();
    let blocks = blocks_mode();
    let look = pick(world, eye, player.forward(), REACH, RayMode::Solids, blocks);
    // With a spell in hand (keys 1-4) the look cell stops at water, where
    // Frost and Fire land (round 2: the ford is aimable from the bank);
    // with the Pick or a block it is the solid block the tool works on.
    let looked = if keys.hot < 5 {
        pick(world, eye, player.forward(), REACH, RayMode::Fluids, blocks)
    } else {
        look
    };
    keys.look_cell = looked.map(|h| {
        let m = world.get(h.cell).mat;
        [h.cell[0], h.cell[1], h.cell[2], i32::from(m)]
    });

    // Where the player is: biome and stamp entry events.
    let feet = player.pos.floor().as_ivec3().to_array();
    let biome = vox::biome_at(feet);
    if whereabouts.biome != Some(biome) {
        whereabouts.biome = Some(biome);
        keys.biome = biome.id();
        events.write(PlayerEvent::EnteredBiome { id: biome.id() });
    }
    let inside: Vec<&'static str> = vox::stamp::ALL
        .iter()
        .filter(|s| s.contains(feet))
        .map(|s| s.id)
        .collect();
    for id in &inside {
        if !whereabouts.stamps.contains(id) {
            events.write(PlayerEvent::InBox { stamp: id });
        }
    }
    whereabouts.stamps = inside;

    if keys.hot < 5 || game.paused {
        dig.cell = None;
        dig.t = 0.0;
        keys.dig_t = if dig.rest > 0.0 { 1.0 } else { 0.0 };
        dig.rest = (dig.rest - dt).max(0.0);
        return;
    }
    // The clicks are the tool's, not a spell's.
    let mut clicked = false;
    let mut auto_target: Option<Option<vox::Hit>> = None;
    web.cmds.retain(|c| {
        let a: Vec<&str> = c.split_whitespace().collect();
        match a.first().copied() {
            Some("fire") => {
                clicked = true;
                false
            }
            Some("cast") => {
                auto_target = Some(look);
                false
            }
            Some("tap") if a.len() == 3 => {
                let f = Vec2::new(a[1].parse().unwrap_or(-1.0), a[2].parse().unwrap_or(-1.0));
                let hit = tap_ray(&player, &settings, f)
                    .and_then(|d| pick(world, eye, d, REACH, RayMode::Solids, blocks));
                auto_target = Some(hit);
                false
            }
            _ => true,
        }
    });
    game.clicks += u32::from(clicked);
    #[cfg(target_arch = "wasm32")]
    let held = {
        let _ = &mouse;
        web.fire && game.locked
    };
    #[cfg(not(target_arch = "wasm32"))]
    let held = mouse.pressed(MouseButton::Left) && game.locked;

    if keys.hot == 5 {
        if clicked {
            dig.told = false;
        }
        // Which block the Pick works on this frame.
        let target = if held || clicked {
            // A click alone digs too: the Pick keeps working on the block
            // clicked after the button comes up (Paawan's playtest: a
            // click at the coal seam seemed to do nothing).
            dig.auto = true;
            look.map(|h| h.cell)
        } else if let Some(t) = auto_target {
            dig.told = false;
            dig.auto = true;
            dig.t = 0.0;
            t.map(|h| h.cell)
        } else if dig.auto {
            dig.cell.filter(|c| {
                (Vec3::from_array(c.map(|v| v as f32 + 0.5)) - eye).length() < REACH + 1.0
            })
        } else {
            None
        };
        if dig.rest > 0.0 {
            dig.rest -= dt;
            keys.dig_t = 1.0;
            return;
        }
        if target != dig.cell {
            dig.t = 0.0;
            dig.cell = target;
        }
        let Some(cell) = target else {
            dig.auto = false;
            keys.dig_t = 0.0;
            return;
        };
        let m = world.get(cell).mat;
        if !world.can_break(cell) {
            dig.t = 0.0;
            dig.auto = false;
            if !dig.told {
                dig.told = true;
                game.say(PROTECTED_LINE);
            }
            keys.dig_t = 0.0;
            return;
        }
        let Some(secs) = materials::dig_secs(m) else {
            keys.dig_t = 0.0;
            return;
        };
        dig.t += dt / secs;
        if dig.t >= 1.0 {
            // Gather what it drops; a full pack leaves the block where it is.
            let drop = materials::drops(m).and_then(materials::by_name);
            if let Some(d) = drop {
                if pack.0.add(Item::Block(d), 1) > 0 {
                    game.say("Pack full. Drop or use something first.");
                    dig.t = 0.0;
                    dig.auto = false;
                    keys.dig_t = 0.0;
                    return;
                }
                auto_bind(&mut pack.0, Item::Block(d));
            }
            world.set(cell, Cell::AIR);
            // Water next to the hole flows in, gas finds the new path.
            if let Some(mt) = matter.as_mut() {
                mt.world.touch(world, cell);
            }
            events.write(PlayerEvent::Dug { mat: m });
            dig.t = 0.0;
            dig.cell = None;
            dig.auto = false;
            dig.rest = DIG_REST;
            keys.dig_t = 1.0;
            return;
        }
        keys.dig_t = dig.t.clamp(0.0, 1.0);
        return;
    }

    // Keys 6-9: place the held block on the face looked at.
    dig.cell = None;
    keys.dig_t = 0.0;
    let hit = if clicked {
        look
    } else if let Some(t) = auto_target {
        t
    } else {
        return;
    };
    let Some(m) = held_block(&pack.0, keys.hot) else {
        let bound = pack
            .0
            .hot
            .get(usize::from(keys.hot.saturating_sub(6)))
            .copied()
            .flatten()
            .and_then(|s| pack.0.slots.get(usize::from(s)).copied().flatten())
            .is_some();
        game.say(if bound {
            UNPLACEABLE_LINE
        } else {
            "Nothing on this key. Press Tab to open your pack."
        });
        return;
    };
    let Some(h) = hit else {
        return;
    };
    let p = [0, 1, 2].map(|a| h.cell[a] + i32::from(h.normal[a]));
    if !world.can_place(p) {
        game.say("Nothing can be built here.");
        return;
    }
    let here = world.get(p).mat;
    if materials::flags(here).phase == materials::Phase::Solid {
        return;
    }
    // Never inside the player.
    let b = body_box(&Body {
        pos: player.pos.to_array(),
        vel: [0.0; 3],
        grounded: true,
        on_ice: false,
    });
    let cell_box = Box3 {
        min: p.map(|v| v as f32),
        max: p.map(|v| v as f32 + 1.0),
    };
    if materials::flags(m).solid && b.overlaps(&cell_box) {
        return;
    }
    if pack.0.take(placed_item(m), 1) {
        world.set(p, Cell::new(m));
        if let Some(mt) = matter.as_mut() {
            mt.world.touch(world, p);
        }
        events.write(PlayerEvent::Placed { mat: m });
    }
}

// ---------------------------------------------------------------------------
// Meshing.
// ---------------------------------------------------------------------------

/// The 3 x 2 x 3 chunks around `p` (the spawn ring of section 5.9).
fn ring(p: Vec3) -> Vec<usize> {
    let c = (p / CHUNK as f32).floor().as_ivec3();
    let mut out = Vec::new();
    for dz in -1..=1 {
        for dy in [-1, 0] {
            for dx in -1..=1 {
                let q = [c.x + dx, c.y + dy, c.z + dz].map(|v| v * CHUNK);
                if let Some(i) = World::chunk_index(q) {
                    out.push(i);
                }
            }
        }
    }
    out
}

/// Remeshes dirty chunks, nearest first, within the frame's time budget
/// (4 ms, 2 ms with `?low`; at least one chunk a frame). Until the spawn
/// ring is meshed the budget is larger, while the intro shows. With
/// `?debug&meshms=N` (capture scripts on software GPUs, where a frame takes
/// seconds) the budget is N ms.
#[allow(clippy::too_many_arguments)]
fn mesh_chunks(
    mut debug_budget: Local<Option<Option<f32>>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    settings: Res<Settings>,
    player: Res<Player>,
    mut world: ResMut<VoxWorld>,
    mut chunks: ResMut<Chunks>,
    mut keys: ResMut<Keys>,
    mesh3d: Query<&Mesh3d>,
) {
    let t0 = Instant::now();
    let dirty = if chunks.smooth.is_some() {
        world.0.take_dirty_smooth()
    } else {
        world.0.take_dirty()
    };
    for c in dirty {
        if !chunks.queued[c] {
            chunks.queued[c] = true;
            chunks.queue.push(c);
        }
    }
    let debug_ms = *debug_budget.get_or_insert_with(|| {
        settings
            .debug
            .then(|| crate::param("meshms").and_then(|v| v.parse::<f32>().ok()))
            .flatten()
            .filter(|v| v.is_finite() && *v > 0.0)
    });
    let budget_ms = if let Some(ms) = debug_ms {
        ms
    } else if !chunks.ring_done {
        40.0
    } else if settings.low {
        2.0
    } else {
        4.0
    };
    let centre = |i: usize| {
        let o = World::chunk_origin(i);
        Vec3::new(o[0] as f32, o[1] as f32, o[2] as f32) + Vec3::splat(CHUNK as f32 / 2.0)
    };
    let me = player.pos;
    // Nearest last, so `pop` takes it.
    chunks.queue.sort_by(|&a, &b| {
        centre(b)
            .distance_squared(me)
            .total_cmp(&centre(a).distance_squared(me))
    });
    while let Some(c) = chunks.queue.pop() {
        chunks.queued[c] = false;
        let o = World::chunk_origin(c);
        // The smooth surface may bulge a little past the chunk's cells, and
        // a tree's crown or a crystal (`props`) a few metres.
        let lo = Vec3::new(o[0] as f32, o[1] as f32, o[2] as f32) - Vec3::splat(4.0);
        let aabb = Aabb::from_min_max(lo, lo + Vec3::splat(CHUNK as f32 + 8.0));
        let passes: [Option<(Mesh, PassMat)>; 5] = match &chunks.smooth {
            Some(sm) => {
                let m = vox::mesh_chunk_smooth(&world.0, c);
                [
                    (&m.opaque, PassMat::Smooth(sm.opaque.clone()), false),
                    (&m.clear, PassMat::Smooth(sm.clear.clone()), true),
                    (&m.water, PassMat::Smooth(sm.water.clone()), false),
                    (&m.props, PassMat::Smooth(sm.opaque.clone()), false),
                    (&m.props_far, PassMat::Smooth(sm.opaque.clone()), false),
                ]
                .map(|(d, mat, clear)| (!d.is_empty()).then(|| (to_smooth_mesh(d, clear), mat)))
            }
            None => {
                let m = vox::mesh_chunk(&world.0, c);
                [
                    (!m.opaque.is_empty())
                        .then(|| (to_mesh(&m.opaque), PassMat::Atlas(chunks.opaque.clone()))),
                    (!m.clear.is_empty())
                        .then(|| (to_mesh(&m.clear), PassMat::Atlas(chunks.clear.clone()))),
                    None,
                    None,
                    None,
                ]
            }
        };
        for (pass, data) in passes.into_iter().enumerate() {
            let old = chunks.ents[c][pass];
            let Some((mesh, mat)) = data else {
                if let Some(e) = old {
                    if let Ok(h) = mesh3d.get(e) {
                        meshes.remove(h.0.id());
                    }
                    commands.entity(e).despawn();
                    chunks.ents[c][pass] = None;
                }
                continue;
            };
            let handle = meshes.add(mesh);
            match old {
                Some(e) => {
                    if let Ok(h) = mesh3d.get(e) {
                        meshes.remove(h.0.id());
                    }
                    commands.entity(e).insert((Mesh3d(handle), aabb));
                }
                None => {
                    let mut e = commands.spawn((Mesh3d(handle), Transform::IDENTITY, aabb));
                    match mat {
                        PassMat::Atlas(h) => e.insert(MeshMaterial3d(h)),
                        PassMat::Smooth(h) => e.insert(MeshMaterial3d(h)),
                    };
                    if (pass > 0 && pass != PROPS_NEAR) || settings.low {
                        e.insert(NotShadowCaster);
                    }
                    if pass == PROPS_NEAR || pass == PROPS_FAR {
                        // `prop_lod` shows one of the two.
                        e.insert(Visibility::Hidden);
                        chunks.far[c] = None;
                    }
                    chunks.ents[c][pass] = Some(e.id());
                }
            }
        }
        if t0.elapsed().as_secs_f32() * 1000.0 >= budget_ms {
            break;
        }
    }
    if !chunks.ring_done {
        let r = ring(me);
        chunks.ring_done = r.iter().all(|&c| !chunks.queued[c]);
        if chunks.ring_done {
            info!("VOXEL spawn ring meshed");
        }
    }
    keys.chunks_meshed = chunks
        .ents
        .iter()
        .filter(|e| e.iter().any(Option::is_some))
        .count() as u32;
    keys.mesh_q = chunks.queue.len() as u32;
    keys.mesh_ms = t0.elapsed().as_secs_f32() * 1000.0;
}

/// Props swap to their simpler meshes with distance from the chunk's
/// centre, with 10 m of hysteresis so a chunk at the edge does not flicker.
/// (Done by hand rather than with `VisibilityRange`, whose dithering needs a
/// storage buffer in the vertex stage that some WebGPU adapters lack.)
fn prop_lod(player: Res<Player>, mut chunks: ResMut<Chunks>, mut vis: Query<&mut Visibility>) {
    let me = player.pos;
    for c in 0..chunks.ents.len() {
        let [.., near, far] = chunks.ents[c];
        if near.is_none() && far.is_none() {
            continue;
        }
        let o = World::chunk_origin(c);
        let centre =
            Vec3::new(o[0] as f32, o[1] as f32, o[2] as f32) + Vec3::splat(CHUNK as f32 / 2.0);
        let d = centre.distance(me);
        let was = chunks.far[c];
        let want = match was {
            Some(true) => d > PROP_LOD_M - 5.0,
            _ => d > PROP_LOD_M + 5.0,
        };
        if was == Some(want) {
            continue;
        }
        chunks.far[c] = Some(want);
        for (e, show) in [(near, !want), (far, want)] {
            if let Some(mut v) = e.and_then(|e| vis.get_mut(e).ok()) {
                *v = if show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

/// Chunk passes holding the props near and far.
const PROPS_NEAR: usize = 3;
const PROPS_FAR: usize = 4;
/// Where props swap to their far meshes: metres from the chunk's centre to
/// the camera (a chunk reaches 16 m either side, so the swap happens
/// between about 45 m and 75 m from the props themselves).
const PROP_LOD_M: f32 = 60.0;

/// A chunk pass's material: the block atlas (`?blocks`) or the smooth look.
enum PassMat {
    Atlas(Handle<crate::voxel_tex::VoxMat>),
    Smooth(Handle<crate::voxel_tex::SmoothMat>),
}

/// A smooth `MeshData` as a Bevy mesh that lives on the GPU only: the
/// colour carries the material's colour and AO, UV_0 its glow and grassy
/// values (in the clear pass, its glow and alpha).
fn to_smooth_mesh(d: &vox::MeshData, clear: bool) -> Mesh {
    let uv0: Vec<[f32; 2]> = if clear {
        d.uvs
            .iter()
            .zip(&d.mats)
            .map(|(uv, &m)| [uv[0], crate::voxel_tex::clear_alpha(m)])
            .collect()
    } else {
        d.uvs.clone()
    };
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv0)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, d.colors.clone())
    .with_inserted_indices(Indices::U32(d.indices.clone()))
}

/// A `MeshData` as a Bevy mesh that lives on the GPU only.
/// The atlas material reads block UVs (UV_0), the tile's origin (UV_1) and
/// the AO light as the vertex colour (`voxel_tex`).
fn to_mesh(d: &vox::MeshData) -> Mesh {
    use crate::voxel_tex::{block_uv, tile_origin};
    let n = d.positions.len();
    let uv0: Vec<[f32; 2]> = (0..n)
        .map(|i| block_uv(d.positions[i], d.normals[i]))
        .collect();
    let uv1: Vec<[f32; 2]> = (0..n)
        .map(|i| tile_origin(d.mats[i], d.normals[i]))
        .collect();
    let colors: Vec<[f32; 4]> = d.light.iter().map(|&l| [l, l, l, 1.0]).collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv0)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, uv1)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(d.indices.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(
        p: &mut Player,
        w: &mut Walk,
        world: &World,
        wish: Vec2,
        jump: bool,
        frames: usize,
    ) -> f32 {
        let mut top = p.pos.y;
        for _ in 0..frames {
            let input = crate::player::MoveInput {
                wish,
                sprint: false,
                jump,
            };
            walk(p, w, world, input, 1.0 / 60.0);
            top = top.max(p.pos.y);
        }
        top
    }

    #[test]
    fn holding_w_into_a_one_block_step_never_hops() {
        // The Kiln terrace's first brick step is at z 180 (top y 36).
        let world = vox::generate(vox::DEFAULT_SEED);
        for z0 in [181.36f32, 184.0] {
            let mut p = Player {
                pos: Vec3::new(128.5, 35.0, z0),
                grounded: true,
                ..Player::default()
            };
            let mut w = Walk::default();
            let top = press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), false, 240);
            assert!(top <= 35.0 + 0.5, "from z {z0}: rose to {top}");
            assert!(p.pos.z > 180.9, "stopped at the step: {:?}", p.pos);
        }
    }

    #[test]
    fn space_jumps_onto_one_block() {
        let world = vox::generate(vox::DEFAULT_SEED);
        let mut p = Player {
            pos: Vec3::new(128.5, 35.0, 181.36),
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        // Standing still, Space alone: up about 1.25 blocks and back down.
        let top = press(&mut p, &mut w, &world, Vec2::ZERO, true, 1);
        let top = top.max(press(&mut p, &mut w, &world, Vec2::ZERO, false, 90));
        assert!(
            (1.15..=1.35).contains(&(top - 35.0)),
            "jump height {}",
            top - 35.0
        );
        assert!((p.pos.y - 35.0).abs() < 0.01);
        // W with a jump gets onto the step.
        press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), true, 1);
        press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), false, 40);
        assert!(p.pos.y >= 35.99 && p.pos.z < 181.0, "{:?}", p.pos);
    }

    /// Flat grass at y 34 (feet at 35), and a one-block step of `m` from
    /// x 40 on (feet at 36).
    fn step_world(m: u8) -> World {
        let mut w = World::new();
        for z in 0..64 {
            for x in 0..64 {
                for y in 0..=34 {
                    w.set([x, y, z], Cell::new(vox::mat::GRASS));
                }
                if x >= 40 {
                    w.set([x, 35, z], Cell::new(m));
                }
            }
        }
        w
    }

    #[test]
    fn w_walks_up_a_grassy_terrace_ramp_and_the_feet_stay_on_the_drawn_ground() {
        let world = step_world(vox::mat::GRASS);
        let mut p = Player {
            pos: Vec3::new(34.5, 35.0, 20.5),
            // Facing +x.
            yaw: -std::f32::consts::FRAC_PI_2,
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        let mut worst = 0f32;
        for _ in 0..180 {
            press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), false, 1);
            if let vox::Ground::Floor(g) =
                vox::ground(&world, p.pos.x, p.pos.z, p.pos.y + 1.0, p.pos.y - 2.0)
            {
                worst = worst.max((p.pos.y - g).abs());
            }
        }
        assert!(p.pos.x > 44.0, "walked up the ramp: {:?}", p.pos);
        assert!(
            (p.pos.y - 36.0).abs() < 0.01,
            "on the upper ground: {:?}",
            p.pos
        );
        assert!(worst < 0.15, "feet off the drawn ground by {worst}");
    }

    #[test]
    fn w_stops_at_a_brick_step_and_a_grass_wall_two_blocks_high() {
        let world = step_world(vox::mat::BRICK);
        let mut p = Player {
            pos: Vec3::new(34.5, 35.0, 20.5),
            yaw: -std::f32::consts::FRAC_PI_2,
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        let top = press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), false, 180);
        assert!(top < 35.2 && p.pos.x < 40.0, "{top} {:?}", p.pos);
        let mut world = step_world(vox::mat::GRASS);
        for z in 0..64 {
            for x in 40..64 {
                world.set([x, 36, z], Cell::new(vox::mat::GRASS));
            }
        }
        let mut p = Player {
            pos: Vec3::new(34.5, 35.0, 20.5),
            yaw: -std::f32::consts::FRAC_PI_2,
            grounded: true,
            ..Player::default()
        };
        let mut top = 0f32;
        for _ in 0..180 {
            top = top.max(press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), false, 1));
        }
        // Up the ramp of the first block, then stopped by the second.
        assert!(top < 36.5 && p.pos.x < 40.0, "{top} {:?}", p.pos);
    }

    #[test]
    fn w_with_space_climbs_the_kiln_terrace_steps() {
        // The golden's E3 walk: W held, Space every 0.6 s, from the plaza.
        let world = vox::generate(vox::DEFAULT_SEED);
        let mut p = Player {
            pos: Vec3::new(128.5, 35.0, 187.5),
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        for f in 0..600 {
            let input = crate::player::MoveInput {
                wish: Vec2::new(0.0, 1.0),
                sprint: false,
                jump: f > 0 && f % 36 == 0,
            };
            walk(&mut p, &mut w, &world, input, 1.0 / 60.0);
        }
        assert!(p.pos.z < 176.0 && p.pos.y > 40.9, "{:?}", p.pos);
        // The same at 4 frames a second (a software GPU, long frames).
        let mut p = Player {
            pos: Vec3::new(128.5, 35.0, 187.5),
            grounded: true,
            ..Player::default()
        };
        for f in 0..100 {
            let input = crate::player::MoveInput {
                wish: Vec2::new(0.0, 1.0),
                sprint: false,
                jump: f > 0 && f % 3 == 0,
            };
            walk(&mut p, &mut w, &world, input, 0.25);
        }
        assert!(p.pos.z < 176.0 && p.pos.y > 40.9, "at 4 fps: {:?}", p.pos);
    }

    #[test]
    fn feet_inside_a_hill_come_up_on_its_surface_and_walk_off_it() {
        // The golden's ford check: `tp 128.5 35 163.5` is inside the
        // village's rise above the ford (its top is y 41 there).
        let world = vox::generate(vox::DEFAULT_SEED);
        let mut p = Player {
            pos: Vec3::new(128.5, 35.0, 163.5),
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        press(&mut p, &mut w, &world, Vec2::ZERO, false, 2);
        assert!((p.pos.y - 41.0).abs() < 0.2, "on the top: {:?}", p.pos);
        // W walks off the edge and down the bank to the water's edge.
        for _ in 0..480 {
            press(&mut p, &mut w, &world, Vec2::new(0.0, 1.0), false, 1);
            if p.pos.z < 158.7 {
                break;
            }
        }
        press(&mut p, &mut w, &world, Vec2::ZERO, false, 90);
        assert!(p.pos.z < 158.9 && p.pos.y < 33.1, "{:?}", p.pos);
    }

    #[test]
    fn a_long_frame_down_the_ford_bank_keeps_the_feet_on_it() {
        // The ford's mouth once its gate has sunk: one 0.25 s frame from
        // the bank's ramp lands on the ground, not a metre above it.
        let mut world = vox::generate(vox::DEFAULT_SEED);
        let cells = crate::gates::build(&mut world, &crate::gates::GATES[0]);
        crate::gates::open(&mut world, &cells);
        let mut p = Player {
            pos: Vec3::new(128.5, 33.59, 159.35),
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        let input = crate::player::MoveInput {
            wish: Vec2::new(0.0, 1.0),
            sprint: false,
            jump: false,
        };
        walk(&mut p, &mut w, &world, input, 0.25);
        let vox::Ground::Floor(g) = vox::ground(&world, p.pos.x, p.pos.z, 36.0, 28.0) else {
            panic!("no ground at {:?}", p.pos);
        };
        assert!(
            p.pos.z < 158.5 && (p.pos.y - g).abs() < 0.05,
            "{:?} ground {g}",
            p.pos
        );
    }

    #[test]
    fn long_frames_on_a_frozen_ford_stay_on_the_ice_at_the_far_bank() {
        // The ford frozen (its top water layer is ice at y 30); W at 4 fps
        // from the ice toward the far bank, Space now and then: the body
        // never drops into the ice or is thrown back across it.
        let mut world = vox::generate(vox::DEFAULT_SEED);
        for z in 150..158 {
            for x in 120..136 {
                if world.get([x, 30, z]).mat == vox::mat::WATER {
                    world.set([x, 30, z], realgame_voxel::Cell::new(vox::mat::ICE));
                }
            }
        }
        let mut p = Player {
            pos: Vec3::new(128.5, 31.0, 151.5),
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        for f in 0..40 {
            if p.pos.z < 148.5 && p.pos.y > 32.9 {
                break;
            }
            let z0 = p.pos.z;
            let input = crate::player::MoveInput {
                wish: Vec2::new(0.0, 1.0),
                sprint: false,
                jump: f % 5 == 4,
            };
            walk(&mut p, &mut w, &world, input, 0.25);
            assert!(p.pos.z <= z0 + 0.01, "thrown back: {z0} -> {:?}", p.pos);
            assert!(p.pos.y > 30.99, "into the ice: {:?}", p.pos);
        }
        assert!(
            p.pos.z < 148.5 && p.pos.y > 32.9,
            "up the far bank: {:?}",
            p.pos
        );
    }

    /// P2 on smooth ground: with the gate sunk and the ford frozen, W alone
    /// (no Space) walks from the Village bank down onto the ice, across
    /// and up the far bank (z < 147, the quest's "across"), at 60 fps and
    /// at 4 fps, down the middle and along either side.
    #[test]
    fn w_alone_walks_across_the_frozen_ford() {
        let mut world = vox::generate(vox::DEFAULT_SEED);
        let cells = crate::gates::build(&mut world, &crate::gates::GATES[0]);
        crate::gates::open(&mut world, &cells);
        for z in 140..160 {
            for x in 120..136 {
                if world.get([x, 30, z]).mat == vox::mat::WATER {
                    world.set([x, 30, z], realgame_voxel::Cell::new(vox::mat::ICE));
                }
            }
        }
        for x0 in [124.5f32, 128.5, 131.5] {
            for dt in [1.0f32 / 60.0, 0.25] {
                let mut p = Player {
                    pos: Vec3::new(x0, 35.0, 162.5),
                    grounded: true,
                    ..Player::default()
                };
                let mut w = Walk::default();
                let mut path = Vec::new();
                let mut t = 0.0;
                while t < 12.0 && !(p.pos.z < 147.0 && p.pos.y > 33.5) {
                    let input = crate::player::MoveInput {
                        wish: Vec2::new(0.0, 1.0),
                        sprint: false,
                        jump: false,
                    };
                    walk(&mut p, &mut w, &world, input, dt);
                    t += dt;
                    if path.last().is_none_or(|q: &Vec3| q.distance(p.pos) > 0.5) {
                        path.push(p.pos);
                    }
                }
                assert!(
                    p.pos.z < 147.0 && p.pos.y > 33.5,
                    "x {x0}, dt {dt}: stuck at {:?} after {t:.1} s; path {:?}",
                    p.pos,
                    path.iter()
                        .map(|v| format!("({:.2},{:.2})", v.y, v.z))
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    /// A ray that meets a drawn ramp (over the air cell above a terrace
    /// step) hits the ground under it, as drawn; with `?blocks` it passes.
    #[test]
    fn a_ray_hits_the_drawn_ramp_not_the_air_cell() {
        let world = vox::generate(vox::DEFAULT_SEED);
        // The ford's north bank (rows a block a metre from the ice): look
        // down the bank from above its top, along it.
        let eye = Vec3::new(128.5, 37.0, 144.0);
        let mut found = 0;
        for k in 0..40 {
            let dir = Vec3::new(0.0, -0.2 - k as f32 * 0.02, 1.0).normalize();
            let drawn = pick(&world, eye, dir, 20.0, RayMode::Solids, false);
            let plain = pick(&world, eye, dir, 20.0, RayMode::Solids, true);
            let (Some(a), Some(b)) = (drawn, plain) else {
                continue;
            };
            assert!(a.t <= b.t + 1e-4, "never further than the blocks");
            assert!(
                world.get(a.cell).mat != vox::mat::AIR,
                "hits a block: {:?}",
                a.cell
            );
            if a.t + 0.05 < b.t {
                // Met a ramp first: the hit point is on the drawn ground.
                let q = eye + dir * a.t;
                let vox::Ground::Floor(g) = vox::ground(&world, q.x, q.z, q.y + 0.5, q.y - 1.5)
                else {
                    panic!("no drawn ground at {q}");
                };
                assert!((g - q.y).abs() < 0.15, "on the drawn ground: {q} vs {g}");
                found += 1;
            }
        }
        assert!(found > 0, "some rays meet a ramp before a block");
    }

    #[test]
    fn seeds_parse_in_hex_and_decimal() {
        assert_eq!(parse_seed("0x524731"), Some(0x52_47_31));
        assert_eq!(parse_seed("12"), Some(12));
        assert_eq!(parse_seed("zz"), None);
    }
}

#[cfg(test)]
mod cave_walk {
    use super::*;

    /// Paawan's playtest (2026-11-04): walking in the cave was glitchy, and a
    /// scripted walk froze for good at the mouth with the head wedged in the
    /// rock. W (and Space when held up for a third of a second) along the
    /// tunnels' centre lines reaches the rig room, never stuck for a second.
    #[test]
    fn w_walks_from_the_cave_mouth_to_the_rig_without_freezing() {
        let world = vox::generate(vox::DEFAULT_SEED);
        let pts: [[f32; 3]; 6] = [
            [220.5, 31.0, 58.5],
            [220.5, 22.0, 50.5],
            [218.5, 13.0, 42.5],
            [228.5, 12.0, 34.5],
            [231.5, 11.0, 30.5],
            [231.5, 11.0, 27.5],
        ];
        let mut path: Vec<Vec3> = Vec::new();
        for k in 0..pts.len() - 1 {
            let (a, b) = (Vec3::from_array(pts[k]), Vec3::from_array(pts[k + 1]));
            let n = ((b - a).xz().length().ceil() as i32).max(1);
            path.extend((0..n).map(|i| a.lerp(b, i as f32 / n as f32)));
        }
        path.push(Vec3::from_array(pts[5]));
        let mut p = Player {
            pos: Vec3::new(220.5, 28.0, 58.5),
            grounded: true,
            ..Player::default()
        };
        let mut w = Walk::default();
        let (mut wp, mut held, mut worst) = (0usize, 0u32, 0u32);
        for _ in 0..60 * 40 {
            if wp >= path.len() {
                break;
            }
            let d = path[wp] - p.pos;
            if d.xz().length() < 0.6 {
                wp += 1;
                continue;
            }
            p.yaw = (-d.x).atan2(-d.z);
            let before = p.pos;
            let jump = held >= 20;
            let input = crate::player::MoveInput {
                wish: Vec2::new(0.0, 1.0),
                sprint: false,
                jump,
            };
            walk(&mut p, &mut w, &world, input, 1.0 / 60.0);
            if (p.pos - before).xz().length() < 0.02 && !jump {
                held += 1;
            } else {
                held = 0;
            }
            worst = worst.max(held);
        }
        assert_eq!(wp, path.len(), "reached the rig: at {:?}", p.pos);
        assert!(worst < 60, "held up {worst} frames");
    }

    /// A body set inside the rock is moved to the nearest free place.
    #[test]
    fn a_wedged_body_is_moved_out_of_the_blocks() {
        let ceiling = Box3 {
            min: [-2.0, 1.5, -2.0],
            max: [2.0, 2.5, 2.0],
        };
        let b = Body {
            pos: [0.0, 0.0, 0.0],
            vel: [0.0; 3],
            grounded: false,
            on_ice: false,
        };
        let out = unwedge(&[ceiling], &b).expect("moved");
        assert!(out[1] + motion::HEIGHT <= 1.5 + 1e-4, "{out:?}");
        let free = Body {
            pos: [0.0, -1.0, 0.0],
            ..b
        };
        assert_eq!(unwedge(&[ceiling], &free), None);
    }
}
