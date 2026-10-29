//! The four rooms of the first trial, left to right, each closed by a stone
//! gate that opens once its goal is physically true:
//!
//! 1. Splash Garden: a bounded hedge fire; water (or CO2) puts it out.
//! 2. Frozen Gate: an ice wall over a dry pit; heat (a boom, or a carried
//!    torch) melts it into water.
//! 3. Crystal Lab: electrolysis bubbles 2:1 under a stone overhang; lighting
//!    the trapped hydrogen blasts the door.
//! 4. Marsh Door: methane vents, a second burning hedge, and a brazier that
//!    opens the final door.
//!
//! No goal completes without something the player did: every gate checks a
//! player cause as well as the world state.

use super::*;
use crate::grid::{CH4_IGNITION_C, FLAME_C_IN_AIR, WOOD_FIRE_C};

/// A rectangle of cells, `x0..x1` by `y0..y1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    pub fn grow(&self, d: i32) -> Rect {
        Rect::new(self.x0 - d, self.y0 - d, self.x1 + d, self.y1 + d)
    }

    pub fn centre(&self) -> (i32, i32) {
        ((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2)
    }

    fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.y0..self.y1).flat_map(move |y| (self.x0..self.x1).map(move |x| (x, y)))
    }
}

/// Ground level: the top row of stone on flat ground.
pub const GROUND_Y: i32 = 148;
/// Each room's gate: a stone column with a door at its foot.
pub const GATE_X: [i32; ROOMS] = [308, 628, 948, 1264];
pub const GATE_W: i32 = 6;
pub const DOOR_TOP: i32 = 112;

// Room 1: Splash Garden.
pub const HEDGE1: Rect = Rect::new(236, 132, 260, GROUND_Y);
/// Where to stand to douse it: past the low wall, where the trial starts.
pub const HEDGE1_STAND_X: i32 = 214;
// Room 2: Frozen Gate.
pub const TORCH_X: i32 = 344;
const TORCH_H: i32 = 12;
pub const ICE: Rect = Rect::new(470, 128, 482, GROUND_Y);
const ICE_ROCK: Rect = Rect::new(458, 0, 496, 128);
const PIT: Rect = Rect::new(504, GROUND_Y, 576, 164);
/// The pit's stairs: each step is this many cells high...
const PIT_RISER: i32 = 2;
/// ...and this many cells deep.
const PIT_TREAD: i32 = 2;
// Room 3: Crystal Lab.
pub const CRYSTAL: Rect = Rect::new(668, 134, 674, GROUND_Y);
pub const POOL: Rect = Rect::new(732, GROUND_Y, 784, 160);
const ROOF: Rect = Rect::new(722, 104, 794, 108);
/// The gas pocket under the overhang, where rising hydrogen collects.
pub const TRAP: Rect = Rect::new(726, 108, 790, 120);
/// A blast counts once this much hydrogen has been trapped...
const TRAP_MIN_H2: u32 = 4;
/// ...and burns hot enough to be hydrogen (it burns at 2000 C in air, 2800 C
/// in oxygen; a boom on its own peaks near 1100 C).
const H2_BLAST_C: f32 = 1500.0;
// Room 4: Marsh Door.
const MARSH: Rect = Rect::new(1000, GROUND_Y, 1064, 156);
pub const VENTS: [i32; 2] = [1016, 1048];
const VENT_TICKS: u64 = 30;
/// A lit vent burns its methane as fast as it seeps out.
const VENT_LIT_TICKS: u64 = 4;
/// A doused vent can't relight for this long.
const VENT_WET_TICKS: u64 = 180;
/// Heat this close to a vent's mouth lights it.
const VENT_LIGHT_R: i32 = 6;
/// Vents stop when this much methane is already pooled near them.
const VENT_MAX_CH4: usize = 40;
/// A rock overhang above the marsh. Methane (0.717 g/L at 0 C, CRC Handbook)
/// is lighter than air (1.293 g/L), so it rises and pools under the rock
/// within reach of the suit, as hydrogen does under room 3's roof.
const MARSH_CAP: Rect = Rect::new(990, 104, 1074, 108);
/// The pocket under the overhang, between its two lips.
const MARSH_POCKET: Rect = Rect::new(994, 108, 1070, 122);
/// Where to stand to gather the marsh methane: at the marsh's edge.
#[cfg(test)]
pub const MARSH_STAND_X: i32 = 996;
/// Methane counted toward the vents' cap: the pocket and the air below it.
const MARSH_AIR: Rect = Rect::new(990, 104, 1074, GROUND_Y + 8);
const ALCOVE_ROOF: Rect = Rect::new(1176, 118, GATE_X[3], 124);
pub const HEDGE2: Rect = Rect::new(1182, 124, 1198, GROUND_Y);
pub const BRAZIER: Rect = Rect::new(1234, 141, 1244, GROUND_Y);
/// A heat cast landing this close (cells) in the last `CAUSE_TICKS` counts as
/// the player lighting the brazier.
const BRAZIER_REACH: f32 = 24.0;
const CAUSE_TICKS: u64 = 240;
/// Under water this many ticks in a row puts a carried torch out.
const DUNK_TICKS: u8 = 20;
/// Hot CO2 within this many cells of the hand is drawn into the suit.
const CAPTURE_RADIUS: i32 = 14;
/// Fire spreads through wood more slowly than v1's forest did (0.25).
const ROOM_WOOD_FLAME_CHANCE: f32 = 0.2;
/// Wood that blocks the player: you can't push through burning wood.
pub const HOT_WOOD_BLOCKS_C: f32 = 150.0;
/// A doused hedge that stays out this long (no flame, no hot wood touching
/// oxygen) has had its fire triangle broken: it falls to ash for good.
const HEDGE_OUT_TICKS: u32 = 20;
/// Out at least this long, then burning again, counts as re-igniting.
const RELIGHT_AFTER_TICKS: u32 = 3;
/// A CO2 cloud cast on a hedge keeps the oxygen away this long (3 s).
/// CO2 is denser than air (1.977 vs 1.293 g/L at 0 C, CRC Handbook), so it
/// sinks onto and around the hedge and stays there for a while.
const SMOTHER_TICKS: u64 = 180;
/// While smothered, hot wood loses this fraction of its excess heat per
/// tick: with no oxygen nothing burns to keep it hot, so the charcoal cools
/// (a gameplay rate, not a measured one).
const SMOTHER_COOL: f32 = 0.03;

#[derive(Debug, Clone)]
pub struct Hedge {
    pub rect: Rect,
    /// Water or CO2 casts that landed on this hedge.
    pub hits: u32,
    /// How many of those were CO2 (it smothers: takes away the oxygen)
    /// rather than water (it cools: takes away the heat).
    pub co2_hits: u32,
    /// Burning cells now, and the most there have been (for the meter).
    pub burning: u32,
    pub peak: u32,
    pub out: bool,
    /// Put out for good: the charred hedge fell to ash and can't relight.
    pub crumbled: bool,
    /// Ticks in a row it has been out after the player's casts.
    out_ticks: u32,
    /// A CO2 cloud blankets it until this tick: no oxygen reaches it.
    smother_until: u64,
    /// Mote has said it burned out with no help.
    said_self_out: bool,
}

impl Hedge {
    fn new(rect: Rect) -> Hedge {
        Hedge {
            rect,
            hits: 0,
            co2_hits: 0,
            burning: 0,
            peak: 0,
            out: false,
            crumbled: false,
            out_ticks: 0,
            smother_until: 0,
            said_self_out: false,
        }
    }

    /// Where its flames can be: the hedge, its sides and the air above.
    pub fn zone(&self) -> Rect {
        Rect::new(
            self.rect.x0 - 4,
            self.rect.y0 - 24,
            self.rect.x1 + 4,
            self.rect.y1,
        )
    }

    /// "Fire left", 0..=1.
    pub fn meter(&self) -> f32 {
        if self.out || self.peak == 0 {
            0.0
        } else {
            (self.burning as f32 / self.peak as f32).clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Torch {
    /// Where it stands (bottom of the post) when nobody holds it.
    pub x: f32,
    pub y: f32,
    pub lit: bool,
    pub carried: bool,
    /// Ticks in a row the holder's head has been under water.
    wet: u8,
}

#[derive(Debug, Clone)]
pub struct Rooms {
    pub hedges: [Hedge; 2],
    pub gates_open: [bool; ROOMS],
    pub ice_start: u32,
    pub ice_left: u32,
    /// The player's heat (a boom or the torch) reached the ice.
    pub ice_heated: bool,
    pub torch: Torch,
    pub trap_h2: u32,
    pub trap_peak: u32,
    /// The player split water in room 3 (step 1 of its goal).
    pub gas_made: bool,
    /// "Now boom the gas!" was said for the gas trapped now.
    boom_prompted: bool,
    pub brazier_lit: bool,
    /// The last heat spell's landing point and tick (a player cause).
    pub last_heat: Option<(u64, i32, i32)>,
    pub vent_lit: [bool; 2],
    vent_wet_until: [u64; 2],
    ch4_loose: u32,
    entered: [bool; ROOMS],
    capture_cells: u32,
}

impl Rooms {
    pub fn new() -> Rooms {
        Rooms {
            hedges: [Hedge::new(HEDGE1), Hedge::new(HEDGE2)],
            gates_open: [false; ROOMS],
            ice_start: 0,
            ice_left: 0,
            ice_heated: false,
            torch: Torch {
                x: TORCH_X as f32,
                y: GROUND_Y as f32,
                lit: false,
                carried: false,
                wet: 0,
            },
            trap_h2: 0,
            trap_peak: 0,
            gas_made: false,
            boom_prompted: false,
            brazier_lit: false,
            last_heat: None,
            vent_lit: [false; 2],
            vent_wet_until: [0; 2],
            ch4_loose: 0,
            entered: [false; ROOMS],
            capture_cells: 0,
        }
    }
}

impl Rooms {
    /// Puts room `k`'s state back the way a freshly built level has it.
    fn reset_room(&mut self, k: usize, fresh: &Rooms) {
        self.gates_open[k] = false;
        // Room 1's intro is spoken when the level is built; later rooms
        // introduce themselves again when the player is back inside.
        self.entered[k] = k == 0;
        self.last_heat = None;
        match k {
            0 => self.hedges[0] = fresh.hedges[0].clone(),
            1 => {
                self.ice_start = fresh.ice_start;
                self.ice_left = fresh.ice_left;
                self.ice_heated = false;
                self.torch = fresh.torch.clone();
            }
            2 => {
                self.trap_h2 = 0;
                self.trap_peak = 0;
                self.gas_made = false;
                self.boom_prompted = false;
            }
            _ => {
                self.hedges[1] = fresh.hedges[1].clone();
                self.brazier_lit = false;
                self.vent_lit = fresh.vent_lit;
                self.vent_wet_until = fresh.vent_wet_until;
                self.ch4_loose = fresh.ch4_loose;
                self.capture_cells = fresh.capture_cells;
            }
        }
    }
}

impl Default for Rooms {
    fn default() -> Self {
        Rooms::new()
    }
}

/// A target the HUD marks with a bobbing icon (and a meter for fires).
#[derive(Debug, Clone, Serialize)]
pub struct Goal {
    pub room: u8,
    /// "fire", "ice", "gas" or "brazier".
    pub kind: &'static str,
    pub x: i32,
    pub y: i32,
    pub done: bool,
    /// "Fire left" for the hedges and "ice left" for the ice wall, or how
    /// full the gas trap is in room 3, 0..=1.
    pub meter: Option<f32>,
    /// The spell that moves this goal on right now: "water", "boom",
    /// "split" (for hints; any other valid solution still counts).
    pub need: &'static str,
    /// Where to stand (player centre, world cells) to cast `need` at it.
    pub stand_x: i32,
    /// The room's goal in one tier-appropriate line (the objective banner).
    pub text: &'static str,
    /// A target area to outline (x0, y0, x1, y1), such as the gas pocket.
    pub area: Option<[i32; 4]>,
    /// Where to label the crystal battery (top centre), in room 3.
    pub battery: Option<[i32; 2]>,
    /// The goal in ordered steps, each ticked when done (rooms 3 and 4).
    pub steps: Option<Vec<Step>>,
}

/// One step of a room's goal, as the objective banner lists it.
#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub text: &'static str,
    pub done: bool,
}

/// The torch as the renderer needs it.
#[derive(Debug, Clone, Serialize)]
pub struct TorchView {
    pub x: f32,
    pub y: f32,
    pub lit: bool,
    pub carried: bool,
}

impl Game {
    // ------------------------------------------------------------ building

    /// Clears features' footprints down to flat ground at `GROUND_Y`.
    fn flatten(&mut self, x0: i32, x1: i32) {
        let g = &mut self.grid;
        g.fill_rect(x0, 100, x1, GROUND_Y, Species::Air);
        g.fill_rect(x0, GROUND_Y, x1, GROUND_Y + 2, Species::Stone);
    }

    fn build_gate(&mut self, k: usize) {
        let x = GATE_X[k];
        self.flatten(x - 2, x + GATE_W + 2);
        let g = &mut self.grid;
        g.fill_rect(x, 0, x + GATE_W, DOOR_TOP, Species::Stone);
        g.fill_rect(x, DOOR_TOP, x + GATE_W, GROUND_Y, Species::Door);
    }

    /// One line per tier: Sprout (simple, read aloud), Seedling (eight words
    /// or fewer) and Apprentice (the full explanation).
    pub(super) fn line(
        &self,
        sprout: &'static str,
        seedling: &'static str,
        apprentice: &'static str,
    ) -> &'static str {
        match self.tier {
            Tier::Sprout => sprout,
            Tier::Seedling => seedling,
            Tier::Apprentice => apprentice,
        }
    }

    /// Room `k`'s goal in one line for the objective banner: a few spoken
    /// words for Sprout, eight words or fewer for Seedling, the full line
    /// for Apprentice.
    pub fn objective_line(&self, k: usize) -> &'static str {
        match k {
            0 => self.line(
                "Put out the fire!",
                "Put out the burning hedge.",
                "Put out the burning hedge: cast H2O or CO2 at the flames.",
            ),
            1 => self.line(
                "Melt the ice!",
                "Melt the ice wall with heat.",
                "Melt the ice wall: cast 2H2 + O2 -> 2H2O at it, or carry a lit torch.",
            ),
            2 => self.line(
                "Make bubbles, then boom!",
                "Split water, trap the gas, light it.",
                "Trap the gas under the stone roof, then light it.",
            ),
            _ => self.line(
                "Fire out, then light the bowl!",
                "Put out the hedge, light the brazier.",
                "Put out the hedge with H2O or CO2, then light the brazier by the door.",
            ),
        }
    }

    /// Opens gate `k` (0-based) with a rumble. Returns false if already open.
    pub(super) fn open_gate(&mut self, k: usize) -> bool {
        if self.rooms.gates_open[k] {
            return false;
        }
        self.rooms.gates_open[k] = true;
        let x = GATE_X[k];
        for y in DOOR_TOP..GROUND_Y {
            for cx in x..x + GATE_W {
                if self.grid.get(cx, y) == Species::Door {
                    self.grid.set(cx, y, Species::Air, AMBIENT_C);
                }
            }
        }
        let (fx, fy) = ((x + GATE_W / 2) as f32, ((DOOR_TOP + GROUND_Y) / 2) as f32);
        self.emit_at("door", fx, fy, 1.0);
        if let Some(o) = self.objectives.get_mut(k) {
            o.done = true;
        }
        true
    }

    /// The whole trial: four rooms of 320 cells, left to right.
    pub(super) fn build_level(&mut self) {
        self.grid.wood_flame_chance = ROOM_WOOD_FLAME_CHANCE;
        let g = &mut self.grid;
        g.fill_rect(0, GROUND_Y, WORLD_W as i32, HEIGHT as i32, Species::Stone);
        // Gentle bumps (at most one cell per column, so walking steps over them).
        for x in 0..WORLD_W as i32 {
            let wave = (x as f32 * 0.07).sin() * 1.2 + (x as f32 * 0.031).cos() * 1.2;
            let bump = wave.round().clamp(0.0, 2.0) as i32;
            g.fill_rect(x, GROUND_Y - bump, x + 1, GROUND_Y, Species::Stone);
        }

        // ---- Room 1: Splash Garden.
        // The stream: a pool dug into the ground, for topping up water.
        g.fill_rect(30, 140, 96, 162, Species::Air);
        g.fill_rect(30, 162, 96, 166, Species::Stone);
        g.fill_rect(30, 150, 96, 162, Species::Water);
        // A small wood of cool trees (scenery you walk in front of).
        for (tx, h) in [(112, 14), (124, 18), (137, 12), (150, 16), (163, 13)] {
            let top = GROUND_Y - 1 - h;
            g.fill_rect(tx, top, tx + 2, GROUND_Y, Species::Wood);
            g.fill_rect(tx - 4, top - 6, tx + 6, top + 1, Species::Wood);
        }
        // A low stone wall between the trees and the hedge.
        g.fill_rect(200, 134, 204, GROUND_Y, Species::Stone);
        self.flatten(226, 300);
        let g = &mut self.grid;
        // Stone curbs either side of the hedge: the fire's fuel is the hedge only.
        // Two cells tall, so walking steps over them without a jump.
        g.fill_rect(228, GROUND_Y - 2, 232, GROUND_Y, Species::Stone);
        g.fill_rect(264, GROUND_Y - 2, 268, GROUND_Y, Species::Stone);
        let h = HEDGE1;
        g.fill_rect(h.x0, h.y0, h.x1, h.y1, Species::Wood);
        // Its left end is already burning.
        for (x, y) in h.cells() {
            if x < h.x0 + 5 {
                let i = g.idx(x as usize, y as usize);
                g.temp[i] = 650.0;
            }
        }
        self.build_gate(0);

        // ---- Room 2: Frozen Gate.
        self.flatten(TORCH_X - 6, TORCH_X + 6);
        self.flatten(ICE_ROCK.x0 - 4, PIT.x1 + 4);
        let g = &mut self.grid;
        g.fill_rect(
            ICE_ROCK.x0,
            ICE_ROCK.y0,
            ICE_ROCK.x1,
            ICE_ROCK.y1,
            Species::Stone,
        );
        for (x, y) in ICE.cells() {
            g.set(x, y, Species::Ice, -5.0);
        }
        g.fill_rect(PIT.x0, PIT.y0, PIT.x1, PIT.y1, Species::Air);
        // Stone stairs down each side of the pit, 2-cell risers, so holding
        // walk alone climbs out (the step-up handles 2 cells; no jump needed).
        let rises = (PIT.y1 - PIT.y0) / PIT_RISER;
        for k in 1..rises {
            let top = PIT.y0 + k * PIT_RISER;
            let w = k * PIT_TREAD;
            g.fill_rect(PIT.x0, top, PIT.x0 + w, PIT.y1, Species::Stone);
            g.fill_rect(PIT.x1 - w, top, PIT.x1, PIT.y1, Species::Stone);
        }
        self.build_gate(1);

        // ---- Room 3: Crystal Lab.
        self.flatten(CRYSTAL.x0 - 4, ROOF.x1 + 4);
        let g = &mut self.grid;
        g.fill_rect(
            CRYSTAL.x0,
            CRYSTAL.y0,
            CRYSTAL.x1,
            CRYSTAL.y1,
            Species::Crystal,
        );
        g.fill_rect(POOL.x0, POOL.y0, POOL.x1, POOL.y1, Species::Water);
        g.fill_rect(ROOF.x0, ROOF.y0, ROOF.x1, ROOF.y1, Species::Stone);
        g.fill_rect(ROOF.x0, ROOF.y1, ROOF.x0 + 4, TRAP.y1, Species::Stone);
        g.fill_rect(ROOF.x1 - 4, ROOF.y1, ROOF.x1, TRAP.y1, Species::Stone);
        self.build_gate(2);

        // ---- Room 4: Marsh Door.
        self.flatten(MARSH.x0 - 2, MARSH.x1 + 2);
        self.flatten(ALCOVE_ROOF.x0 - 2, GATE_X[3]);
        let g = &mut self.grid;
        g.fill_rect(MARSH.x0, MARSH.y0, MARSH.x1, MARSH.y1, Species::Water);
        let c = MARSH_CAP;
        g.fill_rect(c.x0, c.y0, c.x1, c.y1, Species::Stone);
        g.fill_rect(c.x0, c.y1, MARSH_POCKET.x0, MARSH_POCKET.y1, Species::Stone);
        g.fill_rect(MARSH_POCKET.x1, c.y1, c.x1, MARSH_POCKET.y1, Species::Stone);
        g.fill_rect(
            ALCOVE_ROOF.x0,
            ALCOVE_ROOF.y0,
            ALCOVE_ROOF.x1,
            ALCOVE_ROOF.y1,
            Species::Stone,
        );
        let h = HEDGE2;
        g.fill_rect(h.x0, h.y0, h.x1, h.y1, Species::Wood);
        for (x, y) in h.cells() {
            if x < h.x0 + 6 {
                let i = g.idx(x as usize, y as usize);
                g.temp[i] = 650.0;
            }
        }
        // The brazier: a stone bowl, open at the top.
        let b = BRAZIER;
        g.fill_rect(b.x0, b.y1 - 2, b.x1, b.y1, Species::Stone);
        g.fill_rect(b.x0, b.y0, b.x0 + 2, b.y1, Species::Stone);
        g.fill_rect(b.x1 - 2, b.y0, b.x1, b.y1, Species::Stone);
        self.build_gate(3);

        self.rooms.ice_start = ICE.cells().count() as u32;
        self.rooms.ice_left = self.rooms.ice_start;

        self.objectives = ["fire", "ice", "gas", "brazier"]
            .iter()
            .enumerate()
            .map(|(k, id)| Objective {
                id,
                text: self.objective_line(k).into(),
                done: false,
            })
            .collect();
        let intro = self.line(
            "Fire! Tap the water drop!",
            "Fire! Cast H2O at the flames.",
            "Fire in the garden. Your suit is full of water: cast H2O at the flames.",
        );
        self.say(intro);
        self.rooms.entered[0] = true;
    }

    // -------------------------------------------------------------- update

    /// The room the player is standing in, 0-based.
    pub fn room(&self) -> usize {
        (((self.player.x + PLAYER_W / 2.0) as usize) / ROOM_W).min(ROOMS - 1)
    }

    fn count_in(&self, r: Rect, s: Species) -> u32 {
        r.cells().filter(|&(x, y)| self.grid.get(x, y) == s).count() as u32
    }

    fn update_hedge(&mut self, k: usize) {
        let h = self.rooms.hedges[k].clone();
        let zone = h.zone();
        let mut flames = 0;
        let mut exposed = 0;
        let mut hot = 0;
        for (x, y) in zone.cells() {
            match self.grid.get(x, y) {
                Species::Flame => flames += 1,
                Species::Wood if self.grid.temp_at(x, y) >= WOOD_IGNITION_C => {
                    hot += 1;
                    if [(0, 1), (0, -1), (1, 0), (-1, 0)]
                        .iter()
                        .any(|(dx, dy)| self.grid.get(x + dx, y + dy).oxidiser_strength() > 0.0)
                    {
                        exposed += 1;
                    }
                }
                _ => {}
            }
        }
        let burning = flames + hot;
        let out = flames == 0 && exposed == 0;
        let h = &mut self.rooms.hedges[k];
        h.burning = burning;
        h.peak = h.peak.max(burning);
        if !h.out && out && h.hits == 0 && !h.said_self_out {
            h.said_self_out = true;
            let line = self.line(
                "It burned out. Splash it!",
                "It burned out by itself. Cool the embers.",
                "It burned out by itself. Cast water on the embers to cool them, and the gate will open.",
            );
            self.say(line);
        }
        let h = &mut self.rooms.hedges[k];
        h.out = out || h.crumbled;
    }

    /// Hedge 2 after the player's casts: out for `HEDGE_OUT_TICKS` in a row
    /// and it falls to ash (no fuel left, so nothing can relight it and heat
    /// spells fly through); burning again before that is explained.
    fn settle_hedge_2(&mut self) {
        let h = &mut self.rooms.hedges[1];
        if h.crumbled || h.hits == 0 {
            return;
        }
        if h.out {
            h.out_ticks += 1;
        } else {
            let relit = h.out_ticks >= RELIGHT_AFTER_TICKS;
            h.out_ticks = 0;
            if relit {
                let line = self.line(
                    "It lit again! Splash more!",
                    "Hot charcoal re-ignited. Cool it more.",
                    "Hot charcoal re-ignited: it was still above ignition temperature with oxygen around. Cool it with more water or CO2.",
                );
                self.say(line);
            }
            return;
        }
        if h.out_ticks >= HEDGE_OUT_TICKS {
            self.crumble_hedge(1);
            self.rooms.hedges[1].crumbled = true;
            let line = self.line(
                "Hedge out! Walk through!",
                "Hedge out! Walk through, light the brazier.",
                "Hedge out! It crumbled to ash, so walk through and light the brazier.",
            );
            self.say(line);
        }
    }

    fn enter_rooms(&mut self) {
        let room = self.room();
        if self.rooms.entered[room] {
            return;
        }
        self.rooms.entered[room] = true;
        let line = match room {
            1 => self.line(
                "Ice! Boom melts ice!",
                "Ice wall! Heat melts ice. Balance the boom.",
                "An ice wall. Heat melts ice: cast boom (2H2 + O2 -> 2H2O) at it, or light the torch and carry it there.",
            ),
            2 => self.line(
                "Splash the pool! Make bubbles!",
                "Split water, trap the gas, then light it.",
                "The crystal pool. Electricity splits water into two hydrogen for every oxygen. Trap the gas under the stone roof, then light it.",
            ),
            3 => self.line(
                "Fire! Then light the bowl!",
                "Douse hedge, light brazier. Hold G in bubbles.",
                "The marsh bubbles methane: hold G in the bubbles to gather it. Put out the hedge with water or carbon dioxide, then light the brazier.",
            ),
            _ => return,
        };
        self.say(line);
    }

    /// The gas cell just above the marsh surface at a vent, if any.
    fn vent_mouth(&self, x: i32) -> Option<(i32, i32)> {
        (90..GROUND_Y + 8)
            .rev()
            .find(|&y| {
                self.grid.get(x, y).is_gas() && self.grid.get(x, y + 1).phase() == Phase::Liquid
            })
            .map(|y| (x, y))
    }

    /// Methane seeps up from the marsh vents while room 4 is being simulated.
    /// Once something hot lights a vent it burns steadily at the surface, like
    /// real natural-gas seeps that stay alight for years: CH4 + 2O2 -> CO2 + 2H2O.
    fn vents_tick(&mut self) {
        let vx = self.view_x.round() as i32;
        let active = vx + ACTIVE_AHEAD as i32 > MARSH.x0 && vx - (ACTIVE_BEHIND as i32) < MARSH.x1;
        if !active {
            return;
        }
        if self.ticks.is_multiple_of(60) {
            // Only methane near the vents counts: gas that wandered off (up
            // to the sky, or into the alcove) no longer stops them.
            self.rooms.ch4_loose = self.count_in(MARSH_AIR, Species::CH4);
        }
        for (k, &vent) in VENTS.iter().enumerate() {
            let Some((x, y)) = self.vent_mouth(vent) else {
                continue;
            };
            if !self.rooms.vent_lit[k] && self.ticks >= self.rooms.vent_wet_until[k] {
                let hot = (-VENT_LIGHT_R..=VENT_LIGHT_R).any(|dy| {
                    (-VENT_LIGHT_R..=VENT_LIGHT_R).any(|dx| {
                        let (cx, cy) = (x + dx, y + dy);
                        let s = self.grid.get(cx, cy);
                        s == Species::Flame
                            || (s.is_gas() && self.grid.temp_at(cx, cy) >= CH4_IGNITION_C)
                    })
                });
                if hot {
                    self.rooms.vent_lit[k] = true;
                    // No Grimoire page: pages come only from spells the
                    // player casts (CH4 + 2O2 -> CO2 + 2H2O, typed, finds it).
                    let line = self.line(
                        "Bubbles on fire! Stand close!",
                        "Methane burns into CO2. Stand close.",
                        "The methane vent caught fire! Burning methane makes carbon dioxide: stand close and your suit will catch it.",
                    );
                    self.say(line);
                }
            }
            let lit = self.rooms.vent_lit[k];
            let period = if lit { VENT_LIT_TICKS } else { VENT_TICKS };
            if !(self.ticks + k as u64 * 7).is_multiple_of(period) {
                continue;
            }
            if !lit {
                // Big, visible bubbles where the methane comes up (they keep
                // marking the spot even while the pocket is full).
                self.emit_at("seep", x as f32, y as f32, 1.0);
            }
            if !lit && self.rooms.ch4_loose as usize >= VENT_MAX_CH4 {
                continue;
            }
            if self.grid.get(x, y) == Species::Air {
                // A lit vent's methane comes out into its own flame, hot enough to burn.
                let t = if lit { CH4_IGNITION_C + 200.0 } else { 18.0 };
                self.grid.set(x, y, Species::CH4, t);
                if lit && self.grid.get(x, y - 1) == Species::Air {
                    self.grid.spawn_flame(x, y - 1, 4, false, FLAME_C_IN_AIR);
                }
            }
        }
    }

    /// Water landing on a lit vent puts it out.
    pub(super) fn douse_vents(&mut self, at: (i32, i32)) {
        for (k, &vent) in VENTS.iter().enumerate() {
            if let Some((x, y)) = self.vent_mouth(vent) {
                if (at.0 - x).abs() <= 8 && (at.1 - y).abs() <= 8 {
                    self.rooms.vent_lit[k] = false;
                    // Wet and cooled: the leftover heat can't relight it at once.
                    self.rooms.vent_wet_until[k] = self.ticks + VENT_WET_TICKS;
                }
            }
        }
    }

    /// The suit's CO2 scrubber pulls in fresh (hot) carbon dioxide from a
    /// flame near the hand, so a methane fire can be captured for the cloud
    /// spell. Cool CO2 is left alone unless the player gathers it.
    fn capture_tick(&mut self) {
        if self.jar("CO2") >= Self::jar_cap("CO2") {
            return;
        }
        let (hx, hy) = self.hand();
        let mut picked = Vec::new();
        Self::rings(hx as i32, hy as i32, CAPTURE_RADIUS, |x, y| {
            if self.grid.get(x, y) == Species::CO2
                && self.grid.temp_at(x, y) > 60.0
                && self.grid.in_bounds(x, y)
            {
                picked.push((x, y));
            }
            picked.len() >= 3
        });
        for (x, y) in picked {
            self.grid.set(x, y, Species::Air, AMBIENT_C);
            self.rooms.capture_cells += 1;
            if self.rooms.capture_cells >= GAS_CELLS_PER_MOL {
                self.rooms.capture_cells = 0;
                self.gain("CO2", 1, "capture", (x as f32, y as f32));
            }
        }
    }

    /// Where the torch's flame is.
    pub fn torch_head(&self) -> (i32, i32) {
        let t = &self.rooms.torch;
        if t.carried {
            let (hx, _) = self.hand();
            (hx as i32, self.player.y as i32 - 1)
        } else {
            (t.x as i32, t.y as i32 - TORCH_H)
        }
    }

    fn torch_tick(&mut self, gather: bool) {
        let (tx, ty) = self.torch_head();
        let near_heat = |g: &Grid, r: i32| {
            (-r..=r).any(|dy| {
                (-r..=r).any(|dx| {
                    let (x, y) = (tx + dx, ty + dy);
                    let s = g.get(x, y);
                    s == Species::Flame
                        || (s != Species::Stone && g.in_bounds(x, y) && g.temp_at(x, y) >= 300.0)
                })
            })
        };
        if !self.rooms.torch.lit && near_heat(&self.grid, 3) {
            self.rooms.torch.lit = true;
            self.torch_lit = true;
            // The torch is wood burning: carbon + oxygen -> carbon dioxide
            // (wood simplified to carbon, as everywhere in the game). It finds
            // no Grimoire page: only spells the player casts do.
            let line = if self.rooms.torch.carried {
                "Your torch caught fire."
            } else {
                self.line(
                    "Torch lit! Walk into it!",
                    "Torch lit! Walk into it to carry it.",
                    "The torch is lit! Walk into it to pick it up.",
                )
            };
            self.say(line);
        }
        let p = &self.player;
        let touching = (p.x + PLAYER_W / 2.0 - self.rooms.torch.x).abs() < 5.0
            && p.y < self.rooms.torch.y
            && p.y + PLAYER_H > self.rooms.torch.y - TORCH_H as f32;
        if !self.rooms.torch.carried && touching && (self.rooms.torch.lit || gather) {
            self.rooms.torch.carried = true;
            let line = self.line(
                "Got the torch! Go to ice!",
                "Got the torch. Hold it to the ice.",
                "You picked up the torch. Hold it against something to heat it.",
            );
            self.say(line);
        }
        if !self.rooms.torch.carried {
            return;
        }
        self.rooms.torch.x = self.player.x + PLAYER_W / 2.0;
        self.rooms.torch.y = self.player.y + PLAYER_H;
        let (tx, ty) = self.torch_head();
        // Dunked: the player went under water, torch and all. (Meltwater
        // dripping past the flame doesn't put it out.)
        let head_cell = self.grid.get(
            (self.player.x + PLAYER_W / 2.0) as i32,
            self.player.y as i32,
        );
        if head_cell.phase() == Phase::Liquid {
            self.rooms.torch.wet = self.rooms.torch.wet.saturating_add(1);
        } else {
            self.rooms.torch.wet = 0;
        }
        if self.rooms.torch.lit && self.rooms.torch.wet >= DUNK_TICKS {
            self.rooms.torch.lit = false;
            self.torch_lit = false;
            self.emit_at("hiss", tx as f32, ty as f32, 1.0);
            let line = self.line(
                "Oh no! Torch went out!",
                "Water put the torch out. Relight it.",
                "Your torch went out in the water. Light it again with fire.",
            );
            self.say(line);
            return;
        }
        if !self.rooms.torch.lit {
            return;
        }
        // Holding the flame against things: ice melts, wood and fuel gases heat up.
        let facing = self.player.facing;
        let front = if facing > 0 {
            self.player.x + PLAYER_W
        } else {
            self.player.x - 1.0
        } as i32;
        // The flame reaches the ice in front of you, above you and under your feet.
        let (y0, y1) = (
            self.player.y as i32 - 8,
            (self.player.y + PLAYER_H) as i32 + 5,
        );
        for y in y0..y1 {
            for d in -(PLAYER_W as i32)..4 {
                let x = front + d * facing;
                if self.grid.get(x, y) == Species::Ice && self.grid.in_bounds(x, y) {
                    let i = self.grid.idx(x as usize, y as usize);
                    self.grid.temp[i] = self.grid.temp[i].max(40.0);
                    if ICE.contains(x, y) {
                        self.rooms.ice_heated = true;
                    }
                }
            }
        }
        for dy in -2..=2 {
            for dx in -2..=2 {
                let (x, y) = (tx + dx, ty + dy);
                if !self.grid.in_bounds(x, y) {
                    continue;
                }
                let i = self.grid.idx(x as usize, y as usize);
                match self.grid.cells[i] {
                    Species::H2 | Species::CH4 => self.grid.temp[i] = self.grid.temp[i].max(650.0),
                    Species::Wood => self.grid.temp[i] = self.grid.temp[i].max(350.0),
                    Species::Air => self.grid.temp[i] = self.grid.temp[i].max(320.0),
                    _ => {}
                }
            }
        }
    }

    /// Checks every room's goal; opens gates whose goal is true.
    pub(super) fn update_rooms(&mut self, gather: bool) {
        self.enter_rooms();
        self.vents_tick();
        self.capture_tick();
        self.torch_tick(gather);

        // Room 1: the hedge is out, and the player's water helped.
        self.smother_tick(0);
        self.update_hedge(0);
        let h = &self.rooms.hedges[0];
        if h.out && h.hits > 0 && self.open_gate(0) {
            self.crumble_hedge(0);
            self.rooms.hedges[0].crumbled = true;
            let h = &self.rooms.hedges[0];
            let line = if h.co2_hits == 0 {
                self.line(
                    "Fire out! Gate open!",
                    "Fire out! Cooling took its heat. Gate open.",
                    "The fire is out. Fire needs fuel, oxygen and heat: your water took away the heat. The gate is open.",
                )
            } else if h.co2_hits == h.hits {
                self.line(
                    "Fire out! Gate open!",
                    "Fire out! CO2 took its oxygen. Gate open.",
                    "The fire is out. Fire needs fuel, oxygen and heat: CO2 is denser than air, so it sank over the flames and took away the oxygen. The gate is open.",
                )
            } else {
                self.line(
                    "Fire out! Gate open!",
                    "Fire out! No heat, no oxygen. Gate open.",
                    "The fire is out. Fire needs fuel, oxygen and heat: water took away the heat and CO2, denser than air, took away the oxygen. The gate is open.",
                )
            };
            self.say(line);
        }

        // Room 2: at least 90% of the ice is gone, melted by the player's heat.
        self.rooms.ice_left = self.count_in(ICE, Species::Ice);
        if self.rooms.ice_heated
            && self.rooms.ice_left * 10 <= self.rooms.ice_start
            && self.open_gate(1)
        {
            // The last loose chunks (already near 0 C) melt too, so a stray
            // block of ice can never wall the player in under the rock.
            for (x, y) in ICE.cells() {
                if self.grid.get(x, y) == Species::Ice {
                    self.grid.set(x, y, Species::Water, 1.0);
                }
            }
            self.rooms.ice_left = 0;
            let line = self.line(
                "Ice melted! Gate open!",
                "Heat melted the ice into water. Gate open!",
                "The ice melted into water. Heat changes ice to water, and water to steam. The gate is open.",
            );
            self.say(line);
        }

        // Room 3: hydrogen collected under the overhang, then burned.
        let h2 = self.count_in(TRAP, Species::H2);
        self.rooms.trap_h2 = h2;
        self.rooms.trap_peak = self.rooms.trap_peak.max(h2);
        if !self.rooms.gates_open[2] && self.rooms.trap_peak >= TRAP_MIN_H2 {
            let blast = TRAP.grow(4).cells().any(|(x, y)| {
                !self.grid.get(x, y).is_solid() && self.grid.temp_at(x, y) >= H2_BLAST_C
            });
            if blast && self.open_gate(2) {
                let line = self.line(
                    "Bang! The door opened!",
                    "Bang! Trapped hydrogen burned. The door opened!",
                    "Bang! The trapped hydrogen burned with the oxygen and blew the door open. 2H2 + O2 -> 2H2O.",
                );
                self.say(line);
            }
        }
        if self.rooms.trap_h2 == 0 && !self.rooms.gates_open[2] {
            // Gas that drifted away no longer counts as trapped.
            self.rooms.trap_peak = 0;
            self.rooms.boom_prompted = false;
        }
        if self.trap_ready() && !self.rooms.boom_prompted && !self.rooms.gates_open[2] {
            self.rooms.boom_prompted = true;
            let line = self.line(
                "Now Boom the gas!",
                "Gas trapped! Now boom the gas!",
                "The gas is trapped. Now light it: 2H2 + O2 -> 2H2O.",
            );
            self.say(line);
        }

        // Room 4: the hedge (for the meter) and the brazier.
        self.smother_tick(1);
        self.update_hedge(1);
        self.settle_hedge_2();
        if !self.rooms.brazier_lit {
            let zone = Rect::new(BRAZIER.x0, BRAZIER.y0 - 6, BRAZIER.x1, BRAZIER.y1 - 2);
            let hot = zone.cells().any(|(x, y)| {
                let s = self.grid.get(x, y);
                s == Species::Flame || (!s.is_solid() && self.grid.temp_at(x, y) >= 300.0)
            });
            let (bx, by) = zone.centre();
            let cast_here = self.rooms.last_heat.is_some_and(|(t, x, y)| {
                self.ticks.saturating_sub(t) <= CAUSE_TICKS
                    && ((x - bx) as f32).hypot((y - by) as f32) <= BRAZIER_REACH
            });
            let (tx, ty) = self.torch_head();
            let torch_here = self.rooms.torch.carried
                && self.rooms.torch.lit
                && ((tx - bx) as f32).hypot((ty - by) as f32) <= 8.0;
            if hot && (cast_here || torch_here) {
                self.rooms.brazier_lit = true;
                self.open_gate(3);
                let line = self.line(
                    "Bowl lit! You did it!",
                    "Brazier lit! You broke through!",
                    "The brazier is lit and the last door is open. Trial complete: you broke through to the next realm!",
                );
                self.say(line);
            }
        }
        if self.rooms.brazier_lit && self.ticks.is_multiple_of(3) {
            // The brazier keeps burning (charcoal: drawn as flame only).
            let w = (BRAZIER.x1 - BRAZIER.x0 - 4) as u32;
            let x = BRAZIER.x0 + 2 + (self.grid.rand_u32() % w) as i32;
            let y = BRAZIER.y1 - 3;
            if self.grid.get(x, y) == Species::Air {
                self.grid.spawn_flame(x, y, 8, false, WOOD_FIRE_C);
            }
        }
    }

    /// A put-out hedge falls to ash (drawn as a dust puff) so its scorched
    /// stump can't block the path. Ash itself is not simulated: it is too
    /// little mass to matter, so the cells become air at the air's temperature.
    fn crumble_hedge(&mut self, k: usize) {
        let r = self.rooms.hedges[k].rect;
        let mut n = 0;
        for (x, y) in r.cells() {
            if matches!(self.grid.get(x, y), Species::Wood | Species::Flame) {
                self.grid.set(x, y, Species::Air, AMBIENT_C);
                n += 1;
            }
        }
        if n > 0 {
            let (cx, _) = r.centre();
            self.emit_at("land", cx as f32, r.y1 as f32, 5.0);
        }
    }

    /// Some of the ice wall melted but not enough to pass: say how much is
    /// left and to go again (the gate opens on its own when it's enough).
    pub(super) fn ice_progress_line(&mut self, spell: &str) {
        if self.rooms.gates_open[1] || self.rooms.ice_start == 0 {
            return;
        }
        let left = self.count_in(ICE, Species::Ice);
        if left * 10 <= self.rooms.ice_start {
            return;
        }
        let pct = (100 * left / self.rooms.ice_start).max(1);
        let line = match self.tier {
            Tier::Sprout => "Half melted! Again!".to_string(),
            Tier::Seedling => format!("{pct}% of the ice left. Boom again!"),
            Tier::Apprentice => format!(
                "{pct}% of the ice is left. Melting ice takes 6.01 kJ per mole (latent heat of fusion, CRC), so a bigger {spell} melts more: balance it, or cast again."
            ),
        };
        self.say(line);
    }

    /// A CO2 cloud landed at `at`: any hedge it hit is blanketed, so its
    /// flames die and its wood cools for `SMOTHER_TICKS`.
    pub(super) fn smother(&mut self, at: (i32, i32)) {
        let until = self.ticks + SMOTHER_TICKS;
        for h in &mut self.rooms.hedges {
            if h.zone().grow(8).contains(at.0, at.1) {
                h.smother_until = h.smother_until.max(until);
            }
        }
    }

    /// A blanketed hedge: flames in its zone go out (their fuel gas has no
    /// oxygen) and hot wood cools toward the air's temperature.
    fn smother_tick(&mut self, k: usize) {
        if self.ticks >= self.rooms.hedges[k].smother_until {
            return;
        }
        let zone = self.rooms.hedges[k].zone();
        for (x, y) in zone.cells() {
            if !self.grid.in_bounds(x, y) {
                continue;
            }
            let i = self.grid.idx(x as usize, y as usize);
            match self.grid.cells[i] {
                Species::Flame => self.grid.set(x, y, Species::Air, AMBIENT_C),
                Species::Wood if self.grid.temp[i] > AMBIENT_C => {
                    let t = self.grid.temp[i];
                    self.grid.temp[i] = t - (t - AMBIENT_C) * SMOTHER_COOL;
                }
                _ => {}
            }
        }
    }

    /// A water (`co2` false) or CO2 cast landed at `at`: credit any hedge
    /// it hit, remembering which agent did it so feedback names the right
    /// side of the fire triangle.
    pub(super) fn note_douse(&mut self, at: (i32, i32), co2: bool) {
        for h in &mut self.rooms.hedges {
            if h.zone().grow(8).contains(at.0, at.1) {
                h.hits += 1;
                if co2 {
                    h.co2_hits += 1;
                }
            }
        }
    }

    /// Water soaks through foliage: starting where it lands, it cools every
    /// connected burning wood or flame cell, up to `budget` hot cells (cool
    /// wood it passes through costs nothing).
    pub(super) fn soak(&mut self, at: (i32, i32), budget: u32) -> u32 {
        const REACH: i32 = 30;
        // A stream that lands on the curb or the ground beside a hedge still splashes it.
        const SOAK_SPLASH: i32 = 8;
        let burnable = |s: Species| s == Species::Wood || s == Species::Flame;
        let mut seen = std::collections::BTreeSet::new();
        let mut queue = std::collections::VecDeque::new();
        for dy in -SOAK_SPLASH..=SOAK_SPLASH {
            for dx in -SOAK_SPLASH..=SOAK_SPLASH {
                let (x, y) = (at.0 + dx, at.1 + dy);
                if burnable(self.grid.get(x, y)) && self.grid.in_bounds(x, y) && seen.insert((x, y))
                {
                    queue.push_back((x, y));
                }
            }
        }
        let mut cooled = 0;
        while let Some((x, y)) = queue.pop_front() {
            if cooled >= budget {
                break;
            }
            let i = self.grid.idx(x as usize, y as usize);
            match self.grid.cells[i] {
                Species::Flame => {
                    self.grid.set(x, y, Species::Steam, 100.0);
                    cooled += 1;
                }
                Species::Wood => {
                    if self.grid.temp[i] > 100.0 {
                        cooled += 1;
                    }
                    self.grid.temp[i] = self.grid.temp[i].min(30.0);
                }
                _ => continue,
            }
            for (dx, dy) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (-1, 1),
                (1, -1),
                (-1, -1),
            ] {
                let (nx, ny) = (x + dx, y + dy);
                if (nx - at.0).abs() <= REACH
                    && (ny - at.1).abs() <= REACH
                    && self.grid.in_bounds(nx, ny)
                    && burnable(self.grid.get(nx, ny))
                    && seen.insert((nx, ny))
                {
                    queue.push_back((nx, ny));
                }
            }
        }
        cooled
    }

    /// Water landing in the crystal pool is split by the crystal's charge:
    /// two hydrogen bubbles for every oxygen bubble, rising from the water.
    /// Returns the number of splits (2H2O each).
    pub(super) fn pool_electrolysis(&mut self, at: (i32, i32), mol: u32) -> u32 {
        if !POOL.grow(3).contains(at.0, at.1) || mol == 0 {
            return 0;
        }
        let splits = mol.min((self.crystal_kj / self.split_kj) as u32);
        if splits == 0 {
            return 0;
        }
        // Water cells just under the impact become gas: per split, 2 H2 for 1 O2.
        let mut water: Vec<(i32, i32)> = vec![];
        Self::rings(at.0, POOL.y0 + 2, 10, |x, y| {
            if self.grid.get(x, y) == Species::Water && POOL.contains(x, y) {
                water.push((x, y));
            }
            water.len() as u32 >= splits * 3 * GAS_CELLS_PER_MOL
        });
        let mut made = 0;
        for chunk in water.chunks_exact(3 * GAS_CELLS_PER_MOL as usize) {
            for (k, &(x, y)) in chunk.iter().enumerate() {
                let s = if k % 3 == 2 { Species::O2 } else { Species::H2 };
                self.grid.set(x, y, s, 18.0);
            }
            made += 1;
        }
        if made > 0 {
            self.rooms.gas_made = true;
            self.crystal_kj = (self.crystal_kj - self.split_kj * made as f32).max(0.0);
            let (cx, cy) = CRYSTAL.centre();
            self.emit(Fx {
                kind: "arc",
                x: cx as f32,
                y: cy as f32,
                tx: at.0 as f32,
                ty: at.1 as f32,
                dh_kj: self.split_kj * made as f32,
                outcome: None,
                leftovers: Atoms::new(),
                reaction_id: Some("electrolysis_water".into()),
                id: 0,
                power: 1.0,
            });
        }
        made
    }

    /// Enough hydrogen is trapped under the overhang to light.
    pub fn trap_ready(&self) -> bool {
        self.rooms.trap_h2 >= TRAP_MIN_H2
    }

    /// How full the gas trap is, 0..=1, where 1 means ready to light (shown
    /// as a "gas 3/4" meter on the trap).
    pub fn trap_fill(&self) -> f32 {
        if self.rooms.gates_open[2] {
            return 1.0;
        }
        (self.rooms.trap_h2 as f32 / TRAP_MIN_H2 as f32).min(1.0)
    }

    /// The hydrogen and oxygen cells in view, for the renderer's glowing
    /// bubbles: flat `[x, y, kind]` groups in viewport cells, kind 1 for H2
    /// and 2 for O2, at most `max` groups. Presentation only.
    pub fn gas_cells(&self, view_x: i32, max: usize) -> Vec<f32> {
        let mut out = Vec::new();
        let x0 = view_x.max(0);
        let x1 = (view_x + VIEW_W as i32).min(WORLD_W as i32);
        'rows: for y in 0..HEIGHT as i32 {
            for x in x0..x1 {
                let kind = match self.grid.get(x, y) {
                    Species::H2 => 1.0,
                    Species::O2 => 2.0,
                    _ => continue,
                };
                if out.len() / 3 >= max {
                    break 'rows;
                }
                out.extend([(x - view_x) as f32, y as f32, kind]);
            }
        }
        out
    }

    /// The goals the HUD marks, with where to put each icon.
    pub fn goals(&self) -> Vec<Goal> {
        let (hx, _) = HEDGE1.centre();
        let (ix, _) = ICE.centre();
        let (gx, gy) = TRAP.centre();
        let (bx, _) = BRAZIER.centre();
        let r = &self.rooms;
        vec![
            Goal {
                room: 1,
                kind: "fire",
                x: hx,
                y: HEDGE1.y0 - 10,
                done: r.gates_open[0],
                meter: Some(r.hedges[0].meter()),
                need: "water",
                stand_x: HEDGE1_STAND_X,
                text: self.objective_line(0),
                area: None,
                battery: None,
                steps: None,
            },
            Goal {
                room: 2,
                kind: "ice",
                // On the ice itself (above it is solid rock), so pointing at
                // the marker points at the wall.
                x: ix,
                y: ICE.y0 + 6,
                done: r.gates_open[1],
                // "Ice left", 0..=1, like the hedges' fire meter.
                meter: Some(if r.ice_start == 0 {
                    0.0
                } else {
                    r.ice_left as f32 / r.ice_start as f32
                }),
                need: "boom",
                stand_x: 440,
                text: self.objective_line(1),
                area: None,
                battery: None,
                steps: None,
            },
            Goal {
                room: 3,
                kind: "gas",
                x: gx,
                y: gy,
                done: r.gates_open[2],
                // How full the gas trap is (0..=1; 1 = ready to light).
                meter: Some(self.trap_fill()),
                // Make the gas first, then light it.
                need: if r.trap_h2 >= TRAP_MIN_H2 {
                    "boom"
                } else {
                    "split"
                },
                stand_x: 686,
                text: self.objective_line(2),
                area: Some([TRAP.x0, TRAP.y0, TRAP.x1, TRAP.y1]),
                battery: Some([CRYSTAL.centre().0, CRYSTAL.y0]),
                steps: Some(vec![
                    Step {
                        text: self.line(
                            "Bubbles!",
                            "1. Split water.",
                            "1. Split water: 2H2O -> 2H2 + O2.",
                        ),
                        done: r.gas_made || self.trap_ready() || r.gates_open[2],
                    },
                    Step {
                        text: self.line(
                            "Gas trapped!",
                            "2. Trap the gas.",
                            "2. Let H2 and O2 collect under the roof.",
                        ),
                        done: self.trap_ready() || r.gates_open[2],
                    },
                    Step {
                        text: self.line(
                            "Boom the gas!",
                            "3. Light it with boom.",
                            "3. Light it: 2H2 + O2 -> 2H2O.",
                        ),
                        done: r.gates_open[2],
                    },
                ]),
            },
            Goal {
                room: 4,
                kind: "brazier",
                x: bx,
                y: BRAZIER.y0 - 10,
                done: r.brazier_lit,
                meter: Some(r.hedges[1].meter()),
                // The hedge blocks the way: put it out, then light the bowl.
                need: if r.hedges[1].out { "boom" } else { "water" },
                // In front of the hedge, then through it beside the brazier.
                stand_x: if r.hedges[1].out { 1218 } else { 1150 },
                text: self.objective_line(3),
                area: None,
                battery: None,
                steps: Some(vec![
                    Step {
                        text: self.line(
                            "Fire out!",
                            "1. Put out the hedge.",
                            "1. Put out the hedge: H2O or CO2.",
                        ),
                        done: r.hedges[1].out,
                    },
                    Step {
                        text: self.line(
                            "Light the bowl!",
                            "2. Light the brazier.",
                            "2. Light the brazier: 2H2 + O2 -> 2H2O.",
                        ),
                        done: r.brazier_lit,
                    },
                ]),
            },
        ]
    }

    /// Starts the room the player is in again: its cells and state come back
    /// from a freshly built level with the same seed, earlier rooms stay
    /// solved, the player stands at the room's entrance and the suit gets
    /// its starter charge back. Returns the room (0-based).
    pub fn restart_room(&mut self) -> usize {
        let k = self.room();
        let fresh = Game::new(self.tier, self.seed);
        self.grid
            .copy_columns(&fresh.grid, k * ROOM_W, (k + 1) * ROOM_W);
        self.rooms.reset_room(k, &fresh.rooms);
        if let Some(o) = self.objectives.get_mut(k) {
            o.done = false;
        }
        self.bolts.clear();
        self.misses_in_a_row = 0;
        self.top_up_starter();
        let x = if k == 0 {
            SPAWN_X
        } else {
            (k * ROOM_W) as f32 + 12.0
        };
        let p = &mut self.player;
        (p.x, p.y, p.vx, p.vy) = (x, GROUND_Y as f32 - PLAYER_H - 4.0, 0.0, 0.0);
        (p.prev_x, p.prev_y, p.facing) = (p.x, p.y, 1);
        self.snap_camera();
        self.refresh_version();
        k
    }

    /// Starts the whole trial again from the same seed.
    pub fn restart(&mut self) {
        *self = Game::new(self.tier, self.seed);
    }

    /// After the trial: every gate open and the suit full, to experiment.
    pub fn free_play(&mut self) {
        for k in 0..ROOMS {
            self.open_gate(k);
        }
        let caps: Vec<(String, u32)> = self
            .jars
            .keys()
            .map(|f| (f.clone(), Self::jar_cap(f)))
            .collect();
        self.jars.extend(caps);
        self.refresh_version();
    }

    /// Gives back at least the charge a new trial starts with.
    fn top_up_starter(&mut self) {
        for (f, n) in [("H2O", Self::jar_cap("H2O")), ("H2", 4), ("O2", 2)] {
            let jar = self.jars.entry(f.to_string()).or_insert(0);
            *jar = (*jar).max(n);
        }
    }

    pub fn torch_view(&self) -> TorchView {
        let t = &self.rooms.torch;
        TorchView {
            x: t.x,
            y: t.y,
            lit: t.lit,
            carried: t.carried,
        }
    }

    /// Soft auto-aim targets for heat spells: ice, trapped hydrogen, the
    /// brazier. (Not the torch: lighting it is an optional second way, and a
    /// card tapped in room 2 must go to the ice.)
    pub(super) fn heat_target(&self, x: i32, y: i32) -> bool {
        let s = self.grid.get(x, y);
        if s == Species::Ice {
            return true;
        }
        if s == Species::H2 && TRAP.contains(x, y) {
            return true;
        }
        !self.rooms.brazier_lit
            && self.rooms.hedges[1].out
            && (x, y) == (BRAZIER.centre().0, BRAZIER.y1 - 4)
    }
}

impl Game {
    /// Where soft auto-aim sends a spell when nothing it acts on is in
    /// reach: the target of the room's unfinished goal (the ice, the gas
    /// pocket, the hedge or the brazier), so a tapped card always flies at
    /// the goal. None when the room's goal is done.
    pub(super) fn goal_aim_point(&self) -> Option<(i32, i32)> {
        let k = self.room();
        if self.goals().get(k).is_none_or(|g| g.done) {
            return None;
        }
        Some(match k {
            0 => HEDGE1.centre(),
            1 => {
                let (hx, hy) = self.hand();
                let d = |&(x, y): &(i32, i32)| (x as f32 - hx).powi(2) + (y as f32 - hy).powi(2);
                ICE.cells()
                    .filter(|&(x, y)| self.grid.get(x, y) == Species::Ice)
                    .min_by(|a, b| d(a).total_cmp(&d(b)))
                    .unwrap_or((ICE.x0, ICE.centre().1))
            }
            2 => TRAP.centre(),
            _ if self.rooms.hedges[1].out => (BRAZIER.centre().0, BRAZIER.y1 - 4),
            _ => HEDGE2.centre(),
        })
    }
}

/// How hot wood must be before it blocks the player.
pub(super) fn blocks_player(s: Species, t: f32) -> bool {
    if s == Species::Wood {
        t >= HOT_WOOD_BLOCKS_C
    } else {
        s.is_solid()
    }
}

#[cfg(test)]
pub(super) mod script {
    //! Helpers for scripted play-throughs: walking with real input and
    //! casting at targets, the way a player would.
    use super::*;

    pub const SEEDS: [u64; 5] = [1, 7, 42, 99, 2024];

    /// Puts the player at `x` in a later room, standing on the ground.
    pub fn teleport(g: &mut Game, x: f32) {
        g.player.x = x - PLAYER_W / 2.0;
        g.player.y = GROUND_Y as f32 - PLAYER_H - 4.0;
        g.snap_camera();
        settle(g, 20);
    }

    pub fn settle(g: &mut Game, ticks: u32) {
        for _ in 0..ticks {
            g.tick(0);
        }
    }

    /// Lets every spell in flight land.
    pub fn land(g: &mut Game) {
        for _ in 0..60 {
            if g.bolts_in_flight() == 0 {
                break;
            }
            g.tick(0);
        }
    }

    /// Walks right (or left) until the player's centre reaches `x`, hopping
    /// when blocked, like a player holding the arrow key and tapping jump.
    /// Returns false if it ran out of ticks.
    pub fn walk_to(g: &mut Game, x: f32, max_ticks: u32) -> bool {
        let mut stuck = 0;
        for _ in 0..max_ticks {
            let cx = g.player.x + PLAYER_W / 2.0;
            if (cx - x).abs() < 2.0 {
                // Stop and stand still.
                for _ in 0..20 {
                    g.tick(0);
                }
                return true;
            }
            let dir = if x > cx { INPUT_RIGHT } else { INPUT_LEFT };
            let before = g.player.x;
            let hop = if stuck > 2 && stuck % 24 < 14 {
                INPUT_JUMP
            } else {
                0
            };
            g.tick(dir | hop);
            if (g.player.x - before).abs() < 0.3 {
                stuck += 1;
            } else {
                stuck = 0;
            }
        }
        false
    }

    pub fn cast_at(g: &mut Game, spell: &str, at: (i32, i32)) -> CastOutcome {
        g.set_aim(at.0, at.1, true);
        let out = g.cast(spell);
        land(g);
        out
    }

    /// Casts water at a hedge's centre up to `max` times, 30 ticks apart.
    /// Returns how many casts it took for the fire to go out.
    pub fn douse(g: &mut Game, k: usize, max: u32) -> Option<u32> {
        for n in 1..=max {
            assert_eq!(
                cast_at(g, "water", g.rooms.hedges[k].rect.centre()).outcome,
                Outcome::Full
            );
            settle(g, 30);
            if g.rooms.hedges[k].out {
                return Some(n);
            }
        }
        None
    }

    /// Booms the nearest ice until gate 2 opens, waiting for the suit's
    /// electrolyser to refill when the gas runs out.
    pub fn melt_ice_with_booms(g: &mut Game, max_ticks: u32) -> u32 {
        let mut booms = 0;
        let start = g.ticks;
        while !g.rooms.gates_open[1] && g.ticks - start < max_ticks as u64 {
            let ice = ICE.cells().find(|&(x, y)| g.grid.get(x, y) == Species::Ice);
            let Some(target) = ice else {
                settle(g, 10);
                continue;
            };
            if g.jars["H2"] >= 2 && g.jars["O2"] >= 1 {
                assert_ne!(cast_at(g, "boom", target).outcome, Outcome::Fizzle);
                booms += 1;
                settle(g, 40);
            } else {
                settle(g, 20);
            }
        }
        booms
    }
}

#[cfg(test)]
mod tests {
    use super::script::*;
    use super::*;

    fn burning_in(g: &Game, r: Rect) -> usize {
        r.cells()
            .filter(|&(x, y)| {
                let s = g.grid.get(x, y);
                s == Species::Flame
                    || (s == Species::Wood && g.grid.temp_at(x, y) >= WOOD_IGNITION_C)
            })
            .count()
    }

    // ------------------------------------------------------------- room 1

    #[test]
    fn three_water_casts_at_the_hedge_put_it_out_and_open_gate_1() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Seedling, seed);
            settle(&mut g, 60);
            assert!(
                walk_to(&mut g, HEDGE1_STAND_X as f32, 2000),
                "seed {seed}: stuck at {}",
                g.player.x
            );
            let casts = douse(&mut g, 0, 3);
            assert!(
                casts.is_some(),
                "seed {seed}: still burning after 3 casts ({} cells)",
                burning_in(&g, g.rooms.hedges[0].zone())
            );
            settle(&mut g, 60);
            assert!(g.rooms.gates_open[0], "seed {seed}: gate 1 shut");
            assert_eq!(g.grid.get(GATE_X[0] + 2, GROUND_Y - 4), Species::Air);
            assert_eq!(g.goals()[0].meter, Some(0.0));
        }
    }

    #[test]
    fn a_fire_that_burns_out_alone_is_not_credited_until_the_player_cools_it() {
        let mut g = Game::new(Tier::Seedling, 42);
        settle(&mut g, 30);
        g.state();
        // Stand-in for the hedge burning out by itself: no flames, cold wood.
        for (x, y) in g.rooms.hedges[0].zone().cells() {
            match g.grid.get(x, y) {
                Species::Flame => g.grid.set(x, y, Species::Air, AMBIENT_C),
                Species::Wood => {
                    let i = g.grid.idx(x as usize, y as usize);
                    g.grid.temp[i] = AMBIENT_C;
                }
                _ => {}
            }
        }
        settle(&mut g, 5);
        assert!(g.rooms.hedges[0].out);
        assert!(!g.rooms.gates_open[0], "no credit without the player");
        let said = g.state().messages.join(" ");
        assert!(said.contains("burned out by itself"), "{said}");
        assert!(!said.contains("your water"), "{said}");
        // Water on the embers is the player's cause: now the gate opens.
        assert!(walk_to(&mut g, HEDGE1_STAND_X as f32, 2000));
        cast_at(&mut g, "water", HEDGE1.centre());
        settle(&mut g, 10);
        assert!(g.rooms.gates_open[0]);
    }

    #[test]
    fn the_fire_spreads_within_the_hedge_if_ignored() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Seedling, seed);
            settle(&mut g, 60);
            let early = burning_in(&g, HEDGE1);
            settle(&mut g, 2400);
            let later = burning_in(&g, HEDGE1);
            assert!(later > early, "seed {seed}: {early} -> {later}");
            assert!(!g.rooms.gates_open[0]);
        }
    }

    // ------------------------------------------------------------- room 2

    /// An Apprentice who points at the goal marker (as a naive player does)
    /// opens the ice gate too: the marker sits on the ice and pointing near
    /// any marker locks onto that goal's target.
    #[test]
    fn pointing_at_the_ice_marker_melts_the_ice() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Apprentice, seed);
            teleport(&mut g, 440.0);
            let marker = g.goals()[1].clone();
            assert!(ICE.contains(marker.x, marker.y), "marker on the ice");
            let start = g.ticks;
            let mut booms = 0;
            while !g.rooms.gates_open[1] && g.ticks - start < 3000 {
                if g.jars["H2"] >= 2 && g.jars["O2"] >= 1 {
                    // A little above the marker, on the rock: still locks on.
                    let out = cast_at(&mut g, "2H2 + O2 -> 2H2O", (marker.x, marker.y - 10));
                    assert_eq!(out.outcome, Outcome::Full);
                    booms += 1;
                    settle(&mut g, 40);
                } else {
                    settle(&mut g, 20);
                }
            }
            assert!(
                g.rooms.gates_open[1],
                "seed {seed}: {} ice left after {booms} booms",
                g.rooms.ice_left
            );
            assert!(booms <= 4, "seed {seed}: took {booms} booms");
        }
    }

    #[test]
    fn booms_at_the_ice_melt_it_and_open_gate_2() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Apprentice, seed);
            teleport(&mut g, 440.0);
            assert_eq!(g.rooms.ice_left, g.rooms.ice_start);
            g.take_fx();
            let booms = melt_ice_with_booms(&mut g, 3000);
            assert!(
                g.rooms.gates_open[1],
                "seed {seed}: {} of {} ice left after {booms} booms",
                g.rooms.ice_left, g.rooms.ice_start
            );
            assert!(g.rooms.ice_left * 10 <= g.rooms.ice_start);
            assert!(booms <= 4, "seed {seed}: took {booms} booms");
            assert!(
                g.take_fx().iter().any(|f| f.kind == "door"),
                "the gate rumbles"
            );
            // No stray chunk of ice is left to wall the player in.
            assert_eq!(g.count_in(ICE, Species::Ice), 0, "seed {seed}");
            assert!(
                walk_to(&mut g, 600.0, 3000),
                "seed {seed}: stuck at {}",
                g.player.x
            );
            // The meltwater runs off into the pit.
            settle(&mut g, 300);
            assert!(g.count_in(PIT, Species::Water) > 20, "seed {seed}: pit dry");
        }
    }

    #[test]
    fn lighting_the_torch_finds_no_page_the_player_did_not_cast() {
        let mut g = Game::new(Tier::Apprentice, 7);
        teleport(&mut g, TORCH_X as f32 - 20.0);
        g.take_fx();
        let head = g.torch_head();
        let out = cast_at(&mut g, "boom", head);
        settle(&mut g, 10);
        assert!(g.rooms.torch.lit);
        // The boom the player cast is theirs; the burning wood is not.
        assert_eq!(g.discovered, ["water_synthesis"]);
        let discovers: Vec<_> = g
            .take_fx()
            .into_iter()
            .filter(|f| f.kind == "discover")
            .collect();
        assert!(
            discovers.iter().all(|f| f.id == out.id),
            "a discover event not from the cast: {discovers:?}"
        );
    }

    #[test]
    fn a_lit_torch_held_to_the_ice_also_opens_gate_2() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Seedling, seed);
            teleport(&mut g, TORCH_X as f32 - 20.0);
            // Light it with a boom...
            let head = g.torch_head();
            cast_at(&mut g, "boom", head);
            settle(&mut g, 10);
            assert!(g.rooms.torch.lit, "seed {seed}: torch not lit");
            settle(&mut g, 60);
            // ...pick it up, and walk into the ice.
            assert!(walk_to(&mut g, TORCH_X as f32, 400));
            assert!(g.rooms.torch.carried, "seed {seed}: not carried");
            walk_to(&mut g, ICE.x1 as f32 + 4.0, 1500);
            settle(&mut g, 30);
            assert!(
                g.rooms.gates_open[1],
                "seed {seed}: {} of {} ice left; player at {}",
                g.rooms.ice_left, g.rooms.ice_start, g.player.x
            );
        }
    }

    #[test]
    fn ice_left_alone_does_not_open_gate_2() {
        let mut g = Game::new(Tier::Seedling, 42);
        teleport(&mut g, 440.0);
        settle(&mut g, 3000);
        assert!(!g.rooms.gates_open[1]);
        assert!(
            g.rooms.ice_left * 2 > g.rooms.ice_start,
            "{} left",
            g.rooms.ice_left
        );
    }

    // ------------------------------------------------------------- room 3

    #[test]
    fn water_in_the_crystal_pool_bubbles_two_hydrogen_per_oxygen() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Sprout, seed);
            teleport(&mut g, 690.0);
            let room3 = Rect::new(2 * ROOM_W as i32, 0, 3 * ROOM_W as i32, HEIGHT as i32);
            let kj = g.crystal_kj;
            g.set_aim(POOL.centre().0, POOL.y0, true);
            let out = g.cast("water");
            assert_eq!(out.outcome, Outcome::Full);
            for _ in 0..30 {
                g.tick(0);
                let (h, o) = (
                    g.count_in(room3, Species::H2),
                    g.count_in(room3, Species::O2),
                );
                assert_eq!(h, 2 * o, "seed {seed}: {h} H2, {o} O2");
                if g.bolts_in_flight() == 0 && h > 0 {
                    break;
                }
            }
            assert!(
                g.count_in(room3, Species::H2) >= 6,
                "seed {seed}: no bubbles"
            );
            assert!(g.crystal_kj < kj, "the crystal paid for the split");
            // The bubbles rise into the pocket under the overhang.
            settle(&mut g, 90);
            assert!(
                g.rooms.trap_h2 >= 3,
                "seed {seed}: {} trapped",
                g.rooms.trap_h2
            );
        }
    }

    #[test]
    fn split_then_boom_under_the_overhang_opens_gate_3() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Apprentice, seed);
            teleport(&mut g, 684.0);
            let out = cast_at(&mut g, "split", TRAP.centre());
            assert_eq!(out.outcome, Outcome::Full);
            let (h, o) = (out.products["H2"], out.products["O2"]);
            assert_eq!(h, 2 * o);
            settle(&mut g, 30);
            assert!(
                g.rooms.trap_h2 >= TRAP_MIN_H2,
                "seed {seed}: {} trapped",
                g.rooms.trap_h2
            );
            g.take_fx();
            assert_ne!(
                cast_at(&mut g, "boom", TRAP.centre()).outcome,
                Outcome::Fizzle
            );
            settle(&mut g, 30);
            assert!(g.rooms.gates_open[2], "seed {seed}: gate 3 shut");
            assert!(g.take_fx().iter().any(|f| f.kind == "door"));
        }
    }

    #[test]
    fn sprout_cards_with_no_aiming_split_and_light_the_gas() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Sprout, seed);
            teleport(&mut g, 686.0);
            assert_eq!(g.cast("split").outcome, Outcome::Full);
            land(&mut g);
            settle(&mut g, 30);
            assert_eq!(g.cast("boom").outcome, Outcome::Full);
            land(&mut g);
            settle(&mut g, 30);
            assert!(
                g.rooms.gates_open[2],
                "seed {seed}: auto-aim missed the gas"
            );
        }
    }

    #[test]
    fn room_3_goal_shows_trap_fill_and_three_ticked_steps() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            for seed in SEEDS {
                let mut g = Game::new(tier, seed);
                teleport(&mut g, 686.0);
                let goal = g.goals()[2].clone();
                assert_eq!(goal.meter, Some(0.0));
                let steps = goal.steps.expect("room 3 has steps");
                assert_eq!(steps.len(), 3);
                assert!(steps.iter().all(|s| !s.done));
                g.state();
                if tier == Tier::Apprentice {
                    // No auto-aim at Apprentice: point at the goal marker.
                    g.set_aim(goal.x, goal.y, true);
                }
                assert_eq!(g.cast("split").outcome, Outcome::Full);
                land(&mut g);
                let steps = g.goals()[2].steps.clone().unwrap();
                assert!(steps[0].done, "{tier:?} seed {seed}: split not ticked");
                settle(&mut g, 30);
                let goal = g.goals()[2].clone();
                let fill = goal.meter.unwrap();
                assert!(fill > 0.0 && fill <= 1.0, "{tier:?} seed {seed}: {fill}");
                assert!(g.trap_ready(), "{tier:?} seed {seed}: not ready");
                assert_eq!(fill, 1.0);
                let steps = goal.steps.unwrap();
                assert!(steps[1].done && !steps[2].done);
                assert_eq!(goal.need, "boom");
                let said = g.state().messages;
                assert!(
                    said.iter()
                        .any(|m| m.to_lowercase().contains("boom the gas")
                            || m.contains("Now light it")),
                    "{tier:?} seed {seed}: {said:?}"
                );
                if tier == Tier::Apprentice {
                    // No auto-aim at Apprentice: point at the goal marker.
                    g.set_aim(goal.x, goal.y, true);
                }
                assert_eq!(g.cast("boom").outcome, Outcome::Full);
                land(&mut g);
                settle(&mut g, 30);
                let goal = g.goals()[2].clone();
                assert!(
                    goal.done,
                    "{tier:?} seed {seed} aim {:?} h2 {} player {}",
                    g.aim, g.rooms.trap_h2, g.player.x
                );
                assert!(goal.steps.unwrap().iter().all(|s| s.done));
                // The prompt is said once, not every tick.
                let again = g.state().messages;
                assert!(!again.iter().any(|m| m.contains("Now Boom")));
            }
        }
    }

    /// Holds right only (never jump) for up to `max` ticks; returns the
    /// tick the player's centre reached `x`, or where it got stuck.
    fn hold_right_to(g: &mut Game, x: f32, max: u32) -> Result<u32, f32> {
        for t in 0..max {
            if g.player.x + PLAYER_W / 2.0 >= x {
                return Ok(t);
            }
            g.tick(INPUT_RIGHT);
        }
        Err(g.player.x + PLAYER_W / 2.0)
    }

    #[test]
    fn holding_right_from_room_2_reaches_the_crystal_with_no_jumps() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            for seed in SEEDS {
                let mut g = Game::new(tier, seed);
                g.open_gate(0);
                teleport(&mut g, 440.0);
                melt_ice_with_booms(&mut g, 3000);
                assert!(g.rooms.gates_open[1], "{tier:?} seed {seed}: ice");
                settle(&mut g, 120);
                // The meltwater fills the pit: walk in, wade, climb the lip.
                let reached = hold_right_to(&mut g, (CRYSTAL.x0 - 12) as f32, 600);
                assert!(
                    reached.is_ok(),
                    "{tier:?} seed {seed}: stuck at {reached:?}"
                );
            }
        }
    }

    /// Walks into room 3 holding right with no jumps, the way a new player
    /// arrives (stopped by the crystal column), then casts what the goal
    /// asks for: cards with no aim for Sprout, the tray equation for
    /// Seedling, a typed formula aimed at the goal marker for Apprentice.
    /// Returns the ticks it took to open gate 3, if it opened.
    fn room_3_from_arrival(tier: Tier, seed: u64) -> Option<u64> {
        let mut g = Game::new(tier, seed);
        for k in 0..2 {
            g.open_gate(k);
        }
        teleport(&mut g, (GATE_X[1] + GATE_W + 8) as f32);
        for _ in 0..240 {
            g.tick(INPUT_RIGHT);
        }
        settle(&mut g, 10);
        let arrived = g.player.x + PLAYER_W / 2.0;
        assert!(
            arrived < CRYSTAL.x0 as f32,
            "seed {seed}: walked past the crystal to {arrived}"
        );
        let start = g.ticks;
        for _ in 0..12 {
            if g.rooms.gates_open[2] {
                break;
            }
            let goal = g.goals()[2].clone();
            for _ in 0..40 {
                let ready = match goal.need {
                    "split" => g.jars["H2O"] >= 2,
                    _ => g.jars["H2"] >= 2 && g.jars["O2"] >= 1,
                };
                if ready {
                    break;
                }
                settle(&mut g, 15);
            }
            let text = match (tier, goal.need) {
                (Tier::Sprout, need) => need,
                (_, "split") => "2H2O -> 2H2 + O2",
                _ => "2H2 + O2 -> 2H2O",
            };
            if tier == Tier::Apprentice {
                g.set_aim(goal.x, goal.y, true);
            }
            assert_ne!(g.cast(text).outcome, Outcome::Fizzle, "seed {seed}");
            land(&mut g);
            settle(&mut g, 40);
        }
        g.rooms.gates_open[2].then(|| g.ticks - start)
    }

    #[test]
    fn room_3_clears_from_the_arrival_point_on_every_tier() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            for seed in SEEDS {
                let ticks = room_3_from_arrival(tier, seed);
                assert!(
                    ticks.is_some_and(|t| t < 1500),
                    "{tier:?} seed {seed}: {ticks:?}"
                );
            }
        }
    }

    #[test]
    fn a_held_jump_does_not_fling_the_player_past_room_3() {
        let mut g = Game::new(Tier::Seedling, 42);
        for k in 0..2 {
            g.open_gate(k);
        }
        teleport(&mut g, (GATE_X[1] + GATE_W + 8) as f32);
        // Run right holding jump for one long jump, then let go of everything.
        for t in 0..40 {
            g.tick(INPUT_RIGHT | INPUT_JUMP);
            if t > 2 && !g.player.grounded {
                assert!(g.player.vx.abs() <= AIR_MAX + 1e-3, "{}", g.player.vx);
            }
        }
        settle(&mut g, 60);
        assert!(g.player.x < TRAP.x1 as f32, "flung to {}", g.player.x);
    }

    #[test]
    fn a_boom_with_no_trapped_gas_does_not_open_gate_3() {
        let mut g = Game::new(Tier::Apprentice, 42);
        teleport(&mut g, 684.0);
        cast_at(&mut g, "boom", TRAP.centre());
        settle(&mut g, 60);
        assert!(!g.rooms.gates_open[2]);
    }

    #[test]
    fn ignored_fire_stays_in_room_1_and_never_opens_the_gate() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Seedling, seed);
            let outside = Rect::new(ROOM_W as i32, 0, 2 * ROOM_W as i32, HEIGHT as i32);
            let trees = Rect::new(100, 100, 210, GROUND_Y);
            for t in 0..20_000 {
                g.tick(0);
                if t % 250 == 0 {
                    assert_eq!(
                        burning_in(&g, outside),
                        0,
                        "seed {seed}: fire left room 1 at {t}"
                    );
                    assert_eq!(
                        burning_in(&g, trees),
                        0,
                        "seed {seed}: the trees caught at {t}"
                    );
                    assert!(
                        !g.rooms.gates_open[0],
                        "seed {seed}: gate opened by itself at {t}"
                    );
                }
            }
            assert!(!g.rooms.gates_open[0]);
            assert!(!g.trial_complete());
        }
    }
}

#[cfg(test)]
mod marsh {
    use super::script::*;
    use super::*;

    fn cells_of(g: &Game, r: Rect, s: Species) -> Vec<(i32, i32)> {
        r.cells().filter(|&(x, y)| g.grid.get(x, y) == s).collect()
    }

    /// Open ground in room 4 between the marsh and the alcove.
    const OPEN: Rect = Rect::new(1080, 0, 1170, GROUND_Y);

    #[test]
    fn co2_released_above_air_sinks_below_it_within_300_ticks() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Apprentice, seed);
            teleport(&mut g, 1060.0);
            let blob = Rect::new(1110, 50, 1130, 56);
            for (x, y) in blob.cells() {
                g.grid.set(x, y, Species::CO2, AMBIENT_C);
            }
            settle(&mut g, 300);
            let co2 = cells_of(&g, OPEN, Species::CO2);
            assert!(co2.len() >= 100, "seed {seed}: CO2 lost: {}", co2.len());
            let low = co2.iter().filter(|&&(_, y)| y >= GROUND_Y - 16).count();
            assert!(
                low * 10 >= co2.len() * 9,
                "seed {seed}: only {low} of {} CO2 cells near the floor",
                co2.len()
            );
            // Air sits above it: the cells it left are air again.
            assert!(blob.cells().all(|(x, y)| g.grid.get(x, y) != Species::CO2));
        }
    }

    #[test]
    fn a_pool_of_co2_puts_out_the_flames_it_covers() {
        let burning = |with_co2: bool| {
            let mut g = Game::new(Tier::Apprentice, 42);
            teleport(&mut g, 1060.0);
            // A hot log on the floor, burning.
            let log = Rect::new(1120, GROUND_Y - 3, 1136, GROUND_Y);
            for (x, y) in log.cells() {
                g.grid.set(x, y, Species::Wood, 650.0);
            }
            settle(&mut g, 20);
            if with_co2 {
                // A CO2 pool poured over it, 12 cells deep.
                for (x, y) in Rect::new(1108, GROUND_Y - 15, 1148, GROUND_Y).cells() {
                    let s = g.grid.get(x, y);
                    if s == Species::Air || s == Species::Flame {
                        g.grid.set(x, y, Species::CO2, AMBIENT_C);
                    }
                }
            }
            let zone = Rect::new(1110, GROUND_Y - 30, 1146, GROUND_Y);
            let mut flames = 0;
            for _ in 0..60 {
                g.tick(0);
                flames += cells_of(&g, zone, Species::Flame).len();
            }
            flames
        };
        let open_air = burning(false);
        let under_co2 = burning(true);
        assert!(open_air > 50, "the log should burn in air: {open_air}");
        assert!(
            under_co2 * 10 < open_air,
            "CO2 should smother it: {under_co2} flame-ticks vs {open_air}"
        );
    }

    #[test]
    fn a_spark_burns_vent_methane_into_co2_the_suit_can_capture() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Apprentice, seed);
            teleport(&mut g, 990.0);
            g.jars.insert("CO2".into(), 0);
            // Let methane build up over the marsh.
            settle(&mut g, 600);
            let marsh_air = Rect::new(MARSH.x0 - 20, 0, MARSH.x1 + 20, GROUND_Y);
            let ch4 = cells_of(&g, marsh_air, Species::CH4).len();
            assert!(ch4 >= 10, "seed {seed}: vents gave only {ch4} CH4");
            let co2 = cells_of(&g, marsh_air, Species::CO2).len();
            // A boom at the vent lights it: the methane burns into CO2.
            let mouth = g.vent_mouth(VENTS[0]).expect("vent under water");
            cast_at(&mut g, "boom", (mouth.0, mouth.1 - 2));
            settle(&mut g, 30);
            assert!(g.rooms.vent_lit[0], "seed {seed}: vent not lit");
            // Seeing methane burn finds no page: only casting it would.
            assert!(
                !g.discovered.iter().any(|d| d == "methane_combustion"),
                "seed {seed}: {:?}",
                g.discovered
            );
            assert!(
                cells_of(&g, marsh_air, Species::CO2).len() > co2,
                "seed {seed}: no CO2 made"
            );
            // Stand by the burning vent: the suit's scrubber catches hot CO2.
            walk_to(&mut g, VENTS[0] as f32 - 8.0, 300);
            settle(&mut g, 300);
            assert!(g.jars["CO2"] >= 1, "seed {seed}: no CO2 captured");
            // Water puts the vent out again.
            cast_at(&mut g, "water", mouth);
            settle(&mut g, 5);
            assert!(!g.rooms.vent_lit[0]);
        }
    }

    #[test]
    fn water_puts_out_the_second_hedge_then_a_boom_lights_the_brazier() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Seedling, seed);
            teleport(&mut g, 1150.0);
            assert!(
                douse(&mut g, 1, 4).is_some(),
                "seed {seed}: hedge 2 still burning"
            );
            assert!(!g.trial_complete());
            // Walk through the cooled hedge into the alcove and light the brazier.
            assert!(
                walk_to(&mut g, 1218.0, 600),
                "seed {seed}: blocked at {}",
                g.player.x
            );
            let target = (BRAZIER.centre().0, BRAZIER.y1 - 4);
            assert_ne!(cast_at(&mut g, "boom", target).outcome, Outcome::Fizzle);
            settle(&mut g, 20);
            assert!(g.rooms.brazier_lit, "seed {seed}: brazier dark");
            assert!(g.trial_complete());
            assert!(g.rooms.gates_open[3]);
            assert!(g.state().trial_complete);
        }
    }

    #[test]
    fn a_burning_hedge_blocks_the_way_to_the_brazier() {
        let mut g = Game::new(Tier::Seedling, 42);
        settle(&mut g, 1);
        teleport(&mut g, 1150.0);
        // Make sure the whole face is burning.
        for (x, y) in HEDGE2.cells() {
            let i = g.grid.idx(x as usize, y as usize);
            g.grid.temp[i] = 650.0;
        }
        walk_to(&mut g, 1218.0, 400);
        assert!(
            g.player.x + PLAYER_W <= HEDGE2.x0 as f32 + 1.0,
            "walked into fire at {}",
            g.player.x
        );
    }

    #[test]
    fn co2_cloud_cast_smothers_the_second_hedge() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Apprentice, seed);
            teleport(&mut g, 1150.0);
            g.jars.insert("CO2".into(), 10);
            let mut casts = 0;
            for _ in 0..10 {
                if g.rooms.hedges[1].out {
                    break;
                }
                let at = (HEDGE2.x0, GROUND_Y - 6);
                assert_eq!(cast_at(&mut g, "2CO2", at).outcome, Outcome::Full);
                casts += 1;
                settle(&mut g, 40);
            }
            assert!(
                g.rooms.hedges[1].out,
                "seed {seed}: still burning after {casts} clouds"
            );
        }
    }

    /// Hedge 2's wood above ignition, or flame anywhere in its zone.
    fn hedge2_alight(g: &Game) -> usize {
        g.rooms.hedges[1]
            .zone()
            .cells()
            .filter(|&(x, y)| {
                let s = g.grid.get(x, y);
                s == Species::Flame
                    || (s == Species::Wood && g.grid.temp_at(x, y) >= WOOD_IGNITION_C)
            })
            .count()
    }

    /// Puts hedge 2 out from in front of it, with water or with `clouds`
    /// CO2 casts, then checks it stays out for 30 s of ticks.
    fn hedge2_put_out(tier: Tier, seed: u64, clouds: Option<u32>) -> Game {
        let mut g = Game::new(tier, seed);
        teleport(&mut g, 1150.0);
        match clouds {
            None => {
                assert!(
                    douse(&mut g, 1, 4).is_some(),
                    "seed {seed}: hedge 2 burning"
                );
            }
            Some(n) => {
                g.jars.insert("CO2".into(), 10);
                for _ in 0..n {
                    let at = (HEDGE2.x0, GROUND_Y - 6);
                    assert_eq!(cast_at(&mut g, "cloud", at).outcome, Outcome::Full);
                    settle(&mut g, 40);
                }
            }
        }
        settle(&mut g, 60);
        assert!(g.rooms.hedges[1].out, "seed {seed}: hedge 2 not out");
        assert!(
            g.rooms.hedges[1].crumbled,
            "seed {seed}: hedge 2 still standing"
        );
        for t in 0..30 * 60 {
            g.tick(0);
            assert!(g.rooms.hedges[1].out, "seed {seed}: relit after {t} ticks");
            assert_eq!(hedge2_alight(&g), 0, "seed {seed}: alight after {t} ticks");
        }
        g
    }

    /// One boom from the left of the hedge lights the brazier, and the
    /// hedge never reaches ignition on the way.
    fn boom_lights_brazier_from_the_left(g: &mut Game, aimed: bool, seed: u64) {
        while g.jars["H2"] < 2 || g.jars["O2"] < 1 {
            settle(g, 10);
        }
        let out = if aimed {
            cast_at(g, "boom", (BRAZIER.centre().0, BRAZIER.y1 - 4))
        } else {
            let out = g.cast("boom");
            land(g);
            out
        };
        assert_eq!(out.outcome, Outcome::Full, "seed {seed}");
        for _ in 0..60 {
            g.tick(0);
            assert_eq!(hedge2_alight(g), 0, "seed {seed}: the boom relit the hedge");
        }
        assert!(
            g.rooms.brazier_lit,
            "seed {seed}: brazier dark (aimed {aimed})"
        );
    }

    #[test]
    fn water_puts_hedge_2_out_for_good_and_a_boom_from_the_left_lights_the_brazier() {
        for seed in SEEDS {
            let mut g = hedge2_put_out(Tier::Sprout, seed, None);
            assert!(
                g.player.x + PLAYER_W < HEDGE2.x0 as f32,
                "still left of the hedge"
            );
            boom_lights_brazier_from_the_left(&mut g, false, seed);
            let mut g = hedge2_put_out(Tier::Apprentice, seed, None);
            boom_lights_brazier_from_the_left(&mut g, true, seed);
        }
    }

    #[test]
    fn three_co2_casts_put_hedge_2_out_for_good() {
        for seed in SEEDS {
            let mut g = hedge2_put_out(Tier::Apprentice, seed, Some(3));
            boom_lights_brazier_from_the_left(&mut g, true, seed);
            let mut g = hedge2_put_out(Tier::Sprout, seed, Some(3));
            boom_lights_brazier_from_the_left(&mut g, false, seed);
        }
    }

    /// Holds G for `ticks` ticks, standing still.
    fn hold_gather(g: &mut Game, ticks: u32) {
        for _ in 0..ticks {
            g.tick(INPUT_GATHER);
        }
    }

    #[test]
    fn marsh_methane_stays_in_reach_after_a_minute_and_fuels_a_fireball() {
        for tier in [Tier::Seedling, Tier::Apprentice] {
            for seed in SEEDS {
                let mut g = Game::new(tier, seed);
                teleport(&mut g, 1150.0);
                assert!(
                    douse(&mut g, 1, 4).is_some(),
                    "seed {seed}: hedge 2 burning"
                );
                g.jars.insert("CH4".into(), 0);
                settle(&mut g, 60 * 60);
                assert!(
                    walk_to(&mut g, MARSH_STAND_X as f32, 800),
                    "seed {seed}: can't reach the marsh, at {}",
                    g.player.x
                );
                hold_gather(&mut g, 3 * 60);
                assert!(
                    g.jars["CH4"] >= 2,
                    "{tier:?} seed {seed}: gathered {} CH4",
                    g.jars["CH4"]
                );
                assert_eq!(
                    g.cast("fireball").outcome,
                    Outcome::Full,
                    "{tier:?} seed {seed}"
                );
            }
        }
    }

    #[test]
    fn gathering_with_nothing_in_reach_says_why() {
        let mut g = Game::new(Tier::Seedling, 42);
        teleport(&mut g, 1120.0);
        let before = g.jars.clone();
        g.messages.clear();
        hold_gather(&mut g, 40);
        assert_eq!(before, g.jars);
        assert!(
            g.messages.iter().any(|m| m.contains("No methane in reach")),
            "{:?}",
            g.messages
        );
        // A full jar is skipped, and named.
        let mut g = Game::new(Tier::Apprentice, 42);
        settle(&mut g, 2);
        walk_to(&mut g, 60.0, 600);
        let cap = Game::jar_cap("H2O");
        g.jars.insert("H2O".into(), cap);
        // Full gas jars too, so the electrolyser doesn't draw the water down.
        g.jars.insert("H2".into(), Game::jar_cap("H2"));
        g.jars.insert("O2".into(), Game::jar_cap("O2"));
        let before = g.jars.clone();
        g.messages.clear();
        hold_gather(&mut g, 40);
        assert_eq!(before["H2O"], g.jars["H2O"]);
        assert!(
            g.messages.iter().any(|m| m.contains("water jar is full")),
            "{:?}",
            g.messages
        );
    }

    #[test]
    fn room_4_goal_has_two_steps_ticked_in_order() {
        let mut g = Game::new(Tier::Seedling, 42);
        teleport(&mut g, 1150.0);
        let steps = |g: &Game| g.goals()[3].steps.clone().expect("room 4 has steps");
        let s = steps(&g);
        assert_eq!(s.len(), 2);
        assert!(!s[0].done && !s[1].done);
        assert!(douse(&mut g, 1, 4).is_some());
        settle(&mut g, 60);
        let s = steps(&g);
        assert!(s[0].done && !s[1].done, "{s:?}");
        assert!(
            g.messages.iter().any(|l| l.contains("Hedge out")),
            "{:?}",
            g.messages
        );
        // The stand spot moves to beside the brazier, past the hedge.
        assert!(g.goals()[3].stand_x > HEDGE2.x1);
    }
}

#[cfg(test)]
mod restart_tests {
    use super::script::*;
    use super::*;

    #[test]
    fn restarting_room_2_rebuilds_its_ice_and_keeps_room_1_solved() {
        let mut g = Game::new(Tier::Seedling, 7);
        settle(&mut g, 30);
        assert!(walk_to(&mut g, HEDGE1_STAND_X as f32, 2000));
        assert!(douse(&mut g, 0, 4).is_some());
        settle(&mut g, 20);
        assert!(g.rooms.gates_open[0]);
        assert!(walk_to(&mut g, 440.0, 2000), "stuck at {}", g.player.x);
        cast_at(&mut g, "boom", ICE.centre());
        settle(&mut g, 60);
        assert!(
            g.rooms.ice_left < g.rooms.ice_start,
            "the boom melted nothing"
        );
        g.jars.insert("H2".into(), 0);

        assert_eq!(g.restart_room(), 1);
        settle(&mut g, 2);
        assert_eq!(g.room(), 1);
        assert_eq!(g.rooms.ice_left, g.rooms.ice_start);
        assert!(!g.rooms.ice_heated);
        assert!(g.rooms.gates_open[0], "room 1 must stay solved");
        assert!(!g.rooms.gates_open[1]);
        assert_eq!(g.grid.get(GATE_X[0] + 2, GROUND_Y - 4), Species::Air);
        assert_eq!(g.grid.get(GATE_X[1] + 2, GROUND_Y - 4), Species::Door);
        assert!(g.jars["H2"] >= 4, "starter charge comes back");
        assert!(g.bolts_in_flight() == 0);
    }

    #[test]
    fn restarting_room_1_lights_the_hedge_again_and_still_needs_the_player() {
        let mut g = Game::new(Tier::Sprout, 42);
        settle(&mut g, 30);
        assert!(walk_to(&mut g, HEDGE1_STAND_X as f32, 2000));
        assert!(douse(&mut g, 0, 4).is_some());
        settle(&mut g, 20);
        assert!(g.rooms.gates_open[0]);
        assert_eq!(g.restart_room(), 0);
        assert!(!g.rooms.gates_open[0]);
        assert_eq!(g.rooms.hedges[0].hits, 0);
        settle(&mut g, 60);
        assert!(g.rooms.hedges[0].burning > 0, "the hedge burns again");
        assert!(!g.rooms.gates_open[0]);
    }

    #[test]
    fn restart_rebuilds_the_same_level_from_the_seed() {
        let mut a = Game::new(Tier::Seedling, 99);
        settle(&mut a, 30);
        a.cast("water");
        settle(&mut a, 200);
        a.restart();
        let b = Game::new(Tier::Seedling, 99);
        assert_eq!(a.ticks, 0);
        assert!(a.grid.cells == b.grid.cells);
        assert_eq!(a.jars, b.jars);
    }

    #[test]
    fn free_play_opens_every_gate_and_fills_the_suit() {
        let mut g = Game::new(Tier::Apprentice, 1);
        g.free_play();
        assert!(g.rooms.gates_open.iter().all(|&o| o));
        for (f, n) in &g.jars {
            assert_eq!(*n, Game::jar_cap(f), "{f}");
        }
        assert!(walk_to(&mut g, 440.0, 4000), "stuck at {}", g.player.x);
    }

    #[test]
    fn every_room_has_its_own_objective_line_on_every_tier() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let g = Game::new(tier, 42);
            let lines: Vec<_> = g.goals().iter().map(|goal| goal.text).collect();
            assert_eq!(lines.len(), ROOMS);
            for (k, line) in lines.iter().enumerate() {
                assert!(!line.trim().is_empty(), "{tier:?} room {k}");
                assert!(
                    !lines[..k].contains(line),
                    "{tier:?}: room {k} repeats a line"
                );
                if tier == Tier::Seedling {
                    assert!(line.split_whitespace().count() <= 8, "{line}");
                }
            }
        }
        let g = Game::new(Tier::Apprentice, 42);
        assert_eq!(
            g.goals()[2].text,
            "Trap the gas under the stone roof, then light it."
        );
    }

    #[test]
    fn goals_name_the_spell_that_moves_them_on() {
        let g = Game::new(Tier::Sprout, 42);
        let needs: Vec<_> = g.goals().iter().map(|goal| goal.need).collect();
        assert_eq!(needs, ["water", "boom", "split", "water"]);
        let mut g = Game::new(Tier::Seedling, 42);
        teleport(&mut g, 1150.0);
        assert!(douse(&mut g, 1, 4).is_some());
        assert_eq!(g.goals()[3].need, "boom");
        assert_eq!(g.goals()[3].stand_x, 1218);
    }

    /// Plays the trial the way the hint ladder and go arrow suggest: stand at
    /// each goal's `stand_x` and cast its `need` with soft auto-aim (no
    /// aiming). Returns the game and every line said along the way.
    pub(super) fn play_by_goals(tier: Tier, seed: u64) -> (Game, Vec<String>, Vec<CastOutcome>) {
        let mut g = Game::new(tier, seed);
        let mut said = vec![];
        let mut outs = vec![];
        settle(&mut g, 30);
        let mut casts = 0;
        for k in 0..ROOMS {
            while !g.goals()[k].done {
                let goal = g.goals()[k].clone();
                assert!(
                    walk_to(&mut g, goal.stand_x as f32, 3000),
                    "room {k}: stuck at {}",
                    g.player.x
                );
                let spell = goal.need;
                // Wait for the suit to have what the spell needs, as a player would.
                for _ in 0..40 {
                    let ok = match spell {
                        "water" => g.jars["H2O"] >= 1,
                        "split" => g.jars["H2O"] >= 2,
                        _ => g.jars["H2"] >= 2 && g.jars["O2"] >= 1,
                    };
                    if ok {
                        break;
                    }
                    settle(&mut g, 15);
                }
                let text = match (tier, spell) {
                    (Tier::Sprout, s) => s,
                    (_, "water") => "H2O",
                    (_, "split") => "2H2O -> 2H2 + O2",
                    _ => "2H2 + O2 -> 2H2O",
                };
                outs.push(g.cast(text));
                land(&mut g);
                settle(&mut g, 40);
                said.extend(g.state().messages);
                casts += 1;
                assert!(casts < 40, "room {k}: no progress with {spell}");
            }
        }
        (g, said, outs)
    }

    #[test]
    fn standing_where_each_goal_says_and_casting_its_spell_moves_it_on() {
        for tier in [Tier::Sprout, Tier::Seedling] {
            let (g, _, _) = play_by_goals(tier, 7);
            assert!(g.trial_complete(), "{tier:?}");
        }
    }

    #[test]
    fn grimoire_pages_come_only_from_spells_the_player_cast() {
        for tier in [Tier::Sprout, Tier::Seedling] {
            let (g, _, outs) = play_by_goals(tier, 42);
            assert!(g.trial_complete());
            let firsts: Vec<_> = outs
                .iter()
                .filter(|o| o.first_discovery)
                .filter_map(|o| o.reaction_id.clone())
                .collect();
            assert_eq!(g.discovered, firsts, "{tier:?}");
            assert!(!g.discovered.iter().any(|d| d == "carbon_combustion"));
        }
    }

    #[test]
    fn sprout_boom_card_with_no_aim_melts_the_ice_in_three_casts() {
        for seed in SEEDS {
            for start in [GATE_X[0] as f32 + 24.0, 440.0] {
                let mut g = Game::new(Tier::Sprout, seed);
                g.open_gate(0);
                teleport(&mut g, start);
                assert_eq!(g.aim_tick, 0, "never aimed");
                let mut casts = 0;
                while !g.rooms.gates_open[1] && casts < 3 {
                    while g.jars["H2"] < 2 || g.jars["O2"] < 1 {
                        settle(&mut g, 10);
                    }
                    assert_eq!(g.cast("boom").outcome, Outcome::Full);
                    casts += 1;
                    land(&mut g);
                    settle(&mut g, 40);
                }
                assert!(
                    g.rooms.gates_open[1],
                    "seed {seed} from {start}: ice left {} of {}",
                    g.rooms.ice_left, g.rooms.ice_start
                );
                assert!(!g.rooms.torch.lit, "the card went to the torch");
            }
        }
    }

    /// One balanced boom from where the goal says to stand clears room 2
    /// within 5 s, on every tier, aimed the way that tier aims.
    #[test]
    fn one_balanced_boom_from_the_stand_spot_clears_the_ice() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            for seed in 1..=10u64 {
                let mut g = Game::new(tier, seed);
                g.open_gate(0);
                let stand = g.goals()[1].stand_x;
                teleport(&mut g, stand as f32);
                let out = match tier {
                    // A tapped card with no aim: soft auto-aim.
                    Tier::Sprout => {
                        let out = g.cast("boom");
                        land(&mut g);
                        out
                    }
                    // Pointing at the goal marker.
                    Tier::Seedling => {
                        let m = &g.goals()[1];
                        let at = (m.x, m.y);
                        cast_at(&mut g, "2H2 + O2 -> 2H2O", at)
                    }
                    Tier::Apprentice => cast_at(&mut g, "2H2 + O2 -> 2H2O", ICE.centre()),
                };
                assert_eq!(out.outcome, Outcome::Full, "{tier:?} seed {seed}");
                settle(&mut g, 5 * 60);
                assert!(
                    g.rooms.gates_open[1],
                    "{tier:?} seed {seed}: ice left {} of {}",
                    g.rooms.ice_left, g.rooms.ice_start
                );
            }
        }
    }

    #[test]
    fn a_weak_boom_melts_part_of_the_ice_and_says_go_again() {
        for tier in [Tier::Sprout, Tier::Seedling, Tier::Apprentice] {
            let mut g = Game::new(tier, 42);
            g.open_gate(0);
            teleport(&mut g, 440.0);
            g.messages.clear();
            // Half a boom: only one H2 left in the jar, so half the heat.
            g.jars.insert("H2".into(), 1);
            let out = cast_at(&mut g, "2H2 + O2 -> 2H2O", ICE.centre());
            assert_ne!(out.outcome, Outcome::Fizzle, "{tier:?}");
            settle(&mut g, 60);
            assert!(
                !g.rooms.gates_open[1],
                "{tier:?}: half a boom cleared it: {} {} {}",
                out.equation, out.dh_kj, g.rooms.ice_left
            );
            let left = g.goals()[1].meter.expect("ice meter");
            assert!(left > 0.1 && left < 0.9, "{tier:?}: ice left {left}");
            let said = g.messages.join(" | ");
            assert!(
                said.contains("gain") || said.contains("Again"),
                "{tier:?}: {said}"
            );
            if tier == Tier::Apprentice {
                assert!(said.contains("6.01 kJ"), "{said}");
            }
        }
    }

    #[test]
    fn water_on_the_ice_says_too_slow_with_a_frost_puff() {
        let mut g = Game::new(Tier::Sprout, 42);
        g.open_gate(0);
        teleport(&mut g, 440.0);
        g.take_fx();
        let out = g.cast("water");
        assert_eq!(out.outcome, Outcome::Full);
        assert!(out.label.contains("Too slow"), "{}", out.label);
        land(&mut g);
        assert!(g.take_fx().iter().any(|f| f.kind == "frost"));
    }

    #[test]
    fn seedling_prompts_are_eight_words_or_fewer() {
        let (g, said, _) = play_by_goals(Tier::Seedling, 42);
        assert!(said.len() >= 6, "{said:?}");
        // Grimoire discoveries quote the sourced concept text; prompts are short.
        let book = ReactionBook::load();
        for line in said {
            if book.reactions.iter().any(|(r, _)| r.concept == line) {
                continue;
            }
            let words = line.split_whitespace().count();
            assert!(words <= 8, "{words} words: {line}");
        }
        assert!(g.trial_complete());
    }
}

#[cfg(test)]
mod full_run {
    use super::script::*;
    use super::*;

    /// A scripted play-through of all four rooms with real movement input,
    /// the way a player would: douse, melt, split and light, walk on.
    fn play(seed: u64) -> Game {
        let mut g = Game::new(Tier::Seedling, seed);
        settle(&mut g, 30);
        // Room 1: three water casts at the hedge.
        assert!(
            walk_to(&mut g, HEDGE1_STAND_X as f32, 2000),
            "seed {seed}: stuck in room 1"
        );
        assert!(douse(&mut g, 0, 4).is_some(), "seed {seed}: fire 1");
        settle(&mut g, 20);
        assert!(g.rooms.gates_open[0], "seed {seed}: gate 1");
        // Room 2: boom the ice wall.
        assert!(
            walk_to(&mut g, 440.0, 2000),
            "seed {seed}: stuck before the ice at {}",
            g.player.x
        );
        melt_ice_with_booms(&mut g, 4000);
        assert!(g.rooms.gates_open[1], "seed {seed}: gate 2");
        // Room 3: split under the overhang, then light it.
        assert!(
            walk_to(&mut g, 684.0, 3000),
            "seed {seed}: stuck in room 2 at {}",
            g.player.x
        );
        let mut tries = 0;
        while !g.rooms.gates_open[2] && tries < 6 {
            tries += 1;
            cast_at(&mut g, "split", TRAP.centre());
            settle(&mut g, 30);
            while g.jars["H2"] < 2 || g.jars["O2"] < 1 {
                settle(&mut g, 10);
            }
            cast_at(&mut g, "boom", TRAP.centre());
            settle(&mut g, 30);
        }
        assert!(g.rooms.gates_open[2], "seed {seed}: gate 3");
        // Room 4: put out the hedge, walk through, light the brazier.
        assert!(
            walk_to(&mut g, 1150.0, 3000),
            "seed {seed}: stuck in room 3/4 at {}",
            g.player.x
        );
        while g.jars["H2O"] < 4 {
            settle(&mut g, 30);
        }
        assert!(douse(&mut g, 1, 4).is_some(), "seed {seed}: fire 2");
        assert!(
            walk_to(&mut g, 1218.0, 1000),
            "seed {seed}: stuck at hedge 2 at {}",
            g.player.x
        );
        while g.jars["H2"] < 2 || g.jars["O2"] < 1 {
            settle(&mut g, 10);
        }
        cast_at(&mut g, "boom", (BRAZIER.centre().0, BRAZIER.y1 - 4));
        settle(&mut g, 30);
        g
    }

    #[test]
    fn the_trial_can_be_finished() {
        for seed in SEEDS {
            let g = play(seed);
            assert!(g.trial_complete(), "seed {seed}: brazier not lit");
            assert!(g.rooms.gates_open.iter().all(|&o| o));
            assert!(g.ticks < 20_000, "seed {seed}: took {} ticks", g.ticks);
        }
    }

    #[test]
    fn the_trial_never_completes_with_no_input() {
        for seed in SEEDS {
            let mut g = Game::new(Tier::Seedling, seed);
            for _ in 0..20_000 {
                g.tick(0);
            }
            assert!(!g.trial_complete());
            assert!(
                g.rooms.gates_open.iter().all(|&o| !o),
                "{:?}",
                g.rooms.gates_open
            );
        }
    }

    #[test]
    fn room_4_left_alone_never_completes() {
        // With the camera parked in room 4, its vents and hedge are simulated.
        let mut g = Game::new(Tier::Seedling, 42);
        teleport(&mut g, 1100.0);
        settle(&mut g, 8000);
        assert!(!g.trial_complete());
        assert!(!g.rooms.brazier_lit);
    }
}

#[cfg(test)]
mod path_tests {
    use super::script::*;
    use super::*;

    /// Puts out the room-1 fire from the spawn with water casts.
    fn douse_from_spawn(seed: u64) -> Game {
        let mut g = Game::new(Tier::Seedling, seed);
        settle(&mut g, 30);
        let mut casts = 0;
        while !g.rooms.gates_open[0] && casts < 6 {
            g.cast("water");
            casts += 1;
            settle(&mut g, 60);
        }
        assert!(g.rooms.gates_open[0], "seed {seed}: fire still burning");
        g
    }

    #[test]
    fn holding_right_with_a_jump_tap_every_second_reaches_room_2() {
        for seed in SEEDS {
            let mut g = douse_from_spawn(seed);
            let mut reached = None;
            for t in 0..600u32 {
                let tap = if t % 60 == 0 { INPUT_JUMP } else { 0 };
                g.tick(INPUT_RIGHT | tap);
                if g.room() == 1 {
                    reached = Some(t);
                    break;
                }
            }
            let t = reached.unwrap_or_else(|| panic!("seed {seed}: stuck at x={}", g.player.x));
            // About 100 cells of walking at 2.2 cells/tick, plus slack.
            assert!(t < 240, "seed {seed}: took {t} ticks");
        }
    }

    /// The gate-1 line names the side of the fire triangle the player took
    /// away: heat for water, oxygen for CO2.
    #[test]
    fn room_1_line_names_heat_for_water_and_oxygen_for_co2() {
        let g = douse_from_spawn(42);
        let said = g.messages.join(" | ");
        assert!(said.contains("heat"), "{said}");
        assert!(!said.contains("oxygen"), "{said}");

        for tier in [Tier::Seedling, Tier::Apprentice] {
            let mut g = Game::new(tier, 42);
            settle(&mut g, 30);
            g.jars.insert("CO2".into(), 20);
            g.messages.clear();
            let mut casts = 0;
            while !g.rooms.gates_open[0] && casts < 10 {
                let at = (HEDGE1.centre().0, HEDGE1.y0);
                cast_at(&mut g, "2CO2", at);
                casts += 1;
                settle(&mut g, 60);
            }
            assert!(g.rooms.gates_open[0], "{tier:?}: CO2 never put it out");
            let said = g.messages.join(" | ");
            assert!(said.contains("oxygen"), "{tier:?}: {said}");
            if tier == Tier::Apprentice {
                // It names all three sides, then the one that was taken.
                assert!(said.contains("took away the oxygen"), "{said}");
            } else {
                assert!(!said.contains("heat"), "{tier:?}: {said}");
            }
        }
    }

    #[test]
    fn a_put_out_hedge_falls_to_ash_and_stops_blocking() {
        let g = douse_from_spawn(42);
        let wood = HEDGE1
            .cells()
            .filter(|&(x, y)| g.grid.get(x, y) == Species::Wood)
            .count();
        assert_eq!(wood, 0, "scorched stump left behind");
        assert!(HEDGE1
            .cells()
            .all(|(x, y)| !blocks_player(g.grid.get(x, y), g.grid.temp_at(x, y))));
    }
}

#[cfg(test)]
mod copy_tests {
    use super::script::*;
    use super::*;

    /// Every `line("sprout", "seedling", "apprentice")` call in the game's
    /// source, read straight from the files so no line can be missed.
    fn tier_lines() -> Vec<[String; 3]> {
        let mut out = Vec::new();
        for src in [include_str!("rooms.rs"), include_str!("../game.rs")] {
            let mut rest = src;
            while let Some(i) = rest.find("line(") {
                rest = &rest[i + 5..];
                let mut lits = Vec::new();
                let mut s = rest.trim_start();
                while lits.len() < 3 && s.starts_with('"') {
                    let body = &s[1..];
                    let end = body.find('"').expect("closed string");
                    lits.push(body[..end].to_string());
                    s = body[end + 1..]
                        .trim_start()
                        .trim_start_matches(',')
                        .trim_start();
                }
                if let [a, b, c] = &lits[..] {
                    out.push([a.clone(), b.clone(), c.clone()]);
                }
            }
        }
        out
    }

    fn words(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn found_every_tier_line() {
        // Keeps the scanner honest: the game has well over a dozen of them.
        assert!(tier_lines().len() >= 20, "{}", tier_lines().len());
    }

    #[test]
    fn sprout_lines_are_six_words_or_fewer() {
        let book = ReactionBook::load();
        let mut lines: Vec<String> = tier_lines().into_iter().map(|[s, _, _]| s).collect();
        lines.extend(book.reactions.iter().map(|(r, _)| r.short_sprout.clone()));
        for line in lines {
            assert!(
                words(&line) <= 6,
                "{} words for Sprout: {line}",
                words(&line)
            );
        }
    }

    #[test]
    fn seedling_lines_are_eight_words_or_fewer() {
        let book = ReactionBook::load();
        let mut lines: Vec<String> = tier_lines().into_iter().map(|[_, s, _]| s).collect();
        lines.extend(book.reactions.iter().map(|(r, _)| r.short_seedling.clone()));
        for line in lines {
            assert!(
                words(&line) <= 8,
                "{} words for Seedling: {line}",
                words(&line)
            );
        }
    }

    #[test]
    fn sprout_cast_labels_are_six_words_or_fewer() {
        let mut g = Game::new(Tier::Sprout, 42);
        settle(&mut g, 30);
        for spell in ["water", "boom", "split", "H2", "cloud", "xyz", "fireball"] {
            let out = g.cast(spell);
            assert!(words(&out.label) <= 6, "{spell}: {}", out.label);
            land(&mut g);
            settle(&mut g, 10);
        }
        // Everything Mote said in a full Sprout play-through, too.
        let (_, said, _) = super::restart_tests::play_by_goals(Tier::Sprout, 42);
        for line in said {
            assert!(words(&line) <= 6, "Sprout heard: {line}");
        }
    }

    #[test]
    fn a_first_discovery_speaks_the_tier_line() {
        let book = ReactionBook::load();
        let (r, _) = book
            .reactions
            .iter()
            .find(|(r, _)| r.id == "water_synthesis")
            .unwrap();
        for (tier, want) in [
            (Tier::Sprout, &r.short_sprout),
            (Tier::Seedling, &r.short_seedling),
            (Tier::Apprentice, &r.concept),
        ] {
            let mut g = Game::new(tier, 42);
            settle(&mut g, 30);
            g.messages.clear();
            let out = g.cast("2H2 + O2 -> 2H2O");
            assert!(out.first_discovery, "{tier:?}");
            assert!(
                std::mem::take(&mut g.messages).contains(want),
                "{tier:?} did not hear {want}"
            );
        }
    }
}
