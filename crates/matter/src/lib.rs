//! Block chemistry on the voxel world (I-052, spec section 5.3-5.7): heat,
//! phase changes, fire, volume-conserving water, the coarse 2 m gas field
//! (CO2 sinks, H2 and CH4 rise), flame smothering, the glass and iron rules
//! and balloon lift. Pure Rust, no Bevy, deterministic: the same seed plus
//! the same emits give a bit-identical `hash` on native and wasm.
//!
//! **Determinism rules** (section 3): fixed iteration order (chunk index,
//! then cell index); no `HashMap` iteration in a step; no `sin`, `exp` or
//! `powf` inside a step (tables and `+ - * /` only); the seeded xorshift RNG
//! lives in `World`.
//!
//! **Contract rules** (section 10): cells are read and written only through
//! `realgame_voxel::World::{get, set}`; this crate never meshes. Reaction
//! ids and gas densities come from `realgame_sim` (`reactions`, `species`).
//!
//! The contract step fixed the types and signatures; Lane B filled `step` and
//! `lift_n` (spec section 12). See `docs/loop/items/I-052/lanes/B/`.

#![deny(unsafe_code)]

use realgame_voxel as voxel;

/// Fixed step: 10 steps per second.
pub const DT: f32 = 0.1;
/// A Water or Frost hit knocks a gas vent's flame down for this long, then it
/// relights (the value from the client's `chem.rs`).
pub const FLICKER_S: f32 = 1.5;

/// What a landed spell adds to the world (section 5.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmitKind {
    /// Liquid water: 8 liquid levels at the landing cell (24 per cast at most).
    Water,
    /// Liquid nitrogen (−196 °C): cools a 1.5-block radius by a set energy.
    Frost,
    /// Methane flame: heats a 1-block radius to 1100 °C for 0.5 s and adds
    /// CO2 and H2O to the gas cell.
    Fire,
    /// CO2 foam: 1 m³ of cold CO2 at the hit, which sinks.
    Co2,
}

/// One landed spell, built by the client's `blockchem::on_impact`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emit {
    /// Hit point in world metres.
    pub pos: [f32; 3],
    /// The face that was hit (unit axis vector).
    pub normal: [i8; 3],
    pub what: EmitKind,
    /// Scale of the cast (1.0 for one ordinary cast).
    pub amount: f32,
}

/// What put a fire out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum By {
    Water,
    Frost,
    /// Its gas cell's O2 fell below 15 %.
    NoOxygen,
}

/// Something the matter step did, for captions, sounds and the quest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatterEvent {
    Melted { cell: [i32; 3] },
    Froze { cell: [i32; 3] },
    Ignited { cell: [i32; 3] },
    Extinguished { cell: [i32; 3], by: By },
    Smothered { vent: [i32; 3] },
    Relit { vent: [i32; 3] },
    Glass { cell: [i32; 3] },
    Iron { cell: [i32; 3] },
    Leidenfrost { pos: [f32; 3] },
    Flowed { chunk: usize },
}

/// The gases in the coarse field (they are not blocks).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gas {
    Co2,
    H2,
    Ch4,
    O2,
}

pub mod gas;
pub mod props;

use std::collections::{BTreeMap, BTreeSet};

use props::{Props, BOIL_K, LF_J_KG, LV_J_KG, MELT_K};
use voxel::{mat, Cell, CHUNK_CELLS};

/// Cell updates per step in each pass (section 5.3). Work over a cap carries
/// over to the next step.
pub const HEAT_CAP: usize = 8000;
pub const FIRE_CAP: usize = 2000;
pub const WATER_CAP: usize = 4000;
pub const GAS_CAP: usize = 6000;

/// A wood fire's temperature, about 1100 °C (`grid.rs` `WOOD_FIRE_C`).
pub const FIRE_K: f32 = 1100.0 + 273.15;
/// A flame needs about 15 % oxygen.
pub const SMOTHER_O2: f32 = 0.15;
/// Sand melts into glass at this temperature: silica melts at 1713 °C, the
/// sand row's `melt_c` (a unit test keeps them equal).
pub const GLASS_K: f32 = 1713.0 + 273.15;
/// Iron ore touching coal is reduced to iron at this temperature.
pub const IRON_K: f32 = 1200.0 + 273.15;
/// Liquid nitrogen's boiling point: Frost never cools below it.
pub const LN2_K: f32 = 77.35;
/// Water hotter than this shows Leidenfrost beads under a Frost hit.
pub const LEIDENFROST_K: f32 = 10.0 + 273.15;
/// Radius (m, to cell centres) that Water, Frost and Fire emits act on.
pub const EMIT_RADIUS: f32 = 1.5;
/// Gameplay tuning: energy a Frost emit takes from each cell in its radius
/// (enough to freeze one full block of 12 °C water).
pub const FROST_J: f32 = 4.0e8;
/// Gameplay tuning: energy a Fire emit gives each cell in its radius per
/// step, for `FIRE_EMIT_STEPS` steps (melts one ice block per cast, without
/// boiling the meltwater).
pub const FIRE_EMIT_J: f32 = 9.0e7;
pub const FIRE_EMIT_STEPS: u8 = 5;
/// Gameplay tuning: energy a burning block gives each touching block per step
/// (lights touching wood in about 2 s).
pub const BURN_J: f32 = 1.6e7;
/// Gameplay tuning: CO2 in one CO2 emit, m³ (spec 5.5: three casts fill the
/// methane hollow; see the round notes).
pub const CO2_EMIT_M3: f32 = 23.0;
/// CO2 units a Fire emit adds (methane burns to CO2 and water).
pub const FIRE_CO2_UNITS: u32 = 8;
/// Water levels in one Water emit, and the most one cast can add.
pub const WATER_EMIT_LEVELS: f32 = 8.0;
pub const WATER_EMIT_MAX: u32 = 24;
/// Steam rises one cell per step and vanishes after 5 s.
pub const STEAM_TICKS: u16 = 50;
/// Gameplay tuning: conduction runs this many times faster than real 1 m
/// blocks would (real conduction through a metre of ice takes days).
pub const CONDUCTION_X: f32 = 2.0e4;
/// Gameplay tuning: share of its distance from rest a hot or cold cell loses
/// each step (about a 10 s time constant).
pub const REST_PULL: f32 = 0.01;

const FACES: [[i32; 3]; 6] = [
    [0, -1, 0],
    [-1, 0, 0],
    [1, 0, 0],
    [0, 0, -1],
    [0, 0, 1],
    [0, 1, 0],
];
const SIDES: [[i32; 3]; 4] = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];

fn add(a: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [a[0] + d[0], a[1] + d[1], a[2] + d[2]]
}

/// A cell's key: chunk index, then cell index, so ordered maps iterate in
/// the spec's fixed order.
fn key(p: [i32; 3]) -> Option<u32> {
    voxel::World::chunk_index(p).map(|c| (c * CHUNK_CELLS + voxel::World::cell_index(p)) as u32)
}

fn pos(k: u32) -> [i32; 3] {
    let c = k as usize / CHUNK_CELLS;
    let i = (k as usize % CHUNK_CELLS) as i32;
    let o = voxel::World::chunk_origin(c);
    let n = voxel::CHUNK;
    [o[0] + i % n, o[1] + (i / n) % n, o[2] + i / (n * n)]
}

/// How much work each pass did in the last step (M11 and `matterMs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub heat: usize,
    pub fire: usize,
    /// Burning cells left for later steps by the fire cap.
    pub fire_carried: usize,
    pub water: usize,
    pub gas: usize,
}

/// A Fire emit's flame, burning on for a few steps where it landed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Flame {
    pos: [f32; 3],
    left: u8,
}

/// The matter simulation's own state: the gas field, pending emits, the
/// active cells, carry-over cursors and the RNG. The blocks themselves live
/// in `realgame_voxel::World`.
#[derive(Clone, Debug)]
pub struct World {
    rng: u64,
    ticks: u64,
    pending: Vec<Emit>,
    touched: Vec<[i32; 3]>,
    scanned: bool,
    props: Vec<Props>,
    solid: Vec<bool>,
    /// Cells off their resting temperature, in kelvin.
    temps: BTreeMap<u32, f32>,
    /// Phase-change energy stored at the melting or boiling point, J: for ice
    /// the melt so far; for water negative toward ice, positive toward steam.
    latent: BTreeMap<u32, f32>,
    /// Burning cells and the steps each has left.
    fires: BTreeMap<u32, u16>,
    /// Gas vents and their flicker countdown (steps).
    vents: BTreeMap<u32, u16>,
    /// Liquid cells that may move.
    liquid: BTreeSet<u32>,
    /// Liquid cells' steps without change (two settles them).
    idle: BTreeMap<u32, u8>,
    steam: BTreeMap<u32, u16>,
    flames: Vec<Flame>,
    gas: gas::Field,
    cursor: [u32; 3],
    gas_cursor: usize,
    stats: Stats,
}

impl World {
    /// A matter world with no gas and nothing active, seeded for its RNG.
    pub fn new(seed: u64) -> World {
        let props = props::table();
        let solid = props.iter().map(|p| p.solid).collect();
        World {
            // xorshift must not start at 0.
            rng: seed ^ 0x9e37_79b9_7f4a_7c15,
            ticks: 0,
            pending: Vec::new(),
            touched: Vec::new(),
            scanned: false,
            props,
            solid,
            temps: BTreeMap::new(),
            latent: BTreeMap::new(),
            fires: BTreeMap::new(),
            vents: BTreeMap::new(),
            liquid: BTreeSet::new(),
            idle: BTreeMap::new(),
            steam: BTreeMap::new(),
            flames: Vec::new(),
            gas: gas::Field::default(),
            cursor: [0; 3],
            gas_cursor: 0,
            stats: Stats::default(),
        }
    }

    /// Queues a landed spell; it enters the world on the next `step`.
    pub fn emit(&mut self, e: Emit) {
        self.pending.push(e);
    }

    /// Tells the sim a cell changed outside it (a dig, a placed block, a
    /// debug `setcell`): on the next step it and its 6 neighbours are looked
    /// at again (water wakes, a placed hot block starts cooling, a burning
    /// block starts burning).
    pub fn touch(&mut self, _vox: &voxel::World, p: [i32; 3]) {
        self.touched.push(p);
    }

    /// Steps taken so far.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// What the last step did.
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// Chunks with any matter work (hot, burning, moving water, vents,
    /// steam, gas): the `activeChunks` state key.
    pub fn active_chunks(&self) -> usize {
        let mut on = [false; 192];
        let keys = self
            .temps
            .keys()
            .chain(self.fires.keys())
            .chain(self.vents.keys())
            .chain(self.liquid.iter())
            .chain(self.steam.keys());
        for &k in keys {
            on[k as usize / CHUNK_CELLS] = true;
        }
        for c in self.gas.active_chunks() {
            on[c] = true;
        }
        on.iter().filter(|&&b| b).count()
    }

    /// Steps a freshly lit block of `m` burns.
    pub fn burn_full(&self, m: u8) -> u16 {
        self.props[m as usize].burn_ticks
    }

    /// Steps the burning cell at `p` has left, if it burns.
    pub fn burn_left(&self, p: [i32; 3]) -> Option<u16> {
        key(p).and_then(|k| self.fires.get(&k).copied())
    }

    /// Burning blocks (not gas vents), in key order.
    pub fn burning_cells(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        self.fires.keys().map(|&k| pos(k))
    }

    /// Gas vents whose flame is lit now, in key order.
    pub fn lit_vents<'a>(&'a self, vox: &'a voxel::World) -> impl Iterator<Item = [i32; 3]> + 'a {
        self.vents
            .keys()
            .map(|&k| pos(k))
            .filter(|&v| vox.get(v).has(Cell::BURNING))
    }

    /// Temperature of the cell at `p` in °C (the `probe` debug command): an
    /// untouched cell reads its material's resting temperature.
    pub fn temp_c(&self, vox: &voxel::World, p: [i32; 3]) -> f32 {
        self.temp_of(vox, p) - 273.15
    }

    /// CO2, H2, CH4 and extra-O2 fractions (0-1) of the gas cell holding
    /// block `p`.
    pub fn gas_at(&self, p: [i32; 3]) -> [f32; 4] {
        self.gas
            .get(gas::of_block(p))
            .map(|v| v as f32 / gas::FULL as f32)
    }

    /// The oxygen fraction (0-1) at block `p`: 0.21 in plain air.
    pub fn o2_at(&self, p: [i32; 3]) -> f32 {
        self.gas.o2(gas::of_block(p))
    }

    /// The largest CO2 fraction in the chunk holding block `p` (the
    /// `gasCO2Max` state key).
    pub fn co2_max_in_chunk(&self, p: [i32; 3]) -> f32 {
        let Some(c) = voxel::World::chunk_index(p) else {
            return 0.0;
        };
        let o = voxel::World::chunk_origin(c);
        let mut m = 0u8;
        for z in (0..voxel::CHUNK).step_by(2) {
            for y in (0..voxel::CHUNK).step_by(2) {
                for x in (0..voxel::CHUNK).step_by(2) {
                    m = m.max(self.gas.get(gas::of_block(add(o, [x, y, z])))[gas::CO2]);
                }
            }
        }
        m as f32 / gas::FULL as f32
    }

    /// Adds `m3` cubic metres of a gas at `p` (the rig's flasks and balloon,
    /// tests). It fills the gas cell there and spreads outward if full.
    pub fn add_gas(&mut self, vox: &voxel::World, p: [f32; 3], g: Gas, m3: f32) {
        let s = match g {
            Gas::Co2 => gas::CO2,
            Gas::H2 => gas::H2,
            Gas::Ch4 => gas::CH4,
            Gas::O2 => gas::O2,
        };
        let b = p.map(|v| v.floor() as i32);
        let units = (m3 * gas::FULL as f32 / 8.0).round().max(0.0) as u32;
        self.gas.add(vox, &self.solid, gas::of_block(b), s, units);
    }

    /// One fixed step of `DT` on the active cells (emits, heat, phase, fire,
    /// vents, steam, water, gas, in that order, each with its cap), appending
    /// what happened.
    pub fn step(&mut self, vox: &mut voxel::World, events: &mut Vec<MatterEvent>) {
        self.stats = Stats::default();
        if !self.scanned {
            self.scan(vox);
            self.scanned = true;
        }
        for p in std::mem::take(&mut self.touched) {
            for q in std::iter::once(p).chain(FACES.iter().map(|&d| add(p, d))) {
                self.classify(vox, q, true);
            }
        }
        for e in std::mem::take(&mut self.pending) {
            self.apply_emit(vox, e, events);
        }
        self.flames_step(vox, events);
        self.heat(vox, events);
        self.fire(vox, events);
        self.vents_step(vox, events);
        self.steam_step(vox);
        self.water(vox, events);
        self.stats.gas = self
            .gas
            .step(vox, &self.solid, self.ticks, &mut self.gas_cursor, GAS_CAP);
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.ticks += 1;
    }

    /// The test hook (M10): a hash of the blocks and the matter state. Equal
    /// on native and wasm for the same seed and emits.
    pub fn hash(&self, vox: &voxel::World) -> u64 {
        let mut h = vox.hash();
        let mut eat = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        eat(self.rng);
        eat(self.pending.len() as u64);
        for (&k, &t) in &self.temps {
            eat(k as u64);
            eat(t.to_bits() as u64);
        }
        for (&k, &l) in &self.latent {
            eat(k as u64);
            eat(l.to_bits() as u64);
        }
        for (&k, &v) in self
            .fires
            .iter()
            .chain(self.vents.iter())
            .chain(self.steam.iter())
        {
            eat(k as u64);
            eat(v as u64);
        }
        for &k in &self.liquid {
            eat(k as u64);
        }
        gas::hash_into(&self.gas, &mut h);
        h
    }

    // -----------------------------------------------------------------
    // Bookkeeping
    // -----------------------------------------------------------------

    fn rest(&self, m: u8) -> f32 {
        self.props[m as usize].rest_k
    }

    fn temp_of(&self, vox: &voxel::World, p: [i32; 3]) -> f32 {
        let c = vox.get(p);
        match key(p).and_then(|k| self.temps.get(&k)) {
            Some(&t) => t,
            None if c.temp != 0 => c.temp_k(),
            None => self.rest(c.mat),
        }
    }

    /// Heat capacity of the cell (J/K); water scales with its level.
    fn cap(&self, c: Cell) -> f32 {
        let p = self.props[c.mat as usize];
        if c.mat == mat::WATER {
            p.c_vol * (c.level().max(1) as f32 / 8.0)
        } else {
            p.c_vol
        }
    }

    fn flammable(&self, m: u8) -> bool {
        self.props[m as usize].ignite_k.is_some()
    }

    /// Looks at one cell and puts it on the lists it belongs on. A cell's
    /// stored `temp` moves into the sim (and the cell's field goes back to
    /// 0), so the mesher is not woken every step by heat alone.
    fn classify(&mut self, vox: &mut voxel::World, p: [i32; 3], wake: bool) {
        let Some(k) = key(p) else {
            return;
        };
        let mut c = vox.get(p);
        if c.temp != 0 {
            self.temps.insert(k, c.temp_k());
            c.temp = 0;
            vox.set(p, c);
        }
        match c.mat {
            mat::WATER if wake || !c.has(Cell::SETTLED) => {
                self.liquid.insert(k);
            }
            mat::GAS_VENT => {
                self.vents.entry(k).or_insert(0);
            }
            mat::STEAM => {
                self.steam.entry(k).or_insert(STEAM_TICKS);
            }
            m if c.has(Cell::BURNING) && self.flammable(m) => {
                let full = self.burn_full(m);
                self.fires.entry(k).or_insert(full);
                self.temps.insert(k, FIRE_K);
            }
            _ => {}
        }
        if wake && self.temps.contains_key(&k) {
            // Already hot: nothing more.
        }
    }

    /// Finds every cell that needs the sim (first step only): burning,
    /// unsettled water, vents, steam and cells with a stored temperature.
    fn scan(&mut self, vox: &mut voxel::World) {
        let mut found = Vec::new();
        for ci in 0..vox.chunk_count() {
            let o = voxel::World::chunk_origin(ci);
            let interesting = |c: Cell| {
                c.temp != 0
                    || c.has(Cell::BURNING)
                    || matches!(c.mat, mat::GAS_VENT | mat::STEAM)
                    || (c.mat == mat::WATER && !c.has(Cell::SETTLED))
            };
            match vox.chunk(ci) {
                voxel::Chunk::Uniform(c) => {
                    if interesting(*c) {
                        for i in 0..CHUNK_CELLS {
                            found.push(pos((ci * CHUNK_CELLS + i) as u32));
                        }
                    }
                }
                voxel::Chunk::Array(a) => {
                    for (i, c) in a.iter().enumerate() {
                        if interesting(*c) {
                            let i = i as i32;
                            let n = voxel::CHUNK;
                            found.push([o[0] + i % n, o[1] + (i / n) % n, o[2] + i / (n * n)]);
                        }
                    }
                }
            }
        }
        for p in found {
            self.classify(vox, p, false);
        }
    }

    /// Writes a new material at `p` (a phase change), clearing what the old
    /// one had on the lists, with temperature `t`.
    fn change(&mut self, vox: &mut voxel::World, p: [i32; 3], cell: Cell, t: f32) {
        let Some(k) = key(p) else {
            return;
        };
        vox.set(p, cell);
        self.latent.remove(&k);
        self.fires.remove(&k);
        self.idle.remove(&k);
        if cell.mat == mat::WATER {
            self.liquid.insert(k);
        } else {
            self.liquid.remove(&k);
        }
        if cell.mat == mat::STEAM {
            self.steam.insert(k, STEAM_TICKS);
        } else {
            self.steam.remove(&k);
        }
        if cell.mat == mat::AIR {
            self.temps.remove(&k);
        } else {
            self.temps.insert(k, t);
        }
        self.wake_around(vox, p);
    }

    fn wake_around(&mut self, vox: &voxel::World, p: [i32; 3]) {
        for d in FACES {
            let q = add(p, d);
            if vox.get(q).mat == mat::WATER {
                if let Some(k) = key(q) {
                    self.liquid.insert(k);
                }
            }
        }
    }

    /// Adds `e` joules to the cell at `p`, with the latent-heat buffers at
    /// 0 °C and 100 °C, and the phase changes they lead to (ice ↔ water →
    /// steam). Air and steam take no heat.
    fn add_energy(
        &mut self,
        vox: &mut voxel::World,
        p: [i32; 3],
        mut e: f32,
        ev: &mut Vec<MatterEvent>,
    ) {
        let c = vox.get(p);
        let Some(k) = key(p) else {
            return;
        };
        if matches!(c.mat, mat::AIR | mat::STEAM) || e == 0.0 {
            return;
        }
        let cap = self.cap(c);
        let mut t = self.temp_of(vox, p);
        let mut lat = self.latent.get(&k).copied().unwrap_or(0.0);
        match c.mat {
            mat::ICE => {
                if e < 0.0 && lat > 0.0 {
                    let d = lat.min(-e);
                    lat -= d;
                    e += d;
                }
                t += e / cap;
                if t > MELT_K {
                    lat += (t - MELT_K) * cap;
                    t = MELT_K;
                }
                let lf = LF_J_KG * self.props[mat::ICE as usize].density;
                if lat >= lf {
                    let mut w = Cell::new(mat::WATER);
                    w.set_level(8);
                    let wc = self.props[mat::WATER as usize].c_vol;
                    let tw = (MELT_K + (lat - lf) / wc).min(BOIL_K);
                    self.change(vox, p, w, tw);
                    ev.push(MatterEvent::Melted { cell: p });
                    return;
                }
            }
            mat::WATER => {
                let frac = c.level().max(1) as f32 / 8.0;
                let mass = self.props[mat::WATER as usize].density * frac;
                if lat < 0.0 && e > 0.0 {
                    let d = e.min(-lat);
                    lat += d;
                    e -= d;
                }
                if lat > 0.0 && e < 0.0 {
                    let d = (-e).min(lat);
                    lat -= d;
                    e += d;
                }
                t += e / cap;
                if t < MELT_K {
                    // The top cell of a column freezes first; flowing river
                    // water (`SOURCE`, refilled from upstream) keeps moving
                    // and does not freeze from a cast (P2).
                    let top = vox.get(add(p, [0, 1, 0])).mat != mat::WATER;
                    if top && !c.has(Cell::SOURCE) {
                        lat -= (MELT_K - t) * cap;
                    }
                    t = MELT_K;
                }
                if t > BOIL_K {
                    lat += (t - BOIL_K) * cap;
                    t = BOIL_K;
                }
                if lat <= -LF_J_KG * mass {
                    let ic = self.props[mat::ICE as usize].c_vol;
                    let ti = (MELT_K - (-lat - LF_J_KG * mass) / ic).max(LN2_K);
                    self.change(vox, p, Cell::new(mat::ICE), ti);
                    ev.push(MatterEvent::Froze { cell: p });
                    return;
                }
                if lat >= LV_J_KG * mass {
                    self.change(vox, p, Cell::new(mat::STEAM), BOIL_K);
                    return;
                }
            }
            _ => {
                t = (t + e / cap).clamp(1.0, 3276.0);
            }
        }
        self.temps.insert(k, t);
        if lat.abs() > 1.0 {
            self.latent.insert(k, lat);
        } else {
            self.latent.remove(&k);
        }
    }

    /// Heats toward `max_k` only (Fire): never pushes a cell past it, except
    /// that ice and water take their energy whole (the latent buffers stop
    /// them at 0 °C and 100 °C).
    fn heat_toward(
        &mut self,
        vox: &mut voxel::World,
        p: [i32; 3],
        e: f32,
        max_k: f32,
        ev: &mut Vec<MatterEvent>,
    ) {
        let c = vox.get(p);
        if matches!(c.mat, mat::ICE | mat::WATER) {
            self.add_energy(vox, p, e, ev);
            return;
        }
        let t = self.temp_of(vox, p);
        if t < max_k {
            let room = (max_k - t) * self.cap(c);
            self.add_energy(vox, p, e.min(room), ev);
        }
    }

    /// The rules that depend on temperature alone: ignition, sand to glass,
    /// iron ore plus coal to iron.
    fn check_phase(&mut self, vox: &mut voxel::World, p: [i32; 3], ev: &mut Vec<MatterEvent>) {
        let c = vox.get(p);
        let t = self.temp_of(vox, p);
        match c.mat {
            mat::SAND if t >= GLASS_K => {
                self.change(vox, p, Cell::new(mat::GLASS), t);
                ev.push(MatterEvent::Glass { cell: p });
            }
            mat::IRON_ORE if t >= IRON_K => {
                if let Some(q) = FACES
                    .iter()
                    .map(|&d| add(p, d))
                    .find(|&q| vox.get(q).mat == mat::COAL_ORE)
                {
                    self.change(vox, q, Cell::AIR, 0.0);
                    self.change(vox, p, Cell::new(mat::IRON), t);
                    ev.push(MatterEvent::Iron { cell: p });
                }
            }
            m if !c.has(Cell::BURNING) => {
                if let Some(ig) = self.props[m as usize].ignite_k {
                    if t >= ig && self.o2_at(p) >= SMOTHER_O2 {
                        self.ignite(vox, p, ev);
                    }
                }
            }
            _ => {}
        }
    }

    fn ignite(&mut self, vox: &mut voxel::World, p: [i32; 3], ev: &mut Vec<MatterEvent>) {
        let mut c = vox.get(p);
        let Some(k) = key(p) else {
            return;
        };
        c.set_flag(Cell::BURNING, true);
        vox.set(p, c);
        self.fires.insert(k, self.burn_full(c.mat));
        self.temps.insert(k, FIRE_K);
        ev.push(MatterEvent::Ignited { cell: p });
    }

    fn extinguish(
        &mut self,
        vox: &mut voxel::World,
        p: [i32; 3],
        by: By,
        ev: &mut Vec<MatterEvent>,
    ) {
        let mut c = vox.get(p);
        let Some(k) = key(p) else {
            return;
        };
        c.set_flag(Cell::BURNING, false);
        vox.set(p, c);
        self.fires.remove(&k);
        ev.push(MatterEvent::Extinguished { cell: p, by });
    }

    // -----------------------------------------------------------------
    // Emits (section 5.6)
    // -----------------------------------------------------------------

    /// Cells whose centres are within `EMIT_RADIUS` of `at`, in key order.
    fn in_radius(at: [f32; 3]) -> Vec<[i32; 3]> {
        let c = at.map(|v| v.floor() as i32);
        let mut out: Vec<(u32, [i32; 3])> = Vec::new();
        let r2 = EMIT_RADIUS * EMIT_RADIUS;
        for dz in -2..=2 {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let p = add(c, [dx, dy, dz]);
                    let d2: f32 = (0..3)
                        .map(|a| {
                            let d = p[a] as f32 + 0.5 - at[a];
                            d * d
                        })
                        .sum();
                    if d2 <= r2 + 1e-4 {
                        if let Some(k) = key(p) {
                            out.push((k, p));
                        }
                    }
                }
            }
        }
        out.sort_by_key(|e| e.0);
        out.into_iter().map(|e| e.1).collect()
    }

    /// Lit vents whose block or flame cell is within reach of `at`.
    fn vents_near(&self, vox: &voxel::World, at: [f32; 3]) -> Vec<[i32; 3]> {
        let near = Self::in_radius(at);
        let mut out = Vec::new();
        for &k in self.vents.keys() {
            let v = pos(k);
            let flame = add(v, [0, 1, 0]);
            if (near.contains(&v) || near.contains(&flame)) && vox.get(v).has(Cell::BURNING) {
                out.push(v);
            }
        }
        out
    }

    fn knock_vent(
        &mut self,
        vox: &mut voxel::World,
        v: [i32; 3],
        by: By,
        ev: &mut Vec<MatterEvent>,
    ) {
        let mut c = vox.get(v);
        c.set_flag(Cell::BURNING, false);
        vox.set(v, c);
        if let Some(k) = key(v) {
            self.vents.insert(k, (FLICKER_S / DT).round() as u16);
        }
        ev.push(MatterEvent::Extinguished { cell: v, by });
    }

    fn apply_emit(&mut self, vox: &mut voxel::World, e: Emit, ev: &mut Vec<MatterEvent>) {
        let n = e.normal.map(|v| v as f32);
        let land = [0, 1, 2].map(|a| (e.pos[a] + n[a] * 0.5).floor() as i32);
        let amount = e.amount.max(0.0);
        match e.what {
            EmitKind::Water => {
                let mut left = ((WATER_EMIT_LEVELS * amount).round() as u32).min(WATER_EMIT_MAX);
                let mut at = land;
                if self.solid[vox.get(at).mat as usize] {
                    at = add(at, e.normal.map(|v| v as i32));
                }
                for _ in 0..4 {
                    if left == 0 {
                        break;
                    }
                    let c = vox.get(at);
                    let have = match c.mat {
                        mat::AIR => 0,
                        mat::WATER => c.level() as u32,
                        _ => break,
                    };
                    let put = left.min(8 - have);
                    if put > 0 {
                        let mut w = Cell::new(mat::WATER);
                        w.set_level((have + put) as u8);
                        vox.set(at, w);
                        if let Some(k) = key(at) {
                            self.liquid.insert(k);
                            self.idle.remove(&k);
                        }
                        self.wake_around(vox, at);
                        left -= put;
                    }
                    at = add(at, [0, 1, 0]);
                }
                for p in Self::in_radius(e.pos) {
                    if vox.get(p).has(Cell::BURNING)
                        && self.fires.contains_key(&key(p).unwrap_or(0))
                    {
                        self.extinguish(vox, p, By::Water, ev);
                        if let Some(k) = key(p) {
                            let t = self.temp_of(vox, p).min(BOIL_K);
                            self.temps.insert(k, t);
                        }
                    }
                }
                for v in self.vents_near(vox, e.pos) {
                    self.knock_vent(vox, v, By::Water, ev);
                }
            }
            EmitKind::Frost => {
                let cells = Self::in_radius(e.pos);
                if cells
                    .iter()
                    .any(|&p| vox.get(p).mat == mat::WATER && self.temp_of(vox, p) > LEIDENFROST_K)
                {
                    ev.push(MatterEvent::Leidenfrost { pos: e.pos });
                }
                for v in self.vents_near(vox, e.pos) {
                    self.knock_vent(vox, v, By::Frost, ev);
                }
                for p in cells {
                    let c = vox.get(p);
                    if matches!(c.mat, mat::AIR | mat::STEAM) {
                        continue;
                    }
                    let mut j = -FROST_J * amount;
                    if c.mat != mat::WATER && c.mat != mat::ICE {
                        let floor = (LN2_K - self.temp_of(vox, p)) * self.cap(c);
                        j = j.max(floor.min(0.0));
                    }
                    self.add_energy(vox, p, j, ev);
                    let c = vox.get(p);
                    if c.has(Cell::BURNING) && self.fires.contains_key(&key(p).unwrap_or(0)) {
                        if let Some(ig) = self.props[c.mat as usize].ignite_k {
                            if self.temp_of(vox, p) < ig {
                                self.extinguish(vox, p, By::Frost, ev);
                            }
                        }
                    }
                }
            }
            EmitKind::Fire => {
                self.flames.push(Flame {
                    pos: e.pos,
                    left: FIRE_EMIT_STEPS,
                });
                self.gas.add(
                    vox,
                    &self.solid,
                    gas::of_block(land),
                    gas::CO2,
                    FIRE_CO2_UNITS,
                );
            }
            EmitKind::Co2 => {
                let units = (CO2_EMIT_M3 * amount * gas::FULL as f32 / 8.0).round() as u32;
                self.gas
                    .add(vox, &self.solid, gas::of_block(land), gas::CO2, units);
            }
        }
    }

    fn flames_step(&mut self, vox: &mut voxel::World, ev: &mut Vec<MatterEvent>) {
        let flames = std::mem::take(&mut self.flames);
        for mut f in flames {
            for p in Self::in_radius(f.pos) {
                self.heat_toward(vox, p, FIRE_EMIT_J, FIRE_K, ev);
                self.check_phase(vox, p, ev);
            }
            f.left -= 1;
            if f.left > 0 {
                self.flames.push(f);
            }
        }
    }

    // -----------------------------------------------------------------
    // Passes (section 5.3)
    // -----------------------------------------------------------------

    /// Up to `cap` keys of `keys` starting at `*cursor`, wrapping; moves the
    /// cursor past the last one taken.
    fn batch(keys: &[u32], cursor: &mut u32, cap: usize) -> Vec<u32> {
        if keys.len() <= cap {
            return keys.to_vec();
        }
        let start = keys.partition_point(|&k| k < *cursor);
        let out: Vec<u32> = (0..cap).map(|i| keys[(start + i) % keys.len()]).collect();
        *cursor = out.last().map_or(0, |&k| k.wrapping_add(1));
        out
    }

    fn heat(&mut self, vox: &mut voxel::World, ev: &mut Vec<MatterEvent>) {
        let keys: Vec<u32> = self.temps.keys().copied().collect();
        let batch = Self::batch(&keys, &mut self.cursor[0], HEAT_CAP);
        self.stats.heat = batch.len();
        // A cell that arrives hot (a placed block, the Furnace, a test)
        // changes before it starts to cool.
        for &k in &batch {
            self.check_phase(vox, pos(k), ev);
        }
        let mut delta: BTreeMap<u32, f32> = BTreeMap::new();
        for &k in &batch {
            let a = pos(k);
            let ca = vox.get(a);
            if matches!(ca.mat, mat::AIR | mat::STEAM) {
                continue;
            }
            let ta = self.temp_of(vox, a);
            let capa = self.cap(ca);
            let pa = self.props[ca.mat as usize];
            // Pull back toward rest.
            let lat = self.latent.get(&k).copied().unwrap_or(0.0);
            let mut e = (pa.rest_k - ta) * capa * REST_PULL - lat * REST_PULL * 2.0;
            if ca.has(Cell::BURNING) {
                e = 0.0;
            }
            *delta.entry(k).or_insert(0.0) += e;
            for d in FACES {
                let b = add(a, d);
                let Some(kb) = key(b) else { continue };
                if kb < k && batch.binary_search(&kb).is_ok() {
                    continue; // counted from b
                }
                let cb = vox.get(b);
                if matches!(cb.mat, mat::AIR | mat::STEAM) {
                    continue;
                }
                let tb = self.temp_of(vox, b);
                let pb = self.props[cb.mat as usize];
                let g = 2.0 * pa.k * pb.k / (pa.k + pb.k);
                let capb = self.cap(cb);
                let mut q = CONDUCTION_X * g * (ta - tb) * DT;
                let max = (ta - tb).abs() * capa * capb / (capa + capb) * 0.5;
                q = q.clamp(-max, max);
                if q != 0.0 {
                    *delta.entry(k).or_insert(0.0) -= q;
                    *delta.entry(kb).or_insert(0.0) += q;
                }
            }
        }
        for (k, e) in delta {
            let p = pos(k);
            self.add_energy(vox, p, e, ev);
            self.check_phase(vox, p, ev);
        }
        for k in batch {
            let p = pos(k);
            let c = vox.get(p);
            let rest = self.rest(c.mat);
            let near = self.temps.get(&k).is_some_and(|&t| (t - rest).abs() < 1.0);
            let lat = self.latent.get(&k).copied().unwrap_or(0.0);
            if lat.abs() < 1.0e5 {
                self.latent.remove(&k);
            }
            if (near && !self.latent.contains_key(&k) && !c.has(Cell::BURNING)) || c.mat == mat::AIR
            {
                self.temps.remove(&k);
            }
        }
    }

    fn fire(&mut self, vox: &mut voxel::World, ev: &mut Vec<MatterEvent>) {
        let keys: Vec<u32> = self.fires.keys().copied().collect();
        let batch = Self::batch(&keys, &mut self.cursor[1], FIRE_CAP);
        self.stats.fire = batch.len();
        self.stats.fire_carried = keys.len() - batch.len();
        for k in batch {
            let p = pos(k);
            let c = vox.get(p);
            if !c.has(Cell::BURNING) || !self.flammable(c.mat) {
                self.fires.remove(&k);
                continue;
            }
            if self.o2_at(p) < SMOTHER_O2 {
                self.extinguish(vox, p, By::NoOxygen, ev);
                continue;
            }
            if FACES.iter().any(|&d| vox.get(add(p, d)).mat == mat::WATER) {
                self.extinguish(vox, p, By::Water, ev);
                let t = self.temp_of(vox, p).min(BOIL_K);
                self.temps.insert(k, t);
                continue;
            }
            self.temps.insert(k, FIRE_K);
            for d in FACES {
                let q = add(p, d);
                let cq = vox.get(q);
                if matches!(cq.mat, mat::AIR | mat::STEAM) || cq.has(Cell::BURNING) {
                    continue;
                }
                self.heat_toward(vox, q, BURN_J, FIRE_K, ev);
                self.check_phase(vox, q, ev);
            }
            // A fuel store (`SOURCE` on a flammable block: a stack of
            // charcoal and logs) holds far more fuel than one block, so it
            // burns on until water or smothering puts it out (P1).
            let left = if c.has(Cell::SOURCE) {
                self.fires.get(&k).copied().unwrap_or(1).max(1)
            } else {
                self.fires.get(&k).copied().unwrap_or(0).saturating_sub(1)
            };
            if left == 0 {
                let to = if c.mat == mat::LEAVES {
                    Cell::AIR
                } else {
                    Cell::new(mat::ASH)
                };
                self.change(vox, p, to, FIRE_K);
            } else {
                self.fires.insert(k, left);
            }
        }
    }

    fn vents_step(&mut self, vox: &mut voxel::World, ev: &mut Vec<MatterEvent>) {
        let keys: Vec<u32> = self.vents.keys().copied().collect();
        for k in keys {
            let v = pos(k);
            let mut c = vox.get(v);
            if c.mat != mat::GAS_VENT {
                self.vents.remove(&k);
                continue;
            }
            let o2 = self.o2_at(add(v, [0, 1, 0]));
            let lit = c.has(Cell::BURNING);
            let flicker = self.vents.get(&k).copied().unwrap_or(0);
            if flicker > 0 {
                self.vents.insert(k, flicker - 1);
                if flicker > 1 {
                    continue;
                }
            }
            if lit && o2 < SMOTHER_O2 {
                c.set_flag(Cell::BURNING, false);
                vox.set(v, c);
                ev.push(MatterEvent::Smothered { vent: v });
            } else if !lit && o2 >= SMOTHER_O2 {
                c.set_flag(Cell::BURNING, true);
                vox.set(v, c);
                ev.push(MatterEvent::Relit { vent: v });
            }
        }
    }

    fn steam_step(&mut self, vox: &mut voxel::World) {
        let all: Vec<(u32, u16)> = self.steam.iter().map(|(&k, &v)| (k, v)).collect();
        for (k, left) in all {
            let p = pos(k);
            if vox.get(p).mat != mat::STEAM {
                self.steam.remove(&k);
                continue;
            }
            if left <= 1 {
                self.change(vox, p, Cell::AIR, 0.0);
                continue;
            }
            let up = add(p, [0, 1, 0]);
            if vox.get(up).mat == mat::AIR && voxel::World::in_bounds(up) {
                self.change(vox, p, Cell::AIR, 0.0);
                self.change(vox, up, Cell::new(mat::STEAM), BOIL_K);
                if let Some(ku) = key(up) {
                    self.steam.insert(ku, left - 1);
                }
            } else {
                self.steam.insert(k, left - 1);
            }
        }
    }

    /// Moves `mv` levels of water from `a` (level `la`) to `b`, carrying
    /// temperature, and wakes both.
    fn move_water(&mut self, vox: &mut voxel::World, a: [i32; 3], b: [i32; 3], mv: u8) {
        let ca = vox.get(a);
        let cb = vox.get(b);
        let (la, lb) = (
            ca.level(),
            if cb.mat == mat::WATER { cb.level() } else { 0 },
        );
        let (ka, kb) = (key(a).unwrap_or(0), key(b).unwrap_or(0));
        let ta = self.temp_of(vox, a);
        let tb = if cb.mat == mat::WATER {
            self.temp_of(vox, b)
        } else {
            ta
        };
        let src_hot = self.temps.contains_key(&ka);
        let mut nb = Cell::new(mat::WATER);
        nb.set_level(lb + mv);
        vox.set(b, nb);
        if src_hot || self.temps.contains_key(&kb) {
            let t = (ta * mv as f32 + tb * lb as f32) / (mv + lb) as f32;
            self.temps.insert(kb, t);
        }
        if la - mv == 0 && !ca.has(Cell::SOURCE) {
            vox.set(a, Cell::AIR);
            self.temps.remove(&ka);
            self.latent.remove(&ka);
            self.liquid.remove(&ka);
        } else {
            let mut na = ca;
            na.set_level(la - mv);
            na.set_flag(Cell::SETTLED, false);
            vox.set(a, na);
            self.liquid.insert(ka);
        }
        self.liquid.insert(kb);
        self.idle.remove(&ka);
        self.idle.remove(&kb);
        self.wake_around(vox, a);
        self.wake_around(vox, b);
    }

    fn water(&mut self, vox: &mut voxel::World, ev: &mut Vec<MatterEvent>) {
        let keys: Vec<u32> = self.liquid.iter().copied().collect();
        let batch = Self::batch(&keys, &mut self.cursor[2], WATER_CAP);
        self.stats.water = batch.len();
        let mut flowed: BTreeSet<usize> = BTreeSet::new();
        for k in batch {
            let p = pos(k);
            let c = vox.get(p);
            if c.mat != mat::WATER {
                self.liquid.remove(&k);
                self.idle.remove(&k);
                continue;
            }
            let mut changed = false;
            if c.has(Cell::SOURCE) && c.level() < 8 {
                let mut n = c;
                n.set_level(8);
                vox.set(p, n);
                changed = true;
            }
            // Down first.
            let below = add(p, [0, -1, 0]);
            let cb = vox.get(below);
            let l = vox.get(p).level();
            if p[1] > 0 && l > 0 {
                let room = match cb.mat {
                    mat::AIR => 8,
                    mat::WATER => 8 - cb.level(),
                    _ => 0,
                };
                let mv = room.min(l);
                if mv > 0 {
                    self.move_water(vox, p, below, mv);
                    changed = true;
                }
            }
            // Then share sideways.
            for d in SIDES {
                let c = vox.get(p);
                if c.mat != mat::WATER {
                    break;
                }
                let l = c.level();
                let q = add(p, d);
                if !voxel::World::in_bounds(q) {
                    continue;
                }
                let cq = vox.get(q);
                let lq = match cq.mat {
                    mat::AIR => 0,
                    mat::WATER => cq.level(),
                    _ => continue,
                };
                let mv = if l >= lq + 2 {
                    (l - lq) / 2
                } else if l == lq + 1 && cq.mat == mat::AIR {
                    // A last drop runs off an edge it can fall from.
                    let under = vox.get(add(q, [0, -1, 0]));
                    let falls =
                        under.mat == mat::AIR || (under.mat == mat::WATER && under.level() < 8);
                    falls as u8
                } else {
                    0
                };
                if mv > 0 {
                    self.move_water(vox, p, q, mv);
                    changed = true;
                }
            }
            if changed {
                if let Some(c) = voxel::World::chunk_index(p) {
                    flowed.insert(c);
                }
            } else {
                let n = self.idle.entry(k).or_insert(0);
                *n += 1;
                if *n >= 2 {
                    self.idle.remove(&k);
                    self.liquid.remove(&k);
                    let mut s = vox.get(p);
                    s.set_flag(Cell::SETTLED, true);
                    vox.set(p, s);
                }
            }
        }
        for chunk in flowed {
            ev.push(MatterEvent::Flowed { chunk });
        }
    }
}

/// A gas's density at 20 °C and 1 atm from the ideal gas law, kg/m³.
fn gas_density(molar_mass_g: f32) -> f32 {
    // ρ = pM / (RT)
    101_325.0 * (molar_mass_g / 1000.0) / (8.314_462 * 293.15)
}

/// Net upward force in newtons on a balloon of `volume_m3` filled with
/// `gas` carrying `payload_kg`, in air at 20 °C (section 5.7):
/// `(ρ_air − ρ_gas) · V · g − payload · g`. Densities come from the molar
/// masses in `realgame_sim::species` through the ideal gas law.
pub fn lift_n(gas: Gas, volume_m3: f32, payload_kg: f32) -> f32 {
    use realgame_sim::species::{Species, AIR_MOLAR_MASS};
    const G: f32 = 9.81;
    let s = match gas {
        Gas::Co2 => Species::CO2,
        Gas::H2 => Species::H2,
        Gas::Ch4 => Species::CH4,
        Gas::O2 => Species::O2,
    };
    let rho_air = gas_density(AIR_MOLAR_MASS);
    let rho = gas_density(s.props().molar_mass);
    (rho_air - rho) * volume_m3 * G - payload_kg * G
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_point_is_the_sand_rows_melting_point() {
        let rows = voxel::materials::load(voxel::materials::MATERIALS_JSON).expect("table");
        let sand = rows[mat::SAND as usize].melt_c.get().expect("sand melt_c");
        assert_eq!(sand + 273.15, GLASS_K);
    }

    #[test]
    fn keys_round_trip() {
        for p in [[0, 0, 0], [31, 32, 33], [255, 95, 255], [196, 28, 101]] {
            assert_eq!(pos(key(p).unwrap()), p);
        }
        assert!(key([0, 0, 0]) < key([1, 0, 0]));
    }
}
