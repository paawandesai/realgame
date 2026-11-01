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

use crate::chem::{self, EventKind, Kind};
use crate::world::{Aabb, Collider};
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

/// The lab's colliders and chemistry. Contract stub.
pub fn layout() -> Layout {
    Layout {
        colliders: Vec::new(),
        world: chem::World::with_goal(Vec::new(), chem::Goal::Reach),
        boxes: Vec::new(),
    }
}

/// Level 4's caption for an event from a spell that hit `obj`, at the
/// player's band (`None` reads as Apprentice), with the Sprout picture.
/// Contract stub.
pub fn caption(
    kind: EventKind,
    obj: Kind,
    band: Option<Tier>,
) -> Option<(&'static str, &'static str)> {
    let _ = (kind, obj, band);
    None
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
            assert!(p.pos.z > WALL_Z, "x={x}: stopped south of the wall, z={}", p.pos.z);
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
                let p = walk(&w, &lay.colliders, Vec3::new(x, 0.0, WALL_Z + 3.0), NORTH, 4.0);
                assert!(p.pos.z > WALL_Z, "x={x}: passed the wall, z={}", p.pos.z);
            }
            x += 1.0;
        }
    }
}
