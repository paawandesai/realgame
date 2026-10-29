//! The matter grid: a falling-sand cellular automaton where every cell is a
//! real substance with a temperature.
//!
//! Each step runs three passes: movement (gravity and buoyancy), chemistry
//! (phase changes and combustion) and heat conduction. Steps are deterministic
//! for a given seed, so a server can replay what a client predicted.

use crate::species::{Phase, Species, AIR_MOLAR_MASS};

pub const AMBIENT_C: f32 = 20.0;
/// Adiabatic flame temperatures, rounded: hydrogen or methane in pure oxygen
/// burns near 2800 C, in air near 2000 C. Product heat is capped at these
/// instead of the much higher figure the raw enthalpy gives, because real
/// flames lose energy to dissociation.
pub const FLAME_C_IN_O2: f32 = 2800.0;
pub const FLAME_C_IN_AIR: f32 = 2000.0;
pub const WOOD_FIRE_C: f32 = 1100.0;
pub const WOOD_IGNITION_C: f32 = 300.0;
/// Autoignition temperatures at 1 atm: hydrogen about 500 C, methane about 580 C.
pub const H2_IGNITION_C: f32 = 500.0;
pub const CH4_IGNITION_C: f32 = 580.0;
/// Molar heat capacity of liquid water, J/(mol K). Latent heats are stored as
/// an equivalent temperature buffer: 40.65 kJ/mol / 75.3 J/(mol K) = 540 K.
pub const WATER_CP_J_PER_MOL_K: f32 = 75.3;

/// Flames whose timer starts at or above this become CO2 when they die
/// (carbon burned); flames below it leave plain air (hydrogen burned).
const FLAME_LEAVES_CO2: i16 = 1000;

/// Densities at 0 C and 1 atm, g/L (CRC Handbook of Chemistry and Physics):
/// carbon dioxide 1.977, dry air 1.293. CO2 is about 1.53 times as dense as
/// air, so it sinks, pools on the floor and pushes the air (and its oxygen) up.
pub const CO2_DENSITY_G_PER_L: f32 = 1.977;
pub const AIR_DENSITY_G_PER_L: f32 = 1.293;
/// How eagerly a denser-than-air gas sinks, per unit of excess relative density
/// (a tuning value: CO2 sinks about 0.8 cells per tick through still air).
const SINK_GAIN: f32 = 1.5;

/// Air cooler than this is the background medium and only moves when displaced.
const STILL_AIR_BELOW_C: f32 = 60.0;

/// Neighbouring cells closer in temperature than this exchange no heat; the
/// transfer would be far below anything visible or gameplay-relevant.
const HEAT_EPSILON_C: f32 = 0.01;

/// Heat-pass sleeping works on row segments this many cells wide, aligned to
/// absolute x, so a segment means the same cells wherever the camera is.
const HEAT_CHUNK: usize = 64;
/// "Never": a frame number no real step reaches before wrapping.
const NEVER: u32 = u32::MAX;

const NEIGHBORS: [(i32, i32); 8] = [
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];

pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Species>,
    pub temp: Vec<f32>,
    /// Per-cell scratch: latent heat progress for water, fuel burned for wood,
    /// oxygen used for O2 and CH4, remaining life for flames.
    pub aux: Vec<i16>,
    moved: Vec<u32>,
    reacted: Vec<u32>,
    pub frame: u32,
    rng: u64,
    boil_buffer: f32,
    melt_buffer: f32,
    /// Chance per tick that hot wood with pure oxygen beside it puts out a
    /// flame (scaled down for air). Rooms tune this to keep fires bounded.
    pub wood_flame_chance: f32,
    /// Skip heat-pass segments that provably cannot change (see `heat_pass`).
    /// Results are bit-for-bit the same either way; off only for tests.
    pub heat_sleep: bool,
    heat: HeatSleep,
}

/// Bookkeeping for heat-pass sleeping. Heat conduction has no randomness, so
/// a row segment whose inputs are bit-for-bit what they were last step, and
/// which changed nothing last step, changes nothing again. Inputs are checked
/// against a snapshot rather than tracked at every write, so writes from
/// anywhere (chemistry, spells, level resets) are always noticed.
struct HeatSleep {
    /// Temperatures and species of every cell at the start of the last
    /// heat pass that covered it.
    snap_temp: Vec<f32>,
    snap_cells: Vec<Species>,
    /// Per segment, this step: the covered cells match the snapshot.
    same: Vec<bool>,
    /// Per segment: the last frame its processing changed any temperature.
    touched: Vec<u32>,
    /// Per segment: the last frame it was fully inside the stepped range and
    /// changed nothing (whether it ran or was skipped).
    quiet: Vec<u32>,
}

impl HeatSleep {
    fn new(w: usize, h: usize) -> HeatSleep {
        let chunks = w.div_ceil(HEAT_CHUNK) * h;
        HeatSleep {
            snap_temp: vec![f32::NAN; w * h],
            snap_cells: vec![Species::Air; w * h],
            same: vec![false; chunks],
            touched: vec![0; chunks],
            quiet: vec![NEVER; chunks],
        }
    }
}

impl Grid {
    pub fn new(w: usize, h: usize, seed: u64, boil_kj: f32, melt_kj: f32) -> Grid {
        let n = w * h;
        Grid {
            w,
            h,
            cells: vec![Species::Air; n],
            temp: vec![AMBIENT_C; n],
            aux: vec![0; n],
            moved: vec![0; n],
            reacted: vec![0; n],
            frame: 0,
            rng: seed.max(1),
            boil_buffer: boil_kj * 1000.0 / WATER_CP_J_PER_MOL_K,
            melt_buffer: melt_kj * 1000.0 / WATER_CP_J_PER_MOL_K,
            wood_flame_chance: 0.25,
            heat_sleep: true,
            heat: HeatSleep::new(w, h),
        }
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h
    }

    /// Outside the map counts as stone, so nothing leaks off the edges.
    pub fn get(&self, x: i32, y: i32) -> Species {
        if self.in_bounds(x, y) {
            self.cells[self.idx(x as usize, y as usize)]
        } else {
            Species::Stone
        }
    }

    pub fn temp_at(&self, x: i32, y: i32) -> f32 {
        if self.in_bounds(x, y) {
            self.temp[self.idx(x as usize, y as usize)]
        } else {
            AMBIENT_C
        }
    }

    pub fn set(&mut self, x: i32, y: i32, s: Species, t: f32) {
        if self.in_bounds(x, y) {
            let i = self.idx(x as usize, y as usize);
            self.cells[i] = s;
            self.temp[i] = t;
            self.aux[i] = 0;
        }
    }

    /// Copies columns `x0..x1` (cells, temperatures and per-cell state) from
    /// another grid of the same size, such as a freshly built level.
    pub fn copy_columns(&mut self, from: &Grid, x0: usize, x1: usize) {
        debug_assert!(self.w == from.w && self.h == from.h);
        let x1 = x1.min(self.w);
        for y in 0..self.h {
            let (a, b) = (y * self.w + x0, y * self.w + x1);
            self.cells[a..b].copy_from_slice(&from.cells[a..b]);
            self.temp[a..b].copy_from_slice(&from.temp[a..b]);
            self.aux[a..b].copy_from_slice(&from.aux[a..b]);
        }
    }

    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, s: Species) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, s, AMBIENT_C);
            }
        }
    }

    pub fn count(&self, s: Species) -> usize {
        self.cells.iter().filter(|&&c| c == s).count()
    }

    pub fn rand_u32(&mut self) -> u32 {
        // xorshift64*
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        (self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        (self.rand_u32() as f32 / u32::MAX as f32) < p
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.cells.swap(i, j);
        self.temp.swap(i, j);
        self.aux.swap(i, j);
        self.moved[j] = self.frame;
        self.moved[i] = self.frame;
    }

    pub fn step(&mut self) {
        self.step_columns(0, self.w);
    }

    /// Steps only columns `x0..x1` (clamped to the grid). Everything outside is
    /// frozen until it is back in range; the range is a pure function of game
    /// state, so this stays deterministic.
    pub fn step_columns(&mut self, x0: usize, x1: usize) {
        let x1 = x1.min(self.w);
        let x0 = x0.min(x1);
        self.frame = self.frame.wrapping_add(1);
        self.move_pass(x0, x1);
        self.chemistry_pass(x0, x1);
        self.heat_pass(x0, x1);
    }

    // ---------------------------------------------------------------- movement

    fn move_pass(&mut self, x0: usize, x1: usize) {
        let left_to_right = self.frame.is_multiple_of(2);
        for y in (0..self.h).rev() {
            let row = y * self.w;
            for k in x0..x1 {
                let x = if left_to_right { k } else { x1 - 1 - (k - x0) };
                let i = row + x;
                let s = self.cells[i];
                // Cool air is the still background (see `move_gas`) and solids
                // never move: most cells end here without touching the RNG.
                let still = if s == Species::Air {
                    self.temp[i] < STILL_AIR_BELOW_C
                } else {
                    s.is_solid()
                };
                if still || self.moved[i] == self.frame {
                    continue;
                }
                match s.phase() {
                    Phase::Liquid => self.move_liquid(x as i32, y as i32),
                    Phase::Gas => self.move_gas(x as i32, y as i32),
                    Phase::Flame => self.move_flame(x as i32, y as i32),
                    Phase::Solid => {}
                }
            }
        }
    }

    fn passable_for_liquid(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && matches!(self.get(x, y).phase(), Phase::Gas | Phase::Flame)
    }

    fn move_liquid(&mut self, x: i32, y: i32) {
        let i = self.idx(x as usize, y as usize);
        let dir = if self.rand_u32() & 1 == 0 { 1 } else { -1 };
        for (dx, dy) in [(0, 1), (dir, 1), (-dir, 1)] {
            if self.passable_for_liquid(x + dx, y + dy) {
                let j = self.idx((x + dx) as usize, (y + dy) as usize);
                self.swap(i, j);
                return;
            }
        }
        // Spread sideways up to 3 cells so pools level out.
        for d in [dir, -dir] {
            let mut tx = x;
            for _ in 0..3 {
                if self.passable_for_liquid(tx + d, y) {
                    tx += d;
                } else {
                    break;
                }
            }
            if tx != x {
                let j = self.idx(tx as usize, y as usize);
                self.swap(i, j);
                return;
            }
        }
    }

    /// Effective molar mass: hot gas is less dense (ideal gas, density ~ M / T).
    fn buoyant_mass(&self, s: Species, t: f32) -> f32 {
        let kelvin = (t + 273.15).max(1.0);
        s.props().molar_mass * 293.15 / kelvin
    }

    /// Density relative to room-temperature air. Ideal gas from molar mass,
    /// except CO2, which uses its measured density (see `CO2_DENSITY_G_PER_L`).
    fn relative_density(&self, s: Species, t: f32) -> f32 {
        if s == Species::CO2 {
            let kelvin = (t + 273.15).max(1.0);
            CO2_DENSITY_G_PER_L / AIR_DENSITY_G_PER_L * 293.15 / kelvin
        } else {
            self.buoyant_mass(s, t) / AIR_MOLAR_MASS
        }
    }

    fn move_gas(&mut self, x: i32, y: i32) {
        let i = self.idx(x as usize, y as usize);
        let s = self.cells[i];
        let t = self.temp[i];
        // Cool air is the background medium and only moves when displaced.
        if s == Species::Air && t < STILL_AIR_BELOW_C {
            return;
        }
        let m = self.buoyant_mass(s, t);
        let rel = self.relative_density(s, t);
        let dir = if m < AIR_MOLAR_MASS - 1.0 {
            -1
        } else if m > AIR_MOLAR_MASS + 1.0 {
            1
        } else {
            0
        };
        let side = if self.rand_u32() & 1 == 0 { 1 } else { -1 };
        let urge = if dir < 0 {
            0.7
        } else {
            ((rel - 1.0) * SINK_GAIN).clamp(0.1, 0.9)
        };
        if dir != 0 && self.chance(urge) {
            for (dx, dy) in [(0, dir), (side, dir), (-side, dir)] {
                let (nx, ny) = (x + dx, y + dy);
                if !self.in_bounds(nx, ny) {
                    continue;
                }
                let other = self.get(nx, ny);
                if !other.is_gas() || other == s {
                    continue;
                }
                let om = self.buoyant_mass(other, self.temp_at(nx, ny));
                let rises_past = dir < 0 && om > m;
                let sinks_past = dir > 0 && om < m;
                if rises_past || sinks_past {
                    let j = self.idx(nx as usize, ny as usize);
                    self.swap(i, j);
                    return;
                }
            }
        }
        // Diffusion: gases slowly mix with neighbouring gases. Heavy gases
        // mix more slowly, so a CO2 pool stays a pool for a while.
        let mixing = if dir > 0 { 0.2 } else { 0.5 };
        if self.chance(mixing) {
            let (dx, dy) = NEIGHBORS[(self.rand_u32() % 4) as usize];
            let (nx, ny) = (x + dx, y + dy);
            if self.in_bounds(nx, ny) {
                let other = self.get(nx, ny);
                if other.is_gas() && other != s {
                    let j = self.idx(nx as usize, ny as usize);
                    self.swap(i, j);
                }
            }
        }
    }

    fn move_flame(&mut self, x: i32, y: i32) {
        if self.chance(0.4) && self.get(x, y - 1) == Species::Air && self.in_bounds(x, y - 1) {
            let i = self.idx(x as usize, y as usize);
            let j = self.idx(x as usize, (y - 1) as usize);
            self.swap(i, j);
        }
    }

    // --------------------------------------------------------------- chemistry

    fn neighbors_of(&self, x: i32, y: i32, s: Species) -> Option<(i32, i32)> {
        NEIGHBORS
            .iter()
            .map(|(dx, dy)| (x + dx, y + dy))
            .find(|&(nx, ny)| self.in_bounds(nx, ny) && self.get(nx, ny) == s)
    }

    fn has_neighbor(&self, x: i32, y: i32, s: Species) -> bool {
        self.neighbors_of(x, y, s).is_some()
    }

    fn hot_neighbor(&self, x: i32, y: i32, at_least: f32) -> bool {
        NEIGHBORS.iter().any(|(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            self.in_bounds(nx, ny)
                && (self.get(nx, ny) == Species::Flame || self.temp_at(nx, ny) >= at_least)
        })
    }

    /// The strongest oxidiser touching (x, y): pure O2 first, then air.
    fn oxidiser_near(&self, x: i32, y: i32) -> Option<(i32, i32, Species)> {
        self.neighbors_of(x, y, Species::O2)
            .map(|(a, b)| (a, b, Species::O2))
            .or_else(|| {
                self.neighbors_of(x, y, Species::Air)
                    .map(|(a, b)| (a, b, Species::Air))
            })
    }

    fn put(&mut self, x: i32, y: i32, s: Species, t: f32, aux: i16) {
        let i = self.idx(x as usize, y as usize);
        self.cells[i] = s;
        self.temp[i] = t;
        self.aux[i] = aux;
        self.reacted[i] = self.frame;
    }

    pub fn spawn_flame(&mut self, x: i32, y: i32, life: i16, leaves_co2: bool, t: f32) {
        if self.in_bounds(x, y) {
            let aux = if leaves_co2 {
                FLAME_LEAVES_CO2 + life
            } else {
                life
            };
            self.put(x, y, Species::Flame, t, aux);
        }
    }

    fn chemistry_pass(&mut self, x0: usize, x1: usize) {
        for y in 0..self.h as i32 {
            for x in x0 as i32..x1 as i32 {
                let i = self.idx(x as usize, y as usize);
                let s = self.cells[i];
                // Most cells (air, stone, crystal, settled gases) never react
                // on their own; test the species before touching `reacted`.
                if !s.reacts_alone() || self.reacted[i] == self.frame {
                    continue;
                }
                match s {
                    Species::Water => self.water(x, y, i),
                    Species::Ice => self.ice(i),
                    Species::Steam => self.steam(i),
                    Species::Wood => self.wood(x, y, i),
                    Species::H2 => self.hydrogen(x, y, i),
                    Species::CH4 => self.methane(x, y, i),
                    Species::Flame => self.flame(x, y, i),
                    _ => {}
                }
            }
        }
    }

    fn water(&mut self, x: i32, y: i32, i: usize) {
        // Water poured on a flame puts it out and soaks up some of its heat.
        if let Some((fx, fy)) = self.neighbors_of(x, y, Species::Flame) {
            let ft = self.temp_at(fx, fy);
            self.put(fx, fy, Species::Air, ft.min(150.0), 0);
            self.aux[i] = self.aux[i].saturating_add(40);
        }
        // Douse: water touching hot wood flashes to steam and soaks the wood
        // around it. Each cell stands for a spray of droplets, so it cools a
        // small patch (radius 3) instead of a single cell, paying for that
        // heat out of its own boiling budget.
        if NEIGHBORS.iter().any(|(dx, dy)| {
            self.get(x + dx, y + dy) == Species::Wood && self.temp_at(x + dx, y + dy) > 100.0
        }) {
            for dy in -3..=3 {
                for dx in -3..=3 {
                    let (nx, ny) = (x + dx, y + dy);
                    if self.get(nx, ny) == Species::Wood && self.in_bounds(nx, ny) {
                        let j = self.idx(nx as usize, ny as usize);
                        // Soaked wood ends up near the water's own temperature.
                        let excess = (self.temp[j] - 40.0).max(0.0);
                        self.temp[j] -= excess;
                        self.aux[i] = self.aux[i].saturating_add((excess * 0.25) as i16);
                    } else if self.get(nx, ny) == Species::Flame && self.in_bounds(nx, ny) {
                        let ft = self.temp_at(nx, ny);
                        self.put(nx, ny, Species::Air, ft.min(150.0), 0);
                    }
                }
            }
        }
        let t = self.temp[i];
        if t > 100.0 {
            self.aux[i] = self.aux[i].saturating_add((t - 100.0) as i16);
            self.temp[i] = 100.0;
        } else if t < 0.0 {
            self.aux[i] = self.aux[i].saturating_sub((-t) as i16);
            self.temp[i] = 0.0;
        }
        if self.aux[i] as f32 >= self.boil_buffer {
            self.put(x, y, Species::Steam, 100.0, 0);
        } else if (self.aux[i] as f32) <= -self.melt_buffer {
            self.put(x, y, Species::Ice, 0.0, 0);
        }
    }

    fn ice(&mut self, i: usize) {
        let t = self.temp[i];
        if t > 0.0 {
            self.aux[i] = self.aux[i].saturating_add(t as i16);
            self.temp[i] = 0.0;
            if self.aux[i] as f32 >= self.melt_buffer {
                self.cells[i] = Species::Water;
                self.aux[i] = 0;
                self.reacted[i] = self.frame;
            }
        }
    }

    fn steam(&mut self, i: usize) {
        let t = self.temp[i];
        if t < 100.0 {
            // Condensing releases latent heat, which holds the steam at 100 C
            // while its neighbours draw the heat away.
            self.aux[i] = self.aux[i].saturating_sub((100.0 - t) as i16);
            self.temp[i] = 100.0;
            if (self.aux[i] as f32) <= -self.boil_buffer {
                self.cells[i] = Species::Water;
                self.aux[i] = 0;
                self.reacted[i] = self.frame;
            }
        }
    }

    fn wood(&mut self, x: i32, y: i32, i: usize) {
        // Wet wood does not burn: water holds it near 100 C.
        if self.has_neighbor(x, y, Species::Water) {
            self.temp[i] = self.temp[i].min(100.0);
            return;
        }
        if self.temp[i] < WOOD_IGNITION_C {
            return;
        }
        let Some((ox, oy, oxidiser)) = self.oxidiser_near(x, y) else {
            // Smothered (gas touching it, but none with oxygen, such as a CO2
            // blanket): nothing burns, so no heat is released and the wood
            // slowly cools. Simplification: the loss stands in for heat soaking
            // into the ground and the gas around it.
            if NEIGHBORS
                .iter()
                .any(|(dx, dy)| self.get(x + dx, y + dy).is_gas())
            {
                self.temp[i] -= (self.temp[i] - AMBIENT_C) * 0.01;
            }
            return;
        };
        if self.chance(self.wood_flame_chance * oxidiser.oxidiser_strength()) {
            let life = 20 + (self.rand_u32() % 20) as i16;
            // A flame cell is mostly the air it burned in. Air is 21% O2, so only
            // about that share of air-fed flames leaves a whole cell of CO2.
            let leaves_co2 = oxidiser == Species::O2 || self.chance(oxidiser.oxidiser_strength());
            self.spawn_flame(ox, oy, life, leaves_co2, WOOD_FIRE_C);
            self.aux[i] += 1;
            self.temp[i] = self.temp[i].max(600.0);
            if self.aux[i] > 60 {
                self.put(x, y, Species::Air, 600.0, 0);
            }
        }
    }

    /// 2H2 + O2 -> 2H2O. Each H2 cell becomes one steam cell; an O2 cell is used
    /// up by two H2 cells, so the cell counts follow the equation.
    fn hydrogen(&mut self, x: i32, y: i32, i: usize) {
        if !(self.temp[i] >= H2_IGNITION_C || self.hot_neighbor(x, y, H2_IGNITION_C)) {
            return;
        }
        let Some((ox, oy, oxidiser)) = self.oxidiser_near(x, y) else {
            return;
        };
        match oxidiser {
            Species::O2 => {
                self.put(x, y, Species::Steam, FLAME_C_IN_O2, 0);
                let j = self.idx(ox as usize, oy as usize);
                if self.aux[j] >= 1 {
                    self.spawn_flame(ox, oy, 6, false, FLAME_C_IN_O2);
                } else {
                    self.aux[j] = 1;
                    self.temp[j] = FLAME_C_IN_O2;
                }
            }
            _ => {
                if self.chance(oxidiser.oxidiser_strength()) {
                    self.put(x, y, Species::Steam, FLAME_C_IN_AIR, 0);
                    self.spawn_flame(ox, oy, 4, false, FLAME_C_IN_AIR);
                }
            }
        }
    }

    /// CH4 + 2O2 -> CO2 + 2H2O. Each O2 used becomes a steam cell; after two,
    /// the methane cell becomes CO2.
    fn methane(&mut self, x: i32, y: i32, i: usize) {
        if !(self.temp[i] >= CH4_IGNITION_C || self.hot_neighbor(x, y, CH4_IGNITION_C)) {
            return;
        }
        let Some((ox, oy, oxidiser)) = self.oxidiser_near(x, y) else {
            return;
        };
        let flame_t = if oxidiser == Species::O2 {
            FLAME_C_IN_O2
        } else {
            FLAME_C_IN_AIR
        };
        if oxidiser == Species::Air && !self.chance(oxidiser.oxidiser_strength()) {
            return;
        }
        self.put(ox, oy, Species::Steam, flame_t, 0);
        self.aux[i] += 1;
        self.temp[i] = flame_t;
        if self.aux[i] >= 2 {
            self.put(x, y, Species::CO2, flame_t, 0);
        }
    }

    fn flame(&mut self, x: i32, y: i32, i: usize) {
        let leaves_co2 = self.aux[i] >= FLAME_LEAVES_CO2;
        // Without oxygen a flame chokes; away from fuel it burns out quickly.
        let smothered = self.oxidiser_near(x, y).is_none();
        let fed = leaves_co2 && self.has_neighbor(x, y, Species::Wood);
        self.aux[i] -= if smothered {
            4
        } else if fed || !leaves_co2 {
            1
        } else {
            3
        };
        self.temp[i] = self.temp[i].max(WOOD_FIRE_C);
        let life = if leaves_co2 {
            self.aux[i] - FLAME_LEAVES_CO2
        } else {
            self.aux[i]
        };
        if life <= 0 {
            let t = self.temp[i] * 0.5;
            self.put(
                x,
                y,
                if leaves_co2 {
                    Species::CO2
                } else {
                    Species::Air
                },
                t,
                0,
            );
        }
    }

    // -------------------------------------------------------------------- heat

    fn heat_pass(&mut self, x0: usize, x1: usize) {
        if !self.heat_sleep {
            for y in 0..self.h {
                self.heat_segment(y, x0, x1, x1);
            }
            return;
        }
        let (w, h, f) = (self.w, self.h, self.frame);
        let per_row = w.div_ceil(HEAT_CHUNK);
        let (c0, c1) = (x0 / HEAT_CHUNK, x1.div_ceil(HEAT_CHUNK));
        // Compare the covered cells with the snapshot, then refresh it.
        for y in 0..h {
            for c in c0..c1 {
                let a = y * w + (c * HEAT_CHUNK).max(x0);
                let b = y * w + ((c + 1) * HEAT_CHUNK).min(x1);
                let hs = &mut self.heat;
                // Branch-free folds over bits, so they vectorise.
                let diff = self.temp[a..b]
                    .iter()
                    .zip(&hs.snap_temp[a..b])
                    .fold(0u32, |acc, (t, s)| acc | (t.to_bits() ^ s.to_bits()));
                let diff_cells = self.cells[a..b]
                    .iter()
                    .zip(&hs.snap_cells[a..b])
                    .fold(0u8, |acc, (c, s)| acc | (*c as u8 ^ *s as u8));
                hs.same[y * per_row + c] = diff == 0 && diff_cells == 0;
                hs.snap_temp[a..b].copy_from_slice(&self.temp[a..b]);
                hs.snap_cells[a..b].copy_from_slice(&self.cells[a..b]);
            }
        }
        let prev = f.wrapping_sub(1);
        let calm = |hs: &HeatSleep, k: usize| hs.touched[k] != f && hs.touched[k] != prev;
        for y in 0..h {
            for c in c0..c1 {
                let k = y * per_row + c;
                let (sx0, sx1) = (c * HEAT_CHUNK, ((c + 1) * HEAT_CHUNK).min(w));
                // Fully inside, with the right-hand neighbour cell inside too,
                // so the segment does exactly the same work every step.
                let interior = sx0 >= x0 && sx1 < x1;
                let hs = &self.heat;
                // The segment reads its own cells, the row below and the cell
                // to its right; it is written by the segment to its left
                // (right exchange) and the two above it (down exchanges).
                // Those writers must have been calm this step and last step
                // (last step's writes would make last step's inputs differ
                // from the snapshot even when this step's match it).
                let skip = interior
                    && hs.quiet[k] == prev
                    && hs.same[k]
                    && hs.same[k + 1]
                    && (y + 1 == h || hs.same[k + per_row])
                    && (c == 0 || calm(hs, k - 1))
                    && (y == 0 || (calm(hs, k - per_row) && calm(hs, k - per_row + 1)));
                let changed = !skip && self.heat_segment(y, sx0.max(x0), sx1.min(x1), x1);
                if changed {
                    self.heat.touched[k] = f;
                } else if interior {
                    self.heat.quiet[k] = f;
                }
            }
        }
    }

    /// Conducts heat for cells `xa..xb` of row `y` (each to its right and
    /// downward neighbour, right only while inside `x_end`), and relaxes open
    /// air toward ambient. Returns whether any temperature may have changed.
    #[inline]
    fn heat_segment(&mut self, y: usize, xa: usize, xb: usize, x_end: usize) -> bool {
        let w = self.w;
        let row = y * w;
        let rows = if y + 1 < self.h { 2 } else { 1 };
        // Row slices (this row, and the one below when there is one) so the
        // loop indexes short slices it can prove are in range.
        let temp = &mut self.temp[row..row + rows * w];
        let cells = &self.cells[row..row + rows * w];
        let (t_row, t_below) = temp.split_at_mut(w);
        let (c_row, c_below) = cells.split_at(w);
        let x_end = x_end.min(w);
        let xb = xb.min(x_end);
        let mut changed = false;
        for x in xa..xb {
            let si = c_row[x];
            // This cell's temperature stays in a register; the arithmetic
            // and its order are exactly those of `exchange`, so results are
            // bit-for-bit what they were.
            let mut ti = t_row[x];
            // Right neighbour, then the one below.
            if x + 1 < x_end {
                let tj = t_row[x + 1];
                let dt = tj - ti;
                if dt.abs() >= HEAT_EPSILON_C {
                    let sj = c_row[x + 1];
                    let q = si.conductivity().min(sj.conductivity()) * dt;
                    ti += q / si.heat_capacity();
                    t_row[x + 1] = tj - q / sj.heat_capacity();
                    changed = true;
                }
            }
            if let (Some(&tj), Some(&sj)) = (t_below.get(x), c_below.get(x)) {
                let dt = tj - ti;
                if dt.abs() >= HEAT_EPSILON_C {
                    let q = si.conductivity().min(sj.conductivity()) * dt;
                    ti += q / si.heat_capacity();
                    t_below[x] = tj - q / sj.heat_capacity();
                    changed = true;
                }
            }
            // Open air loses heat to the wider world. Air exactly at
            // ambient (most of the sky) would add zero, so it is skipped.
            if si == Species::Air {
                let d = AMBIENT_C - ti;
                if d != 0.0 {
                    let t = ti + d * 0.01;
                    changed |= t.to_bits() != ti.to_bits();
                    ti = t;
                }
            }
            t_row[x] = ti;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::new(32, 32, 7, 40.65, 6.01)
    }

    /// Heat-pass sleeping against the plain pass on random scenes: sparse hot
    /// spots poked in from outside, species swapped, and a stepped column
    /// range that slides and resizes. Temperatures must match bit for bit.
    #[test]
    fn heat_sleep_is_exact_under_random_writes_and_ranges() {
        let (w, h) = (300, 24);
        let mut a = Grid::new(w, h, 1, 40.65, 6.01);
        a.fill_rect(0, 18, w as i32, h as i32, Species::Stone);
        a.fill_rect(70, 10, 140, 14, Species::Wood);
        a.fill_rect(150, 12, 200, 18, Species::Water);
        let mut b = Grid::new(w, h, 1, 40.65, 6.01);
        b.cells.clone_from(&a.cells);
        b.heat_sleep = false;
        let mut rng = 12345u64;
        let mut next = |n: usize| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng % n as u64) as usize
        };
        let (mut x0, mut x1) = (0, w);
        let mut slept = 0;
        for step in 0..6000 {
            if step % 300 == 0 {
                x0 = next(120);
                x1 = (x0 + 64 + next(w)).min(w);
            }
            // A few writes from outside the heat pass, most steps none.
            // Rare, so most segments settle and sleep; half land on the
            // columns either side of a segment edge, where mistakes would hide.
            if next(20) == 0 {
                let x = if next(2) == 0 {
                    (HEAT_CHUNK * (1 + next(4)) + next(3))
                        .saturating_sub(1)
                        .min(w - 1)
                } else {
                    next(w)
                };
                let y = next(h);
                let t = [AMBIENT_C, AMBIENT_C, 20.005, 20.5, 35.0][next(5)];
                let sp = [Species::Air, Species::Stone, Species::Water][next(3)];
                for g in [&mut a, &mut b] {
                    g.set(x as i32, y as i32, sp, t);
                }
            }
            for g in [&mut a, &mut b] {
                g.frame = g.frame.wrapping_add(1);
                g.heat_pass(x0, x1);
            }
            let f = a.frame;
            slept += a.heat.quiet.iter().filter(|&&q| q == f).count();
            for i in 0..w * h {
                assert_eq!(
                    a.temp[i].to_bits(),
                    b.temp[i].to_bits(),
                    "step {step}, cell ({}, {}), range {x0}..{x1}",
                    i % w,
                    i / w
                );
            }
        }
        // The test only means something if segments really did sleep.
        assert!(slept > 20_000, "only {slept} quiet segment-steps");
    }

    /// The cases a sleeping segment is most likely to get wrong: a quiet
    /// segment whose only changed input is a neighbour's cell (the one to
    /// its right, or the row below), poked from outside between steps.
    #[test]
    fn heat_sleep_wakes_for_writes_just_outside_a_segment() {
        let (w, h) = (256, 8);
        for (px, py) in [(64, 3), (128, 0), (100, 5), (63, 0), (64, 0), (191, 7)] {
            let mut a = Grid::new(w, h, 1, 40.65, 6.01);
            a.fill_rect(0, 0, w as i32, h as i32, Species::Stone);
            let mut b = Grid::new(w, h, 1, 40.65, 6.01);
            b.cells.clone_from(&a.cells);
            b.heat_sleep = false;
            for step in 0..12 {
                if step == 6 {
                    for g in [&mut a, &mut b] {
                        g.set(px, py, Species::Stone, 500.0);
                    }
                }
                for g in [&mut a, &mut b] {
                    g.frame = g.frame.wrapping_add(1);
                    g.heat_pass(0, w);
                }
                for i in 0..w * h {
                    assert_eq!(
                        a.temp[i].to_bits(),
                        b.temp[i].to_bits(),
                        "poke ({px}, {py}), step {step}, cell {i}"
                    );
                }
            }
        }
    }

    #[test]
    fn latent_heat_buffers_come_from_real_values() {
        let g = grid();
        assert!((g.boil_buffer - 539.8).abs() < 1.0);
        assert!((g.melt_buffer - 79.8).abs() < 1.0);
    }

    #[test]
    fn water_falls_and_settles_on_stone() {
        let mut g = grid();
        g.fill_rect(0, 30, 32, 32, Species::Stone);
        g.set(10, 2, Species::Water, AMBIENT_C);
        for _ in 0..60 {
            g.step();
        }
        assert_eq!(g.count(Species::Water), 1);
        assert_eq!(
            g.cells.iter().position(|&c| c == Species::Water).unwrap() / 32,
            29
        );
    }

    #[test]
    fn hydrogen_rises_and_co2_sinks() {
        let mut g = grid();
        g.set(10, 16, Species::H2, AMBIENT_C);
        g.set(20, 16, Species::CO2, AMBIENT_C);
        for _ in 0..200 {
            g.step();
        }
        let y_of = |g: &Grid, s| g.cells.iter().position(|&c| c == s).unwrap() / 32;
        assert!(y_of(&g, Species::H2) < 16);
        assert!(y_of(&g, Species::CO2) > 16);
    }

    #[test]
    fn hydrogen_and_oxygen_make_steam_in_the_right_ratio() {
        let mut g = grid();
        // Two H2 next to one O2, ignited by heat.
        g.set(10, 10, Species::H2, 600.0);
        g.set(11, 10, Species::O2, 600.0);
        g.set(12, 10, Species::H2, 600.0);
        for _ in 0..3 {
            g.step();
        }
        assert_eq!(g.count(Species::H2), 0);
        assert_eq!(g.count(Species::O2), 0);
        assert_eq!(g.count(Species::Steam) + g.count(Species::Water), 2);
    }

    #[test]
    fn hot_water_boils_into_steam() {
        let mut g = grid();
        g.set(10, 10, Species::Water, 100.0);
        let i = g.idx(10, 10);
        g.temp[i] = 700.0;
        g.step();
        assert_eq!(g.count(Species::Steam), 1);
    }

    #[test]
    fn cold_water_freezes() {
        let mut g = grid();
        g.fill_rect(0, 0, 32, 32, Species::Stone);
        g.set(10, 10, Species::Water, -150.0);
        g.step();
        assert_eq!(g.count(Species::Ice), 1);
    }

    #[test]
    fn hot_wood_in_air_catches_fire() {
        let mut g = grid();
        g.fill_rect(8, 20, 14, 30, Species::Wood);
        for y in 20..30 {
            for x in 8..14 {
                let i = g.idx(x, y);
                g.temp[i] = 500.0;
            }
        }
        let mut saw_flame = false;
        for _ in 0..60 {
            g.step();
            saw_flame |= g.count(Species::Flame) > 0;
        }
        assert!(saw_flame);
    }

    #[test]
    fn same_seed_gives_same_world() {
        let run = || {
            let mut g = grid();
            g.fill_rect(0, 28, 32, 32, Species::Stone);
            g.fill_rect(4, 4, 20, 10, Species::Water);
            g.fill_rect(22, 20, 26, 24, Species::H2);
            for _ in 0..100 {
                g.step();
            }
            g.cells.iter().map(|&c| c as u8).collect::<Vec<u8>>()
        };
        assert_eq!(run(), run());
    }
}
