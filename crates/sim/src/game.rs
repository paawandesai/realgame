//! Game rules on top of the grid: the player, reagent jars, casting, and the
//! first breakthrough trial.

use crate::formula::Atoms;
use crate::grid::{Grid, AMBIENT_C, CH4_IGNITION_C, WOOD_IGNITION_C};
use crate::reactions::ReactionBook;
use crate::species::{Phase, Species};
use crate::spell::{self, Outcome, Plan, Resolution, Stock};
use serde::Serialize;
use std::collections::BTreeMap;

mod rooms;
pub use rooms::{Goal, Rect, TorchView};

/// The world is four 320-cell rooms side by side; the camera shows one room's width.
pub const ROOM_W: usize = 320;
pub const ROOMS: usize = 4;
pub const WORLD_W: usize = ROOM_W * ROOMS;
pub const VIEW_W: usize = 320;
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
/// How close (in cells) the player must be to a crystal to draw on it.
const CRYSTAL_RANGE: i32 = 16;
/// The crystal holds enough energy for this many splits, and refills slowly.
const CRYSTAL_SPLITS: f32 = 10.0;
const CRYSTAL_RECHARGE_KJ_PER_TICK: f32 = 1.0;
/// One water cast becomes a stream this many cells big...
const WATER_CELLS_PER_MOL: u32 = 40;
/// ...that cools everything within this radius of where it lands...
const WATER_COOL_RADIUS: i32 = 6;
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
/// Sprout and Seedling aim at the obvious target when the player has not
/// aimed on purpose for this long (3 s).
const AUTO_AIM_TICKS: u64 = 180;
pub const PLAYER_W: f32 = 6.0;
pub const PLAYER_H: f32 = 12.0;
/// Movement tuning, in cells and ticks (60 ticks a second).
const RUN_MAX: f32 = 2.2;
/// Reaches RUN_MAX in under 7 ticks from a standstill.
const RUN_ACCEL: f32 = 0.34;
const RUN_DECEL: f32 = 0.5;
const SWIM_MAX: f32 = 1.1;
const JUMP_V: f32 = -4.5;
/// Letting go of jump while rising keeps only this share of the upward speed.
const JUMP_CUT: f32 = 0.45;
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
    pub concept: Option<String>,
    pub real_world: Option<String>,
    /// Shown at Apprentice tier and above, with its source.
    pub delta_h_kj: Option<f32>,
    pub source: Option<String>,
    pub first_discovery: bool,
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
    /// True while rising from a jump the player can still cut short.
    jumping: bool,
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
    /// Crystal battery energy, kJ. Electrolysis draws on it.
    pub crystal_kj: f32,
    pub crystal_max_kj: f32,
    /// Energy to split 2 H2O, kJ (the sourced +571.7 from the reaction table).
    split_kj: f32,
    condense_timer: u32,
    electro_timer: u32,
    gather_cells: u32,
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
    bolts: Vec<Bolt>,
    state_version: u64,
    state_sig: u64,
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
                x: 12.0,
                y: 130.0,
                prev_x: 12.0,
                prev_y: 130.0,
                vx: 0.0,
                vy: 0.0,
                facing: 1,
                stride: 0.0,
                grounded: false,
                coyote: 0,
                jump_buffer: 0,
                jump_held: false,
                jumping: false,
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
            condense_timer: 0,
            electro_timer: 0,
            gather_cells: 0,
            objectives: vec![],
            messages: vec![],
            fx: vec![],
            aim: (40, 140),
            aim_tick: 0,
            ticks: 0,
            view_x: 0.0,
            prev_view_x: 0.0,
            book,
            discovered: vec![],
            misses_in_a_row: 0,
            next_cast_id: 1,
            torch_lit: false,
            rooms: rooms::Rooms::new(),
            met_crystal: false,
            gather_target: None,
            bolts: vec![],
            state_version: 1,
            state_sig: 0,
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
        self.grid
            .step_columns(vx.saturating_sub(VIEW_W), vx + 2 * VIEW_W);
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
        let pressed = held && !self.player.jump_held;
        self.player.jump_held = held;

        // Run: accelerate toward the target speed, stop quickly when let go.
        let max = if swimming { SWIM_MAX } else { RUN_MAX };
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
        if p.jumping && (!held || p.vy >= 0.0) {
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

        // Horizontal, in steps of at most one cell, each with a one-cell
        // step-up so bumpy ground doesn't stop you.
        let steps = self.player.vx.abs().ceil().max(1.0) as i32;
        let sx = self.player.vx / steps as f32;
        for _ in 0..steps {
            let (x, y) = (self.player.x + sx, self.player.y);
            if !self.box_blocked(x, y) {
                self.player.x = x;
            } else if grounded && !self.box_blocked(x, y - 1.0) {
                self.player.x = x;
                self.player.y -= 1.0;
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
            return;
        }
        let aimed = self.substance_near_aim().filter(|_| self.aim_in_reach());
        if self.gather_target.is_none() {
            self.gather_target = aimed.or_else(|| self.nearest_gatherable());
        }
        let Some(target) = self.gather_target else {
            return;
        };
        let Some(jar) = target.jar() else { return };
        if self.jar(jar) >= Self::jar_cap(jar) {
            return;
        }
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
        for (x, y) in picked {
            self.grid.set(x, y, Species::Air, AMBIENT_C);
            self.gather_cells += 1;
            if self.gather_cells >= per_mol {
                self.gather_cells = 0;
                self.add(jar, 1);
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
    /// air, and a wrist electrolyser that splits condensed water 2:1 into
    /// hydrogen and oxygen, powered by a crystal (fast) or a solar strip (slow).
    /// The electrolyser draws on the condenser directly, so it never empties
    /// the water tank the player casts from.
    fn suit_tick(&mut self) {
        if self.jar("H2O") < H2O_CAP {
            self.condense_timer += 1;
            if self.condense_timer >= CONDENSE_TICKS {
                self.condense_timer = 0;
                self.add("H2O", 1);
            }
        } else {
            self.condense_timer = 0;
        }
        self.crystal_kj = (self.crystal_kj + CRYSTAL_RECHARGE_KJ_PER_TICK).min(self.crystal_max_kj);

        let crystal = self.nearest_crystal();
        if crystal.is_some() && !self.met_crystal {
            self.met_crystal = true;
            self.say("The crystal stores electricity. Your suit uses it to split water into hydrogen and oxygen, fast.");
        }
        let room =
            self.jar("H2") + 2 <= Self::jar_cap("H2") && self.jar("O2") < Self::jar_cap("O2");
        if !room {
            self.electro_timer = 0;
            return;
        }
        let powered = crystal.filter(|_| self.crystal_kj >= self.split_kj);
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
        // 2H2O -> 2H2 + O2: always exactly two hydrogen for each oxygen.
        self.add("H2", 2);
        self.add("O2", 1);
        if let Some((x, y)) = powered {
            self.crystal_kj -= self.split_kj;
            let (hx, hy) = self.hand();
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
            Plan::Unknown { suggestion } => {
                self.misses_in_a_row += 1;
                let out = self.unknown_outcome(id, &input, suggestion.clone());
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
                let (reaction, known) = self
                    .book
                    .by_id(reaction_id)
                    .cloned()
                    .expect("plans only name reactions from the book");
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
        // Splitting water costs energy: the crystal pays when it is in reach.
        if spell == "split" && res.units > 0 && self.nearest_crystal().is_some() {
            let cost = self.split_kj * res.units as f32;
            self.crystal_kj = (self.crystal_kj - cost).max(0.0);
        }

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
            let offer = self.tier == Tier::Seedling || self.misses_in_a_row >= 3;
            if offer && !balanced {
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

        let first = match &reaction_id {
            Some(rid) if res.units > 0 && !self.discovered.contains(rid) => {
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
                self.say(r.concept.clone());
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
        let label = self.label(spell, outcome, &res);
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
            concept: entry.as_ref().map(|r| r.concept.clone()),
            real_world: entry.as_ref().map(|r| r.real_world.clone()),
            delta_h_kj: entry.as_ref().filter(|_| show_numbers).map(|_| res.dh_kj),
            source: entry
                .as_ref()
                .filter(|_| show_numbers)
                .map(|r| r.source.clone()),
            first_discovery: first,
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

    fn unknown_outcome(&self, id: u32, input: &str, suggestion: Option<String>) -> CastOutcome {
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
            label: match &suggestion {
                Some(s) => format!("Did you mean {s}? (letter O, not zero)"),
                None => "Not in your Grimoire yet".into(),
            },
            suggestion,
            imbalance: vec![],
            concept: None,
            real_world: None,
            delta_h_kj: None,
            source: None,
            first_discovery: false,
        }
    }

    /// A short line for the floating label at the impact point.
    fn label(&self, spell: &str, outcome: Outcome, res: &Resolution) -> String {
        let young = self.tier == Tier::Sprout;
        if outcome == Outcome::Fizzle {
            let what = res.missing.first().map(String::as_str).unwrap_or("");
            let name = match what {
                "H2O" => "water",
                "H2" => "hydrogen",
                "O2" => "oxygen",
                "CO2" => "carbon dioxide",
                "CH4" => "methane",
                _ => what,
            };
            return if young {
                format!("Out of {name}!")
            } else {
                format!("Not enough {what}")
            };
        }
        let main = match (spell, young) {
            ("water", true) => "Splash!",
            ("water", false) => "Water stream",
            ("boom", true) => "Boom! Hydrogen and oxygen made water",
            ("boom", false) => "Boom! Hydrogen burns into water",
            ("split", true) => "Bubbles! Water split in two",
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
        if !res.leftovers.is_empty() {
            text.push_str(&format!(
                "  {} left over",
                spell::leftover_label(&res.leftovers)
            ));
        }
        text
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

    fn stops_bolt(s: Species) -> bool {
        !s.is_gas()
    }

    /// Sprout and Seedling: if the player has not aimed in the last 3 s, the
    /// spell goes to the obvious target (fire for water, ice or the torch for
    /// heat) when one is in reach.
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
        if self.tier > Tier::Seedling {
            return;
        }
        if !never_aimed && self.ticks.saturating_sub(self.aim_tick) <= AUTO_AIM_TICKS {
            return;
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
        if let Some(p) = found {
            self.aim = p;
        }
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
                self.note_douse(at);
                self.douse_vents(at);
                let cells = (n("H2O") * WATER_CELLS_PER_MOL).min(4 * WATER_CELLS_PER_MOL);
                self.spawn_blob_at(at, Species::Water, cells, AMBIENT_C);
                extra.push("splash");
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
                self.note_douse(at);
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
                let melted = self.melt_ice(at, radius);
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
    fn unbalanced_hydrogen_is_partial_with_one_oxygen_left() {
        let mut g = game(Tier::Apprentice);
        let out = g.cast("H2 + O2 -> H2O");
        assert_eq!(out.outcome, Outcome::Partial);
        assert_eq!(out.leftovers, [("O".to_string(), 1)].into_iter().collect());
        assert_eq!(out.products, [("H2O".to_string(), 1)].into_iter().collect());
        assert_eq!(g.loose.get("O"), Some(&1), "the stray O waits in the suit");
    }

    #[test]
    fn electrolysis_works_away_from_a_crystal_and_drains_it_near_one() {
        let mut g = game(Tier::Apprentice);
        assert!(g.nearest_crystal().is_none());
        let kj = g.crystal_kj;
        let out = g.cast("2H2O -> 2H2 + O2");
        assert_eq!(out.outcome, Outcome::Full, "{}", out.label);
        assert_eq!(
            g.crystal_kj, kj,
            "the solar strip powers it away from crystals"
        );
        assert_eq!(out.delta_h_kj, Some(571.7));
        // Beside the crystal, the crystal pays the sourced +571.7 kJ per 2 H2O.
        g.player.x = 680.0;
        g.player.y = 136.0;
        assert!(g.nearest_crystal().is_some());
        let kj = g.crystal_kj;
        let out = g.cast("4H2O -> 4H2 + 2O2");
        assert_eq!(out.outcome, Outcome::Full);
        assert_eq!(out.products.get("H2"), Some(&4));
        assert_eq!(out.products.get("O2"), Some(&2));
        assert!(
            (kj - g.crystal_kj - 2.0 * 571.7).abs() < 0.01,
            "drop {}",
            kj - g.crystal_kj
        );
    }

    #[test]
    fn unbalanced_cast_is_partial_for_seedling_with_a_fix() {
        let mut g = game(Tier::Seedling);
        let r = g.cast("H2 + O2 -> H2O");
        assert_eq!(r.outcome, Outcome::Partial);
        assert_eq!(r.suggestion.as_deref(), Some("2H2 + O2 -> 2H2O"));
        assert!(!r.imbalance.is_empty());
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
    fn apprentice_gets_no_fix_until_three_misses() {
        let mut g = game(Tier::Apprentice);
        g.jars.insert("H2".into(), 12);
        g.jars.insert("O2".into(), 6);
        assert!(g.cast("H2 + O2 -> H2O").suggestion.is_none());
        assert!(g.cast("H2 + O2 -> H2O").suggestion.is_none());
        assert!(g.cast("H2 + O2 -> H2O").suggestion.is_some());
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
        g.player.x = 680.0;
        g.player.y = 136.0;
        g.crystal_kj = g.crystal_max_kj - 1000.0;
        let kj = g.crystal_kj;
        settle(&mut g, 60);
        let (h, o) = (g.jar("H2"), g.jar("O2"));
        assert!(h >= 8 && h == 2 * o, "H2 {h}, O2 {o}");
        let splits = o as f32;
        let expected = splits * 571.7 - 60.0 * CRYSTAL_RECHARGE_KJ_PER_TICK;
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
