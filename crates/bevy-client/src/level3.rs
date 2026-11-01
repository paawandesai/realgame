//! Level 3, the Marsh (I-037): a misty marsh the size of the courtyard. A
//! plank boardwalk runs north from the start to a lantern on a hummock, with
//! deep reedy water either side. Twice the boardwalk sinks into a low place
//! where marsh gas (methane) bubbles up and burns across the planks: the
//! shallow dip (3 vents) and the deeper hollow (4 vents). Water and Frost
//! only knock a gas flame down; CO2, denser than air, sinks and fills the
//! low place, and once the pool is full every flame in it goes out at once.
//!
//! Layout (metres, y up; the player starts at z = +18 looking toward -z):
//! the back arch to the Frozen Gate is behind the start at (0, 22.6); the
//! boardwalk is x -1.5..1.5; the dip is z 7.5..12.5 with its vents on
//! z = 10; the hollow is z -11..-5 with vent rows on z = -7 and -9; the
//! lantern stands on the hummock at (0, -17).
//!
//! [`layout`] is the pure part (colliders and chemistry objects, unit
//! tested); [`build`] adds the visuals and [`sync`] moves them. Player text
//! for level 3 is [`caption`]: one line per (event, band), registered in
//! `content/science-sources.md`.

use crate::chem::{self, EventKind, Fizzle, Goal, Kind, Obj, Spell};
use crate::fx::{Fx, P};
use crate::world::{self, Aabb, Builder, Chem, Collider, Colliders, Kit, LevelRoot, StateMats};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use realgame_sim::tier::Tier;

/// Where the player arrives (feet), facing -z.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 18.0);
/// The arch back to the Frozen Gate (same place as the other levels' arch).
pub const BACK_ARCH: (f32, f32) = (0.0, 22.6);
/// The lantern's centre (x, z): walking within [`LANTERN_REACH`] wins.
pub const LANTERN: (f32, f32) = (0.0, -17.0);
pub const LANTERN_REACH: f32 = 1.5;

/// Half width of the boardwalk (the walkable strip).
pub const WALK_HALF: f32 = 1.5;
/// The reed beds start this far from the middle (the rails fill the rest).
const REED_X: f32 = 1.7;
/// North of this z the reeds open into the clearing round the hummock.
const CLEARING_Z: f32 = -13.5;
/// Half width of the clearing round the hummock.
const CLEARING_X: f32 = 4.0;
/// Height of the reed beds' and rails' solid boxes: above the highest jump
/// (1.0 m), so nothing can be climbed.
const BANK_H: f32 = 3.0;
const RAIL_H: f32 = 1.2;
/// A burning vent's hot zone (its collider and picking box): the full
/// boardwalk width between its neighbours, and above the highest jump.
const FLAME_H: f32 = 1.2;
const FLAME_HALF_Z: f32 = 0.4;
/// The flames themselves stay low: tops under 0.6 m.
const FLAME_TOP: f32 = 0.5;
/// The haze of a full pool stands just over the flame tops, far below head
/// height (eye 1.6 m).
pub const HAZE_FULL: f32 = 0.7;

/// The dip: footprint centre z and half length, and its vents' x.
pub const DIP_Z: f32 = 10.0;
const DIP_HALF_Z: f32 = 2.5;
const DIP_VENTS_X: [f32; 3] = [-1.0, 0.0, 1.0];
/// The hollow: footprint centre z and half length, and its vents (x, z).
pub const HOLLOW_Z: f32 = -8.0;
const HOLLOW_HALF_Z: f32 = 3.0;
const HOLLOW_VENTS: [(f32, f32); 4] = [(-0.85, -7.0), (0.85, -7.0), (-0.85, -9.0), (0.85, -9.0)];

/// Object indices in level 3's chemistry world (the tests use them; `pools`
/// lists the hollows in this order, dip first).
#[cfg(test)]
pub const DIP: usize = 0;
#[cfg(test)]
pub const HOLLOW: usize = 4;
/// Target names (golden finds targets by name).
pub const NAMES: [&str; 10] = [
    "dip",
    "dip vent",
    "dip vent",
    "dip vent",
    "hollow",
    "hollow vent",
    "hollow vent",
    "hollow vent",
    "hollow vent",
    "lantern",
];

/// Level 3's colliders, chemistry objects and picking boxes, without any
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

/// A box from its x, y and z ranges.
fn span(x: (f32, f32), y: (f32, f32), z: (f32, f32)) -> Aabb {
    Aabb {
        min: Vec3::new(x.0, y.0, z.0),
        max: Vec3::new(x.1, y.1, z.1),
    }
}

/// A vent's hot zone: x range, and its row's z.
fn vent_box(x0: f32, x1: f32, z: f32) -> Aabb {
    span(
        (x0, x1),
        (0.0, FLAME_H),
        (z - FLAME_HALF_Z, z + FLAME_HALF_Z),
    )
}

/// The marsh's colliders and chemistry. Pure: [`build`] and the tests use it.
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
    // Reed beds either side of the boardwalk, then round the clearing.
    for side in [-1.0f32, 1.0] {
        let reeds = [
            span((REED_X, h), (0.0, BANK_H), (CLEARING_Z, h)),
            span((CLEARING_X, h), (0.0, BANK_H), (-h, CLEARING_Z)),
        ];
        for a in reeds {
            let (x0, x1) = (a.min.x * side, a.max.x * side);
            c.push(Collider {
                aabb: span(
                    (x0.min(x1), x0.max(x1)),
                    (a.min.y, a.max.y),
                    (a.min.z, a.max.z),
                ),
                obj: None,
            });
        }
        // The boardwalk rails, from the arch to the clearing.
        let x = side * (WALK_HALF + REED_X) / 2.0;
        let rail_half = (REED_X - WALK_HALF) / 2.0;
        let (z0, z1) = (CLEARING_Z, h);
        solid(
            &mut c,
            Vec3::new(x, RAIL_H / 2.0, (z0 + z1) / 2.0),
            Vec3::new(rail_half, RAIL_H / 2.0, (z1 - z0) / 2.0),
        );
    }
    // The lantern's plinth and post.
    let (lx, lz) = LANTERN;
    solid(&mut c, Vec3::new(lx, 1.1, lz), Vec3::new(0.4, 1.1, 0.4));

    // Chemistry: the dip, its vents, the hollow, its vents, the lantern.
    let mut objs = Vec::new();
    let mut boxes = Vec::new();
    let floor = |z: f32, half_z: f32| {
        span(
            (-WALK_HALF, WALK_HALF),
            (0.0, 0.1),
            (z - half_z, z + half_z),
        )
    };
    let footprint = |a: &Aabb| {
        let (cc, half) = (a.center(), (a.max - a.min) * 0.5);
        ([cc.x, cc.z], [half.x, half.z])
    };
    objs.push(Obj::hollow(
        [0.0, DIP_Z],
        [WALK_HALF + 0.1, DIP_HALF_Z],
        chem::DIP_CO2_PER_CAST,
    ));
    boxes.push(floor(DIP_Z, DIP_HALF_Z));
    let mut vents: Vec<Aabb> = DIP_VENTS_X
        .iter()
        .map(|&x| vent_box(x - 0.5, x + 0.5, DIP_Z))
        .collect();
    // Edge vents reach the rails: no gap at the sides.
    vents[0].min.x = -WALK_HALF - 0.05;
    vents[2].max.x = WALK_HALF + 0.05;
    for a in &vents {
        let (p, half) = footprint(a);
        objs.push(Obj::new(Kind::GasVent, p, half));
        boxes.push(*a);
    }
    objs.push(Obj::hollow(
        [0.0, HOLLOW_Z],
        [WALK_HALF + 0.1, HOLLOW_HALF_Z],
        chem::HOLLOW_CO2_PER_CAST,
    ));
    boxes.push(floor(HOLLOW_Z, HOLLOW_HALF_Z));
    for (x, z) in HOLLOW_VENTS {
        // Each covers its half of the boardwalk, overlapping in the middle.
        let a = if x < 0.0 {
            vent_box(-WALK_HALF - 0.05, 0.05, z)
        } else {
            vent_box(-0.05, WALK_HALF + 0.05, z)
        };
        let (p, half) = footprint(&a);
        objs.push(Obj::new(Kind::GasVent, p, half));
        boxes.push(a);
    }
    objs.push(Obj::new(Kind::Lantern, [lx, lz], [0.4, 0.4]));
    boxes.push(Aabb::new(
        Vec3::new(lx, 1.0, lz),
        Vec3::new(0.45, 1.0, 0.45),
    ));
    // Each burning vent is solid: too hot to pass.
    for (i, o) in objs.iter().enumerate() {
        if o.kind == Kind::GasVent {
            c.push(Collider {
                aabb: boxes[i],
                obj: Some(i),
            });
        }
    }
    Layout {
        colliders: c,
        world: chem::World::with_goal(objs, Goal::Reach),
        boxes,
    }
}

/// I-036: the arch in the north wall to the Crystal Lab, shut until the
/// lantern is reached (the same place as level 2's north arch).
pub const NEXT_ARCH: (f32, f32) = (0.0, -22.6);

/// I-036: the Marsh as the player left it, for the walk back from the
/// Crystal Lab. Contract stub.
pub fn solved(w: &mut chem::World) {
    let _ = w;
}

/// Each hollow's CO2 fill, dip first (`pools`); empty outside level 3.
pub fn pools(w: &chem::World) -> Vec<f32> {
    w.objs
        .iter()
        .filter(|o| o.kind == Kind::Hollow)
        .map(|o| o.co2)
        .collect()
}

/// Gas vents still burning (`ventsLit`).
pub fn vents_lit(w: &chem::World) -> usize {
    w.objs
        .iter()
        .filter(|o| o.kind == Kind::GasVent && o.burning)
        .count()
}

/// Level 3's caption for an event from a spell that hit `obj`, at the
/// player's band (`None` reads as Apprentice), with the Sprout picture
/// (`"gas"`, `"wrong"`, or `""`). `None` for events level 3 has no line for
/// (the courtyard's caption is used).
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
    let marsh = matches!(obj, Kind::GasVent | Kind::Hollow);
    match kind {
        E::Flicker(Spell::Water) if marsh => pick(
            [
                "The fire is on a gas. Water can't stop it. Try CO₂!",
                "The flame burns gas that bubbles up through the water. Water cannot soak a gas, so the flame grows back.",
                E::Flicker(Spell::Water).caption(),
                "Gas fires are fed from below: water cools the flame but does not cut off the fuel or the oxygen, so the CH₄ keeps burning.",
            ],
            "wrong",
        ),
        E::Flicker(Spell::Frost) if marsh => pick(
            [
                "Brrr! The fire came back. Try CO₂!",
                "The cold shrank the flame, but the gas keeps coming up and burning.",
                E::Flicker(Spell::Frost).caption(),
                "At air temperature N₂ (28 g/mol) is nearly as dense as air (29 g/mol): the cold boil-off warms, mixes away instead of pooling, and the CH₄ flame recovers.",
            ],
            "wrong",
        ),
        E::NoChange if marsh => two(
            "It is already burning!",
            "That gas is already burning. Fire cannot put out fire.",
            "wrong",
        ),
        E::Filling if marsh => pick(
            [
                "The gas sinks down low. More!",
                "CO₂ is heavier than air, so it sinks and fills the hollow from the bottom.",
                E::Filling.caption(),
                "At the same temperature and pressure, gas density goes with molar mass: CO₂ is about 1.5 times as dense as air, so it settles in low places.",
            ],
            "gas",
        ),
        E::Pooled if marsh => pick(
            [
                "These fires went out!",
                "The CO₂ covered the flames. No oxygen, no fire.",
                E::Pooled.caption(),
                "Smothered: pooled CO₂ cut off the oxygen to every flame at once. A deep pool of CO₂ is dangerous for people to breathe too.",
            ],
            "gas",
        ),
        E::Fizzle(Fizzle::NoOxygen) if marsh => two(
            "No air down there, no fire.",
            "Fire needs oxygen, and the CO₂ keeps it away.",
            "wrong",
        ),
        E::Spilled => two(
            "It rolled away! Find a low spot.",
            E::Spilled.caption(),
            "wrong",
        ),
        // Water or Frost where the flames are already out.
        E::Soaked | E::Chilled if marsh => two(
            "These flames are already out.",
            "These flames are already out.",
            "",
        ),
        // Any spell on the lantern: it is the goal, not a target.
        _ if obj == Kind::Lantern => two(
            "That is the lantern. Walk up to it!",
            "That is the lantern. Walk up to it!",
            "",
        ),
        _ => None,
    }
}

/// A burning vent's flame cone (blue, low). `k` spreads the flicker phase.
#[derive(Component)]
pub struct VentFlame {
    pub obj: usize,
    pub k: f32,
    /// Full-size width and height.
    pub size: Vec2,
}

/// The pale haze that shows how full a hollow's CO2 pool is (a game cue:
/// real CO2 is colourless).
#[derive(Component)]
pub struct Haze {
    pub obj: usize,
}

/// Spawns the marsh. Every entity carries [`LevelRoot`].
pub fn build(
    commands: &mut Commands,
    kit: &Kit,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> (Colliders, Chem) {
    let lay = layout();
    let h = world::HALF;
    let wh = world::WALL_H;
    let grass_tex = world::grass_texture(images);
    let stone_tex = world::stone_texture(images);
    let ground = world::textured(
        materials,
        Color::srgb(0.24, 0.36, 0.3),
        &grass_tex,
        Vec2::splat(25.0),
    );
    let wall_long = world::textured(
        materials,
        Color::srgb(0.58, 0.66, 0.6),
        &stone_tex,
        Vec2::new(25.0, 2.0),
    );
    let moss = world::toon(materials, Color::srgb(0.36, 0.5, 0.3));
    let mud = world::toon(materials, Color::srgb(0.32, 0.26, 0.2));
    let mud_dark = world::toon(materials, Color::srgb(0.22, 0.19, 0.16));
    let plank = world::toon(materials, Color::srgb(0.6, 0.45, 0.3));
    let plank_dark = world::toon(materials, Color::srgb(0.45, 0.33, 0.22));
    let reed = world::toon(materials, Color::srgb(0.46, 0.6, 0.3));
    let reed_dry = world::toon(materials, Color::srgb(0.66, 0.62, 0.38));
    let cattail = world::toon(materials, Color::srgb(0.4, 0.27, 0.16));
    let rock = world::toon(materials, Color::srgb(0.46, 0.5, 0.46));
    let rock_light = world::toon(materials, Color::srgb(0.62, 0.66, 0.6));
    let wood = world::toon(materials, Color::srgb(0.5, 0.34, 0.22));
    let lamp = world::glow(materials, Color::srgb(1.0, 0.82, 0.4), 6.0);
    let pond = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.36, 0.38, 0.85),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        reflectance: 0.5,
        ..default()
    });
    let flame = world::glow(materials, Color::srgb(0.3, 0.55, 1.0), 5.0);
    let haze = materials.add(StandardMaterial {
        base_color: Color::srgba(0.86, 0.9, 0.92, 0.42),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
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

    // Ground (dark marsh) with the open water of the reed beds over it.
    b.piece(
        &cube,
        &ground,
        Transform::from_xyz(0.0, -0.25, 0.0).with_scale(Vec3::new(
            2.0 * h + 2.0,
            0.5,
            2.0 * h + 2.0,
        )),
    );
    for side in [-1.0f32, 1.0] {
        let w = h - REED_X;
        b.commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(pond.clone()),
            Transform::from_xyz(side * (REED_X + w / 2.0), 0.03, (CLEARING_Z + h) / 2.0)
                .with_scale(Vec3::new(w, 0.04, h - CLEARING_Z)),
            NotShadowCaster,
            LevelRoot,
        ));
    }

    // Perimeter walls with mossy caps (colliders come from `layout`).
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
            &moss,
            Transform::from_translation(Vec3::new(c.x, wh + 0.15, c.z))
                .with_rotation(rot)
                .with_scale(Vec3::new(wl, 0.3, 1.4)),
        );
    }

    // The way back: the arch, with a snowflake of ice on its lintel.
    world::arch(&mut b, materials, &rock);
    {
        let (x, z) = BACK_ARCH;
        b.piece(
            &cube,
            &kit.ice,
            Transform::from_xyz(x, 4.3, z + 0.4)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4))
                .with_scale(Vec3::new(0.6, 0.6, 0.5)),
        );
    }

    // The boardwalk: planks from the arch to the clearing, darker in the low
    // places, and the rails on posts.
    let low = [
        (DIP_Z - DIP_HALF_Z, DIP_Z + DIP_HALF_Z),
        (HOLLOW_Z - HOLLOW_HALF_Z, HOLLOW_Z + HOLLOW_HALF_Z),
    ];
    let mut segs = vec![(CLEARING_Z, h)];
    for (z0, z1) in low {
        let mut next = Vec::new();
        for (a, bb) in segs {
            if z1 <= a || z0 >= bb {
                next.push((a, bb));
            } else {
                if a < z0 {
                    next.push((a, z0));
                }
                if z1 < bb {
                    next.push((z1, bb));
                }
            }
        }
        segs = next;
    }
    for (z0, z1) in segs {
        b.piece(
            &cube,
            &plank,
            Transform::from_xyz(0.0, 0.04, (z0 + z1) / 2.0).with_scale(Vec3::new(
                2.0 * WALK_HALF,
                0.08,
                z1 - z0,
            )),
        );
    }
    for (z0, z1) in low {
        // Muddy floor a little below the planks, and wet planks over it.
        b.piece(
            &cube,
            &mud_dark,
            Transform::from_xyz(0.0, 0.005, (z0 + z1) / 2.0).with_scale(Vec3::new(
                2.0 * REED_X,
                0.01,
                z1 - z0,
            )),
        );
        for k in 0..((z1 - z0) / 0.5) as usize {
            if k % 2 == 1 {
                continue;
            }
            let z = z0 + 0.25 + k as f32 * 0.5;
            b.piece(
                &cube,
                &plank_dark,
                Transform::from_xyz(0.0, 0.02, z).with_scale(Vec3::new(2.0 * WALK_HALF, 0.03, 0.4)),
            );
        }
        // Mud banks rise either side, so the place reads as a hollow.
        for side in [-1.0f32, 1.0] {
            b.piece(
                &ball,
                &mud,
                Transform::from_xyz(side * (REED_X + 1.0), 0.2, (z0 + z1) / 2.0)
                    .with_scale(Vec3::new(2.4, 1.4, z1 - z0 + 1.0)),
            );
        }
    }
    for side in [-1.0f32, 1.0] {
        let x = side * (WALK_HALF + REED_X) / 2.0;
        b.piece(
            &cube,
            &wood,
            Transform::from_xyz(x, 0.85, (CLEARING_Z + h) / 2.0).with_scale(Vec3::new(
                0.12,
                0.1,
                h - CLEARING_Z,
            )),
        );
        let mut z = h - 1.0;
        while z > CLEARING_Z {
            b.piece(
                &cube,
                &wood,
                Transform::from_xyz(x, 0.42, z).with_scale(Vec3::new(0.14, 0.84, 0.14)),
            );
            z -= 4.0;
        }
    }

    // Reeds and cattails in clumps over the water (the beds are solid).
    for (k, (x, z, s)) in [
        (-3.0f32, 19.0f32, 1.1f32),
        (3.2, 16.0, 1.0),
        (-2.6, 13.5, 0.9),
        (2.8, 6.0, 1.2),
        (-3.1, 4.0, 1.0),
        (3.0, 0.5, 0.9),
        (-2.8, -2.5, 1.1),
        (2.7, -4.0, 1.0),
        (-3.3, -12.5, 1.2),
        (3.4, -12.0, 1.0),
        (-5.0, -18.0, 1.3),
        (5.2, -16.0, 1.1),
        (-7.0, 10.0, 1.4),
        (8.0, -6.0, 1.5),
        (-9.0, -8.0, 1.2),
        (10.0, 14.0, 1.3),
        (-12.0, 18.0, 1.1),
        (14.0, 4.0, 1.4),
        (-15.0, 2.0, 1.3),
        (17.0, -15.0, 1.2),
    ]
    .into_iter()
    .enumerate()
    {
        let m = if k % 3 == 0 { &reed_dry } else { &reed };
        b.piece(
            &cone,
            m,
            Transform::from_xyz(x, 0.9 * s, z).with_scale(Vec3::new(1.4 * s, 1.8 * s, 1.4 * s)),
        );
        b.piece(
            &cyl,
            &cattail,
            Transform::from_xyz(x + 0.3 * s, 1.6 * s, z - 0.2).with_scale(Vec3::new(
                0.12 * s,
                0.4 * s,
                0.12 * s,
            )),
        );
    }
    // Dead trees standing in the water.
    for (x, z, s) in [
        (-11.0f32, -14.0f32, 1.2f32),
        (12.0, 9.0, 1.0),
        (-16.0, 12.0, 1.1),
    ] {
        b.piece(
            &cyl,
            &wood,
            Transform::from_xyz(x, 1.6 * s, z).with_scale(Vec3::new(0.35 * s, 3.2 * s, 0.35 * s)),
        );
        b.piece(
            &cyl,
            &wood,
            Transform::from_xyz(x + 0.5 * s, 2.6 * s, z)
                .with_rotation(Quat::from_rotation_z(-0.9))
                .with_scale(Vec3::new(0.18 * s, 1.4 * s, 0.18 * s)),
        );
    }

    // The hummock and its lantern.
    let (lx, lz) = LANTERN;
    b.piece(
        &ball,
        &moss,
        Transform::from_xyz(lx, -0.35, lz).with_scale(Vec3::new(7.0, 0.9, 6.0)),
    );
    for (dx, dz, s) in [
        (-2.6f32, 1.2f32, 0.9f32),
        (2.4, -1.6, 1.0),
        (-1.8, -2.4, 0.7),
    ] {
        b.piece(
            &ball,
            &rock_light,
            Transform::from_xyz(lx + dx, 0.2 * s, lz + dz).with_scale(Vec3::new(
                1.2 * s,
                0.7 * s,
                s,
            )),
        );
        b.colliders.push(Collider {
            aabb: Aabb::new(
                Vec3::new(lx + dx, 0.35 * s, lz + dz),
                Vec3::new(0.5 * s, 0.35 * s, 0.4 * s),
            ),
            obj: None,
        });
    }
    b.piece(
        &cube,
        &rock,
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
            mats: plain(&mud),
            last: None,
        };
        match o.kind {
            Kind::GasVent => {
                // Low blue flames across the vent's part of the boardwalk.
                let a = lay.boxes[i];
                let z = a.center().z;
                let (x0, x1) = (a.min.x.max(-WALK_HALF), a.max.x.min(WALK_HALF));
                let n = ((x1 - x0) / 0.5).round().max(2.0) as usize;
                for k in 0..n {
                    let x = x0 + (k as f32 + 0.5) * (x1 - x0) / n as f32;
                    let wide = 0.5 * (x1 - x0) / n as f32 + 0.15;
                    b.commands.spawn((
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(flame.clone()),
                        Transform::from_xyz(x, FLAME_TOP / 2.0, z)
                            .with_scale(Vec3::new(wide, FLAME_TOP, wide)),
                        NotShadowCaster,
                        VentFlame {
                            obj: i,
                            k: (i * 7 + k * 3) as f32,
                            size: Vec2::new(wide, FLAME_TOP),
                        },
                        LevelRoot,
                    ));
                }
                // The vent: a dark ring of mud where the gas comes up.
                b.piece(
                    &cyl,
                    &mud_dark,
                    Transform::from_xyz((x0 + x1) / 2.0, 0.03, z).with_scale(Vec3::new(
                        (x1 - x0) * 0.8,
                        0.04,
                        0.5,
                    )),
                );
            }
            Kind::Hollow => {
                let a = lay.boxes[i];
                let (cc, size) = (a.center(), a.max - a.min);
                b.commands.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(haze.clone()),
                    Transform::from_xyz(cc.x, 0.0, cc.z).with_scale(Vec3::new(
                        size.x + 0.2,
                        0.001,
                        size.z,
                    )),
                    Visibility::Hidden,
                    NotShadowCaster,
                    Haze { obj: i },
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

/// How tall a vent's flame is right now, as a fraction of full size: knocked
/// down to 0.4 by Water or Frost, growing back over [`chem::FLICKER_S`].
pub fn flame_scale(o: &Obj) -> f32 {
    1.0 - 0.6 * (o.flicker / chem::FLICKER_S).clamp(0.0, 1.0)
}

/// Level 3's moving parts each frame: flames (shown while burning, low,
/// wobbling unless `still`, shrunk by a flicker), the haze in each hollow
/// (its height follows the pool) and gas bubbles rising from every vent, lit
/// or smothered (at most about 12 alive at `low`).
#[allow(clippy::too_many_arguments)]
pub fn sync<F: bevy::ecs::query::QueryFilter, G: bevy::ecs::query::QueryFilter>(
    chem: &Chem,
    t: f32,
    dt: f32,
    still: bool,
    low: bool,
    flames: &mut Query<(&mut Transform, &mut Visibility, &VentFlame), F>,
    haze: &mut Query<(&mut Transform, &mut Visibility, &Haze), G>,
    fx: &mut Fx,
) {
    let objs = &chem.world.objs;
    for (mut tr, mut vis, f) in flames.iter_mut() {
        let Some(o) = objs.get(f.obj) else {
            continue;
        };
        let want = if o.burning {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if !o.burning {
            continue;
        }
        let wob = if still {
            1.0
        } else {
            1.0 + 0.12 * (t * 9.0 + f.k).sin()
        };
        let k = flame_scale(o);
        let hgt = (f.size.y * k * wob).min(0.58);
        let wid = f.size.x * (0.6 + 0.4 * k);
        tr.scale = Vec3::new(wid, hgt, wid);
        tr.translation.y = hgt / 2.0 + 0.02;
    }
    for (mut tr, mut vis, hz) in haze.iter_mut() {
        let Some(o) = objs.get(hz.obj) else {
            continue;
        };
        let hgt = HAZE_FULL * o.co2.clamp(0.0, 1.0);
        let want = if hgt > 0.01 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if (tr.scale.y - hgt.max(0.001)).abs() > 1e-4 {
            tr.scale.y = hgt.max(0.001);
            tr.translation.y = hgt / 2.0;
        }
    }
    // Bubbles of marsh gas rising out of the water at each vent, burning or
    // smothered: the gas keeps coming up after its flame goes out (I-037
    // break 1, S3); only the flame needs oxygen.
    let rate = if low { 1.5 } else { 3.0 };
    for (i, o) in objs.iter().enumerate() {
        if o.kind != Kind::GasVent || fx.rand() >= rate * dt {
            continue;
        }
        let a = chem.targets[i].aabb;
        let x = a.min.x + (a.max.x - a.min.x) * (0.2 + 0.6 * fx.rand());
        let z = a.center().z + (fx.rand() - 0.5) * 0.4;
        let v = Vec3::new(0.0, 0.7 + 0.3 * fx.rand(), 0.0);
        fx.emit(P::Foam, Vec3::new(x, 0.05, z), v, 0.09, 0.8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{MoveInput, Player};

    // ---- I-037 acceptance test 4: the caption table ----

    const SPROUT: Option<Tier> = Some(Tier::Sprout);
    const SEEDLING: Option<Tier> = Some(Tier::Seedling);
    const APPRENTICE: Option<Tier> = Some(Tier::Apprentice);
    const ADEPT: Option<Tier> = Some(Tier::Adept);

    /// (event, objects it applies to, [Sprout, Seedling, Apprentice, Adept],
    /// Sprout icon), exactly as the spec's table.
    fn table() -> Vec<(EventKind, Vec<Kind>, [&'static str; 4], &'static str)> {
        let vent = vec![Kind::GasVent, Kind::Hollow];
        let fire = "That gas is already burning. Fire cannot put out fire.";
        let no_o2 = "Fire needs oxygen, and the CO₂ keeps it away.";
        let spill = "No hollow here: the CO₂ spills away downhill.";
        vec![
            (
                EventKind::Flicker(Spell::Water),
                vent.clone(),
                [
                    "The fire is on a gas. Water can't stop it. Try CO₂!",
                    "The flame burns gas that bubbles up through the water. Water cannot soak a gas, so the flame grows back.",
                    "Marsh gas is methane (CH₄). It keeps bubbling up through the water, so a splash only knocks the flame down for a moment.",
                    "Gas fires are fed from below: water cools the flame but does not cut off the fuel or the oxygen, so the CH₄ keeps burning.",
                ],
                "wrong",
            ),
            (
                EventKind::Flicker(Spell::Frost),
                vent.clone(),
                [
                    "Brrr! The fire came back. Try CO₂!",
                    "The cold shrank the flame, but the gas keeps coming up and burning.",
                    "Liquid nitrogen boils into N₂ gas. Once it warms up, N₂ is about as heavy as air, so it mixes away and the methane keeps burning.",
                    "At air temperature N₂ (28 g/mol) is nearly as dense as air (29 g/mol): the cold boil-off warms, mixes away instead of pooling, and the CH₄ flame recovers.",
                ],
                "wrong",
            ),
            (
                EventKind::NoChange,
                vent.clone(),
                ["It is already burning!", fire, fire, fire],
                "wrong",
            ),
            (
                EventKind::Filling,
                vent.clone(),
                [
                    "The gas sinks down low. More!",
                    "CO₂ is heavier than air, so it sinks and fills the hollow from the bottom.",
                    "CO₂ (44 g/mol) is denser than air (29 g/mol), so it pools low, like an invisible pond.",
                    "At the same temperature and pressure, gas density goes with molar mass: CO₂ is about 1.5 times as dense as air, so it settles in low places.",
                ],
                "gas",
            ),
            (
                EventKind::Pooled,
                vent.clone(),
                [
                    "These fires went out!",
                    "The CO₂ covered the flames. No oxygen, no fire.",
                    "The CO₂ pool pushed the air out of the hollow. Without O₂, CH₄ + 2O₂ → CO₂ + 2H₂O cannot happen.",
                    "Smothered: pooled CO₂ cut off the oxygen to every flame at once. A deep pool of CO₂ is dangerous for people to breathe too.",
                ],
                "gas",
            ),
            (
                EventKind::Fizzle(Fizzle::NoOxygen),
                vent.clone(),
                ["No air down there, no fire.", no_o2, no_o2, no_o2],
                "wrong",
            ),
            (
                EventKind::Spilled,
                vec![Kind::GasVent],
                ["It rolled away! Find a low spot.", spill, spill, spill],
                "wrong",
            ),
        ]
    }

    #[test]
    fn i037_caption_table_is_exact_per_band() {
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
    }

    #[test]
    fn i037_caption_strings_are_safe_player_text() {
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

    // ---- I-036 acceptance test 8: the Marsh, solved for the way back ----

    #[test]
    fn i036_solved_fills_both_hollows_and_latches_the_win() {
        let mut w = layout().world;
        solved(&mut w);
        assert_eq!(pools(&w), vec![1.0, 1.0]);
        assert_eq!(vents_lit(&w), 0);
        assert_eq!(w.outcome(), chem::Outcome::Won);
        // Solving again changes nothing.
        let before = (pools(&w), vents_lit(&w));
        solved(&mut w);
        assert_eq!((pools(&w), vents_lit(&w)), before);
    }

    // ---- I-037 acceptance test 5: the walk to the lantern ----

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
    fn i037_flames_block_the_boardwalk_until_each_hollow_is_pooled() {
        let lay = layout();
        let mut w = lay.world.clone();
        assert_eq!(w.objs.len(), NAMES.len());
        assert_eq!(pools(&w), vec![0.0, 0.0]);
        assert_eq!(vents_lit(&w), 7);
        // Burning dip: stopped south of the dip's flames.
        let p = walk(&w, &lay.colliders, START, NORTH, 12.0);
        assert!(p.pos.z > DIP_Z, "stopped by the dip flames, z={}", p.pos.z);
        // Also when hugging either rail.
        for x in [-1.1f32, 1.1] {
            let p = walk(&w, &lay.colliders, START + Vec3::X * x, NORTH, 12.0);
            assert!(p.pos.z > DIP_Z, "rail x={x}: z={}", p.pos.z);
        }
        // Dip pooled (two casts), hollow burning: stopped at the hollow.
        for _ in 0..2 {
            w.apply(DIP, Spell::Foam, &mut Vec::new());
        }
        assert_eq!(vents_lit(&w), 4);
        for x in [-1.1f32, 0.0, 1.1] {
            let p = walk(&w, &lay.colliders, START + Vec3::X * x, NORTH, 12.0);
            assert!(
                p.pos.z < DIP_Z - 2.0 && p.pos.z > HOLLOW_Z + 1.0,
                "crossed the dip and stopped at the hollow (x={x}), z={}",
                p.pos.z
            );
            assert!(!near_lantern(&p));
        }
        // Both pooled: the lantern is reached.
        for _ in 0..3 {
            w.apply(HOLLOW, Spell::Foam, &mut Vec::new());
        }
        assert_eq!(vents_lit(&w), 0);
        assert_eq!(pools(&w), vec![1.0, 1.0]);
        let p = walk(&w, &lay.colliders, START, NORTH, 12.0);
        assert!(near_lantern(&p), "reached the lantern, pos={:?}", p.pos);
    }

    #[test]
    fn i037_reed_beds_close_both_sides_of_the_boardwalk() {
        let lay = layout();
        let mut w = lay.world.clone();
        // Flames out, so a sideways sweep can start anywhere on the planks.
        for _ in 0..2 {
            w.apply(DIP, Spell::Foam, &mut Vec::new());
        }
        for _ in 0..3 {
            w.apply(HOLLOW, Spell::Foam, &mut Vec::new());
        }
        let mut z = START.z;
        while z > LANTERN.1 + 2.0 {
            for side in [-1.0f32, 1.0] {
                let p = walk(
                    &w,
                    &lay.colliders,
                    Vec3::new(0.0, 0.0, z),
                    Vec2::new(side, 0.0),
                    3.0,
                );
                let edge = if z > CLEARING_Z { REED_X } else { CLEARING_X };
                assert!(
                    p.pos.x.abs() < edge && p.pos.x * side > 0.5,
                    "z={z} side {side}: stopped at x={}",
                    p.pos.x
                );
            }
            z -= 0.5;
        }
    }
}
