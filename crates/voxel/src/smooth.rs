//! The smooth mesher (the "smooth open world" look): a Surface Nets
//! isosurface over a density field made from the cells, so the block world
//! draws as sculpted ground instead of cubes. The cells (and the chemistry
//! that runs on them) are unchanged underneath; only the drawing and the
//! walking surface are smooth.
//!
//! **The field.** One value per cell, at the cell's centre: 1 for an opaque
//! solid, 0 for open space, except that an open cell standing on natural
//! ground ("soft": dirt, grass, sand, stone, snow...) beside more natural
//! ground gets a partial *fill* value. Iso 0.5 then runs a ramp from the
//! foot of a one-block terrace step up to its top, so terraces read as
//! slopes; a dug pit (open on top, ground on 3 or 4 sides) gets a low fill
//! and stays a pit. Built materials (brick, planks, runestone...) get no
//! fill, so a brick step stays a step.
//!
//! **Vertices.** One per dual cell (the cube between 8 cell centres) that
//! the surface crosses: the mean of the edge crossings (round), blended
//! towards an axis-wise placement (each coordinate from the crossings on
//! edges along that axis, which reproduces block corners) by a sharpness:
//! 0 for foliage, wood and ramps (organic), 1 for inside corners with no
//! fill (a floor meets a wall exactly), 0.5-0.55 for outside corners (a
//! soft bevel at most 0.125 m deep). Flat ground is exactly at the block
//! tops, so standing heights do not change.
//!
//! **Seams.** A chunk owns the lattice edges whose lower end lies in it;
//! every vertex depends only on the cells around it, so neighbouring
//! chunks agree exactly. Normals add up the face normals of every quad
//! around a vertex, including the ring of quads owned by the neighbours,
//! so shading is continuous across chunks too.
//!
//! The same surface serves walking (`ground`): the client stands the
//! player on what is drawn.
//!
//! Plain Rust, no Bevy types.

use crate::materials::{flags, Phase};
use crate::mesh::MeshData;
use crate::{mat, World, CHUNK};

/// The iso level.
pub const ISO: f32 = 0.5;
/// How far (in cells) a change reaches into neighbouring meshes: a cell
/// changes the fill of its neighbours, which moves the vertices of the
/// dual cells around them and the normals one ring further.
pub const MARGIN: i32 = 3;

/// How a material takes part in the smooth surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Air, gases other than steam.
    Empty,
    /// Natural ground: ramps and soft bevels.
    Soft,
    /// Built or gameplay blocks: no ramps, bevelled edges.
    Hard,
    /// Leaves and wood: round, organic shapes.
    Organic,
    /// Ice, glass, crystal, steam: the translucent pass.
    Clear,
    /// Water: the water pass (a surface at its fill level).
    Liquid,
}

/// The kind of material `m`.
pub fn kind(m: u8) -> Kind {
    match m {
        mat::AIR => Kind::Empty,
        mat::DIRT
        | mat::GRASS
        | mat::SAND
        | mat::STONE
        | mat::CLIFFSTONE
        | mat::SNOW
        | mat::MUD
        | mat::COAL_ORE
        | mat::IRON_ORE
        | mat::ASH => Kind::Soft,
        mat::LEAVES | mat::WOOD => Kind::Organic,
        mat::ICE | mat::GLASS | mat::CRYSTAL | mat::STEAM => Kind::Clear,
        mat::TORCH => Kind::Hard,
        _ => {
            let f = flags(m);
            match f.phase {
                Phase::Liquid => Kind::Liquid,
                _ if f.solid && f.transparent => Kind::Clear,
                _ if f.solid => Kind::Hard,
                _ => Kind::Empty,
            }
        }
    }
}

/// True for the kinds drawn in the opaque pass.
fn opaque(k: Kind) -> bool {
    matches!(k, Kind::Soft | Kind::Hard | Kind::Organic)
}

/// The smooth look's palette: base colour (linear RGB), how much the
/// colour stays on flat tops and gives way to rock on slopes (`grassy`),
/// and how strongly it glows (`glow`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub rgb: [f32; 3],
    pub grassy: f32,
    pub glow: f32,
}

const fn look(r: f32, g: f32, b: f32, grassy: f32, glow: f32) -> Look {
    Look {
        rgb: [r, g, b],
        grassy,
        glow,
    }
}

/// One `Look` per material id (stylised: soft greens, warm clay, cool
/// violet stone, teal glow for runes and crystals).
pub const LOOKS: [Look; mat::COUNT] = [
    look(0.0, 0.0, 0.0, 0.0, 0.0),     // air
    look(0.30, 0.19, 0.11, 0.0, 0.0),  // dirt
    look(0.17, 0.45, 0.16, 1.0, 0.0),  // grass
    look(0.80, 0.66, 0.43, 0.25, 0.0), // sand
    look(0.36, 0.37, 0.41, 0.0, 0.0),  // stone
    look(0.30, 0.28, 0.36, 0.0, 0.0),  // cliffstone
    look(0.16, 0.22, 0.36, 0.0, 0.55), // runestone
    look(0.36, 0.21, 0.11, 0.0, 0.0),  // wood
    look(0.13, 0.40, 0.17, 0.6, 0.0),  // leaves
    look(0.58, 0.40, 0.21, 0.0, 0.0),  // planks
    look(0.88, 0.93, 1.00, 1.0, 0.0),  // snow
    look(0.55, 0.82, 1.00, 0.0, 0.10), // ice
    look(0.10, 0.38, 0.62, 0.0, 0.0),  // water
    look(0.95, 0.96, 0.98, 0.0, 0.0),  // steam
    look(0.21, 0.15, 0.09, 0.3, 0.0),  // mud
    look(0.13, 0.13, 0.15, 0.0, 0.0),  // coal_ore
    look(0.50, 0.33, 0.25, 0.0, 0.0),  // iron_ore
    look(0.35, 0.90, 0.95, 0.0, 1.0),  // crystal
    look(0.80, 0.92, 1.00, 0.0, 0.0),  // glass
    look(0.60, 0.62, 0.67, 0.0, 0.0),  // iron
    look(0.64, 0.30, 0.17, 0.0, 0.0),  // brick
    look(0.32, 0.31, 0.31, 0.0, 0.0),  // ash
    look(1.00, 0.72, 0.30, 0.0, 1.0),  // torch
    look(0.55, 0.38, 0.62, 0.0, 0.25), // bench
    look(0.86, 0.66, 0.22, 0.0, 0.35), // invention_bench
    look(0.40, 0.14, 0.08, 0.0, 0.8),  // kiln_hearth
    look(0.38, 0.30, 0.18, 0.0, 0.0),  // gas_vent
];

/// The look of material `id` (magenta past the table, so a missing row
/// shows).
pub fn look_of(id: u8) -> Look {
    LOOKS
        .get(id as usize)
        .copied()
        .unwrap_or(look(1.0, 0.0, 1.0, 0.0, 0.0))
}

/// A chunk's three smooth passes: `opaque` (ground, rock, buildings,
/// trees), `clear` (ice, glass, crystal, steam) and `water` (the surface of
/// liquids at their level, plus any side open to air).
///
/// Vertex streams: `colors` is the material's colour with the AO light in
/// alpha, `uvs` is `[glow, grassy]`, `mats` the dominant material and
/// `light` the AO light.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmoothMesh {
    pub opaque: MeshData,
    pub clear: MeshData,
    pub water: MeshData,
}

impl SmoothMesh {
    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.clear.is_empty() && self.water.is_empty()
    }
}

// ---------------------------------------------------------------------------
// The padded grid of cells a region reads.
// ---------------------------------------------------------------------------

struct Grid {
    lo: [i32; 3],
    n: [i32; 3],
    mat: Vec<u8>,
    level: Vec<u8>,
    kinds: Vec<Kind>,
}

impl Grid {
    fn read(world: &World, lo: [i32; 3], hi: [i32; 3]) -> Grid {
        let n = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let len = (n[0] * n[1] * n[2]) as usize;
        let mut g = Grid {
            lo,
            n,
            mat: vec![mat::AIR; len],
            level: vec![0; len],
            kinds: Vec::new(),
        };
        let mut i = 0;
        for z in lo[2]..hi[2] {
            for y in lo[1]..hi[1] {
                for x in lo[0]..hi[0] {
                    if y < 0 {
                        // Below the world counts as solid rock.
                        g.mat[i] = mat::STONE;
                    } else {
                        let c = world.get([x, y, z]);
                        g.mat[i] = c.mat;
                        g.level[i] = c.level();
                    }
                    i += 1;
                }
            }
        }
        // One kind per material id, then per cell.
        let mut by_id = [Kind::Empty; 256];
        for (id, k) in by_id.iter_mut().enumerate() {
            *k = kind(id as u8);
        }
        g.kinds = g.mat.iter().map(|&m| by_id[m as usize]).collect();
        g
    }

    fn has(&self, k: Kind) -> bool {
        self.kinds.contains(&k)
    }

    fn idx(&self, p: [i32; 3]) -> Option<usize> {
        let l = [p[0] - self.lo[0], p[1] - self.lo[1], p[2] - self.lo[2]];
        if (0..3).all(|a| l[a] >= 0 && l[a] < self.n[a]) {
            Some((l[0] + self.n[0] * (l[1] + self.n[1] * l[2])) as usize)
        } else {
            None
        }
    }

    fn mat(&self, p: [i32; 3]) -> u8 {
        match self.idx(p) {
            Some(i) => self.mat[i],
            None if p[1] < 0 => mat::STONE,
            None => mat::AIR,
        }
    }

    fn level(&self, p: [i32; 3]) -> u8 {
        self.idx(p).map_or(0, |i| self.level[i])
    }

    fn kind(&self, p: [i32; 3]) -> Kind {
        match self.idx(p) {
            Some(i) => self.kinds[i],
            None if p[1] < 0 => Kind::Soft,
            None => Kind::Empty,
        }
    }
}

/// Which surface a pass draws.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Opaque,
    Clear,
}

/// The field value at cell `p` for `pass`.
fn value(g: &Grid, p: [i32; 3], pass: Pass) -> f32 {
    let k = g.kind(p);
    match pass {
        Pass::Clear => {
            if k == Kind::Clear || opaque(k) {
                1.0
            } else {
                0.0
            }
        }
        Pass::Opaque => {
            if opaque(k) {
                return 1.0;
            }
            if k == Kind::Clear {
                return 0.0;
            }
            fill(g, p)
        }
    }
}

/// The ramp fill of an open cell `p`: only on natural ground, beside
/// natural ground.
fn fill(g: &Grid, p: [i32; 3]) -> f32 {
    let [x, y, z] = p;
    if g.kind([x, y - 1, z]) != Kind::Soft {
        return 0.0;
    }
    let soft = |dx: i32, dz: i32| g.kind([x + dx, y, z + dz]) == Kind::Soft;
    fill_from(
        [soft(1, 0), soft(-1, 0), soft(0, 1), soft(0, -1)],
        [soft(1, 1), soft(1, -1), soft(-1, 1), soft(-1, -1)],
    )
}

/// The fill of an open cell on natural ground from which of its four side
/// neighbours (east, west, south, north) and four diagonal neighbours are
/// natural ground.
fn fill_from(side: [bool; 4], diag: [bool; 4]) -> f32 {
    let [e, w, s, n] = side;
    let n4 = side.iter().filter(|&&b| b).count();
    let nd = diag.iter().filter(|&&b| b).count() as f32;
    match n4 {
        // An outside corner: a touch of rounding, never a ramp.
        0 => 0.1 * nd,
        // The foot of a straight step: a ramp.
        1 => 0.58 + 0.04 * nd.min(2.0),
        // An inside corner (a ramp both ways), or a one-wide trench.
        2 if (e && w) || (s && n) => 0.42,
        2 => 0.72,
        // A notch or a pit: it stays open.
        3 => 0.4,
        _ => 0.25,
    }
}

/// The 8 corners of a dual cell, as offsets (bit 0 x, bit 1 y, bit 2 z).
const CORNERS: [[i32; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];

/// The 12 edges of a dual cell: corner pairs and the axis they run along.
const EDGES: [(usize, usize, usize); 12] = [
    (0, 1, 0),
    (2, 3, 0),
    (4, 5, 0),
    (6, 7, 0),
    (0, 2, 1),
    (1, 3, 1),
    (4, 6, 1),
    (5, 7, 1),
    (0, 4, 2),
    (1, 5, 2),
    (2, 6, 2),
    (3, 7, 2),
];

/// The field over a region's lattice, and the vertices of its dual cells.
struct Net {
    /// Lattice origin and size (lattice point = cell; its position is the
    /// cell's centre).
    lo: [i32; 3],
    n: [i32; 3],
    val: Vec<f32>,
    /// Vertex index per dual cell (dual cell q has corners q..q+1), or
    /// `u32::MAX` for none. Dual cells span `lo .. lo + n - 1`.
    vert: Vec<u32>,
    pos: Vec<[f32; 3]>,
    mat: Vec<u8>,
    light: Vec<f32>,
    nrm: Vec<[f32; 3]>,
}

impl Net {
    fn li(&self, p: [i32; 3]) -> usize {
        let l = [p[0] - self.lo[0], p[1] - self.lo[1], p[2] - self.lo[2]];
        (l[0] + self.n[0] * (l[1] + self.n[1] * l[2])) as usize
    }

    fn v(&self, p: [i32; 3]) -> f32 {
        self.val[self.li(p)]
    }

    fn dual(&self, q: [i32; 3]) -> u32 {
        let l = [q[0] - self.lo[0], q[1] - self.lo[1], q[2] - self.lo[2]];
        if (0..3).all(|a| l[a] >= 0 && l[a] < self.n[a] - 1) {
            self.vert[(l[0] + (self.n[0] - 1) * (l[1] + (self.n[1] - 1) * l[2])) as usize]
        } else {
            u32::MAX
        }
    }
}

fn build_net(g: &Grid, lo: [i32; 3], hi: [i32; 3], pass: Pass) -> Net {
    let n = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let mut val = Vec::with_capacity((n[0] * n[1] * n[2]) as usize);
    // The lattice lies inside the grid with a ring to spare, so the fill's
    // neighbours are plain index offsets.
    let (sy, sz) = (g.n[0] as usize, (g.n[0] * g.n[1]) as usize);
    let k = &g.kinds;
    let soft = |i: usize| k[i] == Kind::Soft;
    for z in lo[2]..hi[2] {
        for y in lo[1]..hi[1] {
            let row = g.idx([lo[0], y, z]).expect("lattice inside the grid");
            for (dx, _) in (lo[0]..hi[0]).enumerate() {
                let i = row + dx;
                let kd = k[i];
                let v = match pass {
                    Pass::Clear => f32::from(u8::from(kd == Kind::Clear || opaque(kd))),
                    Pass::Opaque if opaque(kd) => 1.0,
                    Pass::Opaque if kd == Kind::Clear || !soft(i - sy) => 0.0,
                    Pass::Opaque => fill_from(
                        [soft(i + 1), soft(i - 1), soft(i + sz), soft(i - sz)],
                        [
                            soft(i + 1 + sz),
                            soft(i + 1 - sz),
                            soft(i - 1 + sz),
                            soft(i - 1 - sz),
                        ],
                    ),
                };
                val.push(v);
            }
        }
    }
    let dn = [n[0] - 1, n[1] - 1, n[2] - 1];
    let mut net = Net {
        lo,
        n,
        val,
        vert: vec![u32::MAX; (dn[0] * dn[1] * dn[2]).max(0) as usize],
        pos: Vec::new(),
        mat: Vec::new(),
        light: Vec::new(),
        nrm: Vec::new(),
    };
    let (ly, lz) = (n[0] as usize, (n[0] * n[1]) as usize);
    let offs = CORNERS.map(|c| c[0] as usize + c[1] as usize * ly + c[2] as usize * lz);
    let mut di = 0usize;
    for z in lo[2]..hi[2] - 1 {
        for y in lo[1]..hi[1] - 1 {
            let row = net.li([lo[0], y, z]);
            for (dx, x) in (lo[0]..hi[0] - 1).enumerate() {
                let base = row + dx;
                let mut v = [0f32; 8];
                let mut inside = 0u8;
                for c in 0..8 {
                    v[c] = net.val[base + offs[c]];
                    inside += u8::from(v[c] >= ISO);
                }
                if inside == 0 || inside == 8 {
                    di += 1;
                    continue;
                }
                let (p, m, l) = place(g, [x, y, z], &v, inside, pass);
                net.vert[di] = net.pos.len() as u32;
                net.pos.push(p);
                net.mat.push(m);
                net.light.push(l);
                net.nrm.push([0.0; 3]);
                di += 1;
            }
        }
    }
    net
}

/// The vertex of dual cell `q` (corner values `v`, `inside` of them at or
/// above the iso level): its position, material and AO light.
fn place(g: &Grid, q: [i32; 3], v: &[f32; 8], inside: u8, pass: Pass) -> ([f32; 3], u8, f32) {
    let mut sum = [0f32; 3];
    let mut count = 0f32;
    let mut axis_sum = [0f32; 3];
    let mut axis_n = [0f32; 3];
    for &(a, b, ax) in &EDGES {
        let (va, vb) = (v[a], v[b]);
        if (va >= ISO) == (vb >= ISO) {
            continue;
        }
        let t = (ISO - va) / (vb - va);
        let ca = CORNERS[a];
        let mut p = [
            q[0] as f32 + ca[0] as f32 + 0.5,
            q[1] as f32 + ca[1] as f32 + 0.5,
            q[2] as f32 + ca[2] as f32 + 0.5,
        ];
        p[ax] += t;
        for k in 0..3 {
            sum[k] += p[k];
        }
        count += 1.0;
        axis_sum[ax] += p[ax];
        axis_n[ax] += 1.0;
    }
    let mean = sum.map(|s| s / count);
    // What the dual cell is made of.
    let mut organic = false;
    let mut hard = false;
    let mut filled = false;
    let mut best: Option<(u8, i32)> = None;
    let mut occ = 0.0;
    for (k, c) in CORNERS.iter().enumerate() {
        let p = [q[0] + c[0], q[1] + c[1], q[2] + c[2]];
        let kd = g.kind(p);
        organic |= kd == Kind::Organic;
        hard |= kd == Kind::Hard;
        if v[k] > 0.0 && v[k] < 1.0 {
            filled = true;
        }
        occ += v[k];
        let counts = match pass {
            Pass::Opaque => opaque(kd),
            Pass::Clear => kd == Kind::Clear,
        };
        if counts {
            // The top-most cell wins (grass over dirt), then the first.
            let score = c[1];
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((g.mat(p), score));
            }
        }
    }
    let sharp = if organic || (filled && pass == Pass::Opaque) {
        0.0
    } else if inside >= 5 {
        // An inside corner: exact, so floors meet walls at block height.
        1.0
    } else if hard {
        0.5
    } else {
        0.55
    };
    let mut p = mean;
    for k in 0..3 {
        if axis_n[k] > 0.0 {
            let a = axis_sum[k] / axis_n[k];
            p[k] = mean[k] + sharp * (a - mean[k]);
        }
    }
    // A cheap ambient occlusion: how much of the 8 corners is inside, plus
    // the cells just beyond them.
    let mut wide = 0.0;
    for d in [
        [-1, 0, 0],
        [2, 0, 0],
        [0, -1, 0],
        [0, 2, 0],
        [0, 0, -1],
        [0, 0, 2],
    ] {
        let base = [
            q[0] + d[0].clamp(0, 1),
            q[1] + d[1].clamp(0, 1),
            q[2] + d[2].clamp(0, 1),
        ];
        let off = [d[0].signum(), d[1].signum(), d[2].signum()];
        let probe = [
            base[0] + if d[0] != 0 { off[0] } else { 0 },
            base[1] + if d[1] != 0 { off[1] } else { 0 },
            base[2] + if d[2] != 0 { off[2] } else { 0 },
        ];
        if opaque(g.kind(probe)) {
            wide += 1.0;
        }
    }
    let crowd = occ / 8.0 * 0.7 + wide / 6.0 * 0.3;
    let light = (1.25 - crowd * 1.1).clamp(0.45, 1.0);
    let m = best.map_or(mat::STONE, |b| b.0);
    (p, m, light)
}

/// Adds the quads of lattice edges with their lower end in `[elo, ehi)` to
/// `out`; with `accumulate` only, adds their face normals to the vertices.
fn quads(
    net: &mut Net,
    g: &Grid,
    pass: Pass,
    elo: [i32; 3],
    ehi: [i32; 3],
    emit: Option<&mut Emit>,
) {
    let mut emit = emit;
    for z in elo[2]..ehi[2] {
        for y in elo[1]..ehi[1] {
            for x in elo[0]..ehi[0] {
                let p = [x, y, z];
                let va = net.v(p);
                for axis in 0..3 {
                    let mut q = p;
                    q[axis] += 1;
                    let vb = net.v(q);
                    let (ia, ib) = (va >= ISO, vb >= ISO);
                    if ia == ib {
                        continue;
                    }
                    // The solid end decides whether this pass draws it.
                    let (solid, open) = if ia { (p, q) } else { (q, p) };
                    let draws = match pass {
                        Pass::Opaque => true,
                        Pass::Clear => g.kind(solid) == Kind::Clear && !opaque(g.kind(open)),
                    };
                    if !draws {
                        continue;
                    }
                    let (u, w) = ((axis + 1) % 3, (axis + 2) % 3);
                    let mut ids = [u32::MAX; 4];
                    for (k, (du, dw)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
                        let mut d = p;
                        d[u] -= 1 - du;
                        d[w] -= 1 - dw;
                        ids[k] = net.dual(d);
                    }
                    if ids.contains(&u32::MAX) {
                        continue;
                    }
                    // Outward (from solid to open) is +axis when the lower
                    // end is the solid one.
                    if !ia {
                        ids.swap(1, 3);
                    }
                    let pp = ids.map(|i| net.pos[i as usize]);
                    let n = quad_normal(&pp);
                    for &i in &ids {
                        let v = &mut net.nrm[i as usize];
                        for k in 0..3 {
                            v[k] += n[k];
                        }
                    }
                    if let Some(e) = emit.as_deref_mut() {
                        e.quads.push(ids);
                    }
                }
            }
        }
    }
}

struct Emit {
    quads: Vec<[u32; 4]>,
}

fn quad_normal(p: &[[f32; 3]; 4]) -> [f32; 3] {
    // Cross of the diagonals: twice the quad's area, pointing outward.
    let a = sub(p[2], p[0]);
    let b = sub(p[3], p[1]);
    cross(a, b)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 1e-12 {
        v.map(|c| c / l)
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Meshes one pass over the region whose lattice edges have their lower
/// end in `[lo, hi)`.
fn surface(g: &Grid, lo: [i32; 3], hi: [i32; 3], pass: Pass, normals: bool) -> MeshData {
    // Edges [lo-1, hi+1) for normals; their dual cells [lo-2, hi+1); their
    // lattice [lo-2, hi+2).
    let llo = lo.map(|v| v - 2);
    let lhi = hi.map(|v| v + 2);
    let mut net = build_net(g, llo, lhi, pass);
    let mut out = MeshData::default();
    if net.pos.is_empty() {
        return out;
    }
    let mut emit = Emit { quads: Vec::new() };
    quads(&mut net, g, pass, lo, hi, Some(&mut emit));
    if emit.quads.is_empty() {
        return out;
    }
    if normals {
        // The ring of edges around the region, for normals only.
        let elo = lo.map(|v| v - 1);
        let ehi = hi.map(|v| v + 1);
        for axis in 0..3 {
            for side in 0..2 {
                let mut a = elo;
                let mut b = ehi;
                if side == 0 {
                    b[axis] = lo[axis];
                } else {
                    a[axis] = hi[axis];
                }
                // Restrict the other axes so each ring edge counts once.
                a[..axis].copy_from_slice(&lo[..axis]);
                b[..axis].copy_from_slice(&hi[..axis]);
                quads(&mut net, g, pass, a, b, None);
            }
        }
    }
    // Compact the used vertices.
    let mut remap = vec![u32::MAX; net.pos.len()];
    for quad in &emit.quads {
        for &i in quad {
            if remap[i as usize] == u32::MAX {
                remap[i as usize] = out.positions.len() as u32;
                let i = i as usize;
                let m = net.mat[i];
                let lk = look_of(m);
                let l = net.light[i];
                out.positions.push(net.pos[i]);
                out.normals.push(normalize(net.nrm[i]));
                out.colors.push([lk.rgb[0], lk.rgb[1], lk.rgb[2], l]);
                out.uvs.push([lk.glow, lk.grassy]);
                out.mats.push(m);
                out.light.push(l);
            }
        }
    }
    for quad in &emit.quads {
        let r = quad.map(|i| remap[i as usize]);
        let p = r.map(|i| out.positions[i as usize]);
        // Split along the shorter diagonal.
        let d02 = dist2(p[0], p[2]);
        let d13 = dist2(p[1], p[3]);
        if d02 <= d13 {
            out.indices
                .extend_from_slice(&[r[0], r[1], r[2], r[0], r[2], r[3]]);
        } else {
            out.indices
                .extend_from_slice(&[r[0], r[1], r[3], r[1], r[2], r[3]]);
        }
    }
    out
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = sub(a, b);
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

/// The water pass over cells `[lo, hi)`: a surface at each liquid column's
/// level, its corners shared with the neighbouring liquid cells (so it is
/// smooth), and a side wherever a liquid cell is open to air.
fn water(g: &Grid, lo: [i32; 3], hi: [i32; 3]) -> MeshData {
    let mut out = MeshData::default();
    let liquid = |p: [i32; 3]| g.kind(p) == Kind::Liquid;
    // The surface height of liquid cell p with no liquid above.
    let top = |p: [i32; 3]| -> f32 {
        let l = f32::from(g.level(p).clamp(1, 8));
        p[1] as f32 + l / 8.0 - 0.1
    };
    let corner_h = |c: [i32; 3]| -> f32 {
        // Corner c (block corner coordinates) on the plane of cell y = c[1].
        let mut s = 0.0;
        let mut n = 0.0;
        for (dx, dz) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
            let p = [c[0] + dx, c[1], c[2] + dz];
            if liquid(p) && !liquid([p[0], p[1] + 1, p[2]]) {
                s += top(p);
                n += 1.0;
            }
        }
        if n > 0.0 {
            s / n
        } else {
            c[1] as f32 + 0.9
        }
    };
    let lk = look_of(mat::WATER);
    let push = |out: &mut MeshData, p: [[f32; 3]; 4], n: [f32; 3]| {
        let first = out.positions.len() as u32;
        for v in p {
            out.positions.push(v);
            out.normals.push(n);
            out.colors.push([lk.rgb[0], lk.rgb[1], lk.rgb[2], 1.0]);
            out.uvs.push([0.0, 0.0]);
            out.mats.push(mat::WATER);
            out.light.push(1.0);
        }
        out.indices
            .extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
    };
    for z in lo[2]..hi[2] {
        for y in lo[1]..hi[1] {
            for x in lo[0]..hi[0] {
                let p = [x, y, z];
                if !liquid(p) {
                    continue;
                }
                let covered = liquid([x, y + 1, z]);
                if !covered
                    && g.kind([x, y + 1, z]) != Kind::Clear
                    && !opaque(g.kind([x, y + 1, z]))
                {
                    let h = [
                        corner_h([x, y, z]),
                        corner_h([x, y, z + 1]),
                        corner_h([x + 1, y, z + 1]),
                        corner_h([x + 1, y, z]),
                    ];
                    let (fx, fz) = (x as f32, z as f32);
                    push(
                        &mut out,
                        [
                            [fx, h[0], fz],
                            [fx, h[1], fz + 1.0],
                            [fx + 1.0, h[2], fz + 1.0],
                            [fx + 1.0, h[3], fz],
                        ],
                        [0.0, 1.0, 0.0],
                    );
                }
                // Sides open to air (a dug bank, a spill).
                let ytop = if covered { y as f32 + 1.0 } else { top(p) };
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let q = [x + dx, y, z + dz];
                    if g.kind(q) != Kind::Empty {
                        continue;
                    }
                    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                    let (a, b) = match (dx, dz) {
                        (1, 0) => ([fx + 1.0, fz + 1.0], [fx + 1.0, fz]),
                        (-1, 0) => ([fx, fz], [fx, fz + 1.0]),
                        (0, 1) => ([fx, fz + 1.0], [fx + 1.0, fz + 1.0]),
                        _ => ([fx + 1.0, fz], [fx, fz]),
                    };
                    push(
                        &mut out,
                        [
                            [a[0], fy, a[1]],
                            [b[0], fy, b[1]],
                            [b[0], ytop, b[1]],
                            [a[0], ytop, a[1]],
                        ],
                        [dx as f32, 0.0, dz as f32],
                    );
                }
            }
        }
    }
    out
}

/// Meshes the cells whose lattice edges start in `[lo, hi)` (all three
/// passes).
pub fn mesh_region(world: &World, lo: [i32; 3], hi: [i32; 3]) -> SmoothMesh {
    let g = Grid::read(world, lo.map(|v| v - MARGIN), hi.map(|v| v + MARGIN));
    // A region of nothing but open space, or nothing but buried solid,
    // has no surface.
    let first = g.kinds[0];
    let uniform = g.kinds.iter().all(|&k| k == first);
    if uniform && matches!(first, Kind::Empty | Kind::Soft | Kind::Hard | Kind::Organic) {
        return SmoothMesh::default();
    }
    SmoothMesh {
        opaque: surface(&g, lo, hi, Pass::Opaque, true),
        clear: if g.has(Kind::Clear) {
            surface(&g, lo, hi, Pass::Clear, true)
        } else {
            MeshData::default()
        },
        water: if g.has(Kind::Liquid) {
            water(&g, lo, hi)
        } else {
            MeshData::default()
        },
    }
}

/// Meshes chunk `chunk` smoothly. Positions are in world blocks.
pub fn mesh_chunk_smooth(world: &World, chunk: usize) -> SmoothMesh {
    let o = World::chunk_origin(chunk);
    mesh_region(world, o, o.map(|v| v + CHUNK))
}

/// What is under a point, on the drawn opaque surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ground {
    /// The surface's height below the point.
    Floor(f32),
    /// The point is inside the ground (a wall or a ceiling is in the way).
    Inside,
    /// No surface within reach (a drop, or open sky to the bottom).
    None,
}

/// The drawn opaque surface under `(x, top, z)`, searching down to
/// `bottom`. This is the same surface the chunks draw, so a body stood on
/// it neither hovers nor sinks.
pub fn ground(world: &World, x: f32, z: f32, top: f32, bottom: f32) -> Ground {
    if ![x, z, top, bottom].iter().all(|v| v.is_finite()) || bottom >= top {
        return Ground::None;
    }
    let (cx, cz) = (x.floor() as i32, z.floor() as i32);
    let lo = [cx - 2, bottom.floor() as i32 - 2, cz - 2];
    let hi = [cx + 2, top.floor() as i32 + 2, cz + 2];
    let g = Grid::read(world, lo.map(|v| v - MARGIN), hi.map(|v| v + MARGIN));
    let m = surface(&g, lo, hi, Pass::Opaque, false);
    let mut best: Option<(f32, bool)> = None;
    for t in m.indices.as_chunks::<3>().0 {
        let [a, b, c] = t.map(|i| m.positions[i as usize]);
        let Some(y) = vertical_hit(a, b, c, x, z) else {
            continue;
        };
        if y > top || y < bottom {
            continue;
        }
        let n = cross(sub(b, a), sub(c, a));
        if best.is_none_or(|(by, _)| y > by) {
            best = Some((y, n[1] > 0.0));
        }
    }
    match best {
        Some((y, true)) => Ground::Floor(y),
        Some((_, false)) => Ground::Inside,
        None => {
            if inside_field(&g, [x, top, z]) {
                Ground::Inside
            } else {
                Ground::None
            }
        }
    }
}

/// The height of triangle abc above (x, z), if (x, z) is inside it.
fn vertical_hit(a: [f32; 3], b: [f32; 3], c: [f32; 3], x: f32, z: f32) -> Option<f32> {
    let d = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
    if d.abs() < 1e-9 {
        return None;
    }
    let l1 = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / d;
    let l2 = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / d;
    let l3 = 1.0 - l1 - l2;
    const E: f32 = -1e-5;
    if l1 < E || l2 < E || l3 < E {
        return None;
    }
    Some(l1 * a[1] + l2 * b[1] + l3 * c[1])
}

/// Trilinear field value at point `p` (world metres) is at or above iso.
fn inside_field(g: &Grid, p: [f32; 3]) -> bool {
    let b = p.map(|v| (v - 0.5).floor());
    let f = [p[0] - 0.5 - b[0], p[1] - 0.5 - b[1], p[2] - 0.5 - b[2]];
    let bi = b.map(|v| v as i32);
    let mut s = 0.0;
    for c in CORNERS {
        let w = (0..3)
            .map(|k| if c[k] == 1 { f[k] } else { 1.0 - f[k] })
            .product::<f32>();
        s += w * value(g, [bi[0] + c[0], bi[1] + c[1], bi[2] + c[2]], Pass::Opaque);
    }
    s >= ISO
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;

    fn flat(top: i32, m: u8) -> World {
        let mut w = World::new();
        for z in 0..64 {
            for x in 0..64 {
                for y in 0..=top {
                    w.set([x, y, z], Cell::new(m));
                }
            }
        }
        w
    }

    fn max_y(m: &MeshData) -> f32 {
        m.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max)
    }

    #[test]
    fn flat_ground_sits_exactly_on_the_block_tops() {
        let w = flat(34, mat::GRASS);
        let m = mesh_region(&w, [8, 24, 8], [24, 40, 24]);
        assert!(!m.opaque.is_empty());
        for (p, n) in m.opaque.positions.iter().zip(&m.opaque.normals) {
            assert_eq!(p[1], 35.0, "{p:?}");
            assert!(n[1] > 0.99, "{n:?}");
        }
        assert_eq!(ground(&w, 16.3, 16.7, 36.0, 30.0), Ground::Floor(35.0));
        assert!(m.clear.is_empty() && m.water.is_empty());
    }

    #[test]
    fn a_soft_terrace_step_becomes_a_ramp_and_a_brick_step_stays_a_step() {
        for (m, ramp) in [(mat::GRASS, true), (mat::BRICK, false)] {
            let mut w = flat(34, mat::GRASS);
            for z in 0..64 {
                for x in 32..64 {
                    w.set([x, 35, z], Cell::new(m));
                }
            }
            // Half a block before the step face (x 32), the drawn ground.
            let Ground::Floor(y) = ground(&w, 31.5, 20.5, 37.0, 30.0) else {
                panic!("no floor");
            };
            if ramp {
                assert!(y > 35.3, "the ramp rises before the step: {y}");
            } else {
                assert!(
                    (y - 35.0).abs() < 1e-4,
                    "the floor meets the brick flat: {y}"
                );
            }
            // On top of the step, a block in: exactly the top.
            assert_eq!(ground(&w, 33.5, 20.5, 38.0, 30.0), Ground::Floor(36.0));
            // The bevel at the top edge is never deeper than 0.13 m.
            let Ground::Floor(e) = ground(&w, 32.2, 20.5, 38.0, 30.0) else {
                panic!("no floor at the edge");
            };
            assert!(e > 35.87, "{m}: edge {e}");
        }
    }

    #[test]
    fn a_dug_hole_stays_a_hole() {
        let mut w = flat(34, mat::DIRT);
        w.set([20, 34, 20], Cell::AIR);
        let Ground::Floor(y) = ground(&w, 20.5, 20.5, 36.0, 30.0) else {
            panic!("no floor");
        };
        assert!(y < 34.6, "the hole's floor shows: {y}");
        // A two-deep hole shows deeper still.
        w.set([20, 33, 20], Cell::AIR);
        let Ground::Floor(y2) = ground(&w, 20.5, 20.5, 36.0, 30.0) else {
            panic!("no floor");
        };
        assert!(y2 < y - 0.5, "{y2} vs {y}");
    }

    #[test]
    fn walls_read_inside_and_drops_read_none() {
        let mut w = flat(34, mat::STONE);
        for y in 35..40 {
            w.set([20, y, 20], Cell::new(mat::STONE));
        }
        assert_eq!(ground(&w, 20.5, 20.5, 36.0, 30.0), Ground::Inside);
        assert_eq!(ground(&w, 100.5, 100.5, 36.0, 30.0), Ground::None);
    }

    #[test]
    fn chunk_seams_match() {
        // A hill across the chunk border at x 32: every vertex on the border
        // plane appears in both chunks' meshes, with the same normal.
        let mut w = flat(34, mat::GRASS);
        for z in 0..64 {
            for x in 0..64 {
                let (x, z): (i32, i32) = (x, z);
                let h = 35
                    + ((x - 32).abs().max((z - 32).abs()) < 6) as i32
                    + ((x - 32).abs().max((z - 32).abs()) < 3) as i32;
                for y in 35..h {
                    w.set([x, y, z], Cell::new(mat::GRASS));
                }
            }
        }
        let a = mesh_region(&w, [0, 32, 0], [32, 64, 64]).opaque;
        let b = mesh_region(&w, [32, 32, 0], [64, 64, 64]).opaque;
        let border = |m: &MeshData| {
            let mut v: Vec<([i32; 3], [i32; 3])> = m
                .positions
                .iter()
                .zip(&m.normals)
                .filter(|(p, _)| (p[0] - 32.0).abs() < 0.6 && (31.0..33.0).contains(&p[0]))
                .map(|(p, n)| {
                    (
                        p.map(|c| (c * 1000.0).round() as i32),
                        n.map(|c| (c * 1000.0).round() as i32),
                    )
                })
                .collect();
            v.sort();
            v.dedup();
            v
        };
        let (ba, bb) = (border(&a), border(&b));
        // Vertices of dual cells straddling x = 32 are shared.
        let shared: Vec<_> = ba.iter().filter(|e| bb.contains(e)).collect();
        assert!(shared.len() > 10, "{} shared", shared.len());
        for e in &ba {
            if let Some(f) = bb.iter().find(|f| f.0 == e.0) {
                assert_eq!(e.1, f.1, "normal mismatch at {:?}", e.0);
            }
        }
        assert!(max_y(&a) > 36.5 && max_y(&b) > 36.5);
    }

    #[test]
    fn water_draws_at_its_level_and_ice_in_the_clear_pass() {
        let mut w = flat(30, mat::SAND);
        for z in 10..20 {
            for x in 10..20 {
                let mut c = Cell::new(mat::WATER);
                c.set_level(8);
                w.set([x, 30, z], c);
            }
        }
        w.set([25, 31, 25], Cell::new(mat::ICE));
        let m = mesh_region(&w, [0, 16, 0], [32, 48, 32]);
        assert!(!m.water.is_empty());
        let top = max_y(&m.water);
        assert!((30.85..=30.95).contains(&top), "water top {top}");
        assert!(!m.clear.is_empty(), "ice is drawn");
        assert!(m.clear.mats.iter().all(|&k| k == mat::ICE));
    }

    #[test]
    fn a_village_chunk_meshes_quickly_and_has_no_cubes() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let c = World::chunk_index([128, 40, 190]).unwrap();
        let t = std::time::Instant::now();
        let m = mesh_chunk_smooth(&w, c);
        let ms = t.elapsed().as_secs_f32() * 1000.0;
        assert!(!m.opaque.is_empty());
        // Debug builds are slow; the release budget is checked by the bench.
        assert!(ms < 400.0, "{ms} ms");
        // Some normals are neither axis-aligned: the surface is smooth.
        let slanted = m
            .opaque
            .normals
            .iter()
            .filter(|n| n.iter().all(|c| c.abs() < 0.97))
            .count();
        assert!(slanted * 10 > m.opaque.normals.len(), "{slanted}");
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// `cargo test --release --lib smooth::bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn mesh_every_chunk() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let mut worst = 0f32;
        let mut total = 0f32;
        let mut tris = 0;
        for c in 0..w.chunk_count() {
            let t = std::time::Instant::now();
            let m = mesh_chunk_smooth(&w, c);
            let ms = t.elapsed().as_secs_f32() * 1000.0;
            worst = worst.max(ms);
            total += ms;
            tris += (m.opaque.indices.len() + m.clear.indices.len() + m.water.indices.len()) / 3;
        }
        println!("smooth: all chunks {total:.0} ms, worst {worst:.2} ms, {tris} triangles");
    }
}
