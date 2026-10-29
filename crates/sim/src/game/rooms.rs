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
const GATE_W: i32 = 6;
const DOOR_TOP: i32 = 112;

// Room 1: Splash Garden.
pub const HEDGE1: Rect = Rect::new(236, 132, 260, GROUND_Y);
// Room 2: Frozen Gate.
pub const TORCH_X: i32 = 344;
const TORCH_H: i32 = 12;
pub const ICE: Rect = Rect::new(470, 128, 482, GROUND_Y);
const ICE_ROCK: Rect = Rect::new(458, 0, 496, 128);
const PIT: Rect = Rect::new(504, GROUND_Y, 576, 164);
// Room 3: Crystal Lab.
pub const CRYSTAL: Rect = Rect::new(668, 134, 674, GROUND_Y);
pub const POOL: Rect = Rect::new(732, GROUND_Y, 784, 160);
const ROOF: Rect = Rect::new(722, 104, 794, 108);
/// The gas pocket under the overhang, where rising hydrogen collects.
pub const TRAP: Rect = Rect::new(726, 108, 790, 124);
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
/// Vents stop when this much methane is already loose in the room.
const VENT_MAX_CH4: usize = 40;
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

#[derive(Debug, Clone)]
pub struct Hedge {
    pub rect: Rect,
    /// Water or CO2 casts that landed on this hedge.
    pub hits: u32,
    /// Burning cells now, and the most there have been (for the meter).
    pub burning: u32,
    pub peak: u32,
    pub out: bool,
    /// Mote has said it burned out with no help.
    said_self_out: bool,
}

impl Hedge {
    fn new(rect: Rect) -> Hedge {
        Hedge {
            rect,
            hits: 0,
            burning: 0,
            peak: 0,
            out: false,
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
    /// "Fire left" for the hedges, 0..=1.
    pub meter: Option<f32>,
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
        g.fill_rect(228, 144, 232, GROUND_Y, Species::Stone);
        g.fill_rect(264, 144, 268, GROUND_Y, Species::Stone);
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

        self.objectives = vec![
            Objective {
                id: "fire",
                text: "Put out the burning hedge".into(),
                done: false,
            },
            Objective {
                id: "ice",
                text: "Melt the ice wall".into(),
                done: false,
            },
            Objective {
                id: "gas",
                text: "Trap the gas under the stone roof and light it".into(),
                done: false,
            },
            Objective {
                id: "brazier",
                text: "Light the brazier by the last door".into(),
                done: false,
            },
        ];
        let intro = match self.tier {
            Tier::Sprout => "Fire! Tap the water drop to put it out!",
            Tier::Seedling => "Fire! Your suit is full of water. Cast H2O at the flames.",
            Tier::Apprentice => {
                "Fire in the garden. Your suit is full of water: cast H2O at the flames."
            }
        };
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
            self.say("It burned out by itself. Cast water on the embers to cool them, and the gate will open.");
        }
        let h = &mut self.rooms.hedges[k];
        h.out = out;
    }

    fn enter_rooms(&mut self) {
        let room = self.room();
        if self.rooms.entered[room] {
            return;
        }
        self.rooms.entered[room] = true;
        let young = self.tier == Tier::Sprout;
        let line = match (room, young) {
            (1, true) => "Ice! Hot things melt ice. Try the boom card!",
            (1, false) => "An ice wall. Heat melts ice: cast boom (2H2 + O2 -> 2H2O) at it, or light the torch and carry it there.",
            (2, true) => "A crystal pool! Splash it with water and watch the bubbles.",
            (2, false) => "The crystal pool. Electricity splits water into two hydrogen for every oxygen. Trap the gas under the stone roof, then light it.",
            (3, true) => "Another fire! Put it out, then light the bowl by the door.",
            (3, false) => "The marsh bubbles methane. Put out the hedge with water or carbon dioxide, then light the brazier.",
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
        let active = vx + 2 * VIEW_W as i32 > MARSH.x0 && vx - (VIEW_W as i32) < MARSH.x1;
        if !active {
            return;
        }
        if self.ticks.is_multiple_of(60) {
            let room4 = Rect::new(3 * ROOM_W as i32, 0, WORLD_W as i32, HEIGHT as i32);
            self.rooms.ch4_loose = self.count_in(room4, Species::CH4);
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
                    self.say("The methane vent caught fire! Burning methane makes carbon dioxide: stand close and your suit will catch it.");
                }
            }
            let lit = self.rooms.vent_lit[k];
            let period = if lit { VENT_LIT_TICKS } else { VENT_TICKS };
            if !(self.ticks + k as u64 * 7).is_multiple_of(period) {
                continue;
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
                self.add("CO2", 1);
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
            self.emit_at("discover", tx as f32, ty as f32, 1.0);
            self.say(if self.rooms.torch.carried {
                "Your torch caught fire."
            } else {
                "The torch is lit! Walk into it to pick it up."
            });
        }
        let p = &self.player;
        let touching = (p.x + PLAYER_W / 2.0 - self.rooms.torch.x).abs() < 5.0
            && p.y < self.rooms.torch.y
            && p.y + PLAYER_H > self.rooms.torch.y - TORCH_H as f32;
        if !self.rooms.torch.carried && touching && (self.rooms.torch.lit || gather) {
            self.rooms.torch.carried = true;
            self.say("You picked up the torch. Hold it against something to heat it.");
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
            self.say("Your torch went out in the water. Light it again with fire.");
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
        self.update_hedge(0);
        let h = &self.rooms.hedges[0];
        if h.out && h.hits > 0 && self.open_gate(0) {
            self.say(match self.tier {
                Tier::Sprout => "You put out the fire! The gate is open.",
                _ => "The fire is out. Fire needs fuel, oxygen and heat: your water took away the heat. The gate is open.",
            });
        }

        // Room 2: at least 90% of the ice is gone, melted by the player's heat.
        self.rooms.ice_left = self.count_in(ICE, Species::Ice);
        if self.rooms.ice_heated
            && self.rooms.ice_left * 10 <= self.rooms.ice_start
            && self.open_gate(1)
        {
            self.say("The ice melted into water. Heat changes ice to water, and water to steam. The gate is open.");
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
                self.say("Bang! The trapped hydrogen burned with the oxygen and blew the door open. 2H2 + O2 -> 2H2O.");
            }
        }
        if self.rooms.trap_h2 == 0 && !self.rooms.gates_open[2] {
            // Gas that drifted away no longer counts as trapped.
            self.rooms.trap_peak = 0;
        }

        // Room 4: the hedge (for the meter) and the brazier.
        self.update_hedge(1);
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
                self.say("The brazier is lit and the last door is open. Trial complete: you broke through to the next realm!");
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

    /// A water or CO2 cast landed at `at`: credit any hedge it hit.
    pub(super) fn note_douse(&mut self, at: (i32, i32)) {
        for h in &mut self.rooms.hedges {
            if h.zone().grow(8).contains(at.0, at.1) {
                h.hits += 1;
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
            if !self.discovered.iter().any(|d| d == "electrolysis_water") {
                self.discovered.push("electrolysis_water".into());
            }
        }
        made
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
            },
            Goal {
                room: 2,
                kind: "ice",
                x: ix,
                y: ICE.y0 - 8,
                done: r.gates_open[1],
                meter: None,
            },
            Goal {
                room: 3,
                kind: "gas",
                x: gx,
                y: gy,
                done: r.gates_open[2],
                meter: None,
            },
            Goal {
                room: 4,
                kind: "brazier",
                x: bx,
                y: BRAZIER.y0 - 10,
                done: r.brazier_lit,
                meter: Some(r.hedges[1].meter()),
            },
        ]
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

    /// Soft auto-aim targets for heat spells, in priority order: ice, an
    /// unlit torch, trapped hydrogen, the brazier.
    pub(super) fn heat_target(&self, x: i32, y: i32) -> bool {
        let s = self.grid.get(x, y);
        if s == Species::Ice {
            return true;
        }
        if !self.rooms.torch.lit && !self.rooms.torch.carried && (x, y) == self.torch_head() {
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
                walk_to(&mut g, 200.0, 2000),
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
        assert!(walk_to(&mut g, 200.0, 2000));
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
            // The meltwater runs off into the pit.
            settle(&mut g, 300);
            assert!(g.count_in(PIT, Species::Water) > 20, "seed {seed}: pit dry");
        }
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
        assert!(walk_to(&mut g, 200.0, 2000), "seed {seed}: stuck in room 1");
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
