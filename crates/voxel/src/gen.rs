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

/// Which biome a block is in (the `biome` state key and `?biome=` spawns).
pub fn biome_at(p: [i32; 3]) -> Biome {
    let [x, y, z] = p;
    if (188..248).contains(&x) && (10..70).contains(&z) && y < 27 {
        return Biome::Cave;
    }
    if z >= CANYON_Z.1 {
        if (112..144).contains(&x) && z < 177 {
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

fn column(seed: u64, x: i32, z: i32) -> Column {
    let n = fbm(seed, x, z, 32);
    // The edge ring of tall mountains.
    let edge = x.min(z).min(WORLD[0] - 1 - x).min(WORLD[2] - 1 - z);
    if edge < EDGE {
        let h = 62 + (14.0 * n) as i32 + (EDGE - edge) * 2;
        let mut c = Column::rock(h.min(WORLD[1] - 4), mat::CLIFFSTONE);
        if c.top > 70 {
            c.surf = mat::SNOW;
        }
        return c;
    }
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
        return Column::rock(50 + (4.0 * n) as i32, mat::CLIFFSTONE);
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
        return Column::rock(58 + (3.0 * n) as i32, mat::CLIFFSTONE);
    }
    // The ridge between the Frozen Fields and the Marsh, with the pass.
    if (RIDGE_X.0..RIDGE_X.1).contains(&x) {
        if (PASS_Z.0..PASS_Z.1).contains(&z) {
            let t = (x - RIDGE_X.0) as f32 / (RIDGE_X.1 - RIDGE_X.0) as f32;
            return Column::plain(lerp(42.0, 33.0, t) as i32, mat::SNOW, mat::DIRT);
        }
        return Column::rock(60 + (4.0 * n) as i32, mat::CLIFFSTONE);
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

    // P1: the workshop hut of planks and wood, burning at the start.
    let s = stamp::WORKSHOP;
    stamp_box(w, &s, |[x, y, z]| {
        let edge_x = x == s.min[0] || x == s.max[0] - 1;
        let edge_z = z == s.min[2] || z == s.max[2] - 1;
        match y {
            34 => c(mat::PLANKS),
            35..=38 if edge_x && edge_z => c(mat::WOOD),
            // The door on the south side, a window on the west.
            35..=37 if z == s.max[2] - 1 && (141..=142).contains(&x) => a,
            36..=37 if x == s.min[0] && (189..=190).contains(&z) => a,
            35..=38 if edge_x || edge_z => c(mat::PLANKS),
            39 if x > s.min[0] && x < s.max[0] - 1 && z > s.min[2] && z < s.max[2] - 1 => {
                let mut p = c(mat::PLANKS);
                p.set_flag(Cell::BURNING, (x + z) % 3 == 0);
                p
            }
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

    // P2b: the ice gate fills the opening in the cliff band.
    let s = stamp::ICE_GATE;
    stamp_box(w, &s, |[_, y, _]| match y {
        ..=35 => c(mat::STONE),
        _ => c(mat::ICE),
    });

    // The coal seam behind the ice gate: stone with coal ore, the top of
    // the seam open to the sky.
    let s = stamp::COAL_SEAM;
    stamp_box(w, &s, |[x, y, z]| {
        let h = hash3(0xc0a1, x, y, z);
        if unit(h) < 0.28 || (y == s.max[1] - 1 && unit(h) < 0.4) {
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
        let boardwalk = (100..=101).contains(&z) && !wall;
        match y {
            26 => c(mat::STONE),
            27 if HOLLOW_VENTS.contains(&[x, y, z]) => c(mat::GAS_VENT),
            27 => c(mat::MUD),
            _ if gap => a,
            28 if boardwalk => c(mat::PLANKS),
            28..=29 if wall => c(mat::CLIFFSTONE),
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
    // The hollow's gap drains east along a trench into a low basin, so CO2
    // poured into the hollow spills out while the gap is open.
    let [gx, _, gz] = HOLLOW_GAP;
    fill(w, [gx + 1, 27, gz], [gx + 10, 28, gz + 1], c(mat::MUD));
    fill(w, [gx + 1, 28, gz], [gx + 10, 34, gz + 1], a);
    fill(w, [gx + 10, 26, gz - 5], [gx + 18, 27, gz + 6], c(mat::MUD));
    fill(w, [gx + 10, 27, gz - 5], [gx + 18, 34, gz + 6], a);
    // The cave mouth opens to the sky.
    fill(
        w,
        [CAVE_MOUTH[0] - 2, CAVE_MOUTH[1] - 2, CAVE_MOUTH[2] - 2],
        [CAVE_MOUTH[0] + 3, CAVE_MOUTH[1] + 4, CAVE_MOUTH[2] + 3],
        a,
    );
}

/// The six gas vents on the hollow's floor.
pub const HOLLOW_VENTS: [[i32; 3]; 6] = [
    [188, 27, 98],
    [191, 27, 98],
    [194, 27, 98],
    [188, 27, 103],
    [191, 27, 103],
    [194, 27, 103],
];
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
        assert_eq!(biome_at([60, 40, 100]), Biome::Frozen);
        assert_eq!(biome_at([200, 31, 100]), Biome::Marsh);
        assert_eq!(biome_at([220, 12, 40]), Biome::Cave);
    }
}
