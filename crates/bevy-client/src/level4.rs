//! Level 4, the Crystal Lab (I-036): a stone lab hall the size of the
//! courtyard. A partition wall crosses the hall with one doorway, blocked by
//! a heavy iron crate with a limp balloon tied to it. A hose runs from the
//! balloon back to the water splitter on a bench: Water cast into it is split
//! by electricity into hydrogen and oxygen (the hydrogen tube always fills
//! twice as high as the oxygen tube), and the hydrogen goes up the hose into
//! the balloon. Full, the balloon floats up and lifts the crate onto a ledge,
//! and the way to the lantern is open. Fire on the balloon pops its hydrogen.
//!
//! Layout (metres, y up; the player starts at z = +18 looking toward -z):
//! the back arch to the Marsh is behind the start at (0, 22.6); the splitter
//! stands at (-4, 8); the partition is at z = -4 with its doorway at x = 0;
//! the lantern stands behind it at (0, -16).
//!
//! [`layout`] is the pure part (colliders and chemistry objects, unit
//! tested); [`build`] adds the visuals and [`sync`] moves them. Player text
//! for level 4 is [`caption`]: one line per (event, band), registered in
//! `content/science-sources.md`.

use crate::chem::{self, electrolysis, EventKind, Fizzle, Goal, Kind, Obj};
use crate::fx::{Fx, P};
use crate::world::{self, Aabb, Builder, Chem, Collider, Colliders, Kit, LevelRoot, StateMats};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use realgame_sim::tier::Tier;

/// Where the player arrives (feet), facing -z.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 18.0);
/// The arch back to the Marsh (same place as the other levels' arch).
pub const BACK_ARCH: (f32, f32) = (0.0, 22.6);
/// The lantern's centre (x, z): walking within [`LANTERN_REACH`] wins.
pub const LANTERN: (f32, f32) = (0.0, -16.0);
pub const LANTERN_REACH: f32 = 1.5;
/// The partition wall's centre line (z) and the doorway's half width.
pub const WALL_Z: f32 = -4.0;
pub const DOOR_HALF: f32 = 1.2;
/// Half thickness of the partition wall.
const WALL_HALF_Z: f32 = 0.5;
/// The water splitter's bench centre (x, z) and half extents.
pub const SPLITTER_AT: (f32, f32) = (-4.0, 8.0);
const BENCH_HALF: Vec3 = Vec3::new(1.2, 0.5, 0.55);
/// The crate in the doorway: half extents (it fills the doorway, above the
/// highest jump of 1.0 m).
const CRATE_HALF: Vec3 = Vec3::new(1.25, 1.1, 0.5);
/// The ledge on the wall's south face where the balloon sets the crate down:
/// its centre x, its top y.
const LEDGE_X: f32 = 3.6;
const LEDGE_TOP: f32 = 2.6;
/// How long the balloon takes to carry the crate onto the ledge, seconds
/// (display only).
const LIFT_S: f32 = 3.2;

/// Object indices in level 4's chemistry world.
pub const SPLITTER: usize = 0;
pub const BALLOON: usize = 1;
pub const CRATE: usize = 2;
/// Target names (golden finds targets by name).
pub const NAMES: [&str; 4] = ["water splitter", "balloon", "crate", "lantern"];

/// Level 4's colliders, chemistry objects and picking boxes, without any
/// visuals.
pub struct Layout {
    pub colliders: Vec<Collider>,
    pub world: chem::World,
    /// Picking box per object.
    pub boxes: Vec<Aabb>,
}

fn solid(colliders: &mut Vec<Collider>, center: Vec3, half: Vec3) {
    colliders.push(Collider {
        aabb: Aabb::new(center, half),
        obj: None,
    });
}

/// The lab's colliders and chemistry. Pure: [`build`] and the tests use it.
pub fn layout() -> Layout {
    let mut c: Vec<Collider> = Vec::new();
    let h = world::HALF;
    let wh = world::WALL_H;
    // Perimeter walls (the same boxes as the courtyard's).
    let wl = 2.0 * h + 2.0;
    for (center, half) in [
        (
            Vec3::new(0.0, wh / 2.0, -h - 0.5),
            Vec3::new(wl / 2.0, wh / 2.0, 0.5),
        ),
        (
            Vec3::new(0.0, wh / 2.0, h + 0.5),
            Vec3::new(wl / 2.0, wh / 2.0, 0.5),
        ),
        (
            Vec3::new(-h - 0.5, wh / 2.0, 0.0),
            Vec3::new(0.5, wh / 2.0, wl / 2.0),
        ),
        (
            Vec3::new(h + 0.5, wh / 2.0, 0.0),
            Vec3::new(0.5, wh / 2.0, wl / 2.0),
        ),
    ] {
        solid(&mut c, center, half);
    }
    // The partition across the hall, either side of the doorway.
    for side in [-1.0f32, 1.0] {
        let half_x = (h - DOOR_HALF) / 2.0;
        solid(
            &mut c,
            Vec3::new(side * (DOOR_HALF + half_x), wh / 2.0, WALL_Z),
            Vec3::new(half_x, wh / 2.0, WALL_HALF_Z),
        );
    }
    // The ledge on the wall's south face (well above head height).
    solid(
        &mut c,
        Vec3::new(LEDGE_X, LEDGE_TOP - 0.1, WALL_Z + WALL_HALF_Z + 0.6),
        Vec3::new(1.4, 0.1, 0.6),
    );
    // The lantern's plinth and post.
    let (lx, lz) = LANTERN;
    solid(&mut c, Vec3::new(lx, 1.1, lz), Vec3::new(0.4, 1.1, 0.4));

    // Chemistry: the splitter, the balloon, the crate, the lantern.
    let (sx, sz) = SPLITTER_AT;
    let crate_box = Aabb::new(Vec3::new(0.0, CRATE_HALF.y, WALL_Z), CRATE_HALF);
    // The balloon's picking box covers where it floats over the doorway and
    // where it ends up over the ledge (south of the wall, over head height).
    let balloon_box = Aabb {
        min: Vec3::new(-1.6, 2.3, WALL_Z - 0.6),
        max: Vec3::new(LEDGE_X + 1.5, 6.0, WALL_Z + WALL_HALF_Z + 1.6),
    };
    let boxes = vec![
        // The bench with the battery and the two tubes on it.
        Aabb {
            min: Vec3::new(sx - BENCH_HALF.x, 0.0, sz - BENCH_HALF.z),
            max: Vec3::new(
                sx + BENCH_HALF.x,
                electrolysis::TUBE_Y + electrolysis::TUBE_H / 2.0,
                sz + BENCH_HALF.z,
            ),
        },
        balloon_box,
        crate_box,
        Aabb::new(Vec3::new(lx, 1.0, lz), Vec3::new(0.45, 1.0, 0.45)),
    ];
    let foot = |a: &Aabb| {
        let (cc, half) = (a.center(), (a.max - a.min) * 0.5);
        ([cc.x, cc.z], [half.x, half.z])
    };
    let kinds = [Kind::Splitter, Kind::Balloon, Kind::Crate, Kind::Lantern];
    let objs: Vec<Obj> = kinds
        .iter()
        .zip(&boxes)
        .map(|(&k, a)| {
            let (p, half) = foot(a);
            Obj::new(k, p, half)
        })
        .collect();
    // The bench and the crate collide (the crate only while it is down).
    c.push(Collider {
        aabb: Aabb::new(Vec3::new(sx, BENCH_HALF.y, sz), BENCH_HALF),
        obj: Some(SPLITTER),
    });
    c.push(Collider {
        aabb: crate_box,
        obj: Some(CRATE),
    });
    Layout {
        colliders: c,
        world: chem::World::with_goal(objs, Goal::Reach),
        boxes,
    }
}

/// How full of hydrogen the balloon is (`balloon`); 0 outside level 4.
pub fn balloon(w: &chem::World) -> f32 {
    w.objs
        .iter()
        .find(|o| o.kind == Kind::Balloon)
        .map_or(0.0, |o| o.gas)
}

/// Whether the crate is on the ledge (`crateUp`).
pub fn crate_up(w: &chem::World) -> bool {
    w.objs.iter().any(|o| o.kind == Kind::Crate && o.lifted)
}

/// The H2 and O2 gas columns shown in the splitter's tubes (`tubes`, metres);
/// `[0, 0]` outside level 4 or between splits.
pub fn tubes(w: &chem::World) -> [f32; 2] {
    let run = w
        .objs
        .iter()
        .find(|o| o.kind == Kind::Splitter)
        .map_or(0.0, |o| o.run);
    let (h2, o2) = electrolysis::column(run);
    [h2, o2]
}

/// Level 4's caption for an event from a spell that hit `obj`, at the
/// player's band (`None` reads as Apprentice), with the Sprout picture
/// (`"gas"`, `"wrong"`, or `""`). `None` for events level 4 has no line for
/// (the engine's caption is used).
pub fn caption(
    kind: EventKind,
    obj: Kind,
    band: Option<Tier>,
) -> Option<(&'static str, &'static str)> {
    use EventKind as E;
    // 0 Sprout, 1 Seedling, 2 Apprentice (and no band), 3 Adept.
    let k = match band {
        Some(Tier::Sprout) => 0,
        Some(Tier::Seedling) => 1,
        Some(Tier::Apprentice) | None => 2,
        Some(Tier::Adept) => 3,
    };
    let pick = |lines: [&'static str; 4], icon: &'static str| {
        Some((lines[k], if k == 0 { icon } else { "" }))
    };
    // Sprout's line, then the one line every older band shares.
    let two = |sprout: &'static str, rest: &'static str, icon: &'static str| {
        pick([sprout, rest, rest, rest], icon)
    };
    match (kind, obj) {
        (E::Split, Kind::Splitter) => pick(
            [
                "Zap! The water splits into two gases. More water!",
                "Electricity splits water into hydrogen and oxygen. Twice as much hydrogen comes out as oxygen.",
                E::Split.caption(),
                "Electrolysis: 2H₂O(l) → 2H₂(g) + O₂(g). Two moles of H₂ for every mole of O₂, and equal moles of gas fill equal volumes, so the gas columns are 2:1.",
            ],
            "gas",
        ),
        (E::Lifted, Kind::Splitter) => pick(
            [
                "Up it goes! The light gas lifts the crate.",
                "Hydrogen is much lighter than air, so the full balloon floats up and lifts the crate.",
                E::Lifted.caption(),
                "Buoyancy: the air the balloon pushes aside weighs more than the balloon, its H₂ and the crate together, so they rise.",
            ],
            "gas",
        ),
        (E::Popped, Kind::Balloon) => pick(
            [
                "Pop! Hydrogen burns with a bang.",
                "Squeaky pop! That is the test for hydrogen: it burns with oxygen in the air and makes water.",
                E::Popped.caption(),
                "Hydrogen burns in air across a wide range of mixtures (4 to 74 %), which is why hydrogen balloons were a fire risk.",
            ],
            "",
        ),
        (E::Fizzle(Fizzle::NotFlammable) | E::Chilled | E::Smothered, Kind::Splitter) => two(
            "The machine needs water. Try Water!",
            "Electricity splits water, so the splitter needs water. Cast Water into it.",
            "wrong",
        ),
        (E::NoChange, Kind::Balloon) => two(
            "Too heavy! That gas will not float.",
            "CO₂ is heavier than air, so it would weigh the balloon down. It needs a light gas: hydrogen.",
            "wrong",
        ),
        (E::Soaked | E::Chilled, Kind::Balloon) => two(
            "The balloon needs a light gas. Make some with water!",
            "The balloon needs a gas lighter than air. Split water to make hydrogen.",
            "wrong",
        ),
        (E::Fizzle(Fizzle::NotFlammable), Kind::Balloon) => two(
            "Nothing to burn yet!",
            "The balloon is empty: no hydrogen to burn yet.",
            "wrong",
        ),
        (_, Kind::Crate) => two(
            "Too heavy to push. Fill the balloon!",
            "The crate is too heavy to move. A balloon full of hydrogen can lift it.",
            "wrong",
        ),
        // Any spell on the lantern: it is the goal, not a target.
        (_, Kind::Lantern) => two(
            "That is the lantern. Walk up to it!",
            "That is the lantern. Walk up to it!",
            "",
        ),
        _ => None,
    }
}

/// The crate's body; `t` is the time since it started to rise (display).
#[derive(Component)]
pub struct CrateBody {
    pub t: f32,
}

/// The balloon (its size follows the hydrogen in it) and its string.
#[derive(Component)]
pub struct BalloonBody {
    pub string: bool,
}

/// A gas column in one of the splitter's tubes (`h2`: the hydrogen tube).
#[derive(Component)]
pub struct TubeGas {
    pub h2: bool,
}

/// Where the crate's centre is `t` seconds after the balloon started to
/// lift it: up out of the doorway, across to the ledge, then down onto it.
pub fn crate_path(t: f32) -> Vec3 {
    let down = Vec3::new(0.0, CRATE_HALF.y, WALL_Z);
    let high = WALL_Z + WALL_HALF_Z + 0.6;
    let top = world::WALL_H + 1.4;
    let rest = Vec3::new(LEDGE_X, LEDGE_TOP + CRATE_HALF.y, high);
    let f = |a: f32, b: f32| ((t - a) / (b - a)).clamp(0.0, 1.0);
    let ease = |x: f32| x * x * (3.0 - 2.0 * x);
    let up = ease(f(0.0, 0.45 * LIFT_S));
    let across = ease(f(0.45 * LIFT_S, 0.8 * LIFT_S));
    let land = ease(f(0.8 * LIFT_S, LIFT_S));
    let y = down.y + (top - down.y) * up - (top - rest.y) * land;
    Vec3::new(
        down.x + (rest.x - down.x) * across,
        y,
        down.z + (rest.z - down.z) * across,
    )
}

/// The balloon's radius for a hydrogen fill `gas` (limp at 0).
fn balloon_r(gas: f32) -> f32 {
    0.25 + 0.75 * gas.clamp(0.0, 1.0)
}

/// Spawns the lab. Every entity carries [`LevelRoot`].
pub fn build(
    commands: &mut Commands,
    kit: &Kit,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> (Colliders, Chem) {
    let lay = layout();
    let h = world::HALF;
    let wh = world::WALL_H;
    let stone_tex = world::stone_texture(images);
    let floor = world::textured(
        materials,
        Color::srgb(0.55, 0.57, 0.62),
        &stone_tex,
        Vec2::splat(20.0),
    );
    let wall_long = world::textured(
        materials,
        Color::srgb(0.66, 0.66, 0.74),
        &stone_tex,
        Vec2::new(25.0, 2.0),
    );
    let wall_cap = world::toon(materials, Color::srgb(0.46, 0.44, 0.56));
    let stone = world::toon(materials, Color::srgb(0.5, 0.5, 0.58));
    let wood = world::toon(materials, Color::srgb(0.55, 0.38, 0.24));
    let iron = world::toon(materials, Color::srgb(0.36, 0.38, 0.42));
    let iron_band = world::toon(materials, Color::srgb(0.22, 0.23, 0.26));
    let battery = world::toon(materials, Color::srgb(0.15, 0.17, 0.2));
    let red = world::toon(materials, Color::srgb(0.85, 0.2, 0.2));
    let copper = world::toon(materials, Color::srgb(0.8, 0.5, 0.3));
    let hose = world::toon(materials, Color::srgb(0.3, 0.32, 0.3));
    let rope = world::toon(materials, Color::srgb(0.85, 0.82, 0.7));
    let lamp = world::glow(materials, Color::srgb(1.0, 0.82, 0.4), 6.0);
    let crystal_a = world::glow(materials, Color::srgb(0.62, 0.45, 1.0), 2.2);
    let crystal_b = world::glow(materials, Color::srgb(0.35, 0.85, 1.0), 2.2);
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.95, 1.0, 0.25),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        reflectance: 0.6,
        ..default()
    });
    let water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.35, 0.6, 0.95, 0.7),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });
    // The gases are colourless; the pale columns are a game cue.
    let gas = materials.add(StandardMaterial {
        base_color: Color::srgba(0.96, 0.98, 1.0, 0.55),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.4, 0.35),
        perceptual_roughness: 0.35,
        reflectance: 0.4,
        ..default()
    });

    let mut b = Builder {
        commands,
        kit: kit.clone(),
        colliders: Vec::new(),
    };
    let cube = kit.cube.clone();
    let ball = kit.ball.clone();
    let cyl = kit.cyl.clone();
    let cone = kit.cone.clone();

    // Floor.
    b.piece(
        &cube,
        &floor,
        Transform::from_xyz(0.0, -0.25, 0.0).with_scale(Vec3::new(
            2.0 * h + 2.0,
            0.5,
            2.0 * h + 2.0,
        )),
    );
    // Perimeter walls with caps (colliders come from `layout`).
    let wl = 2.0 * h + 2.0;
    for (c, turn) in [
        (Vec3::new(0.0, wh / 2.0, -h - 0.5), false),
        (Vec3::new(0.0, wh / 2.0, h + 0.5), false),
        (Vec3::new(-h - 0.5, wh / 2.0, 0.0), true),
        (Vec3::new(h + 0.5, wh / 2.0, 0.0), true),
    ] {
        let rot = Quat::from_rotation_y(if turn {
            std::f32::consts::FRAC_PI_2
        } else {
            0.0
        });
        b.piece(
            &cube,
            &wall_long,
            Transform::from_translation(c)
                .with_rotation(rot)
                .with_scale(Vec3::new(wl, wh, 1.0)),
        );
        b.piece(
            &cube,
            &wall_cap,
            Transform::from_translation(Vec3::new(c.x, wh + 0.15, c.z))
                .with_rotation(rot)
                .with_scale(Vec3::new(wl, 0.3, 1.4)),
        );
    }
    // The partition, its doorposts and the ledge.
    for side in [-1.0f32, 1.0] {
        let half_x = (h - DOOR_HALF) / 2.0;
        b.piece(
            &cube,
            &wall_long,
            Transform::from_xyz(side * (DOOR_HALF + half_x), wh / 2.0, WALL_Z)
                .with_scale(Vec3::new(2.0 * half_x, wh, 2.0 * WALL_HALF_Z)),
        );
        b.piece(
            &cube,
            &stone,
            Transform::from_xyz(side * (DOOR_HALF + 0.3), (wh + 0.6) / 2.0, WALL_Z)
                .with_scale(Vec3::new(0.6, wh + 0.6, 2.0 * WALL_HALF_Z + 0.3)),
        );
    }
    b.piece(
        &cube,
        &stone,
        Transform::from_xyz(LEDGE_X, LEDGE_TOP - 0.1, WALL_Z + WALL_HALF_Z + 0.6)
            .with_scale(Vec3::new(2.8, 0.2, 1.2)),
    );
    for dx in [-1.0f32, 1.0] {
        b.piece(
            &cube,
            &stone,
            Transform::from_xyz(LEDGE_X + dx, LEDGE_TOP - 0.45, WALL_Z + WALL_HALF_Z + 0.25)
                .with_rotation(Quat::from_rotation_x(0.6))
                .with_scale(Vec3::new(0.25, 0.7, 0.25)),
        );
    }

    // The way back: the arch, with a blue flame of marsh gas on its lintel.
    world::arch(&mut b, materials, &stone);
    {
        let (x, z) = BACK_ARCH;
        let blue = world::glow(materials, Color::srgb(0.3, 0.55, 1.0), 4.0);
        b.piece(
            &cone,
            &blue,
            Transform::from_xyz(x, 4.3, z + 0.4).with_scale(Vec3::new(0.4, 0.7, 0.4)),
        );
    }

    // Glowing crystals along the walls (the lab's name).
    for (k, (x, z, s)) in [
        (-21.0f32, 18.0f32, 1.2f32),
        (21.5, 14.0, 1.0),
        (-22.0, 4.0, 1.4),
        (22.0, -1.0, 1.1),
        (-21.5, -9.0, 1.0),
        (21.0, -13.0, 1.3),
        (-12.0, -22.0, 1.2),
        (13.0, -21.5, 1.0),
        (-9.0, 22.0, 0.9),
        (10.0, 21.5, 1.1),
    ]
    .into_iter()
    .enumerate()
    {
        let m = if k % 2 == 0 { &crystal_a } else { &crystal_b };
        for (dx, tilt, sc) in [
            (0.0f32, 0.0f32, 1.0f32),
            (0.6, 0.35, 0.6),
            (-0.5, -0.3, 0.7),
        ] {
            b.piece(
                &cone,
                m,
                Transform::from_xyz(x + dx * s, 0.9 * s * sc, z)
                    .with_rotation(Quat::from_rotation_z(tilt))
                    .with_scale(Vec3::new(0.5 * s * sc, 1.8 * s * sc, 0.5 * s * sc)),
            );
        }
    }

    // The water splitter: a bench, a battery with its terminals and wires,
    // and two glass tubes standing in a water trough. The hydrogen tube is on
    // the left (the negative side), the oxygen tube on the right.
    let (sx, sz) = SPLITTER_AT;
    b.piece(
        &cube,
        &wood,
        Transform::from_xyz(sx, 0.95, sz).with_scale(Vec3::new(
            2.0 * BENCH_HALF.x,
            0.1,
            2.0 * BENCH_HALF.z,
        )),
    );
    for (dx, dz) in [(-1.0f32, -0.4f32), (1.0, -0.4), (-1.0, 0.4), (1.0, 0.4)] {
        b.piece(
            &cube,
            &wood,
            Transform::from_xyz(sx + dx, 0.45, sz + dz).with_scale(Vec3::new(0.12, 0.9, 0.12)),
        );
    }
    b.piece(
        &cube,
        &battery,
        Transform::from_xyz(sx, 1.2, sz + 0.25).with_scale(Vec3::new(0.5, 0.4, 0.3)),
    );
    b.piece(
        &cyl,
        &red,
        Transform::from_xyz(sx + 0.15, 1.45, sz + 0.25).with_scale(Vec3::new(0.08, 0.1, 0.08)),
    );
    b.piece(
        &cyl,
        &iron_band,
        Transform::from_xyz(sx - 0.15, 1.45, sz + 0.25).with_scale(Vec3::new(0.08, 0.1, 0.08)),
    );
    b.piece(
        &cube,
        &water,
        Transform::from_xyz(sx, 1.08, sz - 0.1).with_scale(Vec3::new(1.6, 0.16, 0.5)),
    );
    let tube_x = [sx - 0.55, sx + 0.55];
    for (k, &x) in tube_x.iter().enumerate() {
        let t = electrolysis::tube(0.0, 0.0);
        let z = sz - 0.1;
        b.piece(
            &cyl,
            &glass,
            Transform::from_xyz(x, t.glass.y, z)
                .with_scale(Vec3::new(t.glass.d, t.glass.h, t.glass.d)),
        );
        b.piece(
            &cyl,
            &water,
            Transform::from_xyz(x, t.water.y, z)
                .with_scale(Vec3::new(t.water.d, t.water.h, t.water.d)),
        );
        b.commands.spawn((
            Mesh3d(cyl.clone()),
            MeshMaterial3d(gas.clone()),
            Transform::from_xyz(x, t.gas.y, z).with_scale(Vec3::new(t.gas.d, 0.001, t.gas.d)),
            Visibility::Hidden,
            NotShadowCaster,
            TubeGas { h2: k == 0 },
            LevelRoot,
        ));
        // The wire from the battery to the electrode under the tube.
        b.piece(
            &cube,
            &copper,
            Transform::from_xyz((x + sx) / 2.0, 1.2, sz + 0.05).with_scale(Vec3::new(
                (x - sx).abs(),
                0.03,
                0.03,
            )),
        );
    }
    // The hose: from the top of the hydrogen tube, along the floor, up to the
    // balloon over the doorway.
    let top = electrolysis::TUBE_Y + electrolysis::TUBE_H / 2.0;
    let pts = [
        Vec3::new(tube_x[0], top + 0.1, sz - 0.1),
        Vec3::new(tube_x[0], top + 0.4, sz - 0.1),
        Vec3::new(-1.6, 0.06, sz - 1.0),
        Vec3::new(-1.6, 0.06, WALL_Z + 1.2),
        Vec3::new(-0.6, 2.2, WALL_Z + 0.6),
    ];
    for w in pts.windows(2) {
        let (a, c) = (w[0], w[1]);
        let d = c - a;
        b.piece(
            &cyl,
            &hose,
            Transform::from_translation((a + c) / 2.0)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize()))
                .with_scale(Vec3::new(0.07, d.length(), 0.07)),
        );
    }

    // The lantern on its plinth, behind the wall.
    let (lx, lz) = LANTERN;
    b.piece(
        &cube,
        &stone,
        Transform::from_xyz(lx, 0.5, lz).with_scale(Vec3::new(0.8, 1.0, 0.8)),
    );
    b.piece(
        &cube,
        &wood,
        Transform::from_xyz(lx, 1.08, lz).with_scale(Vec3::new(0.5, 0.16, 0.5)),
    );
    let light = b.piece(
        &ball,
        &lamp,
        Transform::from_xyz(lx, 1.45, lz).with_scale(Vec3::splat(0.55)),
    );
    b.piece(
        &cone,
        &wood,
        Transform::from_xyz(lx, 1.9, lz).with_scale(Vec3::new(0.6, 0.35, 0.6)),
    );

    // --- Chemistry targets -------------------------------------------------
    let plain = |m: &Handle<StandardMaterial>| StateMats {
        normal: m.clone(),
        hot: m.clone(),
        burning: m.clone(),
        charred: m.clone(),
        wet: m.clone(),
        frosty: m.clone(),
        foamed: m.clone(),
    };
    let mut targets = Vec::new();
    for (i, o) in lay.world.objs.iter().enumerate() {
        let mut t = world::Target {
            name: NAMES[i],
            aabb: lay.boxes[i],
            melted_aabb: None,
            bodies: vec![],
            flames: vec![],
            foam: None,
            wall: None,
            puddle: None,
            mats: plain(&stone),
            last: None,
        };
        match o.kind {
            Kind::Crate => {
                // The iron crate and its bands move together (one parent).
                let at = crate_path(0.0);
                b.commands
                    .spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(iron.clone()),
                        Transform::from_translation(at).with_scale(2.0 * CRATE_HALF),
                        CrateBody { t: 0.0 },
                        LevelRoot,
                    ))
                    .with_children(|p| {
                        for y in [-0.42f32, 0.0, 0.42] {
                            p.spawn((
                                Mesh3d(cube.clone()),
                                MeshMaterial3d(iron_band.clone()),
                                Transform::from_xyz(0.0, y, 0.0)
                                    .with_scale(Vec3::new(1.02, 0.06, 1.04)),
                            ));
                        }
                    });
            }
            Kind::Balloon => {
                b.commands.spawn((
                    Mesh3d(ball.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_translation(crate_path(0.0) + Vec3::Y * 2.0)
                        .with_scale(Vec3::splat(2.0 * balloon_r(0.0))),
                    BalloonBody { string: false },
                    LevelRoot,
                ));
                b.commands.spawn((
                    Mesh3d(cyl.clone()),
                    MeshMaterial3d(rope.clone()),
                    Transform::from_translation(crate_path(0.0) + Vec3::Y * 1.4)
                        .with_scale(Vec3::new(0.03, 0.6, 0.03)),
                    NotShadowCaster,
                    BalloonBody { string: true },
                    LevelRoot,
                ));
            }
            Kind::Lantern => {
                t.bodies = vec![light];
                t.mats = plain(&lamp);
            }
            _ => {}
        }
        targets.push(t);
    }

    let mut colliders = lay.colliders;
    colliders.append(&mut b.colliders);
    (
        Colliders(colliders),
        Chem {
            initial: lay.world.clone(),
            world: lay.world,
            targets,
        },
    )
}

/// Level 4's moving parts each frame: the crate's ride onto the ledge (and
/// its picking box), the
/// balloon (its size follows the hydrogen; it rides on the crate), the gas
/// columns in the tubes while a split runs, and bubbles rising in them.
#[allow(clippy::too_many_arguments)]
pub fn sync<
    F: bevy::ecs::query::QueryFilter,
    G: bevy::ecs::query::QueryFilter,
    H: bevy::ecs::query::QueryFilter,
>(
    chem: &mut Chem,
    dt: f32,
    low: bool,
    crates: &mut Query<(&mut Transform, &mut CrateBody), F>,
    balloons: &mut Query<(&mut Transform, &BalloonBody), G>,
    tubes_q: &mut Query<(&mut Transform, &mut Visibility, &TubeGas), H>,
    fx: &mut Fx,
) {
    // The crate's picking box follows it: in the doorway while it is down,
    // on the ledge once lifted (so casts through the open doorway pass).
    let up = crate_up(&chem.world);
    if let Some(t) = chem.targets.get_mut(CRATE) {
        let want = Aabb::new(
            if up {
                crate_path(LIFT_S)
            } else {
                crate_path(0.0)
            },
            CRATE_HALF,
        );
        if t.aabb.min != want.min {
            t.aabb = want;
        }
    }
    let w = &chem.world;
    let mut at = crate_path(0.0);
    for (mut tr, mut body) in crates.iter_mut() {
        body.t = if up { (body.t + dt).min(LIFT_S) } else { 0.0 };
        at = crate_path(body.t);
        if tr.translation.distance(at) > 1e-4 {
            tr.translation = at;
        }
    }
    let r = balloon_r(balloon(w));
    let lift = CRATE_HALF.y + 0.9;
    for (mut tr, part) in balloons.iter_mut() {
        if part.string {
            tr.translation = at + Vec3::Y * (CRATE_HALF.y + 0.45);
        } else {
            tr.translation = at + Vec3::Y * (lift + r);
            tr.scale = Vec3::splat(2.0 * r);
        }
    }
    let [h2, o2] = tubes(w);
    let t = electrolysis::tube(0.0, 0.0);
    let inner_top = t.glass.y + t.glass.h / 2.0 - electrolysis::INSET;
    for (mut tr, mut vis, g) in tubes_q.iter_mut() {
        let hgt = if g.h2 { h2 } else { o2 };
        let want = if hgt > 0.002 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        tr.scale.y = hgt.max(0.001);
        tr.translation.y = inner_top - hgt / 2.0;
    }
    // Bubbles in each tube while the split runs (twice as many on the
    // hydrogen side).
    let run = w
        .objs
        .iter()
        .find(|o| o.kind == Kind::Splitter)
        .map_or(0.0, |o| o.run);
    if run > 0.4 * chem::SPLIT_S {
        let (sx, sz) = SPLITTER_AT;
        let rate = if low { 6.0 } else { 12.0 };
        for (x, k) in [(sx - 0.55, 2.0f32), (sx + 0.55, 1.0)] {
            if fx.rand() < rate * k * dt {
                let p = Vec3::new(x + (fx.rand() - 0.5) * 0.1, t.glass.y - 0.4, sz - 0.1);
                fx.emit(P::Foam, p, Vec3::new(0.0, 0.8, 0.0), 0.04, 0.6);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chem::{Fizzle, Spell};
    use crate::player::{MoveInput, Player};
    use crate::world::Colliders;

    // ---- I-036 acceptance test 6: the caption table ----

    const SPROUT: Option<Tier> = Some(Tier::Sprout);
    const SEEDLING: Option<Tier> = Some(Tier::Seedling);
    const APPRENTICE: Option<Tier> = Some(Tier::Apprentice);
    const ADEPT: Option<Tier> = Some(Tier::Adept);

    /// (event, objects it applies to, [Sprout, Seedling, Apprentice, Adept],
    /// Sprout icon), exactly as the spec's table.
    fn table() -> Vec<(EventKind, Vec<Kind>, [&'static str; 4], &'static str)> {
        let all = |s: &'static str| [s, s, s, s];
        let rest = |sprout: &'static str, r: &'static str| [sprout, r, r, r];
        let needs_water = rest(
            "The machine needs water. Try Water!",
            "Electricity splits water, so the splitter needs water. Cast Water into it.",
        );
        let light_gas = rest(
            "The balloon needs a light gas. Make some with water!",
            "The balloon needs a gas lighter than air. Split water to make hydrogen.",
        );
        let lantern = all("That is the lantern. Walk up to it!");
        vec![
            (
                EventKind::Split,
                vec![Kind::Splitter],
                [
                    "Zap! The water splits into two gases. More water!",
                    "Electricity splits water into hydrogen and oxygen. Twice as much hydrogen comes out as oxygen.",
                    "2H₂O → 2H₂ + O₂: the hydrogen tube fills twice as fast as the oxygen tube.",
                    "Electrolysis: 2H₂O(l) → 2H₂(g) + O₂(g). Two moles of H₂ for every mole of O₂, and equal moles of gas fill equal volumes, so the gas columns are 2:1.",
                ],
                "gas",
            ),
            (
                EventKind::Lifted,
                vec![Kind::Splitter],
                [
                    "Up it goes! The light gas lifts the crate.",
                    "Hydrogen is much lighter than air, so the full balloon floats up and lifts the crate.",
                    "H₂ (2 g/mol) is about 14 times lighter than air (29 g/mol). The balloon floats, and the crate goes with it.",
                    "Buoyancy: the air the balloon pushes aside weighs more than the balloon, its H₂ and the crate together, so they rise.",
                ],
                "gas",
            ),
            (
                EventKind::Popped,
                vec![Kind::Balloon],
                [
                    "Pop! Hydrogen burns with a bang.",
                    "Squeaky pop! That is the test for hydrogen: it burns with oxygen in the air and makes water.",
                    "Pop! 2H₂ + O₂ → 2H₂O. Hydrogen is flammable, so the balloon burst. Split more water to fill it again.",
                    "Hydrogen burns in air across a wide range of mixtures (4 to 74 %), which is why hydrogen balloons were a fire risk.",
                ],
                "",
            ),
            (
                EventKind::Fizzle(Fizzle::NotFlammable),
                vec![Kind::Splitter],
                needs_water,
                "wrong",
            ),
            (EventKind::Chilled, vec![Kind::Splitter], needs_water, "wrong"),
            (EventKind::Smothered, vec![Kind::Splitter], needs_water, "wrong"),
            (
                EventKind::NoChange,
                vec![Kind::Balloon],
                rest(
                    "Too heavy! That gas will not float.",
                    "CO₂ is heavier than air, so it would weigh the balloon down. It needs a light gas: hydrogen.",
                ),
                "wrong",
            ),
            (EventKind::Soaked, vec![Kind::Balloon], light_gas, "wrong"),
            (EventKind::Chilled, vec![Kind::Balloon], light_gas, "wrong"),
            (
                EventKind::Fizzle(Fizzle::NotFlammable),
                vec![Kind::Balloon],
                rest(
                    "Nothing to burn yet!",
                    "The balloon is empty: no hydrogen to burn yet.",
                ),
                "wrong",
            ),
            (
                EventKind::NoChange,
                vec![Kind::Crate],
                rest(
                    "Too heavy to push. Fill the balloon!",
                    "The crate is too heavy to move. A balloon full of hydrogen can lift it.",
                ),
                "wrong",
            ),
            (EventKind::Soaked, vec![Kind::Lantern], lantern, ""),
            (EventKind::Ignited, vec![Kind::Lantern], lantern, ""),
        ]
    }

    #[test]
    fn i036_caption_table_is_exact_per_band() {
        for (k, objs, lines, icon) in table() {
            for obj in objs {
                for (band, want) in [SPROUT, SEEDLING, APPRENTICE, ADEPT].into_iter().zip(lines) {
                    let got = caption(k, obj, band);
                    let icon = if band == SPROUT { icon } else { "" };
                    assert_eq!(got, Some((want, icon)), "{k:?} on {obj:?} at {band:?}");
                }
                assert_eq!(
                    caption(k, obj, None),
                    caption(k, obj, APPRENTICE),
                    "no band reads as Apprentice ({k:?} on {obj:?})"
                );
            }
        }
        // The engine's own captions are the Apprentice lines.
        for k in [EventKind::Split, EventKind::Lifted, EventKind::Popped] {
            let obj = if k == EventKind::Popped {
                Kind::Balloon
            } else {
                Kind::Splitter
            };
            assert_eq!(caption(k, obj, APPRENTICE).map(|c| c.0), Some(k.caption()));
        }
    }

    #[test]
    fn i036_caption_strings_are_safe_player_text() {
        for (k, objs, _, _) in table() {
            for obj in objs {
                for band in [None, SPROUT, SEEDLING, APPRENTICE, ADEPT] {
                    let (text, _) = caption(k, obj, band).expect("a line");
                    assert!(!text.is_empty() && text.len() <= 200, "{text}");
                    assert!(
                        !text
                            .chars()
                            .any(|c| c == '"' || c == '\\' || c.is_control()),
                        "{text:?}"
                    );
                }
            }
        }
    }

    // ---- I-036 acceptance test 7: the walk to the lantern ----

    fn walk(w: &chem::World, cols: &[Collider], from: Vec3, wish: Vec2, secs: f32) -> Player {
        let colliders = Colliders(cols.to_vec());
        let solids = crate::player::boxes(colliders.solid(w));
        let mut p = Player {
            pos: from,
            ..Player::default()
        };
        let input = MoveInput {
            wish,
            ..MoveInput::default()
        };
        for _ in 0..(secs * 30.0) as usize {
            p.step(input, 1.0 / 30.0, &solids, &[]);
        }
        p
    }

    fn near_lantern(p: &Player) -> bool {
        Vec2::new(p.pos.x - LANTERN.0, p.pos.z - LANTERN.1).length() < LANTERN_REACH
    }

    const NORTH: Vec2 = Vec2::new(0.0, 1.0);

    #[test]
    fn i036_the_crate_blocks_the_doorway_until_the_balloon_lifts_it() {
        let lay = layout();
        let mut w = lay.world.clone();
        assert_eq!(w.objs.len(), NAMES.len());
        assert_eq!(w.objs[SPLITTER].kind, Kind::Splitter);
        assert_eq!(w.objs[BALLOON].kind, Kind::Balloon);
        assert_eq!(w.objs[CRATE].kind, Kind::Crate);
        assert_eq!(w.objs[3].kind, Kind::Lantern);
        assert_eq!(lay.boxes.len(), NAMES.len());
        // The crate is down: stopped south of the partition, in the middle
        // and hugging either doorpost.
        for x in [-1.0f32, 0.0, 1.0] {
            let p = walk(&w, &lay.colliders, Vec3::new(x, 0.0, 0.0), NORTH, 8.0);
            assert!(
                p.pos.z > WALL_Z,
                "x={x}: stopped south of the wall, z={}",
                p.pos.z
            );
            assert!(!near_lantern(&p));
        }
        // Three splits lift it: the lantern is reached.
        for _ in 0..3 {
            w.apply(SPLITTER, Spell::Water, &mut Vec::new());
        }
        assert!(w.objs[CRATE].lifted);
        let p = walk(&w, &lay.colliders, Vec3::new(0.0, 0.0, 0.0), NORTH, 12.0);
        assert!(near_lantern(&p), "reached the lantern, pos={:?}", p.pos);
    }

    #[test]
    fn i036_the_partition_is_closed_except_at_the_doorway() {
        let lay = layout();
        let mut w = lay.world.clone();
        for _ in 0..3 {
            w.apply(SPLITTER, Spell::Water, &mut Vec::new());
        }
        // From anywhere along the south face but the doorway, walking north
        // is stopped at the wall.
        let mut x = -22.0f32;
        while x <= 22.0 {
            if x.abs() > DOOR_HALF + 0.6 {
                let p = walk(
                    &w,
                    &lay.colliders,
                    Vec3::new(x, 0.0, WALL_Z + 3.0),
                    NORTH,
                    4.0,
                );
                assert!(p.pos.z > WALL_Z, "x={x}: passed the wall, z={}", p.pos.z);
            }
            x += 1.0;
        }
    }
}
