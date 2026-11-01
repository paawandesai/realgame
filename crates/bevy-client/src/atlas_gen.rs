//! The block atlas, generated in code (I-054): no image files. One tile per
//! material row of `content/materials.json` (tile = the row's `atlas` index),
//! then the extra face tiles (grass, snow and soil sides, log rings, the
//! Furnace front). Style: toon blocks in one warm palette (warm stone, soft
//! greens, glowing teal crystal, amber torch light): a few flat value bands
//! per tile, shaded by gentle tileable value noise and a 4 x 4 ordered
//! dither, a thin darker rim, and one small seeded detail per material.
//! Deterministic from the seed. Pure Rust, no Bevy types: Lane A's
//! `voxel_tex.rs` wraps `Atlas::rgba` (and `Atlas::glow` for the emissive
//! texture) in an `Image`.

/// Tiles per atlas side (16 x 16 tiles).
pub const GRID: u32 = 16;
/// Tiles in use: the 27 material rows, then the extra faces below.
pub const TILES: usize = 31;
/// Extra face tiles, after the material rows.
pub const GRASS_SIDE: u8 = 27;
pub const LOG_TOP: u8 = 28;
pub const SNOW_SIDE: u8 = 29;
/// The Furnace's front (`valley::mat` row 27, not yet in materials.json).
pub const FURNACE: u8 = 30;

/// Which face of a block a tile is for.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    Side,
}

/// The tile for face `face` of material `mat`: the row's own tile, except
/// grass and snow sides (soil with a grass or snow lip), grass and snow
/// bottoms (soil) and the ends of a log (rings).
#[allow(dead_code)]
pub fn tile_for(mat: u8, face: Face) -> u8 {
    use realgame_voxel::mat as m;
    match (mat, face) {
        (m::GRASS, Face::Side) => GRASS_SIDE,
        (m::SNOW, Face::Side) => SNOW_SIDE,
        (m::GRASS | m::SNOW, Face::Bottom) => m::DIRT,
        (m::WOOD, Face::Top | Face::Bottom) => LOG_TOP,
        (27, _) => FURNACE,
        _ => mat,
    }
}

/// The UV rectangle of tile `t`: `[u0, v0, u1, v1]`, v down.
#[allow(dead_code)]
pub fn uv_rect(t: u8) -> [f32; 4] {
    let (c, r) = ((t as u32 % GRID) as f32, (t as u32 / GRID) as f32);
    let s = 1.0 / GRID as f32;
    [c * s, r * s, (c + 1.0) * s, (r + 1.0) * s]
}

/// A generated atlas: `size` x `size` RGBA8 pixels (`tile` px per tile).
/// `glow` is the emissive texture: black except where a tile glows (runes,
/// crystal cores, flames, hearth mouths, circuits).
pub struct Atlas {
    pub size: u32,
    pub tile: u32,
    pub rgba: Vec<u8>,
    pub glow: Vec<u8>,
}

impl Atlas {
    /// Pixel `(x, y)` of tile `t`, RGBA (tests and previews).
    #[allow(dead_code)]
    pub fn px(&self, t: u8, x: u32, y: u32) -> [u8; 4] {
        let i = self.index(t, x, y);
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    fn index(&self, t: u8, x: u32, y: u32) -> usize {
        let ox = (t as u32 % GRID) * self.tile + x;
        let oy = (t as u32 / GRID) * self.tile + y;
        ((oy * self.size + ox) * 4) as usize
    }
}

/// The 256 x 256 atlas (16 px tiles).
pub fn generate_atlas(seed: u64) -> Atlas {
    generate_atlas_sized(seed, 16)
}

/// The atlas with `tile` px tiles (16 or 32; any multiple of 16 works).
/// Tiles are drawn on a 16-unit design grid, so a 32 px atlas is the same
/// picture with finer dither.
pub fn generate_atlas_sized(seed: u64, tile: u32) -> Atlas {
    let tile = tile.max(16) / 16 * 16;
    let size = tile * GRID;
    let mut a = Atlas {
        size,
        tile,
        rgba: vec![0; (size * size * 4) as usize],
        glow: vec![0; (size * size * 4) as usize],
    };
    for t in 0..TILES as u8 {
        let cx = Ctx {
            seed,
            t,
            k: (tile / 16) as f32,
        };
        for y in 0..tile {
            for x in 0..tile {
                let mut p = paint(&cx, x, y);
                rim(&cx, &mut p, t, x, y, tile);
                let i = a.index(t, x, y);
                let c = p.c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                a.rgba[i..i + 3].copy_from_slice(&c);
                a.rgba[i + 3] = (p.a.clamp(0.0, 1.0) * 255.0).round() as u8;
                let g = p.c.map(|v| (v * p.glow).clamp(0.0, 1.0));
                a.glow[i..i + 3].copy_from_slice(&g.map(|v| (v * 255.0).round() as u8));
                a.glow[i + 3] = 255;
            }
        }
    }
    a
}

type C = [f32; 3];

/// One pixel: colour, alpha and how much of it glows (0-1).
struct Px {
    c: C,
    a: f32,
    glow: f32,
}

fn px(c: C) -> Px {
    Px {
        c,
        a: 1.0,
        glow: 0.0,
    }
}

const fn hex(h: u32) -> C {
    [
        ((h >> 16) & 255) as f32 / 255.0,
        ((h >> 8) & 255) as f32 / 255.0,
        (h & 255) as f32 / 255.0,
    ]
}

fn mix(a: C, b: C, t: f32) -> C {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(a: C, f: f32) -> C {
    a.map(|v| v * f)
}

/// Three tones of one material: shadow, base and highlight.
#[derive(Clone, Copy)]
struct Pal(C, C, C);

const DIRT: Pal = Pal(hex(0x5c3c2c), hex(0x86593c), hex(0xa4744e));
const GRASS: Pal = Pal(hex(0x4a8a3e), hex(0x6aae4c), hex(0x96cc68));
const SAND: Pal = Pal(hex(0xc4a274), hex(0xdcc290), hex(0xeed8aa));
const STONE: Pal = Pal(hex(0x72686c), hex(0x988c86), hex(0xb8ac9e));
const CLIFF: Pal = Pal(hex(0x865c4e), hex(0xa47862), hex(0xc0987a));
const RUNE: Pal = Pal(hex(0x30303c), hex(0x444452), hex(0x5a5868));
const BARK: Pal = Pal(hex(0x4a3224), hex(0x6a4830), hex(0x86603e));
const RINGS: Pal = Pal(hex(0x8e6440), hex(0xb48656), hex(0xd0a46e));
const LEAVES: Pal = Pal(hex(0x2c6a3c), hex(0x3c8e4e), hex(0x68b85a));
const PLANKS: Pal = Pal(hex(0x966640), hex(0xb4824e), hex(0xd09e66));
const SNOW: Pal = Pal(hex(0xc4d4e6), hex(0xe8eef4), hex(0xfcfdff));
const ICE: Pal = Pal(hex(0x88c4dc), hex(0xb0dff0), hex(0xdcf4fc));
const WATER: Pal = Pal(hex(0x2a66ac), hex(0x3886cc), hex(0x66b0e4));
const STEAM: Pal = Pal(hex(0xc4ccd6), hex(0xe0e6ec), hex(0xfafcff));
const MUD: Pal = Pal(hex(0x3c2a20), hex(0x52392a), hex(0x6c4e36));
const CRYSTAL: Pal = Pal(hex(0x1c8484), hex(0x34c0b4), hex(0x8cf4e2));
const IRON: Pal = Pal(hex(0x666a76), hex(0x868a96), hex(0xacb0ba));
const BRICK: Pal = Pal(hex(0x9a4a34), hex(0xb86244), hex(0xd07c56));
const ASH: Pal = Pal(hex(0x686462), hex(0x86827e), hex(0xa29e98));
const PURPLE_WOOD: Pal = Pal(hex(0x30283a), hex(0x423650), hex(0x584a68));
const FURNACE_STONE: Pal = Pal(hex(0x5e5a62), hex(0x7c7680), hex(0x9a94a0));

const TEAL: C = hex(0x5cf0d6);
const AMBER: C = hex(0xffa63c);
const FLAME_CORE: C = hex(0xfff0b4);
const MORTAR: C = hex(0xd6c2aa);
const COAL: C = hex(0x24202a);
const RUST: C = hex(0xd88a58);
const RUST_DARK: C = hex(0xa8603e);
const VENT_GLOW: C = hex(0x8ee07a);

/// What one tile's painter knows: the seed, the tile and the px per unit.
struct Ctx {
    seed: u64,
    t: u8,
    k: f32,
}

fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The 4 x 4 ordered dither threshold at pixel (x, y), in -0.5..0.5.
fn bayer(x: u32, y: u32) -> f32 {
    const M: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    (M[((y % 4) * 4 + x % 4) as usize] as f32 + 0.5) / 16.0 - 0.5
}

impl Ctx {
    /// A hash in 0..1 of integer lattice point (i, j) with `salt`.
    fn h(&self, i: i32, j: i32, salt: u32) -> f32 {
        let z = mix64(
            self.seed
                ^ ((self.t as u64) << 56)
                ^ ((salt as u64) << 40)
                ^ (((i as u32) as u64) << 20)
                ^ ((j as u32) as u64),
        );
        (z >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Tileable value noise (0..1) at design units (u, v), cells `cell`
    /// units across (16 must be a multiple of it, so tiles repeat).
    fn n(&self, u: f32, v: f32, cell: f32, salt: u32) -> f32 {
        let p = (16.0 / cell).round() as i32;
        let (gu, gv) = (u / cell, v / cell);
        let (i, j) = (gu.floor() as i32, gv.floor() as i32);
        let (fu, fv) = (gu - i as f32, gv - j as f32);
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let at = |a: i32, b: i32| self.h(a.rem_euclid(p), b.rem_euclid(p), salt);
        let top = at(i, j) + (at(i + 1, j) - at(i, j)) * s(fu);
        let bot = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * s(fu);
        top + (bot - top) * s(fv)
    }

    /// Two octaves of `n`, 0..1.
    fn fbm(&self, u: f32, v: f32, salt: u32) -> f32 {
        0.65 * self.n(u, v, 8.0, salt) + 0.35 * self.n(u, v, 4.0, salt + 1)
    }

    /// The toon shade: `s` (0..1, 0.5 = base) dithered into 5 bands across
    /// shadow, base and highlight.
    fn toon(&self, pal: Pal, s: f32, x: u32, y: u32) -> C {
        let bands = 4.0;
        let q = ((s.clamp(0.0, 1.0) * bands + bayer(x, y) * 0.9).round() / bands).clamp(0.0, 1.0);
        if q < 0.5 {
            mix(pal.0, pal.1, q * 2.0)
        } else {
            mix(pal.1, pal.2, (q - 0.5) * 2.0)
        }
    }

    /// The material's base shading: gentle noise around the base tone.
    fn base(&self, pal: Pal, u: f32, v: f32, x: u32, y: u32, amp: f32) -> C {
        self.toon(pal, 0.5 + (self.fbm(u, v, 1) - 0.5) * amp, x, y)
    }

    /// Seeded blobs: whether (u, v) lies in one of up to `n` round spots of
    /// radius `r` (units) on a 4-unit jittered grid (tileable).
    fn spots(&self, u: f32, v: f32, r: f32, chance: f32, salt: u32) -> f32 {
        let (ci, cj) = ((u / 4.0).floor() as i32, (v / 4.0).floor() as i32);
        let mut best = f32::MAX;
        for dj in -1..=1 {
            for di in -1..=1 {
                let (i, j) = (ci + di, cj + dj);
                let (wi, wj) = (i.rem_euclid(4), j.rem_euclid(4));
                if self.h(wi, wj, salt) > chance {
                    continue;
                }
                let cu = i as f32 * 4.0 + 0.8 + self.h(wi, wj, salt + 1) * 2.4;
                let cv = j as f32 * 4.0 + 0.8 + self.h(wi, wj, salt + 2) * 2.4;
                let r2 = r * (0.7 + 0.6 * self.h(wi, wj, salt + 3));
                best = best.min(((u - cu).powi(2) + (v - cv).powi(2)).sqrt() / r2);
            }
        }
        best
    }
}

fn paint(cx: &Ctx, x: u32, y: u32) -> Px {
    use realgame_voxel::mat as m;
    let (u, v) = ((x as f32 + 0.5) / cx.k, (y as f32 + 0.5) / cx.k);
    match cx.t {
        m::AIR => Px {
            c: [0.0; 3],
            a: 0.0,
            glow: 0.0,
        },
        m::DIRT => px(dirt(cx, u, v, x, y)),
        m::GRASS => {
            let mut c = cx.base(GRASS, u, v, x, y, 0.75);
            let tuft = cx.h((u * 1.0) as i32, (v * 1.0) as i32, 9);
            if tuft > 0.86 {
                c = GRASS.2;
            } else if tuft < 0.07 {
                c = GRASS.0;
            }
            px(c)
        }
        m::SAND => {
            let ripple =
                ((v * 0.9 + (cx.n(u, v, 8.0, 5) - 0.5) * 3.0) * std::f32::consts::TAU / 4.0).sin();
            px(cx.toon(
                SAND,
                0.5 + ripple * 0.18 + (cx.fbm(u, v, 1) - 0.5) * 0.4,
                x,
                y,
            ))
        }
        m::STONE => px(stone(cx, STONE, u, v, x, y)),
        m::CLIFFSTONE => {
            // Layered strata, gently wavy.
            let layer = v + (cx.n(u, 0.0, 8.0, 6) - 0.5) * 3.0;
            let band = ((layer / 4.0).floor() as i32).rem_euclid(4);
            let s = [0.62, 0.42, 0.55, 0.35][band as usize] + (cx.fbm(u, v, 1) - 0.5) * 0.5;
            let seam = (layer.rem_euclid(4.0)) < 0.7;
            px(if seam {
                CLIFF.0
            } else {
                cx.toon(CLIFF, s, x, y)
            })
        }
        m::RUNESTONE => runestone(cx, u, v, x, y),
        m::WOOD => {
            // Bark: vertical ridges.
            let ridge = cx.n(u, v * 0.25, 4.0, 7);
            let s = 0.25 + ridge * 0.6 + (cx.fbm(u, v, 1) - 0.5) * 0.25;
            let groove = (u + (cx.n(u, v, 8.0, 8) - 0.5) * 2.0).rem_euclid(4.0) < 0.9;
            px(if groove {
                BARK.0
            } else {
                cx.toon(BARK, s, x, y)
            })
        }
        m::LEAVES => {
            let d = cx.spots(u, v, 1.8, 0.9, 20);
            let s = if d < 0.55 {
                0.9
            } else if d < 1.0 {
                0.55
            } else {
                0.2
            };
            px(cx.toon(LEAVES, s + (cx.fbm(u, v, 1) - 0.5) * 0.3, x, y))
        }
        m::PLANKS => planks(cx, PLANKS, u, v, x, y),
        m::SNOW => {
            let mut c = cx.base(SNOW, u, v, x, y, 0.7);
            if cx.h(u as i32, v as i32, 11) > 0.93 {
                c = hex(0xb8cce4);
            }
            px(c)
        }
        m::ICE => {
            let streak = ((u + v) * 0.5 + (cx.n(u, v, 8.0, 12) - 0.5) * 2.0).rem_euclid(5.0);
            let s = if streak < 0.6 {
                1.0
            } else {
                0.35 + (cx.fbm(u, v, 1) - 0.5) * 0.4
            };
            Px {
                c: cx.toon(ICE, s, x, y),
                a: 0.8,
                glow: 0.0,
            }
        }
        m::WATER => {
            let wave =
                ((u * 0.5 + (cx.n(u, v, 8.0, 13) - 0.5) * 4.0) * std::f32::consts::TAU / 8.0 + v)
                    .sin();
            let crest = (v + wave * 1.2).rem_euclid(5.0) < 0.6;
            let s = if crest {
                0.95
            } else {
                0.4 + (cx.fbm(u, v, 1) - 0.5) * 0.5
            };
            Px {
                c: cx.toon(WATER, s, x, y),
                a: 0.74,
                glow: 0.0,
            }
        }
        m::STEAM => {
            let s = cx.fbm(u, v, 14);
            Px {
                c: cx.toon(STEAM, s, x, y),
                a: 0.25 + 0.3 * s,
                glow: 0.0,
            }
        }
        m::MUD => {
            let mut c = cx.base(MUD, u, v, x, y, 0.8);
            let d = cx.spots(u, v, 1.0, 0.45, 15);
            if d < 0.5 {
                c = hex(0x8a6a4e); // wet shine
            } else if d < 1.0 {
                c = MUD.2;
            }
            px(c)
        }
        m::COAL_ORE => ore(cx, u, v, x, y, 1.2, COAL, hex(0x48424e)),
        m::IRON_ORE => ore(cx, u, v, x, y, 1.7, RUST, RUST_DARK),
        m::CRYSTAL => crystal(cx, u, v, x, y),
        m::GLASS => {
            let edge = u < 1.5 || v < 1.5 || u > 14.5 || v > 14.5;
            let glint = ((u - v).abs() < 0.8 && u > 3.0 && u < 8.0)
                || ((u - v - 3.0).abs() < 0.5 && u > 7.0 && u < 10.0);
            if edge {
                Px {
                    c: hex(0xe6f4fa),
                    a: 0.85,
                    glow: 0.0,
                }
            } else if glint {
                Px {
                    c: hex(0xffffff),
                    a: 0.7,
                    glow: 0.0,
                }
            } else {
                Px {
                    c: hex(0xc8e4ee),
                    a: 0.22,
                    glow: 0.0,
                }
            }
        }
        m::IRON => {
            let bevel = u < 1.5 || v < 1.5;
            let shade = u > 14.5 || v > 14.5;
            let rivet = [(3.0, 3.0), (13.0, 3.0), (3.0, 13.0), (13.0, 13.0)]
                .iter()
                .any(|&(a, b)| (u - a).powi(2) + (v - b).powi(2) < 1.2);
            let c = if rivet {
                IRON.2
            } else if bevel {
                mix(IRON.2, IRON.1, 0.3)
            } else if shade {
                IRON.0
            } else {
                // Brushed: faint horizontal streaks.
                cx.toon(
                    IRON,
                    0.5 + (cx.n(u * 0.25, v * 2.0, 4.0, 16) - 0.5) * 0.6,
                    x,
                    y,
                )
            };
            px(c)
        }
        m::BRICK => bricks(cx, BRICK, MORTAR, u, v, x, y),
        m::ASH => {
            let mut c = cx.base(ASH, u, v, x, y, 1.0);
            if cx.h(u as i32, v as i32, 17) > 0.9 {
                c = hex(0x4c4846);
            }
            px(c)
        }
        m::TORCH => torch(cx, u, v, x, y),
        m::BENCH => {
            let mut p = planks(cx, PLANKS, u, v, x, y);
            // A teal measuring grid inlaid in the top.
            let line = |w: f32| matches!(w.floor() as i32, 4 | 8 | 12);
            if (line(u) || line(v)) && u > 2.0 && u < 14.0 && v > 2.0 && v < 14.0 {
                p = Px {
                    c: mix(TEAL, PLANKS.1, 0.35),
                    a: 1.0,
                    glow: 0.25,
                };
            }
            p
        }
        m::INVENTION_BENCH => invention(cx, u, v, x, y),
        m::KILN_HEARTH => {
            let p = bricks(cx, BRICK, MORTAR, u, v, x, y);
            hearth(cx, p, u, v, x, y)
        }
        m::GAS_VENT => {
            let mut c = stone(cx, STONE, u, v, x, y);
            let r = ((u - 8.0).powi(2) + (v - 8.0).powi(2)).sqrt();
            let mut glow = 0.0;
            if r < 5.5 {
                let bar = (u.rem_euclid(2.0)) < 1.0;
                if r > 4.6 {
                    c = hex(0x5a5460);
                } else if bar {
                    c = hex(0x2c2a32);
                } else {
                    c = mix(VENT_GLOW, hex(0x2c2a32), (r / 4.6).powi(2));
                    glow = 0.5 * (1.0 - r / 4.6);
                }
            }
            Px { c, a: 1.0, glow }
        }
        GRASS_SIDE => {
            // Soil with a grass lip that hangs down unevenly.
            let lip = 3.0 + cx.n(u, 0.0, 4.0, 18) * 2.6;
            if v < lip {
                px(cx.base(GRASS, u, v, x, y, 0.7))
            } else if v < lip + 0.8 {
                px(GRASS.0)
            } else {
                px(dirt(cx, u, v, x, y))
            }
        }
        LOG_TOP => {
            let r =
                ((u - 8.0).powi(2) + (v - 8.0).powi(2)).sqrt() + (cx.n(u, v, 8.0, 19) - 0.5) * 1.4;
            if r > 6.8 {
                // The bark rim.
                px(cx.toon(BARK, 0.45, x, y))
            } else {
                let ring = r.rem_euclid(2.0) < 0.6;
                px(if ring {
                    RINGS.0
                } else {
                    cx.toon(
                        RINGS,
                        0.45 + (cx.fbm(u, v, 1) - 0.5) * 0.4 + (1.0 - r / 7.0) * 0.2,
                        x,
                        y,
                    )
                })
            }
        }
        SNOW_SIDE => {
            let lip = 3.5 + cx.n(u, 0.0, 4.0, 21) * 2.0;
            if v < lip {
                px(cx.base(SNOW, u, v, x, y, 0.6))
            } else if v < lip + 0.7 {
                px(SNOW.0)
            } else {
                px(dirt(cx, u, v, x, y))
            }
        }
        FURNACE => {
            let p = bricks(cx, FURNACE_STONE, hex(0x4a4650), u, v, x, y);
            hearth(cx, p, u, v, x, y)
        }
        _ => px([1.0, 0.0, 1.0]),
    }
}

fn dirt(cx: &Ctx, u: f32, v: f32, x: u32, y: u32) -> C {
    let mut c = cx.base(DIRT, u, v, x, y, 0.9);
    let d = cx.spots(u, v, 0.9, 0.5, 30);
    if d < 0.6 {
        c = hex(0xb08a66); // a pebble
    } else if d < 1.0 {
        c = DIRT.0;
    }
    c
}

fn stone(cx: &Ctx, pal: Pal, u: f32, v: f32, x: u32, y: u32) -> C {
    let mut c = cx.base(pal, u, v, x, y, 1.0);
    // A couple of rounded inset pebbles with a lit upper edge.
    let d = cx.spots(u, v, 2.2, 0.4, 31);
    if d < 0.85 {
        c = cx.toon(pal, 0.62, x, y);
    } else if d < 1.0 {
        let du = cx.spots(u, v - 0.6, 2.2, 0.4, 31);
        c = if du < 0.85 { pal.0 } else { pal.2 };
    }
    c
}

fn planks(cx: &Ctx, pal: Pal, u: f32, v: f32, x: u32, y: u32) -> Px {
    let row = (v / 4.0).floor() as i32;
    let off = cx.h(row.rem_euclid(4), 0, 40) * 16.0;
    let seam_v = v.rem_euclid(4.0) < 0.6;
    let seam_u = (u + off).rem_euclid(16.0) < 0.6;
    let shade = [0.55, 0.42, 0.6, 0.48][row.rem_euclid(4) as usize];
    let grain = (cx.n(u * 0.3, v * 3.0, 4.0, 41) - 0.5) * 0.5;
    let nail = [(2.0, 2.0), (14.0, 2.0)].iter().any(|&(a, b)| {
        let nu = (u + off).rem_euclid(16.0);
        (nu - a).abs() < 0.6 && (v.rem_euclid(4.0) - b).abs() < 0.6
    });
    px(if seam_v || seam_u {
        pal.0
    } else if nail {
        hex(0x5a4a40)
    } else {
        cx.toon(pal, shade + grain, x, y)
    })
}

fn bricks(cx: &Ctx, pal: Pal, mortar: C, u: f32, v: f32, x: u32, y: u32) -> Px {
    let row = (v / 4.0).floor() as i32;
    let off = if row % 2 == 0 { 0.0 } else { 4.0 };
    let col = ((u + off) / 8.0).floor() as i32;
    let seam = v.rem_euclid(4.0) < 0.8 || (u + off).rem_euclid(8.0) < 0.8;
    if seam {
        return px(mortar);
    }
    let s = 0.3 + cx.h(col.rem_euclid(2), row.rem_euclid(4), 50) * 0.4;
    // Each brick lit on its top edge.
    let top = v.rem_euclid(4.0) < 1.6;
    px(cx.toon(
        pal,
        s + if top { 0.2 } else { 0.0 } + (cx.fbm(u, v, 1) - 0.5) * 0.3,
        x,
        y,
    ))
}

#[allow(clippy::too_many_arguments)]
fn ore(cx: &Ctx, u: f32, v: f32, x: u32, y: u32, r: f32, ore: C, ore_dark: C) -> Px {
    let mut c = stone(cx, STONE, u, v, x, y);
    let d = cx.spots(u, v, r, 0.5, 60);
    if d < 0.55 {
        c = mix(ore, [1.0; 3], 0.15);
    } else if d < 0.85 {
        c = ore;
    } else if d < 1.05 {
        c = ore_dark;
    }
    px(c)
}

/// Dark slate with a carved, faintly glowing teal glyph, mirrored so it
/// reads as a rune (seeded per world).
fn runestone(cx: &Ctx, u: f32, v: f32, x: u32, y: u32) -> Px {
    let c = cx.base(RUNE, u, v, x, y, 0.8);
    let (gu, gv) = (u.floor() as i32 - 3, v.floor() as i32 - 3);
    if (0..10).contains(&gu) && (0..10).contains(&gv) {
        // A 5 x 10 half-glyph of strokes, mirrored left to right.
        let hu = if gu < 5 { gu } else { 9 - gu };
        let on = |a: i32, b: i32| -> bool {
            let spine = a == 4;
            let bar = b % 3 == 0 && cx.h(0, b, 70) > 0.35;
            let tick = cx.h(a, b, 71) > 0.82;
            (spine && b > 0 && b < 9) || (bar && a >= 1) || tick
        };
        if on(hu, gv) {
            return Px {
                c: TEAL,
                a: 1.0,
                glow: 0.85,
            };
        }
        // A carved shadow under each stroke.
        if gv > 0 && on(hu, gv - 1) {
            return px(RUNE.0);
        }
    }
    px(c)
}

/// Teal crystal: facets around a bright core that glows.
fn crystal(cx: &Ctx, u: f32, v: f32, x: u32, y: u32) -> Px {
    let (du, dv) = (u - 8.0, v - 8.0);
    let diamond = du.abs() + dv.abs();
    // Facet planes: each quadrant has its own light.
    let quad = match (du >= 0.0, dv >= 0.0) {
        (false, false) => 0.85,
        (true, false) => 0.6,
        (false, true) => 0.45,
        (true, true) => 0.25,
    };
    let s = quad + (cx.fbm(u, v, 1) - 0.5) * 0.25;
    if diamond < 2.6 {
        return Px {
            c: hex(0xeafffa),
            a: 1.0,
            glow: 1.0,
        };
    }
    if diamond < 4.6 {
        return Px {
            c: CRYSTAL.2,
            a: 0.95,
            glow: 0.7,
        };
    }
    let edge = (du.abs() - dv.abs()).abs() < 0.6 || du.abs() < 0.4 || dv.abs() < 0.4;
    let c = if edge {
        CRYSTAL.2
    } else {
        cx.toon(CRYSTAL, s, x, y)
    };
    Px {
        c,
        a: 0.88,
        glow: if edge { 0.35 } else { 0.15 },
    }
}

/// A torch on a clear tile: a wooden stick and an amber flame.
fn torch(cx: &Ctx, u: f32, v: f32, x: u32, y: u32) -> Px {
    let stick = (u - 8.0).abs() < 1.1 && v > 7.0 && v < 15.5;
    if stick {
        let c = if (u - 8.0) < -0.3 { BARK.2 } else { BARK.1 };
        return px(c);
    }
    let fu = (u - 8.0) / (2.4 + (7.0 - v).max(0.0) * -0.15);
    let fv = (v - 5.0) / 3.4;
    let r = fu * fu + fv * fv;
    if r < 1.0 && v < 7.6 {
        let flick = cx.n(u, v, 4.0, 80) * 0.25;
        let c = if r < 0.3 + flick {
            FLAME_CORE
        } else if r < 0.65 + flick {
            hex(0xffcf5c)
        } else {
            AMBER
        };
        return Px {
            c,
            a: 1.0,
            glow: 1.0,
        };
    }
    let _ = (x, y);
    Px {
        c: AMBER,
        a: 0.0,
        glow: 0.0,
    }
}

/// The purple-wood Invention Bench top with glowing teal circuit traces.
fn invention(cx: &Ctx, u: f32, v: f32, x: u32, y: u32) -> Px {
    let base = planks(cx, PURPLE_WOOD, u, v, x, y);
    let (gu, gv) = (u.floor() as i32, v.floor() as i32);
    let trace_h = gv % 5 == 2 && gu > 1 && gu < 14 && cx.h(gu / 4, gv, 90) > 0.3;
    let trace_v = gu % 5 == 3 && gv > 1 && gv < 14 && cx.h(gu, gv / 4, 91) > 0.45;
    let pad = gu % 5 == 3 && gv % 5 == 2;
    if pad {
        return Px {
            c: hex(0xc8fff4),
            a: 1.0,
            glow: 1.0,
        };
    }
    if trace_h || trace_v {
        return Px {
            c: TEAL,
            a: 1.0,
            glow: 0.7,
        };
    }
    let _ = (x, y);
    base
}

/// An arched opening at the bottom with an amber fire inside.
fn hearth(cx: &Ctx, p: Px, u: f32, v: f32, x: u32, y: u32) -> Px {
    let (du, dv) = (u - 8.0, v - 10.0);
    let arch = (du / 5.0).powi(2) + (dv.min(0.0) / 5.0).powi(2) < 1.0 && v < 15.0;
    if !arch {
        return p;
    }
    let rim = (du / 5.0).powi(2) + (dv.min(0.0) / 5.0).powi(2) > 0.72;
    if rim {
        return px(hex(0x3a2a26));
    }
    let heat = ((v - 6.0) / 9.0).clamp(0.0, 1.0) + (cx.n(u, v, 4.0, 95) - 0.5) * 0.4;
    let s = (heat + bayer(x, y) * 0.3).clamp(0.0, 1.0);
    let c = if s > 0.75 {
        FLAME_CORE
    } else if s > 0.45 {
        AMBER
    } else {
        scale(hex(0xb04a24), 0.9)
    };
    Px {
        c,
        a: 1.0,
        glow: 0.4 + 0.6 * s,
    }
}

/// The thin darker rim on opaque blocks (glass draws its own frame; torch
/// and steam have none).
fn rim(cx: &Ctx, p: &mut Px, t: u8, x: u32, y: u32, tile: u32) {
    use realgame_voxel::mat as m;
    if matches!(t, m::AIR | m::TORCH | m::STEAM | m::GLASS | m::WATER) || p.glow > 0.5 {
        return;
    }
    let w = cx.k as u32;
    let edge = x < w || y < w || x >= tile - w || y >= tile - w;
    if edge {
        p.c = scale(p.c, 0.84);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::materials::{load, MATERIALS_JSON};

    /// The palette range of every tile: its average colour (alpha weighted,
    /// 0-255) is within `tol` of `want` on each channel.
    fn want(t: u8) -> ([f32; 3], f32) {
        use realgame_voxel::mat as m;
        let p = |pal: Pal| pal.1;
        let c = match t {
            m::DIRT => p(DIRT),
            m::GRASS => p(GRASS),
            m::SAND => p(SAND),
            m::STONE | m::COAL_ORE | m::IRON_ORE | m::GAS_VENT => p(STONE),
            m::CLIFFSTONE => p(CLIFF),
            m::RUNESTONE => p(RUNE),
            m::WOOD => p(BARK),
            m::LEAVES => p(LEAVES),
            m::PLANKS | m::BENCH => p(PLANKS),
            m::SNOW => p(SNOW),
            m::ICE => p(ICE),
            m::WATER => p(WATER),
            m::STEAM => p(STEAM),
            m::MUD => p(MUD),
            m::CRYSTAL => p(CRYSTAL),
            m::GLASS => hex(0xd4eaf2),
            m::IRON => p(IRON),
            m::BRICK | m::KILN_HEARTH => p(BRICK),
            m::ASH => p(ASH),
            m::TORCH => hex(0xb08048),
            m::INVENTION_BENCH => p(PURPLE_WOOD),
            GRASS_SIDE => mix(p(DIRT), p(GRASS), 0.35),
            // The rings, with the bark rim around them.
            LOG_TOP => mix(p(RINGS), p(BARK), 0.35),
            SNOW_SIDE => mix(p(DIRT), p(SNOW), 0.35),
            FURNACE => p(FURNACE_STONE),
            _ => [0.0; 3],
        };
        (c.map(|v| v * 255.0), 40.0)
    }

    fn avg(a: &Atlas, t: u8) -> ([f32; 3], f32) {
        let (mut s, mut w) = ([0.0f32; 3], 0.0f32);
        for y in 0..a.tile {
            for x in 0..a.tile {
                let p = a.px(t, x, y);
                let al = p[3] as f32 / 255.0;
                for i in 0..3 {
                    s[i] += p[i] as f32 * al;
                }
                w += al;
            }
        }
        (s.map(|v| v / w.max(1e-6)), w)
    }

    #[test]
    fn i054_one_tile_per_material_row_at_its_atlas_index() {
        let rows = load(MATERIALS_JSON).expect("materials load");
        assert_eq!(rows.len(), realgame_voxel::mat::COUNT);
        for r in &rows {
            assert_eq!(r.atlas, r.id, "{}: tile = id", r.name);
            assert!((r.atlas as usize) < TILES);
        }
        // Then the extra faces, all inside the 16 x 16 grid.
        assert_eq!(TILES, rows.len() + 4);
        assert!(TILES <= (GRID * GRID) as usize);
        let a = generate_atlas(7);
        assert_eq!((a.size, a.tile, a.rgba.len()), (256, 16, 256 * 256 * 4));
    }

    #[test]
    fn i054_the_same_seed_gives_the_same_atlas_and_another_differs() {
        let (a, b, c) = (generate_atlas(42), generate_atlas(42), generate_atlas(43));
        assert!(a.rgba == b.rgba && a.glow == b.glow);
        assert!(a.rgba != c.rgba);
    }

    #[test]
    fn i054_only_air_is_fully_clear_and_opaque_rows_are_opaque() {
        let rows = load(MATERIALS_JSON).unwrap();
        let a = generate_atlas(1);
        for t in 0..TILES as u8 {
            let (_, w) = avg(&a, t);
            assert_eq!(w == 0.0, t == 0, "tile {t}: alpha sum {w}");
        }
        for r in rows.iter().filter(|r| !r.transparent) {
            for y in 0..16 {
                for x in 0..16 {
                    assert_eq!(a.px(r.atlas, x, y)[3], 255, "{} at {x},{y}", r.name);
                }
            }
        }
    }

    #[test]
    fn i054_each_tile_averages_inside_its_palette_range() {
        for seed in [1, 99, 12345] {
            let a = generate_atlas(seed);
            for t in 1..TILES as u8 {
                let ((got, _), (w, tol)) = (avg(&a, t), want(t));
                let off = (0..3).map(|i| (got[i] - w[i]).abs()).fold(0.0, f32::max);
                assert!(
                    off <= tol,
                    "seed {seed} tile {t}: avg {got:?} want {w:?} (off {off:.0})"
                );
            }
        }
    }

    #[test]
    fn i054_glowing_tiles_glow_and_plain_ones_do_not() {
        use realgame_voxel::mat as m;
        let a = generate_atlas(5);
        let lit = |t: u8| {
            (0..16 * 16)
                .filter(|i| {
                    let j = a.index(t, i % 16, i / 16);
                    a.glow[j] as u32 + a.glow[j + 1] as u32 + a.glow[j + 2] as u32 > 120
                })
                .count()
        };
        for t in [
            m::RUNESTONE,
            m::CRYSTAL,
            m::TORCH,
            m::KILN_HEARTH,
            m::INVENTION_BENCH,
            FURNACE,
        ] {
            assert!(lit(t) >= 4, "tile {t} glows: {}", lit(t));
        }
        for t in [m::DIRT, m::STONE, m::GRASS, m::WATER, m::SNOW, GRASS_SIDE] {
            assert_eq!(lit(t), 0, "tile {t}");
        }
    }

    #[test]
    fn i054_a_32px_atlas_draws_the_same_tiles() {
        let a = generate_atlas_sized(3, 32);
        assert_eq!((a.size, a.tile), (512, 32));
        let b = generate_atlas(3);
        for t in 1..TILES as u8 {
            let ((x, _), (y, _)) = (avg(&a, t), avg(&b, t));
            let off = (0..3).map(|i| (x[i] - y[i]).abs()).fold(0.0, f32::max);
            assert!(off < 24.0, "tile {t}: {x:?} vs {y:?}");
        }
    }

    #[test]
    fn i054_face_tiles() {
        use realgame_voxel::mat as m;
        assert_eq!(tile_for(m::GRASS, Face::Top), m::GRASS);
        assert_eq!(tile_for(m::GRASS, Face::Side), GRASS_SIDE);
        assert_eq!(tile_for(m::GRASS, Face::Bottom), m::DIRT);
        assert_eq!(tile_for(m::WOOD, Face::Top), LOG_TOP);
        assert_eq!(tile_for(m::WOOD, Face::Side), m::WOOD);
        assert_eq!(tile_for(m::SNOW, Face::Side), SNOW_SIDE);
        assert_eq!(tile_for(m::STONE, Face::Top), m::STONE);
        assert_eq!(
            uv_rect(17),
            [1.0 / 16.0, 1.0 / 16.0, 2.0 / 16.0, 2.0 / 16.0]
        );
    }

    /// Writes the preview PNGs (`cargo test --release i054_preview --
    /// --ignored`): the atlas at 4x, and a sheet of sample blocks.
    #[test]
    #[ignore]
    fn i054_preview() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/loop/items/I-054");
        let seed = std::env::var("SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2026);
        let a = generate_atlas(seed);
        // The used tiles on a checkerboard (so clear tiles show), 8x.
        let (cols, rows, z) = (8u32, TILES.div_ceil(8) as u32, 8u32);
        let (w, h) = (cols * 17 * z, rows * 17 * z);
        let mut img = vec![0u8; (w * h * 4) as usize];
        for py in 0..h {
            for px_ in 0..w {
                let (cx, cy) = (px_ / z / 17, py / z / 17);
                let (lx, ly) = ((px_ / z) % 17, (py / z) % 17);
                let t = cy * cols + cx;
                let chk = if ((px_ / 8) + (py / 8)) % 2 == 0 {
                    70.0
                } else {
                    90.0
                };
                let mut c = [chk, chk, chk + 6.0];
                if lx < 16 && ly < 16 && (t as usize) < TILES {
                    let p = a.px(t as u8, lx, ly);
                    let al = p[3] as f32 / 255.0;
                    for i in 0..3 {
                        c[i] = c[i] * (1.0 - al) + p[i] as f32 * al;
                    }
                } else if lx == 16 || ly == 16 {
                    c = [40.0, 38.0, 46.0];
                }
                let i = ((py * w + px_) * 4) as usize;
                img[i..i + 3].copy_from_slice(&c.map(|v| v as u8));
                img[i + 3] = 255;
            }
        }
        std::fs::write(format!("{dir}/atlas-preview.png"), png(w, h, &img)).unwrap();
        std::fs::write(format!("{dir}/atlas-256.png"), png(a.size, a.size, &a.rgba)).unwrap();
        std::fs::write(
            format!("{dir}/atlas-glow-256.png"),
            png(a.size, a.size, &a.glow),
        )
        .unwrap();
        std::fs::write(format!("{dir}/blocks-preview.png"), scene(&a)).unwrap();
    }

    /// A little isometric scene of blocks (top, left and right faces shaded
    /// as a toon light would), to judge the tiles together.
    fn scene(a: &Atlas) -> Vec<u8> {
        use realgame_voxel::mat as m;
        let (w, h) = (1024u32, 640u32);
        let mut img = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            let sky = mix(hex(0x8cc6f4), hex(0xf2d8b4), y as f32 / h as f32);
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                img[i..i + 3].copy_from_slice(&sky.map(|v| (v * 255.0) as u8));
                img[i + 3] = 255;
            }
        }
        // (column, row, height, material) in a 9 x 5 ground patch.
        let mut blocks: Vec<(i32, i32, i32, u8)> = Vec::new();
        let ground = [
            [
                m::GRASS,
                m::GRASS,
                m::GRASS,
                m::SAND,
                m::WATER,
                m::WATER,
                m::SAND,
                m::SNOW,
                m::SNOW,
            ],
            [
                m::GRASS,
                m::STONE,
                m::GRASS,
                m::SAND,
                m::WATER,
                m::ICE,
                m::SAND,
                m::SNOW,
                m::ICE,
            ],
            [
                m::DIRT,
                m::GRASS,
                m::MUD,
                m::MUD,
                m::SAND,
                m::SAND,
                m::CLIFFSTONE,
                m::SNOW,
                m::SNOW,
            ],
            [
                m::GRASS,
                m::GRASS,
                m::GRASS,
                m::STONE,
                m::COAL_ORE,
                m::IRON_ORE,
                m::CLIFFSTONE,
                m::STONE,
                m::CRYSTAL,
            ],
            [
                m::GRASS,
                m::PLANKS,
                m::PLANKS,
                m::BRICK,
                m::ASH,
                m::STONE,
                m::RUNESTONE,
                m::STONE,
                m::GAS_VENT,
            ],
        ];
        for (r, row) in ground.iter().enumerate() {
            for (c, &mt) in row.iter().enumerate() {
                blocks.push((c as i32, r as i32, 0, mt));
            }
        }
        for (c, r, z, mt) in [
            (0, 0, 1, m::WOOD),
            (0, 0, 2, m::WOOD),
            (0, 0, 3, m::LEAVES),
            (1, 0, 3, m::LEAVES),
            (0, 1, 3, m::LEAVES),
            (2, 3, 1, m::BENCH),
            (3, 3, 1, m::INVENTION_BENCH),
            (3, 4, 1, m::KILN_HEARTH),
            (3, 4, 2, m::BRICK),
            (6, 3, 1, m::CLIFFSTONE),
            (6, 2, 1, m::CLIFFSTONE),
            (6, 3, 2, m::RUNESTONE),
            (8, 3, 1, m::CRYSTAL),
            (8, 3, 2, m::CRYSTAL),
            (5, 4, 1, m::GLASS),
            (5, 4, 2, m::IRON),
            (1, 4, 1, m::TORCH),
            (8, 0, 1, m::SNOW),
            (4, 4, 1, 27),
            (7, 4, 1, m::STEAM),
        ] {
            blocks.push((c, r, z, mt));
        }
        // Draw back to front.
        blocks.sort_by_key(|&(c, r, z, _)| (r + c, z));
        let (s, ox, oy) = (46.0f32, 470.0f32, 170.0f32);
        let face =
            |img: &mut Vec<u8>, o: [f32; 2], e1: [f32; 2], e2: [f32; 2], t: u8, light: f32| {
                let steps = 64;
                for j in 0..steps {
                    for i in 0..steps {
                        let (fu, fv) = (
                            (i as f32 + 0.5) / steps as f32,
                            (j as f32 + 0.5) / steps as f32,
                        );
                        let p = a.px(t, (fu * 16.0) as u32, (fv * 16.0) as u32);
                        if p[3] == 0 {
                            continue;
                        }
                        let x = o[0] + e1[0] * fu + e2[0] * fv;
                        let y = o[1] + e1[1] * fu + e2[1] * fv;
                        for (dx, dy) in [(0.0, 0.0), (0.5, 0.0), (0.0, 0.5), (0.5, 0.5)] {
                            let (xi, yi) = ((x + dx) as i32, (y + dy) as i32);
                            if xi < 0 || yi < 0 || xi >= w as i32 || yi >= h as i32 {
                                continue;
                            }
                            let k = ((yi as u32 * w + xi as u32) * 4) as usize;
                            let al = p[3] as f32 / 255.0;
                            for ch in 0..3 {
                                let v = (p[ch] as f32 * light).min(255.0);
                                img[k + ch] = (img[k + ch] as f32 * (1.0 - al) + v * al) as u8;
                            }
                        }
                    }
                }
            };
        for (c, r, z, mt) in blocks {
            let (c, r, z) = (c as f32, r as f32, z as f32);
            let iso = |a: f32, b: f32, zz: f32| [ox + (a - b) * s, oy + (a + b) * s * 0.5 - zz * s];
            let (tt, ts) = (tile_for(mt, Face::Top), tile_for(mt, Face::Side));
            let sub = |p: [f32; 2], q: [f32; 2]| [p[0] - q[0], p[1] - q[1]];
            let o = iso(c, r, z + 1.0);
            face(
                &mut img,
                o,
                sub(iso(c + 1.0, r, z + 1.0), o),
                sub(iso(c, r + 1.0, z + 1.0), o),
                tt,
                1.08,
            );
            // The two faces toward the viewer: +r (left) and +c (right).
            let o = iso(c, r + 1.0, z + 1.0);
            face(
                &mut img,
                o,
                sub(iso(c + 1.0, r + 1.0, z + 1.0), o),
                [0.0, s],
                ts,
                0.8,
            );
            let o = iso(c + 1.0, r, z + 1.0);
            face(
                &mut img,
                o,
                sub(iso(c + 1.0, r + 1.0, z + 1.0), o),
                [0.0, s],
                ts,
                0.94,
            );
        }
        png(w, h, &img)
    }

    /// A minimal PNG encoder (stored deflate blocks), for the preview.
    fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
        fn crc(d: &[u8]) -> u32 {
            let mut c = 0xffff_ffffu32;
            for &b in d {
                c ^= b as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 {
                        0xedb8_8320 ^ (c >> 1)
                    } else {
                        c >> 1
                    };
                }
            }
            !c
        }
        let chunk = |out: &mut Vec<u8>, ty: &[u8], data: &[u8]| {
            out.extend((data.len() as u32).to_be_bytes());
            let mut body = ty.to_vec();
            body.extend_from_slice(data);
            out.extend_from_slice(&body);
            out.extend(crc(&body).to_be_bytes());
        };
        let mut raw = Vec::new();
        for y in 0..h as usize {
            raw.push(0);
            raw.extend_from_slice(&rgba[y * w as usize * 4..(y + 1) * w as usize * 4]);
        }
        let mut z = vec![0x78, 0x01];
        for (i, blk) in raw.chunks(65535).enumerate() {
            let last = (i + 1) * 65535 >= raw.len();
            z.push(last as u8);
            z.extend((blk.len() as u16).to_le_bytes());
            z.extend((!(blk.len() as u16)).to_le_bytes());
            z.extend_from_slice(blk);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &x in &raw {
            a = (a + x as u32) % 65521;
            b = (b + a) % 65521;
        }
        z.extend(((b << 16) | a).to_be_bytes());
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut ihdr = Vec::new();
        ihdr.extend(w.to_be_bytes());
        ihdr.extend(h.to_be_bytes());
        ihdr.extend([8, 6, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &z);
        chunk(&mut out, b"IEND", &[]);
        out
    }
}
