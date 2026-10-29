//! The browser-facing API, exported to JavaScript with wasm-bindgen, and the
//! renderer that turns the grid into pixels.

use crate::game::{Game, Tier, HEIGHT, WIDTH};
use crate::species::Species;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebGame {
    game: Game,
    pixels: Vec<u8>,
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new(tier: &str, seed: u32) -> WebGame {
        WebGame {
            game: Game::new(Tier::parse(tier), seed as u64 + 1),
            pixels: vec![0; WIDTH * HEIGHT * 4],
        }
    }

    pub fn width(&self) -> usize {
        WIDTH
    }

    pub fn height(&self) -> usize {
        HEIGHT
    }

    pub fn tick(&mut self, input: u32) {
        self.game.tick(input);
    }

    pub fn set_aim(&mut self, x: i32, y: i32) {
        self.game.aim = (x.clamp(0, WIDTH as i32 - 1), y.clamp(0, HEIGHT as i32 - 1));
    }

    /// Returns a JSON `CastResult`.
    pub fn cast(&mut self, formula: &str) -> String {
        serde_json::to_string(&self.game.cast(formula)).unwrap_or_default()
    }

    /// Returns a JSON `State` for the HUD. Drains pending mentor messages.
    pub fn state(&mut self) -> String {
        serde_json::to_string(&self.game.state()).unwrap_or_default()
    }

    /// Draws the frame and returns a pointer to WIDTH * HEIGHT RGBA bytes in
    /// WebAssembly memory. `lens` switches to the temperature view; `alpha`
    /// (0..=1) is how far the display is between the last two ticks.
    pub fn render(&mut self, lens: bool, alpha: f32) -> *const u8 {
        render(&self.game, &mut self.pixels, lens, alpha.clamp(0.0, 1.0));
        self.pixels.as_ptr()
    }
}

fn hash(x: usize, y: usize) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t) as u8)
}

fn sky(y: usize) -> [u8; 3] {
    mix([148, 196, 238], [214, 232, 246], y as f32 / HEIGHT as f32)
}

/// Temperature colour map for the Insight lens: blue cold, dark at room
/// temperature, then red, orange, yellow and white as it heats.
fn heat_color(t: f32) -> [u8; 3] {
    let stops: [(f32, [u8; 3]); 7] = [
        (-60.0, [60, 120, 255]),
        (0.0, [70, 170, 230]),
        (20.0, [24, 28, 40]),
        (100.0, [120, 40, 140]),
        (500.0, [220, 40, 30]),
        (1500.0, [255, 170, 30]),
        (2800.0, [255, 255, 230]),
    ];
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            return mix(c0, c1, (t - t0) / (t1 - t0));
        }
    }
    stops[6].1
}

fn render(game: &Game, out: &mut [u8], lens: bool, alpha: f32) {
    let g = &game.grid;
    for y in 0..g.h {
        for x in 0..g.w {
            let i = g.idx(x, y);
            let s = g.cells[i];
            let t = g.temp[i];
            let n = (hash(x, y) % 21) as i32 - 10;
            let mut c = if lens {
                let base = heat_color(t);
                if s.is_solid() {
                    mix(base, [90, 90, 100], 0.35)
                } else {
                    base
                }
            } else {
                let p = s.props();
                let base = match s {
                    Species::Air => sky(y),
                    Species::Flame => {
                        let flicker = (hash(x, y + g.frame as usize) % 80) as u8;
                        [255, 120 + flicker, 30]
                    }
                    // Most real gases are colourless; show a faint tint so players
                    // can follow them, and steam (tiny droplets) clearly.
                    Species::Steam => mix(sky(y), p.color, 0.7),
                    _ if s.is_gas() => mix(sky(y), p.color, 0.4),
                    _ => p.color,
                };
                let shaded = if s == Species::Air {
                    base
                } else {
                    base.map(|v| (v as i32 + n).clamp(0, 255) as u8)
                };
                if t > 400.0 && s != Species::Flame {
                    mix(shaded, [255, 110, 30], (t - 400.0) / 1400.0)
                } else {
                    shaded
                }
            };
            if !lens && s == Species::Crystal && (g.frame / 8 + x as u32).is_multiple_of(12) {
                c = [230, 200, 255];
            }
            out[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    draw_player(game, out, alpha);
    draw_aim(game, out);
}

fn put(out: &mut [u8], x: i32, y: i32, c: [u8; 3]) {
    if x >= 0 && y >= 0 && (x as usize) < WIDTH && (y as usize) < HEIGHT {
        let i = (y as usize * WIDTH + x as usize) * 4;
        out[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    }
}

fn draw_player(game: &Game, out: &mut [u8], alpha: f32) {
    let p = &game.player;
    let lerp = |a: f32, b: f32| a + (b - a) * alpha;
    let (px, py) = (
        lerp(p.prev_x, p.x).round() as i32,
        lerp(p.prev_y, p.y).round() as i32,
    );
    let skin = [240, 196, 160];
    let suit = [30, 150, 140];
    let trim = [250, 210, 80];
    let dark = [30, 40, 50];
    // A small robed cultivator: head, suit with a glowing core, legs.
    for dx in 0..3 {
        put(out, px + dx, py, dark);
        put(out, px + dx, py + 1, skin);
        for dy in 2..6 {
            put(out, px + dx, py + dy, suit);
        }
    }
    put(out, px + 1, py + 3, trim);
    let eye = if game.player.facing > 0 { px + 2 } else { px };
    put(out, eye, py + 1, dark);
    put(out, px, py + 6, dark);
    put(out, px, py + 7, dark);
    put(out, px + 2, py + 6, dark);
    put(out, px + 2, py + 7, dark);
}

fn draw_aim(game: &Game, out: &mut [u8]) {
    let (ax, ay) = game.aim;
    let c = [255, 255, 255];
    for d in [2, 3] {
        put(out, ax + d, ay, c);
        put(out, ax - d, ay, c);
        put(out, ax, ay + d, c);
        put(out, ax, ay - d, c);
    }
}
