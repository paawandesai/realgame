//! Level 2, the Frozen Gate (I-035): a snowy valley the size of the
//! courtyard. A stream too deep to wade crosses it (freeze it into a bridge
//! with Frost), a rock ridge with an ice gate closes it (melt the gate with
//! Fire), and a lantern behind the gate is the goal.
//!
//! Layout (metres, y up; the player starts at z = +18 looking toward -z):
//! the back arch to the courtyard is behind the start at (0, 22.6); the
//! stream fills z 4..8 across the whole valley, with rock banks that leave
//! one landing on each side at x -2..2; the ridge runs along z = -8 with the
//! ice gate in its 4 m opening; the lantern stands on a plinth at (0, -17).
//!
//! [`layout`] is the pure part (colliders and chemistry objects, unit
//! tested); [`build`] adds the visuals. Player text for level 2 is
//! [`caption`]: one line per (event, band), registered in
//! `content/science-sources.md`.

use crate::chem::{self, EventKind, Goal, Kind, Obj};
use crate::world::{self, Aabb, Builder, Chem, Collider, Colliders, Kit, LevelRoot, StateMats};
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::prelude::*;
use realgame_sim::tier::Tier;

/// Where the player arrives (feet), facing -z.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 18.0);
/// The arch back to the courtyard (same place as level 1's exit arch).
pub const BACK_ARCH: (f32, f32) = (0.0, 22.6);
/// The lantern's centre (x, z): walking within [`LANTERN_REACH`] wins.
pub const LANTERN: (f32, f32) = (0.0, -17.0);
pub const LANTERN_REACH: f32 = 1.5;
/// I-037: the arch in the north wall to the Marsh, shut until the lantern
/// is reached.
pub const NEXT_ARCH: (f32, f32) = (0.0, -22.6);
/// Where the player arrives back from the Marsh (feet), facing +z: in front
/// of the north arch, out of its reach.
pub const ARRIVE_NORTH: Vec3 = Vec3::new(0.0, 0.0, -20.4);

/// I-037: the north arch's door panel; it shows `ArchMats::open` while the
/// way to the Marsh is open.
#[derive(Component)]
pub struct NextDoor;

/// The closed north door's solid box (before the lantern is reached).
pub fn next_door() -> Aabb {
    Aabb::new(
        Vec3::new(NEXT_ARCH.0, 1.5, NEXT_ARCH.1 - 0.4),
        Vec3::new(1.5, 1.5, 0.2),
    )
}

/// The stream's extent along z (it spans the whole valley along x).
pub const STREAM_Z: (f32, f32) = (4.0, 8.0);
/// Height of the stream's (invisible) solid box while liquid: above the
/// highest jump (1.0 m), so it cannot be hopped onto.
const STREAM_H: f32 = 1.2;
/// The landings in the rock banks: x within ±`LANDING_X`, at these z.
pub const LANDING_X: f32 = 2.0;
pub const LANDING_SOUTH: f32 = 8.4;
pub const LANDING_NORTH: f32 = 3.6;
/// Half width of the gap in the banks (the landings plus the player radius).
const GAP_X: f32 = 2.4;
/// The ridge with the ice gate.
const RIDGE_Z: f32 = -8.0;
const GATE_HALF_X: f32 = 2.0;
const GATE_H: f32 = 3.2;

/// Boulders by the landings: (x, z, size). The ball is `size` x 1.3 wide,
/// `size` tall and deep, centred 1.2 m up.
pub const BOULDERS: [(f32, f32, f32); 4] = [
    (-3.2, 9.0, 1.3),
    (3.3, 8.9, 1.2),
    (-3.1, 3.1, 1.2),
    (3.2, 3.2, 1.3),
];
/// Snow mounds: (x, z, size). The half ball is `size` x 0.8 wide (x),
/// `size` x 0.35 tall and `size` x 0.65 deep (z).
pub const MOUNDS: [(f32, f32, f32); 6] = [
    (-8.0, 19.0, 1.6),
    (7.0, 12.0, 1.3),
    (-12.0, -12.0, 1.8),
    (9.0, -13.0, 1.4),
    (-20.0, 10.0, 2.0),
    (20.0, 9.0, 1.7),
];

/// I-049 (I-035 break 1, should-fix 7): the solid boxes of the snow mounds
/// and boulders, inside each ball's footprint (so the walk stops at the snow
/// or rock and the camera never ends up inside it).
pub fn prop_boxes() -> Vec<Aabb> {
    let mut v = Vec::new();
    for (x, z, s) in BOULDERS {
        // The rock: 0.65 s x 0.5 s x 0.5 s radii, 1.2 m up; the box reaches
        // the ground so it cannot be walked under.
        v.push(Aabb::new(
            Vec3::new(x, (1.2 + 0.5 * s) / 2.0, z),
            Vec3::new(0.65 * s * 0.8, (1.2 + 0.5 * s) / 2.0, 0.5 * s * 0.8),
        ));
    }
    for (x, z, s) in MOUNDS {
        v.push(Aabb::new(
            Vec3::new(x, 0.35 * s / 2.0, z),
            Vec3::new(0.8 * s * 0.85, 0.35 * s / 2.0, 0.65 * s * 0.85),
        ));
    }
    v
}

/// I-049 (I-035 break 1, should-fix 6): any spell on the lantern. It is the
/// goal, not a target, so the line says nothing about burning (the same
/// words as the Marsh's lantern).
pub const LANTERN_LINE: &str = "That is the lantern. Walk up to it!";

/// Object indices in level 2's chemistry world.
pub const STREAM: usize = 0;
pub const GATE: usize = 1;
pub const LANTERN_OBJ: usize = 2;
/// Target names (golden finds targets by name).
pub const NAMES: [&str; 3] = ["stream", "ice gate", "lantern"];

/// Level 2's colliders, chemistry objects and picking boxes, without any
/// visuals.
pub struct Layout {
    pub colliders: Vec<Collider>,
    pub world: chem::World,
    /// Picking box per object, and the box of the puddle a melted gate leaves.
    pub boxes: Vec<Aabb>,
    pub melted: Vec<Option<Aabb>>,
}

fn solid(colliders: &mut Vec<Collider>, center: Vec3, half: Vec3) {
    colliders.push(Collider {
        aabb: Aabb::new(center, half),
        obj: None,
    });
}

/// The valley's colliders and chemistry. Pure: [`build`] and the tests use it.
pub fn layout() -> Layout {
    let mut c: Vec<Collider> = Vec::new();
    let h = world::HALF;
    let wh = world::WALL_H;
    // Perimeter walls (the same boxes as the courtyard's).
    let wl = 2.0 * h + 2.0;
    solid(
        &mut c,
        Vec3::new(0.0, wh / 2.0, -h - 0.5),
        Vec3::new(wl / 2.0, wh / 2.0, 0.5),
    );
    solid(
        &mut c,
        Vec3::new(0.0, wh / 2.0, h + 0.5),
        Vec3::new(wl / 2.0, wh / 2.0, 0.5),
    );
    solid(
        &mut c,
        Vec3::new(-h - 0.5, wh / 2.0, 0.0),
        Vec3::new(0.5, wh / 2.0, wl / 2.0),
    );
    solid(
        &mut c,
        Vec3::new(h + 0.5, wh / 2.0, 0.0),
        Vec3::new(0.5, wh / 2.0, wl / 2.0),
    );
    let (z0, z1) = STREAM_Z;
    let zc = (z0 + z1) / 2.0;
    let stream_box = Aabb::new(
        Vec3::new(0.0, STREAM_H / 2.0, zc),
        Vec3::new(h, STREAM_H / 2.0, (z1 - z0) / 2.0),
    );
    c.push(Collider {
        aabb: stream_box,
        obj: Some(STREAM),
    });
    // Rock banks either side of the landings (too high to jump onto).
    let rock_half_x = (h - GAP_X) / 2.0;
    for side in [-1.0f32, 1.0] {
        let x = side * (GAP_X + rock_half_x);
        solid(
            &mut c,
            Vec3::new(x, 0.7, z1 + 0.9),
            Vec3::new(rock_half_x, 0.7, 0.9),
        );
        solid(
            &mut c,
            Vec3::new(x, 0.7, z0 - 0.9),
            Vec3::new(rock_half_x, 0.7, 0.9),
        );
    }
    // The ridge, either side of the gate.
    let ridge_half_x = (h - GATE_HALF_X) / 2.0;
    for side in [-1.0f32, 1.0] {
        solid(
            &mut c,
            Vec3::new(side * (GATE_HALF_X + ridge_half_x), wh / 2.0, RIDGE_Z),
            Vec3::new(ridge_half_x, wh / 2.0, 0.6),
        );
    }
    let gate_box = Aabb::new(
        Vec3::new(0.0, GATE_H / 2.0, RIDGE_Z),
        Vec3::new(GATE_HALF_X, GATE_H / 2.0, 0.5),
    );
    c.push(Collider {
        aabb: gate_box,
        obj: Some(GATE),
    });
    // The lantern's plinth.
    let (lx, lz) = LANTERN;
    solid(&mut c, Vec3::new(lx, 0.5, lz), Vec3::new(0.4, 0.5, 0.4));
    // Snow mounds and boulders (I-049).
    for a in prop_boxes() {
        c.push(Collider { aabb: a, obj: None });
    }
    let objs = vec![
        Obj::new(Kind::Stream, [0.0, zc], [h, (z1 - z0) / 2.0]),
        Obj::new(Kind::IceWall, [0.0, RIDGE_Z], [GATE_HALF_X, 0.5]),
        Obj::new(Kind::Lantern, [lx, lz], [0.4, 0.4]),
    ];
    Layout {
        colliders: c,
        world: chem::World::with_goal(objs, Goal::Reach),
        boxes: vec![
            stream_box,
            gate_box,
            Aabb::new(Vec3::new(lx, 1.0, lz), Vec3::new(0.45, 1.0, 0.45)),
        ],
        melted: vec![
            None,
            Some(Aabb::new(
                Vec3::new(0.0, 0.15, RIDGE_Z),
                Vec3::new(GATE_HALF_X, 0.25, 1.4),
            )),
            None,
        ],
    }
}

/// I-037: level 2 as the player left it, for the walk back from the Marsh
/// (I-037 break 1, B1). The stream is frozen solid and the gate melted with
/// the game's own Frost and Fire rules (so every visual and collider follows
/// from the state), and the lantern counts as reached: the win is latched, so
/// walking up to it cannot win the level again, and the banner it shows
/// describes the state on screen (gate open, stream frozen).
pub fn solved(w: &mut chem::World) {
    let mut ev = Vec::new();
    for _ in 0..8 {
        if w.objs[STREAM].ice >= 1.0 {
            break;
        }
        w.apply(STREAM, chem::Spell::Frost, &mut ev);
    }
    for _ in 0..8 {
        if !w.objs[GATE].wall {
            break;
        }
        w.apply(GATE, chem::Spell::Fire, &mut ev);
    }
    w.reach();
}

/// When the stream turns solid (liquid again) under a player standing on
/// it, the bank landing to move them to: the nearer one, x clamped to the
/// landing. `None` while the stream is frozen or the player is clear of it.
pub fn bank_push(pos: Vec3, w: &chem::World) -> Option<Vec3> {
    let o = w.objs.iter().find(|o| o.kind == Kind::Stream)?;
    if !world::solid_obj(o) {
        return None;
    }
    let (z0, z1) = STREAM_Z;
    let r = crate::player::RADIUS;
    let inside = pos.z > z0 - r && pos.z < z1 + r && pos.y < STREAM_H;
    inside.then(|| {
        let z = if pos.z >= (z0 + z1) / 2.0 {
            LANDING_SOUTH
        } else {
            LANDING_NORTH
        };
        Vec3::new(pos.x.clamp(-LANDING_X, LANDING_X), 0.0, z)
    })
}

/// Level 2's caption for an event on `obj`, at the player's band (`None`
/// reads as Apprentice), with the Sprout picture (`"freeze"`, `"melt"`,
/// `"wrong"`, or `""`). `None` for events level 2 has no line for (the
/// courtyard's caption is used).
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
        (E::Freezing, Kind::Stream) => pick(
            [
                "Cold! The water is turning to ice.",
                "See the drops skate? Liquid nitrogen makes a cushion of its own gas and rides on it.",
                E::Freezing.caption(),
                "Film boiling (Leidenfrost): a layer of N₂ gas insulates each 77 K drop and slows the heat flow. Freezing water must lose 6.01 kJ/mol.",
            ],
            "freeze",
        ),
        (E::Froze, Kind::Stream) => pick(
            [
                "Ice bridge! Now walk across.",
                "The water froze into solid ice. A solid keeps its shape, so it holds you up.",
                E::Froze.caption(),
                "Frozen: H₂O(l) → H₂O(s) at 0 °C (273 K). Liquid nitrogen at 77 K is 196 K colder, so heat kept flowing out.",
            ],
            "freeze",
        ),
        (E::Melting, Kind::IceWall) => pick(
            [
                "Hot! The ice is melting.",
                "Heat is melting the ice into water.",
                E::Melting.caption(),
                "Ice stays at 0 °C while it melts: the heat (6.01 kJ/mol) breaks up the solid instead of warming it.",
            ],
            "melt",
        ),
        (E::Melted, Kind::IceWall) => pick(
            [
                "The ice door melted! Go through.",
                "The ice melted: solid ice became liquid water.",
                E::Melted.caption(),
                "Melted at 0 °C (273 K): H₂O(s) → H₂O(l). Same molecules, new state.",
            ],
            "melt",
        ),
        (E::Melting | E::Melted, Kind::Stream) => two(
            "The bridge melted! Make it cold again.",
            "The ice melted back into liquid water. Freeze it again to cross.",
            "melt",
        ),
        (E::Chilled, Kind::Stream | Kind::IceWall) => two(
            "Brrr! Cold makes ice colder. Try heat!",
            "Cold cannot melt ice. Ice needs heat to turn into water.",
            "wrong",
        ),
        (E::Steam, Kind::Stream) => two(
            "Hot water is still water. Try cold!",
            "Heat cannot freeze water. To make a solid, take heat away.",
            "wrong",
        ),
        (E::Soaked, Kind::Stream | Kind::IceWall) => two(
            "More water! Try cold or heat.",
            "Adding water does not change its state. Heat melts ice, cold freezes water.",
            "wrong",
        ),
        (E::NoChange, Kind::Stream | Kind::IceWall) => two(
            "That is a gas. Try cold or heat.",
            E::NoChange.caption(),
            "wrong",
        ),
        _ => None,
    }
}

/// [`caption`] in play (I-049): `bridge` says whether the stream has been
/// frozen solid since the last reset. Before that, Fire on a part-frozen
/// stream gives Sprout "The ice melted!", not "The bridge melted!" (I-035
/// break 1, should-fix 4); and any spell on the lantern gives
/// [`LANTERN_LINE`] instead of the courtyard's lines (should-fix 6).
pub fn caption_at(
    kind: EventKind,
    obj: Kind,
    band: Option<Tier>,
    bridge: bool,
) -> Option<(&'static str, &'static str)> {
    if obj == Kind::Lantern {
        return caption(kind, obj, band).or(Some((LANTERN_LINE, "")));
    }
    let line = caption(kind, obj, band);
    match line {
        Some(("The bridge melted! Make it cold again.", icon)) if !bridge => {
            Some(("The ice melted! Make it cold again.", icon))
        }
        _ => line,
    }
}

/// The two ice crusts that grow from the banks inward as the stream freezes.
#[derive(Component)]
pub struct IceCrust {
    pub north: bool,
}

/// Marks the stream's water (its material's texture scrolls slowly:
/// ripples). A component, not a resource, so it goes with the level.
#[derive(Component)]
pub struct Ripple;

/// Spawns the valley. Every entity carries [`LevelRoot`].
pub fn build(
    commands: &mut Commands,
    kit: &Kit,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> (Colliders, Chem) {
    let lay = layout();
    let h = world::HALF;
    let wh = world::WALL_H;
    let snow_tex = world::grass_texture(images);
    let stone_tex = world::stone_texture(images);
    let snow = world::textured(
        materials,
        Color::srgb(0.93, 0.96, 1.0),
        &snow_tex,
        Vec2::splat(25.0),
    );
    let wall_tint = Color::srgb(0.74, 0.78, 0.9);
    let wall_long = world::textured(materials, wall_tint, &stone_tex, Vec2::new(25.0, 2.0));
    let cap = world::toon(materials, Color::srgb(0.98, 0.99, 1.0));
    let rock = world::toon(materials, Color::srgb(0.5, 0.52, 0.62));
    let rock_light = world::toon(materials, Color::srgb(0.66, 0.68, 0.78));
    let path = world::toon(materials, Color::srgb(0.8, 0.86, 0.95));
    let wood = world::toon(materials, Color::srgb(0.5, 0.34, 0.22));
    let pine = world::toon(materials, Color::srgb(0.16, 0.45, 0.36));
    let lamp = world::glow(materials, Color::srgb(1.0, 0.82, 0.4), 6.0);
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.95, 1.0, 0.35),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        ..default()
    });
    let leaf = world::toon(materials, Color::srgb(0.3, 0.75, 0.4));
    // Ripples: soft bands across the stream, scrolled by `ripple`.
    let ripple_tex = world::texture(images, 64, |x, y| {
        let a = (y as f32 / 64.0 * std::f32::consts::TAU * 2.0).sin();
        let b = ((x as f32 / 64.0 + y as f32 / 128.0) * std::f32::consts::TAU).sin();
        0.78 + 0.12 * a + 0.06 * b
    });
    let water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.6, 0.95, 0.9),
        base_color_texture: Some(ripple_tex),
        uv_transform: Affine2::from_scale(Vec2::new(12.0, 1.0)),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.08,
        reflectance: 0.6,
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

    // Ground and trampled-snow paths.
    b.piece(
        &cube,
        &snow,
        Transform::from_xyz(0.0, -0.25, 0.0).with_scale(Vec3::new(
            2.0 * h + 2.0,
            0.5,
            2.0 * h + 2.0,
        )),
    );
    for (z0, z1) in [(8.0f32, 22.0f32), (-8.0, 4.0), (-17.0, -8.0)] {
        b.piece(
            &cube,
            &path,
            Transform::from_xyz(0.0, 0.01, (z0 + z1) / 2.0).with_scale(Vec3::new(
                3.0,
                0.02,
                z1 - z0,
            )),
        );
    }

    // Perimeter walls with snow caps (colliders come from `layout`).
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
            &cap,
            Transform::from_translation(Vec3::new(c.x, wh + 0.15, c.z))
                .with_rotation(rot)
                .with_scale(Vec3::new(wl, 0.3, 1.4)),
        );
    }

    // The way back: the arch, with a little greenhouse on its lintel.
    world::arch(&mut b, materials, &rock);
    {
        let (x, z) = BACK_ARCH;
        b.piece(
            &cube,
            &glass,
            Transform::from_xyz(x, 4.35, z + 0.4).with_scale(Vec3::new(0.9, 0.7, 0.6)),
        );
        b.piece(
            &ball,
            &leaf,
            Transform::from_xyz(x, 4.25, z + 0.4).with_scale(Vec3::splat(0.4)),
        );
    }

    // I-037: the way on, the north arch (solid posts and lintel; the door
    // panel glows once the lantern is reached), with a reed on its lintel.
    {
        let (x, z) = NEXT_ARCH;
        for dx in [-1.75f32, 1.75] {
            b.block(
                Vec3::new(x + dx, 1.75, z - 0.4),
                Vec3::new(0.5, 3.5, 0.7),
                &rock,
            );
        }
        b.piece(
            &cube,
            &rock,
            Transform::from_xyz(x, 3.7, z - 0.4).with_scale(Vec3::new(4.0, 0.5, 0.8)),
        );
        b.piece(
            &cone,
            &pine,
            Transform::from_xyz(x, 4.3, z - 0.4).with_scale(Vec3::new(0.4, 0.8, 0.4)),
        );
        let closed = world::toon(materials, Color::srgb(0.22, 0.24, 0.32));
        b.commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(closed),
            Transform::from_xyz(x, 1.5, z - 0.5).with_scale(Vec3::new(3.0, 3.0, 0.12)),
            NotShadowCaster,
            NextDoor,
            LevelRoot,
        ));
    }

    // Rock banks with snow on top, and the ridge.
    for col in &lay.colliders {
        if col.obj.is_some() {
            continue;
        }
        let a = col.aabb;
        let (c, s) = (a.center(), a.max - a.min);
        let is_bank = s.y < 1.5;
        let is_ridge = (c.z - RIDGE_Z).abs() < 0.01 && s.x < 2.0 * h;
        if !(is_bank || is_ridge) {
            continue;
        }
        b.piece(&cube, &rock, Transform::from_translation(c).with_scale(s));
        b.piece(
            &cube,
            &cap,
            Transform::from_xyz(c.x, a.max.y + 0.08, c.z).with_scale(Vec3::new(
                s.x,
                0.16,
                s.z + 0.2,
            )),
        );
    }
    // Boulders by the landings (their colliders are in `layout`).
    for (x, z, s) in BOULDERS {
        b.piece(
            &ball,
            &rock_light,
            Transform::from_xyz(x, 1.2, z).with_scale(Vec3::new(s * 1.3, s, s)),
        );
    }

    // Snowy pines (solid trunks).
    for (x, z, s) in [
        (-16.0f32, 16.0f32, 1.2f32),
        (15.0, 14.0, 1.0),
        (-18.0, -2.0, 1.1),
        (17.0, -3.0, 1.2),
        (-14.0, -18.0, 1.0),
        (13.0, -19.0, 1.1),
        (-9.0, 13.0, 0.8),
        (10.0, 18.0, 0.9),
    ] {
        b.piece(
            &cyl,
            &wood,
            Transform::from_xyz(x, 0.6 * s, z).with_scale(Vec3::new(0.4 * s, 1.2 * s, 0.4 * s)),
        );
        b.piece(
            &cone,
            &pine,
            Transform::from_xyz(x, 2.4 * s, z).with_scale(Vec3::new(2.6 * s, 3.2 * s, 2.6 * s)),
        );
        b.piece(
            &cone,
            &cap,
            Transform::from_xyz(x, 3.5 * s, z).with_scale(Vec3::new(1.2 * s, 1.1 * s, 1.2 * s)),
        );
        b.colliders.push(Collider {
            aabb: Aabb::new(
                Vec3::new(x, 1.5 * s, z),
                Vec3::new(0.3 * s, 1.5 * s, 0.3 * s),
            ),
            obj: None,
        });
    }
    // Snow mounds (their colliders are in `layout`).
    for (x, z, s) in MOUNDS {
        b.piece(
            &ball,
            &cap,
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::new(s * 1.6, s * 0.7, s * 1.3)),
        );
    }

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
    let target = |name: &'static str, i: usize| world::Target {
        name,
        aabb: lay.boxes[i],
        melted_aabb: lay.melted[i],
        bodies: vec![],
        flames: vec![],
        foam: None,
        wall: None,
        puddle: None,
        mats: plain(&kit.water),
        last: None,
    };
    let (z0, z1) = STREAM_Z;
    // The stream: rippling water and the two ice crusts.
    b.commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(water),
        Transform::from_xyz(0.0, 0.02, (z0 + z1) / 2.0).with_scale(Vec3::new(
            2.0 * h,
            0.04,
            z1 - z0,
        )),
        NotShadowCaster,
        Ripple,
        LevelRoot,
    ));
    for north in [false, true] {
        b.commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(kit.ice.clone()),
            Transform::from_xyz(0.0, 0.06, if north { z0 } else { z1 }).with_scale(Vec3::ZERO),
            Visibility::Hidden,
            NotShadowCaster,
            IceCrust { north },
            LevelRoot,
        ));
    }
    let stream = target(NAMES[STREAM], STREAM);

    // The ice gate (melts into a puddle in the opening).
    let gate_wall = b.piece(
        &cube,
        &kit.ice,
        Transform::from_xyz(0.0, GATE_H / 2.0, RIDGE_Z).with_scale(Vec3::new(
            2.0 * GATE_HALF_X,
            GATE_H,
            1.0,
        )),
    );
    let gate_puddle = b
        .commands
        .spawn((
            Mesh3d(cyl.clone()),
            MeshMaterial3d(kit.water.clone()),
            Transform::from_xyz(0.0, 0.03, RIDGE_Z).with_scale(Vec3::new(4.2, 0.04, 2.8)),
            Visibility::Hidden,
            NotShadowCaster,
            LevelRoot,
        ))
        .id();
    let mut gate = target(NAMES[GATE], GATE);
    gate.wall = Some(gate_wall);
    gate.puddle = Some(gate_puddle);
    gate.mats = plain(&kit.ice);

    // The lantern on its plinth.
    let (lx, lz) = LANTERN;
    b.piece(
        &cube,
        &rock_light,
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
    let mut lantern = target(NAMES[LANTERN_OBJ], LANTERN_OBJ);
    lantern.bodies = vec![light];
    lantern.mats = plain(&lamp);

    let mut colliders = lay.colliders;
    colliders.append(&mut b.colliders);
    (
        Colliders(colliders),
        Chem {
            initial: lay.world.clone(),
            world: lay.world,
            targets: vec![stream, gate, lantern],
        },
    )
}

/// Level 2's moving parts each frame: the ice crusts follow the stream's
/// ice, and the water's ripples scroll (static when `still`, which follows
/// the Hand sway setting: off under prefers-reduced-motion).
pub fn sync<F: bevy::ecs::query::QueryFilter>(
    chem: &Chem,
    t: f32,
    still: bool,
    crusts: &mut Query<(&mut Transform, &mut Visibility, &IceCrust), F>,
    ripple: Option<&Handle<StandardMaterial>>,
    materials: &mut Assets<StandardMaterial>,
) {
    let Some(o) = chem
        .world
        .objs
        .get(STREAM)
        .filter(|o| o.kind == Kind::Stream)
    else {
        return;
    };
    let (z0, z1) = STREAM_Z;
    // Each crust reaches half way across at ice 1, so they meet in the middle.
    let d = (z1 - z0) / 2.0 * o.ice.clamp(0.0, 1.0);
    for (mut tr, mut vis, c) in crusts.iter_mut() {
        let want = if d > 0.01 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        let z = if c.north { z0 + d / 2.0 } else { z1 - d / 2.0 };
        let scale = Vec3::new(2.0 * world::HALF, 0.1, d.max(0.001));
        if tr.scale != scale || tr.translation.z != z {
            tr.scale = scale;
            tr.translation.z = z;
        }
    }
    if let Some(r) = ripple {
        if !still && o.ice < 1.0 {
            if let Some(mut m) = materials.get_mut(r) {
                m.uv_transform = Affine2::from_scale_angle_translation(
                    Vec2::new(12.0, 1.0),
                    0.0,
                    Vec2::new(t * 0.05, t * 0.08),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chem::Spell;
    use crate::player::{MoveInput, Player};

    // ---- I-035 acceptance test 3: the caption table ----

    const SPROUT: Option<Tier> = Some(Tier::Sprout);
    const SEEDLING: Option<Tier> = Some(Tier::Seedling);
    const APPRENTICE: Option<Tier> = Some(Tier::Apprentice);
    const ADEPT: Option<Tier> = Some(Tier::Adept);

    /// (event, object, Sprout, Seedling, Apprentice, Adept), exactly as the spec's table.
    fn table() -> Vec<(EventKind, Kind, [&'static str; 4])> {
        let froze = EventKind::Froze.caption();
        let melting = EventKind::Melting.caption();
        let melted = EventKind::Melted.caption();
        let stream_melt = "The ice melted back into liquid water. Freeze it again to cross.";
        let chilled = "Cold cannot melt ice. Ice needs heat to turn into water.";
        let soaked = "Adding water does not change its state. Heat melts ice, cold freezes water.";
        let co2 = "CO₂ keeps oxygen away. That does not melt ice or freeze water here.";
        let mut t = vec![
            (
                EventKind::Freezing,
                Kind::Stream,
                [
                    "Cold! The water is turning to ice.",
                    "See the drops skate? Liquid nitrogen makes a cushion of its own gas and rides on it.",
                    "Freezing: H₂O(l) → H₂O(s). The nitrogen drops glide on their own gas (the Leidenfrost effect), so it takes a few casts.",
                    "Film boiling (Leidenfrost): a layer of N₂ gas insulates each 77 K drop and slows the heat flow. Freezing water must lose 6.01 kJ/mol.",
                ],
            ),
            (
                EventKind::Froze,
                Kind::Stream,
                [
                    "Ice bridge! Now walk across.",
                    "The water froze into solid ice. A solid keeps its shape, so it holds you up.",
                    froze,
                    "Frozen: H₂O(l) → H₂O(s) at 0 °C (273 K). Liquid nitrogen at 77 K is 196 K colder, so heat kept flowing out.",
                ],
            ),
            (
                EventKind::Melting,
                Kind::IceWall,
                [
                    "Hot! The ice is melting.",
                    "Heat is melting the ice into water.",
                    melting,
                    "Ice stays at 0 °C while it melts: the heat (6.01 kJ/mol) breaks up the solid instead of warming it.",
                ],
            ),
            (
                EventKind::Melted,
                Kind::IceWall,
                [
                    "The ice door melted! Go through.",
                    "The ice melted: solid ice became liquid water.",
                    melted,
                    "Melted at 0 °C (273 K): H₂O(s) → H₂O(l). Same molecules, new state.",
                ],
            ),
            (
                EventKind::Steam,
                Kind::Stream,
                [
                    "Hot water is still water. Try cold!",
                    "Heat cannot freeze water. To make a solid, take heat away.",
                    "Heat cannot freeze water. To make a solid, take heat away.",
                    "Heat cannot freeze water. To make a solid, take heat away.",
                ],
            ),
        ];
        for k in [EventKind::Melting, EventKind::Melted] {
            t.push((
                k,
                Kind::Stream,
                [
                    "The bridge melted! Make it cold again.",
                    stream_melt,
                    stream_melt,
                    stream_melt,
                ],
            ));
        }
        for obj in [Kind::Stream, Kind::IceWall] {
            t.push((
                EventKind::Chilled,
                obj,
                [
                    "Brrr! Cold makes ice colder. Try heat!",
                    chilled,
                    chilled,
                    chilled,
                ],
            ));
            t.push((
                EventKind::Soaked,
                obj,
                ["More water! Try cold or heat.", soaked, soaked, soaked],
            ));
            t.push((
                EventKind::NoChange,
                obj,
                ["That is a gas. Try cold or heat.", co2, co2, co2],
            ));
        }
        t
    }

    fn icon_for(k: EventKind) -> &'static str {
        match k {
            EventKind::Freezing | EventKind::Froze => "freeze",
            EventKind::Melting | EventKind::Melted => "melt",
            _ => "wrong",
        }
    }

    #[test]
    fn i035_caption_table_is_exact_per_band() {
        for (k, obj, lines) in table() {
            for (band, want) in [SPROUT, SEEDLING, APPRENTICE, ADEPT].into_iter().zip(lines) {
                let got = caption(k, obj, band);
                let icon = if band == SPROUT { icon_for(k) } else { "" };
                assert_eq!(got, Some((want, icon)), "{k:?} on {obj:?} at {band:?}");
            }
            assert_eq!(
                caption(k, obj, None),
                caption(k, obj, APPRENTICE),
                "no band reads as Apprentice ({k:?} on {obj:?})"
            );
        }
        // Spells on the lantern say nothing new.
        for k in [EventKind::Chilled, EventKind::Soaked, EventKind::Smothered] {
            assert_eq!(caption(k, Kind::Lantern, SPROUT), None);
        }
    }

    #[test]
    fn i035_caption_strings_are_safe_player_text() {
        for (k, obj, _) in table() {
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

    // ---- I-035 acceptance test 4: the walk to the lantern ----

    fn walk(w: &chem::World, cols: &[Collider], from: Vec3, secs: f32) -> Player {
        let colliders = Colliders(cols.to_vec());
        let solids = crate::player::boxes(colliders.solid(w));
        let mut p = Player {
            pos: from,
            ..Player::default()
        };
        let input = MoveInput {
            wish: Vec2::new(0.0, 1.0),
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

    #[test]
    fn i035_the_stream_and_the_gate_block_the_walk_until_changed() {
        let lay = layout();
        let mut w = lay.world.clone();
        assert_eq!(w.objs.len(), NAMES.len());
        // Liquid stream: stopped on the south bank.
        let p = walk(&w, &lay.colliders, START, 12.0);
        assert!(
            p.pos.z > LANDING_SOUTH - 0.2,
            "stopped by the stream, z={}",
            p.pos.z
        );
        // Frozen stream, standing gate: stopped at the gate.
        for _ in 0..3 {
            w.apply(STREAM, Spell::Frost, &mut Vec::new());
        }
        let p = walk(&w, &lay.colliders, START, 12.0);
        assert!(
            p.pos.z < STREAM_Z.0 - 0.5 && p.pos.z > RIDGE_Z + 0.5,
            "crossed the stream and stopped at the gate, z={}",
            p.pos.z
        );
        assert!(!near_lantern(&p));
        // Melted gate: the lantern is reached.
        for _ in 0..3 {
            w.apply(GATE, Spell::Fire, &mut Vec::new());
        }
        let p = walk(&w, &lay.colliders, START, 12.0);
        assert!(near_lantern(&p), "reached the lantern, pos={:?}", p.pos);
    }

    #[test]
    fn i037_back_from_the_marsh_level_2_is_already_solved() {
        let lay = layout();
        let mut w = lay.world.clone();
        solved(&mut w);
        assert!(
            w.objs[STREAM].frozen() && w.objs[STREAM].ice >= 1.0,
            "stream frozen solid"
        );
        assert!(!w.objs[GATE].wall && !w.objs[GATE].frozen(), "gate melted");
        assert_eq!(
            w.outcome(),
            chem::Outcome::Won,
            "the lantern win is latched"
        );
        // The way is really open: from beside the arrival point (clear of
        // the lantern's plinth) the walk south crosses the melted gate and
        // the frozen stream to the start bank.
        let mut p = walk(&w, &lay.colliders, ARRIVE_NORTH + Vec3::X, 0.0);
        p.yaw = std::f32::consts::PI;
        let solids = crate::player::boxes(Colliders(lay.colliders.clone()).solid(&w));
        let input = MoveInput {
            wish: Vec2::new(0.0, 1.0),
            ..MoveInput::default()
        };
        for _ in 0..(14.0 * 30.0) as usize {
            p.step(input, 1.0 / 30.0, &solids, &[]);
        }
        assert!(
            p.pos.z > STREAM_Z.1 + 0.5,
            "walked back over the stream, pos={:?}",
            p.pos
        );
    }

    #[test]
    fn i035_melting_the_bridge_under_the_player_moves_them_to_a_landing() {
        let lay = layout();
        let colliders = Colliders(lay.colliders.clone());
        for from in [
            Vec3::new(0.5, 0.0, 5.0),
            Vec3::new(10.0, 0.0, 7.5),
            Vec3::new(-15.0, 0.0, 4.2),
            Vec3::new(-1.0, 0.0, 6.0),
        ] {
            let mut w = lay.world.clone();
            for _ in 0..3 {
                w.apply(STREAM, Spell::Frost, &mut Vec::new());
            }
            assert_eq!(bank_push(from, &w), None, "frozen: nothing to do");
            // The player stands on the bridge; one Fire hit lands.
            let mut p = walk(&w, &lay.colliders, from, 0.0);
            w.apply(STREAM, Spell::Fire, &mut Vec::new());
            assert!(crate::world::solid_obj(&w.objs[STREAM]));
            if let Some(to) = bank_push(p.pos, &w) {
                p.pos = to;
            }
            let want_z = if from.z >= 6.0 {
                LANDING_SOUTH
            } else {
                LANDING_NORTH
            };
            assert!((p.pos.z - want_z).abs() < 1e-4, "{from:?} -> {:?}", p.pos);
            assert!(p.pos.x.abs() <= LANDING_X + 1e-4, "{from:?} -> {:?}", p.pos);
            let r = crate::player::RADIUS;
            let body = Aabb::new(p.pos + Vec3::Y * 0.9, Vec3::new(r, 0.875, r));
            for s in colliders.solid(&w) {
                let overlap = body.min.cmplt(s.max).all() && body.max.cmpgt(s.min).all();
                assert!(!overlap, "{from:?} -> {:?} is inside {s:?}", p.pos);
            }
        }
        // Clear of the stream: nothing to do.
        let w = lay.world.clone();
        assert_eq!(bank_push(START, &w), None);
    }

    // ---- I-049: polish ----

    #[test]
    fn i049_sprout_melt_line_says_bridge_only_after_a_bridge() {
        for k in [EventKind::Melting, EventKind::Melted] {
            assert_eq!(
                caption_at(k, Kind::Stream, SPROUT, false),
                Some(("The ice melted! Make it cold again.", "melt")),
                "{k:?} before any bridge"
            );
            assert_eq!(
                caption_at(k, Kind::Stream, SPROUT, true),
                Some(("The bridge melted! Make it cold again.", "melt")),
                "{k:?} after a bridge"
            );
            // Older bands' line never says bridge: unchanged either way.
            for band in [SEEDLING, APPRENTICE, ADEPT, None] {
                for bridge in [false, true] {
                    assert_eq!(caption_at(k, Kind::Stream, band, bridge), caption(k, Kind::Stream, band));
                }
            }
        }
        // Every other row of the table is unchanged.
        for (k, obj, _) in table() {
            for band in [None, SPROUT, SEEDLING, APPRENTICE, ADEPT] {
                assert_eq!(caption_at(k, obj, band, true), caption(k, obj, band), "{k:?} {obj:?} {band:?}");
            }
        }
    }

    #[test]
    fn i049_spells_on_the_lantern_never_talk_about_burning() {
        for k in [
            EventKind::Soaked,
            EventKind::Chilled,
            EventKind::Smothered,
            EventKind::NoChange,
            EventKind::Steam,
        ] {
            for band in [None, SPROUT, SEEDLING, APPRENTICE, ADEPT] {
                let (line, icon) = caption_at(k, Kind::Lantern, band, false).expect("a lantern line");
                assert_eq!((line, icon), (LANTERN_LINE, ""), "{k:?} at {band:?}");
                assert!(!line.contains("burn") && !line.contains("dry"), "{line}");
            }
        }
    }

    /// Walks from `from` with yaw `yaw` (0 faces -z) for `secs`.
    fn walk_yaw(cols: &[Collider], from: Vec3, yaw: f32, secs: f32) -> Player {
        let w = layout().world;
        let solids = crate::player::boxes(Colliders(cols.to_vec()).solid(&w));
        let mut p = Player {
            pos: from,
            yaw,
            ..Player::default()
        };
        let input = MoveInput {
            wish: Vec2::new(0.0, 1.0),
            ..MoveInput::default()
        };
        for _ in 0..(secs * 30.0) as usize {
            p.step(input, 1.0 / 30.0, &solids, &[]);
        }
        p
    }

    #[test]
    fn i049_snow_mounds_stop_the_walk_outside_the_snow() {
        let lay = layout();
        for (x, z, s) in MOUNDS {
            // Walk at the mound from 4 m south (+z) and from 4 m east (+x).
            for (from, yaw) in [
                (Vec3::new(x, 0.0, z + 4.0), 0.0f32),
                (Vec3::new(x + 4.0, 0.0, z), std::f32::consts::FRAC_PI_2),
            ] {
                let edge = world::HALF - crate::player::RADIUS;
                if from.x.abs() > edge || from.z.abs() > edge {
                    continue;
                }
                let p = walk_yaw(&lay.colliders, from, yaw, 3.0);
                // The eye (straight above the feet) is outside the half
                // ellipsoid's footprint at ground level.
                let (rx, rz) = (0.8 * s, 0.65 * s);
                let d = ((p.pos.x - x) / rx).powi(2) + ((p.pos.z - z) / rz).powi(2);
                assert!(d > 1.0, "mound ({x}, {z}): walked to {:?}", p.pos);
            }
        }
    }

    #[test]
    fn i049_prop_colliders_leave_every_path_open() {
        let boxes = prop_boxes();
        let r = crate::player::RADIUS;
        let clear = |p: Vec3| {
            boxes.iter().all(|a| {
                p.x < a.min.x - r || p.x > a.max.x + r || p.z < a.min.z - r || p.z > a.max.z + r
            })
        };
        // Arrival points, the landings (every x on them), the gate opening, the lantern.
        for p in [START, ARRIVE_NORTH, Vec3::new(LANTERN.0, 0.0, LANTERN.1 + LANTERN_REACH)] {
            assert!(clear(p), "{p:?}");
        }
        for k in 0..=8 {
            let x = -LANDING_X + k as f32 * LANDING_X / 4.0;
            for z in [LANDING_SOUTH, LANDING_NORTH, RIDGE_Z] {
                assert!(clear(Vec3::new(x, 0.0, z)), "x {x} z {z}");
            }
        }
        // The level-2 acceptance walk still reaches the lantern (melted gate,
        // frozen stream) with the props in place.
        let lay = layout();
        let mut w = lay.world.clone();
        let mut ev = Vec::new();
        for _ in 0..3 {
            w.apply(STREAM, Spell::Frost, &mut ev);
            w.apply(GATE, Spell::Fire, &mut ev);
        }
        let p = walk(&w, &lay.colliders, START, 12.0);
        assert!(near_lantern(&p), "reached the lantern, pos={:?}", p.pos);
    }
}
