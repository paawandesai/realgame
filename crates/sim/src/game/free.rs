//! Free play after the trial: every room can be reset (its hedge, ice, gas
//! trap and brazier rebuilt from the seed) and a short list of challenges,
//! picked and tuned from the seed, gives the player something to aim for.
//! Some challenges unlock "coming soon" Grimoire pages; each such page is
//! an entry already in `content/reactions.json` (a reaction or a phase
//! change), so no unsourced science is added.

use super::{CastOutcome, Game, Tier, ROOMS};
use serde::Serialize;

/// The ice challenge's heat budget, kJ: one balanced boom (-483.6 kJ, NIST)
/// fits under it; two do not.
pub const ICE_BUDGET_KJ: f32 = 500.0;
/// The trap challenge: fill the trap in exactly the fewest splits it takes
/// (one split cast for Sprout and Seedling, whose trap needs less gas; two
/// for Apprentice).
pub fn trap_splits(tier: Tier) -> u32 {
    if tier == Tier::Apprentice {
        2
    } else {
        1
    }
}

/// One free-play challenge as the HUD lists it.
#[derive(Debug, Clone, Serialize)]
pub struct Challenge {
    /// "smother", "ice", "trap", "water", "douse" or "fireball".
    pub id: &'static str,
    pub text: String,
    /// A picture for readers who can't read yet (an icon name).
    pub pic: &'static str,
    /// The room (0-based) whose reset sets it up again.
    pub room: u8,
    pub done: bool,
    /// The Grimoire page it unlocks, if any.
    pub unlocks: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct FreePlay {
    pub challenges: Vec<Challenge>,
    /// Grimoire pages unlocked by challenges, in the order earned.
    pub unlocked: Vec<&'static str>,
    /// Make this much H2O (seeded).
    water_goal: u32,
    water_made: u32,
    /// Put a hedge out in this many casts or fewer (seeded).
    douse_max: u32,
    /// Heat released by the player's casts in room 2 since it was last
    /// reset (or since free play began), kJ.
    ice_kj: f32,
    /// Splits that bubbled gas into the crystal pool since room 3 was last
    /// reset (or since free play began).
    splits: u32,
    /// Last tick's view of what the challenges watch, to catch changes.
    was_burning: [bool; 2],
    was_gate: [bool; ROOMS],
    was_trap: bool,
    was_bonus: bool,
}

/// A small deterministic mix of the seed (splitmix64).
fn mix(seed: u64, salt: u64) -> u64 {
    let mut z = seed.wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Game {
    /// Builds the challenge list from the seed: the same seed always gives
    /// the same challenges, targets and order.
    pub(super) fn start_free_play(&mut self) {
        let water_goal = [8, 10, 12][(mix(self.seed, 1) % 3) as usize];
        let douse_max = [2, 3][(mix(self.seed, 2) % 2) as usize];
        let mut list = vec![
            Challenge {
                id: "smother",
                text: self
                    .line(
                        "Cloud on a burning hedge!",
                        "Smother a flame with CO₂ (cloud).",
                        "Smother a burning hedge with CO₂ (cloud): take away its oxygen.",
                    )
                    .into(),
                pic: "cloud",
                room: 0,
                done: false,
                unlocks: Some("carbon_combustion"),
            },
            Challenge {
                id: "ice",
                text: self
                    .line(
                        "Melt ice with one boom!",
                        "Melt the ice using under 500 kJ.",
                        "Melt the ice wall using under 500 kJ of heat.",
                    )
                    .into(),
                pic: "ice",
                room: 1,
                done: false,
                unlocks: Some("water_melting"),
            },
            Challenge {
                id: "trap",
                text: self
                    .line(
                        "One bubbles card fills trap!",
                        "Fill the trap with just 1 split.",
                        "Fill the gas trap in exactly 2 splits.",
                    )
                    .into(),
                pic: "bubbles",
                room: 2,
                done: false,
                unlocks: None,
            },
            Challenge {
                id: "water",
                text: match self.tier {
                    Tier::Sprout => format!("Boom makes {water_goal} water!"),
                    Tier::Seedling => format!("Make {water_goal} H₂O with boom."),
                    Tier::Apprentice => {
                        format!("Make {water_goal} mol of H₂O with reactions.")
                    }
                },
                // Pouring water is not making it: the picture is the boom.
                pic: if self.tier == Tier::Apprentice {
                    "drop"
                } else {
                    "boom"
                },
                room: 1,
                done: false,
                unlocks: None,
            },
            Challenge {
                id: "douse",
                text: match self.tier {
                    Tier::Sprout => format!("Hedge out in {douse_max} taps!"),
                    _ => format!("Put out a hedge in {douse_max} casts or fewer."),
                },
                pic: "hedgeFire",
                room: 0,
                done: false,
                unlocks: None,
            },
        ];
        if self.tier != Tier::Sprout {
            list.push(Challenge {
                id: "fireball",
                text: self
                    .line(
                        "",
                        "Light the brazier with a fireball.",
                        "Light the brazier with CH₄ + 2O₂ → CO₂ + 2H₂O.",
                    )
                    .into(),
                pic: "methane",
                room: 3,
                done: false,
                unlocks: None,
            });
        }
        // The smother challenge always leads; the rest are in seeded order.
        let mut rest = list.split_off(1);
        let n = rest.len();
        for i in (1..n).rev() {
            let j = (mix(self.seed, 10 + i as u64) % (i as u64 + 1)) as usize;
            rest.swap(i, j);
        }
        list.extend(rest);
        self.free = Some(FreePlay {
            challenges: list,
            water_goal,
            douse_max,
            ..FreePlay::default()
        });
        self.free_watch();
    }

    /// Remembers what the challenges watch, so the next tick sees changes.
    fn free_watch(&mut self) {
        let r = &self.rooms;
        let burning = [!r.hedges[0].out, !r.hedges[1].out];
        let gates = r.gates_open;
        let trap = self.trap_ready();
        let bonus = r.fireball_bonus;
        if let Some(f) = self.free.as_mut() {
            f.was_burning = burning;
            f.was_gate = gates;
            f.was_trap = trap;
            f.was_bonus = bonus;
        }
    }

    /// A room was reset in free play: its challenge counters start again.
    pub(super) fn free_room_reset(&mut self, k: usize) {
        if let Some(f) = self.free.as_mut() {
            match k {
                1 => f.ice_kj = 0.0,
                2 => f.splits = 0,
                _ => {}
            }
        }
        self.free_watch();
    }

    /// A cast happened: count heat in room 2 and water made.
    pub(super) fn free_note_cast(&mut self, out: &CastOutcome) {
        let room = self.room();
        let Some(f) = self.free.as_mut() else {
            return;
        };
        if room == 1 && out.dh_kj < 0.0 {
            f.ice_kj += -out.dh_kj;
        }
        if out.reaction_id.is_some() {
            f.water_made += out.products.get("H2O").copied().unwrap_or(0);
        }
    }

    /// Water was split into the crystal pool (`made` splits at once).
    pub(super) fn free_note_split(&mut self, made: u32) {
        if let Some(f) = self.free.as_mut() {
            f.splits += made;
        }
    }

    /// Checks every challenge against this tick's changes.
    pub(super) fn free_tick(&mut self) {
        let Some(f) = self.free.as_ref() else {
            return;
        };
        let r = &self.rooms;
        let mut done: Vec<&'static str> = vec![];
        for k in 0..2 {
            let h = &r.hedges[k];
            if f.was_burning[k] && h.out {
                if h.co2_hits > 0 {
                    done.push("smother");
                }
                if h.hits > 0 && h.hits <= f.douse_max {
                    done.push("douse");
                }
            }
        }
        if !f.was_gate[1] && r.gates_open[1] && f.ice_kj < ICE_BUDGET_KJ {
            done.push("ice");
        }
        let trap = self.trap_ready();
        if !f.was_trap && trap && f.splits == trap_splits(self.tier) {
            done.push("trap");
        }
        if f.water_made >= f.water_goal {
            done.push("water");
        }
        if !f.was_bonus && r.fireball_bonus {
            done.push("fireball");
        }
        let mut finished = 0;
        if let Some(f) = self.free.as_mut() {
            for id in done {
                if let Some(c) = f.challenges.iter_mut().find(|c| c.id == id && !c.done) {
                    c.done = true;
                    finished += 1;
                    if let Some(page) = c.unlocks {
                        if !f.unlocked.contains(&page) {
                            f.unlocked.push(page);
                        }
                    }
                }
            }
        }
        if finished > 0 {
            let line = self.line(
                "Challenge done! Star!",
                "Challenge done! Check the list.",
                "Challenge complete. Pick another from the list, or reset a room.",
            );
            self.say(line);
            self.refresh_version();
        }
        self.free_watch();
    }

    /// The challenge list (empty outside free play).
    pub fn challenges(&self) -> &[Challenge] {
        self.free.as_ref().map_or(&[], |f| &f.challenges)
    }

    /// Grimoire pages unlocked by challenges.
    pub fn unlocked_pages(&self) -> &[&'static str] {
        self.free.as_ref().map_or(&[], |f| &f.unlocked)
    }
}

#[cfg(test)]
mod tests {
    use super::super::rooms::script::*;
    use super::super::rooms::{HEDGE1, POOL};
    use super::super::*;

    fn free(tier: Tier, seed: u64) -> Game {
        let mut g = Game::new(tier, seed);
        g.free_play();
        g
    }

    #[test]
    fn free_play_lists_four_to_six_seeded_challenges() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            for seed in [1, 2, 3, 42] {
                let a = free(tier, seed);
                let b = free(tier, seed);
                let ids = |g: &Game| -> Vec<(String, String)> {
                    g.challenges()
                        .iter()
                        .map(|c| (c.id.to_string(), c.text.clone()))
                        .collect()
                };
                assert_eq!(ids(&a), ids(&b), "same seed, same list");
                let n = a.challenges().len();
                assert!((4..=6).contains(&n), "{n} challenges");
                assert_eq!(a.challenges()[0].id, "smother");
                assert!(a.challenges().iter().all(|c| !c.done));
                if tier == Tier::Sprout {
                    for c in a.challenges() {
                        assert!(c.text.split_whitespace().count() <= 5, "{}", c.text);
                    }
                }
            }
        }
        let texts: std::collections::BTreeSet<String> = (0..12)
            .map(|s| {
                free(Tier::Seedling, s)
                    .challenges()
                    .iter()
                    .map(|c| c.text.clone())
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect();
        assert!(texts.len() > 1, "seeds tune the list");
    }

    /// Every cell of the reset room (its columns, full height) matches a
    /// fresh level built from the same seed, and its target is back.
    #[test]
    fn resetting_a_room_rebuilds_its_cells_from_the_seed() {
        for (k, seed) in [(0usize, 5u64), (1, 6), (2, 7), (3, 8)] {
            let mut g = free(Tier::Seedling, seed);
            // Change the world: walk, cast, let it run.
            teleport(&mut g, (k * ROOM_W) as f32 + 60.0);
            for spell in ["water", "boom", "split", "cloud"] {
                g.cast(spell);
                settle(&mut g, 40);
            }
            settle(&mut g, 200);
            g.reset_room(k);
            let fresh = Game::new(Tier::Seedling, seed);
            let w = g.grid.w;
            let mut differ = 0;
            for y in 0..HEIGHT {
                for x in k * ROOM_W..(k + 1) * ROOM_W {
                    let i = y * w + x;
                    if g.grid.cells[i] != fresh.grid.cells[i]
                        || g.grid.temp[i].to_bits() != fresh.grid.temp[i].to_bits()
                        || g.grid.aux[i] != fresh.grid.aux[i]
                    {
                        differ += 1;
                    }
                }
            }
            assert_eq!(
                differ, 0,
                "room {k}: {differ} cells differ from a fresh build"
            );
            assert!(!g.rooms.gates_open[k], "room {k}'s gate is closed again");
            assert!(!g.goals()[k].done, "room {k}'s goal is back");
            // Other rooms stay open for free play.
            for j in (0..ROOMS).filter(|&j| j != k) {
                assert!(g.rooms.gates_open[j], "room {j} stays open");
            }
        }
    }

    #[test]
    fn reset_room_1_lights_the_hedge_and_water_puts_it_out() {
        let mut g = free(Tier::Sprout, 42);
        g.reset_room(0);
        settle(&mut g, 30);
        assert!(g.rooms.hedges[0].burning > 0, "the hedge burns again");
        assert!(douse(&mut g, 0, 4).is_some(), "water puts it out");
        assert!(g.rooms.gates_open[0]);
    }

    #[test]
    fn smothering_a_reset_hedge_with_co2_completes_the_challenge_and_unlocks_a_page() {
        let mut g = free(Tier::Seedling, 42);
        assert!(g.unlocked_pages().is_empty());
        g.reset_room(0);
        settle(&mut g, 30);
        let target = HEDGE1.centre();
        let mut casts = 0;
        while !g.rooms.hedges[0].out && casts < 8 {
            cast_at(&mut g, "cloud", target);
            settle(&mut g, 90);
            casts += 1;
        }
        settle(&mut g, 120);
        assert!(g.rooms.hedges[0].out, "CO2 put the hedge out");
        let c = g.challenges().iter().find(|c| c.id == "smother").unwrap();
        assert!(c.done, "smother challenge done");
        assert_eq!(g.unlocked_pages(), ["carbon_combustion"]);
        // The page it unlocks is a sourced reaction in reactions.json.
        assert!(g.book.by_id("carbon_combustion").is_some());
    }

    #[test]
    fn one_boom_melts_reset_ice_under_budget_and_unlocks_melting() {
        let mut g = free(Tier::Apprentice, 42);
        g.reset_room(1);
        let booms = melt_ice_with_booms(&mut g, 1500);
        settle(&mut g, 60);
        assert!(g.rooms.gates_open[1], "ice melted");
        let c = g.challenges().iter().find(|c| c.id == "ice").unwrap();
        // One balanced boom (483.6 kJ) melts the whole wall, under budget.
        assert_eq!(booms, 1);
        assert!(c.done, "{} kJ", g.free.as_ref().unwrap().ice_kj);
        assert!(g.unlocked_pages().contains(&"water_melting"));
        assert!(g.book.latent_heat("water_melting") > 0.0);
    }

    #[test]
    fn making_enough_water_completes_the_water_challenge() {
        let mut g = free(Tier::Apprentice, 3);
        let goal = g.free.as_ref().unwrap().water_goal;
        let mut casts = 0;
        while !g.challenges().iter().any(|c| c.id == "water" && c.done) && casts < 20 {
            g.jars.insert("H2".into(), 12);
            g.jars.insert("O2".into(), 6);
            g.cast("2H2 + O2 -> 2H2O");
            settle(&mut g, 30);
            casts += 1;
        }
        assert!(g.free.as_ref().unwrap().water_made >= goal);
        assert_eq!(casts, goal.div_ceil(2), "2 H2O per balanced boom");
        // Pouring water is not making it.
        let mut h = free(Tier::Apprentice, 3);
        for _ in 0..12 {
            h.cast("water");
            settle(&mut h, 100);
        }
        assert_eq!(h.free.as_ref().unwrap().water_made, 0);
    }

    #[test]
    fn two_booms_on_the_ice_go_over_budget() {
        let mut g = free(Tier::Apprentice, 42);
        g.reset_room(1);
        teleport(&mut g, 440.0);
        // A boom into the air first: heat spent in the room, ice untouched.
        cast_at(&mut g, "boom", (440, 60));
        settle(&mut g, 120);
        melt_ice_with_booms(&mut g, 1500);
        settle(&mut g, 60);
        assert!(g.rooms.gates_open[1]);
        let c = g.challenges().iter().find(|c| c.id == "ice").unwrap();
        assert!(
            !c.done,
            "{} kJ is over budget",
            g.free.as_ref().unwrap().ice_kj
        );
    }

    #[test]
    fn filling_the_trap_in_the_fewest_splits_completes_it() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = free(tier, 42);
            g.reset_room(2);
            let k = room_3_split_count(&mut g);
            assert_eq!(k, super::trap_splits(tier), "{tier:?}");
            let c = g.challenges().iter().find(|c| c.id == "trap").unwrap();
            assert!(c.done, "{tier:?}");
        }
        // One split too many (into the air first) misses it.
        let mut g = free(Tier::Seedling, 42);
        g.reset_room(2);
        teleport(&mut g, 686.0);
        cast_at(&mut g, "split", (700, 60));
        settle(&mut g, 200);
        let k = room_3_split_count(&mut g);
        assert_eq!(k, 2);
        assert!(!g.challenges().iter().any(|c| c.id == "trap" && c.done));
    }

    /// Splits into the pool from beside the crystal until the trap is ready.
    fn room_3_split_count(g: &mut Game) -> u32 {
        teleport(g, 686.0);
        let mut casts = 0;
        while !g.trap_ready() && casts < 6 {
            cast_at(g, "split", (POOL.centre().0, POOL.y0 + 1));
            settle(g, 120);
            casts += 1;
        }
        settle(g, 30);
        assert!(g.trap_ready(), "trap filled");
        g.free.as_ref().unwrap().splits
    }

    #[test]
    fn challenges_never_complete_by_themselves() {
        let mut g = free(Tier::Seedling, 9);
        for k in 0..ROOMS {
            g.reset_room(k);
        }
        settle(&mut g, 900);
        assert!(g.challenges().iter().all(|c| !c.done));
        assert!(g.unlocked_pages().is_empty());
    }
}
