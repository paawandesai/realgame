//! Generated models over built cells (docs/art-bible.md, "Built things"):
//! the workshop, the Village houses, the Formula Bench and the field Bench
//! shrines, runestone gates and doors, the electrolysis rig, the cache
//! towers and the Invention Bench.
//!
//! The cells stay what they are underneath (brick, planks, runestone,
//! iron...): collision, digging, fire and the chemistry read them as
//! before. The smooth surface leaves their faces out (`smooth`), and this
//! module draws them instead:
//!
//! - **Masonry.** Built cells join into one solid with rounded, bevelled
//!   edges (a quarter round of `BEVEL`): a wall of bricks is one thick,
//!   soft-edged wall, a tower one pillar, and a dug cell a bevelled hole.
//!   Faces against the ground or another built cell are not drawn, so the
//!   ground meets the wall cleanly. Each block varies its colour a little,
//!   so masonry reads without textures.
//! - **Runestone** carries teal rune bands and glyphs (the only glow).
//! - **Benches** are models of their own: the Formula Bench a stone
//!   lectern with a teal screen, the Invention Bench a timber worktable
//!   with a crucible and a glowing flask.
//! - **Houses** (`gen::HOUSES`) get a pitched timber roof over their roof
//!   cells, one slice per metre: a slice whose planks burn or are dug goes,
//!   and what is left of it shows as charred blocks.
//! - **Fire leaves marks.** Built cells next to burning cells or ash are
//!   sooted dark, and the workshop's walls and posts are charred.
//!
//! Like `props`, everything is a pure function of the cells and their
//! positions: neighbouring chunks agree and a remesh gives the same model.

use crate::mesh::MeshData;
use crate::smooth::{kind, look_of, Kind};
use crate::{gen, mat, stamp, Cell, Chunk, World, CHUNK};

/// The radius of the rounded edge on built blocks (m): chunky, soft clay
/// and stone (art bible). Built blocks are walked as boxes, so only a foot
/// right at a top edge can look to stand a little proud.
pub const BEVEL: f32 = 0.3;
/// Arc segments per rounded edge.
const SEG: usize = 3;

/// Science teal (#40FFD9) in linear RGB.
const TEAL: V3 = [0.05, 1.0, 0.69];
/// Timber (#8A5A3B), clay (#D99A6C) and rock (#8C8079), linear.
const TIMBER: V3 = [0.254, 0.102, 0.044];
const CLAY: V3 = [0.69, 0.32, 0.15];
const ROCK: V3 = [0.262, 0.216, 0.191];
const CHAR: V3 = [0.07, 0.055, 0.045];

/// How a house's roof is covered.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Roof {
    /// Thick golden straw with a rolled eave.
    Thatch,
    /// Fired clay tiles in close courses.
    Tile,
    /// Timber shingles.
    Shingle,
}

/// One Village house's look (B4 round 3: the four houses read as one
/// family, not four copies): plaster tone, roof covering, how far the roof
/// overhangs and how thick it is, a chimney, the shutters' paint, and a
/// porch canopy or flower boxes. Looks only; the cells are the same.
#[derive(Clone, Copy, Debug)]
struct HouseStyle {
    wall: V3,
    roof: Roof,
    roof_rgb: V3,
    over: f32,
    th: f32,
    /// The chimney's slice (x offset from `min[0]`) and side (-1 north, 1
    /// south), if any.
    chimney: Option<(i32, f32)>,
    shutter: V3,
    porch: bool,
    flowers: bool,
}

/// Plaster: cream (#E9D3B0), clay (#D99A6C) and ochre (#D9AE6A), linear.
const CREAM: V3 = [0.815, 0.651, 0.431];
const OCHRE: V3 = [0.693, 0.423, 0.144];

fn house_style(h: &gen::House) -> HouseStyle {
    let i = gen::HOUSES.iter().position(|q| q == h).unwrap_or(0);
    // Neighbours differ: houses 0 and 3 stand together west of the plaza,
    // 1 and 2 east of it.
    match i % 4 {
        0 => HouseStyle {
            wall: CREAM,
            roof: Roof::Thatch,
            roof_rgb: [0.50, 0.31, 0.075],
            over: 0.6,
            th: 0.34,
            chimney: None,
            shutter: [0.06, 0.11, 0.16],
            porch: false,
            flowers: true,
        },
        1 => HouseStyle {
            wall: OCHRE,
            roof: Roof::Shingle,
            roof_rgb: mix([0.36, 0.12, 0.06], TIMBER, 0.4),
            over: 0.45,
            th: 0.18,
            chimney: Some((4, -1.0)),
            shutter: [0.20, 0.06, 0.04],
            porch: false,
            flowers: true,
        },
        2 => HouseStyle {
            wall: mix(CREAM, CLAY, 0.45),
            roof: Roof::Thatch,
            roof_rgb: [0.44, 0.27, 0.07],
            over: 0.65,
            th: 0.36,
            chimney: Some((4, 1.0)),
            shutter: [0.06, 0.11, 0.16],
            porch: true,
            flowers: false,
        },
        _ => HouseStyle {
            wall: CLAY,
            roof: Roof::Tile,
            roof_rgb: [0.42, 0.085, 0.04],
            over: 0.45,
            th: 0.2,
            chimney: Some((1, 1.0)),
            shutter: mul(TIMBER, 0.9),
            porch: true,
            flowers: false,
        },
    }
}

/// Materials drawn as built models.
pub fn is_built(m: u8) -> bool {
    matches!(
        m,
        mat::BRICK
            | mat::PLANKS
            | mat::RUNESTONE
            | mat::BENCH
            | mat::INVENTION_BENCH
            | mat::IRON
            | mat::KILN_HEARTH
    )
}

/// Built cells drawn as free-standing models (the benches) that do not
/// fill their cell: the smooth surface treats them as open space, so the
/// ground under them is drawn.
pub fn free_standing(m: u8) -> bool {
    matches!(m, mat::BENCH | mat::INVENTION_BENCH)
}

/// Wood that is a built timber (a house's or the workshop's posts), not a
/// tree: drawn as a squared, bevelled post. The workshop's burning fuel
/// store stays a pile of logs.
pub fn timber(p: [i32; 3], m: u8) -> bool {
    if m != mat::WOOD {
        return false;
    }
    let (s0, s1) = gen::WORKSHOP_STORE;
    let store = (0..3).all(|a| p[a] >= s0[a] && p[a] < s1[a]);
    (stamp::WORKSHOP.contains(p) && !store) || gen::HOUSES.iter().any(|h| h.contains(p))
}

/// The Kiln's brick shell above its terrace: the clay dome and chimney
/// models (the client's `kiln_dome`) are its drawing, so these cells draw
/// nothing here.
fn under_dome(p: [i32; 3], m: u8) -> bool {
    m == mat::BRICK && p[1] > gen::TERRACE_Y && stamp::KILN.contains(p)
}

/// The ice gate's plug (`stamp::ICE_GATE`): drawn here as thick, faceted
/// ice set in a stone frame instead of the clear pass's sheet. The smooth
/// surface treats these cells as open space (the cliff draws round them).
pub fn ice_plug(p: [i32; 3], m: u8) -> bool {
    m == mat::ICE && stamp::ICE_GATE.contains(p)
}

/// The electrolysis rig's glass cell between its iron terminals: the
/// client's rig stand (basin, tubes) is its drawing; the clear pass's grey
/// glass box read as a stray beam between the terminals (B4 round 3).
pub fn rig_glass(p: [i32; 3], m: u8) -> bool {
    m == mat::GLASS && stamp::RIG.contains(p)
}

/// The Kiln's runestone door cells: the client draws the arched timber
/// door over them (`voxel_view::kiln_door`), which swings open with them.
fn kiln_door(p: [i32; 3], m: u8) -> bool {
    m == mat::RUNESTONE && stamp::KILN.contains(p)
}

/// Whether the smooth surface leaves cell `p` (material `m`) to this
/// module's models.
pub fn modelled(p: [i32; 3], m: u8) -> bool {
    is_built(m) || timber(p, m)
}

/// Whether a cell counts as solid against a built face (the face is not
/// drawn and its edges are not rounded): any opaque solid that the smooth
/// surface or these models draw, but not trees' models, glass, ice,
/// water or a torch.
fn solid(world: &World, q: [i32; 3]) -> bool {
    let m = world.get(q).mat;
    if modelled(q, m) {
        return true;
    }
    match kind(m) {
        Kind::Soft => true,
        Kind::Hard => m != mat::TORCH,
        Kind::Organic => !crate::props::modelled(world, q, m),
        _ => false,
    }
}

/// Whether a cell is drawn as bevelled masonry (so a neighbour's rounded
/// edge runs on into it).
fn masonry(world: &World, q: [i32; 3]) -> bool {
    let m = world.get(q).mat;
    modelled(q, m)
        && !matches!(m, mat::BENCH | mat::INVENTION_BENCH)
        && !under_dome(q, m)
        && roof_of(q).is_none()
}

// ---------------------------------------------------------------------------
// Small vector helpers.
// ---------------------------------------------------------------------------

type V3 = [f32; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l < 1e-9 {
        [0.0, 1.0, 0.0]
    } else {
        mul(a, 1.0 / l)
    }
}
fn mix(a: V3, b: V3, t: f32) -> V3 {
    add(mul(a, 1.0 - t), mul(b, t))
}
fn axis(a: usize, s: f32) -> V3 {
    let mut v = [0.0; 3];
    v[a] = s;
    v
}
fn iaxis(a: usize, s: i32) -> [i32; 3] {
    let mut v = [0; 3];
    v[a] = s;
    v
}
fn iadd(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn hash(p: [i32; 3], salt: u32) -> f32 {
    let mut h = (p[0] as u32).wrapping_mul(0x8da6_b343)
        ^ (p[1] as u32).wrapping_mul(0xd816_3841)
        ^ (p[2] as u32).wrapping_mul(0xcb1a_b31f)
        ^ salt.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// How a surface is painted.
#[derive(Clone, Copy)]
struct Paint {
    rgb: V3,
    glow: f32,
    mat: u8,
}

/// One vertex.
fn vert(out: &mut MeshData, p: V3, n: V3, paint: Paint, light: f32) -> u32 {
    let i = out.positions.len() as u32;
    out.positions.push(p);
    out.normals.push(n);
    out.colors
        .push([paint.rgb[0], paint.rgb[1], paint.rgb[2], light]);
    out.uvs.push([paint.glow, 0.0]);
    out.mats.push(paint.mat);
    out.light.push(light);
    i
}

/// A triangle of three vertices, wound so its face normal points along
/// `facing`.
fn tri_i(out: &mut MeshData, a: u32, b: u32, c: u32, facing: V3) {
    let pa = out.positions[a as usize];
    let pb = out.positions[b as usize];
    let pc = out.positions[c as usize];
    let n = cross(sub(pb, pa), sub(pc, pa));
    if dot(n, facing) >= 0.0 {
        out.indices.extend_from_slice(&[a, b, c]);
    } else {
        out.indices.extend_from_slice(&[a, c, b]);
    }
}

/// A flat quad (corners in order round it) facing `n`, with one light per
/// corner.
fn quad(out: &mut MeshData, p: [V3; 4], n: V3, paint: Paint, light: [f32; 4]) {
    let i: Vec<u32> = (0..4)
        .map(|k| vert(out, p[k], n, paint, light[k]))
        .collect();
    tri_i(out, i[0], i[1], i[2], n);
    tri_i(out, i[0], i[2], i[3], n);
}

/// A flat-shaded triangle facing away from `inside`.
fn facet(out: &mut MeshData, a: V3, b: V3, c: V3, inside: V3, paint: Paint) {
    let centre = mul(add(add(a, b), c), 1.0 / 3.0);
    let mut n = norm(cross(sub(b, a), sub(c, a)));
    if dot(n, sub(centre, inside)) < 0.0 {
        n = mul(n, -1.0);
    }
    let light = 1.0 - 0.25 * (-n[1]).max(0.0);
    let ia = vert(out, a, n, paint, light);
    let ib = vert(out, b, n, paint, light);
    let ic = vert(out, c, n, paint, light);
    tri_i(out, ia, ib, ic, n);
}

/// A flat-shaded box with chamfered vertical edges (an octagonal prism
/// when `chamfer` is about 0.29 of the half width): centre `c`, half sizes
/// `h`.
fn slab(out: &mut MeshData, c: V3, h: V3, chamfer: f32, paint: Paint) {
    let k = chamfer.min(h[0]).min(h[2]);
    let ring: Vec<V3> = [
        [h[0], h[2] - k],
        [h[0] - k, h[2]],
        [-(h[0] - k), h[2]],
        [-h[0], h[2] - k],
        [-h[0], -(h[2] - k)],
        [-(h[0] - k), -h[2]],
        [h[0] - k, -h[2]],
        [h[0], -(h[2] - k)],
    ]
    .iter()
    .map(|q| [c[0] + q[0], 0.0, c[2] + q[1]])
    .collect();
    prism(out, &ring, c[1] - h[1], c[1] + h[1], c, paint);
}

/// A flat-shaded vertical prism over a horizontal outline (y ignored),
/// from `y0` to `y1`, capped both ends.
fn prism(out: &mut MeshData, ring: &[V3], y0: f32, y1: f32, inside: V3, paint: Paint) {
    let n = ring.len();
    let at = |k: usize, y: f32| [ring[k % n][0], y, ring[k % n][2]];
    let mid = [inside[0], (y0 + y1) / 2.0, inside[2]];
    for k in 0..n {
        let (a0, b0, a1, b1) = (at(k, y0), at(k + 1, y0), at(k, y1), at(k + 1, y1));
        facet(out, a0, b0, b1, mid, paint);
        facet(out, a0, b1, a1, mid, paint);
        facet(
            out,
            [inside[0], y1, inside[2]],
            a1,
            b1,
            [mid[0], y0, mid[2]],
            paint,
        );
        facet(
            out,
            [inside[0], y0, inside[2]],
            a0,
            b0,
            [mid[0], y1, mid[2]],
            paint,
        );
    }
}

/// A tapered, flat-shaded prism of `sides` sides round a vertical axis.
fn column(out: &mut MeshData, base: V3, r0: f32, r1: f32, height: f32, sides: usize, paint: Paint) {
    let ring = |r: f32, y: f32| -> Vec<V3> {
        (0..sides)
            .map(|k| {
                let t = (k as f32 + 0.5) * std::f32::consts::TAU / sides as f32;
                [base[0] + r * t.cos(), y, base[2] + r * t.sin()]
            })
            .collect()
    };
    let lo = ring(r0, base[1]);
    let hi = ring(r1, base[1] + height);
    let inside = [base[0], base[1] + height / 2.0, base[2]];
    for k in 0..sides {
        let j = (k + 1) % sides;
        facet(out, lo[k], lo[j], hi[j], inside, paint);
        facet(out, lo[k], hi[j], hi[k], inside, paint);
        facet(
            out,
            [base[0], base[1] + height, base[2]],
            hi[k],
            hi[j],
            base,
            paint,
        );
        facet(out, base, lo[k], lo[j], hi[0], paint);
    }
}

// ---------------------------------------------------------------------------
// Bevelled masonry.
// ---------------------------------------------------------------------------

/// The 3 x 3 x 3 neighbourhood of a cell: solid, and drawn as masonry.
struct Around {
    solid: [bool; 27],
    masonry: [bool; 27],
}

impl Around {
    fn read(world: &World, p: [i32; 3]) -> Around {
        let mut a = Around {
            solid: [false; 27],
            masonry: [false; 27],
        };
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let i = Around::i([dx, dy, dz]);
                    let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                    a.solid[i] = solid(world, q);
                    a.masonry[i] = a.solid[i] && masonry(world, q);
                }
            }
        }
        a
    }
    fn i(d: [i32; 3]) -> usize {
        ((d[0] + 1) + 3 * (d[1] + 1) + 9 * (d[2] + 1)) as usize
    }
    fn s(&self, d: [i32; 3]) -> bool {
        self.solid[Around::i(d)]
    }
    fn m(&self, d: [i32; 3]) -> bool {
        self.masonry[Around::i(d)]
    }
}

/// The colour of a built block: its material's, varied a little per
/// block, sooted near fire.
fn block_paint(world: &World, p: [i32; 3], m: u8) -> Paint {
    let look = look_of(m);
    let mut rgb = match m {
        mat::BRICK => CLAY,
        mat::PLANKS => mix(TIMBER, look.rgb, 0.45),
        mat::WOOD => TIMBER,
        mat::RUNESTONE => [0.22, 0.25, 0.31],
        mat::IRON => [0.30, 0.31, 0.34],
        _ => look.rgb,
    };
    // A whisper of variation: enough to feel hand-made, too little to show
    // the block grid.
    let vary = 0.97 + 0.06 * hash(p, 11);
    rgb = mul(rgb, vary);
    let house = gen::HOUSES.iter().find(|h| h.contains(p));
    if let (Some(h), mat::BRICK) = (house, m) {
        // Plaster in the house's own tone, a little darker in the bottom
        // course where rain splashes it.
        let base = if p[1] == h.min[1] { 0.86 } else { 1.0 };
        rgb = mul(house_style(h).wall, (0.96 + 0.06 * hash(p, 13)) * base);
    } else if m == mat::BRICK && !stamp::WORKSHOP.contains(p) {
        // Clay masonry (the Kiln's terrace wall and steps): courses of
        // stone a metre high that alternate in tone, each course split into
        // two-metre blocks offset from the course below, and a slightly
        // redder or paler clay per block.
        let course = p[1].rem_euclid(2);
        let off = course;
        let blk = [(p[0] + off).div_euclid(2), p[1], (p[2] + off).div_euclid(2)];
        let tone = if course == 0 { 1.0 } else { 0.88 };
        let k = 0.9 + 0.16 * hash(blk, 17);
        rgb = mul(mix(rgb, [0.62, 0.22, 0.10], 0.35 * hash(blk, 19)), tone * k);
    }
    // Soot: the workshop has been burning (darker higher up), and any
    // block beside fire or ash.
    let mut soot: f32 = 0.0;
    if stamp::WORKSHOP.contains(p) {
        soot = 0.22 + 0.13 * (p[1] - 35) as f32 + 0.12 * hash(p, 5);
        if m == mat::WOOD {
            soot += 0.15;
        }
    }
    'near: for d in [
        [1, 0, 0],
        [-1, 0, 0],
        [0, 1, 0],
        [0, -1, 0],
        [0, 0, 1],
        [0, 0, -1],
    ] {
        let c = world.get(iadd(p, d));
        if c.has(Cell::BURNING) || c.mat == mat::ASH {
            soot = soot.max(0.6);
            break 'near;
        }
    }
    rgb = mix(rgb, CHAR, soot.clamp(0.0, 0.94));
    let glow = if m == mat::KILN_HEARTH {
        look.glow
    } else {
        0.0
    };
    Paint { rgb, glow, mat: m }
}

/// One built block as part of a rounded solid: its open faces (inset
/// where an edge is rounded), the rounded edges it owns, the rounded
/// corners and the end caps where a rounded edge stops against a
/// neighbour.
fn block(out: &mut MeshData, world: &World, p: [i32; 3], m: u8) {
    let around = Around::read(world, p);
    let paint = block_paint(world, p, m);
    let o = [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5];
    let b = BEVEL;
    for a in 0..3 {
        for s in [-1i32, 1] {
            let nd = iaxis(a, s);
            if around.s(nd) {
                continue;
            }
            let n = axis(a, s as f32);
            let (u, v) = ((a + 1) % 3, (a + 2) % 3);
            // Inset on each in-plane side that is open.
            let inset = |ax: usize, sg: i32| {
                if around.s(iaxis(ax, sg)) {
                    0.0
                } else {
                    b
                }
            };
            let (u0, u1) = (-0.5 + inset(u, -1), 0.5 - inset(u, 1));
            let (v0, v1) = (-0.5 + inset(v, -1), 0.5 - inset(v, 1));
            let corner = |cu: f32, cv: f32| {
                add(
                    add(add(o, mul(n, 0.5)), mul(axis(u, 1.0), cu)),
                    mul(axis(v, 1.0), cv),
                )
            };
            // Ambient occlusion at each corner: solids around it in front
            // of the face.
            let ao = |su: i32, sv: i32| {
                let k = [
                    iadd(nd, iaxis(u, su)),
                    iadd(nd, iaxis(v, sv)),
                    iadd(iadd(nd, iaxis(u, su)), iaxis(v, sv)),
                ]
                .iter()
                .filter(|d| around.s(**d))
                .count();
                let down = if s < 0 && a == 1 { 0.72 } else { 1.0 };
                (1.0 - 0.14 * k as f32) * down
            };
            quad(
                out,
                [
                    corner(u0, v0),
                    corner(u1, v0),
                    corner(u1, v1),
                    corner(u0, v1),
                ],
                n,
                paint,
                [ao(-1, -1), ao(1, -1), ao(1, 1), ao(-1, 1)],
            );
            if m == mat::RUNESTONE && a != 1 {
                let top = !around.s([0, 1, 0]);
                runes(out, p, o, n, (u, v), (u0, u1), (v0, v1), top);
            }
            // Rounded edges with the faces of later axes.
            for &e in &[u, v] {
                if e < a {
                    continue;
                }
                for es in [-1i32, 1] {
                    if around.s(iaxis(e, es)) {
                        continue;
                    }
                    edge(out, &around, o, (a, s), (e, es), paint);
                }
            }
            // Rounded corners, owned by the x faces.
            if a == 0 {
                for ys in [-1i32, 1] {
                    for zs in [-1i32, 1] {
                        if !around.s(iaxis(1, ys)) && !around.s(iaxis(2, zs)) {
                            let c = add(
                                o,
                                [
                                    s as f32 * (0.5 - b),
                                    ys as f32 * (0.5 - b),
                                    zs as f32 * (0.5 - b),
                                ],
                            );
                            octant(out, c, [s as f32, ys as f32, zs as f32], paint);
                        }
                    }
                }
            }
        }
    }
}

/// The rounded edge between open faces `(a, s)` and `(e, es)` of the block
/// centred at `o`, along the third axis.
fn edge(
    out: &mut MeshData,
    around: &Around,
    o: V3,
    (a, s): (usize, i32),
    (e, es): (usize, i32),
    paint: Paint,
) {
    let b = BEVEL;
    let t = 3 - a - e;
    let n = axis(a, s as f32);
    let d = axis(e, es as f32);
    let nd = iaxis(a, s);
    let dd = iaxis(e, es);
    // Where the edge ends along t, and whether it needs a cap there.
    let end = |ts: i32| -> (f32, bool) {
        let td = iaxis(t, ts);
        if !around.s(td) {
            (0.5 - b, false)
        } else if around.m(td) && !around.s(iadd(td, nd)) && !around.s(iadd(td, dd)) {
            (0.5, false)
        } else {
            (0.5, true)
        }
    };
    let (lo, cap_lo) = end(-1);
    let (hi, cap_hi) = end(1);
    let tv = axis(t, 1.0);
    let centre = add(o, add(mul(n, 0.5 - b), mul(d, 0.5 - b)));
    // Normalised steps from n to d (the same points `octant` puts on its
    // rim, so edges and corners meet without cracks).
    let dir = |k: usize| {
        let t = k as f32 / SEG as f32;
        norm(add(mul(n, 1.0 - t), mul(d, t)))
    };
    let light = if (a == 1 && s < 0) || (e == 1 && es < 0) {
        0.8
    } else {
        1.0
    };
    // Clay and plaster edges are worn a little darker (B4 round 3: the
    // steps read as flat bands).
    let light = if paint.mat == mat::BRICK {
        light * 0.84
    } else {
        light
    };
    for k in 0..SEG {
        let (d0, d1) = (dir(k), dir(k + 1));
        let p = |dk: V3, tt: f32| add(add(centre, mul(dk, b)), mul(tv, tt));
        let i0 = vert(out, p(d0, -lo), d0, paint, light);
        let i1 = vert(out, p(d0, hi), d0, paint, light);
        let i2 = vert(out, p(d1, hi), d1, paint, light);
        let i3 = vert(out, p(d1, -lo), d1, paint, light);
        let facing = norm(add(d0, d1));
        tri_i(out, i0, i1, i2, facing);
        tri_i(out, i0, i2, i3, facing);
    }
    // Caps: the neighbour's square corner shows where the round stops.
    for (ts, cap, len) in [(-1.0f32, cap_lo, lo), (1.0, cap_hi, hi)] {
        if !cap {
            continue;
        }
        let at = mul(tv, ts * len);
        let sharp = add(add(o, add(mul(n, 0.5), mul(d, 0.5))), at);
        let facing = mul(tv, -ts);
        let ic = vert(out, sharp, facing, paint, 0.8);
        let ids: Vec<u32> = (0..=SEG)
            .map(|k| {
                vert(
                    out,
                    add(add(centre, mul(dir(k), b)), at),
                    facing,
                    paint,
                    0.8,
                )
            })
            .collect();
        for k in 0..SEG {
            tri_i(out, ic, ids[k], ids[k + 1], facing);
        }
    }
}

/// A rounded corner: an eighth of a sphere of radius `BEVEL` round `c`,
/// towards the signs `sg`.
fn octant(out: &mut MeshData, c: V3, sg: V3, paint: Paint) {
    let x = [sg[0], 0.0, 0.0];
    let y = [0.0, sg[1], 0.0];
    let z = [0.0, 0.0, sg[2]];
    let light = if sg[1] < 0.0 { 0.8 } else { 1.0 };
    // A triangular grid of SEG steps on the octant: point (i, j) is the
    // normalised mix of x, y (i steps) and z (j steps).
    let n = SEG;
    let mut ids = Vec::with_capacity((n + 1) * (n + 2) / 2);
    let at = |i: usize, j: usize| i * (n + 1) - i * (i.saturating_sub(1)) / 2 + j;
    for i in 0..=n {
        for j in 0..=n - i {
            let (fi, fj) = (i as f32 / n as f32, j as f32 / n as f32);
            let d = norm(add(add(mul(x, 1.0 - fi - fj), mul(y, fi)), mul(z, fj)));
            debug_assert_eq!(ids.len(), at(i, j));
            ids.push(vert(out, add(c, mul(d, BEVEL)), d, paint, light));
        }
    }
    let f = norm(sg);
    for i in 0..n {
        for j in 0..n - i {
            tri_i(out, ids[at(i, j)], ids[at(i + 1, j)], ids[at(i, j + 1)], f);
            if i + j + 1 < n {
                tri_i(
                    out,
                    ids[at(i + 1, j)],
                    ids[at(i + 1, j + 1)],
                    ids[at(i, j + 1)],
                    f,
                );
            }
        }
    }
}

/// Teal rune work on an open side face of a runestone block: a band along
/// the top course of a wall (and every fourth course), and a diamond
/// glyph on some blocks.
#[allow(clippy::too_many_arguments)]
fn runes(
    out: &mut MeshData,
    p: [i32; 3],
    o: V3,
    n: V3,
    (u, v): (usize, usize),
    (u0, u1): (f32, f32),
    (v0, v1): (f32, f32),
    top: bool,
) {
    let glow = Paint {
        rgb: TEAL,
        glow: 1.0,
        mat: mat::RUNESTONE,
    };
    // The face's in-plane axes: one of u, v is y.
    let (h, (h0, h1), up) = if u == 1 {
        (v, (v0, v1), (u0, u1))
    } else {
        (u, (u0, u1), (v0, v1))
    };
    let face = add(o, mul(n, 0.5 + 0.012));
    let hv = axis(h, 1.0);
    let yv = [0.0, 1.0, 0.0];
    let at = |hh: f32, yy: f32| add(add(face, mul(hv, hh)), mul(yv, yy));
    let band = top || p[1].rem_euclid(4) == 1;
    if band {
        let (y0, y1) = (0.1f32.max(up.0), 0.2f32.min(up.1));
        if y1 > y0 {
            quad(
                out,
                [at(h0, y0), at(h1, y0), at(h1, y1), at(h0, y1)],
                n,
                glow,
                [1.0; 4],
            );
        }
    }
    if band && hash(p, 9) < 0.45 {
        // A diamond with a dot: a little glyph.
        let (cy, r) = (-0.12f32, 0.16f32);
        let ring = [
            at(0.0, cy + r),
            at(r * 0.7, cy),
            at(0.0, cy - r),
            at(-r * 0.7, cy),
        ];
        let inner = [
            at(0.0, cy + r * 0.55),
            at(r * 0.38, cy),
            at(0.0, cy - r * 0.55),
            at(-r * 0.38, cy),
        ];
        for k in 0..4 {
            let j = (k + 1) % 4;
            quad(
                out,
                [ring[k], ring[j], inner[j], inner[k]],
                n,
                glow,
                [1.0; 4],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Benches.
// ---------------------------------------------------------------------------

/// The Formula Bench: a stone lectern on an octagonal plinth, its slanted
/// top a glowing teal screen facing south (towards the plaza and the path).
fn formula_bench(out: &mut MeshData, p: [i32; 3]) {
    let [x, y, z] = [p[0] as f32 + 0.5, p[1] as f32, p[2] as f32 + 0.5];
    let stone = Paint {
        rgb: ROCK,
        glow: 0.0,
        mat: mat::BENCH,
    };
    let dark = Paint {
        rgb: mul(ROCK, 0.6),
        glow: 0.0,
        mat: mat::BENCH,
    };
    let teal = Paint {
        rgb: TEAL,
        glow: 1.0,
        mat: mat::BENCH,
    };
    // Plinth, a teal ring, the column.
    slab(out, [x, y + 0.07, z], [0.48, 0.07, 0.48], 0.14, stone);
    slab(out, [x, y + 0.075, z], [0.495, 0.018, 0.495], 0.145, teal);
    column(out, [x, y + 0.14, z], 0.27, 0.21, 0.56, 8, stone);
    // The slanted desk: a thick board tilted towards +z.
    let tilt = 0.42f32;
    let (c, s) = (tilt.cos(), tilt.sin());
    let top = y + 0.8;
    let corner = |dx: f32, dz: f32, dy: f32| -> V3 {
        // Rotate (dz, dy) about the x axis by `tilt` (front edge lower).
        [x + dx, top + dy * c - dz * s, z + dz * c + dy * s]
    };
    let (hx, hz, ht) = (0.46f32, 0.33f32, 0.07f32);
    let inside = corner(0.0, 0.0, 0.0);
    let pts = |dy: f32| {
        [
            corner(-hx, -hz, dy),
            corner(hx, -hz, dy),
            corner(hx, hz, dy),
            corner(-hx, hz, dy),
        ]
    };
    let (bot, topp) = (pts(-ht), pts(ht));
    for k in 0..4 {
        let j = (k + 1) % 4;
        facet(out, bot[k], bot[j], topp[j], inside, dark);
        facet(out, bot[k], topp[j], topp[k], inside, dark);
    }
    facet(out, topp[0], topp[1], topp[2], inside, stone);
    facet(out, topp[0], topp[2], topp[3], inside, stone);
    facet(out, bot[0], bot[1], bot[2], inside, dark);
    facet(out, bot[0], bot[2], bot[3], inside, dark);
    // The screen: inset on the top face.
    let scr = |dx: f32, dz: f32| corner(dx, dz, ht + 0.012);
    let (sx, sz) = (hx - 0.08, hz - 0.08);
    facet(out, scr(-sx, -sz), scr(sx, -sz), scr(sx, sz), inside, teal);
    facet(out, scr(-sx, -sz), scr(sx, sz), scr(-sx, sz), inside, teal);
}

/// The Invention Bench: a chunky timber worktable with a clay crucible and
/// a glass flask of glowing teal.
fn invention_bench(out: &mut MeshData, p: [i32; 3]) {
    let [x, y, z] = [p[0] as f32 + 0.5, p[1] as f32, p[2] as f32 + 0.5];
    let wood = Paint {
        rgb: TIMBER,
        glow: 0.0,
        mat: mat::INVENTION_BENCH,
    };
    let light_wood = Paint {
        rgb: mix(TIMBER, CLAY, 0.35),
        glow: 0.0,
        mat: mat::INVENTION_BENCH,
    };
    let clay = Paint {
        rgb: CLAY,
        glow: 0.0,
        mat: mat::INVENTION_BENCH,
    };
    let brass = Paint {
        rgb: [0.62, 0.42, 0.12],
        glow: 0.0,
        mat: mat::INVENTION_BENCH,
    };
    let teal = Paint {
        rgb: TEAL,
        glow: 0.85,
        mat: mat::INVENTION_BENCH,
    };
    for (dx, dz) in [(-0.36, -0.32), (0.36, -0.32), (-0.36, 0.32), (0.36, 0.32)] {
        slab(
            out,
            [x + dx, y + 0.3, z + dz],
            [0.07, 0.3, 0.07],
            0.03,
            wood,
        );
    }
    // A low shelf and the top.
    slab(out, [x, y + 0.2, z], [0.4, 0.035, 0.36], 0.04, wood);
    slab(out, [x, y + 0.66, z], [0.5, 0.06, 0.46], 0.08, light_wood);
    // The crucible on the left, a brass stand and flask on the right.
    column(
        out,
        [x - 0.22, y + 0.72, z - 0.05],
        0.1,
        0.15,
        0.18,
        8,
        clay,
    );
    column(
        out,
        [x - 0.22, y + 0.9, z - 0.05],
        0.15,
        0.13,
        0.03,
        8,
        clay,
    );
    column(
        out,
        [x + 0.2, y + 0.72, z + 0.02],
        0.15,
        0.15,
        0.03,
        8,
        brass,
    );
    column(
        out,
        [x + 0.2, y + 0.75, z + 0.02],
        0.12,
        0.05,
        0.16,
        8,
        teal,
    );
    column(
        out,
        [x + 0.2, y + 0.91, z + 0.02],
        0.045,
        0.04,
        0.07,
        6,
        brass,
    );
    // Science detail: a thin teal rune line inlaid round the top's edge,
    // and a small slate on the back with a glowing glyph (art bible: Bench
    // screens are one of the few things that glow).
    let rune = Paint {
        rgb: TEAL,
        glow: 1.0,
        mat: mat::INVENTION_BENCH,
    };
    let slate = Paint {
        rgb: [0.05, 0.06, 0.08],
        glow: 0.0,
        mat: mat::INVENTION_BENCH,
    };
    let (ty0, ty1) = (y + 0.645, y + 0.67);
    let f = 0.462;
    quad(
        out,
        [
            [x - 0.4, ty0, z + f],
            [x + 0.4, ty0, z + f],
            [x + 0.4, ty1, z + f],
            [x - 0.4, ty1, z + f],
        ],
        [0.0, 0.0, 1.0],
        rune,
        [1.0; 4],
    );
    for sx in [-1.0f32, 1.0] {
        let fx = x + sx * 0.5005;
        quad(
            out,
            [
                [fx, ty0, z - 0.36],
                [fx, ty0, z + 0.36],
                [fx, ty1, z + 0.36],
                [fx, ty1, z - 0.36],
            ],
            [sx, 0.0, 0.0],
            rune,
            [1.0; 4],
        );
    }
    let (sx0, sy0, sz0) = (x - 0.05, y + 0.72, z - 0.36);
    slab(
        out,
        [sx0, sy0 + 0.13, sz0],
        [0.13, 0.13, 0.025],
        0.02,
        slate,
    );
    // The glyph: a ring of four bars and a dot, facing the player (+z).
    let gz = sz0 + 0.027;
    let (gx, gy) = (sx0, sy0 + 0.13);
    for (cx, cy, hx, hy) in [
        (0.0, 0.07, 0.07, 0.012),
        (0.0, -0.07, 0.07, 0.012),
        (0.07, 0.0, 0.012, 0.07),
        (-0.07, 0.0, 0.012, 0.07),
        (0.0, 0.0, 0.02, 0.02),
    ] {
        quad(
            out,
            [
                [gx + cx - hx, gy + cy - hy, gz],
                [gx + cx + hx, gy + cy - hy, gz],
                [gx + cx + hx, gy + cy + hy, gz],
                [gx + cx - hx, gy + cy + hy, gz],
            ],
            [0.0, 0.0, 1.0],
            rune,
            [1.0; 4],
        );
    }
    // A brass ring standing up behind the flask.
    let (cx, cy, cz, r) = (x + 0.12, y + 0.85, z - 0.25, 0.12);
    for k in 0..10 {
        let t0 = k as f32 / 10.0 * std::f32::consts::TAU;
        let t1 = (k + 1) as f32 / 10.0 * std::f32::consts::TAU;
        let a = [cx + r * t0.cos(), cy + r * t0.sin(), cz];
        let b = [cx + r * t1.cos(), cy + r * t1.sin(), cz];
        let th = [0.0, 0.0, 0.025];
        facet(out, sub(a, th), sub(b, th), add(b, th), [cx, cy, cz], brass);
        facet(out, sub(a, th), add(b, th), add(a, th), [cx, cy, cz], brass);
    }
}

// ---------------------------------------------------------------------------
// House roofs.
// ---------------------------------------------------------------------------

/// The house whose roof holds cell `p`, if any.
fn roof_of(p: [i32; 3]) -> Option<&'static gen::House> {
    gen::HOUSES.iter().find(|h| h.in_roof(p))
}

/// Whether every roof cell of house `h`'s slice `i` (along x) is still
/// planks.
fn slice_whole(world: &World, h: &gen::House, i: i32) -> bool {
    h.roof_cells(i)
        .iter()
        .all(|&q| world.get(q).mat == mat::PLANKS)
}

/// One metre of a house's pitched roof (slice `i` along its ridge, x):
/// two sloped timber planes 45 degrees steep just over the stepped roof
/// cells, with an overhang, an underside, a fascia under the eaves and
/// gable ends where the roof stops.
fn roof_slice(out: &mut MeshData, world: &World, h: &gen::House, i: i32) {
    let base = (h.min[1] + gen::House::WALL) as f32;
    let (z0, z1) = (h.min[2] as f32, h.max[2] as f32);
    let zc = (z0 + z1) / 2.0;
    let style = house_style(h);
    let over = style.over;
    let th = style.th;
    // The top surface passes 0.12 m over the outer top corner of every
    // roof course: y = base + 1 + 0.12 + (distance in from the wall).
    let top_at = |z: f32| base + 1.12 + (zc - z0) - (z - zc).abs();
    let ridge_y = top_at(zc);
    let first = i == 0;
    let last = i == h.max[0] - h.min[0] - 1;
    let sx0 = (h.min[0] + i) as f32;
    let sx1 = sx0 + 1.0;
    let x0 = if first { sx0 - over } else { sx0 };
    let x1 = if last { sx1 + over } else { sx1 };
    let tile = Paint {
        rgb: mul(style.roof_rgb, 0.9 + 0.2 * hash([i, h.min[0], h.min[2]], 2)),
        glow: 0.0,
        mat: mat::PLANKS,
    };
    let under = Paint {
        rgb: if style.roof == Roof::Thatch {
            mul(style.roof_rgb, 0.55)
        } else {
            mul(TIMBER, 0.7)
        },
        glow: 0.0,
        mat: mat::PLANKS,
    };
    let wall = Paint {
        rgb: mul(style.wall, 0.95),
        glow: 0.0,
        mat: mat::BRICK,
    };
    // Courses on the slope: thatch in two deep layers, tiles in close
    // rows, shingles in thirds.
    let (courses, lip) = match style.roof {
        Roof::Thatch => (3, 0.1f32),
        Roof::Tile => (6, 0.05),
        Roof::Shingle => (3, 0.04),
    };
    for side in [-1.0f32, 1.0] {
        let ze = if side < 0.0 { z0 - over } else { z1 + over };
        let ey = top_at(ze);
        let eave = |x: f32, dy: f32| [x, ey + dy, ze];
        let ridge = |x: f32, dy: f32| [x, ridge_y + dy, zc];
        let n = norm([0.0, 1.0, side]);
        quad(
            out,
            [eave(x0, 0.0), eave(x1, 0.0), ridge(x1, 0.0), ridge(x0, 0.0)],
            n,
            tile,
            [0.9, 0.9, 1.0, 1.0],
        );
        quad(
            out,
            [eave(x0, -th), eave(x1, -th), ridge(x1, -th), ridge(x0, -th)],
            mul(n, -1.0),
            under,
            [0.65; 4],
        );
        // The eave's edge.
        quad(
            out,
            [eave(x0, -th), eave(x1, -th), eave(x1, 0.0), eave(x0, 0.0)],
            [0.0, 0.0, side],
            under,
            [0.85; 4],
        );
        // Courses: a lip at each step up the slope.
        for k in 1..courses {
            let t = k as f32 / courses as f32;
            let a = mix(eave(x0, 0.0), ridge(x0, 0.0), t);
            let b = mix(eave(x1, 0.0), ridge(x1, 0.0), t);
            let up = [0.0, lip, 0.0];
            quad(
                out,
                [a, b, add(b, up), add(a, up)],
                [0.0, 0.3, side],
                tile,
                [0.8; 4],
            );
        }
        // Thatch: a thick rolled edge along the eave.
        if style.roof == Roof::Thatch {
            cbox(
                out,
                [(x0 + x1) / 2.0, ey - th * 0.45, ze - side * 0.02],
                [X, Y, Z],
                [(x1 - x0) / 2.0 + 0.001, th * 0.62, th * 0.5],
                th * 0.3,
                Paint {
                    rgb: mul(style.roof_rgb, 0.82),
                    ..tile
                },
            );
        }
        // The fascia: the wall's plane from the wall top to the roof's
        // underside, so the attic is closed under the eaves.
        let zw = if side < 0.0 { z0 } else { z1 };
        let uy = top_at(zw) - th;
        quad(
            out,
            [
                [sx0, base, zw],
                [sx1, base, zw],
                [sx1, uy, zw],
                [sx0, uy, zw],
            ],
            [0.0, 0.0, side],
            under,
            [0.6, 0.6, 0.5, 0.5],
        );
    }
    // A ridge beam (a fat straw ridge on thatch, a row of capping on
    // tiles).
    let (rh, rw, rp) = match style.roof {
        Roof::Thatch => (0.16, 0.34, tile),
        Roof::Tile => (0.1, 0.18, tile),
        Roof::Shingle => (0.07, 0.1, under),
    };
    slab(
        out,
        [(x0 + x1) / 2.0, ridge_y + 0.03, zc],
        [(x1 - x0) / 2.0, rh, rw],
        rw * 0.5,
        rp,
    );
    // Gable ends: where the house ends or the next slice is gone.
    for (lower, sx) in [(true, -1.0f32), (false, 1.0)] {
        let open = if lower {
            first || !slice_whole(world, h, i - 1)
        } else {
            last || !slice_whole(world, h, i + 1)
        };
        if !open {
            continue;
        }
        let gx = if lower { sx0 } else { sx1 };
        let cap = if lower { x0 } else { x1 };
        let inside = [gx - sx, base + 0.5, zc];
        // The gable wall: up to the roof's underside.
        let under_at = |z: f32| [gx, top_at(z) - th, z];
        let pent = [
            [gx, base, z0],
            [gx, base, z1],
            under_at(z1),
            [gx, ridge_y - th, zc],
            under_at(z0),
        ];
        for k in 1..4 {
            facet(out, pent[0], pent[k], pent[k + 1], inside, wall);
        }
        // The roof's section at its end.
        for ze in [z0 - over, z1 + over] {
            let e_top = [cap, top_at(ze), ze];
            let e_bot = [cap, top_at(ze) - th, ze];
            let r_top = [cap, ridge_y, zc];
            let r_bot = [cap, ridge_y - th, zc];
            facet(out, e_top, e_bot, r_top, inside, under);
            facet(out, e_bot, r_bot, r_top, inside, under);
        }
    }
}

// ---------------------------------------------------------------------------
// The workshop's charred roof beams.
// ---------------------------------------------------------------------------

/// What is left of the workshop's burnt roof: charred beams from post to
/// post, drawn while both posts they rest on stand.
fn workshop_beams(out: &mut MeshData, world: &World) {
    let s = stamp::WORKSHOP;
    let (x0, x1, z0, z1) = (s.min[0], s.max[0] - 1, s.min[2], s.max[2] - 1);
    let top = 38;
    // The posts are wood, or brick once the client has fireproofed them.
    let post = |x: i32, z: i32| matches!(world.get([x, top, z]).mat, mat::WOOD | mat::BRICK);
    let charred = Paint {
        rgb: CHAR,
        glow: 0.0,
        mat: mat::WOOD,
    };
    let y = top as f32 + 0.85;
    let corners = [(x0, z0), (x1, z0), (x1, z1), (x0, z1)];
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        if !(post(a.0, a.1) && post(b.0, b.1)) {
            continue;
        }
        let ca = [a.0 as f32 + 0.5, y, a.1 as f32 + 0.5];
        let cb = [b.0 as f32 + 0.5, y, b.1 as f32 + 0.5];
        let c = mul(add(ca, cb), 0.5);
        let len = (sub(cb, ca)[0].abs() + sub(cb, ca)[2].abs()) / 2.0 + 0.2;
        let h = if a.0 == b.0 {
            [0.13, 0.12, len]
        } else {
            [len, 0.12, 0.13]
        };
        slab(out, c, h, 0.05, charred);
    }
    // One broken rafter leaning across, if the north posts stand.
    if post(x0, z0) && post(x1, z0) {
        let a = [x0 as f32 + 1.0, y, z0 as f32 + 0.5];
        let b = [x0 as f32 + 3.2, y - 1.6, z0 as f32 + 3.5];
        let w = [0.0, 0.1, 0.0];
        let side = norm(cross(sub(b, a), [0.0, 1.0, 0.0]));
        let sd = mul(side, 0.1);
        let pts = [
            sub(sub(a, w), sd),
            add(sub(a, w), sd),
            add(add(a, w), sd),
            sub(add(a, w), sd),
        ];
        let off = sub(b, a);
        let inside = mul(add(a, b), 0.5);
        for k in 0..4 {
            let j = (k + 1) % 4;
            facet(out, pts[k], pts[j], add(pts[j], off), inside, charred);
            facet(
                out,
                pts[k],
                add(pts[j], off),
                add(pts[k], off),
                inside,
                charred,
            );
        }
    }
    workshop_frame(out, world, (x0, x1, z0, z1), &post);
}

/// The workshop as a building (B4 round 3: it read as a pile of
/// pillars): charred top plates on the low walls, a door frame and lintel
/// facing the plaza, and what is left of its pitched roof: rafters up to a
/// ridge beam, a few gone, and a patch of scorched planks on the north
/// slope. Looks only, drawn while the posts stand.
fn workshop_frame(
    out: &mut MeshData,
    world: &World,
    (x0, x1, z0, z1): (i32, i32, i32, i32),
    post: &dyn Fn(i32, i32) -> bool,
) {
    let charred = paint(mix(CHAR, TIMBER, 0.25), mat::WOOD);
    let scorched = paint(mix(TIMBER, CHAR, 0.55), mat::PLANKS);
    let wall = |p: [i32; 3]| matches!(world.get(p).mat, mat::BRICK | mat::WOOD | mat::STONE);
    let (fx0, fx1) = (x0 as f32 + 0.5, x1 as f32 + 0.5);
    let (fz0, fz1) = (z0 as f32 + 0.5, z1 as f32 + 0.5);
    // Top plates along the wall tops (y 37), where the wall below stands.
    for (z, fz) in [(z0, fz0), (z1, fz1)] {
        for x in x0 + 1..x1 {
            if wall([x, 36, z]) {
                cbox(
                    out,
                    [x as f32 + 0.5, 37.08, fz],
                    [X, Y, Z],
                    [0.52, 0.08, 0.36],
                    0.03,
                    charred,
                );
            }
        }
    }
    for (x, fx) in [(x0, fx0), (x1, fx1)] {
        for z in z0 + 1..z1 {
            if wall([x, 36, z]) {
                cbox(
                    out,
                    [fx, 37.08, z as f32 + 0.5],
                    [X, Y, Z],
                    [0.36, 0.08, 0.52],
                    0.03,
                    charred,
                );
            }
        }
    }
    // The door on the south wall (x 141-142): jambs and a lintel.
    let dz = z1 as f32 + 0.5;
    if wall([140, 35, z1]) && wall([143, 35, z1]) {
        for jx in [141.0f32, 143.0] {
            cbox(
                out,
                [jx, 36.2, dz],
                [X, Y, Z],
                [0.12, 1.2, 0.42],
                0.03,
                charred,
            );
        }
        cbox(
            out,
            [142.0, 37.45, dz],
            [X, Y, Z],
            [1.25, 0.16, 0.46],
            0.04,
            charred,
        );
    }
    // The village kit (B4 round 3 judges: the workshop read as a box with
    // plank strips beside the cottages): a stone plinth round its foot and
    // round clay buttresses at the corner posts, as on the houses.
    let stone = paint(mul(ROCK, 1.15), mat::STONE);
    let plaster = paint(mix(mul(CLAY, 0.85), CHAR, 0.12), mat::BRICK);
    let (ox0, ox1) = (x0 as f32, x1 as f32 + 1.0);
    let (oz0, oz1) = (z0 as f32, z1 as f32 + 1.0);
    let (mx, mz) = ((ox0 + ox1) / 2.0, (oz0 + oz1) / 2.0);
    for (cz, hz) in [(oz0, 0.12f32), (oz1, 0.12)] {
        cbox(
            out,
            [mx, 35.16, cz],
            [X, Y, Z],
            [(ox1 - ox0) / 2.0 + 0.12, 0.17, hz],
            0.05,
            stone,
        );
    }
    for cx in [ox0, ox1] {
        cbox(
            out,
            [cx, 35.16, mz],
            [X, Y, Z],
            [0.12, 0.17, (oz1 - oz0) / 2.0 + 0.12],
            0.05,
            stone,
        );
    }
    for (cx, cz) in [(x0, z0), (x1, z0), (x0, z1), (x1, z1)] {
        if !post(cx, cz) {
            continue;
        }
        let b = [cx as f32 + 0.5, 34.98, cz as f32 + 0.5];
        column(out, b, 0.74, 0.7, 0.3, 10, stone);
        column(out, add(b, [0.0, 0.28, 0.0]), 0.66, 0.56, 2.0, 10, plaster);
        column(out, add(b, [0.0, 2.26, 0.0]), 0.62, 0.48, 0.14, 10, plaster);
    }
    // The roof: a ridge along x at the middle, rafters from the eave plates
    // at the posts' tops up to it, every other one burnt through.
    if !(post(x0, z0) && post(x1, z0) && post(x0, z1) && post(x1, z1)) {
        return;
    }
    let ey = 39.0f32;
    let zc = (fz0 + fz1) / 2.0;
    let ry = ey + (zc - fz0) * 0.75;
    let beam = |out: &mut MeshData, a: V3, b: V3, w: f32| {
        let d = sub(b, a);
        let len = dot(d, d).sqrt();
        let along = mul(d, 1.0 / len);
        let side = norm(cross(along, Y));
        let up = cross(side, along);
        cbox(
            out,
            mul(add(a, b), 0.5),
            [along, up, side],
            [len / 2.0, w, w],
            w * 0.3,
            charred,
        );
    };
    beam(out, [fx0 - 0.3, ry, zc], [fx1 + 0.3, ry, zc], 0.13);
    // Posts under the ridge at the gables.
    for fx in [fx0, fx1] {
        beam(out, [fx, ey - 0.1, zc], [fx, ry, zc], 0.1);
    }
    let n = 5;
    for k in 0..n {
        let fx = fx0 + (fx1 - fx0) * k as f32 / (n - 1) as f32;
        for (side, fz) in [(-1.0f32, fz0), (1.0, fz1)] {
            // Burnt through: the south slope's middle rafters, one north.
            let gone = (side > 0.0 && (1..=3).contains(&k) && k != 2) || (side < 0.0 && k == 3);
            if gone {
                // A charred stub at the eave.
                let a = [fx, ey, fz - side * -0.2];
                let b = mix(a, [fx, ry, zc], 0.3);
                beam(out, a, b, 0.08);
                continue;
            }
            beam(out, [fx, ey - 0.05, fz + side * 0.25], [fx, ry, zc], 0.08);
        }
    }
    // Scorched planks left on the north slope's west half.
    let slope = norm([0.0, ry - ey, zc - fz0]);
    let across = norm(cross(X, slope));
    for k in 0..4 {
        let t = 0.18 + 0.2 * k as f32;
        let a = mix([fx0, ey, fz0 - 0.25], [fx0, ry, zc], t);
        let len = 2.4 - 0.45 * k as f32 + 0.3 * hash([k, 0, 7], 60);
        let c = add(add(a, [len / 2.0, 0.0, 0.0]), mul(across, 0.1));
        cbox(
            out,
            c,
            [X, slope, across],
            [len / 2.0, 0.2, 0.035],
            0.015,
            scorched,
        );
    }
    // What is left of the tiles: the north slope's east two thirds, and
    // the south slope's two end bays, scorched towards the burnt hole.
    let tile = paint([0.42, 0.085, 0.04], mat::PLANKS);
    let tile_burnt = paint(mix([0.42, 0.085, 0.04], CHAR, 0.55), mat::PLANKS);
    let span = fx1 - fx0;
    for (side, fz, xs) in [
        (-1.0f32, fz0, vec![(fx0 + span * 0.36, fx1 + 0.45)]),
        (
            1.0,
            fz1,
            vec![
                (fx0 - 0.45, fx0 + span * 0.2),
                (fx1 - span * 0.2, fx1 + 0.45),
            ],
        ),
    ] {
        let eave = [0.0, ey - 0.25, fz + side * 0.55];
        let ridge = [0.0, ry + 0.12, zc];
        let d = sub(ridge, eave);
        let len = dot(d, d).sqrt();
        let slope = mul(d, 1.0 / len);
        let up = norm(cross(slope, X));
        let up = if up[1] < 0.0 { mul(up, -1.0) } else { up };
        for (a, b) in xs {
            let mid_x = (fx0 + fx1) / 2.0;
            let ex = if (a - mid_x).abs() < (b - mid_x).abs() {
                a
            } else {
                b
            };
            let cx = (a + b) / 2.0;
            let c = add([cx, 0.0, 0.0], mul(add(eave, ridge), 0.5));
            let c = add(c, mul(up, 0.12));
            cbox(
                out,
                c,
                [X, slope, up],
                [(b - a) / 2.0, len / 2.0, 0.07],
                0.03,
                tile,
            );
            // Courses across the slope.
            for k in 0..5 {
                let t = -0.4 + 0.2 * k as f32;
                cbox(
                    out,
                    add(add(c, mul(slope, t * len)), mul(up, 0.075)),
                    [X, slope, up],
                    [(b - a) / 2.0 - 0.02, 0.035, 0.025],
                    0.01,
                    tile_dark_of(tile),
                );
            }
            // A scorched edge where the fire ate the rest.
            cbox(
                out,
                add([ex - cx, 0.0, 0.0], add(c, mul(up, 0.02))),
                [X, slope, up],
                [0.22, len / 2.0 - 0.05, 0.08],
                0.03,
                tile_burnt,
            );
        }
    }
}

/// A darker tone of a roof tile paint, for its courses.
fn tile_dark_of(p: Paint) -> Paint {
    Paint {
        rgb: mul(p.rgb, 0.68),
        ..p
    }
}

/// The village well (`gen::WELL`): a round, bevelled stone kerb round
/// the water, two timber posts and a little shingled roof over a winch
/// with its rope and bucket (B4 round 3: the bare stone cells read as a
/// grey blob). Looks only, while the kerb's stone stands.
fn well(out: &mut MeshData, world: &World) {
    let w = gen::WELL;
    if world.get([w[0] - 1, w[1] + 1, w[2] - 1]).mat != mat::STONE {
        return;
    }
    let c = [w[0] as f32 + 1.0, w[1] as f32 + 1.0, w[2] as f32 + 1.0];
    let stone = paint(mul(ROCK, 1.3), mat::STONE);
    let cap = paint(mul(ROCK, 1.55), mat::STONE);
    let timber = paint(mul(TIMBER, 0.95), mat::WOOD);
    let roof = paint(mix([0.36, 0.12, 0.06], TIMBER, 0.4), mat::PLANKS);
    let water = paint([0.012, 0.07, 0.09], mat::WATER);
    // Paving round it: flagstones in two rings, so the stone under the
    // kerb reads as a laid floor, not a lump.
    for (ring_r, count, salt) in [(2.25f32, 14, 60u32), (2.95, 18, 64)] {
        for k in 0..count {
            let t = (k as f32 + 0.5 + 0.2 * hash([k, 0, 1], salt)) / count as f32
                * std::f32::consts::TAU;
            let at = [
                c[0] + ring_r * t.cos(),
                c[1] - 0.02,
                c[2] + ring_r * t.sin(),
            ];
            let along = [-t.sin(), 0.0, t.cos()];
            let out_d = [t.cos(), 0.0, t.sin()];
            let tone = 0.85 + 0.3 * hash([k, 1, 1], salt + 1);
            let flag = paint(mul(ROCK, 1.2 * tone), mat::STONE);
            let w = ring_r * std::f32::consts::PI / count as f32 - 0.05;
            cbox(out, at, [along, Y, out_d], [w, 0.07, 0.34], 0.04, flag);
        }
    }
    // The kerb: a ring of 12 stones, each a bevelled block on the circle.
    let n = 12;
    for k in 0..n {
        let t = (k as f32 + 0.5) / n as f32 * std::f32::consts::TAU;
        let r = 1.72;
        let at = [c[0] + r * t.cos(), c[1] - 0.05 + 0.45, c[2] + r * t.sin()];
        let along = [-t.sin(), 0.0, t.cos()];
        let out_d = [t.cos(), 0.0, t.sin()];
        let h = 0.5 + 0.04 * hash([k, 0, 0], 70);
        cbox(out, at, [along, Y, out_d], [0.47, h, 0.34], 0.1, stone);
        cbox(
            out,
            add(at, [0.0, h + 0.05, 0.0]),
            [along, Y, out_d],
            [0.5, 0.07, 0.38],
            0.04,
            cap,
        );
    }
    // Dark water inside, a little below the kerb's top.
    let ring: Vec<V3> = (0..n)
        .map(|k| {
            let t = k as f32 / n as f32 * std::f32::consts::TAU;
            [c[0] + 1.42 * t.cos(), c[1] + 0.12, c[2] + 1.42 * t.sin()]
        })
        .collect();
    for k in 0..n as usize {
        let j = (k + 1) % n as usize;
        facet(
            out,
            ring[k],
            ring[j],
            [c[0], c[1] + 0.12, c[2]],
            [c[0], c[1] - 1.0, c[2]],
            water,
        );
    }
    // Posts, winch and roof, along x.
    for sx in [-1.0f32, 1.0] {
        let b = [c[0] + sx * 1.72, c[1] + 1.0, c[2]];
        cbox(
            out,
            add(b, [0.0, 0.95, 0.0]),
            [X, Y, Z],
            [0.1, 0.95, 0.1],
            0.03,
            timber,
        );
    }
    let wy = c[1] + 2.25;
    cbox(
        out,
        [c[0], wy, c[2]],
        [X, Y, Z],
        [1.75, 0.07, 0.07],
        0.03,
        timber,
    );
    cbox(
        out,
        [c[0] + 0.2, wy - 0.75, c[2]],
        [X, Y, Z],
        [0.012, 0.72, 0.012],
        0.0,
        paint([0.4, 0.3, 0.15], mat::WOOD),
    );
    column(
        out,
        [c[0] + 0.2, wy - 1.75, c[2]],
        0.17,
        0.2,
        0.28,
        8,
        paint(mix(TIMBER, CLAY, 0.3), mat::WOOD),
    );
    let ridge = c[1] + 3.05;
    for side in [-1.0f32, 1.0] {
        let a = [c[0], ridge, c[2]];
        let e = [c[0], ridge - 0.6, c[2] + side * 1.05];
        let d = sub(e, a);
        let len = dot(d, d).sqrt();
        let down = mul(d, 1.0 / len);
        let across = norm(cross(X, down));
        cbox(
            out,
            mul(add(a, e), 0.5),
            [X, down, across],
            [2.05, len / 2.0, 0.06],
            0.03,
            roof,
        );
    }
    cbox(
        out,
        [c[0], ridge + 0.02, c[2]],
        [X, Y, Z],
        [2.1, 0.06, 0.08],
        0.03,
        timber,
    );
}

// ---------------------------------------------------------------------------
// Trim: house doors and windows, the ice gate, the Marsh vents.
// ---------------------------------------------------------------------------

/// A flat-shaded box with every edge chamfered by `b`: centre `c`, local
/// axes `ax` (unit, right-handed or not), half sizes `h` along them.
fn cbox(out: &mut MeshData, c: V3, ax: [V3; 3], h: V3, b: f32, paint: Paint) {
    let b = b.min(h[0]).min(h[1]).min(h[2]) * 0.999;
    let at = |l: V3| {
        add(
            add(add(c, mul(ax[0], l[0])), mul(ax[1], l[1])),
            mul(ax[2], l[2]),
        )
    };
    // The corner point of corner `sg` on the face of axis `a`.
    let pt = |a: usize, sg: [f32; 3]| {
        let mut l = [0.0; 3];
        for k in 0..3 {
            l[k] = sg[k] * if k == a { h[k] } else { h[k] - b };
        }
        at(l)
    };
    let sq = |out: &mut MeshData, p: [V3; 4]| {
        facet(out, p[0], p[1], p[2], c, paint);
        facet(out, p[0], p[2], p[3], c, paint);
    };
    for a in 0..3 {
        let (u, v) = ((a + 1) % 3, (a + 2) % 3);
        for s in [-1.0f32, 1.0] {
            let corner = |su: f32, sv: f32| {
                let mut sg = [0.0; 3];
                sg[a] = s;
                sg[u] = su;
                sg[v] = sv;
                sg
            };
            // The face.
            sq(
                out,
                [
                    pt(a, corner(-1.0, -1.0)),
                    pt(a, corner(1.0, -1.0)),
                    pt(a, corner(1.0, 1.0)),
                    pt(a, corner(-1.0, 1.0)),
                ],
            );
            // The chamfer between this face and the +-u face, along v.
            for su in [-1.0f32, 1.0] {
                sq(
                    out,
                    [
                        pt(a, corner(su, -1.0)),
                        pt(u, corner(su, -1.0)),
                        pt(u, corner(su, 1.0)),
                        pt(a, corner(su, 1.0)),
                    ],
                );
            }
        }
    }
    // Corner triangles.
    for sx in [-1.0f32, 1.0] {
        for sy in [-1.0f32, 1.0] {
            for sz in [-1.0f32, 1.0] {
                let sg = [sx, sy, sz];
                facet(out, pt(0, sg), pt(1, sg), pt(2, sg), c, paint);
            }
        }
    }
}

const X: V3 = [1.0, 0.0, 0.0];
const Y: V3 = [0.0, 1.0, 0.0];
const Z: V3 = [0.0, 0.0, 1.0];

fn paint(rgb: V3, m: u8) -> Paint {
    Paint {
        rgb,
        glow: 0.0,
        mat: m,
    }
}

/// A Village house's trim (art bible: "doors, chunky timber"): proud
/// timber corner posts and an eave beam, a framed plank door standing
/// ajar on the north wall, and framed windows with a cross bar, a dark
/// pane and open shutters. Looks only: each piece is drawn while the cells
/// it hangs on stand, and the door and window openings stay open.
fn house_trim(out: &mut MeshData, world: &World, h: &gen::House) {
    let [x0, y0, z0] = h.min;
    let (x1, z1) = (h.max[0], h.max[2]);
    let top = y0 + gen::House::WALL;
    let is = |p: [i32; 3], m: u8| world.get(p).mat == m;
    let wall = |p: [i32; 3]| matches!(world.get(p).mat, mat::BRICK | mat::WOOD);
    let style = house_style(h);
    let timber = paint(mul(TIMBER, 0.9), mat::WOOD);
    let plank = paint(mix(style.shutter, CLAY, 0.15), mat::WOOD);
    let dark = paint([0.02, 0.025, 0.035], mat::WOOD);
    let iron = paint([0.05, 0.05, 0.055], mat::IRON);
    let plaster = paint(mul(style.wall, 0.9), mat::BRICK);
    let stone = paint(mul(ROCK, 1.15), mat::STONE);
    // Corners: round clay buttresses on a stone plinth, a little proud of
    // both walls (art bible: rounded, bevelled built things, not boxes).
    for (cx, cz) in [(x0, z0), (x1 - 1, z0), (x0, z1 - 1), (x1 - 1, z1 - 1)] {
        if (y0..top).all(|y| is([cx, y, cz], mat::WOOD)) {
            let b = [cx as f32 + 0.5, y0 as f32 - 0.02, cz as f32 + 0.5];
            column(out, b, 0.78, 0.74, 0.32, 10, stone);
            column(
                out,
                add(b, [0.0, 0.3, 0.0]),
                0.7,
                0.6,
                (top - y0) as f32 - 0.3,
                10,
                plaster,
            );
            column(
                out,
                add(b, [0.0, (top - y0) as f32 - 0.02, 0.0]),
                0.66,
                0.5,
                0.16,
                10,
                plaster,
            );
        }
    }
    // Eave beams along the long walls, under the roof.
    for (z, out_z) in [(z0, z0 as f32 - 0.02), (z1 - 1, z1 as f32 + 0.02)] {
        if (x0..x1).all(|x| wall([x, top - 1, z])) {
            let c = [(x0 + x1) as f32 / 2.0, top as f32 - 0.16, out_z];
            cbox(
                out,
                c,
                [X, Y, Z],
                [(x1 - x0) as f32 / 2.0, 0.15, 0.12],
                0.04,
                timber,
            );
        }
    }
    // The door: frame, lintel and a plank leaf swung half open inwards.
    let d = h.door_x();
    let zf = z0 as f32;
    if is([d, y0 + 2, z0], mat::BRICK) && wall([d - 1, y0, z0]) && wall([d + 1, y0, z0]) {
        let dx0 = d as f32;
        for jx in [dx0, dx0 + 1.0] {
            cbox(
                out,
                [jx, y0 as f32 + 1.0, zf],
                [X, Y, Z],
                [0.1, 1.0, 0.14],
                0.03,
                timber,
            );
        }
        cbox(
            out,
            [dx0 + 0.5, y0 as f32 + 2.05, zf],
            [X, Y, Z],
            [0.66, 0.12, 0.16],
            0.04,
            timber,
        );
        // A stone step.
        cbox(
            out,
            [dx0 + 0.5, y0 as f32 - 0.06, zf - 0.25],
            [X, Y, Z],
            [0.6, 0.08, 0.28],
            0.04,
            paint(mul(ROCK, 1.2), mat::STONE),
        );
        // The leaf: hinged on the west jamb, open 55 degrees into the house.
        let a = 55f32.to_radians();
        let along = [a.cos(), 0.0, a.sin()];
        let across = [-a.sin(), 0.0, a.cos()];
        let hinge = [dx0 + 0.1, y0 as f32, zf + 0.1];
        let w = 0.8f32;
        let c = add(add(hinge, mul(along, w / 2.0)), [0.0, 0.97, 0.0]);
        cbox(
            out,
            c,
            [along, Y, across],
            [w / 2.0, 0.95, 0.04],
            0.02,
            plank,
        );
        for k in [-1.0f32, 0.0, 1.0] {
            let g = add(
                add(c, mul(along, k * w / 4.0 + w / 8.0)),
                mul(across, -0.042),
            );
            cbox(out, g, [along, Y, across], [0.012, 0.9, 0.006], 0.003, dark);
        }
        for yy in [0.45f32, 1.5] {
            let g = add(add(hinge, mul(along, w * 0.4)), [0.0, yy, 0.0]);
            let g = add(g, mul(across, -0.045));
            cbox(
                out,
                g,
                [along, Y, across],
                [w * 0.38, 0.04, 0.01],
                0.008,
                iron,
            );
        }
    }
    // Windows: on the north and south walls, a block above the floor.
    for (z, sg) in [(z0, -1.0f32), (z1 - 1, 1.0)] {
        for x in [x0 + 1, x1 - 2] {
            if x == d && z == z0 {
                continue;
            }
            let hole = world.get([x, y0 + 1, z]).mat == mat::AIR;
            if !(hole && wall([x, y0 + 2, z]) && wall([x, y0, z])) {
                continue;
            }
            let face = if sg < 0.0 { z as f32 } else { z as f32 + 1.0 };
            let (cx, cy) = (x as f32 + 0.5, y0 as f32 + 1.5);
            // Sill, lintel and jambs.
            cbox(
                out,
                [cx, cy - 0.53, face + sg * 0.06],
                [X, Y, Z],
                [0.62, 0.06, 0.14],
                0.03,
                timber,
            );
            cbox(
                out,
                [cx, cy + 0.53, face + sg * 0.02],
                [X, Y, Z],
                [0.6, 0.07, 0.1],
                0.03,
                timber,
            );
            for jx in [-0.5f32, 0.5] {
                cbox(
                    out,
                    [cx + jx, cy, face + sg * 0.02],
                    [X, Y, Z],
                    [0.07, 0.5, 0.1],
                    0.02,
                    timber,
                );
            }
            // A dark pane set in the wall, and a cross bar.
            cbox(
                out,
                [cx, cy, face - sg * 0.45],
                [X, Y, Z],
                [0.5, 0.5, 0.02],
                0.0,
                dark,
            );
            cbox(
                out,
                [cx, cy, face - sg * 0.4],
                [X, Y, Z],
                [0.035, 0.48, 0.03],
                0.01,
                timber,
            );
            cbox(
                out,
                [cx, cy, face - sg * 0.4],
                [X, Y, Z],
                [0.48, 0.035, 0.03],
                0.01,
                timber,
            );
            // Shutters, folded back flat on the wall each side.
            for sx in [-1.0f32, 1.0] {
                let c = [cx + sx * 0.82, cy, face + sg * 0.05];
                cbox(out, c, [X, Y, Z], [0.26, 0.48, 0.035], 0.02, plank);
                cbox(
                    out,
                    add(c, [0.0, 0.0, sg * 0.036]),
                    [X, Y, Z],
                    [0.2, 0.03, 0.004],
                    0.0,
                    dark,
                );
            }
        }
    }
    house_extras(out, world, h, &style);
}

/// A house's chimney, porch canopy and flower boxes (B4 round 3: variety
/// between the houses). Each is drawn while the cells it stands on stand.
fn house_extras(out: &mut MeshData, world: &World, h: &gen::House, style: &HouseStyle) {
    let [x0, y0, z0] = h.min;
    let (x1, z1) = (h.max[0], h.max[2]);
    let top = y0 + gen::House::WALL;
    let wall = |p: [i32; 3]| matches!(world.get(p).mat, mat::BRICK | mat::WOOD);
    let timber = paint(mul(TIMBER, 0.9), mat::WOOD);
    let plaster = paint(mul(style.wall, 0.82), mat::BRICK);
    let soot = paint([0.05, 0.045, 0.04], mat::BRICK);
    // The chimney rises through the roof from its slice, near the eave
    // side it names, and ends in a sooty cap.
    if let Some((i, side)) = style.chimney {
        let x = x0 + i;
        if i < x1 - x0 && slice_whole(world, h, i) {
            let zc = (z0 + z1) as f32 / 2.0;
            let cz = zc + side * 1.0;
            let ridge = top as f32 + 1.12 + (zc - z0 as f32);
            let b = [x as f32 + 0.5, top as f32 + 0.6, cz];
            let hgt = ridge - b[1] + 1.0;
            cbox(
                out,
                add(b, [0.0, hgt / 2.0, 0.0]),
                [X, Y, Z],
                [0.42, hgt / 2.0, 0.42],
                0.12,
                plaster,
            );
            cbox(
                out,
                add(b, [0.0, hgt + 0.08, 0.0]),
                [X, Y, Z],
                [0.5, 0.1, 0.5],
                0.05,
                plaster,
            );
            cbox(
                out,
                add(b, [0.0, hgt + 0.18, 0.0]),
                [X, Y, Z],
                [0.28, 0.02, 0.28],
                0.0,
                soot,
            );
        }
    }
    // A porch canopy over the door: a sloped plank roof on two brackets.
    let d = h.door_x();
    let zf = z0 as f32;
    if style.porch && wall([d - 1, y0 + 2, z0]) && wall([d + 1, y0 + 2, z0]) {
        let y = y0 as f32 + 2.45;
        let roof = paint(mul(style.roof_rgb, 0.95), mat::PLANKS);
        let c = [d as f32 + 0.5, y, zf - 0.5];
        let along = X;
        let down = norm([0.0, -0.6, -1.0]);
        let across = cross(along, down);
        cbox(
            out,
            c,
            [along, down, across],
            [1.05, 0.55, 0.06],
            0.03,
            roof,
        );
        for bx in [d as f32 - 0.4, d as f32 + 1.4] {
            let a = [bx, y - 0.6, zf - 0.08];
            cbox(out, a, [X, Y, Z], [0.06, 0.3, 0.08], 0.02, timber);
            cbox(
                out,
                [bx, y - 0.25, zf - 0.45],
                [X, Y, Z],
                [0.06, 0.05, 0.4],
                0.02,
                timber,
            );
        }
    }
    // Flower boxes under the windows: a timber trough with bushy green and
    // a few warm blooms.
    if style.flowers {
        let leaf = paint([0.10, 0.26, 0.05], mat::LEAVES);
        let bloom = paint([0.85, 0.30, 0.10], mat::LEAVES);
        let bloom2 = paint([0.90, 0.75, 0.20], mat::LEAVES);
        for (z, sg) in [(z0, -1.0f32), (z1 - 1, 1.0)] {
            for x in [x0 + 1, x1 - 2] {
                if x == d && z == z0 {
                    continue;
                }
                if world.get([x, y0 + 1, z]).mat != mat::AIR || !wall([x, y0, z]) {
                    continue;
                }
                let face = if sg < 0.0 { z as f32 } else { z as f32 + 1.0 };
                let c = [x as f32 + 0.5, y0 as f32 + 0.86, face + sg * 0.22];
                cbox(out, c, [X, Y, Z], [0.55, 0.12, 0.16], 0.03, timber);
                for k in 0..5 {
                    let fx = c[0] - 0.44 + 0.22 * k as f32;
                    let r = 0.13 + 0.04 * hash([x, k, z], 50);
                    let q = [fx, c[1] + 0.12, c[2]];
                    column(
                        out,
                        q,
                        r,
                        r * 0.6,
                        0.16 + 0.06 * hash([x, k, z], 51),
                        6,
                        leaf,
                    );
                    if k % 2 == 0 {
                        let pk = if hash([x, k, z], 52) > 0.5 {
                            bloom
                        } else {
                            bloom2
                        };
                        column(out, add(q, [0.02, 0.18, 0.0]), 0.06, 0.03, 0.06, 5, pk);
                    }
                }
            }
        }
    }
    // A water barrel by the back corner.
    let bx = [x1 as f32 + 0.35, y0 as f32, z1 as f32 - 0.6];
    if wall([x1 - 1, y0, z1 - 1]) {
        let staves = paint(mix(TIMBER, CLAY, 0.25), mat::WOOD);
        column(out, bx, 0.34, 0.38, 0.45, 9, staves);
        column(out, add(bx, [0.0, 0.45, 0.0]), 0.38, 0.33, 0.45, 9, staves);
        cbox(
            out,
            add(bx, [0.0, 0.3, 0.0]),
            [X, Y, Z],
            [0.385, 0.025, 0.385],
            0.02,
            paint([0.05, 0.05, 0.055], mat::IRON),
        );
    }
}

/// Ice (#CFE8F2) and frost rim (#9FE3FF), linear.
const ICE: V3 = [0.62, 0.81, 0.89];
const FROST: V3 = [0.35, 0.77, 1.0];

/// The ice gate: each column of the ice plug is a thick faceted ice prism
/// (a ridge forward, the facets alternating ice and frost blue), set in a
/// frame of stone jambs and a snow-capped lintel against the cliff.
fn ice_gate(out: &mut MeshData, world: &World) {
    let g = stamp::ICE_GATE;
    let ice = paint(ICE, mat::ICE);
    // The frost facets glow softly (the gate is held shut by the cold:
    // B4 round 3 asked for a frost glow so its silhouette reads).
    let frost = Paint {
        glow: 0.72,
        ..paint(FROST, mat::ICE)
    };
    for x in g.min[0]..g.max[0] {
        for z in g.min[2]..g.max[2] {
            // Runs of ice up the column.
            let mut y = g.min[1];
            while y < g.max[1] {
                if !ice_plug([x, y, z], world.get([x, y, z]).mat) {
                    y += 1;
                    continue;
                }
                let y0 = y;
                while y < g.max[1] && ice_plug([x, y, z], world.get([x, y, z]).mat) {
                    y += 1;
                }
                let (fx, fz) = (x as f32, z as f32);
                let j = 0.18 * (hash([x, 0, z], 3) - 0.5);
                let reach = 0.16 + 0.1 * hash([x, 1, z], 4);
                let ring = [
                    [fx + 0.01, 0.0, fz - 0.04],
                    [fx + 0.5 - j, 0.0, fz - 0.14],
                    [fx + 0.99, 0.0, fz - 0.04],
                    [fx + 0.99, 0.0, fz + 0.94],
                    [fx + 0.5 + j, 0.0, fz + 1.0 + reach],
                    [fx + 0.01, 0.0, fz + 0.94],
                ];
                let (ya, yb) = (y0 as f32 + 0.01, y as f32 - 0.01);
                let mid = [fx + 0.5, (ya + yb) / 2.0, fz + 0.5];
                let n = ring.len();
                for k in 0..n {
                    let kk = (k + 1) % n;
                    let at = |q: V3, yy: f32| [q[0], yy, q[2]];
                    let p = if k % 2 == 0 { ice } else { frost };
                    // Each side splits at a kink half way up, so the face
                    // is two facets (cut ice, not a pane).
                    let ym = (ya + yb) / 2.0 + 0.3 * (hash([x, k as i32, z], 6) - 0.5);
                    let bulge = |q: V3| {
                        let d = sub(q, [mid[0], q[1], mid[2]]);
                        add(q, mul(d, 0.06))
                    };
                    let (a0, b0) = (at(ring[k], ya), at(ring[kk], ya));
                    let (a1, b1) = (bulge(at(ring[k], ym)), bulge(at(ring[kk], ym)));
                    let (a2, b2) = (at(ring[k], yb), at(ring[kk], yb));
                    facet(out, a0, b0, b1, mid, p);
                    facet(out, a0, b1, a1, mid, p);
                    facet(out, a1, b1, b2, mid, if k % 2 == 0 { frost } else { ice });
                    facet(out, a1, b2, a2, mid, if k % 2 == 0 { frost } else { ice });
                    facet(
                        out,
                        at(ring[k], yb),
                        at(ring[kk], yb),
                        [mid[0], yb, mid[2]],
                        [mid[0], ya, mid[2]],
                        ice,
                    );
                    facet(
                        out,
                        at(ring[k], ya),
                        at(ring[kk], ya),
                        [mid[0], ya, mid[2]],
                        [mid[0], yb, mid[2]],
                        ice,
                    );
                }
            }
        }
    }
    // The frame, while the cliff it stands in is there.
    let (x0, x1) = (g.min[0] as f32, g.max[0] as f32);
    let front = g.max[2] as f32;
    let top = 39.0f32;
    let stone = paint(mul(ROCK, 1.25), mat::CLIFFSTONE);
    let snow = paint([0.86, 0.89, 0.92], mat::SNOW);
    let solid_at = |x: i32| materials_solid(world, [x, 37, g.max[2] - 1]);
    if solid_at(g.min[0] - 1) {
        cbox(
            out,
            [x0 - 0.35, 37.0, front],
            [X, Y, Z],
            [0.45, 2.0, 0.45],
            0.12,
            stone,
        );
    }
    if solid_at(g.max[0]) {
        cbox(
            out,
            [x1 + 0.35, 37.0, front],
            [X, Y, Z],
            [0.45, 2.0, 0.45],
            0.12,
            stone,
        );
    }
    if materials_solid(world, [g.min[0], 39, g.max[2] - 1]) {
        cbox(
            out,
            [(x0 + x1) / 2.0, top + 0.4, front + 0.02],
            [X, Y, Z],
            [(x1 - x0) / 2.0 + 1.0, 0.42, 0.5],
            0.14,
            stone,
        );
        cbox(
            out,
            [(x0 + x1) / 2.0, top + 0.86, front + 0.06],
            [X, Y, Z],
            [(x1 - x0) / 2.0 + 0.95, 0.08, 0.5],
            0.06,
            snow,
        );
        // Icicles hang from the lintel while the ice stands: a carved gate
        // in the cliff (B4 round 3: a pointed stone gable over it read as
        // a hut roof, so the lintel stays flush with the rock).
        let iced = (g.min[0]..g.max[0])
            .any(|x| ice_plug([x, 38, g.max[2] - 1], world.get([x, 38, g.max[2] - 1]).mat));
        if iced {
            for k in 0..9 {
                let x = x0 - 0.6 + (x1 - x0 + 1.2) * (k as f32 + 0.5) / 9.0;
                let len = 0.25 + 0.45 * hash([k, 1, 0], 77);
                let b = [x, top - 0.02, front + 0.2 + 0.2 * hash([k, 2, 0], 78)];
                let r = 0.06 + 0.04 * hash([k, 3, 0], 79);
                let tip = add(b, [0.0, -len, 0.0]);
                let pts: Vec<V3> = (0..4)
                    .map(|i| {
                        let t = i as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
                        [b[0] + r * t.cos(), b[1], b[2] + r * t.sin()]
                    })
                    .collect();
                for i in 0..4 {
                    facet(out, pts[i], pts[(i + 1) % 4], tip, b, frost);
                }
            }
        }
    }
    // Ridged rock buttresses flanking the gate, frost on their tops.
    let rock = paint(mul(ROCK, 0.95), mat::CLIFFSTONE);
    for (k, bx) in [(0, x0 - 2.1), (1, x1 + 2.1)] {
        let base = [bx, 34.6, front - 0.2];
        if !materials_solid(world, [base[0].floor() as i32, 35, g.max[2] - 1]) {
            continue;
        }
        let mut y = base[1];
        for c in 0..4 {
            let h = 1.8 + 0.6 * hash([k, c, 0], 85);
            let r0 = 1.25 - 0.18 * c as f32;
            let r1 = r0 - 0.15;
            let ring = |r: f32, yy: f32, salt: u32| -> Vec<V3> {
                (0..7)
                    .map(|i| {
                        let t =
                            (i as f32 + 0.3 * hash([k, c, i], salt)) / 7.0 * std::f32::consts::TAU;
                        let rr = r * (0.82 + 0.36 * hash([k, c, i], salt + 1));
                        [base[0] + rr * t.cos(), yy, base[2] + rr * t.sin() * 0.7]
                    })
                    .collect()
            };
            let lo = ring(r0, y, 86);
            let hi = ring(r1, y + h, 88);
            let inside = [base[0], y + h / 2.0, base[2]];
            for i in 0..7 {
                let j = (i + 1) % 7;
                facet(out, lo[i], lo[j], hi[j], inside, rock);
                facet(out, lo[i], hi[j], hi[i], inside, rock);
                let cap = if c == 3 { snow } else { rock };
                facet(
                    out,
                    hi[i],
                    hi[j],
                    [base[0], y + h + 0.15, base[2]],
                    inside,
                    cap,
                );
            }
            y += h - 0.1;
        }
    }
}

/// Pools of dark standing water on the methane hollow's mud floor, each
/// side of the boardwalk and between the vents (x, z, half sizes along x
/// and z). Looks only: no water cells. B4 round 3: bigger, glossy, with a
/// shallow-to-deep ramp (art bible water #157080 to #08304D).
const PUDDLES: [(f32, f32, f32, f32); 6] = [
    (189.6, 99.5, 1.35, 0.55),
    (193.1, 99.45, 1.4, 0.5),
    (189.9, 102.5, 1.5, 0.55),
    (193.4, 102.55, 1.25, 0.5),
    (195.3, 97.6, 0.6, 0.5),
    (187.7, 104.35, 0.7, 0.45),
];

/// Water deep and shallow (#08304D, #157080) and wet mud, linear. A
/// negative glow marks a wet, glossy surface for the smooth shader.
const WATER_DEEP: V3 = [0.0024, 0.030, 0.074];
const WATER_SHALLOW: V3 = [0.008, 0.162, 0.216];
const WET: f32 = -1.0;

/// One pool: a ring of wet mud, a shallow band and a dark deep middle, a
/// hair above the mud.
fn puddle(out: &mut MeshData, world: &World, c: [f32; 2], r: [f32; 2]) {
    let cell = [c[0].floor() as i32, 27, c[1].floor() as i32];
    if world.get(cell).mat != mat::MUD || world.get([cell[0], 28, cell[2]]).mat != mat::AIR {
        return;
    }
    let wet = |rgb: V3, m: u8| Paint {
        rgb,
        glow: WET,
        mat: m,
    };
    let deep = wet(WATER_DEEP, mat::WATER);
    let shallow = wet(mix(WATER_DEEP, WATER_SHALLOW, 0.75), mat::WATER);
    let rim = wet([0.05, 0.045, 0.022], mat::MUD);
    let n: usize = 14;
    let ring = |k: f32, y: f32| -> Vec<V3> {
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * std::f32::consts::TAU;
                let w = 1.0 + 0.16 * (hash([cell[0], i as i32, cell[2]], 40) - 0.5);
                [
                    c[0] + r[0] * k * w * t.cos(),
                    y,
                    c[1] + r[1] * k * w * t.sin(),
                ]
            })
            .collect()
    };
    let y = 28.0;
    let outer = ring(1.3, y + 0.01);
    let mid = ring(1.0, y + 0.02);
    let inner = ring(0.6, y + 0.025);
    let centre = [c[0], y + 0.025, c[1]];
    let below = [c[0], y - 1.0, c[1]];
    for k in 0..n {
        let j = (k + 1) % n;
        facet(out, outer[k], outer[j], mid[j], below, rim);
        facet(out, outer[k], mid[j], mid[k], below, rim);
        facet(out, mid[k], mid[j], inner[j], below, shallow);
        facet(out, mid[k], inner[j], inner[k], below, shallow);
        facet(out, inner[k], inner[j], centre, below, deep);
    }
}

/// A board of the hollow's boardwalk (B4 round 3: the path read as an
/// untextured slab): four planks across the walk per metre with dark gaps
/// and nail heads, on a stringer each side and a post at every other
/// metre. Looks only; the plank cell is the walkway.
fn boardwalk(out: &mut MeshData, world: &World, p: [i32; 3]) {
    let top = p[1] as f32 + 1.0;
    let gap = paint([0.03, 0.025, 0.02], mat::PLANKS);
    let nail = paint([0.06, 0.06, 0.065], mat::IRON);
    let timber = paint(mul(TIMBER, 0.8), mat::WOOD);
    let fx = p[0] as f32;
    let fz = p[2] as f32;
    // The dark bed under the boards shows through the gaps.
    cbox(
        out,
        [fx + 0.5, top - 0.2, fz + 0.5],
        [X, Y, Z],
        [0.5, 0.08, 0.5],
        0.0,
        gap,
    );
    for k in 0..4 {
        let h = hash([p[0], k, p[2]], 90);
        let rgb = mul(
            mix(TIMBER, [0.42, 0.28, 0.15], 0.35 + 0.3 * h),
            0.85 + 0.25 * h,
        );
        let board = paint(rgb, mat::PLANKS);
        let cx = fx + 0.125 + 0.25 * k as f32;
        let warp = 0.012 * (hash([p[0], k, p[2]], 91) - 0.5);
        cbox(
            out,
            [cx, top - 0.05 + warp, fz + 0.5],
            [X, Y, Z],
            [0.112, 0.05, 0.5],
            0.02,
            board,
        );
        for nz in [0.15f32, 0.85] {
            cbox(
                out,
                [cx, top + 0.002, fz + nz],
                [X, Y, Z],
                [0.018, 0.004, 0.018],
                0.0,
                nail,
            );
        }
    }
    // Stringers on the open sides, and a post every other metre.
    for (sz, dz) in [(-1i32, 0.0f32), (1, 1.0)] {
        let q = [p[0], p[1], p[2] + sz];
        if world.get(q).mat == mat::PLANKS {
            continue;
        }
        cbox(
            out,
            [fx + 0.5, top - 0.3, fz + dz],
            [X, Y, Z],
            [0.52, 0.16, 0.07],
            0.03,
            timber,
        );
        if p[0] % 2 == 0 {
            column(
                out,
                [fx + 0.5, top - 1.3, fz + dz],
                0.1,
                0.09,
                1.25,
                6,
                timber,
            );
        }
    }
}

/// Reeds: a clump of thin blades leaning out from `base`, some with a
/// cattail head.
fn reeds(out: &mut MeshData, base: V3, seed: [i32; 3]) {
    let green = [0.12, 0.17, 0.035];
    let dry = [0.32, 0.27, 0.09];
    let n = 6 + (hash(seed, 81) * 5.0) as i32;
    for k in 0..n {
        let a = hash(seed, 100 + k as u32) * std::f32::consts::TAU;
        let rr = 0.25 * hash(seed, 120 + k as u32);
        let lean = 0.08 + 0.22 * hash(seed, 140 + k as u32);
        let len = 0.9 + 0.9 * hash(seed, 160 + k as u32);
        let b = [
            base[0] + rr * a.cos(),
            base[1] - 0.05,
            base[2] + rr * a.sin(),
        ];
        let dir = norm([lean * a.cos(), 1.0, lean * a.sin()]);
        let tip = add(b, mul(dir, len));
        let w = 0.035;
        let pts: Vec<V3> = (0..3)
            .map(|i| {
                let t = a + i as f32 * std::f32::consts::TAU / 3.0;
                [b[0] + w * t.cos(), b[1], b[2] + w * t.sin()]
            })
            .collect();
        let c = paint(
            mix(green, dry, hash(seed, 180 + k as u32) * 0.6),
            mat::LEAVES,
        );
        let mid = mul(add(add(pts[0], pts[1]), add(pts[2], tip)), 0.25);
        for i in 0..3 {
            facet(out, pts[i], pts[(i + 1) % 3], tip, mid, c);
        }
        if k % 3 == 0 {
            let head = add(b, mul(dir, len * 0.72));
            column(
                out,
                head,
                0.05,
                0.05,
                0.22,
                5,
                paint([0.11, 0.05, 0.02], mat::LEAVES),
            );
        }
    }
}

/// Reed clumps round the methane hollow: along its inner walls and on
/// the marsh ground round it, drawn by the chunk holding each clump.
fn hollow_reeds(out: &mut MeshData, world: &World, lo: [i32; 3], hi: [i32; 3]) {
    let s = stamp::HOLLOW;
    let (ax, bx) = (lo[0].max(s.min[0] - 8), hi[0].min(s.max[0] + 8));
    let (az, bz) = (lo[2].max(s.min[2] - 8), hi[2].min(s.max[2] + 8));
    for z in az..bz {
        for x in ax..bx {
            let inside = x > s.min[0] && x < s.max[0] - 1 && z > s.min[2] && z < s.max[2] - 1;
            let by_wall = inside
                && (x == s.min[0] + 1
                    || x == s.max[0] - 2
                    || z == s.min[2] + 1
                    || z == s.max[2] - 2);
            let outside = !(s.min[0]..s.max[0]).contains(&x) || !(s.min[2]..s.max[2]).contains(&z);
            let h = hash([x, 0, z], 80);
            let want = (by_wall && h > 0.5) || (outside && h > 0.8);
            if !want || (100..=101).contains(&z) {
                continue;
            }
            // The ground's top in this chunk.
            let mut y = hi[1] - 1;
            let mut found = None;
            while y >= lo[1] {
                let m = world.get([x, y, z]).mat;
                if m != mat::AIR {
                    if matches!(m, mat::MUD | mat::GRASS | mat::DIRT)
                        && world.get([x, y + 1, z]).mat == mat::AIR
                        && y + 1 < hi[1]
                    {
                        found = Some(y + 1);
                    }
                    break;
                }
                y -= 1;
            }
            let Some(fy) = found else { continue };
            let jx = 0.3 * (hash([x, 1, z], 82) - 0.5);
            let jz = 0.3 * (hash([x, 2, z], 83) - 0.5);
            reeds(
                out,
                [x as f32 + 0.5 + jx, fy as f32, z as f32 + 0.5 + jz],
                [x, fy, z],
            );
        }
    }
}

fn materials_solid(world: &World, p: [i32; 3]) -> bool {
    crate::materials::flags(world.get(p).mat).solid
}

/// A Marsh gas vent: a rough rock chimney (B4 round 3: the low mounds read
/// as cones) of three jagged courses round a dark, scorched throat, on
/// the vent cell's top, with loose stones round its foot (looks only; the
/// cell is unchanged). The client's flame rises out of its top.
fn vent_mound(out: &mut MeshData, world: &World, p: [i32; 3]) {
    if world.get([p[0], p[1] + 1, p[2]]).mat != mat::AIR {
        return;
    }
    let base = [p[0] as f32 + 0.5, p[1] as f32 + 1.0, p[2] as f32 + 0.5];
    let rock = paint(mix(ROCK, [0.17, 0.15, 0.08], 0.35), mat::GAS_VENT);
    let rock2 = paint(mix(ROCK, [0.10, 0.09, 0.06], 0.5), mat::GAS_VENT);
    let crust = paint([0.40, 0.32, 0.09], mat::GAS_VENT);
    let throat = paint([0.02, 0.018, 0.015], mat::GAS_VENT);
    // Jagged courses: each a ring of 7 with its own radius per corner.
    let course = |out: &mut MeshData, y0: f32, y1: f32, r0: f32, r1: f32, salt: u32, pt: Paint| {
        let ring = |r: f32, y: f32, s2: u32| -> Vec<V3> {
            (0..7)
                .map(|k| {
                    let t = (k as f32 + 0.3 * hash(p, salt + k)) * std::f32::consts::TAU / 7.0;
                    let rr = r * (0.85 + 0.3 * hash(p, s2 + k));
                    [base[0] + rr * t.cos(), y, base[2] + rr * t.sin()]
                })
                .collect()
        };
        let lo = ring(r0, base[1] + y0, salt + 10);
        let hi = ring(r1, base[1] + y1, salt + 20);
        let inside = [base[0], base[1] + (y0 + y1) / 2.0, base[2]];
        for k in 0..7 {
            let j = (k + 1) % 7;
            facet(out, lo[k], lo[j], hi[j], inside, pt);
            facet(out, lo[k], hi[j], hi[k], inside, pt);
            facet(
                out,
                [base[0], base[1] + y1, base[2]],
                hi[k],
                hi[j],
                [base[0], base[1] + y0, base[2]],
                pt,
            );
        }
    };
    course(out, -0.08, 0.36, 0.66, 0.5, 200, rock);
    course(out, 0.3, 0.62, 0.48, 0.38, 230, rock2);
    course(out, 0.58, 0.84, 0.38, 0.33, 260, rock);
    column(out, add(base, [0.0, 0.82, 0.0]), 0.32, 0.27, 0.08, 7, crust);
    column(out, add(base, [0.0, 0.86, 0.0]), 0.2, 0.2, 0.06, 7, throat);
    // Loose stones round its foot.
    for k in 0..4 {
        let t = (k as f32 + hash(p, 20 + k)) * std::f32::consts::TAU / 4.0;
        let r = 0.78 + 0.14 * hash(p, 30 + k);
        let c = add(base, [r * t.cos(), -0.04, r * t.sin()]);
        column(out, c, 0.14, 0.07, 0.14 + 0.06 * hash(p, 40 + k), 5, rock2);
    }
}

/// The rock a crystal cluster grows from (B4 round 3: crystals read as
/// floating shards): a low, jagged collar of dark rock on the cell's open
/// face, round the cluster's foot.
fn crystal_collar(out: &mut MeshData, world: &World, p: [i32; 3]) {
    let Some(d) = crate::props::crystal_side(world, p) else {
        return;
    };
    let dir = [d[0] as f32, d[1] as f32, d[2] as f32];
    let c = add(
        [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5],
        mul(dir, 0.36),
    );
    // Two axes across the face.
    let u = if d[1] != 0 { X } else { Y };
    let v = cross(dir, u);
    // Rock lit from within by its crystals: a faint teal core and rune-like
    // veins, so the cluster's foot shows in the dark and the shards read as
    // set in rock, not floating (B4 round 3 judges).
    let rock = Paint {
        rgb: [0.05, 0.16, 0.15],
        glow: 0.75,
        mat: mat::STONE,
    };
    let rock2 = Paint {
        rgb: [0.07, 0.2, 0.18],
        glow: 0.75,
        mat: mat::STONE,
    };
    let n = 8;
    let ring = |r: f32, h: f32, salt: u32| -> Vec<V3> {
        (0..n)
            .map(|k| {
                let t = (k as f32 + 0.4 * hash(p, salt + k)) / n as f32 * std::f32::consts::TAU;
                let rr = r * (0.8 + 0.4 * hash(p, salt + 20 + k));
                add(
                    add(c, mul(dir, h)),
                    add(mul(u, rr * t.cos()), mul(v, rr * t.sin())),
                )
            })
            .collect()
    };
    let base = ring(0.78, 0.0, 300);
    let top = ring(0.4, 0.28 + 0.1 * hash(p, 330), 340);
    let inside = sub(c, mul(dir, 0.3));
    // A boss of rock under the collar, back into the cell and the wall
    // behind it, so a cluster on a thin or rounded-off wall still sits on
    // rock instead of floating (B4 round 3 judges).
    let root = ring(0.62, -0.95, 360);
    let tail = add(c, mul(dir, -1.35));
    for k in 0..n as usize {
        let j = (k + 1) % n as usize;
        let pt = if k % 2 == 0 { rock } else { rock2 };
        facet(out, base[k], base[j], top[j], inside, pt);
        facet(out, base[k], top[j], top[k], inside, pt);
        facet(out, top[k], top[j], add(c, mul(dir, 0.18)), inside, rock);
        facet(out, root[k], root[j], base[j], inside, pt);
        facet(out, root[k], base[j], base[k], inside, pt);
        facet(out, root[k], root[j], tail, inside, rock2);
    }
}

/// A placed torch: a timber stake with a pitch-wrapped head and a small
/// glowing flame, instead of a whole glowing block (B4 round 3 judges: the
/// cave's torch read as a blown-out lump). Its light is the client's.
fn torch(out: &mut MeshData, p: [i32; 3]) {
    let b = [p[0] as f32 + 0.5, p[1] as f32, p[2] as f32 + 0.5];
    let stake = paint(mul(TIMBER, 0.9), mat::WOOD);
    let pitch = paint([0.04, 0.03, 0.025], mat::WOOD);
    let flame = Paint {
        rgb: [1.0, 0.45, 0.08],
        glow: 1.0,
        mat: mat::TORCH,
    };
    let core = Paint {
        rgb: [1.0, 0.75, 0.3],
        glow: 1.0,
        mat: mat::TORCH,
    };
    column(out, b, 0.08, 0.06, 0.62, 6, stake);
    column(out, add(b, [0.0, 0.6, 0.0]), 0.1, 0.11, 0.14, 6, pitch);
    column(out, add(b, [0.0, 0.74, 0.0]), 0.11, 0.0, 0.42, 6, flame);
    column(out, add(b, [0.0, 0.74, 0.0]), 0.06, 0.0, 0.26, 5, core);
}

/// An iron cell of the electrolysis rig (its battery terminals each side
/// of the glass cell): a crisp, dark iron block with brass bands and a
/// terminal post, instead of a soft masonry lump (B4 round 3).
fn rig_cell(out: &mut MeshData, p: [i32; 3]) {
    let c = [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5];
    let iron = paint([0.06, 0.065, 0.075], mat::IRON);
    let brass = paint([0.42, 0.27, 0.06], mat::IRON);
    cbox(
        out,
        add(c, [0.0, -0.06, 0.0]),
        [X, Y, Z],
        [0.44, 0.44, 0.42],
        0.05,
        iron,
    );
    for y in [-0.3f32, 0.22] {
        cbox(
            out,
            add(c, [0.0, y, 0.0]),
            [X, Y, Z],
            [0.455, 0.035, 0.435],
            0.01,
            brass,
        );
    }
    column(out, add(c, [0.0, 0.38, 0.0]), 0.08, 0.07, 0.16, 8, brass);
}

/// The Frozen Fields' trail (B4 round 3: no clear way from the ford to
/// the ice gate): snow-capped timber stakes with a faded cloth band, in
/// pairs either side of the way from the ford's north bank to the gate.
/// Each stake stands on the ground under it, drawn by the chunk that
/// holds its foot; looks only.
const TRAIL: [[f32; 2]; 6] = [
    [128.0, 145.0],
    [118.0, 136.0],
    [104.0, 126.0],
    [88.0, 116.0],
    [74.0, 108.0],
    [64.0, 101.0],
];

fn trail(out: &mut MeshData, world: &World, lo: [i32; 3], hi: [i32; 3]) {
    let timber = paint(mul(TIMBER, 0.85), mat::WOOD);
    let snow = paint([0.86, 0.89, 0.92], mat::SNOW);
    let cloth = paint([0.42, 0.10, 0.06], mat::WOOD);
    for k in 0..TRAIL.len() - 1 {
        let (a, b) = (TRAIL[k], TRAIL[k + 1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
        let n = (len / 7.0).ceil() as i32;
        let side = [-d[1] / len, d[0] / len];
        for i in 0..n {
            let t = i as f32 / n as f32;
            for sg in [-1.0f32, 1.0] {
                let x = a[0] + d[0] * t + side[0] * sg * 1.8;
                let z = a[1] + d[1] * t + side[1] * sg * 1.8;
                let (cx, cz) = (x.floor() as i32, z.floor() as i32);
                if cx < lo[0] || cx >= hi[0] || cz < lo[2] || cz >= hi[2] {
                    continue;
                }
                let mut y = 70;
                while y > 20 {
                    let m = world.get([cx, y, cz]).mat;
                    if m != mat::AIR && m != mat::LEAVES {
                        break;
                    }
                    y -= 1;
                }
                let m = world.get([cx, y, cz]).mat;
                let fy = y + 1;
                if fy < lo[1]
                    || fy >= hi[1]
                    || !matches!(m, mat::SNOW | mat::DIRT | mat::GRASS | mat::STONE)
                {
                    continue;
                }
                let lean = 0.04 * (hash([cx, k as i32, cz], 95) - 0.5);
                let base = [x + lean, fy as f32 - 0.25, z];
                let h = 1.5 + 0.2 * hash([cx, 1, cz], 96);
                column(out, base, 0.08, 0.065, h, 6, timber);
                column(out, add(base, [0.0, h, 0.0]), 0.1, 0.03, 0.12, 6, snow);
                column(
                    out,
                    add(base, [0.0, h - 0.42, 0.0]),
                    0.095,
                    0.09,
                    0.16,
                    6,
                    cloth,
                );
            }
        }
    }
}

/// The caches' built towers and walls (the client's `powers::SITES` that
/// are built of brick): centre column and half width in blocks. The
/// client's tests pin these to its sites.
pub const CACHE_TOWERS: [(i32, i32, i32); 3] = [(134, 196, 1), (112, 190, 2), (176, 128, 1)];

/// A cache tower's trim (B4 round 3: the plaza's towers read as sharp clay
/// boxes): a stone plinth, round clay pilasters on the corners, a timber
/// string course and a stepped, overhanging cornice. Looks only, drawn
/// while its corner column stands, by the chunk that holds its south-west
/// foot.
fn cache_trim(
    out: &mut MeshData,
    world: &World,
    (cx, cz, r): (i32, i32, i32),
    lo: [i32; 3],
    hi: [i32; 3],
) {
    let (x0, z0) = (cx - r, cz - r);
    if x0 < lo[0] || x0 >= hi[0] || z0 < lo[2] || z0 >= hi[2] {
        return;
    }
    // The corner column's brick run.
    let mut y = 20;
    while y < 70 && world.get([x0, y, z0]).mat != mat::BRICK {
        y += 1;
    }
    if y >= 70 || y < lo[1] || y >= hi[1] {
        return;
    }
    let y0 = y;
    while world.get([x0, y, z0]).mat == mat::BRICK {
        y += 1;
    }
    let y1 = y;
    if y1 - y0 < 3 {
        return;
    }
    let corners_up = [(x0, z0), (cx + r, z0), (x0, cz + r), (cx + r, cz + r)]
        .iter()
        .all(|&(x, z)| (y0..y1).all(|yy| world.get([x, yy, z]).mat == mat::BRICK));
    if !corners_up {
        return;
    }
    let c = [cx as f32 + 0.5, 0.0, cz as f32 + 0.5];
    let half = r as f32 + 0.5;
    let stone = paint(mul(ROCK, 1.15), mat::STONE);
    let plaster = paint(mul(CLAY, 0.9), mat::BRICK);
    let cap = paint(mul(CLAY, 0.78), mat::BRICK);
    let (fy0, fy1) = (y0 as f32, y1 as f32);
    cbox(
        out,
        [c[0], fy0 + 0.18, c[2]],
        [X, Y, Z],
        [half + 0.16, 0.2, half + 0.16],
        0.06,
        stone,
    );
    cbox(
        out,
        [c[0], fy1 - 0.12, c[2]],
        [X, Y, Z],
        [half + 0.12, 0.12, half + 0.12],
        0.05,
        cap,
    );
    cbox(
        out,
        [c[0], fy1 + 0.06, c[2]],
        [X, Y, Z],
        [half + 0.24, 0.08, half + 0.24],
        0.04,
        cap,
    );
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let b = [
            c[0] + sx * (half - 0.45),
            fy0 + 0.3,
            c[2] + sz * (half - 0.45),
        ];
        column(out, b, 0.62, 0.55, fy1 - fy0 - 0.45, 10, plaster);
    }
    tower_cottage(out, c, half, r, fy0, fy1);
}

/// The village kit on a cache tower (B4 round 3 judges: the towers read as
/// flat-sided boxes with plank strips beside the bevelled, roofed
/// cottages): a tiled pent roof skirting the tower over its string course,
/// and a framed, shuttered window on each face below it. Looks only.
fn tower_cottage(out: &mut MeshData, c: V3, half: f32, r: i32, fy0: f32, fy1: f32) {
    let tile = paint([0.42, 0.085, 0.04], mat::PLANKS);
    let tile_dark = paint([0.3, 0.06, 0.03], mat::PLANKS);
    let timber = paint(mul(TIMBER, 0.9), mat::WOOD);
    let dark = paint([0.02, 0.025, 0.035], mat::WOOD);
    let shutter = paint(mix([0.06, 0.11, 0.16], CLAY, 0.15), mat::WOOD);
    let mid = (fy0 + fy1) / 2.0 + 0.5;
    let (out_by, rise) = (0.78f32, 0.42f32);
    // The skirt: one sloped, tiled board per side, its corners lapped.
    for (n, along) in [(X, Z), (mul(X, -1.0), Z), (Z, X), (mul(Z, -1.0), X)] {
        let slope = norm(add(mul(n, out_by), [0.0, -rise, 0.0]));
        let up = norm(cross(along, slope));
        let up = if up[1] < 0.0 { mul(up, -1.0) } else { up };
        let len = (out_by * out_by + rise * rise).sqrt();
        let base = add(c, add(mul(n, half + out_by / 2.0), [0.0, mid + 0.12, 0.0]));
        cbox(
            out,
            base,
            [slope, up, along],
            [len / 2.0 + 0.04, 0.07, half + out_by + 0.02],
            0.03,
            tile,
        );
        // Tile courses: three ridges down the slope, and a dark drip edge.
        for k in 0..3 {
            let t = -0.3 + 0.3 * k as f32;
            cbox(
                out,
                add(add(base, mul(slope, t * len)), mul(up, 0.075)),
                [slope, up, along],
                [0.035, 0.025, half + out_by - 0.05],
                0.01,
                tile_dark,
            );
        }
        cbox(
            out,
            add(add(base, mul(slope, len / 2.0)), mul(up, -0.02)),
            [slope, up, along],
            [0.05, 0.09, half + out_by + 0.03],
            0.02,
            tile_dark,
        );
        // A window under the skirt, if the wall below it is tall enough.
        let wy = mid - 1.2;
        if wy - 0.45 < fy0 + 0.45 {
            continue;
        }
        let w = if r >= 2 { 0.48 } else { 0.3 };
        let face = add(c, add(mul(n, half), [0.0, wy, 0.0]));
        cbox(
            out,
            add(face, mul(n, -0.02)),
            [along, Y, n],
            [w, 0.42, 0.04],
            0.0,
            dark,
        );
        for dy in [-0.46f32, 0.46] {
            cbox(
                out,
                add(face, add(mul(n, 0.05), [0.0, dy, 0.0])),
                [along, Y, n],
                [w + 0.1, 0.06, 0.09],
                0.02,
                timber,
            );
        }
        for sx in [-1.0f32, 1.0] {
            cbox(
                out,
                add(face, add(mul(n, 0.04), mul(along, sx * (w + 0.04)))),
                [along, Y, n],
                [0.06, 0.42, 0.08],
                0.02,
                timber,
            );
            if r >= 2 {
                cbox(
                    out,
                    add(face, add(mul(n, 0.05), mul(along, sx * (w + 0.36)))),
                    [along, Y, n],
                    [0.25, 0.42, 0.035],
                    0.02,
                    shutter,
                );
            }
        }
        cbox(
            out,
            add(face, mul(n, 0.03)),
            [along, Y, n],
            [0.03, 0.4, 0.03],
            0.01,
            timber,
        );
    }
}

// ---------------------------------------------------------------------------
// A region's models.
// ---------------------------------------------------------------------------

/// Whether chunk-aligned region `[lo, hi)` is one chunk of a single
/// material that is not built (nothing to draw).
fn plainly_empty(world: &World, lo: [i32; 3], hi: [i32; 3]) -> bool {
    if hi != lo.map(|v| v + CHUNK) || lo.iter().any(|v| v.rem_euclid(CHUNK) != 0) {
        return false;
    }
    match World::chunk_index(lo) {
        Some(c) => match world.chunk(c) {
            Chunk::Uniform(cell) => !modelled(lo, cell.mat),
            // A cheap first look: no built material and no wood at all.
            Chunk::Array(cells) => !cells.iter().any(|c| {
                is_built(c.mat)
                    || matches!(
                        c.mat,
                        mat::WOOD | mat::ICE | mat::GAS_VENT | mat::CRYSTAL | mat::TORCH
                    )
            }),
        },
        None => true,
    }
}

/// The built models owned by the cells in `[lo, hi)`.
pub fn models(world: &World, lo: [i32; 3], hi: [i32; 3]) -> MeshData {
    let mut out = MeshData::default();
    let hs = stamp::HOLLOW;
    let near_hollow = lo[0] < hs.max[0] + 8
        && hi[0] > hs.min[0] - 8
        && lo[2] < hs.max[2] + 8
        && hi[2] > hs.min[2] - 8
        && lo[1] < 40
        && hi[1] > 24;
    if near_hollow {
        hollow_reeds(&mut out, world, lo, hi);
    }
    if lo[0] < 132 && hi[0] > 60 && lo[2] < 150 && hi[2] > 96 && hi[1] > 30 && lo[1] < 72 {
        trail(&mut out, world, lo, hi);
    }
    if plainly_empty(world, lo, hi) {
        return out;
    }
    for t in CACHE_TOWERS {
        cache_trim(&mut out, world, t, lo, hi);
    }
    for z in lo[2]..hi[2] {
        for y in lo[1]..hi[1] {
            for x in lo[0]..hi[0] {
                let p = [x, y, z];
                let m = world.get(p).mat;
                if m == mat::GAS_VENT {
                    vent_mound(&mut out, world, p);
                    continue;
                }
                if m == mat::CRYSTAL {
                    crystal_collar(&mut out, world, p);
                    continue;
                }
                if m == mat::TORCH {
                    torch(&mut out, p);
                    continue;
                }
                if m == mat::IRON && stamp::RIG.contains(p) {
                    rig_cell(&mut out, p);
                    continue;
                }
                if !modelled(p, m) || under_dome(p, m) || kiln_door(p, m) {
                    continue;
                }
                if m == mat::PLANKS && stamp::HOLLOW.contains(p) {
                    boardwalk(&mut out, world, p);
                    continue;
                }
                match m {
                    mat::BENCH => formula_bench(&mut out, p),
                    mat::INVENTION_BENCH => invention_bench(&mut out, p),
                    _ => match roof_of(p) {
                        // A whole roof slice is drawn by its roof; what is
                        // left of a broken one shows as charred blocks.
                        Some(h) if slice_whole(world, h, x - h.min[0]) => {}
                        _ => block(&mut out, world, p, m),
                    },
                }
            }
        }
    }
    for h in gen::HOUSES.iter() {
        for i in 0..(h.max[0] - h.min[0]) {
            let owner = h.roof_cells(i)[0];
            let inside = (0..3).all(|a| owner[a] >= lo[a] && owner[a] < hi[a]);
            if inside && slice_whole(world, h, i) {
                roof_slice(&mut out, world, h, i);
            }
        }
    }
    for h in gen::HOUSES.iter() {
        if (0..3).all(|a| h.min[a] >= lo[a] && h.min[a] < hi[a]) {
            house_trim(&mut out, world, h);
        }
    }
    let g = stamp::ICE_GATE;
    if (0..3).all(|a| g.min[a] >= lo[a] && g.min[a] < hi[a]) {
        ice_gate(&mut out, world);
    }
    for &(x, z, rx, rz) in PUDDLES.iter() {
        let c = [x.floor() as i32, 27, z.floor() as i32];
        if (0..3).all(|a| c[a] >= lo[a] && c[a] < hi[a]) {
            puddle(&mut out, world, [x, z], [rx, rz]);
        }
    }

    let wl = gen::WELL;
    if (0..3).all(|a| wl[a] >= lo[a] && wl[a] < hi[a]) {
        well(&mut out, world);
    }
    let ws = [stamp::WORKSHOP.min[0], 38, stamp::WORKSHOP.min[2]];
    if (0..3).all(|a| ws[a] >= lo[a] && ws[a] < hi[a]) {
        workshop_beams(&mut out, world);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> World {
        let mut w = World::new();
        for z in 0..64 {
            for x in 0..64 {
                for y in 30..35 {
                    w.set([x, y, z], Cell::new(mat::GRASS));
                }
            }
        }
        w
    }

    fn bounds(m: &MeshData) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &m.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn a_wall_is_one_rounded_solid_and_a_dug_cell_is_a_hole() {
        let mut w = flat();
        for x in 10..16 {
            for y in 35..38 {
                w.set([x, y, 20], Cell::new(mat::BRICK));
            }
        }
        let m = models(&w, [0, 0, 0], [64, 64, 64]);
        let (lo, hi) = bounds(&m);
        assert!((lo[0] - 10.0).abs() < 1e-4 && (hi[0] - 16.0).abs() < 1e-4);
        assert!((hi[1] - 38.0).abs() < 1e-4, "top at the block tops");
        // No face between two bricks: the wall's front is drawn once per
        // block, and no vertex sits inside the wall's thickness except on
        // its rounded rim.
        let inner = m
            .positions
            .iter()
            .filter(|p| p[2] > 20.0 + BEVEL + 1e-3 && p[2] < 21.0 - BEVEL - 1e-3)
            .filter(|p| p[1] < 38.0 - BEVEL - 1e-3 && p[1] > 35.0 + 1e-3)
            .filter(|p| p[0] > 10.0 + BEVEL + 1e-3 && p[0] < 16.0 - BEVEL - 1e-3)
            .count();
        assert_eq!(inner, 0, "no faces inside the wall");
        // Rounded: some normals are diagonal.
        assert!(m
            .normals
            .iter()
            .any(|n| n[1] > 0.3 && n[1] < 0.95 && n[2].abs() > 0.3));
        // Dig one out of the middle: the hole's ceiling shows.
        let ceiling = |m: &MeshData| {
            m.positions
                .iter()
                .zip(&m.normals)
                .filter(|(p, n)| {
                    n[1] < -0.99 && (p[1] - 37.0).abs() < 1e-4 && p[0] > 11.99 && p[0] < 13.01
                })
                .count()
        };
        assert_eq!(ceiling(&m), 0);
        w.set([12, 36, 20], Cell::AIR);
        let m2 = models(&w, [0, 0, 0], [64, 64, 64]);
        assert!(ceiling(&m2) > 0, "the hole is drawn");
    }

    #[test]
    fn a_lone_block_is_closed_rounded_and_faces_out() {
        let mut w = World::new();
        w.set([20, 35, 20], Cell::new(mat::BRICK));
        let m = models(&w, [0, 0, 0], [64, 64, 64]);
        let c = [20.5, 35.5, 20.5];
        let mut area = 0.0;
        for t in m.indices.chunks(3) {
            let [a, b, cc] = [0, 1, 2].map(|k| m.positions[t[k] as usize]);
            let n = cross(sub(b, a), sub(cc, a));
            let mid = mul(add(add(a, b), cc), 1.0 / 3.0);
            assert!(dot(n, sub(mid, c)) > 0.0, "{a:?} {b:?} {cc:?}");
            area += dot(n, n).sqrt() / 2.0;
        }
        // A rounded cube: 6 m2 less what the rounding takes off (about 1.6).
        assert!(area > 4.1 && area < 4.6, "{area}");
    }

    #[test]
    fn a_house_roof_burns_a_slice_at_a_time() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let h = &gen::HOUSES[0];
        let all = models(&w, [0, 0, 0], crate::WORLD);
        let mut w2 = w.clone();
        for q in h.roof_cells(1) {
            w2.set(q, Cell::new(mat::ASH));
        }
        let burnt = models(&w2, [0, 0, 0], crate::WORLD);
        assert_ne!(all.indices.len(), burnt.indices.len());
        assert!(slice_whole(&w, h, 1) && !slice_whole(&w2, h, 1));
    }

    #[test]
    fn the_benches_are_models_inside_their_cell() {
        let mut w = flat();
        w.set([20, 35, 20], Cell::new(mat::BENCH));
        w.set([30, 35, 20], Cell::new(mat::INVENTION_BENCH));
        for (p, glow) in [([20, 35, 20], true), ([30, 35, 20], true)] {
            let m = models(&w, p, p.map(|v| v + 1));
            let (lo, hi) = bounds(&m);
            for k in 0..3 {
                assert!(
                    lo[k] >= p[k] as f32 - 1e-3 && hi[k] <= p[k] as f32 + 1.0 + 1e-3,
                    "{lo:?} {hi:?}"
                );
            }
            assert_eq!(m.uvs.iter().any(|uv| uv[0] > 0.5), glow, "a teal glow");
        }
    }

    #[test]
    fn walls_burnt_beside_fire_are_sooted() {
        let mut w = flat();
        w.set([20, 35, 20], Cell::new(mat::BRICK));
        let clean = block_paint(&w, [20, 35, 20], mat::BRICK).rgb;
        w.set([21, 35, 20], Cell::new(mat::ASH));
        let sooty = block_paint(&w, [20, 35, 20], mat::BRICK).rgb;
        assert!(sooty[0] < clean[0] * 0.6);
    }

    #[test]
    fn the_ice_gate_is_thick_faceted_ice_that_melts_with_its_cells() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let g = stamp::ICE_GATE;
        let ice_verts = |w: &World| -> Vec<V3> {
            let m = models(w, [32, 32, 64], [64, 64, 96]);
            m.positions
                .iter()
                .zip(&m.mats)
                .filter(|(_, &k)| k == mat::ICE)
                .map(|(p, _)| *p)
                .collect()
        };
        let v = ice_verts(&w);
        assert!(!v.is_empty(), "the plug is drawn");
        let (lo, hi) = v
            .iter()
            .fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p[2]), a.1.max(p[2])));
        assert!(hi - lo > 1.0, "real thickness, {lo}..{hi}");
        let mut w2 = w.clone();
        for z in g.min[2]..g.max[2] {
            for y in g.min[1]..g.max[1] {
                for x in g.min[0]..g.max[0] {
                    if w2.get([x, y, z]).mat == mat::ICE {
                        w2.set([x, y, z], Cell::AIR);
                    }
                }
            }
        }
        assert!(ice_verts(&w2).is_empty(), "melted, nothing left");
    }

    #[test]
    fn houses_have_door_leaves_and_window_frames() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let h = &gen::HOUSES[0];
        let mut out = MeshData::default();
        house_trim(&mut out, &w, h);
        // Posts, eaves, the door and at least three windows.
        assert!(out.positions.len() > 1500, "{}", out.positions.len());
        let mut w2 = w.clone();
        w2.set([h.door_x(), h.min[1] + 2, h.min[2]], Cell::AIR);
        let mut less = MeshData::default();
        house_trim(&mut less, &w2, h);
        assert!(
            less.positions.len() < out.positions.len(),
            "no lintel, no door"
        );
    }
}
