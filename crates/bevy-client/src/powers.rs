//! Spells are powers (Voxel mode): why a player learns and practises spells.
//!
//! - **Rank.** Every spell's stage (Learning 1, Practised 2, Mastered 3) and
//!   every cache found earns rank points. Five sci-fi cultivation tiers, from
//!   Spark to Archon. A rank-up shows a banner and plays a fanfare
//!   (`web/rank.js`); the HUD badge always shows the rank and the next one.
//! - **Powers (F).** The selected spell's traversal power: Fire's Flare (a
//!   light for dark caves), Water's H2 lift (rise like a hydrogen balloon),
//!   Frost's ice walkway (freeze the water's surface ahead) and CO2's dash
//!   (a gas burst pushes you the other way). An Unknown spell's power is
//!   locked until it is learned at the Formula Bench.
//! - **Stronger with practice.** A Practised or Mastered spell flies further,
//!   recharges faster, and its power is bigger (`Scale`).
//! - **Caches.** Four optional places, one per spell, stamped into the world
//!   at start: the Ember Vault (a thicket door only Fire clears), the Frost
//!   Isle (a moat too wide to jump), the Sky Pillar (too tall to climb) and
//!   the Ravine Cache (a gap too wide to jump). Each holds crystals and a
//!   rank point. The page remembers found caches on this device.
//!
//! The rules are pure functions on `World` and `Player`, so the tests walk
//! the real movement code (`voxel_view::walk`) over the stamped world.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use realgame_sim::valley::Item;
use realgame_voxel::{mat, Aabb, Cell, World};

use crate::player::Player;
use crate::progress::Stage;
use crate::voxel_view::VoxWorld;

/// Rank tiers: name and the points they start at.
pub const RANKS: [(&str, u32); 5] = [
    ("Spark", 0),
    ("Kindler", 2),
    ("Catalyst", 5),
    ("Synthesist", 9),
    ("Archon", 13),
];

/// Points for one spell's stage.
pub fn stage_points(s: Stage) -> u32 {
    match s {
        Stage::Unknown => 0,
        Stage::Learning => 1,
        Stage::Practised => 2,
        Stage::Mastered => 3,
    }
}

/// Rank points: every spell's stage plus one per cache found.
pub fn points(stages: &[Stage; 4], caches: u32) -> u32 {
    stages.iter().map(|s| stage_points(*s)).sum::<u32>() + caches
}

/// The rank index for `points`.
pub fn rank_of(points: u32) -> usize {
    RANKS.iter().rposition(|r| points >= r.1).unwrap_or(0)
}

/// How much stronger a spell is at a stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    /// Spell reach (arcs) and spray range (cones), times the base.
    pub range: f32,
    /// The cast cooldown, times the base.
    pub cooldown: f32,
    /// Seconds before the power can be used again.
    pub power_cd: f32,
    /// Lift: blocks risen.
    pub lift: f32,
    /// Dash: horizontal speed, m/s.
    pub dash: f32,
    /// Walkway: metres frozen ahead, and blocks either side of the line.
    pub walk_len: f32,
    pub walk_half: i32,
    /// Flare: seconds of light.
    pub flare: f32,
}

pub fn scale(s: Stage) -> Scale {
    match s {
        Stage::Unknown | Stage::Learning => Scale {
            range: 1.0,
            cooldown: 1.0,
            power_cd: 3.0,
            lift: 8.0,
            dash: 20.0,
            walk_len: 15.5,
            walk_half: 0,
            flare: 20.0,
        },
        Stage::Practised => Scale {
            range: 1.3,
            cooldown: 0.75,
            power_cd: 2.0,
            lift: 10.0,
            dash: 22.0,
            walk_len: 19.5,
            walk_half: 1,
            flare: 30.0,
        },
        Stage::Mastered => Scale {
            range: 1.6,
            cooldown: 0.5,
            power_cd: 1.2,
            lift: 12.0,
            dash: 24.0,
            walk_len: 23.5,
            walk_half: 1,
            flare: 45.0,
        },
    }
}

/// The lift rises at this speed (m/s) until it has gained its height.
pub const LIFT_RISE: f32 = 5.0;
/// While lifted (until landing) the player drifts at most this fast (m/s):
/// a balloon is steered, not thrown.
pub const LIFT_DRIFT: f32 = 1.5;
/// The dash's upward speed (m/s): a low hop.
pub const DASH_UP: f32 = 5.0;
/// Placed ice starts this cold (K), well below freezing.
pub const ICE_K: f32 = 258.0;

pub const SPELL_NAMES: [&str; 4] = ["Fire", "Water", "Frost", "CO₂"];
pub const POWER_NAMES: [&str; 4] = ["Flare", "H₂ lift", "Ice walkway", "CO₂ dash"];
const POWER_HINTS: [&str; 4] = [
    "a bright light that follows you through dark caves",
    "rise like a hydrogen balloon onto high ledges",
    "freeze a path across deep water",
    "a burst of gas pushes you across gaps",
];

/// One cache: where it is and which spell opens it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    pub id: &'static str,
    pub name: &'static str,
    pub spell: usize,
    /// Column of the centre.
    pub at: [i32; 2],
}

/// Caches (the first is the plaza's guided one).
pub const N_SITES: usize = 5;

pub const SITES: [Site; N_SITES] = [
    Site {
        id: "ledge",
        name: "Plaza Tower",
        spell: 1,
        at: [134, 196],
    },
    Site {
        id: "vault",
        name: "Ember Vault",
        spell: 0,
        at: [112, 190],
    },
    Site {
        id: "pillar",
        name: "Sky Pillar",
        spell: 1,
        at: [176, 128],
    },
    Site {
        id: "isle",
        name: "Frost Isle",
        spell: 2,
        at: [70, 120],
    },
    Site {
        id: "ravine",
        name: "Ravine Cache",
        spell: 3,
        at: [214, 128],
    },
];

/// Crystals in each cache.
pub const REWARD: u16 = 4;

// Cache shapes (blocks, Chebyshev distance from the centre column).
/// Frost Isle: the island, and the moat's outer ring (13 wide).
const ISLE: i32 = 1;
const MOAT: i32 = 14;
/// Sky Pillar: 3×3, 7 blocks tall.
const PILLAR: i32 = 1;
pub const PILLAR_H: i32 = 7;
/// Ravine: the 5×5 platform, the pit's outer ring (9 wide), its depth.
const PLATFORM: i32 = 2;
const PIT: i32 = 11;
const PIT_DEPTH: i32 = 4;
/// Plaza Tower: 3×3, 5 blocks tall (a jump reaches 1).
pub const TOWER_H: i32 = 5;
/// Ember Vault: walls on ring 2, 4 blocks tall.
const VAULT: i32 = 2;
const VAULT_H: i32 = 4;

/// The flat pad radius and the ramp beyond it, per cache.
fn pad(id: &str) -> (i32, i32) {
    match id {
        "ledge" => (3, 0),
        "vault" => (4, 4),
        "pillar" => (9, 6),
        "isle" => (MOAT + 3, 6),
        _ => (PIT + 2, 4),
    }
}

/// What a stamped cache looks like to the game: the ground height, where
/// the crystal orb floats and the protected box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Built {
    pub site: Site,
    /// The feet height on the pad (the first air cell above the ground).
    pub h: i32,
    pub orb: Vec3,
    pub guard: Aabb,
}

fn surface_mat(spell: usize) -> u8 {
    match spell {
        2 => mat::SNOW,
        0 => mat::GRASS,
        _ => mat::MUD,
    }
}

fn solid(w: &World, p: [i32; 3]) -> bool {
    realgame_voxel::materials::flags(w.get(p).mat).solid
}

/// The natural ground (feet y) of a column, ignoring trees.
fn ground(w: &World, x: i32, z: i32) -> i32 {
    let mut y = realgame_voxel::WORLD[1] - 2;
    while y > 1 {
        let m = w.get([x, y - 1, z]).mat;
        if solid(w, [x, y - 1, z]) && !matches!(m, mat::LEAVES | mat::WOOD) {
            return y;
        }
        y -= 1;
    }
    1
}

/// Levels a round-cornered pad at feet height `h`, ramping back to the
/// natural ground by at most one block per block outward (walkable).
fn level(w: &mut World, c: [i32; 2], h: i32, r: i32, ramp: i32, top: u8) {
    let top_y = realgame_voxel::WORLD[1] - 1;
    for dz in -(r + ramp)..=(r + ramp) {
        for dx in -(r + ramp)..=(r + ramp) {
            let (x, z) = (c[0] + dx, c[1] + dz);
            let d = dx.abs().max(dz.abs());
            let natural = ground(w, x, z);
            let t = if d <= r {
                h
            } else {
                natural.clamp(h - (d - r), h + (d - r))
            };
            for y in t..top_y {
                if w.get([x, y, z]).mat != mat::AIR {
                    w.set([x, y, z], Cell::AIR);
                }
            }
            for y in (t - 4)..(t - 1) {
                if !solid(w, [x, y, z]) {
                    w.set([x, y, z], Cell::new(mat::DIRT));
                }
            }
            w.set([x, t - 1, z], Cell::new(top));
        }
    }
}

/// Stamps one cache into the world and protects it from digging and
/// building (powers change protected blocks; the player's hands do not).
pub fn stamp(w: &mut World, site: Site) -> Built {
    let c = site.at;
    let h = ground(w, c[0], c[1]);
    let (r, ramp) = pad(site.id);
    let top = surface_mat(site.spell);
    level(w, c, h, r, ramp, top);
    let at = |dx: i32, y: i32, dz: i32| [c[0] + dx, y, c[1] + dz];
    let ring = |rad: i32| {
        let mut v = Vec::new();
        for dz in -rad..=rad {
            for dx in -rad..=rad {
                v.push((dx, dz, dx.abs().max(dz.abs())));
            }
        }
        v
    };
    let (orb_y, guard) = match site.id {
        "vault" => {
            for (dx, dz, d) in ring(VAULT) {
                for y in h..h + VAULT_H {
                    let cell = if d == VAULT { mat::BRICK } else { mat::AIR };
                    w.set(at(dx, y, dz), Cell::new(cell));
                }
                w.set(at(dx, h + VAULT_H, dz), Cell::new(mat::BRICK));
                w.set(at(dx, h - 1, dz), Cell::new(mat::BRICK));
            }
            // The door: a dry thicket (leaves), two high, on the south wall.
            for y in h..h + 2 {
                w.set(at(0, y, VAULT), Cell::new(mat::LEAVES));
            }
            (
                h as f32 + 0.9,
                Aabb {
                    min: at(-VAULT, h - 2, -VAULT),
                    max: at(VAULT + 1, h + VAULT_H + 2, VAULT + 1),
                },
            )
        }
        "ledge" => {
            for (dx, dz, d) in ring(PILLAR) {
                if d <= PILLAR {
                    for y in h..h + TOWER_H {
                        w.set(at(dx, y, dz), Cell::new(mat::CLIFFSTONE));
                    }
                }
            }
            let g = 3;
            (
                (h + TOWER_H) as f32 + 0.9,
                Aabb {
                    min: at(-g, h - 2, -g),
                    max: at(g + 1, realgame_voxel::WORLD[1], g + 1),
                },
            )
        }
        "pillar" => {
            for (dx, dz, d) in ring(PILLAR) {
                if d <= PILLAR {
                    for y in h..h + PILLAR_H {
                        w.set(at(dx, y, dz), Cell::new(mat::CLIFFSTONE));
                    }
                }
            }
            let g = 8;
            (
                (h + PILLAR_H) as f32 + 0.9,
                Aabb {
                    min: at(-g, h - 3, -g),
                    max: at(g + 1, realgame_voxel::WORLD[1], g + 1),
                },
            )
        }
        "isle" => {
            for (dx, dz, d) in ring(MOAT) {
                if d > ISLE {
                    // Two blocks of water on a stone floor, level with the
                    // banks: too deep to wade.
                    w.set(at(dx, h - 3, dz), Cell::new(mat::STONE));
                    for y in h - 2..h {
                        let mut water = Cell::new(mat::WATER);
                        water.set_level(8);
                        w.set(at(dx, y, dz), water);
                    }
                }
            }
            let g = MOAT + 2;
            (
                h as f32 + 0.9,
                Aabb {
                    min: at(-g, h - 4, -g),
                    max: at(g + 1, realgame_voxel::WORLD[1], g + 1),
                },
            )
        }
        _ => {
            for (dx, dz, d) in ring(PIT) {
                if d > PLATFORM {
                    for y in h - PIT_DEPTH..h {
                        w.set(at(dx, y, dz), Cell::AIR);
                    }
                    w.set(at(dx, h - PIT_DEPTH - 1, dz), Cell::new(top));
                } else {
                    w.set(at(dx, h - 1, dz), Cell::new(mat::STONE));
                    for y in h - PIT_DEPTH - 1..h - 1 {
                        w.set(at(dx, y, dz), Cell::new(mat::STONE));
                    }
                }
            }
            // Steps out of the pit along the east wall (a block below the
            // rim at the top, so they are no launch pad).
            for (k, dz) in [-3, -2, -1].into_iter().enumerate() {
                for y in h - PIT_DEPTH..h - PIT_DEPTH + k as i32 + 1 {
                    w.set(at(PIT, y, dz), Cell::new(mat::STONE));
                }
            }
            let g = PIT + 1;
            (
                h as f32 + 0.9,
                Aabb {
                    min: at(-g, h - PIT_DEPTH - 2, -g),
                    max: at(g + 1, realgame_voxel::WORLD[1], g + 1),
                },
            )
        }
    };
    w.protect(guard);
    Built {
        site,
        h,
        orb: Vec3::new(c[0] as f32 + 0.5, orb_y, c[1] as f32 + 0.5),
        guard,
    }
}

/// True when the player is close enough to take the orb.
pub fn touches(player: &Player, orb: Vec3) -> bool {
    let body = player.pos + Vec3::Y * 0.9;
    body.distance(orb) < 1.4
}

/// Freezes the top layer of water ahead of `feet` along `yaw`: `len`
/// metres, `half` blocks either side. Returns the cells turned to ice.
pub fn freeze_walkway(w: &mut World, feet: Vec3, yaw: f32, len: f32, half: i32) -> Vec<[i32; 3]> {
    let fwd = Vec2::new(-yaw.sin(), -yaw.cos());
    let side = Vec2::new(fwd.y, -fwd.x);
    let fy = feet.y.floor() as i32;
    let mut out = Vec::new();
    let mut t = 0.0;
    while t <= len {
        let p = Vec2::new(feet.x, feet.z) + fwd * t;
        for k in -half..=half {
            let q = p + side * k as f32;
            let (x, z) = (q.x.floor() as i32, q.y.floor() as i32);
            for y in [fy - 1, fy - 2] {
                let cell = w.get([x, y, z]);
                if cell.mat == mat::WATER && w.get([x, y + 1, z]).mat == mat::AIR {
                    let mut ice = Cell::new(mat::ICE);
                    ice.set_temp_k(ICE_K);
                    w.set([x, y, z], ice);
                    out.push([x, y, z]);
                    break;
                }
            }
        }
        t += 0.35;
    }
    out
}

/// Ongoing power effects on the body.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    /// Lifting: the height to rise to and the last height seen.
    pub lift_to: Option<f32>,
    last_y: f32,
    age: f32,
    /// Lifted and not yet landed: drift is limited.
    pub floating: bool,
}

impl Motion {
    /// Starts a lift of `blocks` from the ground. False when airborne.
    pub fn lift(&mut self, player: &mut Player, blocks: f32) -> bool {
        if !player.grounded {
            return false;
        }
        self.lift_to = Some(player.pos.y + blocks);
        self.last_y = player.pos.y;
        self.age = 0.0;
        self.floating = true;
        player.vel.y = LIFT_RISE;
        player.grounded = false;
        true
    }

    /// Launches a dash along the view at `speed`. False when airborne.
    pub fn dash(&mut self, player: &mut Player, speed: f32) -> bool {
        if !player.grounded {
            return false;
        }
        let fwd = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
        player.vel = fwd * speed + Vec3::Y * DASH_UP;
        player.grounded = false;
        true
    }

    /// Runs after the frame's movement: holds the lift's rise until it has
    /// gained its height (or hit a ceiling) and limits drift until landing.
    pub fn after_move(&mut self, player: &mut Player, dt: f32) {
        if let Some(to) = self.lift_to {
            self.age += dt;
            let rose = player.pos.y - self.last_y;
            self.last_y = player.pos.y;
            let stuck = self.age > 0.25 && rose < LIFT_RISE * dt * 0.2;
            if player.pos.y >= to || stuck {
                self.lift_to = None;
            } else {
                player.vel.y = LIFT_RISE;
                player.grounded = false;
            }
        }
        if self.floating {
            if player.grounded && self.lift_to.is_none() {
                self.floating = false;
            } else {
                let h = Vec2::new(player.vel.x, player.vel.z);
                if h.length() > LIFT_DRIFT {
                    let h = h.normalize() * LIFT_DRIFT;
                    player.vel.x = h.x;
                    player.vel.z = h.y;
                }
            }
        }
    }
}

/// What the page shows and the tests read.
#[derive(Resource, Default)]
pub struct Powers {
    pub built: Vec<Built>,
    pub found: [bool; N_SITES],
    pub motion: Motion,
    /// Seconds before each spell's power can be used again.
    pub cd: [f32; 4],
    /// Flare light left, seconds.
    pub flare: f32,
    pub stages: [Stage; 4],
    pub rank: usize,
    pub points: u32,
    pub rank_ups: u32,
    /// Uses per power (Flare, lift, walkway, dash).
    pub used: [u32; 4],
    /// Ice cells the last walkway made.
    pub last_walkway: usize,
    /// A rank baseline exists (the first frame sets it silently).
    seen: Option<(u32, [bool; 4], u32)>,
    last_casts: u32,
    /// The guided first lift: seconds left (set when Water is learned in
    /// play and the Plaza Tower is not yet found).
    pub guide: f32,
    stages_before: [Stage; 4],
    /// The guide's line now (empty when there is none).
    pub hint: String,
}

/// Seconds the guided first lift stays up.
pub const GUIDE_S: f32 = 150.0;

/// The guided first lift's line: `water` is Water's stage, `held` whether
/// Water is the selected spell, `dist` metres (flat) to the tower and
/// `floating` whether a lift is under way.
pub fn guide_line(water: Stage, held: bool, dist: f32, floating: bool, guided: bool) -> String {
    if water == Stage::Unknown {
        return if dist <= 5.0 {
            "Something glows up there. Learn Water at the Formula Bench (B) to rise up to it."
                .into()
        } else {
            String::new()
        };
    }
    if !guided {
        return String::new();
    }
    if floating {
        "Hold W to drift onto the top of the tower!".into()
    } else if !held {
        "New power: H₂ lift! Press 2 for Water, then F by the glowing tower.".into()
    } else if dist > 3.0 {
        format!("Walk to the glowing tower ({dist:.0} m), then press F to rise.")
    } else {
        "Press F to rise with the H₂ lift, then hold W onto the top.".into()
    }
}

/// Spell range multipliers for `voxel_cast` (by spell index).
#[derive(Resource, Clone, Copy)]
pub struct Boost(pub [f32; 4]);

impl Default for Boost {
    fn default() -> Self {
        Boost([1.0; 4])
    }
}

impl Powers {
    pub fn found_n(&self) -> u32 {
        self.found.iter().filter(|f| **f).count() as u32
    }

    /// Recomputes rank. `rev` is the progress book's change counter and
    /// `practised` the run's practice: a change there (or a new cache) is a
    /// rank-up moment; a load from the save is not.
    pub fn rerank(&mut self, stages: [Stage; 4], rev: u32, practised: [bool; 4]) -> bool {
        self.stages = stages;
        if self.seen.is_none() {
            self.stages_before = stages;
        }
        self.points = points(&stages, self.found_n());
        let rank = rank_of(self.points);
        let key = (rev, practised, self.found_n());
        let earned = self.seen.is_some_and(|s| s != key);
        self.seen = Some(key);
        // Water learned in play: the guided first lift.
        if earned
            && self.stages_before[1] == Stage::Unknown
            && stages[1] != Stage::Unknown
            && !self.found[0]
        {
            self.guide = GUIDE_S;
        }
        self.stages_before = stages;
        let up = earned && rank > self.rank;
        self.rank = rank;
        if up {
            self.rank_ups += 1;
        }
        up
    }

    pub fn json(&self, spell: usize) -> String {
        let (name, _) = RANKS[self.rank];
        let next = RANKS.get(self.rank + 1);
        let stage = self.stages.get(spell).copied().unwrap_or_default();
        let caches: Vec<String> = self
            .built
            .iter()
            .map(|b| {
                let i = SITES.iter().position(|s| s.id == b.site.id).unwrap_or(0);
                format!(
                    "{{\"id\":\"{}\",\"name\":\"{}\",\"spell\":{},\"found\":{},\"pos\":[{:.1},{:.1},{:.1}],\"h\":{}}}",
                    b.site.id,
                    b.site.name,
                    b.site.spell,
                    self.found[i],
                    b.orb.x,
                    b.orb.y,
                    b.orb.z,
                    b.h
                )
            })
            .collect();
        let stages: Vec<String> = self
            .stages
            .iter()
            .map(|s| format!("\"{}\"", s.as_str()))
            .collect();
        format!(
            "{{\"rank\":{},\"rankName\":\"{}\",\"points\":{},\"next\":{},\"nextAt\":{},\"rankUps\":{},\"spell\":{},\"power\":\"{}\",\"unlocked\":{},\"cd\":{:.2},\"lifting\":{},\"floating\":{},\"flare\":{:.1},\"used\":[{},{},{},{}],\"walkway\":{},\"stages\":[{}],\"found\":{},\"guide\":\"{}\",\"caches\":[{}]}}",
            self.rank,
            name,
            self.points,
            next.map_or("null".to_string(), |n| format!("\"{}\"", n.0)),
            next.map_or("null".to_string(), |n| n.1.to_string()),
            self.rank_ups,
            spell,
            POWER_NAMES.get(spell).copied().unwrap_or(""),
            stage != Stage::Unknown,
            self.cd[spell.min(3)].max(0.0),
            self.motion.lift_to.is_some(),
            self.motion.floating,
            self.flare.max(0.0),
            self.used[0],
            self.used[1],
            self.used[2],
            self.used[3],
            self.last_walkway,
            stages.join(","),
            self.found_n(),
            self.hint,
            caches.join(",")
        )
    }
}

/// What a press of F did.
#[derive(Debug, PartialEq)]
pub enum Use {
    Locked,
    Cooling,
    Flare,
    Lift,
    Walkway(usize),
    NoWater,
    Dash,
    NotGrounded,
}

/// Uses spell `spell`'s power at `stage`.
pub fn use_power(
    pw: &mut Powers,
    world: Option<&mut World>,
    player: &mut Player,
    spell: usize,
    stage: Stage,
) -> Use {
    if stage == Stage::Unknown {
        return Use::Locked;
    }
    if pw.cd[spell.min(3)] > 0.0 {
        return Use::Cooling;
    }
    let s = scale(stage);
    let out = match spell {
        0 => {
            pw.flare = s.flare;
            Use::Flare
        }
        1 => {
            if pw.motion.lift(player, s.lift) {
                Use::Lift
            } else {
                Use::NotGrounded
            }
        }
        2 => {
            let cells = match world {
                Some(w) => freeze_walkway(w, player.pos, player.yaw, s.walk_len, s.walk_half),
                None => Vec::new(),
            };
            pw.last_walkway = cells.len();
            if cells.is_empty() {
                Use::NoWater
            } else {
                Use::Walkway(cells.len())
            }
        }
        _ => {
            if pw.motion.dash(player, s.dash) {
                Use::Dash
            } else {
                Use::NotGrounded
            }
        }
    };
    if matches!(out, Use::Flare | Use::Lift | Use::Walkway(_) | Use::Dash) {
        pw.cd[spell.min(3)] = s.power_cd;
        pw.used[spell.min(3)] += 1;
    }
    out
}

/// The caption for a power use.
pub fn line(u: &Use, spell: usize, cd: f32) -> String {
    let i = spell.min(3);
    match u {
        Use::Locked => format!(
            "{} is locked. Learn {} at the Formula Bench (B) to unlock it: {}.",
            POWER_NAMES[i], SPELL_NAMES[i], POWER_HINTS[i]
        ),
        Use::Cooling => format!("{} recharging: {:.1} s", POWER_NAMES[i], cd.max(0.0)),
        Use::Flare => "Flare! Burning fuel gives out light as well as heat.".into(),
        Use::Lift => "H₂ lift! Hydrogen is much less dense than air, so its balloon rises.".into(),
        Use::Walkway(_) => {
            "Ice walkway! Water freezes from the top, because ice is less dense than water.".into()
        }
        Use::NoWater => "Ice walkway: no open water ahead to freeze. Face the water.".into(),
        Use::Dash => {
            "CO₂ dash! Gas rushing out one way pushes you the other way (Newton's third law)."
                .into()
        }
        Use::NotGrounded => format!("{}: stand on the ground first.", POWER_NAMES[i]),
    }
}

// ---------------------------------------------------------------------
// Bevy side
// ---------------------------------------------------------------------

#[derive(Component)]
struct Orb(usize);

#[derive(Component)]
struct FlareLight;

#[derive(Component)]
struct Beam;

pub fn plugin(app: &mut App) {
    // Stamp the caches now, before the chunks are meshed (Startup).
    let mut pw = Powers::default();
    if let Some(mut w) = app.world_mut().get_resource_mut::<VoxWorld>() {
        for s in SITES {
            pw.built.push(stamp(&mut w.0, s));
        }
    }
    app.insert_resource(pw)
        .init_resource::<Boost>()
        .add_systems(Startup, spawn)
        .add_systems(
            Update,
            (commands, run, draw, publish)
                .chain()
                .after(crate::aim_and_cast)
                .before(crate::publish),
        );
}

fn spawn(
    mut commands: Commands,
    pw: Res<Powers>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ball = meshes.add(Sphere::new(0.32).mesh().ico(2).unwrap());
    let colors = [
        Color::srgb(1.0, 0.55, 0.2),
        Color::srgb(0.35, 0.7, 1.0),
        Color::srgb(0.7, 0.95, 1.0),
        Color::srgb(0.8, 1.0, 0.85),
    ];
    for b in &pw.built {
        let i = SITES.iter().position(|s| s.id == b.site.id).unwrap_or(0);
        let c = colors[b.site.spell.min(3)];
        let m = materials.add(StandardMaterial {
            base_color: c,
            emissive: LinearRgba::from(c) * 6.0,
            ..default()
        });
        commands
            .spawn((
                Mesh3d(ball.clone()),
                MeshMaterial3d(m),
                Transform::from_translation(b.orb),
                NotShadowCaster,
                Orb(i),
                Visibility::Visible,
            ))
            .with_child(PointLight {
                color: c,
                intensity: 30_000.0,
                range: 7.0,
                shadow_maps_enabled: false,
                ..default()
            });
    }
    // A beam of light over the Plaza Tower until its cache is taken.
    if let Some(b) = pw.built.first() {
        let beam = materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.8, 1.0, 0.35),
            emissive: LinearRgba::rgb(0.6, 1.2, 2.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.12, 24.0))),
            MeshMaterial3d(beam),
            Transform::from_translation(b.orb + Vec3::Y * 12.5),
            NotShadowCaster,
            Orb(0),
            Beam,
            Visibility::Visible,
        ));
    }
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.75, 0.45),
            intensity: 0.0,
            range: 18.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        FlareLight,
    ));
}

/// The stage a spell counts as now (the save, raised by this run's practice).
fn stages(prog: &crate::Progress, working: &crate::Working) -> [Stage; 4] {
    [0, 1, 2, 3].map(|i| working.stage(prog, i))
}

fn commands(
    mut pw: ResMut<Powers>,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<crate::Game>,
    mut player: ResMut<Player>,
    mut world: Option<ResMut<VoxWorld>>,
    prog: Res<crate::Progress>,
    working: Res<crate::Working>,
) {
    let mut presses = 0;
    for line in drain() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("power") => presses += 1,
            Some("found") => {
                if let Some(i) = it
                    .next()
                    .and_then(|id| SITES.iter().position(|s| s.id == id))
                {
                    pw.found[i] = true;
                    // Restored from this device's save: not a rank-up.
                    pw.seen = None;
                }
            }
            _ => {}
        }
    }
    if !game.paused && keys.just_pressed(KeyCode::KeyF) {
        presses += 1;
    }
    if presses == 0 || game.paused {
        return;
    }
    let spell = game.spell.min(3);
    let stage = stages(&prog, &working)[spell];
    let used = use_power(
        &mut pw,
        world.as_mut().map(|w| &mut w.0),
        &mut player,
        spell,
        stage,
    );
    let cd = pw.cd[spell];
    game.say(&line(&used, spell, cd));
}

#[allow(clippy::too_many_arguments)]
fn run(
    time: Res<Time>,
    mut pw: ResMut<Powers>,
    mut game: ResMut<crate::Game>,
    mut player: ResMut<Player>,
    mut pack: ResMut<crate::Pack>,
    mut boost: ResMut<Boost>,
    prog: Res<crate::Progress>,
    working: Res<crate::Working>,
) {
    let st = stages(&prog, &working);
    boost.0 = st.map(|s| scale(s).range);
    // A cast this frame: a Practised or Mastered spell recharges faster.
    if game.casts != pw.last_casts {
        if game.cooldown > 0.0 {
            let k = scale(st[game.spell.min(3)]).cooldown;
            game.cooldown -= crate::COOLDOWN * (1.0 - k);
        }
        pw.last_casts = game.casts;
    }
    if game.paused {
        return;
    }
    let dt = time.delta_secs().min(0.1);
    for c in &mut pw.cd {
        *c -= dt;
    }
    pw.flare -= dt;
    let mut m = pw.motion;
    m.after_move(&mut player, dt);
    pw.motion = m;
    // Caches.
    let mut got = None;
    for (k, b) in pw.built.iter().enumerate() {
        let i = SITES.iter().position(|s| s.id == b.site.id).unwrap_or(k);
        if !pw.found[i] && touches(&player, b.orb) {
            got = Some((i, b.site.name));
        }
    }
    let mut line = String::new();
    if let Some((i, name)) = got {
        pw.found[i] = true;
        pack.0.add(Item::Block(mat::CRYSTAL), REWARD);
        line = format!("{name} found! {REWARD} crystals and a rank point.");
        if i == 0 {
            pw.guide = 0.0;
        }
    }
    let up = pw.rerank(st, prog.0.rev(), working.practised);
    if up {
        line = format!("{line} Rank up: {}!", RANKS[pw.rank].0)
            .trim()
            .to_string();
    }
    if !line.is_empty() {
        game.say(&line);
    }
    // The guided first lift.
    pw.guide -= dt;
    let hint = match pw.built.first().filter(|_| !pw.found[0]) {
        Some(b) => {
            let d = Vec2::new(player.pos.x - b.orb.x, player.pos.z - b.orb.z).length();
            guide_line(
                st[1],
                game.spell == 1,
                d,
                pw.motion.floating,
                pw.guide > 0.0,
            )
        }
        None => String::new(),
    };
    if pw.hint != hint {
        pw.hint = hint;
    }
}

fn draw(
    time: Res<Time>,
    pw: Res<Powers>,
    player: Res<Player>,
    mut orbs: Query<(&Orb, &mut Transform, &mut Visibility, Has<Beam>), Without<FlareLight>>,
    mut flare: Query<(&mut PointLight, &mut Transform), With<FlareLight>>,
) {
    let t = time.elapsed_secs();
    for (o, mut tr, mut v, beam) in &mut orbs {
        let want = if pw.found[o.0] {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if *v != want {
            *v = want;
        }
        if beam {
            continue;
        }
        if let Some(b) = pw
            .built
            .iter()
            .find(|b| SITES.iter().position(|s| s.id == b.site.id) == Some(o.0))
        {
            tr.translation = b.orb + Vec3::Y * (0.12 * (t * 2.0 + o.0 as f32).sin());
            tr.rotation = Quat::from_rotation_y(t * 1.2);
        }
    }
    if let Ok((mut l, mut tr)) = flare.single_mut() {
        let on = pw.flare > 0.0;
        // Fades over its last two seconds.
        let k = (pw.flare / 2.0).clamp(0.0, 1.0);
        let want = if on { 120_000.0 * k } else { 0.0 };
        if (l.intensity - want).abs() > 1.0 {
            l.intensity = want;
        }
        tr.translation = player.eye() + Vec3::Y * 0.6;
    }
}

fn publish(pw: Res<Powers>, game: Res<crate::Game>, mut last: Local<String>) {
    let json = pw.json(game.spell.min(3));
    if *last != json {
        write_state(&json);
        *last = json;
    }
}

#[cfg(target_arch = "wasm32")]
fn drain() -> Vec<String> {
    use wasm_bindgen::JsValue;
    use web_sys::js_sys::{Array, Reflect};
    let Some(win) = web_sys::window() else {
        return Vec::new();
    };
    let get =
        |o: &JsValue, k: &str| Reflect::get(o, &JsValue::from_str(k)).unwrap_or(JsValue::UNDEFINED);
    let gi = get(&win.into(), "__gameIn");
    let q = get(&gi, "powers");
    if !Array::is_array(&q) {
        return Vec::new();
    }
    let arr = Array::from(&q);
    if arr.length() == 0 {
        return Vec::new();
    }
    let out = arr.iter().filter_map(|v| v.as_string()).collect();
    let _ = Reflect::set(&gi, &"powers".into(), &Array::new());
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn drain() -> Vec<String> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
fn write_state(json: &str) {
    use wasm_bindgen::JsValue;
    if let Some(win) = web_sys::window() {
        let _ =
            web_sys::js_sys::Reflect::set(&win, &"__powersState".into(), &JsValue::from_str(json));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_state(_json: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::MoveInput;
    use crate::voxel_view::{walk, Walk};
    use realgame_voxel::{gen, stamp as vstamp, DEFAULT_SEED};
    use std::sync::OnceLock;

    const DT: f32 = 1.0 / 60.0;

    /// The generated world with the caches stamped (built once).
    fn world() -> &'static (World, Vec<Built>) {
        static W: OnceLock<(World, Vec<Built>)> = OnceLock::new();
        W.get_or_init(|| {
            let mut w = gen::generate(DEFAULT_SEED);
            let b = SITES.iter().map(|s| stamp(&mut w, *s)).collect();
            (w, b)
        })
    }

    fn built(id: &str) -> Built {
        *world().1.iter().find(|b| b.site.id == id).unwrap()
    }

    /// A player standing at block column (x, z) on height y, facing `yaw`.
    fn at(x: f32, y: i32, z: f32, yaw: f32) -> Player {
        Player {
            pos: Vec3::new(x, y as f32, z),
            yaw,
            grounded: true,
            ..Player::default()
        }
    }

    /// Runs the real walk for `secs` with W (and Shift) held; jumps on the
    /// first frame when `jump`. Powers' after-move runs every frame.
    fn go(w: &World, p: &mut Player, m: &mut Motion, sprint: bool, jump: bool, secs: f32) {
        let mut walk_state = Walk::default();
        for f in 0..(secs / DT) as usize {
            walk(
                p,
                &mut walk_state,
                w,
                MoveInput {
                    wish: Vec2::new(0.0, 1.0),
                    sprint,
                    jump: jump && f < 20,
                },
                DT,
            );
            m.after_move(p, DT);
        }
    }

    /// Settles a player on the ground (a few frames of standing still).
    fn settle(w: &World, p: &mut Player) {
        let mut ws = Walk::default();
        for _ in 0..30 {
            walk(p, &mut ws, w, MoveInput::default(), DT);
        }
    }

    fn on_top(p: &Player, b: &Built, half: i32, y: i32) -> bool {
        let c = b.site.at;
        let (dx, dz) = (p.pos.x - c[0] as f32 - 0.5, p.pos.z - c[1] as f32 - 0.5);
        dx.abs() < half as f32 + 0.9 && dz.abs() < half as f32 + 0.9 && p.pos.y >= y as f32 - 0.05
    }

    #[test]
    fn ranks_rise_with_stages_and_caches() {
        use Stage::*;
        assert_eq!(rank_of(points(&[Unknown; 4], 0)), 0);
        assert_eq!(
            RANKS[rank_of(points(&[Learning, Learning, Unknown, Unknown], 0))].0,
            "Kindler"
        );
        assert_eq!(RANKS[rank_of(points(&[Learning; 4], 1))].0, "Catalyst");
        assert_eq!(RANKS[rank_of(points(&[Practised; 4], 1))].0, "Synthesist");
        assert_eq!(RANKS[rank_of(points(&[Mastered; 4], 4))].0, "Archon");
        // Mastering every spell alone does not make an Archon: it needs a cache.
        assert_eq!(RANKS[rank_of(points(&[Mastered; 4], 0))].0, "Synthesist");
    }

    #[test]
    fn a_load_is_not_a_rank_up_but_learning_is() {
        use Stage::*;
        let mut pw = Powers::default();
        assert!(!pw.rerank([Unknown; 4], 0, [false; 4]));
        // The save loads Practised spells (rev unchanged): silent.
        assert!(!pw.rerank([Practised, Practised, Practised, Mastered], 0, [false; 4]));
        assert_eq!(RANKS[pw.rank].0, "Synthesist");
        // A new cache is a rank-up moment when it crosses a tier.
        pw.found = [false, true, true, true, true];
        assert!(pw.rerank([Practised, Practised, Practised, Mastered], 0, [false; 4]));
        assert_eq!((RANKS[pw.rank].0, pw.rank_ups), ("Archon", 1));
        let mut pw = Powers::default();
        pw.rerank([Unknown; 4], 3, [false; 4]);
        assert!(pw.rerank([Learning, Learning, Unknown, Unknown], 5, [false; 4]));
        // Caches restored from the device's save (the `found` command) are silent.
        let mut pw = Powers::default();
        pw.rerank([Learning; 4], 0, [false; 4]);
        pw.found = [false, true, true, true, false];
        pw.seen = None;
        assert!(!pw.rerank([Learning; 4], 0, [false; 4]));
        assert_eq!((RANKS[pw.rank].0, pw.rank_ups), ("Catalyst", 0));
    }

    #[test]
    fn learning_water_in_play_starts_the_guided_lift() {
        use Stage::*;
        let mut pw = Powers::default();
        pw.rerank([Unknown; 4], 0, [false; 4]);
        assert!(pw.rerank([Unknown, Learning, Unknown, Unknown], 1, [false; 4]) == false);
        assert_eq!(pw.guide, GUIDE_S);
        // Each step of the moment says what to do next.
        assert!(guide_line(Learning, false, 12.0, false, true).contains("Press 2"));
        assert!(guide_line(Learning, true, 12.0, false, true)
            .contains("Walk to the glowing tower (12 m)"));
        assert!(guide_line(Learning, true, 2.0, false, true).contains("Press F"));
        assert!(guide_line(Learning, true, 1.0, true, true).contains("Hold W"));
        assert_eq!(guide_line(Learning, true, 2.0, false, false), "");
        // Before learning: a tease next to the tower only.
        assert!(guide_line(Unknown, false, 3.0, false, false).contains("Formula Bench"));
        assert_eq!(guide_line(Unknown, false, 30.0, false, false), "");
        // Water already known from the save: no guide.
        let mut pw = Powers::default();
        pw.rerank([Unknown; 4], 0, [false; 4]);
        pw.rerank([Unknown, Learning, Unknown, Unknown], 0, [false; 4]);
        assert_eq!(pw.guide, 0.0);
        // Learning Water and taking the tower's cache is a rank-up to Kindler.
        let mut pw = Powers::default();
        pw.rerank([Unknown; 4], 0, [false; 4]);
        pw.rerank([Unknown, Learning, Unknown, Unknown], 1, [false; 4]);
        pw.found[0] = true;
        assert!(pw.rerank([Unknown, Learning, Unknown, Unknown], 1, [false; 4]));
        assert_eq!(RANKS[pw.rank].0, "Kindler");
    }

    #[test]
    fn practice_makes_spells_stronger() {
        let (l, p, m) = (
            scale(Stage::Learning),
            scale(Stage::Practised),
            scale(Stage::Mastered),
        );
        for (a, b) in [(l, p), (p, m)] {
            assert!(b.range > a.range && b.cooldown < a.cooldown && b.power_cd < a.power_cd);
            assert!(b.lift > a.lift && b.dash > a.dash && b.walk_len > a.walk_len);
            assert!(b.flare > a.flare);
        }
    }

    #[test]
    fn an_unknown_spell_has_a_locked_power() {
        let mut pw = Powers::default();
        let mut p = at(0.5, 40, 0.5, 0.0);
        let u = use_power(&mut pw, None, &mut p, 1, Stage::Unknown);
        assert_eq!(u, Use::Locked);
        assert!(line(&u, 1, 0.0).contains("Formula Bench"));
        assert_eq!(p.vel, Vec3::ZERO);
        // Learned: it works, then recharges.
        assert_eq!(
            use_power(&mut pw, None, &mut p, 1, Stage::Learning),
            Use::Lift
        );
        assert_eq!(
            use_power(&mut pw, None, &mut p, 1, Stage::Learning),
            Use::Cooling
        );
        assert_eq!(pw.cd[1], 3.0);
        // Each power recharges on its own.
        assert_eq!(
            use_power(&mut pw, None, &mut p, 3, Stage::Learning),
            Use::NotGrounded
        );
        assert_eq!(pw.cd[3], 0.0);
    }

    #[test]
    fn caches_sit_in_their_areas_and_clear_of_the_story() {
        let (w, built) = world();
        let fresh = gen::generate(DEFAULT_SEED);
        let biomes = ["Village", "Village", "Marsh", "Frozen", "Marsh"];
        for (b, want) in built.iter().zip(biomes) {
            let c = b.site.at;
            assert_eq!(
                format!("{:?}", gen::biome_at([c[0], b.h, c[1]])),
                want,
                "{}",
                b.site.id
            );
            let (r, ramp) = pad(b.site.id);
            let e = r + ramp;
            let area = Aabb {
                min: [c[0] - e, 0, c[1] - e],
                max: [c[0] + e + 1, 96, c[1] + e + 1],
            };
            for s in vstamp::ALL {
                let o = (0..3).all(|a| s.min[a] < area.max[a] && area.min[a] < s.max[a]);
                assert!(!o, "{} overlaps stamp {}", b.site.id, s.id);
            }
            for dz in -e..=e {
                for dx in -e..=e {
                    let p = [c[0] + dx, b.h, c[1] + dz];
                    assert!(
                        fresh.can_place(p),
                        "{} pad touches a protected place at {p:?}",
                        b.site.id
                    );
                }
            }
            // The cache itself cannot be dug into or built over.
            assert!(!w.can_break([c[0], b.h - 1, c[1]]) && !w.can_place([c[0], b.h + 1, c[1]]));
        }
    }

    #[test]
    fn the_ember_vault_opens_only_when_its_thicket_burns() {
        let (w, _) = world();
        let b = built("vault");
        let c = b.site.at;
        let door = [c[0], b.h, c[1] + VAULT];
        assert_eq!(w.get(door).mat, mat::LEAVES);
        assert!(!w.can_break(door), "the thicket cannot be dug by hand");
        // Walking north into the door: stopped outside.
        let mut p = at(c[0] as f32 + 0.5, b.h, (c[1] + VAULT + 3) as f32 + 0.5, 0.0);
        let mut m = Motion::default();
        settle(w, &mut p);
        go(w, &mut p, &mut m, false, false, 3.0);
        assert!(!touches(&p, b.orb), "blocked by the thicket");
        // Burnt (leaves burn to nothing): walk in and reach the orb.
        let mut w2 = w.clone();
        for y in b.h..b.h + 2 {
            w2.set([door[0], y, door[2]], Cell::AIR);
        }
        let mut p = at(c[0] as f32 + 0.5, b.h, (c[1] + VAULT + 3) as f32 + 0.5, 0.0);
        let mut reached = false;
        let mut ws = Walk::default();
        for _ in 0..240 {
            walk(
                &mut p,
                &mut ws,
                &w2,
                MoveInput {
                    wish: Vec2::Y,
                    ..default()
                },
                DT,
            );
            reached |= touches(&p, b.orb);
        }
        assert!(reached, "the orb is inside");
    }

    #[test]
    fn the_sky_pillar_needs_the_lift() {
        lift_onto("pillar", PILLAR_H);
    }

    #[test]
    fn the_plaza_tower_needs_the_lift() {
        lift_onto("ledge", TOWER_H);
    }

    fn lift_onto(id: &str, height: i32) {
        let (w, _) = world();
        let b = built(id);
        let c = b.site.at;
        let top = b.h + height;
        // Standing against the pillar's south face, facing it (north).
        let start = || {
            at(
                c[0] as f32 + 0.5,
                b.h,
                (c[1] + PILLAR + 1) as f32 + 0.45,
                0.0,
            )
        };
        let mut p = start();
        let mut m = Motion::default();
        go(w, &mut p, &mut m, true, true, 3.0);
        assert!(
            !on_top(&p, &b, PILLAR, top),
            "a jump does not reach the top: {:?}",
            p.pos
        );
        for st in [Stage::Learning, Stage::Mastered] {
            let mut p = start();
            let mut m = Motion::default();
            assert!(m.lift(&mut p, scale(st).lift));
            // Hold W until over the middle of the top, then let go.
            let mut ws = Walk::default();
            let mut got = false;
            for _ in 0..300 {
                let over = p.pos.z <= c[1] as f32 + 0.6;
                let wish = if over { Vec2::ZERO } else { Vec2::Y };
                walk(&mut p, &mut ws, w, MoveInput { wish, ..default() }, DT);
                m.after_move(&mut p, DT);
                got |= touches(&p, b.orb);
            }
            assert!(
                on_top(&p, &b, PILLAR, top),
                "{st:?} lift lands on top: {:?}",
                p.pos
            );
            assert!(got, "{st:?}: the orb is taken on the way");
        }
    }

    #[test]
    fn the_ravine_needs_the_dash() {
        let (w, _) = world();
        let b = built("ravine");
        let c = b.site.at;
        // On the north rim, facing south (+z: yaw pi).
        let start = || {
            at(
                c[0] as f32 + 0.5,
                b.h,
                (c[1] - PIT - 1) as f32 + 0.7,
                std::f32::consts::PI,
            )
        };
        let mut p = start();
        let mut m = Motion::default();
        settle(w, &mut p);
        // A run-up: sprint from further back and jump at the edge.
        p.pos.z -= 6.0;
        let mut ws = Walk::default();
        let mut jumped = false;
        for _ in 0..400 {
            let edge = p.pos.z >= (c[1] - PIT) as f32 - 0.4;
            let jump = edge && !jumped && p.grounded;
            jumped |= jump;
            walk(
                &mut p,
                &mut ws,
                w,
                MoveInput {
                    wish: Vec2::Y,
                    sprint: true,
                    jump,
                },
                DT,
            );
        }
        assert!(
            jumped && !on_top(&p, &b, PLATFORM, b.h),
            "a sprint jump falls short: {:?}",
            p.pos
        );
        // Lift from the rim drifts too slowly to cross.
        let mut p = start();
        settle(w, &mut p);
        let mut m2 = Motion::default();
        assert!(m2.lift(&mut p, scale(Stage::Mastered).lift));
        go(w, &mut p, &mut m2, true, false, 6.0);
        assert!(
            !on_top(&p, &b, PLATFORM, b.h),
            "a lift cannot cross: {:?}",
            p.pos
        );
        // The dash at every stage lands on the platform (then W stops on it).
        for st in [Stage::Learning, Stage::Practised, Stage::Mastered] {
            let mut p = start();
            settle(w, &mut p);
            assert!(m.dash(&mut p, scale(st).dash));
            let mut ws = Walk::default();
            let mut landed = false;
            for _ in 0..120 {
                walk(&mut p, &mut ws, w, MoveInput::default(), DT);
                landed |= p.grounded && on_top(&p, &b, PLATFORM, b.h);
                if p.grounded {
                    break;
                }
            }
            assert!(landed, "{st:?} dash lands on the platform: {:?}", p.pos);
        }
    }

    #[test]
    fn the_frost_isle_needs_the_walkway() {
        let (w, _) = world();
        let b = built("isle");
        let c = b.site.at;
        let start = || at(c[0] as f32 + 0.5, b.h, (c[1] + MOAT + 1) as f32 + 0.4, 0.0);
        // Deep water: two blocks.
        assert_eq!(w.get([c[0], b.h - 1, c[1] + MOAT]).mat, mat::WATER);
        assert_eq!(w.get([c[0], b.h - 2, c[1] + MOAT]).mat, mat::WATER);
        // The Mastered dash falls short of the island.
        let mut p = start();
        settle(w, &mut p);
        let mut m = Motion::default();
        assert!(m.dash(&mut p, scale(Stage::Mastered).dash));
        let mut ws = Walk::default();
        for _ in 0..120 {
            walk(&mut p, &mut ws, w, MoveInput::default(), DT);
            if p.grounded {
                break;
            }
        }
        assert!(
            !on_top(&p, &b, ISLE, b.h),
            "the dash cannot cross the moat: {:?}",
            p.pos
        );
        // A Learning walkway reaches the island; walking it takes the orb.
        let mut w2 = w.clone();
        let mut p = start();
        settle(&w2, &mut p);
        let cells = freeze_walkway(&mut w2, p.pos, p.yaw, scale(Stage::Learning).walk_len, 0);
        assert_eq!(
            cells.len() as i32,
            MOAT - ISLE,
            "one ice block per metre of moat"
        );
        assert!(
            cells.iter().all(|c| c[1] == b.h - 1),
            "only the top layer freezes"
        );
        let mut ws = Walk::default();
        let mut reached = false;
        for _ in 0..600 {
            walk(
                &mut p,
                &mut ws,
                &w2,
                MoveInput {
                    wish: Vec2::Y,
                    ..default()
                },
                DT,
            );
            reached |= touches(&p, b.orb);
        }
        assert!(reached, "walked the ice to the orb: {:?}", p.pos);
        // Away from water nothing freezes.
        let mut p = start();
        p.yaw = std::f32::consts::PI;
        assert!(freeze_walkway(&mut w2, p.pos, p.yaw, 15.5, 1).is_empty());
    }

    #[test]
    fn walkway_ice_stays_frozen() {
        let (w, _) = world();
        let b = built("isle");
        let c = b.site.at;
        let mut w2 = w.clone();
        let p = at(c[0] as f32 + 0.5, b.h, (c[1] + MOAT + 1) as f32 + 0.4, 0.0);
        let cells = freeze_walkway(&mut w2, p.pos, p.yaw, 15.5, 0);
        let mut sim = realgame_matter::World::new(DEFAULT_SEED);
        for q in &cells {
            sim.touch(&w2, *q);
        }
        let mut ev = Vec::new();
        for _ in 0..300 {
            sim.step(&mut w2, &mut ev);
        }
        let ice = cells.iter().filter(|q| w2.get(**q).mat == mat::ICE).count();
        assert_eq!(ice, cells.len(), "30 s later the walkway is still ice");
    }
}
