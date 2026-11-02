//! The biome gates (I-052 round 2; Paawan's partial mastery, 2026-11-01):
//! a runestone wall across the one way into each biome, which sinks away
//! once the previous biome's spell is Practised in this run
//! (`valley::practice`): the Frozen Fields need Water, the Marsh needs
//! Frost, the Crystal Cave needs CO2. Each gate has a sign the page draws
//! over it ("Practise Water 3 times to open"). The walls are physical, like
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
            max: [133, 50, 74],
        },
        sign: [132.5, 47.0, 70.0],
    },
    // The Crystal Cave's mouth in the Marsh (`gen::CAVE_MOUTH`).
    Gate {
        biome: "cave",
        wall: Aabb {
            min: [218, 30, 56],
            max: [223, 36, 61],
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
    out
}

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
    practice::biome_open(biome, &story.practice)
        || saved
            .0
            .get(spell)
            .is_some_and(|p| p.stage >= crate::progress::Stage::Practised)
}

/// The sign's words for a shut gate.
pub fn sign_text(biome: &str, story: &Story) -> String {
    let Some(spell) = practice::gate_spell(biome) else {
        return String::new();
    };
    let name = crate::chem::Spell::ALL[spell].name();
    let n = story.practice.casts[spell].min(PRACTISE_CASTS);
    format!("Practise {name} {PRACTISE_CASTS} times to open ({n}/{PRACTISE_CASTS})")
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
            "Practise Water 3 times to open (0/3)"
        );
        assert!(!is_open("frozen", &s, &saved));
        s.practice.record(1);
        assert_eq!(
            sign_text("frozen", &s),
            "Practise Water 3 times to open (1/3)"
        );
        s.practice.record(1);
        s.practice.record(1);
        assert!(is_open("frozen", &s, &saved));
        assert!(!is_open("marsh", &s, &saved), "the Marsh needs Frost");
        assert_eq!(sign_text("cave", &s), "Practise CO₂ 3 times to open (0/3)");
        assert!(is_open("village", &s, &saved), "no gate");
    }
}
