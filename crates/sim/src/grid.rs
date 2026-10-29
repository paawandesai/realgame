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

/// Air cooler than this is the background medium and only moves when displaced.
const STILL_AIR_BELOW_C: f32 = 60.0;

/// Neighbouring cells closer in temperature than this exchange no heat; the
/// transfer would be far below anything visible or gameplay-relevant.
const HEAT_EPSILON_C: f32 = 0.01;

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
            for k in x0..x1 {
                let x = if left_to_right { k } else { x1 - 1 - (k - x0) };
                let i = self.idx(x, y);
                let s = self.cells[i];
                // Cool air is the still background (see `move_gas`) and solids
                // never move: most cells end here without touching the RNG.
                if (s == Species::Air && self.temp[i] < STILL_AIR_BELOW_C)
                    || s.is_solid()
                    || self.moved[i] == self.frame
                {
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

    fn move_gas(&mut self, x: i32, y: i32) {
        let i = self.idx(x as usize, y as usize);
        let s = self.cells[i];
        let t = self.temp[i];
        // Cool air is the background medium and only moves when displaced.
        if s == Species::Air && t < STILL_AIR_BELOW_C {
            return;
        }
        let m = self.buoyant_mass(s, t);
        let dir = if m < AIR_MOLAR_MASS - 1.0 {
            -1
        } else if m > AIR_MOLAR_MASS + 1.0 {
            1
        } else {
            0
        };
        let side = if self.rand_u32() & 1 == 0 { 1 } else { -1 };
        let urge = if dir < 0 { 0.7 } else { 0.25 };
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
        // Diffusion: gases slowly mix with neighbouring gases.
        if self.chance(0.5) {
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
                if self.reacted[i] == self.frame {
                    continue;
                }
                match self.cells[i] {
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
            return;
        };
        if self.chance(0.25 * oxidiser.oxidiser_strength()) {
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
        for y in 0..self.h {
            for x in x0..x1 {
                let i = self.idx(x, y);
                let si = self.cells[i];
                for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                    if nx >= x1 || ny >= self.h {
                        continue;
                    }
                    let j = self.idx(nx, ny);
                    let dt = self.temp[j] - self.temp[i];
                    if dt.abs() < HEAT_EPSILON_C {
                        continue;
                    }
                    let sj = self.cells[j];
                    let k = si.conductivity().min(sj.conductivity());
                    let q = k * dt;
                    self.temp[i] += q / si.heat_capacity();
                    self.temp[j] -= q / sj.heat_capacity();
                }
                // Open air loses heat to the wider world.
                if self.cells[i] == Species::Air {
                    self.temp[i] += (AMBIENT_C - self.temp[i]) * 0.01;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::new(32, 32, 7, 40.65, 6.01)
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
