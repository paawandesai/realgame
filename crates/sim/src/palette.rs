//! How each material looks: colours from the Okabe-Ito palette (safe for the
//! common kinds of colour blindness) plus a per-material texture, so no two
//! materials that matter are told apart by hue alone. Presentation only:
//! nothing here feeds back into the simulation.
//!
//! Okabe, M. and Ito, K. (2008), "Color Universal Design (CUD): how to make
//! figures and presentations that are friendly to colorblind people".

use crate::species::Species;

/// The eight Okabe-Ito colours.
pub mod oi {
    pub const ORANGE: [u8; 3] = [230, 159, 0];
    pub const SKY_BLUE: [u8; 3] = [86, 180, 233];
    pub const BLUISH_GREEN: [u8; 3] = [0, 158, 115];
    pub const YELLOW: [u8; 3] = [240, 228, 66];
    pub const BLUE: [u8; 3] = [0, 114, 178];
    pub const VERMILLION: [u8; 3] = [213, 94, 0];
    pub const REDDISH_PURPLE: [u8; 3] = [204, 121, 167];
    pub const BLACK: [u8; 3] = [0, 0, 0];
}

/// A texture drawn over a material's base colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// Plain colour with a little grain.
    Grain,
    /// Moving bright ripple streaks and a light surface line (water).
    Ripple,
    /// Diagonal white facet lines (ice).
    Facet,
    /// Drifting white wisps (steam).
    Wisp,
    /// Sparse light dots (hydrogen).
    SparseDots,
    /// Paired vermillion dots on a diagonal (oxygen).
    PairDots,
    /// Dense dark dots (carbon dioxide).
    DenseDots,
    /// Short green dashes (methane).
    Dashes,
    /// Orange/yellow flicker (flame).
    Flicker,
    /// Slow sparkles (crystal).
    Sparkle,
    /// Wood grain stripes.
    Stripes,
}

/// A material's look: the colour of a cell with nothing drawn over it, how
/// much it lets the sky through (gases), the accent colour its texture uses,
/// and the texture itself.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub base: [u8; 3],
    /// 0 = opaque; gases are drawn as a tint over the sky.
    pub sky_through: f32,
    pub accent: [u8; 3],
    pub pattern: Pattern,
}

pub fn look(s: Species) -> Look {
    use oi::*;
    use Pattern::*;
    let l = |base, sky_through, accent, pattern| Look {
        base,
        sky_through,
        accent,
        pattern,
    };
    match s {
        Species::Air => l([0, 0, 0], 1.0, [0, 0, 0], Grain),
        Species::Stone => l([108, 102, 98], 0.0, [84, 80, 78], Grain),
        Species::Wood => l([128, 82, 40], 0.0, [92, 58, 28], Stripes),
        Species::Water => l(BLUE, 0.0, [86, 180, 233], Ripple),
        Species::Ice => l([206, 236, 250], 0.0, [255, 255, 255], Facet),
        Species::Steam => l([238, 240, 242], 0.35, [255, 255, 255], Wisp),
        Species::H2 => l([214, 238, 252], 0.6, [255, 255, 255], SparseDots),
        Species::O2 => l([246, 196, 160], 0.6, VERMILLION, PairDots),
        Species::CH4 => l([150, 220, 196], 0.55, BLUISH_GREEN, Dashes),
        Species::CO2 => l([96, 96, 100], 0.45, [40, 40, 44], DenseDots),
        Species::Cl2 => l(YELLOW, 0.5, YELLOW, Grain),
        Species::Flame => l(ORANGE, 0.0, YELLOW, Flicker),
        Species::Door => l([78, 70, 88], 0.0, [56, 50, 64], Grain),
        Species::Crystal => l(REDDISH_PURPLE, 0.0, [248, 226, 240], Sparkle),
    }
}

/// The sky: a dusk gradient, deep enough that glows read and light
/// materials (ice, steam) stand out against it.
pub fn sky(y: usize, height: usize) -> [u8; 3] {
    mix([52, 78, 128], [132, 164, 204], y as f32 / height as f32)
}

pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t) as u8)
}

pub fn hash(x: usize, y: usize) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// Neighbour facts the textures need.
#[derive(Clone, Copy, Default)]
pub struct Around {
    /// The cell above is a different material (a surface).
    pub surface: bool,
}

/// The colour of one cell. `frame` animates ripples, flicker and wisps.
pub fn cell_color(
    s: Species,
    x: usize,
    y: usize,
    frame: u32,
    sky_c: [u8; 3],
    around: Around,
) -> [u8; 3] {
    let lk = look(s);
    if s == Species::Air {
        return sky_c;
    }
    let base = if lk.sky_through > 0.0 {
        mix(lk.base, sky_c, lk.sky_through)
    } else {
        lk.base
    };
    let grain = (hash(x, y) % 17) as i32 - 8;
    let f = frame as usize;
    let c = match lk.pattern {
        Pattern::Grain => base,
        Pattern::Stripes => {
            if (y + (hash(x / 3, 0) % 3) as usize).is_multiple_of(4) {
                lk.accent
            } else {
                base
            }
        }
        Pattern::Ripple => {
            if around.surface {
                [150, 210, 245]
            } else if (x * 3 + y * 5 + f / 3) % 29 < 2 {
                mix(base, lk.accent, 0.6)
            } else {
                base
            }
        }
        Pattern::Facet => {
            if (x + y).is_multiple_of(7) {
                lk.accent
            } else if (x + 200 - y % 200).is_multiple_of(11) {
                [150, 204, 232]
            } else {
                base
            }
        }
        Pattern::Wisp => {
            if hash(x / 2, (y + f / 8) / 2).is_multiple_of(4) {
                lk.accent
            } else {
                base
            }
        }
        Pattern::SparseDots => {
            if x.is_multiple_of(4) && (y + x / 4).is_multiple_of(4) {
                lk.accent
            } else {
                base
            }
        }
        Pattern::PairDots => {
            if (x + y).is_multiple_of(5) && !y.is_multiple_of(3) {
                lk.accent
            } else {
                base
            }
        }
        Pattern::DenseDots => {
            if x.is_multiple_of(2) && y.is_multiple_of(2) {
                lk.accent
            } else {
                base
            }
        }
        Pattern::Dashes => {
            if y.is_multiple_of(3) && (x + y) % 6 < 3 {
                lk.accent
            } else {
                base
            }
        }
        Pattern::Flicker => match hash(x, y + f) % 7 {
            0 | 1 => lk.accent,
            2 => oi::VERMILLION,
            _ => base,
        },
        Pattern::Sparkle => {
            if (f / 8 + x).is_multiple_of(12) && hash(x, y).is_multiple_of(3) {
                lk.accent
            } else {
                base
            }
        }
    };
    if matches!(lk.pattern, Pattern::Flicker | Pattern::Sparkle) || lk.sky_through > 0.0 {
        c
    } else {
        c.map(|v| (v as i32 + grain).clamp(0, 255) as u8)
    }
}

/// Temperature colour map for the Insight lens, a thermal-camera ramp:
/// deep blue when freezing, blue at room temperature, cyan when warm,
/// yellow at water's boiling point, then orange, red and white-hot. The
/// stops sit on the temperatures the game is about (0 °C ice melts, 100 °C
/// water boils, 300-600 °C fuels ignite, 1000+ °C flames).
pub fn heat_color(t: f32) -> [u8; 3] {
    const STOPS: [(f32, [u8; 3]); 9] = [
        (-60.0, [20, 24, 120]),
        (-5.0, [30, 70, 215]),
        (20.0, [30, 100, 200]),
        (55.0, [40, 215, 225]),
        (100.0, [245, 225, 50]),
        (400.0, [250, 140, 30]),
        (800.0, [230, 40, 30]),
        (1500.0, [255, 150, 140]),
        (2000.0, [255, 255, 255]),
    ];
    if t <= STOPS[0].0 {
        return STOPS[0].1;
    }
    for w in STOPS.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            return mix(c0, c1, (t - t0) / (t1 - t0));
        }
    }
    STOPS[STOPS.len() - 1].1
}

/// Under the lens, solids keep their temperature's hue but darker, so
/// shapes stay readable without turning the world grey.
pub fn lens_solid(heat: [u8; 3]) -> [u8; 3] {
    mix(heat, [10, 12, 20], 0.62)
}

/// Temperature range the GPU renderer's one-byte temperatures cover, °C.
pub const TEMP_BYTE_MIN_C: f32 = -60.0;
pub const TEMP_BYTE_SPAN_C: f32 = 2860.0;

/// One byte per temperature for the WebGL renderer: square-root spaced over
/// -60..2800 °C, so it is finest near room temperature (about 4 °C a step)
/// and coarser in flames (about 22 °C a step). Display only; the simulation
/// keeps full f32 temperatures.
pub fn temp_byte(t: f32) -> u8 {
    let u = ((t - TEMP_BYTE_MIN_C) / TEMP_BYTE_SPAN_C).clamp(0.0, 1.0);
    (u.sqrt() * 255.0).round() as u8
}

/// The temperature a byte from `temp_byte` stands for (the shader does the same).
pub fn temp_from_byte(b: u8) -> f32 {
    let u = b as f32 / 255.0;
    u * u * TEMP_BYTE_SPAN_C + TEMP_BYTE_MIN_C
}

/// `heat_color` sampled at every temperature byte, as RGBA.
pub fn heat_lut() -> Vec<u8> {
    (0..=255u8)
        .flat_map(|b| {
            let c = heat_color(temp_from_byte(b));
            [c[0], c[1], c[2], 255]
        })
        .collect()
}

/// What makes a cell glow (for the additive glow sprites in the browser).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Glow {
    None = 0,
    /// Anything above 400 °C that isn't a flame (embers, hot stone).
    Hot = 1,
    Crystal = 2,
    Flame = 3,
}

pub fn glow_of(s: Species, temp_c: f32) -> Glow {
    match s {
        Species::Flame => Glow::Flame,
        Species::Crystal => Glow::Crystal,
        _ if temp_c > 400.0 => Glow::Hot,
        _ => Glow::None,
    }
}

/// The emissive cells of a frame, one entry per block of `BLOCK` x `BLOCK`
/// cells, capped so the browser draws a bounded number of glow sprites.
pub const GLOW_BLOCK: usize = 4;
pub const GLOW_CAP: usize = 320;

#[derive(Default)]
pub struct GlowMap {
    cols: usize,
    rows: usize,
    /// Strongest glow per block and how many glowing cells it holds.
    kind: Vec<u8>,
    count: Vec<u8>,
}

impl GlowMap {
    pub fn reset(&mut self, w: usize, h: usize) {
        self.cols = w.div_ceil(GLOW_BLOCK);
        self.rows = h.div_ceil(GLOW_BLOCK);
        let n = self.cols * self.rows;
        self.kind.clear();
        self.kind.resize(n, 0);
        self.count.clear();
        self.count.resize(n, 0);
    }

    #[inline]
    pub fn add(&mut self, sx: usize, y: usize, g: Glow) {
        if g == Glow::None {
            return;
        }
        let i = (y / GLOW_BLOCK) * self.cols + sx / GLOW_BLOCK;
        self.kind[i] = self.kind[i].max(g as u8);
        self.count[i] = self.count[i].saturating_add(1);
    }

    /// Flat `[x, y, kind, strength]` per glowing block (viewport cells; x and
    /// y are the block centre; strength 0..1 is how much of the block glows).
    /// Over the cap, blocks are thinned evenly, so the result is bounded.
    pub fn entries(&self) -> Vec<f32> {
        let lit: Vec<usize> = (0..self.kind.len()).filter(|&i| self.kind[i] > 0).collect();
        let stride = lit.len().div_ceil(GLOW_CAP).max(1);
        let area = (GLOW_BLOCK * GLOW_BLOCK) as f32;
        let mut out = Vec::with_capacity(lit.len().min(GLOW_CAP) * 4);
        for &i in lit.iter().step_by(stride) {
            let (bx, by) = (i % self.cols, i / self.cols);
            out.push((bx * GLOW_BLOCK) as f32 + GLOW_BLOCK as f32 / 2.0);
            out.push((by * GLOW_BLOCK) as f32 + GLOW_BLOCK as f32 / 2.0);
            out.push(self.kind[i] as f32);
            out.push((self.count[i] as f32 / area).min(1.0));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn temperature_bytes_are_monotonic_and_close() {
        let mut last = 0u8;
        for t in -60..=2800 {
            let b = temp_byte(t as f32);
            assert!(b >= last, "not monotonic at {t}");
            last = b;
            let back = temp_from_byte(b);
            // Finest where the game lives (room temperature), coarse in flames.
            let tol = if t < 120 { 4.0 } else { 25.0 };
            assert!((back - t as f32).abs() <= tol, "{t} -> {b} -> {back}");
        }
        assert_eq!(temp_byte(-500.0), 0);
        assert_eq!(temp_byte(9000.0), 255);
        assert_eq!(heat_lut().len(), 256 * 4);
    }

    use super::*;

    #[test]
    fn the_heat_lens_is_a_thermal_ramp() {
        let hue = |c: [u8; 3]| {
            let [r, g, b] = c.map(|v| v as f32);
            let (max, min) = (r.max(g).max(b), r.min(g).min(b));
            if max - min < 1.0 {
                return 0.0;
            }
            let h = if max == r {
                (g - b) / (max - min)
            } else if max == g {
                2.0 + (b - r) / (max - min)
            } else {
                4.0 + (r - g) / (max - min)
            };
            (h * 60.0).rem_euclid(360.0)
        };
        // Room-temperature air reads blue-cyan, never grey.
        let air = heat_color(20.0);
        assert!((180.0..=240.0).contains(&hue(air)), "{air:?}");
        assert!(air[2] as i32 - air[0] as i32 > 120);
        // Solids keep the hue but are clearly darker than the air around them.
        let ground = lens_solid(air);
        assert!(ground
            .iter()
            .zip(air)
            .all(|(g, a)| (*g as i32) < a as i32 / 2 + 8));
        // Boiling water is yellow, a wood fire red, a flame white-hot.
        assert!((45.0..=70.0).contains(&hue(heat_color(100.0))));
        let fire = heat_color(1100.0);
        assert!(hue(fire) < 20.0 || hue(fire) > 340.0, "{fire:?}");
        assert!(heat_color(2000.0).iter().all(|&v| v > 240));
        // Hotter is never darker past room temperature.
        let lum = |c: [u8; 3]| c[0] as u32 * 3 + c[1] as u32 * 6 + c[2] as u32;
        assert!(lum(heat_color(1500.0)) > lum(heat_color(800.0)));
        assert!(lum(heat_color(2000.0)) > lum(heat_color(1500.0)));
    }

    fn lin(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// WCAG relative luminance of a linear-light RGB triple.
    fn lum_lin(r: f32, g: f32, b: f32) -> f32 {
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Machado, Oliveira and Fernandes (2009) full-severity simulation
    /// matrices, applied in linear RGB.
    const CVD: [(&str, [[f32; 3]; 3]); 4] = [
        (
            "normal",
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        ),
        (
            "protanopia",
            [
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ],
        ),
        (
            "deuteranopia",
            [
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ],
        ),
        (
            "tritanopia",
            [
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ],
        ),
    ];

    fn seen_luminance(c: [u8; 3], m: &[[f32; 3]; 3]) -> f32 {
        let v = [lin(c[0]), lin(c[1]), lin(c[2])];
        let r = (m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2]).clamp(0.0, 1.0);
        let g = (m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2]).clamp(0.0, 1.0);
        let b = (m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2]).clamp(0.0, 1.0);
        lum_lin(r, g, b)
    }

    /// The colour a material shows with nothing drawn over it, at mid-sky.
    fn shown(s: Species) -> [u8; 3] {
        let lk = look(s);
        mix(lk.base, sky(90, 180), lk.sky_through)
    }

    #[test]
    fn the_water_family_and_the_gases_differ_in_luminance_or_pattern_for_every_vision() {
        let set = [
            Species::Water,
            Species::Steam,
            Species::Ice,
            Species::H2,
            Species::O2,
            Species::CO2,
        ];
        for (name, m) in &CVD {
            for (i, &a) in set.iter().enumerate() {
                for &b in &set[i + 1..] {
                    let dl = (seen_luminance(shown(a), m) - seen_luminance(shown(b), m)).abs();
                    let distinct_pattern = look(a).pattern != look(b).pattern;
                    assert!(
                        dl >= 0.08 || distinct_pattern,
                        "{name}: {a:?} and {b:?} look alike (ΔL {dl:.3}, same pattern)"
                    );
                }
            }
        }
        // And every one of the six has its own texture, whatever the colours.
        for (i, &a) in set.iter().enumerate() {
            for &b in &set[i + 1..] {
                assert_ne!(look(a).pattern, look(b).pattern, "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn each_gas_texture_shows_its_accent_somewhere_and_differs_from_the_sky() {
        for s in [
            Species::Steam,
            Species::H2,
            Species::O2,
            Species::CO2,
            Species::CH4,
        ] {
            let sky_c = sky(90, 180);
            let mut accents = 0;
            let mut differs = 0;
            for y in 80..100 {
                for x in 0..40 {
                    let c = cell_color(s, x, y, 0, sky_c, Around::default());
                    if c == look(s).accent {
                        accents += 1;
                    }
                    if c != sky_c {
                        differs += 1;
                    }
                }
            }
            assert!(accents >= 20, "{s:?}: only {accents} accent cells in 800");
            assert_eq!(differs, 800, "{s:?} vanishes into the sky");
        }
    }

    #[test]
    fn glow_entries_are_capped_and_mark_flames_crystals_and_hot_cells() {
        assert_eq!(glow_of(Species::Flame, 20.0), Glow::Flame);
        assert_eq!(glow_of(Species::Crystal, 20.0), Glow::Crystal);
        assert_eq!(glow_of(Species::Stone, 401.0), Glow::Hot);
        assert_eq!(glow_of(Species::Stone, 399.0), Glow::None);
        let mut g = GlowMap::default();
        g.reset(320, 180);
        g.add(10, 10, Glow::Hot);
        g.add(11, 10, Glow::Flame);
        let e = g.entries();
        assert_eq!(e, vec![10.0, 10.0, 3.0, 2.0 / 16.0]);
        // A screen full of fire still gives a bounded list.
        g.reset(320, 180);
        for y in 0..180 {
            for x in 0..320 {
                g.add(x, y, Glow::Flame);
            }
        }
        let e = g.entries();
        assert!(e.len() / 4 <= GLOW_CAP, "{} entries", e.len() / 4);
        assert!(e.len() / 4 >= GLOW_CAP / 2);
    }
}
