//! Turns a window of the 2D simulation grid into a thin 3D voxel slab.
//!
//! Plain Rust with no Bevy types, so it can be benchmarked and tested alone.
//! Each row is split into runs of the same look (species, plus a heat bucket
//! for glowing cells). A run becomes one front quad; its top, bottom and end
//! faces are added only where they are exposed (the neighbour is air or gas),
//! so a flat stone floor 300 cells wide is a handful of quads, not 300 cubes.

use realgame_sim::grid::Grid;
use realgame_sim::palette::{self, heat_color, mix};
use realgame_sim::species::Species;

/// Which mesh (and so which material) a cell goes into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    /// Nothing drawn (air).
    None,
    /// Lit, opaque: stone, wood, water, ice, door, crystal.
    Solid,
    /// Unlit and glowing: flame and anything hotter than 400 C.
    Glow,
    /// Translucent: steam and the gases.
    Gas,
}

#[derive(Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn quads(&self) -> usize {
        self.positions.len() / 4
    }

    fn clear(&mut self) {
        self.positions.clear();
        self.normals.clear();
        self.colors.clear();
        self.indices.clear();
    }

    /// Adds a quad with corners in counter-clockwise order seen from the
    /// side the normal points to.
    fn quad(&mut self, c: [[f32; 3]; 4], n: [f32; 3], col: [f32; 4]) {
        let base = self.positions.len() as u32;
        self.positions.extend(c);
        self.normals.extend([n; 4]);
        self.colors.extend([col; 4]);
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// The three meshes of one frame. Buffers are reused between frames.
#[derive(Default)]
pub struct Slab {
    pub solid: MeshData,
    pub glow: MeshData,
    pub gas: MeshData,
}

/// How deep (in cells, along z) each layer is extruded.
pub const SOLID_DEPTH: f32 = 6.0;
pub const GLOW_DEPTH: f32 = 4.0;
pub const GAS_DEPTH: f32 = 3.0;

pub fn layer_of(s: Species, t: f32) -> Layer {
    match s {
        Species::Air => Layer::None,
        Species::Flame => Layer::Glow,
        _ if t > 400.0 && !s.is_gas() => Layer::Glow,
        Species::Steam | Species::H2 | Species::O2 | Species::CH4 | Species::CO2 | Species::Cl2 => {
            Layer::Gas
        }
        _ => Layer::Solid,
    }
}

/// A run key: cells merge into one quad while this stays the same.
fn key(s: Species, t: f32) -> u16 {
    let heat = if t > 300.0 {
        ((t - 300.0) / 150.0) as u16 + 1
    } else {
        0
    };
    (s as u16) << 8 | heat.min(255)
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn color_of(s: Species, t: f32, row: usize) -> [f32; 4] {
    let look = palette::look(s);
    let mut rgb = look.base;
    // A little per-row shading so big blocks read as layered material.
    if palette::hash(0, row) & 3 == 0 {
        rgb = mix(rgb, look.accent, 0.35);
    }
    if t > 300.0 && s != Species::Flame {
        rgb = mix(rgb, heat_color(t), ((t - 300.0) / 700.0).min(0.85));
    }
    let alpha = if look.sky_through > 0.0 {
        (1.0 - look.sky_through).clamp(0.2, 0.7)
    } else {
        1.0
    };
    [
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
        alpha,
    ]
}

impl Slab {
    pub fn quads(&self) -> usize {
        self.solid.quads() + self.glow.quads() + self.gas.quads()
    }

    /// Rebuilds the meshes for world columns `x0..x1` (all rows). World
    /// space: x = column, y = rows counted up from the bottom, z = depth
    /// (front faces toward +z, where the camera is).
    pub fn build(&mut self, g: &Grid, x0: usize, x1: usize) {
        self.solid.clear();
        self.glow.clear();
        self.gas.clear();
        let x1 = x1.min(g.w);
        let h = g.h;
        let layer_at = |x: i32, y: i32| -> Layer {
            if x < 0 || y < 0 || x as usize >= g.w || y as usize >= h {
                return Layer::None;
            }
            let i = g.idx(x as usize, y as usize);
            layer_of(g.cells[i], g.temp[i])
        };
        // Solids and glow hide each other's faces; gas hides nothing.
        let opaque = |l: Layer| matches!(l, Layer::Solid | Layer::Glow);

        for y in 0..h {
            let mut x = x0;
            while x < x1 {
                let i = g.idx(x, y);
                let (s, t) = (g.cells[i], g.temp[i]);
                let layer = layer_of(s, t);
                let k = key(s, t);
                let mut end = x + 1;
                while end < x1 {
                    let j = g.idx(end, y);
                    if key(g.cells[j], g.temp[j]) != k {
                        break;
                    }
                    end += 1;
                }
                if layer != Layer::None {
                    let (mesh, depth) = match layer {
                        Layer::Solid => (&mut self.solid, SOLID_DEPTH),
                        Layer::Glow => (&mut self.glow, GLOW_DEPTH),
                        _ => (&mut self.gas, GAS_DEPTH),
                    };
                    let col = color_of(s, t, y);
                    let (l, r) = (x as f32, end as f32);
                    let top = (h - y) as f32;
                    let bot = top - 1.0;
                    let (zf, zb) = (depth / 2.0, -depth / 2.0);
                    // Front.
                    mesh.quad(
                        [[l, bot, zf], [r, bot, zf], [r, top, zf], [l, top, zf]],
                        [0.0, 0.0, 1.0],
                        col,
                    );
                    if layer != Layer::Gas {
                        // Top and bottom faces, split where they are covered.
                        for (dy, yy, n) in [(-1, top, 1.0f32), (1, bot, -1.0)] {
                            let mut a = x;
                            while a < end {
                                if opaque(layer_at(a as i32, y as i32 + dy)) {
                                    a += 1;
                                    continue;
                                }
                                let mut b = a + 1;
                                while b < end && !opaque(layer_at(b as i32, y as i32 + dy)) {
                                    b += 1;
                                }
                                let (fa, fb) = (a as f32, b as f32);
                                let shade = if n > 0.0 { 1.0 } else { 0.6 };
                                let c = [col[0] * shade, col[1] * shade, col[2] * shade, col[3]];
                                if n > 0.0 {
                                    mesh.quad(
                                        [[fa, yy, zf], [fb, yy, zf], [fb, yy, zb], [fa, yy, zb]],
                                        [0.0, 1.0, 0.0],
                                        c,
                                    );
                                } else {
                                    mesh.quad(
                                        [[fa, yy, zb], [fb, yy, zb], [fb, yy, zf], [fa, yy, zf]],
                                        [0.0, -1.0, 0.0],
                                        c,
                                    );
                                }
                                a = b;
                            }
                        }
                        // End faces.
                        if !opaque(layer_at(x as i32 - 1, y as i32)) {
                            mesh.quad(
                                [[l, bot, zb], [l, bot, zf], [l, top, zf], [l, top, zb]],
                                [-1.0, 0.0, 0.0],
                                col,
                            );
                        }
                        if !opaque(layer_at(end as i32, y as i32)) {
                            mesh.quad(
                                [[r, bot, zf], [r, bot, zb], [r, top, zb], [r, top, zf]],
                                [1.0, 0.0, 0.0],
                                col,
                            );
                        }
                    }
                }
                x = end;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_sim::game::{Game, Tier, VIEW_W};

    #[test]
    fn builds_a_bounded_mesh_for_the_first_room() {
        let mut game = Game::new(Tier::parse("apprentice"), 1);
        for _ in 0..120 {
            game.tick(0);
        }
        let mut slab = Slab::default();
        let x0 = game.view_x as usize;
        slab.build(&game.grid, x0, x0 + VIEW_W);
        let q = slab.quads();
        assert!(q > 100, "too few quads: {q}");
        assert!(q < 60_000, "too many quads: {q}");
        assert_eq!(slab.solid.indices.len(), slab.solid.quads() * 6);
    }
}
