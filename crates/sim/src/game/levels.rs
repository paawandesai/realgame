//! Single-screen v3 levels: Trials, the Lab and Today's Trial (DESIGN_V3
//! §4.3-§4.5, §5). Track B owns this file after Hour 0.
//!
//! A level is a separate `Game` built from a `LevelSpec`. Only the first
//! `ROOM_W` (320) columns are playable; every column past them is stone, so
//! the story's `ROOMS`-indexed arrays never change and the camera stays put.
//! The story never sets `Game::level`, so its behaviour and grid hashes are
//! untouched.
//!
//! Hour-0 stub: every spec builds the same blank stone room (floor and two
//! walls) and `TrialResult` stays unsolved. Track B fills in the layouts,
//! goals, scoring (`score.rs`) and scripted solutions.

use super::{Game, Tier, GROUND_Y, HEIGHT, PLAYER_H, ROOM_W, WORLD_W};
use crate::grid::AMBIENT_C;
use crate::species::Species;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LevelKind {
    Trial,
    Lab,
}

/// Everything needed to build one level.
#[derive(Clone, Debug, Serialize)]
pub struct LevelSpec {
    /// "T1".."T7" for Trials, "lab" for the Lab.
    pub id: &'static str,
    pub kind: LevelKind,
    pub title: &'static str,
    /// One line: the goal and its twist.
    pub goal: &'static str,
    /// The tiers that see this level (§4.3).
    pub tiers: &'static [Tier],
    /// Par for Apprentice star 2: casts of the best scripted solution
    /// (0 until Track B's solutions set it).
    pub par_casts: u32,
}

const ALL_TIERS: &[Tier] = &[Tier::Sprout, Tier::Seedling, Tier::Apprentice];
const SEEDLING_UP: &[Tier] = &[Tier::Seedling, Tier::Apprentice];
const APPRENTICE: &[Tier] = &[Tier::Apprentice];

const fn trial(
    id: &'static str,
    title: &'static str,
    goal: &'static str,
    tiers: &'static [Tier],
) -> LevelSpec {
    LevelSpec {
        id,
        kind: LevelKind::Trial,
        title,
        goal,
        tiers,
        par_casts: 0,
    }
}

/// The Trials in menu order (§4.3). T7 is the stretch Trial, first to cut.
pub const TRIALS: [LevelSpec; 7] = [
    trial(
        "T1",
        "Foam Bridge",
        "Cross the pit. A stagnant H2O2 pool sits by a dying brazier.",
        ALL_TIERS,
    ),
    trial(
        "T2",
        "Colour Moat",
        "Turn the moat purple (pH 6-8) to open the door.",
        ALL_TIERS,
    ),
    trial(
        "T3",
        "White Fire",
        "Save the wooden bridge from a burning magnesium strip.",
        ALL_TIERS,
    ),
    trial(
        "T4",
        "Flame Lock",
        "Match the rune's three flame colours in the brazier.",
        ALL_TIERS,
    ),
    trial(
        "T5",
        "Milky Proof",
        "Prove the jar's gas is CO2 by turning the limewater milky.",
        SEEDLING_UP,
    ),
    trial(
        "T6",
        "Foundry Finale",
        "Mg fire, a foam pit and a colour rune, all in one screen.",
        ALL_TIERS,
    ),
    trial(
        "T7",
        "Lean & Rich",
        "Light the CH4 lantern without exploding the ceiling pocket.",
        APPRENTICE,
    ),
];

impl LevelSpec {
    pub fn lab() -> LevelSpec {
        LevelSpec {
            id: "lab",
            kind: LevelKind::Lab,
            title: "Lab",
            goal: "Paint what you have found, cast, and watch.",
            tiers: ALL_TIERS,
            par_casts: 0,
        }
    }

    /// The Trial with this id ("T1".."T7"), if any.
    pub fn trial(id: &str) -> Option<LevelSpec> {
        TRIALS.iter().find(|t| t.id == id).cloned()
    }

    pub fn for_tier(&self, tier: Tier) -> bool {
        self.tiers.contains(&tier)
    }
}

/// The Trials a tier sees, in menu order.
pub fn trial_list(tier: Tier) -> Vec<LevelSpec> {
    TRIALS
        .iter()
        .filter(|t| t.for_tier(tier))
        .cloned()
        .collect()
}

/// How a Trial run went (§5). Deterministic: the same inputs give the same
/// result. Star 3 ("a second, different solution") needs earlier runs, so
/// the web shell awards it from `progress.recordSig`; `stars` here counts
/// only what this run earned on its own (star 1 and star 2).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TrialResult {
    pub id: String,
    pub solved: bool,
    pub casts: u32,
    pub hints_used: u32,
    /// Every cast this run was balanced (Seedling star 2).
    pub balanced_all: bool,
    pub atoms_wasted: u32,
    pub kj_net: f32,
    /// 32-bit hash of the sorted reaction ids that changed goal-relevant
    /// cells plus the goal mechanism id. Different solutions differ here.
    pub sig: u32,
    /// 0, 1 or 2 (see above).
    pub stars: u8,
}

/// A level's live state, kept on `Game::level`.
#[derive(Clone, Debug)]
pub struct Level {
    pub spec: LevelSpec,
    pub seed: u64,
    pub result: TrialResult,
}

/// Where the player starts in a level.
const LEVEL_SPAWN_X: f32 = 40.0;
/// Wall thickness at each side of the 320-wide level.
const WALL: i32 = 4;

impl Game {
    /// Builds a Trial or the Lab (`spec.kind`) as its own game.
    pub fn new_level(tier: Tier, spec: LevelSpec, seed: u64) -> Game {
        let mut g = Game::new(tier, seed);
        let room = ROOM_W as i32;
        let grid = &mut g.grid;
        grid.fill_rect(0, 0, WORLD_W as i32, HEIGHT as i32, Species::Air);
        grid.fill_rect(0, GROUND_Y, room, HEIGHT as i32, Species::Stone);
        grid.fill_rect(0, 0, WALL, GROUND_Y, Species::Stone);
        grid.fill_rect(room - WALL, 0, room, GROUND_Y, Species::Stone);
        grid.fill_rect(room, 0, WORLD_W as i32, HEIGHT as i32, Species::Stone);
        for t in grid.temp.iter_mut() {
            *t = AMBIENT_C;
        }
        g.player.x = LEVEL_SPAWN_X;
        g.player.y = GROUND_Y as f32 - PLAYER_H - 1.0;
        g.player.prev_x = g.player.x;
        g.player.prev_y = g.player.y;
        g.objectives.clear();
        g.level = Some(Level {
            result: TrialResult {
                id: spec.id.to_string(),
                ..TrialResult::default()
            },
            spec,
            seed,
        });
        g.snap_camera();
        g
    }

    /// True for the v2 story (and its free play); false in a Trial or the Lab.
    pub fn is_story(&self) -> bool {
        self.level.is_none()
    }

    /// The current Trial's result so far (None in the story).
    pub fn trial_result(&self) -> Option<&TrialResult> {
        self.level.as_ref().map(|l| &l.result)
    }

    /// Lab painting: fills a disc of radius `r` at (x, y) with `s`. Counted
    /// as "spawned" in the atom ledger once Track B adds it. Only the 320
    /// playable columns can be painted; walls and floor stay.
    pub fn paint(&mut self, x: i32, y: i32, s: Species, r: i32) {
        if self.is_story() {
            return;
        }
        let r = r.clamp(0, 16);
        for dy in -r..=r {
            for dx in -r..=r {
                let (cx, cy) = (x + dx, y + dy);
                if dx * dx + dy * dy <= r * r
                    && (WALL..ROOM_W as i32 - WALL).contains(&cx)
                    && (0..GROUND_Y).contains(&cy)
                {
                    self.grid.set(cx, cy, s, AMBIENT_C);
                }
            }
        }
    }

    /// Lab eraser: paints air.
    pub fn erase(&mut self, x: i32, y: i32, r: i32) {
        self.paint(x, y, Species::Air, r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_separate_from_the_story_and_run() {
        assert!(Game::new(Tier::Seedling, 1).is_story());
        for spec in TRIALS.iter().cloned().chain([LevelSpec::lab()]) {
            let mut g = Game::new_level(Tier::Apprentice, spec, 3);
            assert!(!g.is_story());
            for _ in 0..240 {
                g.tick(super::super::INPUT_RIGHT);
            }
            assert!(g.player.x < ROOM_W as f32, "stays in the level");
            assert!(g.trial_result().is_some());
        }
    }

    #[test]
    fn tiers_see_the_right_trials() {
        let ids = |t| trial_list(t).iter().map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(ids(Tier::Sprout), ["T1", "T2", "T3", "T4", "T6"]);
        assert_eq!(ids(Tier::Seedling), ["T1", "T2", "T3", "T4", "T5", "T6"]);
        assert_eq!(ids(Tier::Apprentice).len(), 7);
    }

    #[test]
    fn lab_paints_inside_the_room_only() {
        let mut g = Game::new_level(Tier::Sprout, LevelSpec::lab(), 1);
        g.paint(100, 100, Species::Sand, 3);
        assert_eq!(g.grid.get(100, 100), Species::Sand);
        g.paint(0, 100, Species::Water, 3);
        assert_eq!(g.grid.get(0, 100), Species::Stone);
        g.erase(100, 100, 3);
        assert_eq!(g.grid.get(100, 100), Species::Air);
    }
}
