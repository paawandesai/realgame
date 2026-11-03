//! World generation (section 4): a column heightmap from seeded value noise
//! per biome, the fixed landforms (river canyon, cliff band, ridge, edge
//! ring, Kiln terrace), the cave tunnels, trees and ore, then the stamps and
//! their protected boxes.
//!
//! Biome placement is fixed; the seed only shapes the ground, the trees and
//! the ore inside each biome, so every seed has the same puzzles and every
//! stamp cell is the same for every seed (V2). Generation is deterministic:
//! integer hashing and `+ - * /` only.

use crate::stamp::{self, Aabb};
use crate::{mat, Biome, Cell, Chunk, World, CHUNK, CHUNK_CELLS, WORLD};

/// Sea and river surface (the top water cell).
pub const RIVER_Y: i32 = 30;
/// The village's ground: the top solid block (feet stand at 35).
pub const VILLAGE_Y: i32 = 34;
/// The Kiln terrace's top solid block.
pub const TERRACE_Y: i32 = 40;
/// The edge ring's thickness in blocks.
pub const EDGE: i32 = 8;

/// The river canyon, walls included: z 148..160 across the world. Water
/// fills z 150..158; the walls are z 148..150 and 158..160.
pub const CANYON_Z: (i32, i32) = (148, 160);
/// The river's water: z 150..158, y 28..=30, bed top at y 27.
pub const WATER_Z: (i32, i32) = (150, 158);
/// The ford: x 122..134, where the banks step down to y 31.
pub const FORD_X: (i32, i32) = (122, 134);
/// The Frozen Fields' protected cliff band (z 86..94), with the ice gate.
pub const BAND_Z: (i32, i32) = (86, 94);
/// The ridge between the Frozen Fields and the Marsh (x 132..138), with the
/// east pass (z 66..74) through it.
pub const RIDGE_X: (i32, i32) = (132, 138);
pub const PASS_Z: (i32, i32) = (66, 74);
/// The cliff between the Village and the southern Marsh (x 160..166).
pub const EAST_CLIFF_X: (i32, i32) = (160, 166);
/// The Formula Bench and the Invention Bench (section 4).
pub const BENCH: [i32; 3] = [124, 35, 200];
pub const INVENTION_BENCH: [i32; 3] = [132, 35, 200];
/// The village well (a water source).
pub const WELL: [i32; 3] = [114, 34, 206];
/// The Crystal Cave's mouth in the Marsh (section 4).
pub const CAVE_MOUTH: [i32; 3] = [220, 32, 58];

/// A Village house (docs/art-bible.md, "Built things"): clay walls three
/// high with timber corner posts, a door on the north side and a window in
/// each long wall, under a pitched roof of planks whose ridge runs along x.
/// `min` is the footprint's corner at the floor (feet height); `max` is
/// exclusive in x and z. The cells are ordinary blocks (they burn and dig
/// like any other); `built` draws them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct House {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

impl House {
    /// Wall height in blocks; the roof starts on top.
    pub const WALL: i32 = 3;

    /// Whether `p` is in the house's box, walls to ridge.
    pub fn contains(&self, p: [i32; 3]) -> bool {
        let top = self.min[1] + House::WALL + (self.max[2] - self.min[2] + 1) / 2;
        (self.min[0]..self.max[0]).contains(&p[0])
            && (self.min[1]..top).contains(&p[1])
            && (self.min[2]..self.max[2]).contains(&p[2])
    }

    /// Whether `p` is one of the roof's cells: course `k` above the walls
    /// covers rows `k ..= depth - 1 - k`.
    pub fn in_roof(&self, p: [i32; 3]) -> bool {
        let k = p[1] - self.min[1] - House::WALL;
        k >= 0
            && (self.min[0]..self.max[0]).contains(&p[0])
            && p[2] >= self.min[2] + k
            && p[2] < self.max[2] - k
    }

    /// The roof cells of slice `i` (x = `min[0] + i`), eave course first.
    pub fn roof_cells(&self, i: i32) -> Vec<[i32; 3]> {
        let x = self.min[0] + i;
        let mut v = Vec::new();
        let mut k = 0;
        while self.min[2] + k < self.max[2] - k {
            for z in self.min[2] + k..self.max[2] - k {
                v.push([x, self.min[1] + House::WALL + k, z]);
            }
            k += 1;
        }
        v
    }

    /// The door's column (x) on the north wall.
    pub fn door_x(&self) -> i32 {
        (self.min[0] + self.max[0]) / 2
    }
}

/// The Village's houses, round the plaza and clear of the paths, the
/// caches' pads, the workshop's fire and the trees.
pub const HOUSES: [House; 4] = [
    House {
        min: [104, 35, 212],
        max: [110, 35, 217],
    },
    House {
        min: [140, 35, 206],
        max: [146, 35, 211],
    },
    House {
        min: [150, 35, 210],
        max: [156, 35, 215],
    },
    House {
        min: [112, 35, 216],
        max: [118, 35, 221],
    },
];

/// Builds the houses: levels their plots, then walls, posts, door,
/// windows and roof.
fn houses(w: &mut World) {
    let c = Cell::new;
    for h in HOUSES {
        let [x0, y0, z0] = h.min;
        let (x1, z1) = (h.max[0], h.max[2]);
        // The plot, a block wider all round: ground to y0 - 1, air above.
        for z in z0 - 1..=z1 {
            for x in x0 - 1..=x1 {
                for y in y0 - 4..y0 {
                    if w.get([x, y, z]).mat == mat::AIR {
                        w.set([x, y, z], c(mat::DIRT));
                    }
                }
                for y in y0..y0 + 8 {
                    w.set([x, y, z], Cell::AIR);
                }
            }
        }
        let door = h.door_x();
        for z in z0..z1 {
            for x in x0..x1 {
                let ex = x == x0 || x == x1 - 1;
                let ez = z == z0 || z == z1 - 1;
                w.set([x, y0 - 1, z], c(mat::PLANKS));
                for y in y0..y0 + House::WALL {
                    let cell = if ex && ez {
                        c(mat::WOOD)
                    } else if z == z0 && x == door && y < y0 + 2 {
                        Cell::AIR
                    } else if ez && y == y0 + 1 && (x == x0 + 1 || x == x1 - 2) && x != door {
                        // A window each side of the door, and in the back.
                        Cell::AIR
                    } else if ex || ez {
                        c(mat::BRICK)
                    } else {
                        Cell::AIR
                    };
                    w.set([x, y, z], cell);
                }
            }
        }
        for i in 0..x1 - x0 {
            for q in h.roof_cells(i) {
                w.set(q, c(mat::PLANKS));
            }
        }
    }
}

/// Which biome a block is in (the `biome` state key and `?biome=` spawns).
pub fn biome_at(p: [i32; 3]) -> Biome {
    let [x, y, z] = p;
    if (188..248).contains(&x) && (10..70).contains(&z) && y < 27 {
        return Biome::Cave;
    }
    if z >= CANYON_Z.1 {
        // The Kiln terrace (y 41 from z 162); the river bank in front of
        // it (y 35, z 160-163) is the Village's.
        if (112..144).contains(&x) && z < 177 && (z >= 164 || y >= 39) {
            return Biome::Kiln;
        }
        if x >= EAST_CLIFF_X.1 {
            return Biome::Marsh;
        }
        return Biome::Village;
    }
    if z >= CANYON_Z.0 {
        return if (FORD_X.0..FORD_X.1).contains(&x) || x < 136 {
            Biome::Frozen
        } else {
            Biome::Marsh
        };
    }
    if x >= RIDGE_X.1 - 2 {
        Biome::Marsh
    } else {
        Biome::Frozen
    }
}

/// Where `?biome=` puts the player: the feet position (block centre, on
/// the ground) and the yaw in radians (0 faces north, -z).
pub fn spawn_for(world: &World, biome: Biome) -> ([f32; 3], f32) {
    let (x, z, y_hint, yaw) = match biome {
        Biome::Village => return (crate::SPAWN, 0.0),
        Biome::Frozen => (104, 128, 60, 0.0),
        Biome::Marsh => (156, 120, 60, 0.0),
        Biome::Cave => (220, 46, 24, 0.0),
        Biome::Kiln => (128, 176, 60, 0.0),
    };
    let y = ground_below(world, x, z, y_hint);
    ([x as f32 + 0.5, y as f32, z as f32 + 0.5], yaw)
}

/// The lowest y at or below `from` where the player fits: a solid block
/// under two non-solid ones (the feet position). Falls back to `from`.
pub fn ground_below(world: &World, x: i32, z: i32, from: i32) -> i32 {
    let solid = |y: i32| crate::materials::flags(world.get([x, y, z]).mat).solid;
    let mut y = from.min(WORLD[1] - 2);
    while y > 1 {
        if solid(y - 1) && !solid(y) && !solid(y + 1) {
            return y;
        }
        y -= 1;
    }
    from
}

// ---------------------------------------------------------------------------
// Noise: integer hash, bilinear value noise with a smoothstep fade.
// ---------------------------------------------------------------------------

fn hash3(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    let mut h = seed ^ 0x9e37_79b9_7f4a_7c15;
    for v in [x, y, z] {
        h ^= (v as u32 as u64).wrapping_mul(0xff51_afd7_ed55_8ccd);
        h = h.rotate_left(27).wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    }
    h ^= h >> 31;
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 29)
}

/// Uniform in 0..1 from a hash.
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Value noise in 0..1 with cells `scale` blocks wide.
fn noise2(seed: u64, x: i32, z: i32, scale: i32) -> f32 {
    let (cx, cz) = (x.div_euclid(scale), z.div_euclid(scale));
    let fx = x.rem_euclid(scale) as f32 / scale as f32;
    let fz = z.rem_euclid(scale) as f32 / scale as f32;
    let fade = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (fade(fx), fade(fz));
    let at = |i: i32, j: i32| unit(hash3(seed, cx + i, 0, cz + j));
    let a = at(0, 0) + (at(1, 0) - at(0, 0)) * u;
    let b = at(0, 1) + (at(1, 1) - at(0, 1)) * u;
    a + (b - a) * v
}

/// Two octaves, 0..1.
fn fbm(seed: u64, x: i32, z: i32, scale: i32) -> f32 {
    (noise2(seed, x, z, scale) * 2.0 + noise2(seed ^ 0x51, x, z, scale / 2)) / 3.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Columns.
// ---------------------------------------------------------------------------

/// One column of terrain: solid up to `top` (surface `surf`, three blocks
/// of `sub` under it, `deep` below), then water up to `water_top`.
#[derive(Clone, Copy)]
struct Column {
    top: i32,
    surf: u8,
    sub: u8,
    deep: u8,
    water_top: i32,
    source: bool,
}

impl Column {
    fn plain(top: i32, surf: u8, sub: u8) -> Column {
        Column {
            top,
            surf,
            sub,
            deep: mat::STONE,
            water_top: 0,
            source: false,
        }
    }

    fn rock(top: i32, m: u8) -> Column {
        Column {
            top,
            surf: m,
            sub: m,
            deep: m,
            water_top: 0,
            source: false,
        }
    }

    fn mat_at(&self, y: i32) -> (u8, bool) {
        if y < self.top - 3 {
            (self.deep, false)
        } else if y < self.top {
            (self.sub, false)
        } else if y == self.top {
            (self.surf, false)
        } else if y <= self.water_top {
            (mat::WATER, self.source)
        } else {
            (mat::AIR, false)
        }
    }
}

/// Ridged noise in 0..1: sharp crests where the noise crosses its middle
/// (mountain ridges and peaks rather than round hills).
fn ridged(seed: u64, x: i32, z: i32, scale: i32) -> f32 {
    let r = 1.0 - (2.0 * noise2(seed, x, z, scale) - 1.0).abs();
    r * r
}

/// The edge ring's height at a column `edge` blocks in from the world's
/// border: jagged peaks and saddles (ridged noise), only gently higher
/// towards the border, so its top reads as a mountain range and not as
/// concentric terraces. Never below 60: the ring's inner face stays a
/// wall that cannot be climbed from the valley (its highest ground is 50).
const RING_MIN: i32 = 60;

fn ring_top(seed: u64, x: i32, z: i32, edge: i32, n: f32) -> i32 {
    let peaks = 22.0 * ridged(seed ^ 0xe1, x, z, 11) + 8.0 * ridged(seed ^ 0xe2, x, z, 5);
    let h = 60 + (6.0 * n) as i32 + peaks as i32 + (EDGE - edge).max(0);
    h.clamp(RING_MIN, WORLD[1] - 4)
}

fn ring(seed: u64, x: i32, z: i32, edge: i32, n: f32) -> Column {
    let mut c = Column::rock(ring_top(seed, x, z, edge, n), mat::CLIFFSTONE);
    if c.top > 74 {
        c.surf = mat::SNOW;
    }
    c
}

/// Whether `(x, z)` is within `margin` of a stamp, the river canyon, the
/// village plaza or the ford: cliff skirts and buttresses keep off them.
fn keep_clear(x: i32, z: i32, margin: i32) -> bool {
    let canyon = (CANYON_Z.0 - margin..CANYON_Z.1 + margin).contains(&z);
    let plaza =
        (100 - margin..158 + margin).contains(&x) && (176 - margin..228 + margin).contains(&z);
    let stamp = stamp::ALL.iter().any(|s| {
        x >= s.min[0] - margin
            && x < s.max[0] + margin
            && z >= s.min[2] - margin
            && z < s.max[2] + margin
    });
    canyon || plaza || stamp
}

/// A steep skirt at the foot of a cliff: the cliff's top `top`, `d` blocks
/// out from its face, over ground `ground`. It falls 3-5 blocks a block (so
/// it can never be walked or jumped up) into a short scree of loose stone
/// one or two blocks over the ground, so a cliff meets the valley as
/// buttresses and rubble, not as a box. None where the skirt has already
/// reached the ground.
///
/// `cap` is the highest a skirt may stand: three blocks under the lowest
/// the cliff's top ever is. A skirt's own steps can be climbed (it is
/// rubble), but from its highest point the cliff above is still a wall.
fn skirt(seed: u64, x: i32, z: i32, top: i32, cap: i32, d: i32, ground: &Column) -> Option<Column> {
    // How far out this stretch of cliff reaches (0-6), and how fast it falls.
    let reach = (7.0 * noise2(seed ^ 0xb7, x, z, 7).powf(1.3)) as i32;
    let fall = 3 + (hash3(seed ^ 0xb9, x, 0, z) % 3) as i32;
    if d <= reach {
        let h = (top - fall * d).min(cap).min(ground.top + 12);
        if h > ground.top + 2 {
            return Some(Column::rock(h, mat::CLIFFSTONE));
        }
    }
    // The scree: just past the buttress, a block or two of loose stone.
    if d <= reach + 2 && ground.water_top <= ground.top {
        let rubble = 1 + (hash3(seed ^ 0xbb, x, 1, z) % 2) as i32 - (d - reach - 1).max(0);
        if rubble > 0 {
            let mut c = *ground;
            c.top += rubble;
            c.surf = mat::STONE;
            return Some(c);
        }
    }
    None
}

/// The Frozen Fields' cliff band: at least 56 (the hills reach 50), with
/// crags and saddles along its top.
const BAND_MIN: i32 = 56;
fn band_top(seed: u64, x: i32, z: i32, n: f32) -> i32 {
    BAND_MIN + (3.0 * n) as i32 + (7.0 * ridged(seed ^ 0xc1, x, z, 6)) as i32
}

/// The ridge: at least 58, with peaks along it; level (60-63, as before)
/// for ten blocks either side of the pass, so the Marsh gate's wall stands
/// as high as the ridge beside it.
const RIDGE_MIN: i32 = 58;
fn ridge_top(seed: u64, x: i32, z: i32, n: f32) -> i32 {
    if (PASS_Z.0 - 10..PASS_Z.1 + 10).contains(&z) {
        return 60 + (4.0 * n) as i32;
    }
    RIDGE_MIN + (4.0 * n) as i32 + (8.0 * ridged(seed ^ 0xc3, x, z, 7)) as i32
}

/// The cliff between the Village and the southern Marsh: at least 48.
const EAST_MIN: i32 = 48;
fn east_top(seed: u64, x: i32, z: i32, n: f32) -> i32 {
    EAST_MIN + (4.0 * n) as i32 + (7.0 * ridged(seed ^ 0xc5, x, z, 6)) as i32
}

/// The skirt (buttresses, scree) at the foot of the band, the ridge or the
/// east cliff, for a column `d` blocks out from one of them.
fn cliff_skirt(seed: u64, x: i32, z: i32, ground: &Column) -> Option<Column> {
    const OUT: i32 = 9;
    let near = |a: i32, lo: i32, hi: i32| -> Option<(i32, i32)> {
        // (distance out, the face's coordinate) on either side of [lo, hi).
        if a < lo && lo - a <= OUT {
            Some((lo - a, lo))
        } else if a >= hi && a - hi < OUT {
            Some((a - hi + 1, hi - 1))
        } else {
            None
        }
    };
    // The band (north of the river, west of the ridge).
    if (EDGE..RIDGE_X.0).contains(&x) && z < CANYON_Z.0 {
        if let Some((d, fz)) = near(z, BAND_Z.0, BAND_Z.1) {
            let top = band_top(seed, x, fz, fbm(seed, x, fz, 32));
            if let Some(c) = skirt(seed, x, z, top, BAND_MIN - 3, d, ground) {
                return Some(c);
            }
        }
    }
    // The ridge, away from its pass.
    if (EDGE..CANYON_Z.0).contains(&z) && !(PASS_Z.0 - 12..PASS_Z.1 + 12).contains(&z) {
        if let Some((d, fx)) = near(x, RIDGE_X.0, RIDGE_X.1) {
            let top = ridge_top(seed, fx, z, fbm(seed, fx, z, 32));
            if let Some(c) = skirt(seed, x, z, top, RIDGE_MIN - 3, d, ground) {
                return Some(c);
            }
        }
    }
    // The east cliff, south of the river.
    if z >= CANYON_Z.1 {
        if let Some((d, fx)) = near(x, EAST_CLIFF_X.0, EAST_CLIFF_X.1) {
            let top = east_top(seed, fx, z, fbm(seed, fx, z, 32));
            if let Some(c) = skirt(seed, x, z, top, EAST_MIN - 3, d, ground) {
                return Some(c);
            }
        }
    }
    None
}

fn column(seed: u64, x: i32, z: i32) -> Column {
    let n = fbm(seed, x, z, 32);
    // The edge ring of tall mountains.
    let edge = x.min(z).min(WORLD[0] - 1 - x).min(WORLD[2] - 1 - z);
    if edge < EDGE {
        return ring(seed, x, z, edge, n);
    }
    let inner = inner_column(seed, x, z, n);
    // Buttresses and scree at the ring's foot.
    if edge < EDGE + 9 && !keep_clear(x, z, 4) {
        let d = edge - EDGE + 1;
        // The ring column just outside this one, towards the border.
        let (ox, oz) = if edge == x {
            (EDGE - 1, z)
        } else if edge == z {
            (x, EDGE - 1)
        } else if edge == WORLD[0] - 1 - x {
            (WORLD[0] - EDGE, z)
        } else {
            (x, WORLD[2] - EDGE)
        };
        let top = ring_top(seed, ox, oz, EDGE - 1, fbm(seed, ox, oz, 32));
        if let Some(c) = skirt(seed, x, z, top, RING_MIN - 3, d, &inner) {
            return c;
        }
    }
    let rock = |c: &Column| c.surf == mat::CLIFFSTONE && c.top > 44;
    if !rock(&inner) && !keep_clear(x, z, 4) {
        if let Some(c) = cliff_skirt(seed, x, z, &inner) {
            return c;
        }
    }
    inner
}

fn inner_column(seed: u64, x: i32, z: i32, n: f32) -> Column {
    // The river canyon.
    if (CANYON_Z.0..CANYON_Z.1).contains(&z) {
        if (FORD_X.0..FORD_X.1).contains(&x) {
            // The ford: the banks step down to y 31 on both sides; the water
            // itself is the FORD stamp.
            let top = match z {
                148 => 32,
                149 => 31,
                158 => 31,
                159 => 32,
                _ => 27,
            };
            let mut c = Column::plain(top, mat::SAND, mat::SAND);
            if (WATER_Z.0..WATER_Z.1).contains(&z) {
                c.water_top = RIVER_Y;
            }
            return c;
        }
        if (WATER_Z.0..WATER_Z.1).contains(&z) {
            let mut c = Column::plain(27, mat::SAND, mat::SAND);
            c.water_top = RIVER_Y;
            c.source = true;
            return c;
        }
        return Column::rock(VILLAGE_Y, mat::CLIFFSTONE);
    }
    if z >= CANYON_Z.1 {
        return south(seed, x, z, n);
    }
    north(seed, x, z, n)
}

/// South of the river: the Village, the Kiln terrace, the east cliff and
/// the southern Marsh.
fn south(seed: u64, x: i32, z: i32, n: f32) -> Column {
    // The ford's south bank steps (z 160, 161) continue the canyon's.
    if (FORD_X.0..FORD_X.1).contains(&x) && z < 162 {
        return Column::plain(33 + (z - 160), mat::GRASS, mat::DIRT);
    }
    // The Kiln terrace, raised to y 40, with steps down to the village on
    // its south side (x 124..132, z 176..181).
    if (112..144).contains(&x) && (162..176).contains(&z) {
        return Column::rock(TERRACE_Y, mat::STONE);
    }
    if (124..132).contains(&x) && (176..181).contains(&z) {
        return Column::rock(TERRACE_Y - 1 - (z - 176), mat::BRICK);
    }
    if (EAST_CLIFF_X.0..EAST_CLIFF_X.1).contains(&x) {
        return Column::rock(east_top(seed, x, z, n), mat::CLIFFSTONE);
    }
    if x >= EAST_CLIFF_X.1 {
        return marsh(seed, x, z);
    }
    // The village: a flat plaza around the benches and spawn, gentle
    // meadows further out.
    let plaza = (100..158).contains(&x) && (176..228).contains(&z);
    let near_canyon = z < 164;
    let top = if plaza || near_canyon {
        VILLAGE_Y
    } else {
        VILLAGE_Y + (2.4 * fbm(seed ^ 0x7, x, z, 24)) as i32
    };
    Column::plain(top, mat::GRASS, mat::DIRT)
}

/// North of the river: the Frozen Fields, the cliff band, the ridge with
/// its pass, and the Marsh.
fn north(seed: u64, x: i32, z: i32, n: f32) -> Column {
    // The ford's north bank steps (z 146, 147) continue the canyon's.
    if (FORD_X.0..FORD_X.1).contains(&x) && z >= 146 {
        return Column::plain(VILLAGE_Y - (z - 146), mat::SNOW, mat::DIRT);
    }
    // The cliff band across the Frozen Fields (the ice gate is a stamp).
    if (BAND_Z.0..BAND_Z.1).contains(&z) && x < RIDGE_X.0 {
        return Column::rock(band_top(seed, x, z, n), mat::CLIFFSTONE);
    }
    // The ridge between the Frozen Fields and the Marsh, with the pass.
    if (RIDGE_X.0..RIDGE_X.1).contains(&x) {
        if (PASS_Z.0..PASS_Z.1).contains(&z) {
            let t = (x - RIDGE_X.0) as f32 / (RIDGE_X.1 - RIDGE_X.0) as f32;
            return Column::plain(lerp(42.0, 33.0, t) as i32, mat::SNOW, mat::DIRT);
        }
        return Column::rock(ridge_top(seed, x, z, n), mat::CLIFFSTONE);
    }
    if x >= RIDGE_X.1 {
        return marsh(seed, x, z);
    }
    // The Frozen Fields: snowy hills, 38-50, sloping down to the ford
    // (z > 128), to the ice gate and to the pass.
    let mut h = 38.0 + 12.0 * fbm(seed ^ 0x3, x, z, 28);
    h = lerp(h, 34.0, (z - 128) as f32 / 16.0);
    let gate = stamp::ICE_GATE;
    let dx = (x - (gate.min[0] + gate.max[0]) / 2).abs() as f32;
    let dz = if z < gate.min[2] {
        gate.min[2] - z
    } else {
        z - gate.max[2] + 1
    } as f32;
    h = lerp(35.0, h, (dx.max(dz) - 3.0) / 10.0);
    if (PASS_Z.0 - 6..PASS_Z.1 + 6).contains(&z) && x > 112 {
        h = lerp(h, 42.0, (x - 112) as f32 / 12.0);
    }
    // The coal seam's surface is cut flat at its top (y 42).
    let seam = stamp::COAL_SEAM;
    if (seam.min[0]..seam.max[0]).contains(&x) && (seam.min[2]..seam.max[2]).contains(&z) {
        h = (seam.max[1] - 1) as f32;
    }
    let top = h as i32;
    // Frozen ponds in the low hollows.
    let pond = fbm(seed ^ 0x11, x, z, 20) < 0.3 && top < 44 && z < 126;
    let surf = if pond { mat::ICE } else { mat::SNOW };
    Column::plain(top, surf, mat::DIRT)
}

/// The Marsh: low ground (30-33), mud and grass, shallow pools.
fn marsh(seed: u64, x: i32, z: i32) -> Column {
    let m = fbm(seed ^ 0x5, x, z, 18);
    let top = 30 + (3.4 * m) as i32;
    let mut c = Column::plain(top, mat::GRASS, mat::MUD);
    if m < 0.28 {
        c.top = 30;
        c.surf = mat::MUD;
        c.water_top = 31;
    } else if m < 0.36 {
        c.surf = mat::MUD;
    }
    c
}

// ---------------------------------------------------------------------------
// The Crystal Cave: fixed tunnels (capsules), the same for every seed.
// ---------------------------------------------------------------------------

/// Tunnel segments: from, to (block centres) and radius.
const TUNNELS: [([i32; 3], [i32; 3], i32); 8] = [
    // The mouth slopes down from the marsh.
    ([220, 31, 58], [220, 22, 50], 2),
    ([220, 22, 50], [218, 13, 42], 3),
    // The great hall.
    ([218, 13, 42], [210, 13, 30], 4),
    ([210, 13, 30], [200, 13, 22], 3),
    // To the rig room.
    ([218, 13, 42], [228, 12, 34], 2),
    // From the rig room to the balloon shaft.
    ([234, 12, 26], [239, 12, 22], 2),
    // A side gallery.
    ([200, 13, 22], [196, 14, 50], 2),
    ([196, 14, 50], [205, 14, 64], 2),
];

fn carve_cave(w: &mut World, seed: u64) {
    for (a, b, r) in TUNNELS {
        let lo = [0, 1, 2].map(|k| a[k].min(b[k]) - r - 1);
        let hi = [0, 1, 2].map(|k| a[k].max(b[k]) + r + 1);
        let d = [0, 1, 2].map(|k| (b[k] - a[k]) as f32);
        let len2 = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).max(1e-6);
        for z in lo[2]..=hi[2] {
            for y in lo[1].max(2)..=hi[1] {
                for x in lo[0]..=hi[0] {
                    let p = [x - a[0], y - a[1], z - a[2]].map(|v| v as f32);
                    let t = ((p[0] * d[0] + p[1] * d[1] + p[2] * d[2]) / len2).clamp(0.0, 1.0);
                    let q = [p[0] - d[0] * t, p[1] - d[1] * t, p[2] - d[2] * t];
                    // Flattened: a little wider than tall, with a flat floor.
                    let dist2 = q[0] * q[0] + q[1] * q[1] * 1.6 + q[2] * q[2];
                    let rr = r as f32 + 0.4;
                    if dist2 <= rr * rr && q[1] > -(r as f32) + 0.6 {
                        w.set([x, y, z], Cell::AIR);
                    }
                }
            }
        }
    }
    // Crystals on the cave walls and ceilings (dim, emissive in the view).
    for z in 10..70 {
        for x in 188..248 {
            for y in 6..27 {
                let p = [x, y, z];
                if w.get(p).mat != mat::STONE {
                    continue;
                }
                let open = [
                    [1, 0, 0],
                    [-1, 0, 0],
                    [0, 1, 0],
                    [0, -1, 0],
                    [0, 0, 1],
                    [0, 0, -1],
                ]
                .iter()
                .any(|d| w.get([x + d[0], y + d[1], z + d[2]]).mat == mat::AIR);
                if open && unit(hash3(seed ^ 0xc7, x, y, z)) < 0.035 {
                    w.set(p, Cell::new(mat::CRYSTAL));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trees.
// ---------------------------------------------------------------------------

fn plant_trees(w: &mut World, seed: u64, cols: &[Column]) {
    const GRID: i32 = 7;
    for gz in 0..WORLD[2] / GRID {
        for gx in 0..WORLD[0] / GRID {
            let h = hash3(seed ^ 0x7ee, gx, 0, gz);
            let x = gx * GRID + (h % GRID as u64) as i32;
            let z = gz * GRID + ((h >> 8) % GRID as u64) as i32;
            if !(EDGE + 2..WORLD[0] - EDGE - 2).contains(&x)
                || !(EDGE + 2..WORLD[2] - EDGE - 2).contains(&z)
            {
                continue;
            }
            let c = cols[(z * WORLD[0] + x) as usize];
            let density = unit(h >> 16);
            let (pine, chance) = match c.surf {
                mat::SNOW => (true, 0.45),
                mat::GRASS if biome_at([x, c.top, z]) == Biome::Village => (false, 0.5),
                mat::GRASS => (false, 0.18),
                _ => continue,
            };
            if density > chance || c.water_top > c.top {
                continue;
            }
            // Keep the village plaza, paths and every stamp clear.
            let plaza = (100..158).contains(&x) && (176..228).contains(&z);
            let riverbank = (CANYON_Z.0 - 8..CANYON_Z.1 + 6).contains(&z);
            let near_stamp = stamp::ALL.iter().any(|s| {
                x >= s.min[0] - 3 && x < s.max[0] + 3 && z >= s.min[2] - 3 && z < s.max[2] + 3
            });
            if plaza || riverbank || near_stamp {
                continue;
            }
            let trunk = 4 + (h >> 24) as i32 % 2;
            let base = c.top + 1;
            for y in base..base + trunk {
                w.set([x, y, z], Cell::new(mat::WOOD));
            }
            let crown = base + trunk;
            if pine {
                for (k, r) in [(-2, 2), (-1, 1), (0, 2), (1, 1), (2, 0)] {
                    leaf_disc(w, [x, crown + k, z], r);
                }
            } else {
                for (k, r) in [(-1, 2), (0, 2), (1, 1)] {
                    leaf_disc(w, [x, crown + k, z], r);
                }
            }
        }
    }
}

fn leaf_disc(w: &mut World, c: [i32; 3], r: i32) {
    for dz in -r..=r {
        for dx in -r..=r {
            if dx * dx + dz * dz > r * r + r / 2 {
                continue;
            }
            let p = [c[0] + dx, c[1], c[2] + dz];
            if w.get(p).mat == mat::AIR {
                w.set(p, Cell::new(mat::LEAVES));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stamps: each writes every cell of its box from a fixed template (no seed).
// ---------------------------------------------------------------------------

fn fill(w: &mut World, lo: [i32; 3], hi: [i32; 3], c: Cell) {
    for z in lo[2]..hi[2] {
        for y in lo[1]..hi[1] {
            for x in lo[0]..hi[0] {
                w.set([x, y, z], c);
            }
        }
    }
}

fn water(source: bool) -> Cell {
    let mut c = Cell::new(mat::WATER);
    c.set_level(8);
    c.set_flag(Cell::SOURCE, source);
    c
}

fn stamp_box(w: &mut World, s: &stamp::Stamp, f: impl Fn([i32; 3]) -> Cell) {
    for z in s.min[2]..s.max[2] {
        for y in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                w.set([x, y, z], f([x, y, z]));
            }
        }
    }
}

fn stamps(w: &mut World) {
    let a = Cell::AIR;
    let c = Cell::new;

    // P1: the workshop, burning at the start. Its roof has burnt away;
    // low brick walls, wood corner posts and a stone floor are left, and in
    // the middle its fuel store (stacked charcoal and logs) burns on: a
    // store holds far more fuel than one block, so it keeps burning until
    // water cools it below ignition (`SOURCE` on a flammable block; see
    // content/science-sources.md, p1-fuel-store). A plank workbench stands
    // against the store, and the door faces the plaza.
    let s = stamp::WORKSHOP;
    stamp_box(w, &s, |[x, y, z]| {
        let edge_x = x == s.min[0] || x == s.max[0] - 1;
        let edge_z = z == s.min[2] || z == s.max[2] - 1;
        let store = (WORKSHOP_STORE.0[0]..WORKSHOP_STORE.1[0]).contains(&x)
            && (WORKSHOP_STORE.0[1]..WORKSHOP_STORE.1[1]).contains(&y)
            && (WORKSHOP_STORE.0[2]..WORKSHOP_STORE.1[2]).contains(&z);
        match y {
            34 => c(mat::STONE),
            _ if store => {
                let mut p = c(mat::WOOD);
                p.set_flag(Cell::BURNING, true);
                p.set_flag(Cell::SOURCE, true);
                p
            }
            35 if x == WORKSHOP_STORE.0[0] - 1
                && (WORKSHOP_STORE.0[2]..WORKSHOP_STORE.1[2]).contains(&z) =>
            {
                c(mat::PLANKS)
            }
            35..=38 if edge_x && edge_z => c(mat::WOOD),
            // The door on the south side, towards the plaza.
            35..=36 if z == s.max[2] - 1 && (141..=142).contains(&x) => a,
            35..=36 if edge_x || edge_z => c(mat::BRICK),
            _ => a,
        }
    });

    // P2a: the ford, 3-deep still water (no source) on a sand bed.
    let s = stamp::FORD;
    stamp_box(w, &s, |[_, y, _]| match y {
        ..=26 => c(mat::STONE),
        27 => c(mat::SAND),
        28..=RIVER_Y => water(false),
        _ => a,
    });
    // Its north bank steps up a block a metre, but its first step stood a
    // block above the water, so frozen the ford met a drawn ledge W could
    // not walk up (I-052 r3, smooth ground). Each of the bank's first four
    // rows is lowered a block, so the bank starts level with the ice.
    for k in 0..4 {
        let z = s.min[2] - 1 - k;
        for x in s.min[0]..s.max[0] {
            let top = RIVER_Y + 1 + k;
            let cell = w.get([x, top, z]);
            if cell.mat != mat::AIR && w.get([x, top + 1, z]).mat == mat::AIR {
                w.set([x, top - 1, z], cell);
                w.set([x, top, z], a);
            }
        }
    }

    // P2b: the ice gate: a passage 4 wide and 3 tall through the cliff
    // band (floor top y 35), plugged on its south face by one layer of ice
    // (z 93); cliffstone above. In front of it the ground steps down one
    // block into a trench, so the meltwater runs away downhill.
    let s = stamp::ICE_GATE;
    stamp_box(w, &s, |[_, y, z]| match y {
        ..=35 => c(mat::STONE),
        36..=38 if z == s.max[2] - 1 => c(mat::ICE),
        36..=38 => a,
        _ => c(mat::CLIFFSTONE),
    });
    fill(
        w,
        [s.min[0] - 1, 35, s.max[2]],
        [s.max[0] + 1, 40, s.max[2] + 4],
        a,
    );
    fill(
        w,
        [s.min[0] - 1, 33, s.max[2]],
        [s.max[0] + 1, 35, s.max[2] + 4],
        c(mat::STONE),
    );

    // The coal seam behind the ice gate: stone with coal ore, the top of
    // the seam open to the sky.
    let s = stamp::COAL_SEAM;
    stamp_box(w, &s, |[x, y, z]| {
        let h = hash3(0xc0a1, x, y, z);
        if unit(h) < 0.28 || (y == s.max[1] - 1 && unit(h) < 0.7) {
            c(mat::COAL_ORE)
        } else if unit(h) > 0.985 {
            c(mat::IRON_ORE)
        } else {
            c(mat::STONE)
        }
    });

    // P3: the methane hollow, a boardwalk ravine with 6 gas vents on its
    // floor (y 27) and one low gap in its rim on the marsh side.
    let s = stamp::HOLLOW;
    stamp_box(w, &s, |[x, y, z]| {
        let wall = x == s.min[0] || x == s.max[0] - 1 || z == s.min[2] || z == s.max[2] - 1;
        let gap = [x, y, z] == HOLLOW_GAP;
        // The boardwalk stops one block short of the rim gap, so the gap
        // opens into the hollow (a plank beside it would plug it).
        let boardwalk = (100..=101).contains(&z) && !wall && x < HOLLOW_GAP[0] - 1;
        match y {
            26 => c(mat::STONE),
            27 if HOLLOW_VENTS.contains(&[x, y, z]) => c(mat::GAS_VENT),
            27 => c(mat::MUD),
            _ if gap => a,
            28 if boardwalk => c(mat::PLANKS),
            28..=29 if wall => c(mat::CLIFFSTONE),
            // The east rim stands full height over the pond, so CO2 leaves
            // only by the gap.
            30..=33 if x == s.max[0] - 1 => c(mat::CLIFFSTONE),
            _ => a,
        }
    });

    // The electrolysis rig room in the cave (floor y 10).
    let s = stamp::RIG;
    stamp_box(w, &s, |[x, y, z]| match y {
        10 => c(mat::STONE),
        11 if [x, z] == [231, 29] => c(mat::GLASS),
        11 if [x, z] == [230, 29] || [x, z] == [232, 29] => c(mat::IRON),
        15 if (x + z) % 5 == 0 => c(mat::CRYSTAL),
        _ => a,
    });

    // P4: the balloon shaft, walled, open to the ledge at the top.
    let s = stamp::SHAFT;
    stamp_box(w, &s, |[x, y, z]| {
        let wall = x == s.min[0] || x == s.max[0] - 1 || z == s.min[2] || z == s.max[2] - 1;
        let to_ledge = x == s.max[0] - 1 && y >= 23 && (19..=21).contains(&z);
        let entrance = x == s.min[0] && (11..=13).contains(&y) && (19..=21).contains(&z);
        match y {
            10 => c(mat::STONE),
            _ if to_ledge || entrance => a,
            _ if wall => c(mat::CLIFFSTONE),
            _ => a,
        }
    });

    // The ledge at the top of the shaft (the Kiln Spark sits on it, y 24).
    let s = stamp::LEDGE;
    stamp_box(w, &s, |[_, y, _]| match y {
        24 => c(mat::STONE),
        _ => a,
    });

    // P5: the Great Kiln: a brick dome with a chimney to y 60, a runestone
    // door on the south side, the hearth and a glass-sand bed inside.
    let s = stamp::KILN;
    let (cx, cz) = (128.0f32, 169.0f32);
    stamp_box(w, &s, |[x, y, z]| {
        if y == TERRACE_Y {
            return c(mat::BRICK);
        }
        if [x, y, z] == KILN_HEARTH {
            return c(mat::KILN_HEARTH);
        }
        let (dx, dy, dz) = (
            (x as f32 + 0.5 - cx) / 7.5,
            (y - TERRACE_Y) as f32 / 9.0,
            (z as f32 + 0.5 - cz) / 4.6,
        );
        let r2 = dx * dx + dy * dy + dz * dz;
        let chimney = (127..=129).contains(&x) && (168..=170).contains(&z);
        if chimney {
            return if x == 128 && z == 169 && y > 49 {
                a
            } else {
                c(mat::BRICK)
            };
        }
        let door = (127..=129).contains(&x) && (41..=43).contains(&y) && z >= 172;
        if door && r2 <= 1.0 {
            return c(mat::RUNESTONE);
        }
        if r2 <= 1.0 {
            if r2 > 0.62 {
                return c(mat::BRICK);
            }
            if y == TERRACE_Y + 1 && (124..=126).contains(&x) && (167..=170).contains(&z) {
                return c(mat::SAND);
            }
            return a;
        }
        a
    });

    // The shortcut door: a tunnel through the east cliff, plugged with a
    // runestone door on the village side.
    let s = stamp::SHORTCUT;
    stamp_box(w, &s, |[x, y, z]| {
        let tunnel = (179..=180).contains(&z) && (35..=37).contains(&y);
        match y {
            34 => c(mat::STONE),
            _ if tunnel && x == s.min[0] => c(mat::RUNESTONE),
            _ if tunnel => a,
            _ => c(mat::CLIFFSTONE),
        }
    });

    // The benches on the plaza, and the well.
    w.set(BENCH, c(mat::BENCH));
    w.set(INVENTION_BENCH, c(mat::INVENTION_BENCH));
    fill(
        w,
        [WELL[0] - 1, WELL[1] - 2, WELL[2] - 1],
        [WELL[0] + 3, WELL[1] + 2, WELL[2] + 3],
        c(mat::STONE),
    );
    fill(
        w,
        [WELL[0], WELL[1] - 2, WELL[2]],
        [WELL[0] + 2, WELL[1] + 1, WELL[2] + 2],
        water(true),
    );
    fill(
        w,
        [WELL[0] - 1, WELL[1] + 1, WELL[2] - 1],
        [WELL[0] + 3, WELL[1] + 2, WELL[2] + 3],
        a,
    );
    for (dx, dz) in [(-1, -1), (2, -1), (-1, 2), (2, 2)] {
        w.set([WELL[0] + dx, WELL[1] + 1, WELL[2] + dz], c(mat::STONE));
    }
    // East of the hollow's rim wall: a sunken marsh pond, its still water
    // (y 23-25) five blocks below the gap, open to the sky. CO2 poured into
    // the hollow pours out through an open gap and sinks onto the pond
    // (section 5.5); a block in the gap stops it. Falling in is safe: deep
    // water pushes the player back to the bank (section 5.4).
    let [gx, _, _] = HOLLOW_GAP;
    let (p0, p1) = HOLLOW_POND;
    fill(w, [p0[0], 22, p0[1]], [p1[0], 23, p1[1]], c(mat::STONE));
    fill(w, [p0[0], 23, p0[1]], [p1[0], 26, p1[1]], water(false));
    fill(w, [p0[0], 26, p0[1]], [p1[0], 40, p1[1]], a);
    debug_assert_eq!(p0[0], gx + 1);
    // The cave mouth opens to the sky.
    fill(
        w,
        [CAVE_MOUTH[0] - 2, CAVE_MOUTH[1] - 2, CAVE_MOUTH[2] - 2],
        [CAVE_MOUTH[0] + 3, CAVE_MOUTH[1] + 4, CAVE_MOUTH[2] + 3],
        a,
    );
}

/// The workshop's burning fuel store (min inclusive, max exclusive).
pub const WORKSHOP_STORE: ([i32; 3], [i32; 3]) = ([141, 35, 188], [144, 37, 190]);
/// The six gas vents on the hollow's floor.
pub const HOLLOW_VENTS: [[i32; 3]; 6] = [
    [188, 27, 98],
    [191, 27, 98],
    [194, 27, 98],
    [188, 27, 103],
    [191, 27, 103],
    [194, 27, 103],
];
/// The sunken pond east of the hollow: x and z ranges (min inclusive, max
/// exclusive).
pub const HOLLOW_POND: ([i32; 2], [i32; 2]) = ([197, 96], [215, 106]);
/// The one low gap in the hollow's rim (placeable, section 5.5).
pub const HOLLOW_GAP: [i32; 3] = [196, 28, 101];
/// The Kiln's hearth (the P5 target).
pub const KILN_HEARTH: [i32; 3] = [128, 41, 169];

/// The protected boxes: the edge ring, the bedrock, the canyon walls, the
/// cliff band, the ridge, the east cliff, the benches, and every puzzle
/// stamp except the coal seam (which is there to be dug). The hollow leaves
/// out its gap cell.
fn protect(w: &mut World) {
    let b = |min: [i32; 3], max: [i32; 3]| Aabb { min, max };
    let [wx, wy, wz] = WORLD;
    let all_y = (0, wy);
    // Edge ring and bedrock.
    w.protect(b([0, 0, 0], [EDGE, wy, wz]));
    w.protect(b([wx - EDGE, 0, 0], [wx, wy, wz]));
    w.protect(b([0, 0, 0], [wx, wy, EDGE]));
    w.protect(b([0, 0, wz - EDGE], [wx, wy, wz]));
    w.protect(b([0, 0, 0], [wx, 1, wz]));
    // The river canyon on both sides of the ford.
    for (x0, x1) in [(EDGE, FORD_X.0), (FORD_X.1, wx - EDGE)] {
        w.protect(b([x0, all_y.0, CANYON_Z.0], [x1, all_y.1, CANYON_Z.1]));
    }
    // The cliff band (the ice gate is its own stamp box).
    let g = stamp::ICE_GATE;
    w.protect(b([EDGE, 0, BAND_Z.0], [g.min[0], wy, BAND_Z.1]));
    w.protect(b([g.max[0], 0, BAND_Z.0], [RIDGE_X.0, wy, BAND_Z.1]));
    w.protect(b([g.min[0], g.max[1], BAND_Z.0], [g.max[0], wy, BAND_Z.1]));
    w.protect(b([g.min[0], 0, BAND_Z.0], [g.max[0], g.min[1], BAND_Z.1]));
    // The ridge, except the pass.
    w.protect(b([RIDGE_X.0, 0, EDGE], [RIDGE_X.1, wy, PASS_Z.0]));
    w.protect(b([RIDGE_X.0, 0, PASS_Z.1], [RIDGE_X.1, wy, CANYON_Z.0]));
    // The east cliff between the Village and the southern Marsh.
    w.protect(b(
        [EAST_CLIFF_X.0, 0, CANYON_Z.1],
        [EAST_CLIFF_X.1, wy, wz - EDGE],
    ));
    // The benches.
    for p in [BENCH, INVENTION_BENCH] {
        w.protect(b(p, [p[0] + 1, p[1] + 1, p[2] + 1]));
    }
    // The balloon shaft and the ledge, with the rock around them up to the
    // sky: the Kiln Spark is reached by balloon only, never by digging down
    // from the Marsh or sideways from the cave (late lane, P4).
    let (sh, le) = (stamp::SHAFT, stamp::LEDGE);
    w.protect(b(
        [sh.min[0] - 2, sh.min[1] - 1, sh.min[2] - 3],
        [le.max[0] + 3, wy, sh.max[2] + 3],
    ));
    // The puzzle stamps.
    for s in stamp::ALL {
        match s.id {
            "coal_seam" => {}
            "hollow" => {
                let [gx, gy, gz] = HOLLOW_GAP;
                w.protect(b(s.min, [gx, s.max[1], s.max[2]]));
                w.protect(b([gx, s.min[1], s.min[2]], [gx + 1, s.max[1], gz]));
                w.protect(b([gx, s.min[1], gz + 1], [gx + 1, s.max[1], s.max[2]]));
                w.protect(b([gx, s.min[1], gz], [gx + 1, gy, gz + 1]));
                w.protect(b([gx, gy + 1, gz], [gx + 1, s.max[1], gz + 1]));
            }
            _ => w.protect(s.aabb()),
        }
    }
}

/// Generates the world for `seed`. Biome placement is fixed; noise only
/// shapes the ground inside each biome, so every seed has the same puzzles.
pub fn generate(seed: u64) -> World {
    let mut w = World::new();
    w.seed = seed;
    // 1. Columns.
    let mut cols = Vec::with_capacity((WORLD[0] * WORLD[2]) as usize);
    for z in 0..WORLD[2] {
        for x in 0..WORLD[0] {
            cols.push(column(seed, x, z));
        }
    }
    // 2. Fill every chunk from its columns; chunks that come out all one
    // cell stay `Uniform`. There is no random ore: coal is found only in
    // the seam behind the ice gate (progression, section 2).
    for ci in 0..w.chunk_count() {
        let o = World::chunk_origin(ci);
        let mut cells = vec![Cell::AIR; CHUNK_CELLS];
        for lz in 0..CHUNK {
            for lx in 0..CHUNK {
                let (x, z) = (o[0] + lx, o[2] + lz);
                let col = &cols[(z * WORLD[0] + x) as usize];
                for ly in 0..CHUNK {
                    let y = o[1] + ly;
                    let (m, source) = col.mat_at(y);
                    let cell = if m == mat::WATER {
                        water(source)
                    } else {
                        Cell::new(m)
                    };
                    cells[(lx + CHUNK * (ly + CHUNK * lz)) as usize] = cell;
                }
            }
        }
        let first = cells[0];
        let chunk = if cells.iter().all(|&c| c == first) {
            Chunk::Uniform(first)
        } else {
            Chunk::Array(cells.into_boxed_slice())
        };
        w.put_chunk(ci, chunk);
    }
    // 3. Trees, the cave, then the stamps over everything.
    plant_trees(&mut w, seed, &cols);
    carve_cave(&mut w, seed);
    stamps(&mut w);
    houses(&mut w);
    // 4. Protection.
    protect(&mut w);
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_in_range_and_seeded() {
        for (x, z) in [(0, 0), (17, 300), (-5, 99), (255, 255)] {
            let n = fbm(1, x, z, 32);
            assert!((0.0..=1.0).contains(&n), "{n}");
        }
        assert_ne!(fbm(1, 10, 10, 32), fbm(2, 10, 10, 32));
    }

    #[test]
    fn biomes_are_where_the_spec_says() {
        assert_eq!(biome_at([128, 35, 205]), Biome::Village);
        assert_eq!(biome_at([128, 41, 170]), Biome::Kiln);
        // The river bank before the Kiln terrace is the Village's (break-2
        // should-fix 6); the terrace above it is the Kiln's.
        assert_eq!(biome_at([128, 35, 163]), Biome::Village);
        assert_eq!(biome_at([128, 41, 163]), Biome::Kiln);
        assert_eq!(biome_at([60, 40, 100]), Biome::Frozen);
        assert_eq!(biome_at([200, 31, 100]), Biome::Marsh);
        assert_eq!(biome_at([220, 12, 40]), Biome::Cave);
    }

    /// The top of each column: the highest solid cell + 1 (where feet
    /// stand), water counted as a floor at its surface.
    fn tops(w: &World) -> Vec<i32> {
        let mut out = vec![0; (WORLD[0] * WORLD[2]) as usize];
        for z in 0..WORLD[2] {
            for x in 0..WORLD[0] {
                let mut y = WORLD[1] - 1;
                while y > 0 && w.get([x, y, z]).mat == mat::AIR {
                    y -= 1;
                }
                out[(z * WORLD[0] + x) as usize] = y + 1;
            }
        }
        out
    }

    /// Cliffs with buttresses and scree still cannot be climbed: walking
    /// from the spawn with steps up of at most 2 blocks (a smooth ramp and
    /// a jump), the edge ring's top is never reached, and every column
    /// within reach of the ring stands 3 or more under the ring's lowest top.
    #[test]
    fn the_edge_ring_cannot_be_climbed_from_the_valley() {
        for seed in [crate::DEFAULT_SEED, 7, 99] {
            let w = generate(seed);
            let top = tops(&w);
            let at = |x: i32, z: i32| top[(z * WORLD[0] + x) as usize];
            let edge = |x: i32, z: i32| x.min(z).min(WORLD[0] - 1 - x).min(WORLD[2] - 1 - z);
            let mut seen = vec![false; top.len()];
            let s = crate::SPAWN;
            let mut stack = vec![(s[0] as i32, s[2] as i32)];
            seen[(s[2] as i32 * WORLD[0] + s[0] as i32) as usize] = true;
            while let Some((x, z)) = stack.pop() {
                assert!(
                    edge(x, z) >= EDGE,
                    "seed {seed}: climbed onto the ring at ({x}, {z})"
                );
                let gate = stamp::ICE_GATE;
                let in_band = (BAND_Z.0..BAND_Z.1).contains(&z)
                    && x < RIDGE_X.0
                    && !(gate.min[0] - 1..gate.max[0] + 1).contains(&x);
                assert!(
                    !in_band || at(x, z) < 50,
                    "seed {seed}: up on the band at ({x}, {z}) {}",
                    at(x, z)
                );
                let in_ridge = (RIDGE_X.0..RIDGE_X.1).contains(&x)
                    && z < CANYON_Z.0
                    && !(PASS_Z.0..PASS_Z.1).contains(&z);
                assert!(
                    !in_ridge || at(x, z) < 50,
                    "seed {seed}: up on the ridge at ({x}, {z})"
                );
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, nz) = (x + dx, z + dz);
                    if !(0..WORLD[0]).contains(&nx) || !(0..WORLD[2]).contains(&nz) {
                        continue;
                    }
                    let i = (nz * WORLD[0] + nx) as usize;
                    if !seen[i] && at(nx, nz) - at(x, z) <= 2 {
                        seen[i] = true;
                        stack.push((nx, nz));
                    }
                }
            }
            for z in 0..WORLD[2] {
                for x in 0..WORLD[0] {
                    if edge(x, z) >= EDGE
                        && edge(x, z) < EDGE + 12
                        && seen[(z * WORLD[0] + x) as usize]
                    {
                        assert!(
                            at(x, z) <= RING_MIN - 2,
                            "seed {seed}: ({x}, {z}) stands at {}",
                            at(x, z)
                        );
                    }
                }
            }
        }
    }
}
