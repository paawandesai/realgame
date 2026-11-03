//! Generated models over the cells the smooth mesher leaves out (the
//! "look gaps" of the smooth world): trees and crystals.
//!
//! The cells stay what they were: wood and leaves still burn, crystals are
//! still blocks to the chemistry and to collision. Only the drawing
//! changes:
//!
//! - **Trees.** A tree is a trunk (a single column of wood with nothing
//!   wooden beside it) and the leaves around its top. Its chunk (the one
//!   holding the trunk's top cell) draws a tapered, faceted trunk and a
//!   canopy built from the leaves that are still there, in the style of
//!   its species: tiered cones for pines and firs (snow-dusted in the
//!   Frozen Fields), clusters of faceted crowns for oaks and birches (a
//!   white trunk for birch). So a canopy that burns shrinks as its leaves
//!   go. Leaves with no trunk near them draw as small leafy clumps.
//! - **Crystals.** Each crystal cell is drawn as rock (so a wall stays a
//!   wall) with a cluster of glowing hexagonal prisms growing out of its
//!   open side.
//!
//! Everything is a pure function of the cells and their positions, so
//! neighbouring chunks agree and a remesh gives the same model.

use crate::mesh::MeshData;
use crate::{gen, mat, World};

/// How far (in cells, sideways) a tree's leaves can be from its trunk.
pub const CROWN_REACH: i32 = 3;
/// The leaves of a tree lie this far below .. above its trunk's top cell.
pub const CROWN_BELOW: i32 = 3;
pub const CROWN_ABOVE: i32 = 4;

fn hash(x: i32, y: i32, z: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f)
        ^ salt.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

/// A number in [0, 1) from a hash.
fn unit(x: i32, y: i32, z: i32, salt: u32) -> f32 {
    (hash(x, y, z, salt) >> 8) as f32 / (1u32 << 24) as f32
}

/// Whether the wood cell at `p` is part of a trunk: wood with no wood on
/// its four sides, and wood or leaves above or below it (a lone built log
/// is a trunk too, drawn as a stump). Stacked logs and wooden walls keep
/// their smooth shape.
pub fn is_trunk(world: &World, p: [i32; 3]) -> bool {
    if world.get(p).mat != mat::WOOD || crate::built::timber(p, mat::WOOD) {
        return false;
    }
    let [x, y, z] = p;
    let wood = |q: [i32; 3]| world.get(q).mat == mat::WOOD;
    if wood([x + 1, y, z]) || wood([x - 1, y, z]) || wood([x, y, z + 1]) || wood([x, y, z - 1]) {
        return false;
    }
    let up = world.get([x, y + 1, z]).mat;
    let down = world.get([x, y - 1, z]).mat;
    up == mat::WOOD || down == mat::WOOD || up == mat::LEAVES
}

/// Whether the smooth surface leaves the cell at `p` (material `m`) to
/// the models: leaves and trunks. Crystals are drawn as rock there instead
/// (`surface_mat`).
pub fn modelled(world: &World, p: [i32; 3], m: u8) -> bool {
    m == mat::LEAVES || (m == mat::WOOD && is_trunk(world, p))
}

/// The material the smooth surface uses for a cell of material `m`:
/// crystals are part of the rock they grow from.
pub fn surface_mat(m: u8) -> u8 {
    if m == mat::CRYSTAL {
        mat::STONE
    } else {
        m
    }
}

// ---------------------------------------------------------------------------
// Faceted triangles.
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

/// How a facet is painted: its colour, glow, material id, and an optional
/// colour its upward faces turn to (snow on a pine).
#[derive(Clone, Copy)]
struct Paint {
    rgb: V3,
    glow: f32,
    mat: u8,
    top: Option<(V3, f32)>,
}

/// One flat-shaded triangle facing away from `inside`.
fn tri(out: &mut MeshData, a: V3, b: V3, c: V3, inside: V3, paint: Paint) {
    let mut n = cross(sub(b, a), sub(c, a));
    let centre = mul(add(add(a, b), c), 1.0 / 3.0);
    let (b, c) = if dot(n, sub(centre, inside)) < 0.0 {
        n = mul(n, -1.0);
        (c, b)
    } else {
        (b, c)
    };
    let n = norm(n);
    let mut rgb = paint.rgb;
    if let Some((top, amount)) = paint.top {
        let t = ((n[1] - 0.78) / 0.17).clamp(0.0, 1.0) * amount;
        rgb = mix(rgb, top, t);
    }
    // Faces turned down sit in the canopy's own shade.
    let light = 1.0 - 0.3 * (-n[1]).max(0.0);
    let i = out.positions.len() as u32;
    for p in [a, b, c] {
        out.positions.push(p);
        out.normals.push(n);
        out.colors.push([rgb[0], rgb[1], rgb[2], light]);
        out.uvs.push([paint.glow, 0.0]);
        out.mats.push(paint.mat);
        out.light.push(light);
    }
    out.indices.extend_from_slice(&[i, i + 1, i + 2]);
}

/// A ring of `n` points around `centre` (horizontal unless `axis` is given)
/// at `radius`, turned by `turn` radians, each pushed in or out by `wobble`
/// (a fraction of the radius, seeded by `seed`).
fn ring(
    centre: V3,
    axis: V3,
    radius: f32,
    n: usize,
    turn: f32,
    wobble: f32,
    seed: [i32; 3],
) -> Vec<V3> {
    let a = norm(axis);
    let helper = if a[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = norm(cross(a, helper));
    let v = cross(a, u);
    (0..n)
        .map(|k| {
            let t = turn + k as f32 * std::f32::consts::TAU / n as f32;
            let r = radius
                * (1.0 + wobble * (unit(seed[0], seed[1] + k as i32, seed[2], 7) * 2.0 - 1.0));
            add(centre, add(mul(u, t.cos() * r), mul(v, t.sin() * r)))
        })
        .collect()
}

/// The side of a frustum between two rings (same point count).
fn band(out: &mut MeshData, lo: &[V3], hi: &[V3], inside: V3, paint: Paint) {
    let n = lo.len();
    for k in 0..n {
        let j = (k + 1) % n;
        tri(out, lo[k], lo[j], hi[j], inside, paint);
        tri(out, lo[k], hi[j], hi[k], inside, paint);
    }
}

/// A fan from a ring to a point (a cone's tip or a cap).
fn fan(out: &mut MeshData, rim: &[V3], tip: V3, inside: V3, paint: Paint) {
    let n = rim.len();
    for k in 0..n {
        tri(out, rim[k], rim[(k + 1) % n], tip, inside, paint);
    }
}

/// Snow (#EEF2F5) on a pine tier's upper half: a shell just outside the
/// tier from part way up its side to its top, with a ragged lower edge.
fn snow_cap(out: &mut MeshData, rim: &[V3], cap: &[V3], tip: V3, base: V3) {
    let snow = Paint {
        rgb: [0.86, 0.89, 0.92],
        glow: 0.0,
        mat: mat::SNOW,
        top: None,
    };
    let axis_at = |p: V3| [base[0], p[1], base[2]];
    let out_by = |p: V3, k: f32| add(p, mul(sub(p, axis_at(p)), k));
    let n = rim.len();
    let lo: Vec<V3> = (0..n)
        .map(|k| {
            // Ragged: each point starts a little higher or lower.
            let f = 0.42 + 0.16 * (((k * 7919) % 5) as f32 / 4.0);
            out_by(mix(rim[k], cap[k], f), 0.06)
        })
        .collect();
    let hi: Vec<V3> = cap
        .iter()
        .map(|&c| add(out_by(c, 0.06), [0.0, 0.02, 0.0]))
        .collect();
    let inside = mix(base, tip, 0.5);
    band(out, &lo, &hi, inside, snow);
    if hi.iter().any(|h| dist(*h, tip) > 1e-3) {
        fan(out, &hi, add(tip, [0.0, 0.03, 0.0]), inside, snow);
    }
}

fn dist(a: V3, b: V3) -> f32 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

/// A faceted, slightly lumpy ball (an icosahedron).
fn ball(out: &mut MeshData, c: V3, r: V3, seed: [i32; 3], paint: Paint) {
    const T: f32 = std::f32::consts::GOLDEN_RATIO;
    const V: [V3; 12] = [
        [-1.0, T, 0.0],
        [1.0, T, 0.0],
        [-1.0, -T, 0.0],
        [1.0, -T, 0.0],
        [0.0, -1.0, T],
        [0.0, 1.0, T],
        [0.0, -1.0, -T],
        [0.0, 1.0, -T],
        [T, 0.0, -1.0],
        [T, 0.0, 1.0],
        [-T, 0.0, -1.0],
        [-T, 0.0, 1.0],
    ];
    const F: [[usize; 3]; 20] = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    let spin = unit(seed[0], seed[1], seed[2], 31) * std::f32::consts::TAU;
    let (s, co) = spin.sin_cos();
    let pts: Vec<V3> = V
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let v = norm(*v);
            let v = [v[0] * co - v[2] * s, v[1], v[0] * s + v[2] * co];
            let w = 0.86 + 0.28 * unit(seed[0] + k as i32, seed[1], seed[2], 13);
            add(c, [v[0] * r[0] * w, v[1] * r[1] * w, v[2] * r[2] * w])
        })
        .collect();
    for f in F {
        tri(out, pts[f[0]], pts[f[1]], pts[f[2]], c, paint);
    }
}

// ---------------------------------------------------------------------------
// Trees.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    /// Tiered cones, dark green; snow on their tops in the Frozen Fields.
    Pine,
    /// A narrower, bluer pine.
    Fir,
    /// A broad crown of faceted clumps on a brown trunk.
    Oak,
    /// Light, airy clumps on a white trunk with dark marks.
    Birch,
}

/// The species of the tree whose trunk's top cell is `top` (standing on
/// `ground`, the material under its trunk).
pub fn species(top: [i32; 3], ground: u8) -> Species {
    let r = unit(top[0], 0, top[2], 101);
    if ground == mat::SNOW {
        return if r < 0.6 { Species::Pine } else { Species::Fir };
    }
    match gen::biome_at(top) {
        crate::Biome::Village => {
            if r < 0.65 {
                Species::Oak
            } else {
                Species::Birch
            }
        }
        _ => {
            if r < 0.5 {
                Species::Birch
            } else if r < 0.8 {
                Species::Oak
            } else {
                Species::Fir
            }
        }
    }
}

/// The trunk tops (wood trunks with no wood above) in `[lo, hi)`.
fn trunk_tops(world: &World, lo: [i32; 3], hi: [i32; 3]) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for z in lo[2]..hi[2] {
        for y in lo[1].max(0)..hi[1] {
            for x in lo[0]..hi[0] {
                let p = [x, y, z];
                if world.get(p).mat == mat::WOOD
                    && world.get([x, y + 1, z]).mat != mat::WOOD
                    && is_trunk(world, p)
                {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// Whether a leaf at `leaf` belongs to the tree topped at `top`.
fn in_crown(top: [i32; 3], leaf: [i32; 3]) -> bool {
    let d = [leaf[0] - top[0], leaf[1] - top[1], leaf[2] - top[2]];
    d[0].abs() <= CROWN_REACH
        && d[2].abs() <= CROWN_REACH
        && (-CROWN_BELOW..=CROWN_ABOVE).contains(&d[1])
}

fn tree(out: &mut MeshData, world: &World, top: [i32; 3]) {
    let [x, ty, z] = top;
    let mut base = ty;
    while base > ty - 16 && world.get([x, base - 1, z]).mat == mat::WOOD {
        base -= 1;
    }
    let ground = world.get([x, base - 1, z]).mat;
    let sp = species(top, ground);
    let snowy = ground == mat::SNOW;
    let axis = [x as f32 + 0.5, 0.0, z as f32 + 0.5];
    let at = |y: f32| [axis[0], y, axis[2]];

    // The leaves still there, by layer.
    let mut leaves: Vec<[i32; 3]> = Vec::new();
    for dy in -CROWN_BELOW..=CROWN_ABOVE {
        for dz in -CROWN_REACH..=CROWN_REACH {
            for dx in -CROWN_REACH..=CROWN_REACH {
                let p = [x + dx, ty + dy, z + dz];
                if world.get(p).mat == mat::LEAVES {
                    leaves.push(p);
                }
            }
        }
    }

    // Trunk: a tapered octagon from just under the ground into the crown,
    // with a flared foot.
    let (bark, marks) = match sp {
        Species::Birch => ([0.80, 0.78, 0.72], Some([0.10, 0.09, 0.08])),
        Species::Pine | Species::Fir => ([0.24, 0.13, 0.07], None),
        Species::Oak => ([0.21, 0.12, 0.06], None),
    };
    let thick = match sp {
        Species::Oak => 0.34,
        Species::Birch => 0.22,
        _ => 0.27,
    };
    let into_crown = if leaves.is_empty() { 0.0 } else { 0.8 };
    let y0 = base as f32 - 0.3;
    let y1 = ty as f32 + 1.0 + into_crown;
    let segs = ((y1 - y0) / 0.9).ceil().max(1.0) as usize;
    let seed = top;
    let mut prev = ring(at(y0), [0.0, 1.0, 0.0], thick * 1.45, 7, 0.3, 0.12, seed);
    for k in 1..=segs {
        let t = k as f32 / segs as f32;
        let y = y0 + (y1 - y0) * t;
        let r = thick * (1.0 - 0.45 * t) * if k == 1 { 1.1 } else { 1.0 };
        // A slight lean, so trunks are not ruled lines.
        let lean = [
            (unit(x, k as i32, z, 41) - 0.5) * 0.08 * t,
            0.0,
            (unit(x, k as i32, z, 43) - 0.5) * 0.08 * t,
        ];
        let next = ring(
            add(at(y), lean),
            [0.0, 1.0, 0.0],
            r,
            7,
            0.3,
            0.06,
            [x, base + k as i32, z],
        );
        let rgb = match marks {
            Some(dark) if unit(x, k as i32, z, 47) < 0.35 => mix(bark, dark, 0.7),
            _ => bark,
        };
        let paint = Paint {
            rgb: mul(rgb, 0.9 + 0.2 * unit(x, k as i32, z, 53)),
            glow: 0.0,
            mat: mat::WOOD,
            top: None,
        };
        band(out, &prev, &next, at(y - 0.5), paint);
        prev = next;
    }
    if leaves.is_empty() {
        // A bare trunk (its crown burnt): a cut top.
        let paint = Paint {
            rgb: [0.12, 0.08, 0.05],
            glow: 0.0,
            mat: mat::WOOD,
            top: None,
        };
        fan(out, &prev, at(y1 - 0.05), at(y1 - 1.0), paint);
        return;
    }

    let leaf_paint = |rgb: V3, k: i32| Paint {
        rgb: mul(rgb, 0.85 + 0.3 * unit(x + k, ty, z, 61)),
        glow: 0.0,
        mat: mat::LEAVES,
        top: if snowy {
            Some(([0.86, 0.90, 0.97], 0.4))
        } else {
            None
        },
    };
    match sp {
        Species::Pine | Species::Fir => {
            let green = if sp == Species::Pine {
                [0.05, 0.19, 0.08]
            } else {
                [0.05, 0.20, 0.15]
            };
            let slim = if sp == Species::Fir { 0.8 } else { 1.0 };
            // One tier per leaf layer, as wide as that layer reaches.
            let lo = leaves.iter().map(|p| p[1]).min().unwrap_or(ty);
            let hi = leaves.iter().map(|p| p[1]).max().unwrap_or(ty);
            for y in lo..=hi {
                let layer: Vec<&[i32; 3]> = leaves.iter().filter(|p| p[1] == y).collect();
                if layer.is_empty() {
                    continue;
                }
                let reach = layer
                    .iter()
                    .map(|p| {
                        let dx = p[0] - x;
                        let dz = p[2] - z;
                        ((dx * dx + dz * dz) as f32).sqrt()
                    })
                    .fold(0.0f32, f32::max);
                let r = (reach + 0.45) * slim * 0.85;
                let yb = y as f32 - 0.15;
                let top_y = if y == hi { yb + 2.1 } else { yb + 1.6 };
                let turn = unit(x, y, z, 71) * 1.0;
                let rim = ring(at(yb), [0.0, 1.0, 0.0], r, 9, turn, 0.14, [x, y, z]);
                let paint = leaf_paint(green, y);
                let tip_r = if y == hi { 0.0 } else { r * 0.22 };
                if tip_r > 0.0 {
                    let cap = ring(at(top_y), [0.0, 1.0, 0.0], tip_r, 9, turn, 0.0, [x, y, z]);
                    band(out, &rim, &cap, at(yb + 0.3), paint);
                    fan(out, &cap, at(top_y + 0.15), at(top_y - 0.5), paint);
                    if snowy {
                        snow_cap(out, &rim, &cap, at(top_y + 0.15), at(yb));
                    }
                } else {
                    fan(out, &rim, at(top_y), at(yb + 0.3), paint);
                    if snowy {
                        let tip = vec![at(top_y); rim.len()];
                        snow_cap(out, &rim, &tip, at(top_y), at(yb));
                    }
                }
                // The tier's underside, up to the trunk.
                let under = Paint {
                    rgb: mul(green, 0.6),
                    top: None,
                    ..paint
                };
                fan(out, &rim, at(yb + 0.35), at(yb + 2.0), under);
            }
        }
        Species::Oak | Species::Birch => {
            let green = if sp == Species::Oak {
                if matches!(gen::biome_at(top), crate::Biome::Village) {
                    [0.12, 0.34, 0.09]
                } else {
                    [0.16, 0.28, 0.07]
                }
            } else {
                [0.28, 0.44, 0.10]
            };
            let size = if sp == Species::Oak { 1.0 } else { 0.8 };
            // The crown's centre and a clump for each sector of leaves
            // around it (eight around, upper and lower), sized by how many
            // leaves it holds: fewer leaves, smaller clumps.
            let n = leaves.len() as f32;
            let c = leaves.iter().fold([0.0; 3], |a, p| {
                add(a, [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5])
            });
            let c = mul(c, 1.0 / n);
            let mut sectors: [(V3, f32); 16] = [([0.0; 3], 0.0); 16];
            for p in &leaves {
                let q = [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5];
                let d = sub(q, c);
                let ang = d[2].atan2(d[0]) + std::f32::consts::PI;
                let s = ((ang / std::f32::consts::TAU * 8.0) as usize).min(7)
                    + if d[1] > 0.0 { 8 } else { 0 };
                sectors[s].0 = add(sectors[s].0, q);
                sectors[s].1 += 1.0;
            }
            let core = (0.55 + 0.17 * n.sqrt()).min(1.5) * size;
            ball(
                out,
                add(c, [0.0, 0.4, 0.0]),
                [core, core * 0.85, core],
                [x, ty, z],
                leaf_paint(mul(green, 0.85), 0),
            );
            for (k, (sum, count)) in sectors.iter().enumerate() {
                if *count < 1.0 {
                    continue;
                }
                let m = mul(*sum, 1.0 / count);
                // Push each clump out from the centre a little.
                let out_dir = sub(m, c);
                let p = add(add(m, mul(out_dir, 0.1)), [0.0, 0.35, 0.0]);
                let r = (0.42 + 0.18 * count.sqrt()).min(1.0) * size;
                ball(
                    out,
                    p,
                    [r, r * 0.8, r],
                    [x + k as i32, ty, z],
                    leaf_paint(green, k as i32 + 1),
                );
            }
        }
    }
}

/// A tree seen from far away: a five-sided trunk and one crown (a cone for
/// pines and firs, a faceted ball for oaks and birches) the size of what
/// is left of its leaves.
fn tree_far(out: &mut MeshData, world: &World, top: [i32; 3]) {
    let [x, ty, z] = top;
    let mut base = ty;
    while base > ty - 16 && world.get([x, base - 1, z]).mat == mat::WOOD {
        base -= 1;
    }
    let ground = world.get([x, base - 1, z]).mat;
    let sp = species(top, ground);
    let axis = [x as f32 + 0.5, 0.0, z as f32 + 0.5];
    let at = |y: f32| [axis[0], y, axis[2]];
    let (mut lo, mut hi, mut reach, mut n) = (i32::MAX, i32::MIN, 0.0f32, 0u32);
    for dy in -CROWN_BELOW..=CROWN_ABOVE {
        for dz in -CROWN_REACH..=CROWN_REACH {
            for dx in -CROWN_REACH..=CROWN_REACH {
                if world.get([x + dx, ty + dy, z + dz]).mat == mat::LEAVES {
                    lo = lo.min(ty + dy);
                    hi = hi.max(ty + dy);
                    reach = reach.max(((dx * dx + dz * dz) as f32).sqrt());
                    n += 1;
                }
            }
        }
    }
    let bark = match sp {
        Species::Birch => [0.80, 0.78, 0.72],
        _ => [0.22, 0.12, 0.06],
    };
    let paint = |rgb: V3| Paint {
        rgb,
        glow: 0.0,
        mat: mat::WOOD,
        top: None,
    };
    let y0 = base as f32 - 0.3;
    let y1 = ty as f32 + if n > 0 { 1.5 } else { 1.0 };
    let r0 = ring(at(y0), [0.0, 1.0, 0.0], 0.32, 5, 0.3, 0.0, top);
    let r1 = ring(at(y1), [0.0, 1.0, 0.0], 0.18, 5, 0.3, 0.0, top);
    band(out, &r0, &r1, at(y0 + 0.5), paint(bark));
    if n == 0 {
        return;
    }
    let leaf = |rgb: V3| Paint {
        rgb,
        glow: 0.0,
        mat: mat::LEAVES,
        top: (ground == mat::SNOW).then_some(([0.86, 0.90, 0.97], 0.4)),
    };
    match sp {
        Species::Pine | Species::Fir => {
            let green = if sp == Species::Pine {
                [0.05, 0.19, 0.08]
            } else {
                [0.05, 0.20, 0.15]
            };
            let slim = if sp == Species::Fir { 0.8 } else { 1.0 };
            let yb = lo as f32 - 0.15;
            let rim = ring(
                at(yb),
                [0.0, 1.0, 0.0],
                (reach + 0.45) * slim * 0.9,
                6,
                0.2,
                0.0,
                top,
            );
            fan(out, &rim, at(hi as f32 + 2.0), at(yb + 0.3), leaf(green));
            fan(
                out,
                &rim,
                at(yb + 0.35),
                at(yb + 2.0),
                leaf(mul(green, 0.6)),
            );
        }
        Species::Oak | Species::Birch => {
            let green = match sp {
                Species::Oak => [0.13, 0.32, 0.09],
                _ => [0.28, 0.44, 0.10],
            };
            let c = at((lo + hi) as f32 / 2.0 + 0.9);
            let r = (reach + 0.9).min(3.2);
            let h = ((hi - lo) as f32 / 2.0 + 0.9).max(1.0);
            ball(out, c, [r, h, r], top, leaf(green));
        }
    }
}

/// A leaf with no trunk near it: a small clump.
fn clump(out: &mut MeshData, p: [i32; 3]) {
    let c = [p[0] as f32 + 0.5, p[1] as f32 + 0.45, p[2] as f32 + 0.5];
    let r = 0.62 + 0.12 * unit(p[0], p[1], p[2], 83);
    ball(
        out,
        c,
        [r, r * 0.85, r],
        p,
        Paint {
            rgb: mul([0.13, 0.36, 0.11], 0.85 + 0.3 * unit(p[0], p[1], p[2], 89)),
            glow: 0.0,
            mat: mat::LEAVES,
            top: None,
        },
    );
}

// ---------------------------------------------------------------------------
// Crystals.
// ---------------------------------------------------------------------------

const SIDES: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// The open side a crystal cell's cluster grows out of: an open side
/// whose opposite side is rock, if there is one (`built` sets its rock
/// collar on the same side).
pub(crate) fn crystal_side(world: &World, p: [i32; 3]) -> Option<[i32; 3]> {
    let [x, y, z] = p;
    let solid =
        |d: [i32; 3]| crate::materials::flags(world.get([x + d[0], y + d[1], z + d[2]]).mat).solid;
    let open: Vec<[i32; 3]> = SIDES.iter().copied().filter(|&d| !solid(d)).collect();
    let backed: Vec<[i32; 3]> = open
        .iter()
        .copied()
        .filter(|d| solid([-d[0], -d[1], -d[2]]))
        .collect();
    let pool = if backed.is_empty() { &open } else { &backed };
    pool.get((hash(x, y, z, 97) as usize) % pool.len().max(1))
        .copied()
}

fn crystal(out: &mut MeshData, world: &World, p: [i32; 3]) {
    let [x, y, z] = p;
    // Grow out of the rock.
    let Some(d) = crystal_side(world, p) else {
        return;
    };
    let dir = [d[0] as f32, d[1] as f32, d[2] as f32];
    let face = [
        x as f32 + 0.5 + dir[0] * 0.42,
        y as f32 + 0.5 + dir[1] * 0.42,
        z as f32 + 0.5 + dir[2] * 0.42,
    ];
    // A cluster: one tall crystal and three to five smaller ones round it
    // (B4 round 3: lone shards read as floating).
    let count = 4 + (hash(x, y, z, 101) % 3) as i32;
    for k in 0..count {
        let s = [x + k * 7, y, z];
        let jitter = [
            unit(s[0], s[1], s[2], 103) - 0.5,
            unit(s[0], s[1], s[2], 107) - 0.5,
            unit(s[0], s[1], s[2], 109) - 0.5,
        ];
        let axis = norm(add(dir, mul(jitter, if k == 0 { 0.35 } else { 1.0 })));
        let offset = mul(sub(jitter, mul(dir, dot(jitter, dir))), 0.5);
        let foot = add(add(face, offset), mul(axis, -0.15));
        let len = if k == 0 { 1.1 } else { 0.35 } + 0.6 * unit(s[0], s[1], s[2], 113);
        let r = if k == 0 { 0.24 } else { 0.1 } + 0.05 * unit(s[0], s[1], s[2], 127);
        let turn = unit(s[0], s[1], s[2], 131) * 1.0;
        // Science teal (#40FFD9), a few leaning a little bluer.
        let tint = mix(
            [0.06, 1.0, 0.70],
            [0.20, 0.80, 1.0],
            unit(s[0], s[1], s[2], 137) * 0.5,
        );
        let paint = Paint {
            // Brighter than any wall: the glow is the colour (kept under
            // the bloom ceiling, so it reads teal, not white).
            rgb: mul(tint, 1.2),
            glow: 1.0,
            mat: mat::CRYSTAL,
            top: None,
        };
        let bottom = ring(foot, axis, r, 6, turn, 0.0, s);
        let shoulder_at = add(foot, mul(axis, len));
        let shoulder = ring(shoulder_at, axis, r * 0.92, 6, turn, 0.0, s);
        let tip = add(shoulder_at, mul(axis, r * 1.8 + 0.1));
        let inside = add(foot, mul(axis, len * 0.5));
        band(out, &bottom, &shoulder, inside, paint);
        fan(out, &shoulder, tip, inside, paint);
        fan(out, &bottom, add(foot, mul(axis, -0.05)), inside, paint);
    }
}

// ---------------------------------------------------------------------------
// The far mountains.
// ---------------------------------------------------------------------------

/// A range of far mountains all around the valley, outside the world's
/// cells (nothing there can be reached or changed): faceted peaks of
/// blue-grey rock with snow above y 118, rising behind the edge ring so its
/// line is broken by higher, further summits. Rows run from 14 m to 120 m
/// outside the border; fog softens them with distance.
pub fn backdrop(seed: u64) -> MeshData {
    let mut out = MeshData::default();
    let half = [crate::WORLD[0] as f32 / 2.0, crate::WORLD[2] as f32 / 2.0];
    let centre = [half[0], 0.0, half[1]];
    const STEPS: usize = 220;
    // Distances outside the border, and each row's base height and peak.
    const ROWS: [(f32, f32, f32); 6] = [
        (14.0, 52.0, 18.0),
        (32.0, 64.0, 46.0),
        (52.0, 70.0, 74.0),
        (74.0, 66.0, 92.0),
        (98.0, 60.0, 70.0),
        (122.0, 44.0, 30.0),
    ];
    let s = (seed as u32) ^ 0x6d0;
    // A point `out` metres outside the border, in direction `angle`.
    let at = |k: usize, row: usize| -> V3 {
        let a = k as f32 / STEPS as f32 * std::f32::consts::TAU;
        let (sn, cs) = a.sin_cos();
        // The distance from the centre to the square border along `a`.
        let r_border = (half[0] / cs.abs().max(1e-4)).min(half[1] / sn.abs().max(1e-4));
        let (dist, base, peak) = ROWS[row];
        let kk = (k % STEPS) as i32;
        // Ridged peaks along the range, a few big summits, and a jagged
        // jitter so neighbouring facets differ.
        let big = 1.0 - (2.0 * unit(kk / 18, row as i32, 0, s) - 1.0).abs();
        let next = 1.0 - (2.0 * unit(kk / 18 + 1, row as i32, 0, s) - 1.0).abs();
        let t = (kk % 18) as f32 / 18.0;
        let t = t * t * (3.0 - 2.0 * t);
        let ridge = big + (next - big) * t;
        let jag = unit(kk, row as i32, 1, s);
        let h = base + peak * (0.35 + 0.65 * ridge * ridge) + 9.0 * jag;
        let r = r_border + dist + 6.0 * (unit(kk, row as i32, 2, s) - 0.5);
        [centre[0] + cs * r, h, centre[2] + sn * r]
    };
    let rock = [0.30, 0.33, 0.42];
    let snow = [0.86, 0.90, 0.97];
    for k in 0..STEPS {
        for row in 0..ROWS.len() - 1 {
            let a = at(k, row);
            let b = at(k + 1, row);
            let c = at(k + 1, row + 1);
            let d = at(k, row + 1);
            for (p, q, r) in [(a, b, c), (a, c, d)] {
                let mid = mul(add(add(p, q), r), 1.0 / 3.0);
                let outward = norm([mid[0] - centre[0], 0.0, mid[2] - centre[2]]);
                // Faces turn towards the valley (and up).
                let inside = add(mid, add(mul(outward, 40.0), [0.0, -60.0, 0.0]));
                let high = ((mid[1] - 112.0) / 14.0).clamp(0.0, 1.0);
                let paint = Paint {
                    rgb: mul(
                        mix(rock, snow, high),
                        0.85 + 0.3 * unit(k as i32, row as i32, 3, s),
                    ),
                    glow: 0.0,
                    mat: mat::CLIFFSTONE,
                    top: Some((snow, 0.6)),
                };
                tri(&mut out, p, q, r, inside, paint);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// A region's models.
// ---------------------------------------------------------------------------

/// The models owned by the cells in `[lo, hi)`: the trees whose trunk tops
/// lie there, lone leaves, and crystals.
pub fn models(world: &World, lo: [i32; 3], hi: [i32; 3]) -> MeshData {
    models_lod(world, lo, hi, false)
}

/// The far level of detail of the same models (art bible: beyond 60 m
/// props drop to simpler meshes): each tree a five-sided trunk under one
/// cone or one faceted crown, lone leaves as before, no crystals (they are
/// underground, where nothing is seen that far).
pub fn models_far(world: &World, lo: [i32; 3], hi: [i32; 3]) -> MeshData {
    models_lod(world, lo, hi, true)
}

fn models_lod(world: &World, lo: [i32; 3], hi: [i32; 3], far: bool) -> MeshData {
    let mut out = MeshData::default();
    // Every trunk top a leaf in the region could belong to.
    let reach_lo = [
        lo[0] - CROWN_REACH,
        lo[1] - CROWN_ABOVE,
        lo[2] - CROWN_REACH,
    ];
    let reach_hi = [
        hi[0] + CROWN_REACH,
        hi[1] + CROWN_BELOW,
        hi[2] + CROWN_REACH,
    ];
    let mut tops: Option<Vec<[i32; 3]>> = None;
    for z in lo[2]..hi[2] {
        for y in lo[1].max(0)..hi[1] {
            for x in lo[0]..hi[0] {
                let p = [x, y, z];
                match world.get(p).mat {
                    mat::WOOD => {
                        if world.get([x, y + 1, z]).mat != mat::WOOD && is_trunk(world, p) {
                            if far {
                                tree_far(&mut out, world, p);
                            } else {
                                tree(&mut out, world, p);
                            }
                        }
                    }
                    mat::LEAVES => {
                        let tops =
                            tops.get_or_insert_with(|| trunk_tops(world, reach_lo, reach_hi));
                        if !tops.iter().any(|&t| in_crown(t, p)) {
                            clump(&mut out, p);
                        }
                    }
                    mat::CRYSTAL if !far => crystal(&mut out, world, p),
                    _ => {}
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;

    fn flat() -> World {
        let mut w = World::new();
        for z in 0..40 {
            for x in 0..40 {
                for y in 0..30 {
                    w.set([x, y, z], Cell::new(mat::GRASS));
                }
            }
        }
        w
    }

    fn plant(w: &mut World, x: i32, z: i32, crown: &[(i32, i32)]) -> [i32; 3] {
        for y in 30..35 {
            w.set([x, y, z], Cell::new(mat::WOOD));
        }
        for &(k, r) in crown {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dz * dz <= r * r + r / 2 {
                        let p = [x + dx, 35 + k, z + dz];
                        if w.get(p).mat == mat::AIR {
                            w.set(p, Cell::new(mat::LEAVES));
                        }
                    }
                }
            }
        }
        [x, 34, z]
    }

    fn bounds(m: &MeshData) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &m.positions {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn a_tree_is_a_trunk_and_a_crown_and_both_face_out() {
        let mut w = flat();
        let top = plant(&mut w, 20, 20, &[(-1, 2), (0, 2), (1, 1)]);
        assert!(is_trunk(&w, top));
        assert!(is_trunk(&w, [20, 30, 20]));
        let m = models(&w, [0, 0, 0], [40, 64, 40]);
        assert!(
            m.indices.len() / 3 > 60,
            "{} triangles",
            m.indices.len() / 3
        );
        assert!(m.mats.contains(&mat::WOOD) && m.mats.contains(&mat::LEAVES));
        let (lo, hi) = bounds(&m);
        assert!(lo[1] < 30.0 && hi[1] > 36.0, "{lo:?} {hi:?}");
        assert!(hi[0] - lo[0] < 8.0, "a crown stays near its leaves");
        // Every face points away from the tree's axis or up/down (no faces
        // turned inwards to show their back).
        for t in m.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|k| m.positions[t[k] as usize]);
            let n = cross(sub(b, a), sub(c, a));
            assert!(
                dot(n, m.normals[t[0] as usize]) > 0.0,
                "winding matches the normal"
            );
        }
    }

    #[test]
    fn a_burnt_crown_shrinks_and_a_bare_trunk_is_left() {
        let mut w = flat();
        plant(&mut w, 20, 20, &[(-1, 2), (0, 2), (1, 1)]);
        let full = models(&w, [0, 0, 0], [40, 64, 40]);
        // Burn the upper half of the leaves.
        for z in 15..26 {
            for x in 15..26 {
                for y in 35..37 {
                    if w.get([x, y, z]).mat == mat::LEAVES {
                        w.set([x, y, z], Cell::AIR);
                    }
                }
            }
        }
        let half = models(&w, [0, 0, 0], [40, 64, 40]);
        assert!(
            bounds(&half).1[1] < bounds(&full).1[1],
            "the crown is lower"
        );
        for z in 15..26 {
            for x in 15..26 {
                for y in 30..40 {
                    if w.get([x, y, z]).mat == mat::LEAVES {
                        w.set([x, y, z], Cell::AIR);
                    }
                }
            }
        }
        let bare = models(&w, [0, 0, 0], [40, 64, 40]);
        assert!(!bare.mats.contains(&mat::LEAVES));
        assert!(bare.mats.contains(&mat::WOOD));
    }

    #[test]
    fn each_tree_is_drawn_once_across_chunks() {
        let mut w = flat();
        // A trunk right at a chunk edge (x 31/32), its crown in both.
        plant(&mut w, 31, 20, &[(-1, 2), (0, 2), (1, 1)]);
        let a = models(&w, [0, 0, 0], [32, 64, 32]);
        let b = models(&w, [32, 0, 0], [64, 64, 32]);
        assert!(a.mats.contains(&mat::WOOD));
        assert!(
            !b.mats.contains(&mat::WOOD),
            "the other chunk draws no trunk"
        );
        assert!(!b.mats.contains(&mat::LEAVES), "nor its leaves as clumps");
        // A lone leaf with no trunk is a clump in its own chunk.
        w.set([36, 31, 5], Cell::new(mat::LEAVES));
        let b = models(&w, [32, 0, 0], [64, 64, 32]);
        assert!(b.mats.contains(&mat::LEAVES));
    }

    #[test]
    fn species_follow_the_ground() {
        assert!(matches!(
            species([60, 44, 30], mat::SNOW),
            Species::Pine | Species::Fir
        ));
        let village: Vec<Species> = (0..40)
            .map(|k| species([60 + k * 3, 36, 215], mat::GRASS))
            .collect();
        assert!(village.contains(&Species::Oak) && village.contains(&Species::Birch));
    }

    #[test]
    fn far_trees_are_much_simpler_and_keep_their_size() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let (lo, hi) = ([96, 32, 192], [128, 64, 224]);
        let near = models(&w, lo, hi);
        let far = models_far(&w, lo, hi);
        assert!(!far.is_empty());
        assert!(
            far.indices.len() * 3 < near.indices.len(),
            "{} vs {}",
            far.indices.len(),
            near.indices.len()
        );
        let (a, b) = (bounds(&near), bounds(&far));
        for k in 0..3 {
            assert!(
                (a.0[k] - b.0[k]).abs() < 3.0 && (a.1[k] - b.1[k]).abs() < 3.0,
                "{a:?} {b:?}"
            );
        }
    }

    #[test]
    fn a_crystal_is_glowing_prisms_out_of_its_open_side() {
        let mut w = flat();
        w.set([10, 29, 10], Cell::new(mat::CRYSTAL));
        let m = models(&w, [0, 0, 0], [32, 64, 32]);
        assert!(!m.indices.is_empty());
        assert!(m.uvs.iter().all(|u| u[0] == 1.0), "it glows");
        let (lo, hi) = bounds(&m);
        assert!(
            hi[1] > 30.5 && lo[1] > 29.0,
            "it grows up out of the ground: {lo:?} {hi:?}"
        );
    }

    #[test]
    fn the_far_mountains_ring_the_valley_outside_it() {
        let m = backdrop(crate::DEFAULT_SEED);
        assert!(m.indices.len() / 3 > 1000);
        let (w, d) = (crate::WORLD[0] as f32, crate::WORLD[2] as f32);
        let mut over = 0;
        for p in &m.positions {
            let outside = p[0] < -5.0 || p[0] > w + 5.0 || p[2] < -5.0 || p[2] > d + 5.0;
            assert!(outside, "inside the world: {p:?}");
            // Within the camera's 400 m from anywhere in the valley.
            assert!(p[0] > -140.0 && p[0] < w + 140.0 && p[2] > -140.0 && p[2] < d + 140.0);
            if p[1] > 100.0 {
                over += 1;
            }
        }
        assert!(over > 50, "peaks rise above the edge ring: {over}");
        // Faces are turned towards the valley or up (a few along the range
        // turn sideways).
        let tris = m.indices.len() / 3;
        let facing = m
            .indices
            .chunks(3)
            .filter(|t| {
                let p = m.positions[t[0] as usize];
                let n = m.normals[t[0] as usize];
                let to_centre = [w / 2.0 - p[0], 0.0, d / 2.0 - p[2]];
                dot(n, to_centre) > 0.0 || n[1] > 0.3
            })
            .count();
        assert!(facing * 10 > tris * 9, "{facing} of {tris}");
    }

    #[test]
    fn the_valley_models_mesh_quickly() {
        let w = crate::generate(crate::DEFAULT_SEED);
        let t = std::time::Instant::now();
        let mut tris = 0;
        for c in 0..(crate::CHUNKS[0] * crate::CHUNKS[1] * crate::CHUNKS[2]) as usize {
            let o = World::chunk_origin(c);
            tris += models(&w, o, o.map(|v| v + crate::CHUNK)).indices.len() / 3;
        }
        assert!(tris > 10_000, "{tris}");
        eprintln!("models: {tris} triangles in {:?}", t.elapsed());
    }
}
