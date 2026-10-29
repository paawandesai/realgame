//! Game rules on top of the grid: the player, reagent jars, casting, and the
//! first breakthrough trial.

use crate::formula::{parse_equation, Equation};
use crate::grid::{Grid, AMBIENT_C, FLAME_C_IN_O2, WOOD_IGNITION_C};
use crate::reactions::ReactionBook;
use crate::species::Species;
use serde::Serialize;
use std::collections::BTreeMap;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 180;

pub const INPUT_LEFT: u32 = 1;
pub const INPUT_RIGHT: u32 = 2;
pub const INPUT_JUMP: u32 = 4;
pub const INPUT_GATHER: u32 = 8;

const JAR_CAPACITY: u32 = 200;
const PLAYER_W: f32 = 3.0;
const PLAYER_H: f32 = 8.0;
const REACH: f32 = 140.0;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// K-2: picture tiles, Mote balances equations for you.
    Sprout,
    /// Grades 3-5: unbalanced casts fizzle, with a one-tap fix.
    Seedling,
    /// Grades 6-8: you balance it yourself; real enthalpy values shown.
    Apprentice,
}

impl Tier {
    pub fn parse(s: &str) -> Tier {
        match s {
            "sprout" => Tier::Sprout,
            "seedling" => Tier::Seedling,
            _ => Tier::Apprentice,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Objective {
    pub id: &'static str,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CastResult {
    pub ok: bool,
    /// "release", "reaction", "fizzle" or "error".
    pub kind: String,
    pub message: String,
    pub equation: String,
    /// A corrected equation the UI can offer as a one-tap fix.
    pub suggestion: Option<String>,
    pub imbalance: Vec<String>,
    pub reaction_id: Option<String>,
    pub concept: Option<String>,
    pub real_world: Option<String>,
    /// Shown at Apprentice tier and above, with its source.
    pub delta_h_kj: Option<f32>,
    pub source: Option<String>,
    pub first_discovery: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct State {
    pub tier: Tier,
    pub jars: BTreeMap<String, u32>,
    pub charge: u32,
    pub objectives: Vec<Objective>,
    pub messages: Vec<String>,
    pub player_x: f32,
    pub player_y: f32,
    pub discovered: Vec<String>,
    pub trial_complete: bool,
    pub aim_temp_c: f32,
    pub aim_substance: String,
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    /// Position at the start of the last tick, so the renderer can interpolate.
    pub prev_x: f32,
    pub prev_y: f32,
    vx: f32,
    vy: f32,
    pub facing: i32,
}

pub struct Game {
    pub grid: Grid,
    pub player: Player,
    pub tier: Tier,
    pub jars: BTreeMap<String, u32>,
    pub charge: u32,
    pub objectives: Vec<Objective>,
    messages: Vec<String>,
    pub aim: (i32, i32),
    book: ReactionBook,
    pub discovered: Vec<String>,
    fizzles_in_a_row: u32,
    gathered_water: u32,
    electrolysis_casts: u32,
    torch_lit: bool,
    met_crystal: bool,
    gather_target: Option<Species>,
    announced_complete: bool,
}

impl Game {
    pub fn new(tier: Tier, seed: u64) -> Game {
        let book = ReactionBook::load();
        let grid = Grid::new(
            WIDTH,
            HEIGHT,
            seed,
            book.latent_heat("water_boiling"),
            book.latent_heat("water_melting"),
        );
        let mut game = Game {
            grid,
            player: Player {
                x: 12.0,
                y: 130.0,
                prev_x: 12.0,
                prev_y: 130.0,
                vx: 0.0,
                vy: 0.0,
                facing: 1,
            },
            tier,
            jars: ["H2O", "H2", "O2", "CH4"]
                .iter()
                .map(|k| (k.to_string(), 0))
                .collect(),
            charge: 0,
            objectives: vec![],
            messages: vec![],
            aim: (40, 140),
            book,
            discovered: vec![],
            fizzles_in_a_row: 0,
            gathered_water: 0,
            electrolysis_casts: 0,
            torch_lit: false,
            met_crystal: false,
            gather_target: None,
            announced_complete: false,
        };
        game.build_trial();
        game
    }

    fn say(&mut self, text: impl Into<String>) {
        self.messages.push(text.into());
    }

    // -------------------------------------------------------------- the level

    /// The first breakthrough trial: a stream, a spreading fire, a charged
    /// crystal and a sealed door with a torch beside it.
    fn build_trial(&mut self) {
        let g = &mut self.grid;
        g.fill_rect(0, 150, WIDTH as i32, HEIGHT as i32, Species::Stone);
        // Gentle bumps in the ground so it doesn't look ruled.
        for x in 0..WIDTH as i32 {
            let bump = ((x as f32 * 0.11).sin() * 1.5 + (x as f32 * 0.037).cos() * 1.5) as i32;
            g.fill_rect(x, 148 - bump.max(-1), x + 1, 150, Species::Stone);
        }
        // The stream: a pool dug into the ground.
        g.fill_rect(30, 146, 96, 162, Species::Air);
        g.fill_rect(30, 162, 96, 166, Species::Stone);
        g.fill_rect(30, 150, 96, 162, Species::Water);
        // A small wood: trunks and canopies.
        for (tx, h) in [(112, 14), (124, 18), (137, 12), (150, 16), (163, 13)] {
            let top = 147 - h;
            g.fill_rect(tx, top, tx + 2, 148, Species::Wood);
            g.fill_rect(tx - 4, top - 6, tx + 6, top + 1, Species::Wood);
        }
        // The first tree is already burning.
        for y in 120..148 {
            for x in 104..120 {
                if g.get(x, y) == Species::Wood {
                    let i = g.idx(x as usize, y as usize);
                    g.temp[i] = 650.0;
                }
            }
        }
        // The crystal pillar.
        g.fill_rect(190, 136, 195, 148, Species::Crystal);
        // The wall, its door and the torch on a ledge.
        g.fill_rect(236, 0, 242, 128, Species::Stone);
        g.fill_rect(236, 128, 242, 148, Species::Door);
        g.fill_rect(220, 118, 236, 121, Species::Stone);
        g.fill_rect(226, 111, 230, 118, Species::Wood);

        self.objectives = vec![
            Objective {
                id: "gather",
                text: "Gather water from the stream (hold right-click on it)".into(),
                done: false,
            },
            Objective {
                id: "fire",
                text: "Put out the fire: aim right at the flames and cast H2O".into(),
                done: false,
            },
            Objective {
                id: "split",
                text: "Walk up to the crystal, then split water: 2H2O -> 2H2 + O2".into(),
                done: false,
            },
            Objective {
                id: "torch",
                text: "Light the torch by the door: 2H2 + O2 -> 2H2O".into(),
                done: false,
            },
            Objective {
                id: "exit",
                text: "Walk through the open door".into(),
                done: false,
            },
        ];
        self.say("Your core is empty. Let's fill it. Walk to the stream and gather some water.");
    }

    // ----------------------------------------------------------------- update

    pub fn tick(&mut self, input: u32) {
        self.player.prev_x = self.player.x;
        self.player.prev_y = self.player.y;
        self.move_player(input);
        if input & INPUT_GATHER != 0 {
            // Lock onto what was under the cursor when gathering started.
            if self.gather_target.is_none() {
                self.gather_target = self.substance_near_aim();
            }
            if let Some(target) = self.gather_target {
                self.gather(target);
            }
        } else {
            self.gather_target = None;
        }
        self.grid.step();
        self.touch_crystal();
        self.check_objectives();
    }

    /// Trees are scenery the player walks in front of; everything else solid blocks.
    fn solid_at(&self, x: f32, y: f32) -> bool {
        let s = self.grid.get(x.floor() as i32, y.floor() as i32);
        s.is_solid() && s != Species::Wood
    }

    fn box_blocked(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = (x, x + PLAYER_W - 0.01);
        let (y0, y1) = (y, y + PLAYER_H - 0.01);
        let mut yy = y0;
        while yy <= y1 {
            if self.solid_at(x0, yy) || self.solid_at(x1, yy) || self.solid_at((x0 + x1) / 2.0, yy)
            {
                return true;
            }
            yy += 1.0;
        }
        self.solid_at(x0, y1) || self.solid_at(x1, y1)
    }

    fn in_water(&self) -> bool {
        let p = &self.player;
        self.grid
            .get((p.x + 1.0) as i32, (p.y + PLAYER_H - 2.0) as i32)
            == Species::Water
    }

    fn on_ground(&self) -> bool {
        self.box_blocked(self.player.x, self.player.y + 0.5)
    }

    fn move_player(&mut self, input: u32) {
        let swimming = self.in_water();
        let speed = if swimming { 0.6 } else { 1.1 };
        let mut target_vx = 0.0;
        if input & INPUT_LEFT != 0 {
            target_vx -= speed;
            self.player.facing = -1;
        }
        if input & INPUT_RIGHT != 0 {
            target_vx += speed;
            self.player.facing = 1;
        }
        self.player.vx += (target_vx - self.player.vx) * 0.4;
        if input & INPUT_JUMP != 0 {
            if self.on_ground() {
                self.player.vy = -3.2;
            } else if swimming {
                // At the surface a kick lifts you out; underwater you swim up.
                let head = self
                    .grid
                    .get((self.player.x + 1.0) as i32, (self.player.y - 1.0) as i32);
                self.player.vy = if head == Species::Water {
                    (self.player.vy - 0.4).max(-1.2)
                } else {
                    -2.4
                };
            }
        }
        self.player.vy = (self.player.vy + if swimming { 0.06 } else { 0.22 }).min(if swimming {
            0.8
        } else {
            4.0
        });

        // Horizontal, with a one-cell step-up so bumps don't stop you.
        let nx = self.player.x + self.player.vx;
        if !self.box_blocked(nx, self.player.y) {
            self.player.x = nx;
        } else if !self.box_blocked(nx, self.player.y - 1.0) && self.on_ground() {
            self.player.x = nx;
            self.player.y -= 1.0;
        } else {
            self.player.vx = 0.0;
        }
        // Vertical, in small steps so fast falls don't tunnel.
        let steps = (self.player.vy.abs() / 0.5).ceil().max(1.0) as i32;
        for _ in 0..steps {
            let ny = self.player.y + self.player.vy / steps as f32;
            if self.box_blocked(self.player.x, ny) {
                self.player.vy = 0.0;
                break;
            }
            self.player.y = ny;
        }
        self.player.x = self.player.x.clamp(0.0, WIDTH as f32 - PLAYER_W);
        self.player.y = self.player.y.clamp(0.0, HEIGHT as f32 - PLAYER_H);
    }

    fn aim_in_reach(&self) -> bool {
        let (px, py) = (self.player.x + 1.5, self.player.y + 3.0);
        let (dx, dy) = (self.aim.0 as f32 - px, self.aim.1 as f32 - py);
        (dx * dx + dy * dy).sqrt() <= REACH
    }

    /// Pulls the substance under the cursor into its jar, one cell per tick,
    /// taking only that substance so nearby gases aren't scooped up by accident.
    fn gather(&mut self, target: Species) {
        if !self.aim_in_reach() {
            return;
        }
        let (ax, ay) = self.aim;
        let Some(jar) = target.jar() else { return };
        if self.jar(jar) >= JAR_CAPACITY {
            return;
        }
        for r in 0..6i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (x, y) = (ax + dx, ay + dy);
                    if self.grid.get(x, y) == target && self.grid.in_bounds(x, y) {
                        self.add(jar, 1);
                        if jar == "H2O" {
                            self.gathered_water += 1;
                        }
                        self.grid.set(x, y, Species::Air, AMBIENT_C);
                        return;
                    }
                }
            }
        }
    }

    /// The gatherable substance at the aim point, or the nearest one within 3 cells.
    fn substance_near_aim(&self) -> Option<Species> {
        let (ax, ay) = self.aim;
        (0..4i32).find_map(|r| {
            (-r..=r).find_map(|dy| {
                (-r..=r).find_map(|dx| {
                    let s = self.grid.get(ax + dx, ay + dy);
                    (self.grid.in_bounds(ax + dx, ay + dy) && s.jar().is_some()).then_some(s)
                })
            })
        })
    }

    fn touch_crystal(&mut self) {
        // The crystal charges anything within a few cells of it.
        let (cx, cy) = (self.player.x as i32 + 1, self.player.y as i32 + 4);
        let near = (-8..=8)
            .any(|dx| (-8..=8).any(|dy| self.grid.get(cx + dx, cy + dy) == Species::Crystal));
        if near && self.charge < 3 {
            self.charge = 3;
            if !self.met_crystal {
                self.met_crystal = true;
                self.say("Your suit is charged. Electricity can pull water apart. Try casting 2H2O -> 2H2 + O2.");
            }
        }
    }

    fn region_has(&self, x0: i32, x1: i32, pred: impl Fn(Species, f32) -> bool) -> bool {
        (0..HEIGHT as i32)
            .any(|y| (x0..x1).any(|x| pred(self.grid.get(x, y), self.grid.temp_at(x, y))))
    }

    fn complete(&mut self, id: &str, next_hint: &str) {
        if let Some(o) = self.objectives.iter_mut().find(|o| o.id == id && !o.done) {
            o.done = true;
            let hint = next_hint.to_string();
            self.say(hint);
        }
    }

    fn check_objectives(&mut self) {
        if self.gathered_water >= 20 {
            self.complete("gather", "Good. Matter is made of tiny pieces, and you're carrying them. Now aim right at the flames and cast H2O. Keep going until every flame is out.");
        }
        if self.grid.frame > 30 {
            // Still burning: any flame, or hot wood that air can reach.
            let burning = self.region_has(96, 180, |s, _| s == Species::Flame)
                || (96..180).any(|x| {
                    (100..150).any(|y| {
                        self.grid.get(x, y) == Species::Wood
                            && self.grid.temp_at(x, y) >= WOOD_IGNITION_C
                            && [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().any(|(dx, dy)| {
                                self.grid.get(x + dx, y + dy).oxidiser_strength() > 0.0
                            })
                    })
                });
            if !burning {
                self.complete("fire", "Fire needs fuel, oxygen and heat. Water took the heat away. The crystal ahead holds electricity.");
            }
        }
        if self.electrolysis_casts > 0 {
            self.complete("split", "You split water into hydrogen and oxygen. Now put them back together at the torch: 2H2 + O2 -> 2H2O.");
        }
        if !self.torch_lit {
            let lit = (224..232).any(|x| {
                (108..119).any(|y| {
                    self.grid.get(x, y) == Species::Flame
                        || (self.grid.get(x, y) == Species::Wood
                            && self.grid.temp_at(x, y) >= 300.0)
                })
            });
            if lit {
                self.torch_lit = true;
                for y in 128..148 {
                    for x in 236..242 {
                        if self.grid.get(x, y) == Species::Door {
                            self.grid.set(x, y, Species::Air, AMBIENT_C);
                        }
                    }
                }
                self.complete(
                    "torch",
                    "The torch is lit and the door is open. You balanced your first equation!",
                );
            }
        }
        if self.player.x > 280.0 && self.torch_lit {
            let others_done = self
                .objectives
                .iter()
                .filter(|o| o.id != "exit")
                .all(|o| o.done);
            let hint = if others_done {
                "Trial complete. You broke through to the next realm."
            } else {
                "You made it through the door! Go back and finish the goals that are still open."
            };
            self.complete("exit", hint);
        }
        // Finishing the last open goal after walking through also completes the trial.
        if self.objectives.iter().all(|o| o.done) && !self.announced_complete {
            self.announced_complete = true;
            if self.messages.last().map(String::as_str)
                != Some("Trial complete. You broke through to the next realm.")
            {
                self.say("Trial complete. You broke through to the next realm.");
            }
        }
    }

    // ----------------------------------------------------------------- casting

    pub fn cast(&mut self, text: &str) -> CastResult {
        let mut result = self.cast_inner(text);
        // The most common slip for young players: a zero instead of the letter O.
        if !result.ok && text.contains('0') {
            let alt = text.replace('0', "O");
            if self.recognises(&alt) {
                result.message = format!(
                    "Did you type a zero (0) instead of the letter O? {}",
                    result.message
                );
                result.suggestion = Some(alt);
            }
        }
        result
    }

    /// Whether a formula names a jar or a reaction the game knows.
    fn recognises(&self, text: &str) -> bool {
        match parse_equation(text) {
            Ok(eq) if !eq.has_arrow => {
                eq.reactants.len() == 1 && Species::from_jar(&eq.reactants[0].formula).is_some()
            }
            Ok(eq) => self.book.find_by_species(&eq).is_some(),
            Err(_) => false,
        }
    }

    fn cast_inner(&mut self, text: &str) -> CastResult {
        let mut eq = match parse_equation(text) {
            Ok(eq) => eq,
            Err(e) => {
                let r = CastResult {
                    kind: "error".into(),
                    message: e.to_string(),
                    ..Default::default()
                };
                return r;
            }
        };
        if !self.aim_in_reach() {
            return CastResult {
                kind: "error".into(),
                message: "That's too far away. Move closer.".into(),
                ..Default::default()
            };
        }
        if !eq.has_arrow {
            return self.release(&eq);
        }
        let Some((reaction, known)) = self.book.find_by_species(&eq).cloned() else {
            let message = if eq.is_balanced() {
                "Balanced! But this reaction isn't in the codex yet, so nothing happens."
                    .to_string()
            } else {
                "Nothing happens. Mote doesn't know that reaction.".to_string()
            };
            return CastResult {
                kind: "fizzle".into(),
                message,
                equation: eq.to_string(),
                imbalance: eq.imbalance_report(),
                ..Default::default()
            };
        };

        let mut fixed_note = None;
        if !eq.is_balanced() {
            let imbalance = eq.imbalance_report();
            if self.tier == Tier::Sprout {
                fixed_note = Some(format!("Mote fixed the numbers for you: {known}."));
                eq = known.clone();
            } else {
                self.fizzles_in_a_row += 1;
                let offer_fix = self.tier == Tier::Seedling || self.fizzles_in_a_row >= 3;
                let message = format!("Sooty puff! Almost: {}.", imbalance.join("; "));
                self.puff();
                return CastResult {
                    kind: "fizzle".into(),
                    message,
                    equation: eq.to_string(),
                    suggestion: offer_fix.then(|| {
                        eq.balanced_version()
                            .map(|b| b.to_string())
                            .unwrap_or_else(|| known.to_string())
                    }),
                    imbalance,
                    reaction_id: Some(reaction.id.clone()),
                    ..Default::default()
                };
            }
        }
        self.fizzles_in_a_row = 0;

        if Tier::parse(&reaction.tier_min) > self.tier {
            let message = format!(
                "{} unlocks at a higher realm. Keep climbing!",
                reaction.name
            );
            return CastResult {
                kind: "fizzle".into(),
                message,
                equation: eq.to_string(),
                ..Default::default()
            };
        }

        let outcome = match reaction.id.as_str() {
            "electrolysis_water" => self.electrolysis(),
            "water_synthesis" => self.burn_hydrogen(),
            "methane_combustion" => self.burn_methane(),
            "carbon_combustion" => Err(
                "Wood burns on its own once it's hot enough and has air. Try heating it.".into(),
            ),
            _ => Err("That reaction isn't wired up yet.".into()),
        };
        match outcome {
            Err(message) => CastResult {
                kind: "fizzle".into(),
                message,
                equation: eq.to_string(),
                reaction_id: Some(reaction.id.clone()),
                ..Default::default()
            },
            Ok(message) => {
                let first = !self.discovered.contains(&reaction.id);
                if first {
                    self.discovered.push(reaction.id.clone());
                }
                let message = match fixed_note {
                    Some(note) => format!("{note} {message}"),
                    None => message,
                };
                let show_numbers = self.tier >= Tier::Apprentice;
                CastResult {
                    ok: true,
                    kind: "reaction".into(),
                    message,
                    equation: eq.to_string(),
                    reaction_id: Some(reaction.id.clone()),
                    concept: Some(reaction.concept.clone()),
                    real_world: Some(reaction.real_world.clone()),
                    delta_h_kj: show_numbers.then_some(reaction.delta_h_kj),
                    source: show_numbers.then(|| reaction.source.clone()),
                    first_discovery: first,
                    ..Default::default()
                }
            }
        }
    }

    fn jar(&self, k: &str) -> u32 {
        self.jars.get(k).copied().unwrap_or(0)
    }

    fn take(&mut self, k: &str, n: u32) {
        if let Some(v) = self.jars.get_mut(k) {
            *v -= n.min(*v);
        }
    }

    fn add(&mut self, k: &str, n: u32) {
        let v = self.jars.entry(k.to_string()).or_insert(0);
        *v = (*v + n).min(JAR_CAPACITY);
    }

    /// Puts `n` cells of `s` into open space around the aim point, nearest first.
    fn spawn_blob(&mut self, s: Species, n: u32, t: f32) -> u32 {
        let (ax, ay) = self.aim;
        let mut placed = 0;
        for r in 0..12i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if placed >= n {
                        return placed;
                    }
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (ax + dx, ay + dy);
                    if self.grid.in_bounds(x, y) && self.grid.get(x, y).is_gas() {
                        self.grid.set(x, y, s, t);
                        placed += 1;
                    }
                }
            }
        }
        placed
    }

    fn release(&mut self, eq: &Equation) -> CastResult {
        let term = &eq.reactants[0];
        if eq.reactants.len() != 1 {
            let message =
                "To pour from a jar, name one substance, like H2O. To react things, add an arrow."
                    .to_string();
            return CastResult {
                kind: "error".into(),
                message,
                ..Default::default()
            };
        }
        let Some(species) = Species::from_jar(&term.formula) else {
            let message = format!("You don't have a jar of {}.", term.formula);
            return CastResult {
                kind: "error".into(),
                message,
                ..Default::default()
            };
        };
        let have = self.jar(&term.formula);
        if have == 0 {
            let message = format!("Your {} jar is empty. Gather some first.", term.formula);
            return CastResult {
                kind: "fizzle".into(),
                message,
                equation: eq.to_string(),
                ..Default::default()
            };
        }
        let amount = have.min(12u32.saturating_mul(term.coef));
        let placed = self.spawn_blob(species, amount, AMBIENT_C);
        self.take(&term.formula, placed);
        let message = format!("You released {placed} units of {}.", species.props().name);
        CastResult {
            ok: true,
            kind: "release".into(),
            message,
            equation: eq.to_string(),
            ..Default::default()
        }
    }

    fn electrolysis(&mut self) -> Result<String, String> {
        if self.charge == 0 {
            return Err(
                "Splitting water needs electricity. Touch the crystal to charge your suit.".into(),
            );
        }
        let water = self.jar("H2O").min(24) / 2 * 2;
        if water < 2 {
            return Err("You need at least 2 units of water in your jar.".into());
        }
        self.take("H2O", water);
        self.add("H2", water);
        self.add("O2", water / 2);
        self.charge -= 1;
        self.electrolysis_casts += 1;
        Ok(format!("Bubbles fizz: {water} water became {water} hydrogen and {} oxygen. Twice as much hydrogen, just like the equation says.", water / 2))
    }

    fn burn_hydrogen(&mut self) -> Result<String, String> {
        let pairs = (self.jar("H2") / 2).min(self.jar("O2")).min(6);
        if pairs == 0 {
            return Err("You need 2 hydrogen for every 1 oxygen. Split some water first.".into());
        }
        self.take("H2", pairs * 2);
        self.take("O2", pairs);
        self.spawn_blob(Species::Steam, pairs * 2, FLAME_C_IN_O2);
        self.heat_burst(3 + pairs as i32, 900.0);
        Ok(format!(
            "Pop! {} hydrogen and {} oxygen became {} water as hot steam.",
            pairs * 2,
            pairs,
            pairs * 2
        ))
    }

    fn burn_methane(&mut self) -> Result<String, String> {
        let n = self.jar("CH4").min(self.jar("O2") / 2).min(6);
        if n == 0 {
            return Err("You need 1 methane and 2 oxygen. Marsh vents give off methane.".into());
        }
        self.take("CH4", n);
        self.take("O2", n * 2);
        self.spawn_blob(Species::CO2, n, FLAME_C_IN_O2);
        self.spawn_blob(Species::Steam, n * 2, FLAME_C_IN_O2);
        self.heat_burst(4 + n as i32, 1200.0);
        Ok(format!(
            "Whoosh! {n} methane burned into {n} carbon dioxide and {} water.",
            n * 2
        ))
    }

    /// The visible flash and heat of a spell going off.
    fn heat_burst(&mut self, radius: i32, peak: f32) {
        let (ax, ay) = self.aim;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > radius * radius {
                    continue;
                }
                let (x, y) = (ax + dx, ay + dy);
                if !self.grid.in_bounds(x, y) {
                    continue;
                }
                let falloff = 1.0 - (d2 as f32).sqrt() / (radius as f32 + 1.0);
                let i = self.grid.idx(x as usize, y as usize);
                self.grid.temp[i] = self.grid.temp[i].max(AMBIENT_C + peak * falloff);
                if self.grid.cells[i] == Species::Air
                    && d2 >= (radius - 1) * (radius - 1)
                    && self.grid.chance(0.5)
                {
                    self.grid.spawn_flame(x, y, 5, false, peak);
                }
            }
        }
    }

    /// A harmless puff for a fizzled cast: a little warm air, nothing else.
    fn puff(&mut self) {
        let (ax, ay) = self.aim;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (ax + dx, ay + dy);
                if self.grid.in_bounds(x, y) && self.grid.get(x, y) == Species::Air {
                    let i = self.grid.idx(x as usize, y as usize);
                    self.grid.temp[i] = 70.0;
                }
            }
        }
    }

    // ------------------------------------------------------------------- state

    pub fn state(&mut self) -> State {
        let (ax, ay) = self.aim;
        let s = self.grid.get(ax, ay);
        State {
            tier: self.tier,
            jars: self.jars.clone(),
            charge: self.charge,
            objectives: self.objectives.clone(),
            messages: std::mem::take(&mut self.messages),
            player_x: self.player.x,
            player_y: self.player.y,
            discovered: self.discovered.clone(),
            trial_complete: self.objectives.iter().all(|o| o.done),
            aim_temp_c: self.grid.temp_at(ax, ay),
            aim_substance: s.props().name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(tier: Tier) -> Game {
        let mut g = Game::new(tier, 42);
        g.aim = (20, 140);
        g
    }

    #[test]
    fn electrolysis_needs_charge_then_conserves_atoms() {
        let mut g = game(Tier::Apprentice);
        g.jars.insert("H2O".into(), 20);
        let r = g.cast("2H2O -> 2H2 + O2");
        assert!(!r.ok);
        g.charge = 1;
        let r = g.cast("2H2O -> 2H2 + O2");
        assert!(r.ok, "{}", r.message);
        // 20 H2O = 40 H + 20 O  ->  20 H2 (40 H) + 10 O2 (20 O)
        assert_eq!(g.jar("H2"), 20);
        assert_eq!(g.jar("O2"), 10);
        assert_eq!(g.jar("H2O"), 0);
        assert_eq!(r.delta_h_kj, Some(571.7));
    }

    #[test]
    fn unbalanced_cast_fizzles_for_seedling_with_a_fix() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2".into(), 10);
        g.jars.insert("O2".into(), 5);
        let r = g.cast("H2 + O2 -> H2O");
        assert_eq!(r.kind, "fizzle");
        assert_eq!(r.suggestion.as_deref(), Some("2H2 + O2 -> 2H2O"));
        assert_eq!(g.jar("H2"), 10, "a fizzle costs nothing");
    }

    #[test]
    fn sprout_gets_auto_balanced() {
        let mut g = game(Tier::Sprout);
        g.jars.insert("H2".into(), 4);
        g.jars.insert("O2".into(), 2);
        let r = g.cast("H2 + O2 -> H2O");
        assert!(r.ok, "{}", r.message);
        assert_eq!(r.delta_h_kj, None, "young players see words, not kJ");
        assert_eq!(g.jar("H2"), 0);
    }

    #[test]
    fn apprentice_gets_no_fix_until_three_misses() {
        let mut g = game(Tier::Apprentice);
        g.jars.insert("H2".into(), 4);
        g.jars.insert("O2".into(), 2);
        assert!(g.cast("H2 + O2 -> H2O").suggestion.is_none());
        assert!(g.cast("H2 + O2 -> H2O").suggestion.is_none());
        assert!(g.cast("H2 + O2 -> H2O").suggestion.is_some());
    }

    #[test]
    fn zero_instead_of_letter_o_gets_a_hint() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2O".into(), 5);
        let r = g.cast("H20");
        assert!(!r.ok);
        assert_eq!(r.suggestion.as_deref(), Some("H2O"));
    }

    #[test]
    fn gathering_takes_only_the_aimed_substance() {
        let mut g = game(Tier::Seedling);
        g.player.x = 40.0;
        g.aim = (50, 155);
        assert_eq!(g.grid.get(50, 155), Species::Water);
        g.grid.set(51, 155, Species::CO2, AMBIENT_C);
        for _ in 0..10 {
            g.tick(INPUT_GATHER);
        }
        assert_eq!(g.jar("H2O"), 10);
        assert_eq!(g.jar("CO2"), 0);
    }

    #[test]
    fn release_pours_water_into_the_world() {
        let mut g = game(Tier::Sprout);
        g.aim = (60, 100);
        g.jars.insert("H2O".into(), 10);
        let before = g.grid.count(Species::Water);
        let r = g.cast("H2O");
        assert!(r.ok);
        assert_eq!(g.grid.count(Species::Water), before + 10);
        assert_eq!(g.jar("H2O"), 0);
    }

    #[test]
    fn the_trial_can_be_finished() {
        let mut g = game(Tier::Apprentice);
        // Put the fire out by flooding the wood.
        for _ in 0..30 {
            g.tick(0);
        }
        for x in 100..175 {
            for y in 100..148 {
                if g.grid.get(x, y) == Species::Flame {
                    g.grid.set(x, y, Species::Air, AMBIENT_C);
                }
                if g.grid.get(x, y) == Species::Wood {
                    let i = g.grid.idx(x as usize, y as usize);
                    g.grid.temp[i] = AMBIENT_C;
                }
            }
        }
        g.gathered_water = 20;
        g.jars.insert("H2O".into(), 24);
        g.charge = 1;
        g.player.x = 200.0;
        g.aim = (228, 108);
        assert!(g.cast("2H2O -> 2H2 + O2").ok);
        assert!(g.cast("2H2 + O2 -> 2H2O").ok);
        for _ in 0..120 {
            g.tick(0);
        }
        assert!(g.torch_lit, "torch should catch fire from the hydrogen pop");
        g.player.x = 285.0;
        g.tick(0);
        let st = g.state();
        assert!(st.trial_complete, "{:?}", st.objectives);
    }
}

#[cfg(test)]
mod balance {
    use super::*;

    fn burning_wood(g: &Game) -> usize {
        (96..180)
            .flat_map(|x| (100..150).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                g.grid.get(x, y) == Species::Wood && g.grid.temp_at(x, y) >= WOOD_IGNITION_C
            })
            .count()
    }

    #[test]
    fn the_fire_spreads_if_ignored() {
        let mut g = Game::new(Tier::Seedling, 42);
        for _ in 0..60 {
            g.tick(0);
        }
        let early = burning_wood(&g);
        for _ in 0..900 {
            g.tick(0);
        }
        assert!(burning_wood(&g) > early, "fire should be a real threat");
        assert!(!g.objectives[1].done);
    }

    #[test]
    fn aiming_water_at_the_fire_puts_it_out() {
        let mut g = Game::new(Tier::Seedling, 42);
        for _ in 0..30 {
            g.tick(0);
        }
        g.player.x = 60.0;
        g.jars.insert("H2O".into(), 200);
        for _ in 0..16 {
            // Aim at burning wood, like a player chasing the fire.
            let target = (96..180)
                .flat_map(|x| (100..150).map(move |y| (x, y)))
                .find(|&(x, y)| {
                    g.grid.get(x, y) == Species::Wood && g.grid.temp_at(x, y) >= WOOD_IGNITION_C
                });
            let Some(aim) = target else { break };
            g.aim = aim;
            g.cast("H2O");
            for _ in 0..20 {
                g.tick(0);
            }
        }
        for _ in 0..120 {
            g.tick(0);
        }
        assert!(
            g.objectives[1].done,
            "fire still burning: {} hot wood",
            burning_wood(&g)
        );
    }
}

#[cfg(test)]
mod perf {
    use super::*;
    use std::hash::{Hash, Hasher};

    fn grid_hash(g: &Game) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (i, c) in g.grid.cells.iter().enumerate() {
            (*c as u8).hash(&mut h);
            g.grid.temp[i].to_bits().hash(&mut h);
            g.grid.aux[i].hash(&mut h);
        }
        h.finish()
    }

    fn run(seed: u64, ticks: u32) -> Game {
        let mut g = Game::new(Tier::Seedling, seed);
        for t in 0..ticks {
            // Walk right for a while so the player and camera move too.
            g.tick(if t < 300 { INPUT_RIGHT } else { 0 });
        }
        g
    }

    #[test]
    fn same_seed_gives_identical_grid_after_600_ticks() {
        assert_eq!(grid_hash(&run(7, 600)), grid_hash(&run(7, 600)));
        assert_ne!(
            grid_hash(&run(7, 600)),
            grid_hash(&run(8, 600)),
            "different seeds should diverge"
        );
    }

    /// Reports ms/step; run with `cargo test --release bench_600_ticks -- --nocapture`.
    /// It only asserts a very generous ceiling so it never flakes on slow CI machines.
    #[test]
    fn bench_600_ticks() {
        let mut g = Game::new(Tier::Seedling, 42);
        let start = std::time::Instant::now();
        for _ in 0..600 {
            g.tick(0);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / 600.0;
        println!("bench_600_ticks: {ms:.3} ms/step");
        assert!(ms < 250.0, "{ms} ms/step");
    }
}
