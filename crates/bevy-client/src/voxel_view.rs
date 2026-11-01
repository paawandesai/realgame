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
use realgame_voxel::{self as vox, materials, Biome, Cell, RayMode, World, CHUNK};

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

/// How far the Pick and placing reach, metres from the eye.
pub const REACH: f32 = 5.0;
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
/// The caption for a protected block (spec section 4).
pub const PROTECTED_LINE: &str = "This old runestone will not budge. Try chemistry.";

/// Chunk meshes: the opaque and clear entities of each chunk, the mesh
/// queue, and the materials.
#[derive(Resource)]
struct Chunks {
    ents: Vec<[Option<Entity>; 2]>,
    /// Waiting to be meshed (index order; sorted by distance when picked).
    queue: Vec<usize>,
    queued: Vec<bool>,
    opaque: Handle<crate::voxel_tex::VoxMat>,
    clear: Handle<crate::voxel_tex::VoxMat>,
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
    .init_resource::<Walk>()
    .init_resource::<Whereabouts>()
    .add_systems(Startup, setup.after(crate::setup))
    .add_systems(PostStartup, hide_courtyard)
    .add_systems(
        Update,
        (tools, mesh_chunks)
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

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<crate::voxel_tex::VoxMat>>,
    world: Res<VoxWorld>,
    start: Res<crate::StartBiome>,
    mut player: ResMut<Player>,
    mut game: ResMut<Game>,
) {
    let m = crate::voxel_tex::materials(&mut images, &mut mats, world.0.seed);
    let n = world.0.chunk_count();
    let mut chunks = Chunks {
        ents: vec![[None, None]; n],
        queue: Vec::new(),
        queued: vec![false; n],
        opaque: m.opaque,
        clear: m.clear,
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
        Biome::Village => "Welcome to the valley! Walk with WASD. Press 5 for the Pick and hold the mouse button to dig.",
        _ => "Welcome to the valley! Press 5 for the Pick and hold the mouse button to dig.",
    });
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

/// One frame of walking on blocks: `Player::step`'s rules (`motion`), with
/// the world's solid and ice blocks around the player as boxes, a step up
/// of one block without jumping, and the world's edge as the bound.
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
    let mut solids: Vec<Box3> = vox::solid_boxes(world, aabb).iter().map(box3).collect();
    solids.extend_from_slice(&ice);
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
    let want = target.length() * dt;
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
        _ => None,
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
    let look = vox::raycast(
        world,
        eye.to_array(),
        player.forward().to_array(),
        REACH,
        RayMode::Solids,
    );
    keys.look_cell = look.map(|h| {
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
                let hit = tap_ray(&player, &settings, f).and_then(|d| {
                    vox::raycast(world, eye.to_array(), d.to_array(), REACH, RayMode::Solids)
                });
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
            dig.auto = false;
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
        game.say("Nothing on this key. Press Tab to open your pack.");
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
    if pack.0.take(Item::Block(m), 1) {
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
/// ring is meshed the budget is larger, while the intro shows.
#[allow(clippy::too_many_arguments)]
fn mesh_chunks(
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
    for c in world.0.take_dirty() {
        if !chunks.queued[c] {
            chunks.queued[c] = true;
            chunks.queue.push(c);
        }
    }
    let budget_ms = if !chunks.ring_done {
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
    let mut done = 0;
    while let Some(c) = chunks.queue.pop() {
        chunks.queued[c] = false;
        let m = vox::mesh_chunk(&world.0, c);
        let o = World::chunk_origin(c);
        let aabb = Aabb::from_min_max(
            Vec3::new(o[0] as f32, o[1] as f32, o[2] as f32),
            Vec3::new(o[0] as f32, o[1] as f32, o[2] as f32) + Vec3::splat(CHUNK as f32),
        );
        for (pass, data) in [&m.opaque, &m.clear].into_iter().enumerate() {
            let old = chunks.ents[c][pass];
            if data.is_empty() {
                if let Some(e) = old {
                    if let Ok(h) = mesh3d.get(e) {
                        meshes.remove(h.0.id());
                    }
                    commands.entity(e).despawn();
                    chunks.ents[c][pass] = None;
                }
                continue;
            }
            let handle = meshes.add(to_mesh(data));
            match old {
                Some(e) => {
                    if let Ok(h) = mesh3d.get(e) {
                        meshes.remove(h.0.id());
                    }
                    commands.entity(e).insert(Mesh3d(handle));
                }
                None => {
                    let mat = if pass == 0 {
                        chunks.opaque.clone()
                    } else {
                        chunks.clear.clone()
                    };
                    let mut e = commands.spawn((
                        Mesh3d(handle),
                        MeshMaterial3d(mat),
                        Transform::IDENTITY,
                        aabb,
                    ));
                    if pass == 1 || settings.low {
                        e.insert(NotShadowCaster);
                    }
                    chunks.ents[c][pass] = Some(e.id());
                }
            }
        }
        done += 1;
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
    let _ = done;
    keys.chunks_meshed = chunks
        .ents
        .iter()
        .filter(|e| e[0].is_some() || e[1].is_some())
        .count() as u32;
    keys.mesh_q = chunks.queue.len() as u32;
    keys.mesh_ms = t0.elapsed().as_secs_f32() * 1000.0;
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

    #[test]
    fn seeds_parse_in_hex_and_decimal() {
        assert_eq!(parse_seed("0x524731"), Some(0x52_47_31));
        assert_eq!(parse_seed("12"), Some(12));
        assert_eq!(parse_seed("zz"), None);
    }
}
