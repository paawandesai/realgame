//! The browser-facing API, exported to JavaScript with wasm-bindgen, and the
//! renderer that turns the grid into pixels.

use crate::game::levels::{self, LevelSpec};
use crate::game::{Game, Tier, DOOR_TOP, GATE_W, GATE_X, HEIGHT, VIEW_W, WORLD_W};
use crate::palette::{self, glow_of, hash, heat_color, mix, temp_byte, Around, GlowMap};
use crate::species::Species;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebGame {
    game: Game,
    pixels: Vec<u8>,
    /// Per viewport pixel, for the WebGL renderer: a temperature byte (see
    /// `palette::temp_byte`) then the species id, or `SPRITE` where the
    /// player, torch or crosshair is drawn. Filled only while enabled.
    cell_data: Vec<u8>,
    cell_data_on: bool,
    /// Glowing blocks of the last frame, for the browser's glow sprites.
    glow: GlowMap,
    /// The camera position the last frame was drawn at, so pointer input maps
    /// to exactly the cells the player sees.
    drawn_view_x: i32,
    /// Whether the frame includes the white crosshair (Sprout shows a single
    /// bullseye in the page instead).
    crosshair: bool,
    /// v3 Lab/Predict controls: `tick` does nothing while paused, and runs
    /// `speed` ticks per call on average (0.25, 1 or 2; 1 is exactly one).
    paused: bool,
    speed: f32,
    speed_acc: f32,
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new(tier: &str, seed: u32) -> WebGame {
        WebGame {
            game: Game::new(Tier::parse(tier), seed as u64 + 1),
            pixels: vec![0; VIEW_W * HEIGHT * 4],
            cell_data: vec![0; VIEW_W * HEIGHT * 2],
            cell_data_on: false,
            glow: GlowMap::default(),
            drawn_view_x: 0,
            crosshair: true,
            paused: false,
            speed: 1.0,
            speed_acc: 0.0,
        }
    }

    // ------------------------------------------------------------ v3 levels
    // DESIGN_V3 §12.4. These swap the game inside this WebGame, so the page
    // keeps one object (and its render buffers) across story, Trials and Lab.

    /// Starts Trial `id` ("T1".."T7") at `tier`. False (and nothing changes)
    /// if the id is unknown or the tier does not see that Trial.
    pub fn new_trial(&mut self, tier: &str, id: &str, seed: u32) -> bool {
        let tier = Tier::parse(tier);
        match LevelSpec::trial(id).filter(|s| s.for_tier(tier)) {
            Some(spec) => {
                self.game = Game::new_level(tier, spec, seed as u64 + 1);
                self.after_swap();
                true
            }
            None => false,
        }
    }

    /// Starts the Lab at `tier`.
    pub fn new_lab(&mut self, tier: &str, seed: u32) {
        self.game = Game::new_level(Tier::parse(tier), LevelSpec::lab(), seed as u64 + 1);
        self.after_swap();
    }

    /// Back to a fresh v2 story (the same as constructing a new WebGame).
    pub fn new_story(&mut self, tier: &str, seed: u32) {
        self.game = Game::new(Tier::parse(tier), seed as u64 + 1);
        self.after_swap();
    }

    /// JSON array of the `LevelSpec`s a tier sees, in menu order:
    /// `[{"id":"T1","kind":"trial","title":"Foam Bridge","goal":"...",
    /// "tiers":["sprout",...],"par_casts":0}, ...]`.
    pub fn trial_list(&self, tier: &str) -> String {
        serde_json::to_string(&levels::trial_list(Tier::parse(tier))).unwrap_or_default()
    }

    /// JSON `TrialResult` for the current Trial (`{"id","solved","casts",
    /// "hints_used","balanced_all","atoms_wasted","kj_net","sig","stars"}`),
    /// or `null` in the story.
    pub fn trial_result(&self) -> String {
        serde_json::to_string(&self.game.trial_result()).unwrap_or_default()
    }

    /// Lab: paints species id `species` (`Species as u8`) in a disc of
    /// radius `r` at world cell (x, y). Ignored in the story.
    pub fn paint(&mut self, x: i32, y: i32, species: u8, r: i32) {
        self.game.paint(x, y, Species::from_u8(species), r);
    }

    /// Lab: erases (paints air) in a disc of radius `r`.
    pub fn erase(&mut self, x: i32, y: i32, r: i32) {
        self.game.erase(x, y, r);
    }

    /// Pauses or resumes the sim (Predict cards, Lab, Demo freeze).
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Runs exactly one tick with no input, even while paused (Lab "step").
    pub fn step_once(&mut self) {
        self.game.tick(0);
    }

    /// Sim speed multiplier for `tick`, clamped to 0..=4 (Lab and Demo use
    /// 0.25, 1 and 2). Independent of the page's own speed preference.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = if speed.is_finite() {
            speed.clamp(0.0, 4.0)
        } else {
            1.0
        };
    }

    /// True in the v2 story (and free play), false in a Trial or the Lab.
    pub fn is_story(&self) -> bool {
        self.game.is_story()
    }

    /// The current level's id ("T1".."T7" or "lab"), or "" in the story.
    pub fn level_id(&self) -> String {
        self.game
            .level
            .as_ref()
            .map_or(String::new(), |l| l.spec.id.to_string())
    }

    fn after_swap(&mut self) {
        self.speed_acc = 0.0;
        self.drawn_view_x = self.game.view_x.round() as i32;
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

    /// The player sprite for the browser to draw above the glow layers:
    /// `[x, y, dx, dy, rgb, dx, dy, rgb, ...]`, with `x`/`y` the top-left
    /// world cell for this frame and `rgb` packed as `0xRRGGBB`. Drawing
    /// only; the collision box is unchanged.
    pub fn player_sprite(&self, alpha: f32) -> Vec<i32> {
        let (px, py, cells) = player_cells(&self.game, alpha);
        let mut v = Vec::with_capacity(2 + cells.len() * 3);
        v.extend([px, py]);
        for (dx, dy, c) in cells {
            v.extend([dx, dy, i32::from_be_bytes([0, c[0], c[1], c[2]])]);
        }
        v
    }

    /// Whether the player stands on something or swims, so a jump would
    /// lift them now (the blocked-walk jump cue only shows then).
    pub fn player_footed(&self) -> bool {
        self.game.player_footed()
    }

    /// The player's top edge, in world cells (camera lift and browser tests).
    pub fn player_y(&self) -> f32 {
        self.game.player.y
    }

    /// Where the gauntlet points, in world cells (for browser tests).
    pub fn aim(&self) -> Vec<i32> {
        vec![self.game.aim.0, self.game.aim.1]
    }

    pub fn height(&self) -> usize {
        HEIGHT
    }

    pub fn tick(&mut self, input: u32) {
        if self.paused {
            return;
        }
        self.speed_acc += self.speed;
        while self.speed_acc >= 1.0 {
            self.speed_acc -= 1.0;
            self.game.tick(input);
        }
    }

    /// The player pointed somewhere on purpose.
    pub fn set_aim(&mut self, x: i32, y: i32) {
        self.game.set_aim(x, y, true);
    }

    /// Draws the crosshair into the frame (on by default). Sprout turns it
    /// off and shows one bullseye at the same point instead.
    pub fn set_crosshair(&mut self, on: bool) {
        self.crosshair = on;
    }

    /// A jump key or button went down: latched until the next tick uses it.
    pub fn press_jump(&mut self) {
        self.game.press_jump();
    }

    /// Where the reticle is (the goal a spell will fly at before any aiming).
    pub fn shown_aim(&self) -> Vec<i32> {
        let (x, y) = self.game.shown_aim();
        vec![x, y]
    }

    /// The goal target under the reticle ("hedge", "ice", "gas",
    /// "brazier"), or an empty string when it is on none.
    pub fn aim_target(&self) -> String {
        self.game.aim_target().unwrap_or("").to_string()
    }

    /// Keyboard aiming (T): points at the room's goal. False if none in range.
    pub fn aim_at_goal(&mut self) -> bool {
        self.game.aim_at_goal()
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
        if !self.game.is_story() {
            // Trial goals arrive with Track B's level layouts.
            return "[]".to_string();
        }
        serde_json::to_string(&self.game.goals()).unwrap_or_default()
    }

    /// Returns a JSON `State` for the HUD. Drains pending mentor messages.
    pub fn state(&mut self) -> String {
        serde_json::to_string(&self.game.state()).unwrap_or_default()
    }

    /// Starts the room the player is in again (earlier rooms stay solved).
    /// Returns the room, 0-based.
    pub fn restart_room(&mut self) -> u32 {
        if !self.game.is_story() {
            self.restart();
            return 0;
        }
        self.game.restart_room() as u32
    }

    /// Starts the whole trial again.
    pub fn restart(&mut self) {
        if let Some(l) = &self.game.level {
            let (spec, seed) = (l.spec.clone(), l.seed);
            self.game = Game::new_level(self.game.tier, spec, seed);
            self.after_swap();
            return;
        }
        self.game.restart();
    }

    /// Puts the player at world column `x`, standing above the ground there
    /// (browser tests start in a later room with it; the page only exposes
    /// it under `?debug`).
    pub fn place_player(&mut self, x: f32) {
        let p = &mut self.game.player;
        p.x = x;
        p.y = crate::game::GROUND_Y as f32 - crate::game::PLAYER_H - 4.0;
        p.prev_x = p.x;
        p.prev_y = p.y;
        p.vx = 0.0;
        p.vy = 0.0;
        self.game.snap_camera();
    }

    /// Opens every gate and fills the suit (after the trial).
    pub fn free_play(&mut self) {
        self.game.free_play();
    }

    /// Free play: rebuilds room `k` (0-based) from the seed, with its target
    /// (hedge, ice, gas trap, brazier) back. Returns the room.
    pub fn reset_room(&mut self, k: u32) -> u32 {
        self.game.reset_room(k as usize) as u32
    }

    /// Free play's challenges as a JSON array, and the Grimoire pages they
    /// have unlocked: `{"challenges": [...], "unlocked": [...]}`.
    pub fn free_play_state(&self) -> String {
        serde_json::json!({
            "challenges": self.game.challenges(),
            "unlocked": self.game.unlocked_pages(),
        })
        .to_string()
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
        let mut frame = Frame {
            rgba: &mut self.pixels,
            data: if self.cell_data_on {
                Some(&mut self.cell_data)
            } else {
                None
            },
        };
        render(
            g,
            &mut frame,
            &mut self.glow,
            lens,
            alpha,
            self.drawn_view_x,
            self.crosshair,
        );
        self.pixels.as_ptr()
    }

    /// Turns the per-pixel temperature and species buffer on or off (the
    /// WebGL renderer reads it; Canvas2D does not need it).
    pub fn set_cell_data(&mut self, on: bool) {
        self.cell_data_on = on;
    }

    /// Pointer to VIEW_W * HEIGHT byte pairs (temperature byte, species id)
    /// for the last rendered frame, filled while `set_cell_data(true)`.
    pub fn cell_data(&self) -> *const u8 {
        self.cell_data.as_ptr()
    }

    /// The glowing parts of the last rendered frame (flames, crystals, cells
    /// above 400 °C) as flat `[x, y, kind, strength]` groups in viewport
    /// cells; kind is 1 hot, 2 crystal, 3 flame. At most `GLOW_CAP` groups.
    pub fn emissive(&self) -> Vec<f32> {
        self.glow.entries()
    }

    /// How far along the suit's next automatic refills are, `[water, gas]`,
    /// each 0..=1, or -1 when that process is idle; a third value is 1 while
    /// the suit holds still after a cast.
    pub fn refill(&self) -> Vec<f32> {
        let [w, g] = self.game.refill_progress();
        vec![w, g, if self.game.suit_paused() { 1.0 } else { 0.0 }]
    }

    /// Hydrogen and oxygen in view, for glowing bubbles: flat `[x, y, kind]`
    /// groups in viewport cells (kind 1 H2, 2 O2), at most 240 groups.
    pub fn gas_bubbles(&self) -> Vec<f32> {
        self.game.gas_cells(self.drawn_view_x, 240)
    }

    /// The heat lens's labels: up to three of the hottest spots in view
    /// (and the coldest, when something is freezing), as flat
    /// `[x, y, temp_c]` groups in viewport cells.
    pub fn hotspots(&self) -> Vec<f32> {
        self.game
            .hotspots(self.drawn_view_x, 3)
            .into_iter()
            .flat_map(|(x, y, t)| [(x - self.drawn_view_x) as f32, y as f32, t])
            .collect()
    }
}

/// Species byte in `cell_data` for sprite pixels (drawn as-is by the GPU).
pub const SPRITE: u8 = 255;

/// The frame being drawn: RGBA pixels, plus the optional per-pixel cell
/// data for the WebGL renderer.
struct Frame<'a> {
    rgba: &'a mut [u8],
    data: Option<&'a mut Vec<u8>>,
}

fn sky(y: usize) -> [u8; 3] {
    palette::sky(y, HEIGHT)
}

fn render(
    game: &Game,
    out: &mut Frame,
    glow: &mut GlowMap,
    lens: bool,
    alpha: f32,
    view_x: i32,
    crosshair: bool,
) {
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
                    palette::lens_solid(base)
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
                let c = match gate_at(s, x, y) {
                    Some(k) => pylon(x as i32 - GATE_X[k], y, game.rooms.gates_open[k], g.frame),
                    None => c,
                };
                if t > 400.0 && s != Species::Flame {
                    mix(c, [255, 110, 30], (t - 400.0) / 1400.0)
                } else {
                    c
                }
            };
            out.rgba[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            if let Some(d) = out.data.as_deref_mut() {
                let o2 = (y * VIEW_W + sx) * 2;
                d[o2] = temp_byte(t);
                d[o2 + 1] = s as u8;
            }
        }
    }
    draw_torch(game, out, view_x);
    draw_player(game, out, alpha, view_x);
    if crosshair {
        draw_aim(game, out, view_x);
    }
}

/// Which gate's overhead pylon a stone cell belongs to, if any.
fn gate_at(s: Species, x: usize, y: usize) -> Option<usize> {
    if s != Species::Stone || y >= DOOR_TOP as usize {
        return None;
    }
    let x = x as i32;
    GATE_X.iter().position(|&gx| x >= gx && x < gx + GATE_W)
}

/// The stone above each gate drawn as what it is in the story: a steel
/// gate pylon with bolted bands and a light strip (amber while the gate is
/// shut, green once it opens), so it never reads as a glitch in the sky.
fn pylon(lx: i32, y: usize, open: bool, frame: u32) -> [u8; 3] {
    let band = y % 16 < 2;
    let mut c = match lx {
        0 => [150, 172, 196],
        l if l == GATE_W - 1 => [36, 44, 58],
        _ => [72, 90, 114],
    };
    if band {
        c = if lx == 1 || lx == GATE_W - 2 {
            [196, 206, 220]
        } else {
            [48, 58, 74]
        };
    } else if lx == GATE_W / 2 - 1 || lx == GATE_W / 2 {
        // The light strip, with a slow pulse running down it while shut.
        let pulse = !open && (y as u32 + frame / 3) % 24 < 4;
        c = match (open, pulse) {
            (true, _) => [80, 190, 130],
            (false, true) => [255, 214, 120],
            (false, false) => [230, 150, 40],
        };
    }
    // The emitter cap just above the doorway.
    if y + 3 >= DOOR_TOP as usize {
        c = [30, 36, 48];
    }
    c
}

/// The room-2 torch: a wooden post with a flickering flame when lit. Drawn
/// in the player's hand when carried.
fn draw_torch(game: &Game, out: &mut Frame, view_x: i32) {
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

fn put(out: &mut Frame, x: i32, y: i32, c: [u8; 3]) {
    if x >= 0 && y >= 0 && (x as usize) < VIEW_W && (y as usize) < HEIGHT {
        let p = y as usize * VIEW_W + x as usize;
        out.rgba[p * 4..p * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        if let Some(d) = out.data.as_deref_mut() {
            d[p * 2 + 1] = SPRITE;
        }
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

/// The player's pose for this frame: the top-left cell in world columns and
/// rows (interpolated by `alpha`), and each lit sprite cell as
/// `(dx, dy, colour)`. One source for the frame and the browser overlay.
/// One lit sprite cell: offset from the sprite's top-left and its colour.
type SpriteCell = (i32, i32, [u8; 3]);

fn player_cells(game: &Game, alpha: f32) -> (i32, i32, Vec<SpriteCell>) {
    let p = &game.player;
    let lerp = |a: f32, b: f32| a + (b - a) * alpha;
    let (px, py) = (
        lerp(p.prev_x, p.x).round() as i32,
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
    let mut cells = Vec::with_capacity(72);
    for (row, line) in BODY.iter().chain(legs.iter()).enumerate() {
        for (col, &c) in line.iter().enumerate() {
            if let Some(color) = sprite_color(c) {
                let dx = if p.facing >= 0 { col } else { 5 - col } as i32;
                cells.push((dx, row as i32, color));
            }
        }
    }
    (px, py, cells)
}

fn draw_player(game: &Game, out: &mut Frame, alpha: f32, view_x: i32) {
    let (px, py, cells) = player_cells(game, alpha);
    for (dx, dy, color) in cells {
        put(out, px - view_x + dx, py + dy, color);
    }
}

fn draw_aim(game: &Game, out: &mut Frame, view_x: i32) {
    let (x, y) = game.shown_aim();
    let (ax, ay) = (x - view_x, y);
    // A dark outline under white ticks, so the reticle reads on sky, fire and snow.
    let (c, edge) = ([255, 255, 255], [20, 24, 32]);
    for d in 2..=5i32 {
        for (dx, dy) in [(d, 0), (-d, 0), (0, d), (0, -d)] {
            let (ox, oy) = (dy.signum(), dx.signum());
            put(out, ax + dx + ox, ay + dy + oy, edge);
            put(out, ax + dx - ox, ay + dy - oy, edge);
        }
    }
    for d in 2..=5 {
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

/// The Insight lens colour map as a 256-entry RGBA lookup table indexed by
/// temperature byte (`palette::temp_byte`), for the WebGL renderer.
#[wasm_bindgen]
pub fn heat_lut() -> Vec<u8> {
    palette::heat_lut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v3_level_exports_swap_the_game_and_control_time() {
        let mut w = WebGame::new("seedling", 1);
        assert!(w.is_story());
        assert!(!w.new_trial("seedling", "T7", 1), "T7 is Apprentice only");
        assert!(w.is_story());
        assert!(w.new_trial("seedling", "T1", 1));
        assert!(!w.is_story());
        assert_eq!(w.level_id(), "T1");
        assert!(w.trial_result().contains("\"id\":\"T1\""));
        assert_eq!(w.goals(), "[]");
        let list: serde_json::Value = serde_json::from_str(&w.trial_list("sprout")).unwrap();
        assert_eq!(list.as_array().unwrap().len(), 5);

        w.new_lab("sprout", 2);
        assert_eq!(w.level_id(), "lab");
        w.paint(100, 100, Species::Sand as u8, 2);
        assert_eq!(w.game.grid.get(100, 100), Species::Sand);
        w.erase(100, 100, 2);
        assert_eq!(w.game.grid.get(100, 100), Species::Air);

        let t0 = w.game.ticks;
        w.set_paused(true);
        w.tick(0);
        assert_eq!(w.game.ticks, t0, "paused");
        w.step_once();
        assert_eq!(w.game.ticks, t0 + 1, "step works while paused");
        w.set_paused(false);
        w.set_speed(0.25);
        for _ in 0..8 {
            w.tick(0);
        }
        assert_eq!(w.game.ticks, t0 + 3);
        w.set_speed(2.0);
        w.tick(0);
        assert_eq!(w.game.ticks, t0 + 5);
        w.restart();
        assert_eq!(w.level_id(), "lab");
        w.new_story("seedling", 1);
        assert!(w.is_story());
        assert_eq!(w.trial_result(), "null");
    }

    /// The WebGL renderer's per-pixel data matches the grid it was drawn
    /// from, and marks the player's pixels as sprite so the GPU draws them
    /// as they are (even under the Insight lens).
    #[test]
    fn cell_data_matches_the_grid_and_marks_sprites() {
        let mut w = WebGame::new("apprentice", 9);
        for _ in 0..10 {
            w.tick(0);
        }
        w.render(false, 1.0);
        assert!(w.cell_data.iter().all(|&b| b == 0), "off by default");
        w.set_cell_data(true);
        w.render(false, 1.0);
        let vx = w.view_x() as usize;
        let g = &w.game;
        let (px, py) = (
            g.player.x.round() as usize - vx + 2,
            g.player.y.round() as usize + 5,
        );
        let mut checked = 0;
        for y in 0..HEIGHT {
            for sx in 0..VIEW_W {
                let o = (y * VIEW_W + sx) * 2;
                let i = g.grid.idx(sx + vx, y);
                if w.cell_data[o + 1] == SPRITE {
                    continue;
                }
                assert_eq!(w.cell_data[o + 1], g.grid.cells[i] as u8);
                assert_eq!(w.cell_data[o], temp_byte(g.grid.temp[i]));
                checked += 1;
            }
        }
        assert!(checked > VIEW_W * HEIGHT * 9 / 10);
        assert_eq!(w.cell_data[(py * VIEW_W + px) * 2 + 1], SPRITE);
        assert_eq!(heat_lut().len(), 1024);
    }

    /// The overlay sprite is the same pose the frame draws: every sprite
    /// cell it lists is a sprite pixel in the frame.
    #[test]
    fn player_sprite_matches_the_frame() {
        let mut w = WebGame::new("seedling", 3);
        for _ in 0..10 {
            w.tick(0);
        }
        w.set_cell_data(true);
        w.render(false, 1.0);
        let s = w.player_sprite(1.0);
        let vx = w.view_x();
        assert!(s.len() > 2 + 3 * 40, "a 6 x 12 sprite, mostly filled");
        for c in s[2..].chunks(3) {
            let (x, y) = ((s[0] + c[0] - vx) as usize, (s[1] + c[1]) as usize);
            let o = y * VIEW_W + x;
            assert_eq!(w.cell_data[o * 2 + 1], SPRITE);
        }
    }
}
