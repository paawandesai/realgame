//! The browser-facing API, exported to JavaScript with wasm-bindgen, and the
//! renderer that turns the grid into pixels.

use crate::game::{Game, Tier, HEIGHT, VIEW_W, WORLD_W};
use crate::palette::{self, glow_of, hash, mix, Around, GlowMap};
use crate::species::Species;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebGame {
    game: Game,
    pixels: Vec<u8>,
    /// Glowing blocks of the last frame, for the browser's glow sprites.
    glow: GlowMap,
    /// The camera position the last frame was drawn at, so pointer input maps
    /// to exactly the cells the player sees.
    drawn_view_x: i32,
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new(tier: &str, seed: u32) -> WebGame {
        WebGame {
            game: Game::new(Tier::parse(tier), seed as u64 + 1),
            pixels: vec![0; VIEW_W * HEIGHT * 4],
            glow: GlowMap::default(),
            drawn_view_x: 0,
        }
    }

    /// Width of the viewport (and of the rendered frame) in cells.
    pub fn width(&self) -> usize {
        VIEW_W
    }

    /// Left edge of the last rendered frame, in world cells.
    pub fn view_x(&self) -> i32 {
        self.drawn_view_x
    }

    /// The player's left edge, in world cells (for the HUD and browser tests).
    pub fn player_x(&self) -> f32 {
        self.game.player.x
    }

    pub fn height(&self) -> usize {
        HEIGHT
    }

    pub fn tick(&mut self, input: u32) {
        self.game.tick(input);
    }

    /// The player pointed somewhere on purpose.
    pub fn set_aim(&mut self, x: i32, y: i32) {
        self.game.set_aim(x, y, true);
    }

    /// Keeps the aim under a resting pointer while the camera scrolls; does
    /// not count as aiming on purpose (so soft auto-aim can still help).
    pub fn track_aim(&mut self, x: i32, y: i32) {
        self.game.set_aim(x, y, false);
    }

    /// Returns a JSON `CastOutcome`. Every input gets one; there is no error path.
    pub fn cast(&mut self, formula: &str) -> String {
        serde_json::to_string(&self.game.cast(formula)).unwrap_or_default()
    }

    /// Returns the presentation events (JSON array of `Fx`) since the last call.
    pub fn take_fx(&mut self) -> String {
        serde_json::to_string(&self.game.take_fx()).unwrap_or_default()
    }

    /// Bumps whenever anything the HUD shows changes, so `state()` (which
    /// builds JSON) only needs calling when this moves.
    pub fn state_version(&self) -> f64 {
        self.game.state_version() as f64
    }

    /// Returns a JSON array of `Goal`s (icon positions and fire meters). Cheap;
    /// the overlay polls it a few times a second.
    pub fn goals(&self) -> String {
        serde_json::to_string(&self.game.goals()).unwrap_or_default()
    }

    /// Returns a JSON `State` for the HUD. Drains pending mentor messages.
    pub fn state(&mut self) -> String {
        serde_json::to_string(&self.game.state()).unwrap_or_default()
    }

    /// Starts the room the player is in again (earlier rooms stay solved).
    /// Returns the room, 0-based.
    pub fn restart_room(&mut self) -> u32 {
        self.game.restart_room() as u32
    }

    /// Starts the whole trial again.
    pub fn restart(&mut self) {
        self.game.restart();
    }

    /// Opens every gate and fills the suit (after the trial).
    pub fn free_play(&mut self) {
        self.game.free_play();
    }

    /// The room the player is in, 0-based.
    pub fn room(&self) -> u32 {
        self.game.room() as u32
    }

    /// Draws the viewport and returns a pointer to VIEW_W * HEIGHT RGBA bytes in
    /// WebAssembly memory. `lens` switches to the temperature view; `alpha`
    /// (0..=1) is how far the display is between the last two ticks.
    pub fn render(&mut self, lens: bool, alpha: f32) -> *const u8 {
        let alpha = alpha.clamp(0.0, 1.0);
        let g = &self.game;
        let vx = (g.prev_view_x + (g.view_x - g.prev_view_x) * alpha).round() as i32;
        self.drawn_view_x = vx.clamp(0, (WORLD_W - VIEW_W) as i32);
        render(
            g,
            &mut self.pixels,
            &mut self.glow,
            lens,
            alpha,
            self.drawn_view_x,
        );
        self.pixels.as_ptr()
    }

    /// The glowing parts of the last rendered frame (flames, crystals, cells
    /// above 400 °C) as flat `[x, y, kind, strength]` groups in viewport
    /// cells; kind is 1 hot, 2 crystal, 3 flame. At most `GLOW_CAP` groups.
    pub fn emissive(&self) -> Vec<f32> {
        self.glow.entries()
    }
}

fn sky(y: usize) -> [u8; 3] {
    palette::sky(y, HEIGHT)
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

fn render(game: &Game, out: &mut [u8], glow: &mut GlowMap, lens: bool, alpha: f32, view_x: i32) {
    let g = &game.grid;
    glow.reset(VIEW_W, g.h);
    for y in 0..g.h {
        let sky_c = sky(y);
        for sx in 0..VIEW_W {
            let x = sx + view_x as usize;
            let i = g.idx(x, y);
            let o = (y * VIEW_W + sx) * 4;
            let s = g.cells[i];
            let t = g.temp[i];
            if t > 400.0 || matches!(s, Species::Flame | Species::Crystal) {
                glow.add(sx, y, glow_of(s, t));
            }
            let c = if lens {
                let base = heat_color(t);
                if s.is_solid() {
                    mix(base, [90, 90, 100], 0.35)
                } else {
                    base
                }
            } else {
                let c = if s == Species::Air {
                    sky_c
                } else {
                    // Only water draws its surface, so only water looks up.
                    let around = Around {
                        surface: s == Species::Water && y > 0 && g.cells[i - g.w] != s,
                    };
                    palette::cell_color(s, x, y, g.frame, sky_c, around)
                };
                if t > 400.0 && s != Species::Flame {
                    mix(c, [255, 110, 30], (t - 400.0) / 1400.0)
                } else {
                    c
                }
            };
            out[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    draw_torch(game, out, view_x);
    draw_player(game, out, alpha, view_x);
    draw_aim(game, out, view_x);
}

/// The room-2 torch: a wooden post with a flickering flame when lit. Drawn
/// in the player's hand when carried.
fn draw_torch(game: &Game, out: &mut [u8], view_x: i32) {
    let t = game.torch_view();
    let (hx, hy) = game.torch_head();
    let (x, top) = (hx - view_x, hy);
    let len = if t.carried { 7 } else { 12 };
    for dy in 1..=len {
        put(out, x, top + dy, [122, 84, 48]);
    }
    put(out, x, top, [60, 40, 24]);
    if t.lit {
        let f = game.grid.frame as usize;
        for (dx, dy, c) in [
            (0, -1, [255, 220, 90]),
            (0, -2, [255, 170, 40]),
            (-1, -1, [255, 140, 30]),
            (1, -1, [255, 140, 30]),
            (0, -3, [255, 120, 30]),
        ] {
            if dy == -3 && hash(hx as usize, f / 3).is_multiple_of(2) {
                continue;
            }
            put(out, x + dx, top + dy, c);
        }
    }
}

fn put(out: &mut [u8], x: i32, y: i32, c: [u8; 3]) {
    if x >= 0 && y >= 0 && (x as usize) < VIEW_W && (y as usize) < HEIGHT {
        let i = (y as usize * VIEW_W + x as usize) * 4;
        out[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    }
}

/// The cultivator, 6 x 12 cells, facing right. Rows 0-8 are the body; the
/// legs (rows 9-11) change with the pose.
const BODY: [&[u8; 6]; 9] = [
    b".HHHH.", b"HHHHHH", b"HSSSES", b".SSSS.", b"TCCCCT", b"CCCCCC", b"CCGGCC", b"CCCCCC",
    b"TTTTTT",
];
const LEGS_STAND: [&[u8; 6]; 3] = [b".L..L.", b".L..L.", b".F..F."];
const LEGS_STRIDE: [&[u8; 6]; 3] = [b".L..L.", b"L....L", b"F....F"];
const LEGS_PASS: [&[u8; 6]; 3] = [b"..LL..", b"..LL..", b"..FF.."];
const LEGS_JUMP: [&[u8; 6]; 3] = [b".LLLL.", b"L....L", b"......"];

fn sprite_color(c: u8) -> Option<[u8; 3]> {
    Some(match c {
        b'H' => [44, 40, 64],
        b'S' => [240, 196, 160],
        b'E' => [24, 28, 40],
        b'T' => [250, 210, 80],
        b'C' => [30, 150, 140],
        b'G' => [150, 255, 236],
        b'L' => [40, 52, 74],
        b'F' => [22, 26, 34],
        _ => return None,
    })
}

fn draw_player(game: &Game, out: &mut [u8], alpha: f32, view_x: i32) {
    let p = &game.player;
    let lerp = |a: f32, b: f32| a + (b - a) * alpha;
    let (px, py) = (
        lerp(p.prev_x, p.x).round() as i32 - view_x,
        lerp(p.prev_y, p.y).round() as i32,
    );
    let legs = if !p.grounded {
        LEGS_JUMP
    } else if p.vx.abs() > 0.3 {
        // Two-frame walk cycle, one frame per 5 cells walked.
        if ((p.stride / 5.0) as u32).is_multiple_of(2) {
            LEGS_STRIDE
        } else {
            LEGS_PASS
        }
    } else {
        LEGS_STAND
    };
    for (row, line) in BODY.iter().chain(legs.iter()).enumerate() {
        for (col, &c) in line.iter().enumerate() {
            if let Some(color) = sprite_color(c) {
                let dx = if p.facing >= 0 { col } else { 5 - col } as i32;
                put(out, px + dx, py + row as i32, color);
            }
        }
    }
}

fn draw_aim(game: &Game, out: &mut [u8], view_x: i32) {
    let (ax, ay) = (game.aim.0 - view_x, game.aim.1);
    let c = [255, 255, 255];
    for d in [2, 3] {
        put(out, ax + d, ay, c);
        put(out, ax - d, ay, c);
        put(out, ax, ay + d, c);
        put(out, ax, ay - d, c);
    }
}

/// The curated reaction table (`content/reactions.json`) as shipped, so the
/// Grimoire and hover cards show exactly the sourced values the sim uses.
#[wasm_bindgen]
pub fn reaction_book() -> String {
    crate::reactions::REACTIONS_JSON.to_string()
}
