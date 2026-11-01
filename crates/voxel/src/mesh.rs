//! The greedy mesher (section 5.8): per face direction over 32 x 32 slices,
//! per-vertex AO by the 3-neighbour rule, water tops lowered by level.
//!
//! A face is drawn only when the neighbour is air or a transparent material
//! of a different id; below the world counts as solid (nobody sees the
//! underside of the bedrock), and the other sides of the world as air.
//! Quads merge only when material, AO and liquid height match.
//!
//! Plain vectors, no Bevy types, so a server or the VR client can reuse it.
//! Positions are in world blocks. UVs are in blocks along the quad (so a
//! repeating tile shows once per block on a merged quad), and the colour is
//! the material's toon tint (`TINT`) times the corner's AO.

use crate::materials::{flags, Phase};
use crate::{mat, World, CHUNK};

/// One mesh's vertex streams and indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// AO and tint, linear RGBA.
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Quads in the mesh (4 vertices, 6 indices each).
    pub fn quads(&self) -> usize {
        self.indices.len() / 6
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// A chunk's two passes: `opaque` (alpha-tested atlas) and `clear` (water,
/// ice, glass and crystal, alpha blend).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkMesh {
    pub opaque: MeshData,
    pub clear: MeshData,
}

impl ChunkMesh {
    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.clear.is_empty()
    }
}

/// The toon palette: each material's base colour, linear RGB, plus alpha
/// for the clear pass. Flat colours in the courtyard's palette; the client's
/// tile texture adds the darker border and the detail.
pub const TINT: [[f32; 4]; mat::COUNT] = [
    [0.0, 0.0, 0.0, 0.0],     // air
    [0.32, 0.18, 0.08, 1.0],  // dirt
    [0.22, 0.48, 0.12, 1.0],  // grass
    [0.80, 0.68, 0.40, 1.0],  // sand
    [0.40, 0.40, 0.42, 1.0],  // stone
    [0.30, 0.29, 0.33, 1.0],  // cliffstone
    [0.24, 0.30, 0.45, 1.0],  // runestone
    [0.36, 0.20, 0.08, 1.0],  // wood
    [0.10, 0.32, 0.08, 1.0],  // leaves
    [0.62, 0.42, 0.20, 1.0],  // planks
    [0.92, 0.95, 1.00, 1.0],  // snow
    [0.55, 0.80, 1.00, 0.80], // ice
    [0.10, 0.35, 0.80, 0.70], // water
    [0.90, 0.92, 0.95, 0.35], // steam
    [0.22, 0.15, 0.08, 1.0],  // mud
    [0.16, 0.16, 0.17, 1.0],  // coal_ore
    [0.52, 0.34, 0.26, 1.0],  // iron_ore
    [0.60, 0.40, 1.00, 0.85], // crystal
    [0.80, 0.92, 1.00, 0.30], // glass
    [0.62, 0.63, 0.66, 1.0],  // iron
    [0.58, 0.20, 0.12, 1.0],  // brick
    [0.30, 0.30, 0.30, 1.0],  // ash
    [1.00, 0.70, 0.25, 1.0],  // torch
    [0.55, 0.36, 0.60, 1.0],  // bench
    [0.85, 0.65, 0.20, 1.0],  // invention_bench
    [0.35, 0.12, 0.08, 1.0],  // kiln_hearth
    [0.38, 0.30, 0.18, 1.0],  // gas_vent
];

/// The base colour of material `id` (magenta past the table, so a missing
/// row shows).
pub fn tint(id: u8) -> [f32; 4] {
    TINT.get(id as usize)
        .copied()
        .unwrap_or([1.0, 0.0, 1.0, 1.0])
}

/// Drawn in the clear (alpha-blended) pass.
pub fn is_clear(id: u8) -> bool {
    matches!(
        id,
        mat::WATER | mat::ICE | mat::GLASS | mat::CRYSTAL | mat::STEAM
    )
}

/// Brightness of a corner by how many of its 3 neighbours occlude it.
const AO_LIGHT: [f32; 4] = [0.5, 0.66, 0.83, 1.0];

/// The padded copy of a chunk: 34³ cells, one ring of neighbours.
const P: i32 = CHUNK + 2;

struct Pad {
    mat: Vec<u8>,
    level: Vec<u8>,
}

impl Pad {
    fn idx(x: i32, y: i32, z: i32) -> usize {
        ((x + 1) + P * ((y + 1) + P * (z + 1))) as usize
    }

    fn mat(&self, p: [i32; 3]) -> u8 {
        self.mat[Pad::idx(p[0], p[1], p[2])]
    }

    fn occ(&self, p: [i32; 3]) -> bool {
        flags(self.mat(p)).occludes()
    }
}

/// True when a face of `m` toward neighbour `n` is drawn.
fn visible(m: u8, n: u8) -> bool {
    if n == mat::AIR {
        return true;
    }
    n != m && flags(n).transparent
}

/// Meshes chunk `chunk` (an index from `World::chunk_index`). Positions are
/// in world blocks.
pub fn mesh_chunk(world: &World, chunk: usize) -> ChunkMesh {
    let mut out = ChunkMesh::default();
    let o = World::chunk_origin(chunk);
    if let crate::Chunk::Uniform(c) = world.chunk(chunk) {
        if c.mat == mat::AIR {
            return out;
        }
        // A solid chunk buried in solid chunks has no visible face.
        let buried = flags(c.mat).occludes()
            && [
                [1, 0, 0],
                [-1, 0, 0],
                [0, 1, 0],
                [0, -1, 0],
                [0, 0, 1],
                [0, 0, -1],
            ]
            .iter()
            .all(|d| {
                let q = [0, 1, 2].map(|a| o[a] + d[a] * CHUNK);
                if q[1] < 0 {
                    return true;
                }
                match World::chunk_index(q).map(|i| world.chunk(i)) {
                    Some(crate::Chunk::Uniform(n)) => flags(n.mat).occludes(),
                    _ => false,
                }
            });
        if buried {
            return out;
        }
    }
    let n = (P * P * P) as usize;
    let mut pad = Pad {
        mat: vec![0; n],
        level: vec![0; n],
    };
    for z in -1..=CHUNK {
        for y in -1..=CHUNK {
            for x in -1..=CHUNK {
                let p = [o[0] + x, o[1] + y, o[2] + z];
                let i = Pad::idx(x, y, z);
                if p[1] < 0 {
                    pad.mat[i] = mat::CLIFFSTONE;
                    continue;
                }
                let c = world.get(p);
                pad.mat[i] = c.mat;
                pad.level[i] = c.level();
            }
        }
    }
    let mut mask = vec![0u32; (CHUNK * CHUNK) as usize];
    for axis in 0..3 {
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        for sign in [1i32, -1] {
            let mut dn = [0i32; 3];
            dn[axis] = sign;
            for d in 0..CHUNK {
                // Build the mask for slice `d`: the faces of cells in the
                // slice that face `sign` along `axis`.
                for j in 0..CHUNK {
                    for i in 0..CHUNK {
                        let mut p = [0i32; 3];
                        p[axis] = d;
                        p[u] = i;
                        p[v] = j;
                        mask[(i + CHUNK * j) as usize] = face_key(&pad, p, dn, u, v);
                    }
                }
                // Greedy: grow each face along u, then v.
                for j in 0..CHUNK {
                    let mut i = 0;
                    while i < CHUNK {
                        let k = mask[(i + CHUNK * j) as usize];
                        if k == 0 {
                            i += 1;
                            continue;
                        }
                        let mut w = 1;
                        while i + w < CHUNK && mask[(i + w + CHUNK * j) as usize] == k {
                            w += 1;
                        }
                        let mut h = 1;
                        'grow: while j + h < CHUNK {
                            for t in 0..w {
                                if mask[(i + t + CHUNK * (j + h)) as usize] != k {
                                    break 'grow;
                                }
                            }
                            h += 1;
                        }
                        for jj in 0..h {
                            for ii in 0..w {
                                mask[(i + ii + CHUNK * (j + jj)) as usize] = 0;
                            }
                        }
                        let mut base = o;
                        base[axis] += d;
                        base[u] += i;
                        base[v] += j;
                        emit(&mut out, k, base, axis, u, v, sign, w, h);
                        i += w;
                    }
                }
            }
        }
    }
    out
}

/// The merge key of the face of cell `p` (chunk-local) toward `dn`, or 0
/// for no face: material (8 bits), the 4 corner AO values (2 bits each),
/// and the liquid height (4 bits; 8 is a full block).
fn face_key(pad: &Pad, p: [i32; 3], dn: [i32; 3], u: usize, v: usize) -> u32 {
    let m = pad.mat(p);
    if m == mat::AIR {
        return 0;
    }
    let q = [p[0] + dn[0], p[1] + dn[1], p[2] + dn[2]];
    if !visible(m, pad.mat(q)) {
        return 0;
    }
    // A liquid with no liquid above is lowered to its level.
    let mut height = 8u32;
    if flags(m).phase == Phase::Liquid {
        let above = pad.mat([p[0], p[1] + 1, p[2]]);
        if above != m {
            height = u32::from(pad.level[Pad::idx(p[0], p[1], p[2])].clamp(1, 8));
        }
    }
    // AO at the corners (0,0), (1,0), (1,1), (0,1) in (u, v).
    let mut ao = 0u32;
    for (k, (du, dv)) in [(-1, -1), (1, -1), (1, 1), (-1, 1)].into_iter().enumerate() {
        let mut s1 = q;
        s1[u] += du;
        let mut s2 = q;
        s2[v] += dv;
        let mut c = q;
        c[u] += du;
        c[v] += dv;
        let (a, b, cc) = (pad.occ(s1), pad.occ(s2), pad.occ(c));
        let val = if a && b {
            0
        } else {
            3 - u32::from(a) - u32::from(b) - u32::from(cc)
        };
        ao |= val << (2 * k);
    }
    // Never 0: material ids start at 1 for anything drawn.
    u32::from(m) | (ao << 8) | (height << 16)
}

#[allow(clippy::too_many_arguments)]
fn emit(
    out: &mut ChunkMesh,
    key: u32,
    base: [i32; 3],
    axis: usize,
    u: usize,
    v: usize,
    sign: i32,
    w: i32,
    h: i32,
) {
    let m = (key & 0xff) as u8;
    let ao = (key >> 8) & 0xff;
    let height = ((key >> 16) & 0xf) as f32 / 8.0;
    let mesh = if is_clear(m) {
        &mut out.clear
    } else {
        &mut out.opaque
    };
    let plane = base[axis] as f32 + if sign > 0 { 1.0 } else { 0.0 };
    let t = tint(m);
    let first = mesh.positions.len() as u32;
    let mut normal = [0.0f32; 3];
    normal[axis] = sign as f32;
    let corners = [(0, 0), (w, 0), (w, h), (0, h)];
    let mut lights = [0f32; 4];
    for (k, &(cu, cv)) in corners.iter().enumerate() {
        let mut pos = [0.0f32; 3];
        pos[axis] = plane;
        pos[u] = (base[u] + cu) as f32;
        pos[v] = (base[v] + cv) as f32;
        // A lowered liquid: the top face drops, and side faces end at the
        // liquid's surface (such a face is never more than 1 block tall).
        let top_face = axis == 1 && sign > 0;
        if height < 1.0 && (top_face || (axis != 1 && pos[1] > base[1] as f32)) {
            pos[1] = base[1] as f32 + height;
        }
        mesh.positions.push(pos);
        mesh.normals.push(normal);
        mesh.uvs.push([cu as f32, cv as f32]);
        let a = AO_LIGHT[((ao >> (2 * k)) & 3) as usize];
        lights[k] = a;
        mesh.colors.push([t[0] * a, t[1] * a, t[2] * a, t[3]]);
    }
    // Wind outward: counter-clockwise seen from the face's side. Flip the
    // diagonal so the AO gradient interpolates without a crease.
    let flip = lights[0] + lights[2] < lights[1] + lights[3];
    let quad: [u32; 6] = match (sign > 0, flip) {
        (true, false) => [0, 1, 2, 0, 2, 3],
        (true, true) => [1, 2, 3, 1, 3, 0],
        (false, false) => [0, 2, 1, 0, 3, 2],
        (false, true) => [1, 3, 2, 1, 0, 3],
    };
    mesh.indices.extend(quad.iter().map(|i| first + i));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;

    fn tri_normal(m: &MeshData, t: usize) -> [f32; 3] {
        let [a, b, c] = [0, 1, 2].map(|k| m.positions[m.indices[t * 3 + k] as usize]);
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ]
    }

    #[test]
    fn triangles_wind_outward() {
        let mut w = World::new();
        w.set([3, 40, 3], Cell::new(mat::STONE));
        w.set([3, 41, 3], Cell::new(mat::STONE));
        w.set([4, 40, 3], Cell::new(mat::STONE));
        let m = mesh_chunk(&w, World::chunk_index([0, 40, 0]).unwrap()).opaque;
        for t in 0..m.indices.len() / 3 {
            let n = tri_normal(&m, t);
            let want = m.normals[m.indices[t * 3] as usize];
            let dot = n[0] * want[0] + n[1] * want[1] + n[2] * want[2];
            assert!(dot > 0.0, "triangle {t} faces {n:?}, normal {want:?}");
        }
    }

    #[test]
    fn ao_darkens_an_inner_corner() {
        // A floor with a wall: the floor corners against the wall are darker.
        let mut w = World::new();
        for x in 0..4 {
            for z in 0..4 {
                w.set([x, 40, z], Cell::new(mat::STONE));
            }
            w.set([x, 41, 0], Cell::new(mat::STONE));
        }
        let m = mesh_chunk(&w, World::chunk_index([0, 40, 0]).unwrap()).opaque;
        let mut dark = false;
        for (k, p) in m.positions.iter().enumerate() {
            if m.normals[k] == [0.0, 1.0, 0.0] && (p[1] - 41.0).abs() < 1e-6 && p[2] == 1.0 {
                dark |= m.colors[k][0] < tint(mat::STONE)[0] * 0.99;
            }
        }
        assert!(dark);
    }

    #[test]
    fn a_lowered_water_top_sits_at_its_level() {
        let mut w = World::new();
        let mut c = Cell::new(mat::WATER);
        c.set_level(4);
        w.set([6, 40, 6], c);
        let m = mesh_chunk(&w, World::chunk_index([0, 40, 0]).unwrap()).clear;
        let top = m.positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!((top - 40.5).abs() < 1e-6, "{top}");
    }
}
