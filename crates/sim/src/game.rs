//! Game rules on top of the grid: the player, reagent jars, casting, and the
//! first breakthrough trial.

use crate::formula::Atoms;
use crate::grid::{Grid, AMBIENT_C, CH4_IGNITION_C, WOOD_IGNITION_C};
use crate::reactions::ReactionBook;
use crate::species::{Phase, Species};
use crate::spell::{self, Outcome, Plan, Rejection, Resolution, Stock};
use serde::Serialize;
use std::collections::BTreeMap;

mod rooms;
pub use rooms::{Goal, Rect, TorchView, DOOR_TOP, GATE_W, GATE_X, GROUND_Y};

/// The world is four 320-cell rooms side by side; the camera shows one room's width.
pub const ROOM_W: usize = 320;
pub const ROOMS: usize = 4;
pub const WORLD_W: usize = ROOM_W * ROOMS;
pub const VIEW_W: usize = 320;
/// The simulated strip around the camera: half a room behind its left edge
/// and half a room beyond its right edge, `[view_x - BEHIND, view_x + AHEAD)`.
/// Everything farther away freezes until the camera comes back (the Noita
/// approach). Narrower than the first cut (a whole room each side) so a slow
/// device keeps one sim step per frame.
pub const ACTIVE_BEHIND: usize = VIEW_W / 2;
pub const ACTIVE_AHEAD: usize = VIEW_W + VIEW_W / 2;
pub const HEIGHT: usize = 180;
/// How quickly the camera catches up once the player leaves the deadzone.
const CAMERA_LERP: f32 = 0.2;

pub const INPUT_LEFT: u32 = 1;
pub const INPUT_RIGHT: u32 = 2;
pub const INPUT_JUMP: u32 = 4;
pub const INPUT_GATHER: u32 = 8;

/// Suit gauges, in moles of game matter: one water cast uses 1 H2O.
const H2O_CAP: u32 = 10;
/// The condenser collar adds one water cast this often (90 ticks = 1.5 s).
pub const CONDENSE_TICKS: u32 = 90;
/// The wrist electrolyser splits condensed water this often: fast beside a
/// crystal, a slow trickle from the solar strip elsewhere.
const ELECTRO_TICKS_NEAR: u32 = 12;
const ELECTRO_TICKS_SOLAR: u32 = 150;
/// The electrolyser leaves at least this many water casts in the tank.
const SPLIT_RESERVE: u32 = 2;
/// How close (in cells) the player must be to a crystal to draw on it.
const CRYSTAL_RANGE: i32 = 16;
/// The suit's battery (the ⚡ gauge) holds enough energy for this many
/// splits. Every split, by the electrolyser or a split spell, draws the
/// sourced +571.7 kJ per 2 H2O from it.
const CRYSTAL_SPLITS: f32 = 10.0;
/// Standing by a crystal charges the battery fast (a split every
/// `ELECTRO_TICKS_NEAR` ticks costs a little more, so it still visibly dips)...
const CRYSTAL_CHARGE_KJ_PER_TICK: f32 = 40.0;
/// ...and away from one, only the solar strip charges it, slower than the
/// electrolyser spends, so the gauge runs down while it works.
const SOLAR_KJ_PER_TICK: f32 = 2.0;
/// The name the HUD and labels use for the suit's battery energy.
pub const ENERGY: &str = "⚡";
/// One water cast becomes a stream this many cells big...
const WATER_CELLS_PER_MOL: u32 = 40;
/// ...that cools everything within this radius of where it lands...
const WATER_COOL_RADIUS: i32 = 6;
/// How far past a heat burst's radius ice still melts, in cells.
const ICE_MELT_EXTRA: i32 = 3;
/// Game scale for ice: one cell of ice stands for this many moles of water
/// (5.4 g). A blast's heat melts |dH| / 6.01 kJ/mol (enthalpy of fusion,
/// CRC, from the reaction table) moles, so a balanced boom (-483.6 kJ, NIST)
/// melts about 80 mol, which is 268 cells: the whole 240-cell wall.
const ICE_MOL_PER_CELL: f32 = 0.3;
/// Heat spreads through ice it touches up to this far from the blast.
const ICE_SPREAD_REACH: i32 = 24;
/// ...and soaks through this many burning cells of a hedge it hits.
const SOAK_CELLS_PER_MOL: u32 = 160;
/// One mole of the cloud spell is this many cells of CO2...
const CO2_CELLS_PER_MOL: u32 = 40;
/// ...that cools this many burning cells (it cools less than water does).
const CO2_SOAK_CELLS_PER_MOL: u32 = 60;
/// CO2 sublimes at -78.5 C at 1 atm (CRC Handbook): an extinguisher's gas
/// leaves the nozzle about this cold.
const DRY_ICE_C: f32 = -78.5;
/// Gas spells and gathering: cells per mole of gas.
const GAS_CELLS_PER_MOL: u32 = 3;
/// Gathering: cells of water per mole, and how far (cells) the suit can pull from.
const WATER_GATHER_CELLS_PER_MOL: u32 = 4;
const GATHER_RADIUS: i32 = 48;
/// Gather held this long (0.5 s) with nothing coming in says why.
const GATHER_WHY_TICKS: u32 = 30;
/// Sprout and Seedling aim at the obvious target when the player has not
/// aimed on purpose for this long (2 s).
const AUTO_AIM_TICKS: u64 = 120;
/// Pointing within this many cells of a goal marker locks onto that goal's
/// real target, on every tier (the marker floats a little off the target).
const MARKER_SNAP: f32 = 12.0;
pub const PLAYER_W: f32 = 6.0;
pub const PLAYER_H: f32 = 12.0;
/// Where a new trial starts: beside the burning hedge, already within soft
/// auto-aim reach of it, so the very first spell lands on the fire.
pub const SPAWN_X: f32 = 210.0;
/// Movement tuning, in cells and ticks (60 ticks a second).
const RUN_MAX: f32 = 2.2;
/// Reaches RUN_MAX in under 7 ticks from a standstill.
const RUN_ACCEL: f32 = 0.34;
const RUN_DECEL: f32 = 0.5;
const SWIM_MAX: f32 = 1.1;
/// Top horizontal speed in the air, so a held jump can't fling the player
/// past a room's puzzle (still enough to clear a crystal or a curb).
const AIR_MAX: f32 = 1.5;
const JUMP_V: f32 = -4.5;
/// Letting go of jump while rising keeps only this share of the upward speed.
const JUMP_CUT: f32 = 0.45;
/// A tapped jump rises at full speed for this many ticks before the cut, so
/// a one-tick tap still clears 16 cells: a hedge stump or a crate.
const MIN_JUMP_TICKS: u8 = 4;
/// Grounded walking climbs ledges up to this many cells without a jump.
const STEP_UP: i32 = 2;
const GRAVITY: f32 = 0.3;
const MAX_FALL: f32 = 5.0;
/// A jump still works this many ticks after walking off an edge...
const COYOTE_TICKS: u8 = 6;
/// ...and a jump pressed this many ticks before landing fires on landing.
const JUMP_BUFFER_TICKS: u8 = 6;
/// Landings faster than this make a dust puff.
const LAND_FX_SPEED: f32 = 2.0;
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

/// Everything a cast did, for the HUD, the floating label and the tests.
/// Every cast returns one of these; there is no bare error path.
#[derive(Debug, Clone, Serialize)]
pub struct CastOutcome {
    /// Matches the `id` on this cast's FX events.
    pub id: u32,
    pub outcome: Outcome,
    /// "water", "boom", "split", "cloud", "gas", "fireball", "burn" or "unknown".
    pub spell: &'static str,
    /// What the player typed.
    pub input: String,
    /// The formula actually cast (after aliases and auto-balancing).
    pub equation: String,
    pub reaction_id: Option<String>,
    /// Atoms taken from the suit and the air.
    pub consumed: Atoms,
    /// Atoms taken from the air (part of `consumed`).
    pub from_air: Atoms,
    /// Molecules made, or poured out for a plain substance.
    pub products: Stock,
    /// Atoms that did not react and went back to the suit.
    pub leftovers: Atoms,
    /// Reagents the suit was short of.
    pub missing: Vec<String>,
    /// Where to get the first missing reagent, if any is in the world.
    pub source_x: Option<i32>,
    pub source_y: Option<i32>,
    /// Share of the full effect, 0..=1.
    pub power: f32,
    /// Enthalpy of what actually reacted, kJ (from the sourced table).
    pub dh_kj: f32,
    /// A short tier-appropriate line for the floating label.
    pub label: String,
    /// A corrected equation the UI can offer as a one-tap fix.
    pub suggestion: Option<String>,
    pub imbalance: Vec<String>,
    /// Reagent molecules that did not react ("1 H2", "1/2 O2").
    pub unreacted: Vec<String>,
    /// The reaction that actually ran, scaled from the book's balanced
    /// equation: "H2 + 1/2 O2 -> H2O". Its ΔH is `dh_kj`. None for pours
    /// and casts where nothing reacted.
    pub reacted: Option<String>,
    /// What the cast spent and made, from the reacted coefficients:
    /// "-2 H2, -1 O2, +2 H2O". None when nothing happened.
    pub ledger: Option<String>,
    /// Where the products went: "2 H2O released as steam".
    pub went: Option<String>,
    /// The spell in its simplest balanced form ("2H2 + O2 -> 2H2O" for
    /// "4H2 + 2O2 -> 4H2O"), or the substance for a pour. None when the
    /// cast is not balanced: only balanced spells are kept as hotkeys.
    pub canonical: Option<String>,
    /// How many times the simplest form the player cast (2 for 4H2 + 2O2).
    pub multiple: u32,
    pub concept: Option<String>,
    pub real_world: Option<String>,
    /// Shown at Apprentice tier and above, with its source.
    pub delta_h_kj: Option<f32>,
    pub source: Option<String>,
    pub first_discovery: bool,
    /// Why an `Unknown` cast was rejected: "empty", "parse", "element",
    /// "not_in_world", "unbalanced", "state" or "zero".
    pub reason: Option<&'static str>,
    /// Apprentice only: the reaction with state symbols, matching the ΔH
    /// shown (`2H2(g) + O2(g) -> 2H2O(g)`).
    pub equation_states: Option<String>,
    /// Apprentice only: a short teaching note for the cast card, such as
    /// "symbols are case-sensitive" when lower-case typing was forgiven.
    pub note: Option<String>,
}

/// A presentation-only event for the browser (particles, sound, labels).
/// Never feeds back into the simulation.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Fx {
    pub kind: &'static str,
    pub x: f32,
    pub y: f32,
    /// A second point: the bolt's aim, the nearest source for a fizzle, the
    /// player's hand for leftover orbs, or the crystal for an arc.
    pub tx: f32,
    pub ty: f32,
    pub dh_kj: f32,
    pub outcome: Option<Outcome>,
    pub leftovers: Atoms,
    pub reaction_id: Option<String>,
    /// The cast this event belongs to (0 for events not caused by a cast).
    pub id: u32,
    /// Kind-specific strength, such as landing speed or effect power.
    pub power: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct State {
    pub tier: Tier,
    /// Suit gauges in moles (one water cast is 1 H2O).
    pub jars: BTreeMap<String, u32>,
    pub jar_caps: BTreeMap<String, u32>,
    /// Energy stored in the crystal battery, kJ, and its capacity.
    pub crystal_kj: f32,
    pub crystal_max_kj: f32,
    /// Whether the player stands close enough to draw on a crystal.
    pub near_crystal: bool,
    /// What charges the battery now: "crystal" (fast) or "solar" (slow).
    pub charge_source: &'static str,
    /// Gauge changes the suit made by itself since the last state, per jar
    /// (condenser, electrolyser, CO2 scrubber), so the HUD can tell them
    /// apart from what the player's own casts spent and made.
    pub background: BTreeMap<String, i32>,
    /// Bumps whenever anything on the HUD changes.
    pub state_version: u64,
    pub objectives: Vec<Objective>,
    pub messages: Vec<String>,
    pub player_x: f32,
    pub player_y: f32,
    pub discovered: Vec<String>,
    pub trial_complete: bool,
    pub aim_temp_c: f32,
    pub aim_substance: String,
    pub aim_x: i32,
    pub aim_y: i32,
    /// Left edge of the camera, in world cells.
    pub view_x: i32,
    /// The room the player is in (0-based) and which gates are open.
    pub room: u8,
    pub gates_open: [bool; ROOMS],
    /// Each room's target, for goal icons and fire meters.
    pub goals: Vec<Goal>,
    pub torch: TorchView,
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    /// Position at the start of the last tick, so the renderer can interpolate.
    pub prev_x: f32,
    pub prev_y: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing: i32,
    /// Distance walked, for the walk-cycle animation.
    pub stride: f32,
    pub grounded: bool,
    coyote: u8,
    jump_buffer: u8,
    jump_held: bool,
    /// A jump pressed since the last tick (latched by `press_jump`), so a tap
    /// released before the next tick still jumps.
    jump_tap: bool,
    /// True while rising from a jump the player can still cut short.
    jumping: bool,
    /// Ticks since the jump started, so even a tap rises `MIN_JUMP_TICKS`.
    jump_ticks: u8,
}

pub struct Game {
    pub grid: Grid,
    pub player: Player,
    pub tier: Tier,
    /// Suit gauges in moles.
    pub jars: BTreeMap<String, u32>,
    /// Atoms handed back by unbalanced casts that don't yet make a whole
    /// molecule (one O waits here until a second joins it as O2).
    pub loose: Atoms,
    /// The suit's battery (⚡), kJ: every split draws on it; a crystal
    /// (fast) or the solar strip (slow) charges it.
    pub crystal_kj: f32,
    pub crystal_max_kj: f32,
    /// Energy to split 2 H2O, kJ (the sourced +571.7 from the reaction table).
    split_kj: f32,
    /// Enthalpy of fusion of water, kJ/mol (sourced, from the reaction table).
    fusion_kj: f32,
    condense_timer: u32,
    electro_timer: u32,
    gather_cells: u32,
    /// Gauge changes the suit made by itself (condenser, electrolyser, CO2
    /// scrubber) since the HUD last asked, per jar. Not the player's casts.
    background: BTreeMap<String, i32>,
    pub objectives: Vec<Objective>,
    messages: Vec<String>,
    fx: Vec<Fx>,
    pub aim: (i32, i32),
    /// The tick the player last aimed on purpose (for soft auto-aim).
    aim_tick: u64,
    pub ticks: u64,
    /// Camera left edge in world cells, and its value one tick ago (for
    /// interpolation). A pure function of the player's path, so deterministic.
    pub view_x: f32,
    pub prev_view_x: f32,
    book: ReactionBook,
    pub discovered: Vec<String>,
    misses_in_a_row: u32,
    next_cast_id: u32,
    torch_lit: bool,
    pub rooms: rooms::Rooms,
    met_crystal: bool,
    gather_target: Option<Species>,
    /// Ticks gather has been held this press, whether anything came in,
    /// and whether the reason nothing did has been said.
    gather_held: u32,
    gather_got: bool,
    gather_told: bool,
    bolts: Vec<Bolt>,
    state_version: u64,
    state_sig: u64,
    /// The seed the level was built from (restarting a room rebuilds it).
    seed: u64,
    /// Step every column each tick instead of only the active region around
    /// the camera (the worst case, for benchmarks and tooling).
    pub step_all_columns: bool,
}

impl Game {
    pub fn new(tier: Tier, seed: u64) -> Game {
        let book = ReactionBook::load();
        let grid = Grid::new(
            WORLD_W,
            HEIGHT,
            seed,
            book.latent_heat("water_boiling"),
            book.latent_heat("water_melting"),
        );
        let split_kj = book
            .by_id("electrolysis_water")
            .map(|(r, _)| r.delta_h_kj)
            .unwrap_or(571.7);
        let mut game = Game {
            grid,
            player: Player {
                x: SPAWN_X,
                y: 130.0,
                prev_x: SPAWN_X,
                prev_y: 130.0,
                vx: 0.0,
                vy: 0.0,
                facing: 1,
                stride: 0.0,
                grounded: false,
                coyote: 0,
                jump_buffer: 0,
                jump_held: false,
                jump_tap: false,
                jumping: false,
                jump_ticks: 0,
            },
            tier,
            // A full water tank, and gas "left over from your last trial":
            // enough that the first spells work with no gathering at all.
            jars: [
                ("H2O", H2O_CAP),
                ("H2", 4),
                ("O2", 2),
                ("CH4", 0),
                ("CO2", 0),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect(),
            loose: Atoms::new(),
            crystal_kj: split_kj * CRYSTAL_SPLITS,
            crystal_max_kj: split_kj * CRYSTAL_SPLITS,
            split_kj,
            fusion_kj: book.latent_heat("water_melting"),
            condense_timer: 0,
            electro_timer: 0,
            gather_cells: 0,
            background: BTreeMap::new(),
            objectives: vec![],
            messages: vec![],
            fx: vec![],
            aim: (40, 140),
            aim_tick: 0,
            ticks: 0,
            view_x: 0.0,
            prev_view_x: 0.0,
            step_all_columns: false,
            book,
            discovered: vec![],
            misses_in_a_row: 0,
            next_cast_id: 1,
            torch_lit: false,
            rooms: rooms::Rooms::new(),
            met_crystal: false,
            gather_target: None,
            gather_held: 0,
            gather_got: false,
            gather_told: false,
            bolts: vec![],
            state_version: 1,
            state_sig: 0,
            seed,
        };
        game.build_level();
        game.snap_camera();
        game
    }

    fn say(&mut self, text: impl Into<String>) {
        self.messages.push(text.into());
    }

    fn emit(&mut self, fx: Fx) {
        // Bounded so a page that stops draining can't grow memory forever.
        if self.fx.len() < 256 {
            self.fx.push(fx);
        }
    }

    /// A simple event with no reaction attached.
    fn emit_at(&mut self, kind: &'static str, x: f32, y: f32, power: f32) {
        self.emit(Fx {
            kind,
            x,
            y,
            tx: x,
            ty: y,
            dh_kj: 0.0,
            outcome: None,
            leftovers: Atoms::new(),
            reaction_id: None,
            id: 0,
            power,
        });
    }

    /// Drains the presentation events since the last call.
    pub fn take_fx(&mut self) -> Vec<Fx> {
        std::mem::take(&mut self.fx)
    }

    // ----------------------------------------------------------------- update

    pub fn tick(&mut self, input: u32) {
        self.ticks += 1;
        self.player.prev_x = self.player.x;
        self.player.prev_y = self.player.y;
        self.move_player(input);
        self.update_camera();
        self.gather_tick(input & INPUT_GATHER != 0);
        self.advance_bolts();
        // Only the area around the camera is simulated; distant rooms wait.
        let vx = self.view_x.round() as usize;
        if self.step_all_columns {
            self.grid.step();
        } else {
            self.grid
                .step_columns(vx.saturating_sub(ACTIVE_BEHIND), vx + ACTIVE_AHEAD);
        }
        self.suit_tick();
        self.update_rooms(input & INPUT_GATHER != 0);
        self.refresh_version();
    }

    fn camera_bounds(x: f32) -> f32 {
        x.clamp(0.0, (WORLD_W - VIEW_W) as f32)
    }

    /// Puts the camera where it would settle, with no lerp (new game, teleports).
    pub fn snap_camera(&mut self) {
        let cx = self.player.x + PLAYER_W / 2.0;
        self.view_x = Self::camera_bounds(cx - VIEW_W as f32 / 2.0);
        self.prev_view_x = self.view_x;
    }

    /// Follows the player with a deadzone over the middle third of the view.
    fn update_camera(&mut self) {
        self.prev_view_x = self.view_x;
        let cx = self.player.x + PLAYER_W / 2.0;
        let third = VIEW_W as f32 / 3.0;
        let on_screen = cx - self.view_x;
        let target = if on_screen < third {
            cx - third
        } else if on_screen > 2.0 * third {
            cx - 2.0 * third
        } else {
            self.view_x
        };
        self.view_x = Self::camera_bounds(self.view_x + (target - self.view_x) * CAMERA_LERP);
    }

    /// Cool trees are scenery the player walks in front of; burning wood and
    /// everything else solid blocks.
    fn solid_at(&self, x: f32, y: f32) -> bool {
        let (cx, cy) = (x.floor() as i32, y.floor() as i32);
        rooms::blocks_player(self.grid.get(cx, cy), self.grid.temp_at(cx, cy))
    }

    /// Whether the player's box at (x, y) overlaps any blocking cell.
    fn box_blocked(&self, x: f32, y: f32) -> bool {
        let (x0, x1) = (x.floor() as i32, (x + PLAYER_W - 0.01).floor() as i32);
        let (y0, y1) = (y.floor() as i32, (y + PLAYER_H - 0.01).floor() as i32);
        (y0..=y1).any(|cy| (x0..=x1).any(|cx| self.solid_at(cx as f32, cy as f32)))
    }

    fn in_water(&self) -> bool {
        let p = &self.player;
        self.grid
            .get((p.x + PLAYER_W / 2.0) as i32, (p.y + PLAYER_H - 2.0) as i32)
            == Species::Water
    }

    fn on_ground(&self) -> bool {
        self.box_blocked(self.player.x, self.player.y + 0.5)
    }

    fn move_player(&mut self, input: u32) {
        let swimming = self.in_water();
        let grounded = self.on_ground();
        let held = input & INPUT_JUMP != 0;
        let tap = std::mem::take(&mut self.player.jump_tap);
        let pressed = (held && !self.player.jump_held) || tap;
        self.player.jump_held = held;

        // Run: accelerate toward the target speed, stop quickly when let go.
        let max = if swimming {
            SWIM_MAX
        } else if grounded {
            RUN_MAX
        } else {
            AIR_MAX
        };
        let mut target_vx = 0.0;
        if input & INPUT_LEFT != 0 {
            target_vx -= max;
            self.player.facing = -1;
        }
        if input & INPUT_RIGHT != 0 {
            target_vx += max;
            self.player.facing = 1;
        }
        let rate = if target_vx == 0.0 {
            RUN_DECEL
        } else {
            RUN_ACCEL
        };
        let dv = (target_vx - self.player.vx).clamp(-rate, rate);
        self.player.vx += dv;
        if !grounded && !swimming {
            self.player.vx = self.player.vx.clamp(-AIR_MAX, AIR_MAX);
        }

        // Coyote time and jump buffering make jumps forgiving.
        let p = &mut self.player;
        p.coyote = if grounded {
            COYOTE_TICKS
        } else {
            p.coyote.saturating_sub(1)
        };
        p.jump_buffer = if pressed {
            JUMP_BUFFER_TICKS
        } else {
            p.jump_buffer.saturating_sub(1)
        };
        if p.jump_buffer > 0 && p.coyote > 0 {
            p.vy = JUMP_V;
            p.jump_buffer = 0;
            p.coyote = 0;
            p.jumping = true;
            p.jump_ticks = 0;
        } else if held && swimming {
            // At the surface a kick lifts you out; underwater you swim up.
            let head = self.grid.get(
                (self.player.x + PLAYER_W / 2.0) as i32,
                (self.player.y - 1.0) as i32,
            );
            self.player.vy = if head == Species::Water {
                (self.player.vy - 0.4).max(-1.2)
            } else {
                -3.6
            };
        }
        let p = &mut self.player;
        p.jump_ticks = p.jump_ticks.saturating_add(1);
        if p.jumping && ((!held && p.jump_ticks > MIN_JUMP_TICKS) || p.vy >= 0.0) {
            // Letting go early cuts the jump short: tap for a hop, hold to go high.
            if p.vy < 0.0 {
                p.vy *= JUMP_CUT;
            }
            p.jumping = false;
        }
        p.vy = if swimming {
            (p.vy + 0.06).min(0.8)
        } else {
            (p.vy + GRAVITY).min(MAX_FALL)
        };

        // Horizontal, in steps of at most one cell, each with a step-up of
        // up to `STEP_UP` cells so bumps and low ledges don't stop you.
        let steps = self.player.vx.abs().ceil().max(1.0) as i32;
        let sx = self.player.vx / steps as f32;
        for _ in 0..steps {
            let (x, y) = (self.player.x + sx, self.player.y);
            if !self.box_blocked(x, y) {
                self.player.x = x;
                continue;
            }
            // On the ground or wading: a swimmer bumping a low pit lip
            // steps out the same way.
            let climb = if grounded || swimming {
                (1..=STEP_UP).find(|&up| !self.box_blocked(x, y - up as f32))
            } else {
                None
            };
            if let Some(up) = climb {
                self.player.x = x;
                self.player.y -= up as f32;
            } else {
                self.player.vx = 0.0;
                break;
            }
        }
        self.player.stride += (self.player.x - self.player.prev_x).abs();
        // Vertical, in small steps so fast falls don't tunnel.
        let steps = (self.player.vy.abs() / 0.5).ceil().max(1.0) as i32;
        for _ in 0..steps {
            let ny = self.player.y + self.player.vy / steps as f32;
            if self.box_blocked(self.player.x, ny) {
                if self.player.vy > LAND_FX_SPEED && !swimming {
                    let (fx, fy) = (self.player.x + PLAYER_W / 2.0, self.player.y + PLAYER_H);
                    let power = self.player.vy;
                    self.emit_at("land", fx, fy, power);
                }
                self.player.vy = 0.0;
                self.player.jumping = false;
                break;
            }
            self.player.y = ny;
        }
        self.player.x = self.player.x.clamp(0.0, WORLD_W as f32 - PLAYER_W);
        self.player.y = self.player.y.clamp(0.0, HEIGHT as f32 - PLAYER_H);
        self.player.grounded = self.on_ground();
    }

    /// A jump key went down: the next tick jumps (when the player can) even
    /// if the key is already up again by then, so a quick tap never gets lost
    /// between two ticks.
    pub fn press_jump(&mut self) {
        self.player.jump_tap = true;
    }

    /// Where spells leave the gauntlet.
    pub fn hand(&self) -> (f32, f32) {
        let p = &self.player;
        (p.x + PLAYER_W / 2.0 + p.facing as f32 * 4.0, p.y + 4.0)
    }

    fn aim_in_reach(&self) -> bool {
        let (px, py) = (
            self.player.x + PLAYER_W / 2.0,
            self.player.y + PLAYER_H / 3.0,
        );
        let (dx, dy) = (self.aim.0 as f32 - px, self.aim.1 as f32 - py);
        (dx * dx + dy * dy).sqrt() <= REACH
    }

    /// Points the gauntlet. `on_purpose` is false when the browser only keeps
    /// the aim under a resting pointer while the camera scrolls.
    pub fn set_aim(&mut self, x: i32, y: i32, on_purpose: bool) {
        let aim = (
            x.clamp(0, WORLD_W as i32 - 1),
            y.clamp(0, HEIGHT as i32 - 1),
        );
        if on_purpose {
            self.aim_tick = self.ticks.max(1);
        }
        self.aim = aim;
    }

    /// The room goal's target, when the bolt can reach it from the hand.
    fn goal_in_range(&self) -> Option<(i32, i32)> {
        let (hx, hy) = self.hand();
        self.goal_aim_point().filter(|&(x, y)| {
            ((x as f32 - hx).powi(2) + (y as f32 - hy).powi(2)).sqrt() <= BOLT_RANGE
        })
    }

    /// Where the reticle is drawn: the aim, or, before the player has
    /// pointed anywhere, the goal a typed spell will fly at.
    pub fn shown_aim(&self) -> (i32, i32) {
        if self.aim_tick == 0 {
            self.goal_in_range().unwrap_or(self.aim)
        } else {
            self.aim
        }
    }

    /// Keyboard aiming: points at the current room's goal on purpose.
    /// Returns false (and leaves the aim) when there is no goal in range.
    pub fn aim_at_goal(&mut self) -> bool {
        match self.goal_in_range() {
            Some((x, y)) => {
                self.set_aim(x, y, true);
                true
            }
            None => false,
        }
    }

    pub fn jar_cap(formula: &str) -> u32 {
        match formula {
            "H2O" => H2O_CAP,
            "H2" => 12,
            "O2" => 6,
            _ => 10,
        }
    }

    /// Visits cells in square rings around `(cx, cy)`, nearest ring first,
    /// until `f` returns true or `max_r` is passed. Deterministic order.
    fn rings(cx: i32, cy: i32, max_r: i32, mut f: impl FnMut(i32, i32) -> bool) {
        for r in 0..=max_r {
            for dy in -r..=r {
                for dx in -r..=r {
                    if (dx.abs() == r || dy.abs() == r) && f(cx + dx, cy + dy) {
                        return;
                    }
                }
            }
        }
    }

    /// The nearest gatherable cell to the hand whose gauge has room.
    fn nearest_gatherable(&self) -> Option<Species> {
        let (hx, hy) = self.hand();
        let mut found = None;
        Self::rings(hx as i32, hy as i32, GATHER_RADIUS, |x, y| {
            let s = self.grid.get(x, y);
            if !self.grid.in_bounds(x, y) {
                return false;
            }
            match s.jar() {
                Some(j) if self.jar(j) < Self::jar_cap(j) => {
                    found = Some(s);
                    true
                }
                _ => false,
            }
        });
        found
    }

    /// Gather (G): pulls the substance under the aim if there is one in reach,
    /// otherwise the nearest gatherable source, a few cells per tick. It locks
    /// onto that substance while held, so nearby gases aren't scooped up by accident.
    fn gather_tick(&mut self, held: bool) {
        if !held {
            self.gather_target = None;
            self.gather_cells = 0;
            self.gather_held = 0;
            self.gather_got = false;
            self.gather_told = false;
            return;
        }
        self.gather_held += 1;
        // Gathering skips full jars: aiming at water with a full water jar
        // still pulls the nearest gas that has room.
        let has_room = |g: &Self, s: Species| s.jar().is_some_and(|j| g.jar(j) < Self::jar_cap(j));
        let aimed = self
            .substance_near_aim()
            .filter(|&s| self.aim_in_reach() && has_room(self, s));
        if self.gather_target.is_none_or(|s| !has_room(self, s)) {
            self.gather_target = aimed.or_else(|| self.nearest_gatherable());
        }
        let Some(target) = self.gather_target else {
            self.gather_says_why();
            return;
        };
        let Some(jar) = target.jar() else { return };
        let (origin, radius) = if aimed == Some(target) {
            (self.aim, 6)
        } else {
            let (hx, hy) = self.hand();
            ((hx as i32, hy as i32), GATHER_RADIUS)
        };
        let mut picked = Vec::with_capacity(3);
        Self::rings(origin.0, origin.1, radius, |x, y| {
            if self.grid.in_bounds(x, y) && self.grid.get(x, y) == target {
                picked.push((x, y));
            }
            picked.len() >= 3
        });
        let per_mol = if jar == "H2O" {
            WATER_GATHER_CELLS_PER_MOL
        } else {
            GAS_CELLS_PER_MOL
        };
        if picked.is_empty() {
            self.gather_says_why();
        }
        if let Some(&(x, y)) = picked.first() {
            self.gather_got = true;
            // The suction beam: from the cells being pulled to the hand.
            let (hx, hy) = self.hand();
            self.emit(Fx {
                kind: "gather",
                x: x as f32,
                y: y as f32,
                tx: hx,
                ty: hy,
                dh_kj: 0.0,
                outcome: None,
                leftovers: Atoms::new(),
                reaction_id: Some(jar.to_string()),
                id: 0,
                power: picked.len() as f32,
            });
        }
        for (x, y) in picked {
            self.grid.set(x, y, Species::Air, AMBIENT_C);
            self.gather_cells += 1;
            if self.gather_cells >= per_mol {
                self.gather_cells = 0;
                self.gain(jar, 1, "gather", (x as f32, y as f32));
            }
        }
    }

    /// Gather has been held a moment and nothing came in: say why, once per
    /// press (every jar in reach is full, or there is nothing to gather).
    fn gather_says_why(&mut self) {
        if self.gather_got || self.gather_told || self.gather_held < GATHER_WHY_TICKS {
            return;
        }
        self.gather_told = true;
        let (hx, hy) = self.hand();
        let mut full = None;
        Self::rings(hx as i32, hy as i32, GATHER_RADIUS, |x, y| {
            if self.grid.in_bounds(x, y) {
                full = self.grid.get(x, y).jar();
            }
            full.is_some()
        });
        let line = match full {
            Some(jar) => {
                let name = match jar {
                    "H2O" => "water",
                    "H2" => "hydrogen",
                    "O2" => "oxygen",
                    "CO2" => "carbon dioxide",
                    "CH4" => "methane",
                    other => other,
                };
                match self.tier {
                    Tier::Sprout => {
                        let mut c = name.chars();
                        let first = c.next().map(|f| f.to_ascii_uppercase());
                        format!("{}{} jar full!", first.unwrap_or(' '), c.as_str())
                    }
                    Tier::Seedling => format!("Your {name} jar is full. Cast first."),
                    Tier::Apprentice => format!(
                        "Your {name} jar is full ({}/{}): cast some before gathering more.",
                        self.jar(jar),
                        Self::jar_cap(jar)
                    ),
                }
            }
            None if self.room() == ROOMS - 1 => self
                .line(
                    "No bubbles here! Go closer!",
                    "No methane in reach. Stand in the bubbles.",
                    "No methane in reach: stand at the marsh, under the rock where the bubbles collect.",
                )
                .to_string(),
            None => self
                .line(
                    "Nothing to grab here!",
                    "Nothing to gather here. Find water or gas.",
                    "Nothing to gather in reach: stand near water or a gas.",
                )
                .to_string(),
        };
        self.say(line);
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

    /// The nearest crystal cell within reach of the player, if any.
    pub fn nearest_crystal(&self) -> Option<(i32, i32)> {
        let (cx, cy) = (
            (self.player.x + PLAYER_W / 2.0) as i32,
            (self.player.y + PLAYER_H / 2.0) as i32,
        );
        let mut found = None;
        Self::rings(cx, cy, CRYSTAL_RANGE, |x, y| {
            let hit = self.grid.get(x, y) == Species::Crystal && self.grid.in_bounds(x, y);
            if hit {
                found = Some((x, y));
            }
            hit
        });
        found
    }

    /// The suit's real devices: a condenser collar that pulls water out of the
    /// air (humidity is the one outside source, like a real atmospheric water
    /// generator), and a wrist electrolyser that splits water from the tank
    /// 2:1 into hydrogen and oxygen, powered by a crystal (fast) or a solar
    /// strip (slow). Every split takes exactly 2 H2O for 2 H2 + O2, so atoms
    /// are conserved; it keeps `SPLIT_RESERVE` casts of water in the tank.
    fn suit_tick(&mut self) {
        let (hx, hy) = self.hand();
        if self.jar("H2O") < H2O_CAP {
            self.condense_timer += 1;
            if self.condense_timer >= CONDENSE_TICKS {
                self.condense_timer = 0;
                // Mist streams in from the air just above the hand.
                self.gain("H2O", 1, "condense", (hx, hy - 10.0));
            }
        } else {
            self.condense_timer = 0;
        }
        let crystal = self.nearest_crystal();
        let charge = if crystal.is_some() {
            CRYSTAL_CHARGE_KJ_PER_TICK
        } else {
            SOLAR_KJ_PER_TICK
        };
        self.crystal_kj = (self.crystal_kj + charge).min(self.crystal_max_kj);

        if crystal.is_some() && !self.met_crystal {
            self.met_crystal = true;
            let line = self.line(
                "A crystal! It powers bubbles!",
                "The crystal powers splitting water. Stand close.",
                "The crystal stores electricity. Your suit uses it to split water into hydrogen and oxygen, fast.",
            );
            self.say(line);
        }
        // Splitting water costs energy: no split without a full split's worth.
        let room = self.jar("H2") + 2 <= Self::jar_cap("H2")
            && self.jar("O2") < Self::jar_cap("O2")
            && self.jar("H2O") >= 2 + SPLIT_RESERVE
            && self.crystal_kj >= self.split_kj;
        if !room {
            self.electro_timer = 0;
            return;
        }
        let powered = crystal;
        let period = if powered.is_some() {
            ELECTRO_TICKS_NEAR
        } else {
            ELECTRO_TICKS_SOLAR
        };
        self.electro_timer += 1;
        if self.electro_timer < period {
            return;
        }
        self.electro_timer = 0;
        // 2H2O -> 2H2 + O2: two water from the tank become exactly two
        // hydrogen and one oxygen.
        self.take("H2O", 2);
        *self.background.entry("H2O".into()).or_insert(0) -= 2;
        let from = powered.map_or((hx, hy), |(x, y)| (x as f32, y as f32));
        self.gain("H2", 2, "split", from);
        self.gain("O2", 1, "split", from);
        // The sourced +571.7 kJ per 2 H2O, from the battery, wherever it is.
        self.crystal_kj -= self.split_kj;
        if let Some((x, y)) = powered {
            self.emit(Fx {
                kind: "arc",
                x: x as f32,
                y: y as f32,
                tx: hx,
                ty: hy,
                dh_kj: self.split_kj,
                outcome: None,
                leftovers: Atoms::new(),
                reaction_id: Some("electrolysis_water".into()),
                id: 0,
                power: 1.0,
            });
        }
    }

    /// Adds to a gauge and tells the browser where it came from, so every
    /// gauge change can show a "+n" and a stream from its source.
    /// `cause` is "condense", "split", "capture" or "gather".
    pub(crate) fn gain(&mut self, jar: &str, n: u32, cause: &'static str, from: (f32, f32)) {
        let before = self.jar(jar);
        self.add(jar, n);
        let got = self.jar(jar) - before;
        if got == 0 {
            return;
        }
        if cause != "gather" {
            *self.background.entry(jar.to_string()).or_insert(0) += got as i32;
        }
        let (hx, hy) = self.hand();
        self.emit(Fx {
            kind: "gain",
            x: from.0,
            y: from.1,
            tx: hx,
            ty: hy,
            dh_kj: 0.0,
            outcome: None,
            leftovers: Atoms::new(),
            reaction_id: Some(format!("{cause}:{jar}")),
            id: 0,
            power: got as f32,
        });
    }

    // ----------------------------------------------------------------- casting

    /// The one cast pipeline. Every input, on every tier, comes back as an
    /// outcome (Full, Partial, Fizzle or Unknown) with at least one FX event;
    /// nothing fails silently or with text alone.
    pub fn cast(&mut self, text: &str) -> CastOutcome {
        let id = self.next_cast_id;
        self.next_cast_id += 1;
        let input: String = text.chars().take(crate::formula::MAX_INPUT_LEN).collect();
        let plan = spell::plan(&self.book, &input, self.tier == Tier::Sprout);
        self.soft_auto_aim(&plan);
        // Turn to face where the spell goes.
        let cx = self.player.x + PLAYER_W / 2.0;
        if (self.aim.0 as f32 - cx).abs() > 1.0 {
            self.player.facing = if (self.aim.0 as f32) < cx { -1 } else { 1 };
        }
        let (hx, hy) = self.hand();

        let (res, spell, equation, reaction_id, balanced) = match &plan {
            Plan::Unknown { suggestion, reason } => {
                self.misses_in_a_row += 1;
                let out = self.unknown_outcome(id, &input, suggestion.clone(), reason);
                self.emit(Fx {
                    kind: "unknown",
                    x: hx,
                    y: hy,
                    tx: hx,
                    ty: hy,
                    dh_kj: 0.0,
                    outcome: Some(Outcome::Unknown),
                    leftovers: Atoms::new(),
                    reaction_id: None,
                    id,
                    power: 0.0,
                });
                self.refresh_version();
                return out;
            }
            Plan::Release { formula, coef } => {
                let r = spell::resolve_release(formula, *coef, &self.jars);
                let spell = match formula.as_str() {
                    "H2O" => "water",
                    "CO2" => "cloud",
                    _ => "gas",
                };
                let eq = if *coef > 1 {
                    format!("{coef}{formula}")
                } else {
                    formula.clone()
                };
                (r, spell, eq, None, true)
            }
            Plan::React {
                reaction_id,
                typed,
                air_o2,
                balanced,
            } => {
                let (mut reaction, known) = self
                    .book
                    .by_id(reaction_id)
                    .cloned()
                    .expect("plans only name reactions from the book");
                // States the player wrote (2H2O(l)) shift ΔH by sourced latent
                // heat; plans only allow states with data.
                reaction.delta_h_kj =
                    spell::delta_h_with_states(&self.book, &reaction, &known, typed)
                        .unwrap_or(reaction.delta_h_kj);
                let r = spell::resolve_reaction(
                    &reaction, &known, typed, *balanced, *air_o2, &self.jars,
                );
                let spell = match reaction_id.as_str() {
                    "water_synthesis" => "boom",
                    "electrolysis_water" => "split",
                    "methane_combustion" => "fireball",
                    _ => "burn",
                };
                (
                    r,
                    spell,
                    typed.to_string(),
                    Some(reaction_id.clone()),
                    *balanced,
                )
            }
        };
        let mut res = res;
        // Splitting water costs energy (+571.7 kJ per 2 H2O). Away from a
        // crystal the battery must hold it, or nothing splits.
        let short_of_energy = spell == "split"
            && res.units > 0
            && self.nearest_crystal().is_none()
            && self.crystal_kj < self.split_kj * res.units as f32;
        if short_of_energy {
            res = spell::Resolution {
                outcome: Some(Outcome::Fizzle),
                missing: vec![ENERGY.to_string()],
                ..Default::default()
            };
        }
        let outcome = res.outcome.unwrap_or(Outcome::Fizzle);

        // Commit the reagents: draw what was used, hand back the leftovers.
        for (f, n) in &res.drawn {
            self.take(f, *n);
        }
        for (f, n) in &res.returned {
            self.add(f, *n);
        }
        for (el, n) in &res.loose {
            *self.loose.entry(el.clone()).or_insert(0) += n;
        }
        self.recombine_loose();
        // Splitting water costs energy: the battery pays where the spell
        // lands (see `impact`), unless it lands on the crystal itself.
        let crystal_cost_kj = if spell == "split" && res.units > 0 {
            self.split_kj * res.units as f32
        } else {
            0.0
        };

        // Unbalanced or empty casts count as misses; a fix is offered to
        // Seedlings at once and to Apprentices after three in a row.
        let mut suggestion = None;
        let mut imbalance = vec![];
        if let Plan::React { typed, .. } = &plan {
            if !balanced {
                imbalance = typed.imbalance_report();
            }
        }
        if outcome == Outcome::Full {
            self.misses_in_a_row = 0;
        } else {
            self.misses_in_a_row += 1;
            // An unbalanced cast gets its fix at once (Sprouts never see one:
            // their spells are balanced for them).
            if !balanced {
                if let Plan::React {
                    typed, reaction_id, ..
                } = &plan
                {
                    suggestion = typed
                        .balanced_version()
                        .map(|b| b.to_string())
                        .or_else(|| self.book.by_id(reaction_id).map(|(_, k)| k.to_string()));
                }
            }
        }

        // A Grimoire page is earned by a balanced spell that ran, never by a
        // lopsided one that only half worked.
        let first = match &reaction_id {
            Some(rid) if balanced && res.units > 0 && !self.discovered.contains(rid) => {
                self.discovered.push(rid.clone());
                true
            }
            _ => false,
        };
        let entry = reaction_id
            .as_ref()
            .and_then(|r| self.book.by_id(r))
            .map(|(r, _)| r.clone());
        if first {
            if let Some(r) = &entry {
                // Short, picture-first words for the young tiers; the full
                // sourced concept for Apprentice (and in every Grimoire).
                let line = match self.tier {
                    Tier::Sprout => r.short_sprout.clone(),
                    Tier::Seedling => r.short_seedling.clone(),
                    Tier::Apprentice => r.concept.clone(),
                };
                self.say(line);
            }
        }
        let show_numbers = self.tier >= Tier::Apprentice;
        let source_hint = res.missing.first().and_then(|m| self.nearest_source(m));
        let consumed = {
            let mut c = spell::stock_atoms(&res.drawn);
            for (el, n) in &res.from_air {
                *c.entry(el.clone()).or_insert(0) += n;
            }
            c
        };
        let (unreacted, canonical, multiple) = match &plan {
            Plan::React { typed, .. } => {
                let (simple, k) = typed.simplest();
                (
                    spell::unreacted(&res.returned, &res.loose, &typed.reactants),
                    balanced.then(|| simple.to_string()),
                    k,
                )
            }
            Plan::Release { formula, .. } => (vec![], Some(formula.clone()), 1),
            Plan::Unknown { .. } => (vec![], None, 1),
        };
        // What reacted, what it spent and made, and where the products went,
        // all from the book's own coefficients (so ΔH always matches).
        let (reacted, ledger) = match &plan {
            Plan::React {
                reaction_id,
                air_o2,
                ..
            } if res.units > 0 => self
                .book
                .by_id(reaction_id)
                .map(|(_, known)| {
                    (
                        Some(spell::reacted_equation(known, res.units)),
                        Some(spell::reacted_ledger(known, res.units, *air_o2)),
                    )
                })
                .unwrap_or((None, None)),
            Plan::Release { formula, .. } if res.units > 0 => {
                (None, Some(format!("-{} {formula}", res.units)))
            }
            _ => (None, None),
        };
        let went = Self::where_products_went(spell, &res.products);
        let mut label = self.label(spell, outcome, &res, &unreacted);
        if let Plan::React { typed, .. } = &plan {
            if !balanced {
                label = self.unbalanced_label(typed, outcome, &res, &unreacted, reacted.as_deref());
            } else if multiple > 1 && outcome == Outcome::Full && self.tier > Tier::Sprout {
                label.push_str(&if multiple == 2 {
                    "  (double the amount, double the ΔH)".to_string()
                } else {
                    format!("  ({multiple}× the amount, {multiple}× the ΔH)")
                });
            }
        }
        // Fractions are cast as whole molecules: a bolt can't carry half a
        // molecule, so the amounts are scaled up and the label says so.
        if let Plan::React { typed, .. } = &plan {
            if typed.per > 1 && self.tier > Tier::Sprout && outcome != Outcome::Fizzle {
                label.push_str(&format!(
                    "  (no half molecules: cast ×{} as {typed})",
                    typed.per
                ));
            }
        }
        let equation_states = match &plan {
            Plan::React { typed, .. } if show_numbers => Some(
                typed.with_states_from(
                    self.book
                        .by_id(reaction_id.as_deref().unwrap_or(""))
                        .map(|(_, k)| k),
                ),
            ),
            _ => None,
        };
        // Water thrown at the ice wall: it melts ice only slowly, and the label
        // says what is faster (the impact shows a frost puff).
        if spell == "water" && outcome != Outcome::Fizzle && self.lands_on_ice((hx, hy)) {
            label = self
                .line(
                    "Too slow! Try boom.",
                    "Water melts ice slowly. Boom heats it fast.",
                    "Water melts ice only slowly; a boom gives lots of heat fast: 2H2 + O2 -> 2H2O.",
                )
                .to_string();
        }
        let note = self.case_note(&input);
        let out = CastOutcome {
            id,
            outcome,
            spell,
            input,
            equation,
            reaction_id: reaction_id.clone(),
            consumed,
            from_air: res.from_air.clone(),
            products: res.products.clone(),
            leftovers: res.leftovers.clone(),
            missing: res.missing.clone(),
            source_x: source_hint.map(|p| p.0),
            source_y: source_hint.map(|p| p.1),
            power: res.power,
            dh_kj: res.dh_kj,
            label,
            suggestion,
            imbalance,
            unreacted,
            reacted,
            ledger,
            went,
            canonical,
            multiple,
            concept: entry.as_ref().map(|r| r.concept.clone()),
            real_world: entry.as_ref().map(|r| r.real_world.clone()),
            delta_h_kj: entry.as_ref().filter(|_| show_numbers).map(|_| res.dh_kj),
            source: entry
                .as_ref()
                .filter(|_| show_numbers)
                .map(|r| r.source.clone()),
            first_discovery: first,
            reason: None,
            equation_states,
            note,
        };

        let shot = Shot {
            id,
            spell,
            outcome,
            products: res.products.clone(),
            power: res.power,
            dh_kj: res.dh_kj,
            leftovers: res.leftovers.clone(),
            reaction_id,
            first,
            crystal_cost_kj,
        };
        if outcome == Outcome::Fizzle {
            // Nothing reacts and nothing in the world changes: one grey puff
            // where the spell would have landed, pointing at a source.
            let (ix, iy) = self.trace((hx, hy), self.aim);
            let (tx, ty) = source_hint.map_or((-1.0, -1.0), |(x, y)| (x as f32, y as f32));
            self.emit(Fx {
                kind: "fizzle",
                x: ix as f32,
                y: iy as f32,
                tx,
                ty,
                dh_kj: 0.0,
                outcome: Some(Outcome::Fizzle),
                leftovers: Atoms::new(),
                reaction_id: shot.reaction_id.clone(),
                id,
                power: 0.0,
            });
        } else {
            self.launch(shot);
        }
        self.refresh_version();
        out
    }

    fn unknown_outcome(
        &self,
        id: u32,
        input: &str,
        suggestion: Option<String>,
        reason: &Rejection,
    ) -> CastOutcome {
        let young = self.tier == Tier::Sprout;
        let mut imbalance = vec![];
        let label = match reason {
            Rejection::ZeroForO => match &suggestion {
                Some(s) => format!("Did you mean {s}? (letter O, not zero)"),
                None => "Use the letter O, not zero".into(),
            },
            Rejection::Empty => self
                .line(
                    "Pick a spell!",
                    "Write a formula first.",
                    "Write a formula first.",
                )
                .into(),
            Rejection::NotAFormula => self
                .line(
                    "That's not a spell!",
                    "That isn't a formula. Try H2O.",
                    "That isn't a formula. Try H2O or 2H2 + O2 -> 2H2O.",
                )
                .into(),
            Rejection::UnknownElement(el) => format!("{el} is not an element"),
            Rejection::NotInWorld { balanced: true } => {
                "Balanced, real chemistry! Not in this trial yet".into()
            }
            Rejection::NotInWorld { balanced: false } => {
                let what = input.trim();
                if young || what.chars().count() > 16 {
                    "Not in this trial yet".into()
                } else {
                    format!("{what} is not in this trial yet")
                }
            }
            Rejection::Unbalanced { counts, pair } => {
                imbalance = counts.clone();
                let counts = counts.join("; ");
                match pair {
                    Some(p) if young => format!("It comes in pairs: {p}"),
                    Some(p) => {
                        let el = p.trim_end_matches('2');
                        if counts.is_empty() {
                            format!("{el} gas comes in pairs: {p}")
                        } else {
                            format!("{el} gas comes in pairs: {p}. Then {counts}")
                        }
                    }
                    None if young => "Not balanced".into(),
                    None => format!("Not balanced: {counts}"),
                }
            }
            Rejection::WrongState(term) => {
                format!("{term}: that state is not in this trial yet")
            }
        };
        CastOutcome {
            id,
            outcome: Outcome::Unknown,
            spell: "unknown",
            input: input.to_string(),
            equation: input.trim().to_string(),
            reaction_id: None,
            consumed: Atoms::new(),
            from_air: Atoms::new(),
            products: Stock::new(),
            leftovers: Atoms::new(),
            missing: vec![],
            source_x: None,
            source_y: None,
            power: 0.0,
            dh_kj: 0.0,
            label,
            suggestion,
            imbalance,
            unreacted: vec![],
            reacted: None,
            ledger: None,
            went: None,
            canonical: None,
            multiple: 1,
            concept: None,
            real_world: None,
            delta_h_kj: None,
            source: None,
            first_discovery: false,
            reason: Some(reason.code()),
            equation_states: None,
            note: None,
        }
    }

    /// Apprentice: lower-case typing is forgiven, but says why it matters.
    fn case_note(&self, input: &str) -> Option<String> {
        (self.tier == Tier::Apprentice && spell::forgiven_case(input)).then(|| {
            "Read as capitals. Element symbols are case-sensitive: CO is carbon monoxide, Co is cobalt.".to_string()
        })
    }

    fn label(
        &self,
        spell: &str,
        outcome: Outcome,
        res: &Resolution,
        unreacted: &[String],
    ) -> String {
        let young = self.tier == Tier::Sprout;
        if outcome == Outcome::Fizzle {
            // Always name what ran out, and how to get more.
            let what = res.missing.first().map(String::as_str).unwrap_or("");
            return match (what, young) {
                ("H2O", true) => "Out of water! Wait a moment.".into(),
                ("H2" | "O2", true) => "Out of gas! Wait a moment.".into(),
                (ENERGY, true) => "Out of power! Go to the crystal.".into(),
                ("CH4", true) => "Out of marsh gas! Grab bubbles.".into(),
                ("CO2", true) => "Out of cloud! Go near fire.".into(),
                (_, true) => "Out of stuff! Wait a moment.".into(),
                ("H2O", false) => "Out of H2O: hold G near water.".into(),
                ("H2" | "O2", false) => {
                    format!("Out of {what}: wait, or stand by the crystal.")
                }
                (ENERGY, false) => "Out of ⚡: stand by the crystal to charge.".into(),
                ("CH4", false) => "Out of CH4: hold G in marsh bubbles.".into(),
                ("CO2", false) => "Out of CO2: stand by a flame to catch it.".into(),
                ("", false) => "Nothing to cast: your suit is empty.".into(),
                (_, false) => format!("Out of {what}: none in your suit."),
            };
        }
        let main = match (spell, young) {
            ("water", true) => "Splash!",
            ("water", false) => "Water stream",
            ("boom", true) => "Boom! Water!",
            ("boom", false) => "Boom! Hydrogen burns into water",
            ("split", true) => "Bubbles! Water split!",
            ("split", false) => "Water split: 2 hydrogen for each oxygen",
            ("cloud", _) => "Carbon dioxide cloud",
            ("fireball", _) => "Fireball! Methane burns",
            ("burn", _) => "Carbon burns",
            _ => "Gas released",
        };
        let mut text = main.to_string();
        if !res.from_air.is_empty() {
            text.push_str(if young {
                " (with air)"
            } else {
                " (oxygen from the air: smaller flame)"
            });
        }
        // Sprout sees leftovers as orbs drifting home, not as words.
        if !unreacted.is_empty() && !young {
            text.push_str(&format!(". {} left over", unreacted.join(", ")));
        }
        text
    }

    /// One rule for every unbalanced spell, in short lines (no flavour
    /// text): what is off, what actually reacted (with the ΔH of exactly
    /// that), and what was left over. A missing reagent still says what
    /// ran out and how to get more.
    fn unbalanced_label(
        &self,
        typed: &crate::formula::Equation,
        outcome: Outcome,
        res: &Resolution,
        unreacted: &[String],
        reacted: Option<&str>,
    ) -> String {
        if self.tier == Tier::Sprout {
            return match outcome {
                Outcome::Fizzle if !res.missing.is_empty() => self.label("", outcome, res, &[]),
                Outcome::Fizzle => "Not balanced! Try again.".into(),
                _ => "Weak pop! Some left over.".into(),
            };
        }
        // Name the imbalance per element: "O: 2 left, 1 right".
        let left = crate::formula::side_atoms(&typed.reactants);
        let right = crate::formula::side_atoms(&typed.products);
        let mut els: Vec<&String> = left.keys().chain(right.keys()).collect();
        els.sort();
        els.dedup();
        let parts: Vec<String> = els
            .into_iter()
            .filter_map(|el| {
                let (l, r) = (
                    left.get(el).copied().unwrap_or(0),
                    right.get(el).copied().unwrap_or(0),
                );
                (l != r).then(|| format!("{el}: {l} left, {r} right"))
            })
            .collect();
        let mut lines = vec![if parts.is_empty() {
            "Not balanced.".to_string()
        } else {
            format!("Not balanced ({}).", parts.join("; "))
        }];
        if outcome == Outcome::Fizzle {
            lines.push(if res.missing.is_empty() {
                "Nothing paired up. Fix it and cast again.".into()
            } else {
                self.label("", outcome, res, &[])
            });
            return lines.join("\n");
        }
        if let Some(eq) = reacted {
            lines.push(if self.tier >= Tier::Apprentice {
                let sign = if res.dh_kj < 0.0 { "-" } else { "+" };
                format!("What reacted: {eq}, ΔH {sign}{:.1} kJ", res.dh_kj.abs())
            } else {
                format!("What reacted: {eq}")
            });
        }
        if !unreacted.is_empty() {
            lines.push(format!("{} left over", unreacted.join(", ")));
        }
        lines.join("\n")
    }

    /// Where a cast's products ended up, in words: "2 H2O released as steam".
    fn where_products_went(spell: &str, products: &Stock) -> Option<String> {
        let list = |only: &[&str]| -> Vec<String> {
            products
                .iter()
                .filter(|(f, n)| **n > 0 && (only.is_empty() || only.contains(&f.as_str())))
                .map(|(f, n)| format!("{n} {f}"))
                .collect()
        };
        let parts: Vec<String> = match spell {
            "water" | "cloud" => vec![format!("{} poured out", list(&[]).join(" + "))],
            "split" => vec![format!(
                "{} released as bubbles",
                list(&["H2", "O2"]).join(" + ")
            )],
            "gas" => vec![format!("{} released", list(&[]).join(" + "))],
            _ => {
                let mut v = vec![];
                let water = list(&["H2O"]);
                if !water.is_empty() {
                    v.push(format!("{} released as steam", water.join("")));
                }
                let co2 = list(&["CO2"]);
                if !co2.is_empty() {
                    v.push(format!("{} into the air", co2.join("")));
                }
                v
            }
        };
        let parts: Vec<String> = parts
            .into_iter()
            .filter(|p| p.chars().next().is_some_and(|c| c.is_ascii_digit()))
            .collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// Joins loose leftover atoms back into whole molecules the suit stores.
    fn recombine_loose(&mut self) {
        for (el, jar) in [("O", "O2"), ("H", "H2")] {
            while self.loose.get(el).copied().unwrap_or(0) >= 2 {
                *self.loose.get_mut(el).expect("checked") -= 2;
                self.add(jar, 1);
            }
        }
        self.loose.retain(|_, n| *n > 0);
    }

    /// The nearest place in the world to get more of a reagent.
    fn nearest_source(&self, formula: &str) -> Option<(i32, i32)> {
        let wanted: &[Species] = match formula {
            "H2O" => &[Species::Water, Species::Ice],
            "H2" | "O2" => &[Species::Crystal],
            "CH4" => &[Species::CH4],
            "CO2" => &[Species::CO2, Species::Flame],
            "C" => &[Species::Wood],
            _ => return None,
        };
        let (hx, hy) = self.hand();
        let mut best: Option<(f32, (i32, i32))> = None;
        for y in 0..HEIGHT as i32 {
            for x in 0..WORLD_W as i32 {
                if wanted.contains(&self.grid.get(x, y)) {
                    let d = (x as f32 - hx).powi(2) + (y as f32 - hy).powi(2);
                    if best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, (x, y)));
                    }
                }
            }
        }
        best.map(|(_, p)| p)
    }

    /// Where a spell from `from` toward `to` would land: the first cell that is
    /// solid, liquid or burning (so water lands *in* the fire), or `to` itself.
    pub fn trace(&self, from: (f32, f32), to: (i32, i32)) -> (i32, i32) {
        let (dx, dy) = (to.0 as f32 - from.0, to.1 as f32 - from.1);
        let len = (dx * dx + dy * dy).sqrt().min(BOLT_RANGE);
        let steps = len.ceil().max(1.0) as i32;
        let (sx, sy) = if len > 0.0 {
            let d = (dx * dx + dy * dy).sqrt();
            (dx / d, dy / d)
        } else {
            (0.0, 0.0)
        };
        let mut last = (from.0.floor() as i32, from.1.floor() as i32);
        // A spell cast while swimming passes out of the water it started in.
        let mut leaving_liquid = self.grid.get(last.0, last.1).phase() == Phase::Liquid;
        for i in 1..=steps {
            let (x, y) = (
                (from.0 + sx * i as f32).floor() as i32,
                (from.1 + sy * i as f32).floor() as i32,
            );
            last = (x, y);
            let s = self.grid.get(x, y);
            if leaving_liquid && s.phase() == Phase::Liquid {
                continue;
            }
            leaving_liquid = false;
            if Self::stops_bolt(s) {
                return last;
            }
        }
        last
    }

    /// Whether a spell from `from` toward the aim lands on (or beside) ice.
    fn lands_on_ice(&self, from: (f32, f32)) -> bool {
        let at = self.trace(from, self.aim);
        Self::ice_near(&self.grid, at)
    }

    fn ice_near(grid: &Grid, at: (i32, i32)) -> bool {
        (-3..=3).any(|dy| (-3..=3).any(|dx| grid.get(at.0 + dx, at.1 + dy) == Species::Ice))
    }

    /// Spells stop at anything that isn't a gas, except the crystal
    /// battery: a thin column the player often stands against, which a
    /// spell passes through instead of fizzling into it.
    fn stops_bolt(s: Species) -> bool {
        !s.is_gas() && s != Species::Crystal
    }

    /// Sprout and Seedling: if the player has not aimed in the last 3 s, the
    /// spell goes to the obvious target (fire for water, ice or the torch for
    /// heat) when one is in reach. At every tier, a player who has never
    /// pointed anywhere (keyboard only) gets the same help, so a typed spell
    /// flies at the current goal instead of a spot just ahead.
    fn soft_auto_aim(&mut self, plan: &Plan) {
        let never_aimed = self.aim_tick == 0;
        if never_aimed {
            // Before the player has pointed anywhere, spells go a little way
            // ahead rather than landing at their feet.
            let (hx, hy) = self.hand();
            self.aim = (
                (hx + self.player.facing as f32 * 48.0) as i32,
                (hy as i32).min(HEIGHT as i32 - 1),
            );
        }
        if !never_aimed && !self.aim_on_goal_marker() {
            if self.tier > Tier::Seedling {
                return;
            }
            if !never_aimed && self.ticks.saturating_sub(self.aim_tick) <= AUTO_AIM_TICKS {
                return;
            }
        }
        let douse =
            matches!(plan, Plan::Release { formula, .. } if formula == "H2O" || formula == "CO2");
        let split =
            matches!(plan, Plan::React { reaction_id, .. } if reaction_id == "electrolysis_water");
        let heat = matches!(plan, Plan::React { .. }) && !split;
        if !douse && !heat && !split {
            return;
        }
        let (hx, hy) = self.hand();
        let in_reach =
            |p: (i32, i32)| ((p.0 as f32 - hx).powi(2) + (p.1 as f32 - hy).powi(2)).sqrt() <= REACH;
        let mut found = None;
        if douse || heat {
            Self::rings(hx as i32, hy as i32, REACH as i32, |x, y| {
                let g = &self.grid;
                let s = g.get(x, y);
                let hit = g.in_bounds(x, y)
                    && if douse {
                        s == Species::Flame
                            || (s == Species::Wood && g.temp_at(x, y) >= WOOD_IGNITION_C)
                    } else {
                        self.heat_target(x, y)
                    };
                if hit {
                    found = Some((x, y));
                }
                hit
            });
        }
        // Nothing burning: water goes to the crystal pool, where it splits
        // into bubbles. The split spell goes under the stone overhang.
        let pool = (rooms::POOL.centre().0, rooms::POOL.y0);
        let trap = rooms::TRAP.centre();
        if found.is_none() && plan_is_water(plan) && in_reach(pool) {
            found = Some(pool);
        }
        if split && in_reach(trap) {
            found = Some(trap);
        }
        // Nothing to act on in reach: fly at the room's goal anyway, so a
        // tapped card always goes somewhere that teaches something (water
        // on the ice says "too slow, try boom").
        if found.is_none() {
            found = self.goal_in_range();
        }
        // Trapped hydrogen hugs the roof; aim into the middle of the pocket so
        // the bolt clears the overhang's lip and the fireball reaches the gas.
        if found.is_some_and(|(x, y)| rooms::TRAP.contains(x, y)) {
            found = Some(trap);
        }
        if let Some(p) = found {
            self.aim = p;
        }
    }

    /// Whether the aim rests on the marker of a goal not yet done.
    fn aim_on_goal_marker(&self) -> bool {
        let (ax, ay) = (self.aim.0 as f32, self.aim.1 as f32);
        self.goals().iter().any(|g| {
            !g.done && ((g.x as f32 - ax).powi(2) + (g.y as f32 - ay).powi(2)).sqrt() <= MARKER_SNAP
        })
    }

    /// Sends a resolved spell flying from the hand toward the aim.
    fn launch(&mut self, shot: Shot) {
        let (hx, hy) = self.hand();
        let lands = self.trace((hx, hy), self.aim);
        let fx = Fx {
            kind: "charge",
            x: hx,
            y: hy,
            tx: lands.0 as f32,
            ty: lands.1 as f32,
            dh_kj: shot.dh_kj,
            outcome: Some(shot.outcome),
            leftovers: Atoms::new(),
            reaction_id: shot.reaction_id.clone(),
            id: shot.id,
            power: shot.power,
        };
        self.emit(fx.clone());
        self.emit(Fx { kind: "bolt", ..fx });
        let (dx, dy) = (self.aim.0 as f32 + 0.5 - hx, self.aim.1 as f32 + 0.5 - hy);
        let len = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = if len > 0.01 {
            (dx / len, dy / len)
        } else {
            (self.player.facing as f32, 0.0)
        };
        let start = self.grid.get(hx.floor() as i32, hy.floor() as i32);
        self.bolts.push(Bolt {
            x: hx,
            y: hy,
            ux,
            uy,
            left: len.min(BOLT_RANGE),
            leaving_liquid: start.phase() == Phase::Liquid,
            shot,
        });
    }

    /// Moves every spell in flight about 6 cells, one cell at a time, and
    /// lands it at the first solid, liquid or burning cell, or at its aim.
    fn advance_bolts(&mut self) {
        let mut bolts = std::mem::take(&mut self.bolts);
        bolts.retain_mut(|b| {
            for _ in 0..BOLT_SPEED {
                let step = b.left.min(1.0);
                b.x += b.ux * step;
                b.y += b.uy * step;
                b.left -= step;
                let (cx, cy) = (b.x.floor() as i32, b.y.floor() as i32);
                let s = self.grid.get(cx, cy);
                let in_start_liquid = b.leaving_liquid && s.phase() == Phase::Liquid;
                if !in_start_liquid {
                    b.leaving_liquid = false;
                }
                let blocked = !in_start_liquid && Self::stops_bolt(s);
                if blocked || b.left <= 0.0 {
                    let at = (
                        cx.clamp(0, WORLD_W as i32 - 1),
                        cy.clamp(0, HEIGHT as i32 - 1),
                    );
                    let shot = b.shot.clone();
                    self.impact(&shot, at);
                    return false;
                }
            }
            true
        });
        self.bolts = bolts;
    }

    /// Spells still in flight.
    pub fn bolts_in_flight(&self) -> usize {
        self.bolts.len()
    }

    /// The spell lands: the reaction changes the world around `at`.
    fn impact(&mut self, shot: &Shot, at: (i32, i32)) {
        let (hx, hy) = self.hand();
        let base = Fx {
            kind: "impact",
            x: at.0 as f32,
            y: at.1 as f32,
            tx: hx,
            ty: hy,
            dh_kj: shot.dh_kj,
            outcome: Some(shot.outcome),
            leftovers: shot.leftovers.clone(),
            reaction_id: shot.reaction_id.clone(),
            id: shot.id,
            power: shot.power,
        };
        self.emit(base.clone());
        let n = |f: &str| shot.products.get(f).copied().unwrap_or(0);
        let mut extra: Vec<&'static str> = vec![];
        match shot.spell {
            "water" => {
                let mut doused = self.cool_area(at, WATER_COOL_RADIUS);
                doused += self.soak(at, n("H2O") * SOAK_CELLS_PER_MOL);
                self.note_douse(at, false);
                self.douse_vents(at);
                let cells = (n("H2O") * WATER_CELLS_PER_MOL).min(4 * WATER_CELLS_PER_MOL);
                self.spawn_blob_at(at, Species::Water, cells, AMBIENT_C);
                extra.push("splash");
                if Self::ice_near(&self.grid, at) {
                    extra.push("frost");
                }
                if doused > 0 {
                    extra.push("hiss");
                }
                if self.pool_electrolysis(at, n("H2O")) > 0 {
                    extra.push("bubble");
                }
            }
            "cloud" => {
                // Like a CO2 extinguisher: the gas comes out as cold as dry
                // ice (-78.5 C, CRC) and pushes the oxygen away, so it both
                // cools and smothers what it lands on.
                self.note_douse(at, true);
                self.smother(at);
                let cooled = self.soak(at, n("CO2") * CO2_SOAK_CELLS_PER_MOL);
                self.spawn_blob_at(at, Species::CO2, n("CO2") * CO2_CELLS_PER_MOL, DRY_ICE_C);
                if cooled > 0 {
                    extra.push("hiss");
                }
                extra.push("bubble");
            }
            "gas" => {
                for f in ["O2", "CH4", "H2"] {
                    if let Some(s) = Species::from_jar(f) {
                        self.spawn_blob_at(at, s, n(f) * 4 * GAS_CELLS_PER_MOL, AMBIENT_C);
                    }
                }
                extra.push("bubble");
            }
            "split" => {
                // The crystal pays for the split, unless the spell came down
                // on the crystal itself (its energy would go straight back).
                let on_crystal = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| self.grid.get(at.0 + dx, at.1 + dy) == Species::Crystal)
                });
                if !on_crystal {
                    self.crystal_kj = (self.crystal_kj - shot.crystal_cost_kj).max(0.0);
                }
                if at.0 as usize / ROOM_W == 2 && n("H2") > 0 {
                    self.rooms.gas_made = true;
                }
                // Exactly two hydrogen cells for each oxygen cell, like the equation.
                self.spawn_blob_at(at, Species::H2, n("H2") * GAS_CELLS_PER_MOL, 15.0);
                self.spawn_blob_at(at, Species::O2, n("O2") * GAS_CELLS_PER_MOL, 15.0);
                extra.push("bubble");
            }
            _ => {
                // Combustion: heat scaled from the sourced enthalpy.
                let radius = Self::burst_radius(shot.dh_kj);
                let peak = match shot.spell {
                    "fireball" => 1200.0,
                    "burn" => 700.0,
                    _ => 900.0,
                };
                // Ice soaks up the blast's heat (fusion, 6.01 kJ/mol, CRC), so it melts
                // a little past the fireball's edge.
                let mut melted = self.melt_ice(at, radius + ICE_MELT_EXTRA);
                // The rest of the heat spreads through the ice it touches:
                // melting takes 6.01 kJ per mole, whatever the shape.
                let budget = shot.dh_kj.abs() / self.fusion_kj / ICE_MOL_PER_CELL;
                let budget = (budget as u32).saturating_sub(melted);
                melted += self.melt_ice_through(at, radius + ICE_MELT_EXTRA + 2, budget);
                if melted > 0 {
                    self.ice_progress_line(shot.spell);
                }
                self.rooms.last_heat = Some((self.ticks, at.0, at.1));
                if melted > 0 || rooms::ICE.grow(radius + 4).contains(at.0, at.1) {
                    self.rooms.ice_heated = true;
                }
                self.heat_burst(at, radius, peak * (0.5 + 0.5 * shot.power));
                self.spawn_blob_at(at, Species::Steam, n("H2O") * 4, 300.0);
                self.spawn_blob_at(at, Species::CO2, n("CO2") * 4, 300.0);
                if melted > 0 {
                    extra.push("melt");
                }
            }
        }
        if shot.first {
            extra.push("discover");
        }
        for kind in extra {
            self.emit(Fx {
                kind,
                ..base.clone()
            });
        }
    }

    /// Blast radius in cells from |ΔH| in kJ: about 9 cells for 2H2 + O2.
    fn burst_radius(dh_kj: f32) -> i32 {
        (3.0 + dh_kj.abs().sqrt() / 3.5).clamp(3.0, 14.0) as i32
    }

    /// Water lands: flames within `r` go out and hot things cool. Returns how
    /// many flame cells were put out.
    fn cool_area(&mut self, at: (i32, i32), r: i32) -> u32 {
        let mut doused = 0;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let (x, y) = (at.0 + dx, at.1 + dy);
                if !self.grid.in_bounds(x, y) {
                    continue;
                }
                let i = self.grid.idx(x as usize, y as usize);
                match self.grid.cells[i] {
                    Species::Flame => {
                        self.grid.set(x, y, Species::Steam, 100.0);
                        doused += 1;
                    }
                    Species::Wood => self.grid.temp[i] = self.grid.temp[i].min(40.0),
                    _ if self.grid.temp[i] > 100.0 => self.grid.temp[i] = 100.0,
                    _ => {}
                }
            }
        }
        doused
    }

    /// Heat conducted through ice: starting from ice within `r` of `at`, the
    /// nearest connected ice cells melt first, up to `budget` cells.
    fn melt_ice_through(&mut self, at: (i32, i32), r: i32, budget: u32) -> u32 {
        if budget == 0 {
            return 0;
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut queue = std::collections::VecDeque::new();
        // Seeds: ice next to the blast (and next to what it just melted).
        let mut seeds = vec![];
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (at.0 + dx, at.1 + dy);
                if dx * dx + dy * dy <= r * r
                    && self.grid.in_bounds(x, y)
                    && self.grid.get(x, y) == Species::Ice
                {
                    seeds.push((dx * dx + dy * dy, x, y));
                }
            }
        }
        seeds.sort_unstable();
        for (_, x, y) in seeds {
            if seen.insert((x, y)) {
                queue.push_back((x, y));
            }
        }
        let mut melted = 0;
        while let Some((x, y)) = queue.pop_front() {
            if melted >= budget {
                break;
            }
            if self.grid.get(x, y) != Species::Ice {
                continue;
            }
            self.grid.set(x, y, Species::Water, 1.0);
            melted += 1;
            for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if (nx - at.0).abs() <= ICE_SPREAD_REACH
                    && (ny - at.1).abs() <= ICE_SPREAD_REACH
                    && self.grid.in_bounds(nx, ny)
                    && self.grid.get(nx, ny) == Species::Ice
                    && seen.insert((nx, ny))
                {
                    queue.push_back((nx, ny));
                }
            }
        }
        melted
    }

    /// Heat melts ice within `r` straight into water. Returns cells melted.
    fn melt_ice(&mut self, at: (i32, i32), r: i32) -> u32 {
        let mut melted = 0;
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (at.0 + dx, at.1 + dy);
                if dx * dx + dy * dy <= r * r
                    && self.grid.get(x, y) == Species::Ice
                    && self.grid.in_bounds(x, y)
                {
                    self.grid.set(x, y, Species::Water, 30.0);
                    melted += 1;
                }
            }
        }
        melted
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
        let cap = Self::jar_cap(k);
        let v = self.jars.entry(k.to_string()).or_insert(0);
        *v = (*v + n).min(cap);
    }

    /// Puts `n` cells of `s` into the air around `at`, nearest first.
    fn spawn_blob_at(&mut self, at: (i32, i32), s: Species, n: u32, t: f32) -> u32 {
        let (ax, ay) = at;
        let mut placed = 0;
        if n == 0 {
            return 0;
        }
        for r in 0..20i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if placed >= n {
                        return placed;
                    }
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (ax + dx, ay + dy);
                    // Only air makes way: a second blob never overwrites
                    // the first (split's O2 must not replace its H2).
                    if self.grid.in_bounds(x, y) && self.grid.get(x, y) == Species::Air {
                        self.grid.set(x, y, s, t);
                        placed += 1;
                    }
                }
            }
        }
        placed
    }

    /// The visible flash and heat of a spell going off.
    fn heat_burst(&mut self, at: (i32, i32), radius: i32, peak: f32) {
        let (ax, ay) = at;
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
                // Fuel gas anywhere inside the fireball catches: the flame front
                // reaches it even where the average heat would be too low.
                if matches!(self.grid.cells[i], Species::H2 | Species::CH4) {
                    self.grid.temp[i] = self.grid.temp[i].max(CH4_IGNITION_C + 100.0);
                }
                if self.grid.cells[i] == Species::Air
                    && d2 >= (radius - 1) * (radius - 1)
                    && self.grid.chance(0.5)
                {
                    self.grid.spawn_flame(x, y, 5, false, peak);
                }
            }
        }
    }

    // ------------------------------------------------------------------- state

    /// A cheap fingerprint of everything the HUD shows; the version bumps
    /// when it changes so the browser only rebuilds JSON when it must.
    fn refresh_version(&mut self) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.jars.hash(&mut h);
        ((self.crystal_kj / self.crystal_max_kj * 100.0) as u32).hash(&mut h);
        self.objectives
            .iter()
            .filter(|o| o.done)
            .count()
            .hash(&mut h);
        self.messages.len().hash(&mut h);
        self.discovered.len().hash(&mut h);
        let (ax, ay) = self.aim;
        (self.grid.get(ax, ay) as u8).hash(&mut h);
        (self.grid.temp_at(ax, ay) as i32).hash(&mut h);
        self.nearest_crystal().is_some().hash(&mut h);
        self.rooms.gates_open.hash(&mut h);
        self.room().hash(&mut h);
        // Fire meters flicker every tick; the browser polls `goals()` for them.
        (self.rooms.torch.lit, self.rooms.torch.carried).hash(&mut h);
        let sig = h.finish();
        if sig != self.state_sig {
            self.state_sig = sig;
            self.state_version += 1;
        }
    }

    pub fn state_version(&self) -> u64 {
        self.state_version
    }

    pub fn state(&mut self) -> State {
        let (ax, ay) = self.aim;
        let s = self.grid.get(ax, ay);
        State {
            tier: self.tier,
            jars: self.jars.clone(),
            jar_caps: self
                .jars
                .keys()
                .map(|k| (k.clone(), Self::jar_cap(k)))
                .collect(),
            crystal_kj: self.crystal_kj,
            crystal_max_kj: self.crystal_max_kj,
            near_crystal: self.nearest_crystal().is_some(),
            charge_source: if self.nearest_crystal().is_some() {
                "crystal"
            } else {
                "solar"
            },
            background: std::mem::take(&mut self.background),
            state_version: self.state_version,
            objectives: self.objectives.clone(),
            messages: std::mem::take(&mut self.messages),
            player_x: self.player.x,
            player_y: self.player.y,
            discovered: self.discovered.clone(),
            trial_complete: self.rooms.brazier_lit,
            aim_temp_c: self.grid.temp_at(ax, ay),
            aim_substance: s.props().name.to_string(),
            aim_x: ax,
            aim_y: ay,
            view_x: self.view_x.round() as i32,
            room: self.room() as u8,
            gates_open: self.rooms.gates_open,
            goals: self.goals(),
            torch: self.torch_view(),
        }
    }

    /// Spots worth a label under the heat lens, in world cells: the hottest
    /// cell of up to `max` well-separated hot areas (above 60 °C) in the
    /// view starting at `view_x`, hottest first, then the coldest cell if
    /// anything is below freezing. Display only.
    pub fn hotspots(&self, view_x: i32, max: usize) -> Vec<(i32, i32, f32)> {
        const BLOCK: i32 = 32;
        const APART: i32 = 48;
        let x0 = view_x.clamp(0, (WORLD_W - VIEW_W) as i32);
        let mut hot: Vec<(i32, i32, f32)> = vec![];
        let mut cold: Option<(i32, i32, f32)> = None;
        for by in (0..HEIGHT as i32).step_by(BLOCK as usize) {
            for bx in (x0..x0 + VIEW_W as i32).step_by(BLOCK as usize) {
                let mut best: Option<(i32, i32, f32)> = None;
                for y in by..(by + BLOCK).min(HEIGHT as i32) {
                    for x in bx..(bx + BLOCK).min(x0 + VIEW_W as i32) {
                        let t = self.grid.temp_at(x, y);
                        if best.is_none_or(|b| t > b.2) {
                            best = Some((x, y, t));
                        }
                        if t < -1.0 && cold.is_none_or(|c| t < c.2) {
                            cold = Some((x, y, t));
                        }
                    }
                }
                if let Some(b) = best.filter(|b| b.2 > 60.0) {
                    hot.push(b);
                }
            }
        }
        hot.sort_by(|a, b| b.2.total_cmp(&a.2));
        let mut out: Vec<(i32, i32, f32)> = vec![];
        for h in hot {
            if out.len() >= max {
                break;
            }
            if out
                .iter()
                .all(|o| (o.0 - h.0).abs() >= APART || (o.1 - h.1).abs() >= APART)
            {
                out.push(h);
            }
        }
        out.extend(cold);
        out
    }

    /// True once the brazier is lit and the last door has opened.
    pub fn trial_complete(&self) -> bool {
        self.rooms.brazier_lit
    }
}

fn plan_is_water(plan: &Plan) -> bool {
    matches!(plan, Plan::Release { formula, .. } if formula == "H2O")
}

/// A resolved spell on its way to where it lands.
#[derive(Debug, Clone)]
struct Shot {
    id: u32,
    spell: &'static str,
    outcome: Outcome,
    products: Stock,
    power: f32,
    dh_kj: f32,
    leftovers: Atoms,
    reaction_id: Option<String>,
    first: bool,
    /// What the crystal pays for a split, charged when it lands.
    crystal_cost_kj: f32,
}

/// How far a spell flies before it lands on its own, in cells...
const BOLT_RANGE: f32 = 200.0;
/// ...and how fast, in cells per tick.
const BOLT_SPEED: u32 = 6;

/// A spell in flight. Deterministic: it moves a fixed distance each tick.
#[derive(Debug, Clone)]
struct Bolt {
    x: f32,
    y: f32,
    ux: f32,
    uy: f32,
    /// Cells left before it reaches the aim point.
    left: f32,
    /// Cast while swimming: passes out of the water it started in.
    leaving_liquid: bool,
    shot: Shot,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(tier: Tier) -> Game {
        let mut g = Game::new(tier, 42);
        g.set_aim(20, 140, true);
        g
    }

    /// Lets any spell in flight land and the world react.
    fn settle(g: &mut Game, ticks: u32) {
        for _ in 0..ticks {
            g.tick(0);
        }
    }

    fn suit_atoms(g: &Game) -> Atoms {
        let mut a = spell::stock_atoms(&g.jars);
        for (el, n) in &g.loose {
            *a.entry(el.clone()).or_insert(0) += n;
        }
        a
    }

    fn sum(a: &Atoms, b: &Atoms) -> Atoms {
        let mut out = a.clone();
        for (el, n) in b {
            *out.entry(el.clone()).or_insert(0) += n;
        }
        out.retain(|_, n| *n > 0);
        out
    }

    const FUZZ: [&str; 30] = [
        "water",
        "fire",
        "H20",
        "h20",
        "2H2+O2->H2O",
        "H2 + O2 -> H2O",
        "xyz",
        "",
        "   ",
        "2H2 + O2 -> 2H2O",
        "boom",
        "pop",
        "split",
        "bubbles",
        "hydrogen",
        "oxygen",
        "cloud",
        "smother",
        "co2",
        "H2O",
        "2H2O -> 2H2 + O2",
        "H2O -> H2 + O2",
        "CH4 + 2O2 -> CO2 + 2H2O",
        "C + O2 -> CO2",
        "NaCl",
        "->",
        "2H2 + O2",
        "9H2O",
        "((H2",
        "H2",
    ];

    #[test]
    fn every_cast_yields_an_outcome_and_an_fx_event() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = game(tier);
            for text in FUZZ {
                g.take_fx();
                let out = g.cast(text);
                assert!(!out.label.is_empty(), "{text:?} has no label");
                let fx = g.take_fx();
                assert!(!fx.is_empty(), "{text:?} produced no FX");
                assert!(fx.iter().all(|f| f.id == out.id || f.id == 0));
                settle(&mut g, 2);
            }
        }
    }

    #[test]
    fn atoms_are_conserved_in_every_outcome() {
        let mut g = game(Tier::Apprentice);
        g.jars.insert("CH4".into(), 2);
        for round in 0..3 {
            for text in FUZZ {
                let before = suit_atoms(&g);
                let out = g.cast(text);
                assert_eq!(
                    out.consumed,
                    sum(&spell::stock_atoms(&out.products), &out.leftovers),
                    "{text:?} (round {round}) broke conservation: {out:?}"
                );
                // The suit loses exactly what went into products, minus what the air gave.
                let after = suit_atoms(&g);
                let lost: Atoms = before
                    .iter()
                    .map(|(el, n)| (el.clone(), n - after.get(el).copied().unwrap_or(0)))
                    .filter(|(_, n)| *n > 0)
                    .collect();
                let mut expected = spell::stock_atoms(&out.products);
                for (el, n) in &out.from_air {
                    *expected.get_mut(el).expect("air atoms end up in products") -= n;
                }
                expected.retain(|_, n| *n > 0);
                assert_eq!(lost, expected, "{text:?} suit accounting");
            }
        }
    }

    #[test]
    fn unbalanced_hydrogen_is_partial_names_the_imbalance_and_finds_no_page() {
        let mut g = game(Tier::Apprentice);
        let before = g.discovered.clone();
        let out = g.cast("H2 + O2 -> H2O");
        assert_eq!(out.outcome, Outcome::Partial);
        assert!(!out.first_discovery, "a lopsided spell earns no page");
        assert_eq!(g.discovered, before);
        // Redesigned in playtest round 2: short lines, no flavour text.
        assert_eq!(
            out.label,
            "Not balanced (O: 2 left, 1 right).\nWhat reacted: H2 + 1/2 O2 -> H2O, ΔH -241.8 kJ\n1/2 O2 left over"
        );
        assert!(!out.label.contains("+1 O"), "{}", out.label);
        assert!(!out.label.contains("Boom"), "{}", out.label);
        // The smaller limiting-reagent effect still happens, atoms conserved.
        assert_eq!(out.products, [("H2O".to_string(), 1)].into_iter().collect());
        assert_eq!(out.leftovers, [("O".to_string(), 1)].into_iter().collect());
        assert_eq!(out.unreacted, ["1/2 O2"]);
        assert!(out.power <= 0.5, "a sputter, not a boom: {}", out.power);
        assert_eq!(out.canonical, None, "not kept as a hotkey");
        assert_eq!(g.loose.get("O"), Some(&1), "the stray O waits in the suit");
        // Balancing it earns the page.
        let out = g.cast("2H2 + O2 -> 2H2O");
        assert_eq!(out.outcome, Outcome::Full);
        assert!(out.first_discovery);
        assert_eq!(g.discovered, ["water_synthesis"]);
    }

    /// The ΔH of an equation like "H2 + 1/2 O2 -> H2O", from the book's
    /// sourced value scaled by the written coefficients.
    fn dh_of_written(g: &Game, rid: &str, eq: &str) -> f32 {
        let (r, known) = g.book.by_id(rid).unwrap();
        let first = &known.products[0];
        let written = eq.split("->").nth(1).unwrap();
        let term = written
            .split('+')
            .map(str::trim)
            .find(|t| t.ends_with(first.formula.as_str()))
            .unwrap();
        let coef = term.trim_end_matches(first.formula.as_str()).trim();
        let amount: f32 = match coef {
            "" => 1.0,
            c if c.contains('/') => {
                let (n, d) = c.split_once('/').unwrap();
                n.parse::<f32>().unwrap() / d.parse::<f32>().unwrap()
            }
            c => c.parse().unwrap(),
        };
        r.delta_h_kj * amount / first.coef as f32
    }

    #[test]
    fn unbalanced_hydrogen_and_methane_follow_one_rule() {
        for tier in [Tier::Seedling, Tier::Apprentice] {
            for (text, rid) in [
                ("H2 + O2 -> H2O", "water_synthesis"),
                ("CH4 + O2 -> CO2 + H2O", "methane_combustion"),
            ] {
                let mut g = game(tier);
                g.jars.insert("CH4".into(), 4);
                let atoms = suit_atoms(&g);
                let out = g.cast(text);
                assert_eq!(
                    out.outcome,
                    Outcome::Partial,
                    "{tier:?} {text}: {}",
                    out.label
                );
                assert!(out.power <= 0.5, "{text}: sputter");
                assert!(out.suggestion.is_some(), "{text}: always a Fix it");
                let reacted = out.reacted.clone().expect("what reacted");
                // The reacted equation's own ΔH is the one shown.
                let expected = dh_of_written(&g, rid, &reacted);
                assert!(
                    (out.dh_kj - expected).abs() < 0.05,
                    "{text}: {reacted} ΔH {} vs {expected}",
                    out.dh_kj
                );
                assert!(out.label.starts_with("Not balanced ("), "{}", out.label);
                assert!(out.label.contains(&format!("What reacted: {reacted}")));
                assert!(out.label.contains("left over"), "{}", out.label);
                if tier == Tier::Apprentice {
                    let sign = if out.dh_kj < 0.0 { "-" } else { "+" };
                    assert!(
                        out.label
                            .contains(&format!("ΔH {sign}{:.1} kJ", out.dh_kj.abs())),
                        "{}",
                        out.label
                    );
                }
                for flavour in ["Boom", "Fireball", "burns"] {
                    assert!(!out.label.contains(flavour), "{}", out.label);
                }
                // Atoms are conserved: what left the suit is in the products.
                let after = suit_atoms(&g);
                let mut made = spell::stock_atoms(&out.products);
                for (el, n) in &out.from_air {
                    *made.entry(el.clone()).or_insert(0) -= *n;
                }
                let mut lost = atoms.clone();
                for (el, n) in &after {
                    *lost.entry(el.clone()).or_insert(0) -= n;
                }
                lost.retain(|_, n| *n != 0);
                made.retain(|_, n| *n != 0);
                assert_eq!(lost, made, "{text}: atoms");
            }
        }
        // Methane short of oxygen for one whole set takes its amounts twice.
        let mut g = game(Tier::Apprentice);
        g.jars.insert("CH4".into(), 4);
        let out = g.cast("CH4 + O2 -> CO2 + H2O");
        assert_eq!(out.reacted.as_deref(), Some("CH4 + 2O2 -> CO2 + 2H2O"));
        assert_eq!(out.unreacted, ["1 CH4"]);
        assert!((out.dh_kj + 802.3).abs() < 0.05);
    }

    #[test]
    fn running_out_always_names_the_substance_and_how_to_get_more() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            for (jar, text) in [
                ("H2O", "water"),
                ("H2", "boom"),
                ("CH4", "fireball"),
                ("CO2", "cloud"),
                ("CH4", "CH4 + O2 -> CO2 + H2O"),
            ] {
                let mut g = game(tier);
                g.jars.insert(jar.into(), 0);
                let out = g.cast(text);
                assert_eq!(out.outcome, Outcome::Fizzle, "{tier:?} {text}");
                let l = &out.label;
                assert!(!l.contains("Not enough"), "{l}");
                assert!(l.contains("Out of"), "{tier:?} {text}: {l}");
                if tier != Tier::Sprout {
                    assert!(l.contains(&format!("Out of {jar}:")), "{l}");
                }
                // A recovery cue after the name.
                let cue = l.split("Out of").nth(1).unwrap();
                assert!(cue.split_whitespace().count() >= 3, "{l}");
            }
        }
    }

    #[test]
    fn excess_reagent_is_named_as_whole_molecules() {
        let mut g = game(Tier::Apprentice);
        let out = g.cast("3H2 + O2 -> 2H2O");
        assert_eq!(out.outcome, Outcome::Partial);
        assert_eq!(out.unreacted, ["1 H2"]);
        assert!(out.label.contains("1 H2 left over"), "{}", out.label);
        assert!(out.label.contains("H: 6 left, 4 right"), "{}", out.label);
        assert!(!out.label.contains("excess"), "{}", out.label);
    }

    #[test]
    fn a_multiple_of_a_spell_is_the_same_spell_with_double_the_heat() {
        let mut g = game(Tier::Apprentice);
        let one = g.cast("2H2 + O2 -> 2H2O");
        g.jars.insert("H2".into(), 4);
        g.jars.insert("O2".into(), 2);
        let two = g.cast("4H2 + 2O2 -> 4H2O");
        assert_eq!(two.outcome, Outcome::Full);
        assert_eq!(two.canonical, one.canonical);
        assert_eq!(two.canonical.as_deref(), Some("2H2 + O2 -> 2H2O"));
        assert_eq!(two.multiple, 2);
        assert!((two.dh_kj - 2.0 * one.dh_kj).abs() < 0.01);
        assert!(two.label.contains("double the ΔH"), "{}", two.label);
        assert!(!two.first_discovery, "same page as before");
    }

    #[test]
    fn electrolysis_costs_battery_energy_away_from_a_crystal_and_near_one() {
        let mut g = game(Tier::Apprentice);
        assert!(g.nearest_crystal().is_none());
        let kj = g.crystal_kj;
        let out = g.cast("2H2O -> 2H2 + O2");
        assert_eq!(out.outcome, Outcome::Full, "{}", out.label);
        assert_eq!(out.delta_h_kj, Some(571.7));
        while g.bolts_in_flight() > 0 {
            g.advance_bolts();
        }
        assert!(
            (kj - g.crystal_kj - 571.7).abs() < 0.01,
            "away from crystals the battery pays: drop {}",
            kj - g.crystal_kj
        );
        // An empty battery splits nothing: no energy from nowhere.
        g.crystal_kj = 100.0;
        let jars = g.jars.clone();
        let out = g.cast("2H2O -> 2H2 + O2");
        assert_eq!(out.outcome, Outcome::Fizzle);
        assert_eq!(out.missing, [ENERGY]);
        assert!(out.label.contains('⚡'), "{}", out.label);
        assert_eq!(g.jars, jars);
        g.crystal_kj = g.crystal_max_kj;
        // Beside the crystal, the crystal pays the sourced +571.7 kJ per 2 H2O.
        g.player.x = 680.0;
        g.player.y = 136.0;
        assert!(g.nearest_crystal().is_some());
        let kj = g.crystal_kj;
        g.set_aim(720, 140, true);
        let out = g.cast("4H2O -> 4H2 + 2O2");
        assert_eq!(out.outcome, Outcome::Full);
        assert_eq!(out.products.get("H2"), Some(&4));
        assert_eq!(out.products.get("O2"), Some(&2));
        // The crystal pays when the spell lands (bolts only, no other ticking).
        while g.bolts_in_flight() > 0 {
            g.advance_bolts();
        }
        assert!(
            (kj - g.crystal_kj - 2.0 * 571.7).abs() < 0.01,
            "drop {}",
            kj - g.crystal_kj
        );
    }

    #[test]
    fn a_split_that_lands_on_the_crystal_does_not_drain_it() {
        let mut g = game(Tier::Apprentice);
        g.player.x = 680.0;
        g.player.y = 136.0;
        assert!(g.nearest_crystal().is_some());
        let kj = g.crystal_kj;
        let (cx, cy) = rooms::CRYSTAL.centre();
        g.set_aim(cx, cy, true);
        let out = g.cast("2H2O -> 2H2 + O2");
        assert_eq!(out.outcome, Outcome::Full);
        let mut landed = None;
        while g.bolts_in_flight() > 0 {
            g.advance_bolts();
            if let Some(f) = g.fx.iter().find(|f| f.kind == "impact") {
                landed = Some((f.x as i32, f.y as i32));
            }
        }
        let (x, y) = landed.expect("the split landed");
        assert!(
            rooms::CRYSTAL.grow(1).contains(x, y),
            "landed at {x},{y}, not on the crystal"
        );
        assert_eq!(g.crystal_kj, kj, "a split on the crystal costs it nothing");
    }

    #[test]
    fn spells_pass_through_the_crystal_column() {
        let g = game(Tier::Apprentice);
        let (cx, cy) = rooms::CRYSTAL.centre();
        // From just left of the crystal, aim at the air past it.
        let lands = g.trace((cx as f32 - 8.0, cy as f32), (cx + 30, cy));
        assert_eq!(lands, (cx + 30, cy));
    }

    #[test]
    fn unbalanced_cast_is_partial_for_seedling_with_a_fix() {
        let mut g = game(Tier::Seedling);
        let r = g.cast("H2 + O2 -> H2O");
        assert_eq!(r.outcome, Outcome::Partial);
        assert_eq!(r.suggestion.as_deref(), Some("2H2 + O2 -> 2H2O"));
        assert!(!r.imbalance.is_empty());
        assert!(!r.first_discovery);
        assert!(g.discovered.is_empty());
        assert!(
            r.label.starts_with("Not balanced (O: 2 left, 1 right)."),
            "{}",
            r.label
        );
        // Seedling: no ΔH and no "unreacted (excess)", just what was left.
        assert!(r.label.contains("What reacted: H2 + 1/2 O2 -> H2O"));
        assert!(r.label.contains("1/2 O2 left over"), "{}", r.label);
        assert!(!r.label.contains("ΔH") && !r.label.contains("excess"));
    }

    #[test]
    fn apprentice_lower_case_works_with_a_case_note() {
        let mut g = game(Tier::Apprentice);
        let r = g.cast("h2o");
        assert_ne!(r.outcome, Outcome::Unknown);
        assert_eq!(r.equation, "H2O");
        let note = r.note.expect("a note about case");
        assert!(note.contains("case"), "{note}");
        // Correct capitals, words and younger tiers get no note.
        assert!(g.cast("H2O").note.is_none());
        assert!(g.cast("water").note.is_none());
        assert!(game(Tier::Seedling).cast("h2o").note.is_none());
    }

    #[test]
    fn rejected_casts_say_why_in_kid_words() {
        let mut g = game(Tier::Seedling);
        let cases = [
            ("asdf", "parse", "That isn't a formula. Try H2O."),
            ("Hh", "element", "Hh is not an element"),
            ("NaCl", "not_in_world", "NaCl is not in this trial yet"),
            (
                "2Na + Cl2 -> 2NaCl",
                "not_in_world",
                "Balanced, real chemistry! Not in this trial yet",
            ),
            (
                "H2O -> H2 + O",
                "unbalanced",
                "O gas comes in pairs: O2. Then O: 1 on the left, 2 on the right",
            ),
        ];
        for (text, code, label) in cases {
            let r = g.cast(text);
            assert_eq!(r.outcome, Outcome::Unknown, "{text}");
            assert_eq!(r.reason, Some(code), "{text}");
            assert_eq!(r.label, label, "{text}");
        }
        assert_eq!(g.cast("H2O -> H2 + O").imbalance.len(), 1);
        // Real casts carry no rejection reason.
        assert_eq!(g.cast("H2O").reason, None);
    }

    #[test]
    fn apprentice_states_and_fractions_cast_with_sourced_enthalpy() {
        let mut g = game(Tier::Apprentice);
        let r = g.cast("2H2(g) + O2(g) -> 2H2O(g)");
        assert_eq!(r.outcome, Outcome::Full, "{}", r.label);
        assert_eq!(r.reaction_id.as_deref(), Some("water_synthesis"));
        assert!((r.dh_kj + 483.6).abs() < 0.01);
        assert_eq!(
            r.equation_states.as_deref(),
            Some("2H2(g) + O2(g) -> 2H2O(g)")
        );
        // Plain typing gets the book's states on the card.
        let r = g.cast("2H2O -> 2H2 + O2");
        assert_eq!(
            r.equation_states.as_deref(),
            Some("2H2O(l) -> 2H2(g) + O2(g)")
        );
        // Condensing the product to liquid releases 2 x 44.0 kJ more.
        g.jars.insert("H2".into(), 4);
        g.jars.insert("O2".into(), 2);
        let r = g.cast("2H2 + O2 -> 2H2O(l)");
        assert!((r.dh_kj + 571.6).abs() < 0.01, "{}", r.dh_kj);
        // A fraction is cast as whole molecules, and the label says so.
        let r = g.cast("H2 + 1/2O2 -> H2O");
        assert_ne!(r.outcome, Outcome::Unknown, "{}", r.label);
        assert!(r.label.contains("no half molecules"), "{}", r.label);
        assert_eq!(r.reason, None);
    }

    #[test]
    fn sprout_gets_auto_balanced() {
        let mut g = game(Tier::Sprout);
        let r = g.cast("H2 + O2 -> H2O");
        assert_eq!(r.outcome, Outcome::Full, "{}", r.label);
        assert_eq!(r.equation, "2H2 + O2 -> 2H2O");
        assert_eq!(r.delta_h_kj, None, "young players see words, not kJ");
        assert_eq!(g.jar("H2"), 2);
    }

    #[test]
    fn apprentice_gets_the_fix_after_the_first_unbalanced_cast() {
        let mut g = game(Tier::Apprentice);
        assert_eq!(
            g.cast("H2 + O2 -> H2O").suggestion.as_deref(),
            Some("2H2 + O2 -> 2H2O")
        );
    }

    #[test]
    fn zero_instead_of_letter_o_still_works_or_gets_a_hint() {
        let mut g = game(Tier::Seedling);
        assert_eq!(g.cast("H20").outcome, Outcome::Full, "the alias catches it");
        let r = g.cast("2H20");
        assert_eq!(r.outcome, Outcome::Unknown);
        assert_eq!(r.suggestion.as_deref(), Some("2H2O"));
    }

    #[test]
    fn gathering_takes_only_the_aimed_substance() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2O".into(), 0);
        g.player.x = 40.0;
        g.aim = (50, 155);
        assert_eq!(g.grid.get(50, 155), Species::Water);
        g.grid.set(51, 155, Species::CO2, AMBIENT_C);
        for _ in 0..10 {
            g.tick(INPUT_GATHER);
        }
        assert!(g.jar("H2O") >= 5, "H2O {}", g.jar("H2O"));
        assert_eq!(g.jar("CO2"), 0);
    }

    #[test]
    fn gather_with_no_aim_picks_the_nearest_water() {
        let mut g = game(Tier::Sprout);
        g.jars.insert("H2O".into(), 0);
        g.player.x = 40.0;
        g.player.y = 130.0;
        g.aim = (300, 20); // empty sky, far away
        let water = g.grid.count(Species::Water);
        for _ in 0..8 {
            g.tick(INPUT_GATHER);
        }
        assert!(g.grid.count(Species::Water) < water);
        assert!(g.jar("H2O") >= 4, "H2O {}", g.jar("H2O"));
    }

    #[test]
    fn water_cast_pours_a_stream_into_the_world() {
        let mut g = game(Tier::Sprout);
        g.set_aim(60, 100, true);
        let before = g.grid.count(Species::Water);
        let r = g.cast("H2O");
        assert_eq!(r.outcome, Outcome::Full);
        settle(&mut g, 30);
        assert!(g.grid.count(Species::Water) >= before + 40);
        assert_eq!(g.jar("H2O"), H2O_CAP - 1);
    }

    #[test]
    fn a_fresh_game_casts_water_then_boom_with_no_gathering() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = game(tier);
            for spell in ["water", "water", "water", "boom", "boom", "boom"] {
                let out = g.cast(spell);
                assert_eq!(
                    out.outcome,
                    Outcome::Full,
                    "{tier:?} {spell}: {}",
                    out.label
                );
                settle(&mut g, 120);
            }
        }
    }

    #[test]
    fn condenser_refills_water_within_100_ticks() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2O".into(), 0);
        settle(&mut g, 100);
        assert!(g.jar("H2O") >= 1);
    }

    #[test]
    fn electrolyser_always_makes_two_hydrogen_per_oxygen() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        // Away from the crystal: a slow trickle.
        settle(&mut g, 400);
        let (h, o) = (g.jar("H2"), g.jar("O2"));
        assert!(h > 0 && h == 2 * o, "H2 {h}, O2 {o}");
        // Beside it: fast, still 2:1, and the crystal pays for each split.
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        // The electrolyser now splits water from the tank: start it full.
        g.jars.insert("H2O".into(), H2O_CAP);
        g.player.x = 680.0;
        g.player.y = 136.0;
        g.crystal_kj = g.crystal_max_kj - 3000.0;
        let kj = g.crystal_kj;
        settle(&mut g, 60);
        let (h, o) = (g.jar("H2"), g.jar("O2"));
        assert!(h >= 8 && h == 2 * o, "H2 {h}, O2 {o}");
        let splits = o as f32;
        let expected = splits * 571.7 - 60.0 * CRYSTAL_CHARGE_KJ_PER_TICK;
        assert!(
            (kj - g.crystal_kj - expected).abs() < 1.0,
            "drop {}",
            kj - g.crystal_kj
        );
        // Every split cast is 2:1 too.
        for _ in 0..3 {
            let out = g.cast("split");
            let (h, o) = (out.products.get("H2"), out.products.get("O2"));
            assert_eq!(h.copied().unwrap_or(0), 2 * o.copied().unwrap_or(0));
        }
    }

    #[test]
    fn auto_electrolysis_draws_571_7_kj_per_split_away_from_the_crystal() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = game(tier);
            assert!(g.nearest_crystal().is_none());
            g.jars.insert("H2".into(), 0);
            g.jars.insert("O2".into(), 0);
            g.jars.insert("H2O".into(), H2O_CAP);
            // Below full, so the solar strip's charge is never capped.
            g.crystal_kj = g.crystal_max_kj - 2000.0;
            let kj = g.crystal_kj;
            let ticks = 600;
            let mut splits = 0;
            for _ in 0..ticks {
                let o2 = g.jar("O2");
                g.suit_tick();
                splits += g.jar("O2") - o2;
            }
            assert!(splits >= 3, "{tier:?}: {splits} splits");
            // Each split of 2 H2O costs the sourced +571.7 kJ; only the
            // solar strip puts energy back.
            let expected = splits as f32 * 571.7 - ticks as f32 * SOLAR_KJ_PER_TICK;
            let drop = kj - g.crystal_kj;
            assert!(
                (drop - expected).abs() < 0.5,
                "{tier:?}: dropped {drop}, expected {expected}"
            );
            assert!(drop > 0.0, "the gauge runs down while it works");
        }
    }

    #[test]
    fn an_empty_battery_stops_the_electrolyser_until_the_sun_or_a_crystal_charges_it() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        g.crystal_kj = 0.0;
        for _ in 0..150 {
            g.suit_tick();
        }
        assert_eq!(g.jar("O2"), 0, "no split with no energy");
        assert!((g.crystal_kj - 150.0 * SOLAR_KJ_PER_TICK).abs() < 0.01);
        // Beside the crystal it charges fast and the splitting starts.
        g.player.x = 680.0;
        g.player.y = 136.0;
        settle(&mut g, 120);
        assert!(g.jar("O2") > 0);
        assert_eq!(g.state().charge_source, "crystal");
    }

    #[test]
    fn background_gauge_changes_are_reported_apart_from_casts() {
        let mut g = game(Tier::Apprentice);
        g.state();
        g.jars.insert("H2O".into(), 5);
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        let before = g.jars.clone();
        settle(&mut g, 400);
        let s = g.state();
        // Every jar change in that time came from the suit itself.
        for (jar, n) in &s.jars {
            let d = *n as i32 - before[jar] as i32;
            assert_eq!(
                s.background.get(jar).copied().unwrap_or(0),
                d,
                "{jar}: {:?}",
                s.background
            );
        }
        assert!(s.background.get("H2").copied().unwrap_or(0) > 0);
        // A cast's own changes are not background.
        let out = g.cast("2H2 + O2 -> 2H2O");
        assert_eq!(out.outcome, Outcome::Full);
        let s = g.state();
        assert!(s.background.values().all(|&v| v == 0), "{:?}", s.background);
    }

    #[test]
    fn a_balanced_boom_card_says_what_it_spent_made_and_where_it_went() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = game(tier);
            let text = if tier == Tier::Sprout {
                "boom"
            } else {
                "2H2 + O2 -> 2H2O"
            };
            let out = g.cast(text);
            assert_eq!(out.outcome, Outcome::Full);
            assert_eq!(out.ledger.as_deref(), Some("-2 H2, -1 O2, +2 H2O"));
            assert_eq!(out.reacted.as_deref(), Some("2H2 + O2 -> 2H2O"));
            assert_eq!(out.went.as_deref(), Some("2 H2O released as steam"));
            // The ledger matches what left the suit.
            assert_eq!(g.jar("H2"), 2);
            assert_eq!(g.jar("O2"), 1);
        }
        let mut g = game(Tier::Apprentice);
        let out = g.cast("H2O");
        assert_eq!(out.ledger.as_deref(), Some("-1 H2O"));
        assert_eq!(out.went.as_deref(), Some("1 H2O poured out"));
        let out = g.cast("2H2O -> 2H2 + O2");
        assert_eq!(out.ledger.as_deref(), Some("-2 H2O, +2 H2, +1 O2"));
        assert_eq!(out.went.as_deref(), Some("2 H2 + 1 O2 released as bubbles"));
    }

    #[test]
    fn a_cast_that_fizzles_for_a_missing_reactant_leaves_stocks_unchanged() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = game(tier);
            assert_eq!(g.jar("CH4"), 0);
            let jars = g.jars.clone();
            let loose = g.loose.clone();
            for text in ["CH4 + 2O2 -> CO2 + 2H2O", "fireball", "cloud"] {
                let out = g.cast(text);
                assert_eq!(out.outcome, Outcome::Fizzle, "{tier:?} {text}");
                assert!(out.leftovers.is_empty(), "{text}: no atoms wasted");
                assert!(out.consumed.is_empty(), "{text}: nothing drawn");
                assert_eq!(g.jars, jars, "{tier:?} {text} touched the suit");
                assert_eq!(g.loose, loose);
            }
        }
    }

    #[test]
    fn electrolyser_refills_from_the_water_tank_and_conserves_atoms() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        g.player.x = 680.0;
        g.player.y = 136.0;
        let mut splits = 0;
        for _ in 0..200 {
            let mut before = suit_atoms(&g);
            before.retain(|_, n| *n > 0);
            let (h2o, o2) = (g.jar("H2O"), g.jar("O2"));
            g.suit_tick();
            let condensed = g
                .take_fx()
                .iter()
                .any(|f| f.kind == "gain" && f.reaction_id.as_deref() == Some("condense:H2O"));
            if g.jar("O2") > o2 {
                splits += 1;
                // Two water for one O2 (and two H2), less any condensed water.
                assert_eq!(h2o + u32::from(condensed) - g.jar("H2O"), 2);
            }
            let mut after = suit_atoms(&g);
            if condensed {
                // The collar's one outside source: water vapour from the air.
                *after.get_mut("H").unwrap() -= 2;
                *after.get_mut("O").unwrap() -= 1;
            }
            after.retain(|_, n| *n > 0);
            assert_eq!(before, after, "suit_tick created or lost atoms");
        }
        assert!(splits >= 3, "only {splits} splits");
        assert!(g.jar("H2O") >= SPLIT_RESERVE, "kept a reserve to cast");
    }

    #[test]
    fn every_gauge_gain_names_its_source() {
        let mut g = game(Tier::Seedling);
        g.jars.insert("H2O".into(), 5);
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        let mut causes = std::collections::BTreeSet::new();
        for _ in 0..400 {
            g.tick(0);
            for f in g.take_fx() {
                if f.kind == "gain" {
                    assert!(f.power >= 1.0);
                    causes.insert(f.reaction_id.unwrap());
                }
            }
        }
        assert!(causes.contains("condense:H2O"), "{causes:?}");
        assert!(causes.contains("split:H2") && causes.contains("split:O2"));
    }

    #[test]
    fn heat_lens_labels_the_burning_hedge_and_the_ice() {
        let mut g = game(Tier::Seedling);
        settle(&mut g, 30);
        let spots = g.hotspots(g.view_x.round() as i32, 3);
        let fire = g.goals()[0].x;
        assert!(
            spots
                .iter()
                .any(|&(x, _, t)| t > 300.0 && (x - fire).abs() < 80),
            "{spots:?}"
        );
        assert!(spots.len() <= 4);
        // Room 2's ice wall is the cold spot there.
        g.player.x = 440.0;
        g.snap_camera();
        let spots = g.hotspots(g.view_x.round() as i32, 3);
        assert!(spots.iter().any(|s| s.2 < 0.0), "{spots:?}");
    }

    #[test]
    fn state_version_only_moves_when_the_hud_changes() {
        let mut g = game(Tier::Seedling);
        g.player.x = 600.0;
        g.player.y = 120.0;
        g.snap_camera();
        settle(&mut g, 30);
        g.state();
        settle(&mut g, 2);
        let v = g.state_version();
        g.tick(0);
        assert_eq!(g.state_version(), v, "nothing changed");
        g.cast("water");
        assert!(g.state_version() > v, "the water gauge dropped");
    }
}

#[cfg(test)]
mod world {
    use super::*;

    #[test]
    fn walking_right_crosses_rooms_and_the_camera_follows() {
        let mut g = Game::new(Tier::Seedling, 42);
        // Past the burning hedge, with the gates open and the ice gone.
        for k in 0..ROOMS {
            g.open_gate(k);
        }
        g.grid.fill_rect(
            rooms::ICE.x0,
            rooms::ICE.y0,
            rooms::ICE.x1,
            rooms::ICE.y1,
            Species::Air,
        );
        g.player.x = 270.0;
        g.player.y = 120.0;
        g.snap_camera();
        for t in 0..1200u32 {
            // Hold right; hop now and then to climb out of the pit.
            let hop = if g.player.vx.abs() < 0.5 && t % 20 < 12 {
                INPUT_JUMP
            } else {
                0
            };
            g.tick(INPUT_RIGHT | hop);
            let screen_x = g.player.x - g.view_x;
            assert!(
                screen_x >= 0.0 && screen_x + PLAYER_W <= VIEW_W as f32,
                "player at {} left the view at {}",
                g.player.x,
                g.view_x
            );
        }
        assert!(g.player.x > 600.0, "only reached x = {}", g.player.x);
        assert!(g.view_x > 300.0);
    }

    #[test]
    fn distant_rooms_are_frozen() {
        let mut g = Game::new(Tier::Seedling, 42);
        g.grid.set(1200, 20, Species::Water, AMBIENT_C);
        for _ in 0..30 {
            g.tick(0);
        }
        assert_eq!(g.grid.get(1200, 20), Species::Water, "room 4 is off screen");
    }
}

#[cfg(test)]
mod feel {
    use super::*;

    /// A flat, empty stretch of room 3 with the player standing on it.
    fn arena() -> Game {
        let mut g = Game::new(Tier::Seedling, 42);
        g.grid.fill_rect(660, 0, 980, 150, Species::Air);
        g.grid.fill_rect(660, 150, 980, 180, Species::Stone);
        g.player.x = 700.0;
        g.player.y = 150.0 - PLAYER_H;
        g.snap_camera();
        g.tick(0);
        assert!(g.player.grounded);
        g
    }

    #[test]
    fn player_box_is_6_by_12() {
        let g = arena();
        let (x, y) = (g.player.x, g.player.y);
        assert!(!g.box_blocked(x, y));
        let mut g2 = arena();
        g2.grid
            .set(705, (y + 11.0) as i32, Species::Stone, AMBIENT_C);
        assert!(g2.box_blocked(x, y), "bottom-right cell is inside");
        let mut g3 = arena();
        g3.grid.set(706, y as i32, Species::Stone, AMBIENT_C);
        g3.grid
            .set(700, (y - 1.0) as i32, Species::Stone, AMBIENT_C);
        assert!(!g3.box_blocked(x, y), "cells just outside are not");
    }

    #[test]
    fn walking_reaches_two_cells_per_tick_within_ten_ticks() {
        let mut g = arena();
        for _ in 0..10 {
            g.tick(INPUT_RIGHT);
        }
        assert!(g.player.vx >= 2.0, "vx = {}", g.player.vx);
        let x0 = g.player.x;
        for _ in 0..60 {
            g.tick(INPUT_RIGHT);
        }
        assert!(g.player.x - x0 >= 120.0, "{} cells/s", g.player.x - x0);
    }

    /// Walks off a ledge and presses jump `late` ticks after leaving it.
    fn jump_after_ledge(late: u32) -> f32 {
        let mut g = arena();
        g.grid.fill_rect(680, 100, 720, 104, Species::Stone);
        g.player.x = 712.0;
        g.player.y = 100.0 - PLAYER_H;
        g.tick(0);
        assert!(g.player.grounded);
        while g.player.grounded {
            g.tick(INPUT_RIGHT);
        }
        for _ in 1..late {
            g.tick(INPUT_RIGHT);
        }
        g.tick(INPUT_RIGHT | INPUT_JUMP);
        g.player.vy
    }

    #[test]
    fn coyote_time_allows_a_late_jump() {
        assert!(jump_after_ledge(4) < -3.0, "jump 4 ticks late should work");
        assert!(jump_after_ledge(10) > 0.0, "but not 10 ticks late");
    }

    /// Drops the player and presses (and holds) jump `early` ticks before landing.
    fn jump_before_landing(early: u32) -> bool {
        let fall = |press_at: Option<u32>| {
            let mut g = arena();
            g.player.y = 60.0;
            let mut landed_at = None;
            let mut jumped = false;
            for t in 0..200u32 {
                let held = press_at.is_some_and(|p| t >= p);
                g.tick(if held { INPUT_JUMP } else { 0 });
                if landed_at.is_none() && g.player.grounded {
                    landed_at = Some(t);
                }
                if landed_at.is_some() && g.player.vy < -3.0 {
                    jumped = true;
                }
                if landed_at.is_some_and(|l| t > l + 3) {
                    break;
                }
            }
            (landed_at.expect("never landed"), jumped)
        };
        let (land, jumped) = fall(None);
        assert!(!jumped);
        fall(Some(land - early)).1
    }

    #[test]
    fn jump_buffer_fires_on_landing() {
        assert!(jump_before_landing(4), "pressed 4 ticks early should jump");
        assert!(!jump_before_landing(12), "12 ticks early is too early");
    }

    #[test]
    fn releasing_jump_early_makes_a_lower_jump() {
        let peak = |hold: u32| {
            let mut g = arena();
            let mut top = g.player.y;
            for t in 0..120 {
                g.tick(if t < hold { INPUT_JUMP } else { 0 });
                top = top.min(g.player.y);
            }
            g.player.y - top
        };
        let (tap, full) = (peak(2), peak(60));
        assert!(full > 25.0 && tap < full * 0.6, "tap {tap}, full {full}");
    }

    #[test]
    fn walking_climbs_a_two_cell_step_but_not_three() {
        let climb = |h: i32| {
            let mut g = arena();
            g.grid.fill_rect(720, 150 - h, 760, 150, Species::Stone);
            for _ in 0..40 {
                g.tick(INPUT_RIGHT);
            }
            g.player.x
        };
        assert!(climb(2) > 730.0, "stopped at a 2-cell step: x {}", climb(2));
        assert!(climb(3) < 720.0, "walked up a 3-cell wall: x {}", climb(3));
    }

    #[test]
    fn a_one_tick_jump_tap_clears_a_16_cell_stump() {
        let mut g = arena();
        let y0 = g.player.y;
        let mut top = y0;
        g.tick(INPUT_JUMP);
        for _ in 0..60 {
            g.tick(0);
            top = top.min(g.player.y);
        }
        assert!(y0 - top > 16.0, "tap rose only {}", y0 - top);
    }

    #[test]
    fn one_tick_with_jump_pressed_then_released_jumps() {
        let mut g = arena();
        g.tick(INPUT_JUMP);
        assert!(g.player.vy < 0.0, "vy {}", g.player.vy);
        assert!(g.player.vy <= JUMP_V + GRAVITY + 1e-4);
        g.tick(0);
        assert!(g.player.vy < 0.0, "released at once: vy {}", g.player.vy);
    }

    #[test]
    fn a_jump_tapped_and_released_between_ticks_still_jumps() {
        // Key down and up again before the sim stepped: the held bit never
        // reaches a tick, but the latched press does.
        let mut g = arena();
        let y0 = g.player.y;
        g.press_jump();
        g.tick(INPUT_RIGHT);
        assert!(g.player.vy < 0.0, "vy {}", g.player.vy);
        let mut top = y0;
        for _ in 0..40 {
            g.tick(INPUT_RIGHT);
            top = top.min(g.player.y);
        }
        assert!(y0 - top > 10.0, "rose only {}", y0 - top);
        // The latch is used up: no second jump without a second press.
        settle_ground(&mut g);
        g.tick(INPUT_RIGHT);
        assert!(g.player.vy >= 0.0, "jumped again: vy {}", g.player.vy);
    }

    fn settle_ground(g: &mut Game) {
        for _ in 0..80 {
            g.tick(0);
        }
        assert!(g.player.grounded);
    }

    #[test]
    fn hard_landings_emit_a_land_fx() {
        let mut g = arena();
        g.take_fx();
        g.player.y = 60.0;
        for _ in 0..120 {
            g.tick(0);
        }
        let fx = g.take_fx();
        assert_eq!(fx.iter().filter(|f| f.kind == "land").count(), 1, "{fx:?}");
    }
}

#[cfg(test)]
mod bolts {
    use super::*;

    /// A flat, empty stretch of room 3 (right of its crystal) with the player standing on it.
    fn arena() -> Game {
        let mut g = Game::new(Tier::Apprentice, 42);
        g.grid.fill_rect(680, 0, 980, 150, Species::Air);
        g.grid.fill_rect(680, 150, 980, 180, Species::Stone);
        g.player.x = 700.0;
        g.player.y = 150.0 - PLAYER_H;
        g.snap_camera();
        g.tick(0);
        g.take_fx();
        g
    }

    fn snapshot(g: &Game) -> Vec<(Species, u32)> {
        g.grid
            .cells
            .iter()
            .zip(&g.grid.temp)
            .map(|(c, t)| (*c, t.to_bits()))
            .collect()
    }

    #[test]
    fn water_bolt_lands_on_a_distant_fire_and_leaves_the_player_alone() {
        let mut g = arena();
        // A burning post about 80 cells away.
        g.grid.fill_rect(782, 120, 786, 150, Species::Wood);
        for y in 120..150 {
            for x in 782..786 {
                let i = g.grid.idx(x, y);
                g.grid.temp[i] = 650.0;
            }
            g.grid.spawn_flame(781, y as i32, 100, true, 1100.0);
        }
        let (hx, hy) = g.hand();
        g.set_aim(783, 140, true);
        let before = snapshot(&g);
        let out = g.cast("H2O");
        assert_eq!(out.outcome, Outcome::Full);
        assert_eq!(snapshot(&g), before, "nothing happens until the bolt lands");
        let mut landed = None;
        for _ in 0..30 {
            g.tick(0);
            if let Some(f) = g
                .take_fx()
                .into_iter()
                .find(|f| f.kind == "impact" && f.id == out.id)
            {
                landed = Some((f.x, f.y));
                break;
            }
        }
        let (ix, iy) = landed.expect("the bolt should land within 30 ticks");
        assert!(
            (ix - 781.0).abs() <= 2.0 && (iy - 140.0).abs() <= 3.0,
            "landed at {ix},{iy}"
        );
        let after = snapshot(&g);
        let changed_near = |cx: f32, cy: f32, r: f32| {
            (0..g.grid.cells.len())
                .filter(|&i| before[i].0 != after[i].0)
                .filter(|&i| {
                    let (x, y) = ((i % g.grid.w) as f32, (i / g.grid.w) as f32);
                    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
                })
                .count()
        };
        assert!(
            changed_near(ix, iy, 14.0) >= 40,
            "only {} cells changed",
            changed_near(ix, iy, 14.0)
        );
        assert_eq!(
            changed_near(hx, hy, 12.0),
            0,
            "nothing changes near the player"
        );
    }

    #[test]
    fn a_fizzle_is_one_fx_and_no_grid_change() {
        let mut g = arena();
        g.jars.insert("H2".into(), 0);
        g.jars.insert("O2".into(), 0);
        g.set_aim(760, 140, true);
        let before = snapshot(&g);
        let out = g.cast("boom");
        assert_eq!(out.outcome, Outcome::Fizzle);
        let fx = g.take_fx();
        assert_eq!(fx.len(), 1, "{fx:?}");
        assert_eq!(fx[0].kind, "fizzle");
        assert_eq!(g.bolts_in_flight(), 0);
        assert_eq!(snapshot(&g), before);
        // It points at the nearest crystal, where gas comes from.
        assert!(out.source_x.is_some());
        assert_eq!(
            (fx[0].tx, fx[0].ty),
            (out.source_x.unwrap() as f32, out.source_y.unwrap() as f32)
        );
    }

    #[test]
    fn boom_heat_scales_with_enthalpy() {
        let mut g = arena();
        g.set_aim(760, 140, true);
        let out = g.cast("boom");
        assert!((out.dh_kj + 483.6).abs() < 0.01);
        for _ in 0..12 {
            g.tick(0);
        }
        let hot = (740..780)
            .flat_map(|x| (120..150).map(move |y| (x, y)))
            .filter(|&(x, y)| g.grid.temp_at(x, y) > 200.0)
            .count();
        assert!(hot > 50, "only {hot} hot cells");
    }

    #[test]
    fn water_cards_from_the_spawn_put_the_fire_out_in_a_few_taps() {
        // No walking and no aiming: the first taps land on the fire.
        for tier in [Tier::Sprout, Tier::Seedling] {
            for seed in [1, 7, 42] {
                let mut g = Game::new(tier, seed);
                for _ in 0..30 {
                    g.tick(0);
                }
                let mut casts = 0;
                while !g.rooms.gates_open[0] && casts < 4 {
                    assert_eq!(g.cast("water").outcome, Outcome::Full);
                    casts += 1;
                    for _ in 0..60 {
                        g.tick(0);
                    }
                }
                assert!(g.rooms.hedges[0].hits > 0, "{tier:?} {seed}: missed");
                assert!(
                    g.rooms.gates_open[0],
                    "{tier:?} {seed}: still burning after {casts} taps"
                );
            }
        }
    }

    #[test]
    fn apprentice_who_never_aimed_puts_out_hedge_one_with_typed_water() {
        // Keyboard only: no set_aim or track_aim call at all.
        for seed in [1, 7, 42] {
            let mut g = Game::new(Tier::Apprentice, seed);
            for _ in 0..30 {
                g.tick(0);
            }
            assert_eq!(g.aim_tick, 0);
            let mut t = 0;
            while !g.rooms.gates_open[0] && t < 120 {
                if t % 40 == 0 {
                    assert_eq!(g.cast("water").outcome, Outcome::Full);
                }
                g.tick(0);
                t += 1;
            }
            assert!(g.rooms.hedges[0].hits > 0, "seed {seed}: missed");
            assert!(
                g.rooms.gates_open[0],
                "seed {seed}: still burning after 3 s"
            );
        }
    }

    #[test]
    fn reticle_shows_the_goal_until_the_player_aims_and_t_aims_there() {
        let mut g = Game::new(Tier::Apprentice, 3);
        for _ in 0..30 {
            g.tick(0);
        }
        let goal = g.goal_aim_point().unwrap();
        assert_eq!(g.shown_aim(), goal);
        g.set_aim(10, 10, true);
        assert_eq!(g.shown_aim(), (10, 10));
        assert!(g.aim_at_goal());
        assert_eq!(g.aim, goal);
        assert_eq!(g.shown_aim(), goal);
    }

    #[test]
    fn sprout_water_auto_aims_at_the_fire() {
        let mut g = Game::new(Tier::Sprout, 42);
        for _ in 0..30 {
            g.tick(0);
        }
        g.player.x = 150.0;
        g.player.y = 130.0;
        let out = g.cast("water");
        assert_eq!(out.outcome, Outcome::Full);
        let (ax, _) = g.aim;
        let zone = g.rooms.hedges[0].zone();
        assert!((zone.x0..zone.x1).contains(&ax), "aimed at {ax}");
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
            // Cast now and then, so spells are in flight across ticks.
            match t {
                5 => {
                    g.set_aim(120, 120, true);
                    g.cast("water");
                }
                40 => {
                    g.cast("boom");
                    assert!(g.bolts_in_flight() > 0);
                }
                41 => {
                    g.cast("split");
                }
                _ => {}
            }
            // Walk right for a while so the player and camera move too.
            g.tick(if (60..300).contains(&t) {
                INPUT_RIGHT
            } else {
                0
            });
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

    /// Heat-pass sleeping is an optimisation only: with fire, spells, steam,
    /// a moving camera (so the stepped range slides) and a room restart (an
    /// external rewrite of cells), every step must match the plain pass bit
    /// for bit.
    #[test]
    fn heat_sleep_matches_the_plain_heat_pass_bit_for_bit() {
        for seed in [3, 42] {
            let mut a = Game::new(Tier::Apprentice, seed);
            let mut b = Game::new(Tier::Apprentice, seed);
            b.grid.heat_sleep = false;
            for t in 0..900u32 {
                for g in [&mut a, &mut b] {
                    match t {
                        10 => {
                            g.set_aim(200, 120, true);
                            g.cast("2H2 + O2 -> 2H2O");
                        }
                        30 => {
                            g.cast("water");
                        }
                        200 => {
                            g.cast("2H2O -> 2H2 + O2");
                        }
                        500 => {
                            g.restart_room();
                        }
                        _ => {}
                    }
                    let walk = match t {
                        60..=350 => INPUT_RIGHT,
                        600..=700 => INPUT_LEFT,
                        _ => 0,
                    };
                    g.tick(walk);
                }
                if t % 25 == 0 {
                    assert_eq!(grid_hash(&a), grid_hash(&b), "seed {seed}, tick {t}");
                }
            }
            assert_eq!(grid_hash(&a), grid_hash(&b));
        }
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

    /// Worst case for the W21 budget (< 2 ms/step native, release): all four
    /// rooms stepped every tick, not just the active region around the
    /// camera, while the hedge fire burns and spells land. Run with
    /// `cargo test --release bench_full_level -- --nocapture`.
    #[test]
    fn bench_full_level_600_ticks() {
        let mut g = Game::new(Tier::Apprentice, 42);
        g.step_all_columns = true;
        let start = std::time::Instant::now();
        for t in 0..600u32 {
            match t {
                10 => {
                    g.set_aim(200, 120, true);
                    g.cast("boom");
                }
                60 => {
                    g.cast("2H2O -> 2H2 + O2");
                }
                _ => {}
            }
            g.tick(if (100..400).contains(&t) {
                INPUT_RIGHT
            } else {
                0
            });
            // tick() stepped the active region; step the rest of the world too.
            let vx = g.view_x.round() as usize;
            let (a0, a1) = (vx.saturating_sub(ACTIVE_BEHIND), vx + ACTIVE_AHEAD);
            g.grid.step_columns(0, a0);
            g.grid.step_columns(a1, WORLD_W);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / 600.0;
        println!("bench_full_level_600_ticks: {ms:.3} ms/step");
        assert!(ms < 250.0, "{ms} ms/step");
    }
}
