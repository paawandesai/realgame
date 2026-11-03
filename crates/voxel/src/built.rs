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
    // Soot: the workshop has been burning (darker higher up), and any
    // block beside fire or ash.
    let mut soot: f32 = 0.0;
    if stamp::WORKSHOP.contains(p) {
        soot = 0.74 + 0.06 * (p[1] - 35) as f32 + 0.1 * hash(p, 5);
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
    let over = 0.35f32;
    let th = 0.16f32;
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
        rgb: mul(
            mix([0.36, 0.12, 0.06], TIMBER, 0.4),
            0.9 + 0.2 * hash([i, h.min[0], h.min[2]], 2),
        ),
        glow: 0.0,
        mat: mat::PLANKS,
    };
    let under = Paint {
        rgb: mul(TIMBER, 0.7),
        glow: 0.0,
        mat: mat::PLANKS,
    };
    let wall = Paint {
        rgb: mul(CLAY, 0.95),
        glow: 0.0,
        mat: mat::BRICK,
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
        // Shingle courses: a lip every third of the slope.
        for k in 1..3 {
            let t = k as f32 / 3.0;
            let a = mix(eave(x0, 0.0), ridge(x0, 0.0), t);
            let b = mix(eave(x1, 0.0), ridge(x1, 0.0), t);
            let up = [0.0, 0.04, 0.0];
            quad(
                out,
                [a, b, add(b, up), add(a, up)],
                [0.0, 0.3, side],
                tile,
                [0.8; 4],
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
    // A ridge beam.
    slab(
        out,
        [(x0 + x1) / 2.0, ridge_y + 0.03, zc],
        [(x1 - x0) / 2.0, 0.07, 0.1],
        0.04,
        under,
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
    let post = |x: i32, z: i32| world.get([x, top, z]).mat == mat::WOOD;
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
    let timber = paint(mul(TIMBER, 0.9), mat::WOOD);
    let plank = paint(mix(TIMBER, CLAY, 0.3), mat::WOOD);
    let dark = paint([0.02, 0.025, 0.035], mat::WOOD);
    let iron = paint([0.05, 0.05, 0.055], mat::IRON);
    // Corner posts, a little proud of both walls.
    for (cx, cz) in [(x0, z0), (x1 - 1, z0), (x0, z1 - 1), (x1 - 1, z1 - 1)] {
        if (y0..top).all(|y| is([cx, y, cz], mat::WOOD)) {
            let c = [
                cx as f32 + 0.5,
                (y0 + top) as f32 / 2.0 + 0.05,
                cz as f32 + 0.5,
            ];
            cbox(
                out,
                c,
                [X, Y, Z],
                [0.58, (top - y0) as f32 / 2.0 + 0.05, 0.58],
                0.08,
                timber,
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
    let frost = paint(FROST, mat::ICE);
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
    }
}

/// Shallow pools of standing water on the methane hollow's mud floor,
/// between the vents (x, z, radius). Looks only: no water cells.
const PUDDLES: [(f32, f32, f32); 5] = [
    (189.6, 99.4, 0.6),
    (192.7, 99.3, 0.6),
    (192.6, 102.2, 0.7),
    (189.0, 104.5, 0.55),
    (195.3, 100.4, 0.5),
];

/// One pool: a dark, still disc a hair above the mud with a wet rim.
fn puddle(out: &mut MeshData, world: &World, c: [f32; 2], r: f32) {
    let cell = [c[0].floor() as i32, 27, c[1].floor() as i32];
    if world.get(cell).mat != mat::MUD || world.get([cell[0], 28, cell[2]]).mat != mat::AIR {
        return;
    }
    let water = paint([0.012, 0.07, 0.075], mat::WATER);
    let rim = paint([0.07, 0.065, 0.03], mat::MUD);
    let n: usize = 10;
    let ring = |rr: f32, y: f32| -> Vec<V3> {
        (0..n)
            .map(|k| {
                let t = k as f32 / n as f32 * std::f32::consts::TAU;
                let w = 1.0 + 0.18 * (hash([cell[0], k as i32, cell[2]], 40) - 0.5);
                [c[0] + rr * w * t.cos(), y, c[1] + rr * w * t.sin()]
            })
            .collect()
    };
    let y = 28.0;
    let outer = ring(r * 1.25, y + 0.01);
    let inner = ring(r, y + 0.02);
    let centre = [c[0], y + 0.02, c[1]];
    let below = [c[0], y - 1.0, c[1]];
    for k in 0..n {
        let j = (k + 1) % n;
        facet(out, outer[k], outer[j], inner[j], below, rim);
        facet(out, outer[k], inner[j], inner[k], below, rim);
        facet(out, inner[k], inner[j], centre, below, water);
    }
}

fn materials_solid(world: &World, p: [i32; 3]) -> bool {
    crate::materials::flags(world.get(p).mat).solid
}

/// A Marsh gas vent: a low faceted rock mound round a dark throat, on the
/// vent cell's top (looks only; the cell is unchanged).
fn vent_mound(out: &mut MeshData, world: &World, p: [i32; 3]) {
    if world.get([p[0], p[1] + 1, p[2]]).mat != mat::AIR {
        return;
    }
    let base = [p[0] as f32 + 0.5, p[1] as f32 + 1.0, p[2] as f32 + 0.5];
    let rock = paint(mix(ROCK, [0.17, 0.15, 0.08], 0.4), mat::GAS_VENT);
    let crust = paint([0.36, 0.30, 0.10], mat::GAS_VENT);
    let throat = paint([0.03, 0.025, 0.02], mat::GAS_VENT);
    column(out, add(base, [0.0, -0.05, 0.0]), 0.62, 0.34, 0.32, 7, rock);
    column(out, add(base, [0.0, 0.27, 0.0]), 0.34, 0.24, 0.1, 7, crust);
    column(out, add(base, [0.0, 0.3, 0.0]), 0.2, 0.2, 0.08, 7, throat);
    // A few loose stones round it.
    for k in 0..3 {
        let t = (k as f32 + hash(p, 20 + k)) * std::f32::consts::TAU / 3.0;
        let r = 0.72 + 0.12 * hash(p, 30 + k);
        let c = add(base, [r * t.cos(), -0.04, r * t.sin()]);
        column(out, c, 0.12, 0.06, 0.12, 5, rock);
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
            Chunk::Array(cells) => !cells
                .iter()
                .any(|c| is_built(c.mat) || matches!(c.mat, mat::WOOD | mat::ICE | mat::GAS_VENT)),
        },
        None => true,
    }
}

/// The built models owned by the cells in `[lo, hi)`.
pub fn models(world: &World, lo: [i32; 3], hi: [i32; 3]) -> MeshData {
    let mut out = MeshData::default();
    if plainly_empty(world, lo, hi) {
        return out;
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
                if !modelled(p, m) || under_dome(p, m) || kiln_door(p, m) {
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
    for &(x, z, r) in PUDDLES.iter() {
        let c = [x.floor() as i32, 27, z.floor() as i32];
        if (0..3).all(|a| c[a] >= lo[a] && c[a] < hi[a]) {
            puddle(&mut out, world, [x, z], r);
        }
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
