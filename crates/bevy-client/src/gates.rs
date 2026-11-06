//! The biome gates (I-052 round 2; Paawan's partial mastery, 2026-11-01):
//! a runestone wall across the one way into each biome, which sinks away
//! once the previous biome's spell is Practised in this run
//! (`valley::practice`): the Frozen Fields need Water, the Marsh needs
//! Frost, the Crystal Cave needs CO2. Each gate has a sign the page draws
//! over it ("Cast Water 2 times anywhere to open"). The walls are physical, like
//! every gate in the valley (spec section 7): they are protected blocks,
//! not quest checks. Voxel mode only (registered by `story`).

use bevy::prelude::*;
use realgame_sim::valley::practice::{self, PRACTISE_CASTS};
use realgame_voxel::{self as vox, mat, Aabb, Cell, World};

use crate::story::Story;
use crate::voxel_view::VoxWorld;

/// One gate: the biome it opens, the box its wall fills (only the cells
/// that are open ground there: air, snow layers are left), and where its
/// sign hangs.
#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub biome: &'static str,
    pub wall: Aabb,
    pub sign: [f32; 3],
}

/// The three gates, in the order the valley is played.
pub const GATES: [Gate; 3] = [
    // The ford's mouth on the Village bank: the first bank step (z 158),
    // right at the water, from canyon wall to canyon wall. The river
    // outside the ford flows (`SOURCE`) and does not freeze, so the ford
    // is the one way north.
    Gate {
        biome: "frozen",
        wall: Aabb {
            min: [120, 32, 158],
            max: [136, 40, 159],
        },
        sign: [128.0, 40.6, 158.5],
    },
    // The east pass through the ridge between the Frozen Fields and the
    // Marsh (x 132..138, z 66..74), at its west end.
    Gate {
        biome: "marsh",
        wall: Aabb {
            min: [132, 30, 66],
            // As high as the protected ridge on either side (y 61), so it
            // cannot be pillared over more cheaply than the ridge
            // (break-2 should-fix 1).
            max: [133, 62, 74],
        },
        sign: [132.5, 47.0, 70.0],
    },
    // The Crystal Cave's mouth in the Marsh (`gen::CAVE_MOUTH`).
    Gate {
        biome: "cave",
        wall: Aabb {
            min: [218, 30, 56],
            // Above the rocks around the mouth (y 38; break-2 should-fix 1).
            max: [223, 40, 61],
        },
        sign: [220.5, 37.0, 58.5],
    },
];

/// The gates' state: which are shut, and the cells each wall filled.
#[derive(Resource, Default)]
pub struct Gates {
    pub shut: [bool; 3],
    filled: [Vec<[i32; 3]>; 3],
}

/// Fills gate `g`'s wall: every non-solid cell in its box becomes runestone,
/// and the box is protected. Returns the cells it filled.
pub fn build(world: &mut World, g: &Gate) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for z in g.wall.min[2]..g.wall.max[2] {
        for y in g.wall.min[1]..g.wall.max[1] {
            for x in g.wall.min[0]..g.wall.max[0] {
                let p = [x, y, z];
                let c = world.get(p);
                if !vox::materials::flags(c.mat).solid || c.mat == mat::WATER {
                    world.set(p, Cell::new(mat::RUNESTONE));
                    out.push(p);
                }
            }
        }
    }
    world.protect(g.wall);
    // break-2 should-fix 1: nothing can be built on the wall or dug under
    // it, so the only way past is the spell.
    let (a, b) = (g.wall.min, g.wall.max);
    world.protect(Aabb {
        min: [a[0], b[1], a[2]],
        max: [b[0], b[1] + WARD_UP, b[2]],
    });
    world.protect(Aabb {
        min: [a[0] - 1, a[1] - WARD_DOWN, a[2] - 1],
        max: [b[0] + 1, a[1], b[2] + 1],
    });
    out
}

/// Blocks above each wall that cannot be built on.
pub const WARD_UP: i32 = 16;
/// Blocks under each wall (and one beside it) that cannot be dug.
pub const WARD_DOWN: i32 = 8;

/// Opens a gate: the cells its wall filled become air again.
pub fn open(world: &mut World, cells: &[[i32; 3]]) {
    for &p in cells {
        if world.get(p).mat == mat::RUNESTONE {
            world.set(p, Cell::AIR);
        }
    }
}

/// Whether gate `biome` is open: its spell is Practised in this run, or
/// the saved progress already has it at Practised or above.
pub fn is_open(biome: &str, story: &Story, saved: &crate::Progress) -> bool {
    let Some(spell) = practice::gate_spell(biome) else {
        return true;
    };
    // The cave opens only once the methane hollow is crossed (break-2
    // should-fix 4: spec section 7, "the hollow only when smothered"), so
    // walking round the hollow does not skip it.
    if biome == "cave" && !hollow_done(story) {
        return false;
    }
    practice::biome_open(biome, &story.practice)
        || saved
            .0
            .get(spell)
            .is_some_and(|p| p.stage >= crate::progress::Stage::Practised)
}

/// The sign's words for a shut gate.
/// The quest is past the methane hollow (step 5).
fn hollow_done(story: &Story) -> bool {
    story.quest.step() > 5
}

/// The cave gate's sign once CO2 is practised but the hollow still burns.
pub const HOLLOW_SIGN: &str = "Put out the methane hollow's flames to open";

pub fn sign_text(biome: &str, story: &Story) -> String {
    let Some(spell) = practice::gate_spell(biome) else {
        return String::new();
    };
    if biome == "cave" && practice::biome_open(biome, &story.practice) && !hollow_done(story) {
        return HOLLOW_SIGN.to_string();
    }
    let name = crate::chem::Spell::ALL[spell].name();
    let n = story.practice.casts[spell].min(PRACTISE_CASTS);
    // P10: say what practising is: any correct cast, anywhere.
    format!("Cast {name} {PRACTISE_CASTS} times anywhere to open ({n}/{PRACTISE_CASTS})")
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Gates>()
        .add_systems(Startup, setup.after(crate::setup))
        .add_systems(
            Update,
            run.after(crate::aim_and_cast).before(crate::publish),
        );
}

fn setup(mut world: ResMut<VoxWorld>, mut gates: ResMut<Gates>) {
    for (i, g) in GATES.iter().enumerate() {
        gates.filled[i] = build(&mut world.0, g);
        gates.shut[i] = true;
    }
}

/// Opens each shut gate whose spell is Practised.
fn run(
    mut world: ResMut<VoxWorld>,
    mut gates: ResMut<Gates>,
    mut story: ResMut<Story>,
    saved: Res<crate::Progress>,
    mut game: ResMut<crate::Game>,
    mut matter: Option<ResMut<crate::blockchem::Matter>>,
) {
    for (i, g) in GATES.iter().enumerate() {
        if !gates.shut[i] || !is_open(g.biome, &story, &saved) {
            continue;
        }
        gates.shut[i] = false;
        let cells = std::mem::take(&mut gates.filled[i]);
        open(&mut world.0, &cells);
        if let Some(m) = matter.as_mut() {
            for &p in &cells {
                m.world.touch(&world.0, p);
            }
        }
        game.say(&format!(
            "The runestone gate to {} sinks into the ground!",
            realgame_sim::valley::biome_name(g.biome)
        ));
    }
    let view: Vec<String> = GATES
        .iter()
        .enumerate()
        .map(|(i, g)| {
            format!(
                "{{\"id\":\"{}\",\"open\":{},\"at\":[{:.1},{:.1},{:.1}],\"text\":\"{}\"}}",
                g.biome,
                !gates.shut[i],
                g.sign[0],
                g.sign[1],
                g.sign[2],
                sign_text(g.biome, &story)
            )
        })
        .collect();
    let json = format!("[{}]", view.join(","));
    if story.gates_json != json {
        story.gates_json = json;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_gate_fills_its_way_and_opens_it_again() {
        let mut w = vox::generate(vox::DEFAULT_SEED);
        for g in &GATES {
            let cells = build(&mut w, g);
            assert!(
                cells.len() >= 8,
                "{} gate fills its way: {}",
                g.biome,
                cells.len()
            );
            for &p in &cells {
                assert_eq!(w.get(p).mat, mat::RUNESTONE);
                assert!(!w.can_break(p), "the wall is protected");
            }
            open(&mut w, &cells);
            assert!(cells.iter().all(|&p| w.get(p).mat == mat::AIR));
        }
    }

    /// break-2 should-fix 1: a gate cannot be climbed or dug around. Each
    /// wall stands as high as the protected ridge beside it (the Marsh), the
    /// cells above every wall are protected (no building on it) and so are
    /// the cells under it (no tunnel under the cave gate).
    #[test]
    fn a_gate_cannot_be_climbed_or_dug_around() {
        let mut w = vox::generate(vox::DEFAULT_SEED);
        for g in &GATES {
            build(&mut w, g);
            let (a, b) = (g.wall.min, g.wall.max);
            for x in a[0]..b[0] {
                for z in a[2]..b[2] {
                    for y in a[1]..b[1] {
                        assert!(
                            vox::materials::flags(w.get([x, y, z]).mat).solid,
                            "{}: ({x},{y},{z}) in the wall is open",
                            g.biome
                        );
                    }
                    for y in b[1]..b[1] + 8 {
                        assert!(
                            !w.can_place([x, y, z]),
                            "{}: can build on the wall at ({x},{y},{z})",
                            g.biome
                        );
                    }
                    for y in a[1] - 6..a[1] {
                        assert!(
                            !w.can_break([x, y, z]),
                            "{}: can dig under the wall at ({x},{y},{z})",
                            g.biome
                        );
                    }
                }
            }
        }
        // The Marsh pass's wall is as high as the ridge it closes (y 61).
        assert!(GATES[1].wall.max[1] > 61);
        // The cave gate stands above the rocks around its mouth (y 38).
        assert!(GATES[2].wall.max[1] > 38);
    }

    #[test]
    fn the_ford_gate_shuts_the_whole_ford_mouth() {
        let mut w = vox::generate(vox::DEFAULT_SEED);
        build(&mut w, &GATES[0]);
        // Every column of the ford at z 158 is walled two blocks above its
        // bank step, and the canyon rims beside it too.
        for x in 120..136 {
            let top = (32..40)
                .filter(|&y| w.get([x, y, 158]).mat == mat::RUNESTONE)
                .count();
            assert!(top >= 2, "x {x}: wall {top} high");
        }
    }

    #[test]
    fn signs_count_practice_and_gates_follow_the_previous_spell() {
        let mut s = Story::default();
        let saved = crate::Progress::default();
        assert_eq!(
            sign_text("frozen", &s),
            "Cast Water 2 times anywhere to open (0/2)"
        );
        assert!(!is_open("frozen", &s, &saved));
        s.practice.record(1);
        assert_eq!(
            sign_text("frozen", &s),
            "Cast Water 2 times anywhere to open (1/2)"
        );
        s.practice.record(1);
        assert!(is_open("frozen", &s, &saved));
        assert!(!is_open("marsh", &s, &saved), "the Marsh needs Frost");
        assert_eq!(
            sign_text("cave", &s),
            "Cast CO₂ 2 times anywhere to open (0/2)"
        );
        assert!(is_open("village", &s, &saved), "no gate");
        // The cave needs CO2 practised and the hollow crossed.
        for _ in 0..2 {
            s.practice.record(3);
        }
        assert!(!is_open("cave", &s, &saved), "the hollow still burns");
        assert_eq!(sign_text("cave", &s), HOLLOW_SIGN);
        s.quest = realgame_sim::valley::Quest::at(6);
        assert!(is_open("cave", &s, &saved));
    }
}
