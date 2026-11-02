//! Spell physics in the block world (I-051 ported to I-052): Water and CO2
//! charge while the button is held and fly a gravity arc on release (with a
//! dotted preview of the arc while charging); Fire and Frost are short cones.
//! The landing is found by stepping the arc through the voxel cells, and a
//! landed spell goes out as a `crate::SpellLanded` message, which
//! `blockchem::drive` turns into a matter emit. This is voxel mode's only
//! cast path (`aim_and_cast` hands every cast to `Charge::shoot`); the
//! classic courtyard's beam is not touched. Without a `VoxWorld` (tests),
//! spells land on the plane y = 0.

use crate::voxel_view::{Keys as ViewKeys, VoxWorld};
use crate::{ballistics, bridge::WebIn, chem::Spell, clock, worked, Game, Kit, SpellMats, Working};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use realgame_voxel::{mat, World};

/// Where a planned flight comes to rest: the block (if any), the face it
/// came in by, and the point. Sent on arrival as `crate::SpellLanded`,
/// which `blockchem::drive` turns into the matter emit.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Arrival {
    cell: Option<[i32; 3]>,
    normal: [i8; 3],
    pos: Vec3,
}

/// The charge (seconds held, while Water or CO2 charges), the hold of the
/// cast just released, and the shots `aim_and_cast` has fired this frame.
#[derive(Resource, Default)]
pub(crate) struct Charge {
    pub held: Option<f32>,
    pub release: Option<f32>,
    shots: Vec<Shot>,
}

struct Shot {
    spell: u8,
    hand: Vec3,
    forward: Vec3,
    held: f32,
    epoch: u32,
}

impl Charge {
    /// The cast `aim_and_cast` makes in the block world, in place of the
    /// courtyard's beam: the hold is the release's, else the click's.
    pub fn shoot(&mut self, spell: usize, hand: Vec3, forward: Vec3, held: f32, epoch: u32) {
        let held = self.release.take().unwrap_or(held);
        self.shots.push(Shot {
            spell: spell as u8,
            hand,
            forward,
            held,
            epoch,
        });
    }
}

/// A spell in flight along its planned path.
#[derive(Component)]
struct Flying {
    path: Vec<Vec3>,
    t: f32,
    dur: f32,
    spell: u8,
    landing: Option<Arrival>,
    epoch: u32,
}

/// One dot of the arc preview.
#[derive(Component)]
struct ArcDot;

/// Dots in the arc preview.
const ARC_DOTS: usize = 24;
/// Arcs are traced in steps of this long, for at most `MAX_FLIGHT_S`.
const STEP_S: f32 = 1.0 / 60.0;
const MAX_FLIGHT_S: f32 = 4.0;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Charge>().add_systems(
        Update,
        (
            charge.after(crate::control).before(crate::aim_and_cast),
            (fly, preview)
                .after(crate::aim_and_cast)
                .before(crate::publish),
        ),
    );
}

/// Where a spell cast from `hand` along `forward` goes: the path it flies,
/// its flight time, and where it lands (if it does). Water and CO2 fly a
/// gravity arc whose reach grows with `held`; Fire and Frost are a short
/// straight spray that stops at the first block in its range.
fn plan(
    blocks: Option<&World>,
    spell: u8,
    hand: Vec3,
    forward: Vec3,
    held: f32,
) -> (Vec<Vec3>, f32, Option<Arrival>) {
    let landed = |l: Landing| Arrival {
        cell: Some(l.cell),
        normal: l.normal,
        pos: Vec3::from(l.pos),
    };
    match crate::tuning::motion(spell, held) {
        ballistics::Motion::Arc { reach } => {
            let v0 = forward * ballistics::launch_speed(reach);
            let (end, t) = match blocks {
                Some(w) => match land_in(
                    w,
                    hand.to_array(),
                    v0.to_array(),
                    STEP_S,
                    MAX_FLIGHT_S,
                    if spell == Spell::Water as u8 {
                        stops_water
                    } else {
                        stops
                    },
                ) {
                    Some(l) => (Some(landed(l)), l.t),
                    None => (None, MAX_FLIGHT_S),
                },
                None => {
                    let t = ballistics::landing_time(hand, v0, 0.0).unwrap_or(MAX_FLIGHT_S);
                    let p = ballistics::at(hand, v0, t);
                    (
                        Some(Arrival {
                            cell: None,
                            normal: [0, 1, 0],
                            pos: p,
                        }),
                        t,
                    )
                }
            };
            let n = ((t / STEP_S).ceil() as usize).clamp(2, 240);
            let path = (0..=n)
                .map(|k| ballistics::at(hand, v0, t * k as f32 / n as f32))
                .collect();
            (path, t.max(0.05), end)
        }
        ballistics::Motion::Cone { range, .. } => {
            let b = hand + forward * range;
            let hit = match blocks {
                Some(w) => walk(w, hand, b).map(|(cell, normal, f)| Arrival {
                    cell: Some(cell),
                    normal,
                    pos: hand + (b - hand) * f,
                }),
                // No world: the spray stops where it meets y = 0.
                None => (b.y < 0.0 && hand.y > 0.0).then(|| Arrival {
                    cell: None,
                    normal: [0, 1, 0],
                    pos: hand + (b - hand) * (hand.y / (hand.y - b.y)),
                }),
            };
            let to = hit.map_or(b, |h| h.pos);
            (
                vec![hand, to],
                (hand.distance(to) / crate::tuning::CONE_SPEED).max(0.05),
                hit,
            )
        }
    }
}

fn arc_spell(spell: usize, working: &Working) -> bool {
    matches!(Spell::ALL[spell], Spell::Water | Spell::Foam)
        && working.routes[spell] == worked::Route::Instant
}

/// Water and CO2 charge while the button is held and are cast on release
/// (one press, one cast; a worked route still opens its builder on the
/// press). `cast <hold>` casts as if held that long.
fn charge(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut web: ResMut<WebIn>,
    mut game: ResMut<Game>,
    mut ch: ResMut<Charge>,
    working: Res<Working>,
    view: Option<Res<ViewKeys>>,
) {
    let dt = clock::frame_dt(time.delta_secs());
    for c in &mut web.cmds {
        if let Some(h) = c
            .strip_prefix("cast ")
            .and_then(|v| v.trim().parse::<f32>().ok())
        {
            ch.release = Some(h);
            *c = "cast".to_string();
        }
    }
    // A tool in hand (the Pick or a block, keys 5-9): clicks dig and place.
    let tool = view.as_ref().is_some_and(|k| k.hot >= 5);
    if tool || !arc_spell(game.spell, &working) {
        ch.held = None;
        return;
    }
    #[cfg(target_arch = "wasm32")]
    let firing = {
        let _ = &mouse;
        web.fire
    };
    #[cfg(not(target_arch = "wasm32"))]
    let firing = mouse.pressed(MouseButton::Left);
    let presses = web.cmds.iter().filter(|c| c.as_str() == "fire").count() as u32;
    if presses > 0 && game.locked && !game.paused {
        web.cmds.retain(|c| c != "fire");
        // Counted here, since `aim_and_cast` no longer sees them.
        game.clicks += presses;
        ch.held = Some(0.0);
    }
    let Some(c) = ch.held else {
        return;
    };
    if game.paused {
        // Kept while paused; resumes after.
    } else if firing && game.locked {
        ch.held = Some(c + dt);
        // Hold back the courtyard's held-fire repeat while charging.
        game.cooldown = game.cooldown.max(dt + 1e-3);
    } else {
        ch.held = None;
        ch.release = Some(c);
        web.cmds.push("cast".to_string());
    }
}

/// Launches the shots fired this frame and moves the ones in flight; a
/// spell that arrives sends `SpellLanded` and counts as an impact.
#[allow(clippy::too_many_arguments)]
fn fly(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut ch: ResMut<Charge>,
    blocks: Option<Res<VoxWorld>>,
    kit: Res<Kit>,
    mats: Res<SpellMats>,
    mut flying: Query<(Entity, &mut Transform, &mut Flying)>,
    mut landed: MessageWriter<crate::SpellLanded>,
) {
    let world = blocks.as_deref().map(|b| &b.0);
    for s in std::mem::take(&mut ch.shots) {
        let (path, dur, landing) = plan(world, s.spell, s.hand, s.forward, s.held);
        let size = if s.spell == Spell::Foam as u8 {
            0.4
        } else {
            0.26
        };
        commands.spawn((
            Mesh3d(kit.ball.clone()),
            MeshMaterial3d(mats.glow[s.spell as usize].clone()),
            Transform::from_translation(s.hand).with_scale(Vec3::splat(size)),
            NotShadowCaster,
            Flying {
                path,
                t: 0.0,
                dur,
                spell: s.spell,
                landing,
                epoch: s.epoch,
            },
        ));
    }
    let dt = if game.paused {
        0.0
    } else {
        clock::frame_dt(time.delta_secs())
    };
    for (e, mut tf, mut f) in &mut flying {
        if f.epoch != game.epoch {
            commands.entity(e).despawn();
            continue;
        }
        f.t += dt;
        tf.translation = ballistics::along(&f.path, (f.t / f.dur).min(1.0));
        if f.t < f.dur {
            continue;
        }
        if let Some(l) = f.landing {
            let c = l.cell.unwrap_or([i32::MIN; 3]);
            let h = f.path[0];
            info!(
                "VLAND spell={} x={:.2} y={:.2} z={:.2} cell={},{},{} n={},{},{} from={:.2},{:.2},{:.2}",
                f.spell,
                l.pos.x,
                l.pos.y,
                l.pos.z,
                c[0],
                c[1],
                c[2],
                l.normal[0],
                l.normal[1],
                l.normal[2],
                h.x,
                h.y,
                h.z
            );
            landed.write(crate::SpellLanded {
                spell: Spell::ALL[f.spell as usize],
                pos: l.pos,
                normal: l.normal,
            });
            game.impacts += 1;
        }
        commands.entity(e).despawn();
    }
}

/// While Water or CO2 charges, a faint dotted arc shows the path the
/// release will fly, growing with the charge and ending where it lands.
#[allow(clippy::too_many_arguments)]
fn preview(
    mut commands: Commands,
    game: Res<Game>,
    ch: Res<Charge>,
    player: Res<crate::Player>,
    blocks: Option<Res<VoxWorld>>,
    kit: Res<Kit>,
    mats: Res<SpellMats>,
    mut dots: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<ArcDot>,
    >,
) {
    if dots.is_empty() {
        for _ in 0..ARC_DOTS {
            commands.spawn((
                Mesh3d(kit.ball.clone()),
                MeshMaterial3d(mats.glow[1].clone()),
                Transform::from_scale(Vec3::splat(0.05)),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
                ArcDot,
            ));
        }
        return;
    }
    let Some(held) = ch.held.filter(|_| game.locked && !game.paused) else {
        for (_, mut v, _) in &mut dots {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
        return;
    };
    let hand = player.eye() + player.rotation() * Vec3::new(0.2, -0.15, -0.5);
    let (path, _, _) = plan(
        blocks.as_deref().map(|b| &b.0),
        game.spell as u8,
        hand,
        player.forward(),
        held,
    );
    for (k, (mut t, mut v, mut m)) in dots.iter_mut().enumerate() {
        t.translation = ballistics::along(&path, (k as f32 + 1.0) / ARC_DOTS as f32);
        if *v != Visibility::Visible {
            *v = Visibility::Visible;
        }
        if m.0 != mats.glow[game.spell] {
            m.0 = mats.glow[game.spell].clone();
        }
    }
}

/// Where a spell came to rest in the block world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// The block it hit (solid or water).
    pub cell: [i32; 3],
    /// The face it came in through (the cell it was in just before).
    pub normal: [i8; 3],
    /// The point where it arrived.
    pub pos: [f32; 3],
    /// Flight time, seconds.
    pub t: f32,
}

/// Whether a flying spell stops at a block of material `m`: every block the
/// table marks `solid`, and water; air, steam and torches let it pass.
pub fn stops(m: u8) -> bool {
    !matches!(m, mat::AIR | mat::STEAM | mat::TORCH)
}

/// Traces the arc from `hand` with velocity `v0` through `world` (steps of
/// `dt`, at most `max_t`) and returns where it lands, if it does. Each step's
/// chord is walked cell by cell (a 3D DDA), so a thin wall between two
/// samples is still hit; leaving the world's bottom counts as no landing.
#[cfg(test)]
pub fn land(world: &World, hand: [f32; 3], v0: [f32; 3], dt: f32, max_t: f32) -> Option<Landing> {
    land_in(world, hand, v0, dt, max_t, stops)
}

/// What stops a Water arc (round 2, P1): as `stops`, but water lets it
/// through, since water poured into a puddle runs on to what is under it
/// (the last cast's water on a burning store must not catch the next one).
pub fn stops_water(m: u8) -> bool {
    m != mat::WATER && stops(m)
}

/// `land`, with `stop` deciding which cells stop the arc.
pub fn land_in(
    world: &World,
    hand: [f32; 3],
    v0: [f32; 3],
    dt: f32,
    max_t: f32,
    stop: fn(u8) -> bool,
) -> Option<Landing> {
    let p0 = Vec3::from(hand);
    let v = Vec3::from(v0);
    let mut a = p0;
    let mut t = 0.0;
    while t < max_t {
        let t1 = (t + dt).min(max_t);
        let b = ballistics::at(p0, v, t1);
        if let Some((cell, normal, f)) = walk_in(world, a, b, stop) {
            let pos = a + (b - a) * f;
            return Some(Landing {
                cell,
                normal,
                pos: pos.to_array(),
                t: t + (t1 - t) * f,
            });
        }
        if b.y < 0.0 {
            return None;
        }
        a = b;
        t = t1;
    }
    None
}

/// Walks the segment `a`..`b` through the cells it crosses and returns the
/// first one that stops a spell, the face it entered by, and how far along
/// the segment (0-1) it entered.
fn walk(world: &World, a: Vec3, b: Vec3) -> Option<([i32; 3], [i8; 3], f32)> {
    walk_in(world, a, b, stops)
}

fn walk_in(
    world: &World,
    a: Vec3,
    b: Vec3,
    stop: fn(u8) -> bool,
) -> Option<([i32; 3], [i8; 3], f32)> {
    let d = b - a;
    let mut c = a.floor().as_ivec3();
    let end = b.floor().as_ivec3();
    let step = d.signum().as_ivec3();
    let next = |i: usize| -> f32 {
        if d[i] == 0.0 {
            f32::INFINITY
        } else {
            let edge = if d[i] > 0.0 {
                c[i] as f32 + 1.0
            } else {
                c[i] as f32
            };
            (edge - a[i]) / d[i]
        }
    };
    let delta = Vec3::new(1.0 / d.x.abs(), 1.0 / d.y.abs(), 1.0 / d.z.abs());
    let mut tmax = Vec3::new(next(0), next(1), next(2));
    // The cell the segment starts in is where the last one ended: not a hit.
    for _ in 0..64 {
        if c == end {
            return None;
        }
        let i = if tmax.x <= tmax.y && tmax.x <= tmax.z {
            0
        } else if tmax.y <= tmax.z {
            1
        } else {
            2
        };
        let f = tmax[i];
        if f > 1.0 {
            return None;
        }
        c[i] += step[i];
        tmax[i] += delta[i];
        let p = c.to_array();
        if World::in_bounds(p) && stop(world.get(p).mat) {
            let mut n = [0i8; 3];
            n[i] = -step[i] as i8;
            return Some((p, n, f));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::Cell;

    /// A stone floor at y = 9 (top face at y = 10) over x, z in 0..40.
    fn floor() -> World {
        let mut w = World::new();
        for x in 0..40 {
            for z in 0..40 {
                w.set([x, 9, z], Cell::new(mat::STONE));
            }
        }
        w
    }

    fn launch(speed: f32, deg: f32) -> [f32; 3] {
        let a = deg.to_radians();
        [0.0, speed * a.sin(), -speed * a.cos()]
    }

    #[test]
    fn i051v_an_arc_lands_on_the_ground_where_the_maths_says() {
        let w = floor();
        let hand = [5.5, 11.45, 30.5];
        let v0 = launch(ballistics::launch_speed(6.5), 40.0);
        let l = land(&w, hand, v0, 1.0 / 60.0, 5.0).expect("lands");
        assert_eq!(l.cell[1], 9, "on the floor: {l:?}");
        assert_eq!(l.normal, [0, 1, 0], "from above");
        assert!((l.pos[1] - 10.0).abs() < 0.12, "{l:?}");
        // Same as the flat-ground maths from 1.45 m above the floor.
        let p = bevy::prelude::Vec3::from(hand);
        let v = bevy::prelude::Vec3::from(v0);
        let t = ballistics::landing_time(p, v, 10.0).unwrap();
        let want = ballistics::at(p, v, t);
        assert!(
            (l.pos[2] - want.z).abs() < 0.15,
            "z {} vs {}",
            l.pos[2],
            want.z
        );
        assert!((l.t - t).abs() < 0.05);
    }

    #[test]
    fn i051v_a_wall_in_the_way_stops_the_arc_on_its_face() {
        let mut w = floor();
        // A wall 3 blocks ahead (z = 27), 4 blocks tall.
        for x in 0..12 {
            for y in 10..14 {
                w.set([x, y, 27], Cell::new(mat::STONE));
            }
        }
        let hand = [5.5, 11.45, 30.5];
        let v0 = launch(ballistics::launch_speed(6.5), 20.0);
        let l = land(&w, hand, v0, 1.0 / 60.0, 5.0).expect("lands");
        assert_eq!(l.cell[2], 27, "on the wall: {l:?}");
        assert_eq!(l.normal, [0, 0, 1], "on the face toward the player");
        assert!(l.pos[1] > 10.0, "above the floor: {l:?}");
    }

    #[test]
    fn i051v_an_arc_passes_over_a_gap_and_lands_beyond_it() {
        let mut w = floor();
        // A trench 2 blocks wide, 3 deep, right under the middle of the arc.
        for x in 0..12 {
            for z in 25..27 {
                for y in 7..10 {
                    w.set([x, y, z], Cell::new(mat::AIR));
                }
            }
        }
        let hand = [5.5, 11.45, 30.5];
        let v0 = launch(ballistics::launch_speed(10.0), 45.0);
        let l = land(&w, hand, v0, 1.0 / 60.0, 5.0).expect("lands");
        assert!(l.pos[2] < 25.0, "beyond the trench: {l:?}");
        assert_eq!(l.cell[1], 9, "on the floor past it: {l:?}");
        // Water stops it; air and steam do not.
        assert!(stops(mat::WATER) && stops(mat::STONE) && stops(mat::ICE));
        assert!(!stops(mat::AIR) && !stops(mat::STEAM));
    }
}

/// I-052 round 2 (P1): the workshop fire from the plaza, with the spell
/// physics a player has. Water flies an arc, so a tap aimed level at the
/// fuel store through the door drops onto the floor in front of it; lobbed
/// over the low wall into the open roof, the arc lands on the store, and a
/// few casts put the whole fire out.
#[cfg(test)]
mod workshop_arc {
    use super::*;
    use crate::blockchem::{on_impact, Keys, Matter};

    fn shot(w: &World, target: Vec3, held: f32) -> Option<Arrival> {
        let mut p = crate::player::Player {
            pos: Vec3::new(142.5, 35.0, 197.0),
            ..Default::default()
        };
        let d = (target - p.eye()).normalize();
        p.yaw = (-d.x).atan2(-d.z);
        p.pitch = d.y.asin();
        let hand = p.eye() + p.rotation() * Vec3::new(0.2, -0.15, -0.5);
        plan(Some(w), 1, hand, p.forward(), held).2
    }

    /// The last cast's water still lies on the store (on a slow frame the
    /// sim has had few steps to run it off): the next Water arc goes into
    /// that puddle and lands on the burning block under it, not on the
    /// water's surface. Frost still stops at water (the ford).
    #[test]
    fn a_water_arc_lands_through_a_puddle_on_the_store() {
        let mut w = realgame_voxel::generate(realgame_voxel::DEFAULT_SEED);
        for aim in [Vec3::new(142.5, 40.0, 189.0), Vec3::new(141.5, 40.5, 188.5)] {
            let dry = shot(&w, aim, 0.0).expect("lands");
            let c = dry.cell.unwrap();
            let n = dry.normal.map(i32::from);
            // Water standing on the face the arc comes in by, and above it.
            for k in 1..=2 {
                let q = [c[0] + n[0] * k, c[1] + n[1] * k, c[2] + n[2] * k];
                if w.get(q).mat == mat::AIR {
                    w.set(q, realgame_voxel::Cell::new(mat::WATER));
                }
            }
            let up = [c[0] + n[0], c[1] + n[1] + 1, c[2] + n[2]];
            if w.get(up).mat == mat::AIR {
                w.set(up, realgame_voxel::Cell::new(mat::WATER));
            }
            let wet = shot(&w, aim, 0.0).expect("lands");
            assert_eq!(wet.cell, Some(c), "through the puddle: {dry:?} -> {wet:?}");
        }
        let mut p = crate::player::Player {
            pos: Vec3::new(128.5, 32.0, 159.5),
            ..Default::default()
        };
        p.pitch = -0.6;
        let hand = p.eye() + p.rotation() * Vec3::new(0.2, -0.15, -0.5);
        let frost = plan(Some(&w), 2, hand, p.forward(), 0.0).2.expect("lands");
        assert_eq!(
            w.get(frost.cell.unwrap()).mat,
            mat::WATER,
            "frost stops at the ford's water"
        );
    }

    #[test]
    fn a_lobbed_water_arc_from_the_plaza_puts_the_workshop_out() {
        let mut w = realgame_voxel::generate(realgame_voxel::DEFAULT_SEED);
        let tap = 0.08;
        let level = shot(&w, Vec3::new(142.5, 36.0, 189.0), tap).expect("lands");
        assert_eq!(level.normal, [0, 1, 0], "a level tap lands on the floor");
        assert!(
            level.pos.z > 190.0,
            "in front of the store: {:?}",
            level.pos
        );
        let mut m = Matter::default();
        let mut keys = Keys::default();
        let mut ev = Vec::new();
        let player = Vec3::new(142.5, 35.0, 197.0);
        let mut now = 0.0;
        let mut run = |m: &mut Matter, w: &mut World, secs: f32, now: &mut f32| {
            for _ in 0..(secs / 0.1).round() as usize {
                *now += 0.1;
                m.tick(w, *now, 0.1, player, &mut keys, &mut ev);
            }
            crate::blockchem::workshop_burning(w)
        };
        // Minutes in, as when a player gets there.
        assert!(run(&mut m, &mut w, 240.0, &mut now) >= 6);
        // Front middle, back left, back right: what a player sees burning.
        let aims = [
            Vec3::new(142.5, 40.0, 189.0),
            Vec3::new(141.5, 40.5, 188.5),
            Vec3::new(143.5, 40.5, 188.5),
        ];
        // A click with no hold lands there too (a worked cast's hold).
        for a in aims {
            let l = shot(&w, a, 0.0).expect("lands");
            assert!(l.pos.z < 190.01 && l.pos.z > 187.9, "no hold: {l:?}");
        }
        let mut casts = 0;
        let mut left = crate::blockchem::workshop_burning(&w);
        while left > 0 && casts < 6 {
            let a = shot(&w, aims[casts % 3], tap).expect("lands");
            assert!(
                (141..=143).contains(&a.cell.unwrap()[0]) && a.pos.z < 190.01,
                "cast {casts} lands on the store: {a:?}"
            );
            m.world.emit(on_impact(
                Spell::Water,
                a.pos,
                IVec3::new(a.normal[0].into(), a.normal[1].into(), a.normal[2].into()),
            ));
            casts += 1;
            left = run(&mut m, &mut w, 1.0, &mut now);
        }
        assert_eq!(left, 0, "out after {casts} lobbed casts");
        assert_eq!(run(&mut m, &mut w, 30.0, &mut now), 0, "and it stays out");
    }
}
