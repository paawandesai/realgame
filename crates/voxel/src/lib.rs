//! The block world (I-052): cells, chunks, the material table, generation
//! and stamps, protected zones, raycast, collision boxes, the greedy mesher
//! and the diff log. Pure Rust with no Bevy types, so the browser client, a
//! future headless server and the VR client share it.
//!
//! Coordinates: x points east, z south, y up; 1 block = 1 m. The world is
//! `WORLD` blocks in `CHUNKS` chunks of `CHUNK`³ cells.
//!
//! **Determinism rules** (spec section 3): fixed iteration order (chunk
//! index, then cell index), no `HashMap` iteration, no `sin`/`exp`/`powf` in
//! anything a test hashes, and a seeded RNG stored in the world.
//!
//! The types and signatures were fixed by the contract step (spec section
//! 10); Lane A owns the bodies.

#![deny(unsafe_code)]

pub mod collide;
pub mod diff;
pub mod gen;
pub mod materials;
pub mod mesh;
pub mod raycast;
pub mod stamp;

pub use collide::{ice_boxes, solid_boxes};
pub use diff::DiffLog;
pub use gen::{biome_at, generate, spawn_for};
pub use mesh::{mesh_chunk, ChunkMesh, MeshData};
pub use raycast::{raycast, Hit, RayMode};
pub use stamp::Aabb;

/// Cells along each edge of a chunk.
pub const CHUNK: i32 = 32;
/// Cells in one chunk (32³).
pub const CHUNK_CELLS: usize = (CHUNK * CHUNK * CHUNK) as usize;
/// The world size in blocks (x, y, z).
pub const WORLD: [i32; 3] = [256, 96, 256];
/// The world size in chunks (x, y, z).
pub const CHUNKS: [i32; 3] = [WORLD[0] / CHUNK, WORLD[1] / CHUNK, WORLD[2] / CHUNK];
/// Ambient temperature, read for a cell whose `temp` is 0.
pub const AMBIENT_K: f32 = 293.15;
/// One `temp` unit in kelvin.
pub const TEMP_STEP_K: f32 = 0.05;

/// Material ids: the row of each material in `content/materials.json`
/// (section 5.2). The order is part of the save format (`DiffLog`), so rows
/// are only ever appended.
pub mod mat {
    pub type MatId = u8;
    pub const AIR: MatId = 0;
    pub const DIRT: MatId = 1;
    pub const GRASS: MatId = 2;
    pub const SAND: MatId = 3;
    pub const STONE: MatId = 4;
    pub const CLIFFSTONE: MatId = 5;
    pub const RUNESTONE: MatId = 6;
    pub const WOOD: MatId = 7;
    pub const LEAVES: MatId = 8;
    pub const PLANKS: MatId = 9;
    pub const SNOW: MatId = 10;
    pub const ICE: MatId = 11;
    pub const WATER: MatId = 12;
    pub const STEAM: MatId = 13;
    pub const MUD: MatId = 14;
    pub const COAL_ORE: MatId = 15;
    pub const IRON_ORE: MatId = 16;
    pub const CRYSTAL: MatId = 17;
    pub const GLASS: MatId = 18;
    pub const IRON: MatId = 19;
    pub const BRICK: MatId = 20;
    pub const ASH: MatId = 21;
    pub const TORCH: MatId = 22;
    pub const BENCH: MatId = 23;
    pub const INVENTION_BENCH: MatId = 24;
    pub const KILN_HEARTH: MatId = 25;
    pub const GAS_VENT: MatId = 26;
    /// Number of Must rows.
    pub const COUNT: usize = 27;
    /// Row names, in id order (the `name` field in the JSON).
    pub const NAMES: [&str; COUNT] = [
        "air",
        "dirt",
        "grass",
        "sand",
        "stone",
        "cliffstone",
        "runestone",
        "wood",
        "leaves",
        "planks",
        "snow",
        "ice",
        "water",
        "steam",
        "mud",
        "coal_ore",
        "iron_ore",
        "crystal",
        "glass",
        "iron",
        "brick",
        "ash",
        "torch",
        "bench",
        "invention_bench",
        "kiln_hearth",
        "gas_vent",
    ];
}

/// One block: 4 bytes (section 5.1).
///
/// - `mat`: material id (`mat::*`).
/// - `level_state`: low nibble the liquid level 0-8 (8 = full), high nibble
///   the state bits `BURNING`, `SOURCE`, `SETTLED`, `LIT`.
/// - `temp`: temperature in 0.05 K units, 0..3276.75 K; 0 means ambient
///   (read as 293.15 K).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cell {
    pub mat: u8,
    pub level_state: u8,
    pub temp: u16,
}

impl Cell {
    /// State bit: the cell is on fire.
    pub const BURNING: u8 = 1;
    /// State bit: a liquid source that refills to level 8 (river, well).
    pub const SOURCE: u8 = 2;
    /// State bit: a liquid cell that has not changed for 2 steps.
    pub const SETTLED: u8 = 4;
    /// State bit: a torch, or a lit kiln.
    pub const LIT: u8 = 8;
    /// Plain air at ambient temperature.
    pub const AIR: Cell = Cell {
        mat: mat::AIR,
        level_state: 0,
        temp: 0,
    };

    /// A cell of `mat` at ambient temperature, level 0, no state bits.
    pub const fn new(mat: u8) -> Cell {
        Cell {
            mat,
            level_state: 0,
            temp: 0,
        }
    }

    /// Liquid level 0-8.
    pub fn level(self) -> u8 {
        self.level_state & 0x0f
    }

    /// Sets the liquid level (clamped to 8); leaves the state bits alone.
    pub fn set_level(&mut self, level: u8) {
        self.level_state = (self.level_state & 0xf0) | level.min(8);
    }

    /// The four state bits (`BURNING | SOURCE | SETTLED | LIT`).
    pub fn state(self) -> u8 {
        self.level_state >> 4
    }

    /// Sets all four state bits; leaves the level alone.
    pub fn set_state(&mut self, state: u8) {
        self.level_state = (self.level_state & 0x0f) | ((state & 0x0f) << 4);
    }

    /// True when every bit of `flag` is set.
    pub fn has(self, flag: u8) -> bool {
        self.state() & flag == flag
    }

    /// Sets or clears `flag`.
    pub fn set_flag(&mut self, flag: u8, on: bool) {
        let s = if on {
            self.state() | flag
        } else {
            self.state() & !flag
        };
        self.set_state(s);
    }

    /// Temperature in kelvin (0 reads as ambient).
    pub fn temp_k(self) -> f32 {
        if self.temp == 0 {
            AMBIENT_K
        } else {
            self.temp as f32 * TEMP_STEP_K
        }
    }

    /// Temperature in °C.
    pub fn temp_c(self) -> f32 {
        self.temp_k() - 273.15
    }

    /// Sets the temperature in kelvin, rounded to 0.05 K and clamped to the
    /// range (1 unit at the bottom, so it never reads as "ambient").
    pub fn set_temp_k(&mut self, k: f32) {
        let units = (k / TEMP_STEP_K + 0.5).clamp(1.0, u16::MAX as f32);
        self.temp = units as u16;
    }
}

/// A chunk's storage: one value for a chunk that is all the same cell, or
/// the full 32³ array (x fastest, then y, then z: `cell_index`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chunk {
    Uniform(Cell),
    Array(Box<[Cell]>),
}

impl Chunk {
    fn get(&self, i: usize) -> Cell {
        match self {
            Chunk::Uniform(c) => *c,
            Chunk::Array(a) => a[i],
        }
    }

    fn set(&mut self, i: usize, cell: Cell) {
        if let Chunk::Uniform(c) = self {
            if *c == cell {
                return;
            }
            *self = Chunk::Array(vec![*c; CHUNK_CELLS].into_boxed_slice());
        }
        if let Chunk::Array(a) = self {
            a[i] = cell;
        }
    }
}

/// The whole block world: 192 chunks, the dirty set for meshing, and the
/// protected boxes (a world-level list, not a cell bit, so the diff log can
/// never change it).
#[derive(Clone, Debug)]
pub struct World {
    chunks: Vec<Chunk>,
    dirty: Vec<bool>,
    protected: Vec<Aabb>,
    /// The generation seed (`generate`); 0 for `World::new`.
    pub seed: u64,
}

impl Default for World {
    fn default() -> Self {
        World::new()
    }
}

impl World {
    /// An all-air world with no protected boxes.
    pub fn new() -> World {
        let n = (CHUNKS[0] * CHUNKS[1] * CHUNKS[2]) as usize;
        World {
            chunks: vec![Chunk::Uniform(Cell::AIR); n],
            dirty: vec![false; n],
            protected: Vec::new(),
            seed: 0,
        }
    }

    /// Number of chunks (192).
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// True when `p` is inside the world.
    pub fn in_bounds(p: [i32; 3]) -> bool {
        (0..3).all(|a| p[a] >= 0 && p[a] < WORLD[a])
    }

    /// The chunk holding block `p` (x fastest, then y, then z), or `None`
    /// outside the world.
    pub fn chunk_index(p: [i32; 3]) -> Option<usize> {
        if !World::in_bounds(p) {
            return None;
        }
        let [cx, cy, cz] = [p[0] / CHUNK, p[1] / CHUNK, p[2] / CHUNK];
        Some((cx + CHUNKS[0] * (cy + CHUNKS[1] * cz)) as usize)
    }

    /// The chunk coordinates (cx, cy, cz) of chunk `i`.
    pub fn chunk_coords(i: usize) -> [i32; 3] {
        let i = i as i32;
        let cx = i % CHUNKS[0];
        let cy = (i / CHUNKS[0]) % CHUNKS[1];
        let cz = i / (CHUNKS[0] * CHUNKS[1]);
        [cx, cy, cz]
    }

    /// The block coordinates of chunk `i`'s lowest corner.
    pub fn chunk_origin(i: usize) -> [i32; 3] {
        World::chunk_coords(i).map(|c| c * CHUNK)
    }

    /// The index of block `p` inside its chunk (x fastest, then y, then z).
    pub fn cell_index(p: [i32; 3]) -> usize {
        let [x, y, z] = p.map(|v| v.rem_euclid(CHUNK));
        (x + CHUNK * (y + CHUNK * z)) as usize
    }

    /// The cell at `p`; air outside the world.
    pub fn get(&self, p: [i32; 3]) -> Cell {
        match World::chunk_index(p) {
            Some(c) => self.chunks[c].get(World::cell_index(p)),
            None => Cell::AIR,
        }
    }

    /// Writes the cell at `p` (ignored outside the world) and marks its
    /// chunk dirty, plus each neighbouring chunk whose faces it touches.
    /// Writing the same value changes nothing.
    pub fn set(&mut self, p: [i32; 3], cell: Cell) {
        let Some(c) = World::chunk_index(p) else {
            return;
        };
        let i = World::cell_index(p);
        if self.chunks[c].get(i) == cell {
            return;
        }
        self.chunks[c].set(i, cell);
        self.dirty[c] = true;
        for a in 0..3 {
            let local = p[a].rem_euclid(CHUNK);
            for (edge, d) in [(0, -1), (CHUNK - 1, 1)] {
                if local == edge {
                    let mut q = p;
                    q[a] += d;
                    if let Some(n) = World::chunk_index(q) {
                        self.dirty[n] = true;
                    }
                }
            }
        }
    }

    /// Chunk `i`'s storage (for the mesher and the diff log).
    pub fn chunk(&self, i: usize) -> &Chunk {
        &self.chunks[i]
    }

    /// Replaces chunk `i` wholesale (generation) and marks it dirty.
    pub fn put_chunk(&mut self, i: usize, chunk: Chunk) {
        self.chunks[i] = chunk;
        self.dirty[i] = true;
    }

    /// The dirty chunks, in index order.
    pub fn dirty_chunks(&self) -> Vec<usize> {
        (0..self.dirty.len()).filter(|&i| self.dirty[i]).collect()
    }

    /// The dirty chunks, in index order, clearing the set.
    pub fn take_dirty(&mut self) -> Vec<usize> {
        let out = self.dirty_chunks();
        self.dirty.iter_mut().for_each(|d| *d = false);
        out
    }

    /// Adds a protected box (stamps and the edge ring).
    pub fn protect(&mut self, b: Aabb) {
        self.protected.push(b);
    }

    /// The protected boxes, in the order they were added.
    pub fn protected(&self) -> &[Aabb] {
        &self.protected
    }

    fn is_protected(&self, p: [i32; 3]) -> bool {
        self.protected.iter().any(|b| b.contains(p))
    }

    /// True when the player may dig the block at `p`.
    pub fn can_break(&self, p: [i32; 3]) -> bool {
        World::in_bounds(p) && !self.is_protected(p)
    }

    /// True when the player may place a block at `p`.
    pub fn can_place(&self, p: [i32; 3]) -> bool {
        World::in_bounds(p) && !self.is_protected(p)
    }

    /// FNV-1a over every cell in chunk then cell order (uniform chunks hash
    /// as if expanded), so equal worlds hash equal whatever their storage.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |b: u8| {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        for c in &self.chunks {
            for i in 0..CHUNK_CELLS {
                let cell = c.get(i);
                eat(cell.mat);
                eat(cell.level_state);
                eat(cell.temp as u8);
                eat((cell.temp >> 8) as u8);
            }
        }
        h
    }
}

/// The five biomes (section 4), for `?biome=` and the `biome` state key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Village,
    Frozen,
    Marsh,
    Cave,
    Kiln,
}

impl Biome {
    pub const ALL: [Biome; 5] = [
        Biome::Village,
        Biome::Frozen,
        Biome::Marsh,
        Biome::Cave,
        Biome::Kiln,
    ];

    /// The URL and state-key id.
    pub fn id(self) -> &'static str {
        match self {
            Biome::Village => "village",
            Biome::Frozen => "frozen",
            Biome::Marsh => "marsh",
            Biome::Cave => "cave",
            Biome::Kiln => "kiln",
        }
    }

    /// Exact, lower-case only.
    pub fn from_id(s: &str) -> Option<Biome> {
        Biome::ALL.into_iter().find(|b| b.id() == s)
    }
}

/// The player's spawn in the Village (section 4), feet position, facing north.
pub const SPAWN: [f32; 3] = [128.5, 35.0, 205.5];
/// The default world seed (`?seed=` overrides it in debug builds).
pub const DEFAULT_SEED: u64 = 0x52_47_31;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_coords_invert_chunk_index() {
        for i in 0..192 {
            let o = World::chunk_origin(i);
            assert_eq!(World::chunk_index(o), Some(i));
        }
    }

    #[test]
    fn biome_ids_round_trip() {
        for b in Biome::ALL {
            assert_eq!(Biome::from_id(b.id()), Some(b));
        }
        assert_eq!(Biome::from_id("Marsh"), None);
    }
}
