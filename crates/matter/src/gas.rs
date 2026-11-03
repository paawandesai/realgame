//! The coarse gas field (section 5.5): one gas cell per 2 × 2 × 2 blocks,
//! 128 × 48 × 128 cells, each holding the CO2, H2, CH4 and extra-O2 shares
//! as `u8` in 1/255 steps; the rest is air (N2 + 21 % O2).
//!
//! Each step, on the active chunks only: buoyancy (CO2 and O2 move down one
//! cell, H2 and CH4 up one; H2 and CH4 leave through the top of the world),
//! sideways spread between connected cells, and a slow leak of CO2 back to
//! air. All moves are whole units between cells, in a fixed order, so the
//! field is deterministic and conserves gas except where it says not to.
//!
//! **Connectivity.** A gas cell is closed when 5 or more of its 8 blocks are
//! solid (the spec's rule). Two open cells exchange gas only when at least
//! one pair of blocks facing each other across their shared face are both
//! open, so a one-block wall inside a gas cell (the hollow's rim and its gap)
//! still separates the two sides.

use realgame_voxel::{self as voxel, mat, Cell, CHUNK};

/// Gas cells along x, y, z.
pub const G: [i32; 3] = [128, 48, 128];
/// Gas units in one full cell.
pub const FULL: u32 = 255;
/// Species slots: CO2, H2, CH4, O2 (extra oxygen above air's share).
pub const CO2: usize = 0;
pub const H2: usize = 1;
pub const CH4: usize = 2;
pub const O2: usize = 3;
/// Air's oxygen share.
pub const AIR_O2: f32 = 0.21;
/// Every this many steps (5 s), pooled CO2 loses 1/60 of itself (at least
/// one unit): a 3-cast pool in the methane hollow lets its vents relight about
/// half a minute later (section 5.5's slow leak back to air).
pub const LEAK_EVERY: u64 = 50;
pub const LEAK_DIV: u32 = 60;
/// Gas cells per chunk edge (a chunk is 32 blocks = 16 gas cells).
const GC: i32 = CHUNK / 2;

/// True when a block stops gas: a solid block (but not a gas vent, which is
/// an opening), or water deep enough to fill the gap (level 6 or more).
pub fn blocks_gas(c: Cell, solid: &[bool]) -> bool {
    match c.mat {
        mat::GAS_VENT => false,
        mat::WATER => c.level() >= 6,
        m => solid[m as usize],
    }
}

/// The gas field.
#[derive(Clone, Debug, Default)]
pub struct Field {
    cells: Vec<[u8; 4]>,
    /// One flag per voxel chunk: the chunk holds some gas.
    active: Vec<bool>,
}

pub fn idx(g: [i32; 3]) -> usize {
    (g[0] + G[0] * (g[1] + G[1] * g[2])) as usize
}

pub fn in_field(g: [i32; 3]) -> bool {
    (0..3).all(|a| g[a] >= 0 && g[a] < G[a])
}

/// The gas cell holding block `p`.
pub fn of_block(p: [i32; 3]) -> [i32; 3] {
    p.map(|v| v.div_euclid(2))
}

fn chunk_of(g: [i32; 3]) -> usize {
    let c = g.map(|v| v / GC);
    let n = voxel::CHUNKS;
    (c[0] + n[0] * (c[1] + n[1] * c[2])) as usize
}

fn sum(c: [u8; 4]) -> u32 {
    c.iter().map(|&v| v as u32).sum()
}

impl Field {
    fn ensure(&mut self) {
        if self.cells.is_empty() {
            self.cells = vec![[0; 4]; (G[0] * G[1] * G[2]) as usize];
            self.active =
                vec![false; (voxel::CHUNKS[0] * voxel::CHUNKS[1] * voxel::CHUNKS[2]) as usize];
        }
    }

    /// The four shares at gas cell `g` (all 0 outside or before any gas).
    pub fn get(&self, g: [i32; 3]) -> [u8; 4] {
        if self.cells.is_empty() || !in_field(g) {
            return [0; 4];
        }
        self.cells[idx(g)]
    }

    /// Oxygen fraction at gas cell `g`: the extra O2 plus air's 21 % of
    /// whatever is not CO2, H2, CH4 or extra O2.
    pub fn o2(&self, g: [i32; 3]) -> f32 {
        let c = self.get(g);
        let rest = FULL.saturating_sub(sum(c)) as f32;
        (c[O2] as f32 + AIR_O2 * rest) / FULL as f32
    }

    fn put(&mut self, g: [i32; 3], c: [u8; 4]) {
        self.cells[idx(g)] = c;
        if c != [0; 4] {
            self.active[chunk_of(g)] = true;
        }
    }

    /// Chunks that hold gas, in index order.
    pub fn active_chunks(&self) -> Vec<usize> {
        (0..self.active.len()).filter(|&i| self.active[i]).collect()
    }

    /// Adds `units` of species `s` starting at gas cell `start` and filling
    /// outward through connected cells (breadth first: down, then sideways,
    /// then up), each up to full. Gas that finds no room is lost.
    pub fn add(
        &mut self,
        vox: &voxel::World,
        solid: &[bool],
        start: [i32; 3],
        s: usize,
        units: u32,
    ) {
        if units == 0 || !in_field(start) {
            return;
        }
        self.ensure();
        let mut left = units;
        let mut seen: Vec<usize> = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        if open(vox, solid, start) {
            queue.push_back(start);
        } else {
            for d in DIRS {
                let n = add3(start, d);
                if in_field(n) && open(vox, solid, n) {
                    queue.push_back(n);
                }
            }
        }
        let mut visits = 0;
        while let Some(g) = queue.pop_front() {
            let i = idx(g);
            if seen.contains(&i) {
                continue;
            }
            seen.push(i);
            visits += 1;
            let mut c = self.cells[i];
            let room = FULL.saturating_sub(sum(c));
            let mv = room.min(left);
            c[s] = (c[s] as u32 + mv) as u8;
            self.put(g, c);
            left -= mv;
            if left == 0 || visits > 512 {
                break;
            }
            for d in DIRS {
                let n = add3(g, d);
                if in_field(n) && connected(vox, solid, g, d) {
                    queue.push_back(n);
                }
            }
        }
    }

    /// One gas step on up to `budget` non-empty cells (whole chunks, from
    /// `cursor`). Returns the number of non-empty cells processed.
    pub fn step(
        &mut self,
        vox: &voxel::World,
        solid: &[bool],
        tick: u64,
        cursor: &mut usize,
        budget: usize,
    ) -> usize {
        if self.cells.is_empty() {
            return 0;
        }
        let act = self.active_chunks();
        if act.is_empty() {
            return 0;
        }
        // Pick whole chunks from the cursor until the budget is used.
        let start = act.iter().position(|&c| c >= *cursor).unwrap_or(0);
        let mut batch = Vec::new();
        let mut used = 0;
        for k in 0..act.len() {
            let c = act[(start + k) % act.len()];
            let n = self.nonzero_in(c);
            if !batch.is_empty() && used + n > budget {
                *cursor = c;
                break;
            }
            used += n;
            batch.push(c);
            *cursor = c + 1;
        }
        // Buoyancy: heavy gases down (bottom layer first), light gases up
        // (top layer first), so each unit moves at most one cell per step.
        let mut by_y = batch.clone();
        by_y.sort_by_key(|&c| {
            let cc = voxel::World::chunk_coords(c);
            (cc[1], cc[2], cc[0])
        });
        for &c in &by_y {
            let o = voxel::World::chunk_coords(c).map(|v| v * GC);
            for y in 0..GC {
                for z in 0..GC {
                    for x in 0..GC {
                        self.sink([o[0] + x, o[1] + y, o[2] + z], vox, solid);
                    }
                }
            }
        }
        for &c in by_y.iter().rev() {
            let o = voxel::World::chunk_coords(c).map(|v| v * GC);
            for y in (0..GC).rev() {
                for z in 0..GC {
                    for x in 0..GC {
                        self.rise([o[0] + x, o[1] + y, o[2] + z], vox, solid);
                    }
                }
            }
        }
        // Sideways spread and the CO2 leak, in chunk then cell order.
        let leak = tick.is_multiple_of(LEAK_EVERY);
        for &c in &batch {
            let o = voxel::World::chunk_coords(c).map(|v| v * GC);
            let mut any = false;
            for z in 0..GC {
                for y in 0..GC {
                    for x in 0..GC {
                        let g = [o[0] + x, o[1] + y, o[2] + z];
                        if self.cells[idx(g)] == [0; 4] {
                            continue;
                        }
                        self.spread(g, vox, solid);
                        let i = idx(g);
                        if leak && self.cells[i][CO2] > 0 {
                            let v = self.cells[i][CO2] as u32;
                            self.cells[i][CO2] = (v - v.div_ceil(LEAK_DIV)) as u8;
                        }
                        any |= self.cells[i] != [0; 4];
                    }
                }
            }
            if !any {
                self.active[c] = false;
            }
        }
        used
    }

    fn nonzero_in(&self, c: usize) -> usize {
        let o = voxel::World::chunk_coords(c).map(|v| v * GC);
        let mut n = 0;
        for z in 0..GC {
            for y in 0..GC {
                for x in 0..GC {
                    n += (self.cells[idx([o[0] + x, o[1] + y, o[2] + z])] != [0; 4]) as usize;
                }
            }
        }
        n
    }

    fn transfer(&mut self, from: [i32; 3], to: [i32; 3], s: usize, want: u32) -> u32 {
        let (fi, ti) = (idx(from), idx(to));
        let room = FULL.saturating_sub(sum(self.cells[ti]));
        let mv = want.min(room).min(self.cells[fi][s] as u32);
        if mv > 0 {
            self.cells[fi][s] -= mv as u8;
            let mut t = self.cells[ti];
            t[s] += mv as u8;
            self.put(to, t);
        }
        mv
    }

    fn sink(&mut self, g: [i32; 3], vox: &voxel::World, solid: &[bool]) {
        let c = self.cells[idx(g)];
        if c[CO2] == 0 && c[O2] == 0 {
            return;
        }
        let down = [g[0], g[1] - 1, g[2]];
        if g[1] == 0 || !connected(vox, solid, g, [0, -1, 0]) {
            return;
        }
        // CO2 (1.8 kg/m³) sinks fast; O2 (1.33) only a little heavier than air.
        self.transfer(g, down, CO2, (c[CO2] as u32).div_ceil(2));
        self.transfer(g, down, O2, c[O2] as u32 / 8);
    }

    fn rise(&mut self, g: [i32; 3], vox: &voxel::World, solid: &[bool]) {
        let i = idx(g);
        let c = self.cells[i];
        if c[H2] == 0 && c[CH4] == 0 {
            return;
        }
        if g[1] == G[1] - 1 {
            // Out through the top of the world into the sky.
            for s in [H2, CH4] {
                let v = self.cells[i][s];
                self.cells[i][s] = v - v.div_ceil(2);
            }
            return;
        }
        if !connected(vox, solid, g, [0, 1, 0]) {
            return;
        }
        let up = [g[0], g[1] + 1, g[2]];
        // H2 (0.08 kg/m³) and CH4 (0.67) are both lighter than air.
        self.transfer(g, up, H2, (c[H2] as u32).div_ceil(2));
        self.transfer(g, up, CH4, (c[CH4] as u32).div_ceil(3));
    }

    fn spread(&mut self, g: [i32; 3], vox: &voxel::World, solid: &[bool]) {
        for d in SIDES {
            let n = add3(g, d);
            if !in_field(n) {
                continue;
            }
            let mut linked: Option<bool> = None;
            for s in 0..4 {
                let (a, b) = (self.cells[idx(g)][s] as u32, self.cells[idx(n)][s] as u32);
                if a < b + 2 {
                    continue;
                }
                if *linked.get_or_insert_with(|| connected(vox, solid, g, d)) {
                    // A quarter of the difference, at least one unit, so a
                    // pool levels out to within one unit.
                    self.transfer(g, n, s, ((a - b) / 4).max(1));
                }
            }
        }
    }
}

const DIRS: [[i32; 3]; 6] = [
    [0, -1, 0],
    [-1, 0, 0],
    [1, 0, 0],
    [0, 0, -1],
    [0, 0, 1],
    [0, 1, 0],
];
const SIDES: [[i32; 3]; 4] = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];

fn add3(a: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [a[0] + d[0], a[1] + d[1], a[2] + d[2]]
}

/// True when gas cell `g` has fewer than 5 of its 8 blocks blocking gas.
pub fn open(vox: &voxel::World, solid: &[bool], g: [i32; 3]) -> bool {
    let b = g.map(|v| v * 2);
    let mut n = 0;
    for dz in 0..2 {
        for dy in 0..2 {
            for dx in 0..2 {
                n += blocks_gas(vox.get([b[0] + dx, b[1] + dy, b[2] + dz]), solid) as u32;
            }
        }
    }
    n < 5
}

/// True when gas can pass from cell `g` to its neighbour in direction `d`
/// (a unit axis vector): both cells open, and at least one pair of blocks
/// facing each other across the shared face both let gas through.
pub fn connected(vox: &voxel::World, solid: &[bool], g: [i32; 3], d: [i32; 3]) -> bool {
    let n = add3(g, d);
    if !in_field(n) || !open(vox, solid, g) || !open(vox, solid, n) {
        return false;
    }
    let a = (0..3).find(|&a| d[a] != 0).unwrap_or(0);
    let (u, v) = ((a + 1) % 3, (a + 2) % 3);
    let side = g[a] * 2 + if d[a] > 0 { 1 } else { 0 };
    for i in 0..2 {
        for j in 0..2 {
            let mut p = [0; 3];
            p[a] = side;
            p[u] = g[u] * 2 + i;
            p[v] = g[v] * 2 + j;
            let mut q = p;
            q[a] += d[a];
            if !blocks_gas(vox.get(p), solid) && !blocks_gas(vox.get(q), solid) {
                return true;
            }
        }
    }
    false
}

/// FNV-style fold of the non-empty cells of the active chunks.
pub fn hash_into(f: &Field, h: &mut u64) {
    if f.cells.is_empty() {
        return;
    }
    for c in f.active_chunks() {
        let o = voxel::World::chunk_coords(c).map(|v| v * GC);
        for z in 0..GC {
            for y in 0..GC {
                for x in 0..GC {
                    let i = idx([o[0] + x, o[1] + y, o[2] + z]);
                    let v = f.cells[i];
                    if v != [0; 4] {
                        for b in (i as u32).to_le_bytes().into_iter().chain(v) {
                            *h ^= b as u64;
                            *h = h.wrapping_mul(0x0100_0000_01b3);
                        }
                    }
                }
            }
        }
    }
}
