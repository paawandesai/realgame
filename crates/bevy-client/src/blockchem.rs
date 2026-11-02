//! Block chemistry glue (I-052, Lane B): runs `realgame_matter` on the voxel
//! world at 10 steps/s, turns a landed spell into one `matter::Emit`
//! (`on_impact`), pushes the player back from deep water (`wade_push`), and
//! draws captions, gas puffs, beads, the rig and the balloon crate.
//! Registered only in `Mode::Voxel`.
//!
//! Round 1 (Monday): `on_impact`, the flight delay queue and the fixed-step
//! driver (`Matter::tick`), which fills this plugin's state keys. The voxel
//! world resource is Lane A's (`voxel_view`), so `main.rs` / `voxel_view`
//! call `Matter::queue` from the cast and `Matter::tick` once per frame with
//! that world; see `docs/loop/items/I-052/lanes/B/round-1.md`.

// Called from Lane A's Mode branches (spec section 9), which land on Tuesday.
#![allow(dead_code)]

use bevy::platform::time::Instant;
use bevy::prelude::*;
use realgame_matter::{self as matter, Emit, EmitKind, DT};
use realgame_voxel as voxel;

use crate::chem::Spell;

/// This plugin's values for the voxel-mode state keys (spec section 8);
/// `publish` formats them. Values, not JSON fragments.
#[derive(Resource, Debug, Clone)]
pub struct Keys {
    /// `activeChunks`: chunks the matter step is working on.
    pub active_chunks: u32,
    /// `matterMs`: CPU time of the last matter step.
    pub matter_ms: f32,
    /// `gasCO2Max`: the largest CO2 fraction in the player's chunk.
    pub gas_co2_max: f32,
    /// `burning`: burning blocks within `VIEW_M` of the player (round 2).
    pub burning: u32,
    /// Burning cells in the workshop's stamp box (quest step 1).
    pub workshop_burning: u32,
    /// `ventsLit`: lit gas vents in the methane hollow (round 2).
    pub vents_lit: u32,
    /// Ice cells left in the ice gate's stamp box (quest step 3). Starts
    /// high so nothing completes before the first count.
    pub gate_ice: u32,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            active_chunks: 0,
            matter_ms: 0.0,
            gas_co2_max: 0.0,
            burning: 0,
            workshop_burning: u32::MAX,
            vents_lit: u32::MAX,
            gate_ice: u32::MAX,
        }
    }
}

/// Cells of material `m` inside a stamp's box.
pub fn count_in(vox: &voxel::World, s: &voxel::stamp::Stamp, m: u8) -> u32 {
    let mut n = 0;
    for z in s.min[2]..s.max[2] {
        for y in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                n += u32::from(vox.get([x, y, z]).mat == m);
            }
        }
    }
    n
}

/// The view distance the `burning` key counts within (spec section 8: six
/// chunks, 192 m), measured along the ground.
pub const VIEW_M: f32 = 192.0;

/// How many of `cells` are within `VIEW_M` of `player` along the ground.
pub fn count_near(cells: impl Iterator<Item = [i32; 3]>, player: Vec3) -> u32 {
    cells
        .filter(|c| {
            let (dx, dz) = (c[0] as f32 + 0.5 - player.x, c[2] as f32 + 0.5 - player.z);
            dx * dx + dz * dz <= VIEW_M * VIEW_M
        })
        .count() as u32
}

/// Burning cells inside the workshop's stamp box (quest step 1).
pub fn workshop_burning(vox: &voxel::World) -> u32 {
    let s = voxel::stamp::WORKSHOP;
    let mut n = 0;
    for z in s.min[2]..s.max[2] {
        for y in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                n += u32::from(vox.get([x, y, z]).has(voxel::Cell::BURNING));
            }
        }
    }
    n
}

/// What each spell puts into the world when it lands (section 5.6).
pub fn emit_kind(spell: Spell) -> EmitKind {
    match spell {
        Spell::Water => EmitKind::Water,
        Spell::Frost => EmitKind::Frost,
        Spell::Fire => EmitKind::Fire,
        Spell::Foam => EmitKind::Co2,
    }
}

/// A landed spell as one matter emit: `hit_pos` is the point on the block
/// face the voxel raycast (or, after I-051, the trajectory) hit, and
/// `hit_normal` that face's outward unit normal.
pub fn on_impact(spell: Spell, hit_pos: Vec3, hit_normal: IVec3) -> Emit {
    Emit {
        pos: hit_pos.to_array(),
        normal: [hit_normal.x, hit_normal.y, hit_normal.z].map(|v| v.clamp(-1, 1) as i8),
        what: emit_kind(spell),
        amount: 1.0,
    }
}

/// The hotbar index `cues::flight_secs` expects for a spell.
fn slot(spell: Spell) -> u8 {
    Spell::ALL.iter().position(|&s| s == spell).unwrap_or(0) as u8
}

/// The matter sim, the spells in flight, and the fixed-step clock.
#[derive(Resource)]
pub struct Matter {
    pub world: matter::World,
    /// Emits waiting for their flight time, with the game time they land.
    in_flight: Vec<(f32, Emit)>,
    /// Game time not yet stepped.
    acc: f32,
}

impl Default for Matter {
    fn default() -> Self {
        Matter {
            world: matter::World::new(voxel::DEFAULT_SEED),
            in_flight: Vec::new(),
            acc: 0.0,
        }
    }
}

impl Matter {
    /// A cast at game time `now` from `from` that hits `hit` on a face with
    /// `normal`: it lands after the same flight delay classic mode uses
    /// (`cues::flight_secs`) until I-051 owns landing.
    pub fn queue(&mut self, now: f32, spell: Spell, from: Vec3, hit: Vec3, normal: IVec3) {
        let t = crate::cues::flight_secs(slot(spell), from.distance(hit));
        self.in_flight
            .push((now + t, on_impact(spell, hit, normal)));
    }

    /// Spells still in the air.
    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// One frame: hands landed emits to the sim, then runs at most one fixed
    /// step of `DT` (section 9: at most one step per frame, so a slow frame
    /// makes the world slower, never the frame longer). Matter events are
    /// appended to `events`; the CPU time of the step goes in the keys.
    pub fn tick(
        &mut self,
        vox: &mut voxel::World,
        now: f32,
        dt: f32,
        player: Vec3,
        keys: &mut Keys,
        events: &mut Vec<matter::MatterEvent>,
    ) -> bool {
        let mut i = 0;
        while i < self.in_flight.len() {
            if self.in_flight[i].0 <= now {
                let (_, e) = self.in_flight.remove(i);
                self.world.emit(e);
            } else {
                i += 1;
            }
        }
        self.acc = (self.acc + dt.max(0.0)).min(DT * 3.0);
        if self.acc < DT {
            return false;
        }
        self.acc -= DT;
        let t0 = Instant::now();
        self.world.step(vox, events);
        keys.matter_ms = t0.elapsed().as_secs_f32() * 1000.0;
        keys.active_chunks = self.world.active_chunks() as u32;
        let p = player.floor().as_ivec3();
        keys.gas_co2_max = self.world.co2_max_in_chunk([p.x, p.y, p.z]);
        keys.burning = count_near(self.world.burning_cells(), player);
        keys.workshop_burning = workshop_burning(vox);
        keys.gate_ice = count_in(vox, &voxel::stamp::ICE_GATE, voxel::mat::ICE);
        let hollow = voxel::stamp::HOLLOW;
        keys.vents_lit = self
            .world
            .lit_vents(vox)
            .filter(|&v| hollow.contains(v))
            .count() as u32;
        true
    }
}

/// The caption when deep water pushes the player back (spec 5.4).
pub const WADE_LINE: &str = "Too deep and icy to wade. Freeze it into a bridge?";

/// Whether the water at `feet` is too deep to wade: the feet are in water
/// with more water above or below them (2 or more cells deep).
pub fn too_deep(w: &voxel::World, feet: Vec3) -> bool {
    let p = feet.floor().as_ivec3();
    let water = |dy: i32| w.get([p.x, p.y + dy, p.z]).mat == voxel::mat::WATER;
    water(0) && (water(1) || water(-1))
}

/// Where the player last stood dry, for `wade_push`.
#[derive(Resource, Default)]
pub struct Wade {
    last_dry: Option<Vec3>,
}

/// Deep water pushes the player back to the last dry place they stood,
/// with the caption (spec 5.4; swimming is out of scope).
fn wade_push(
    vox: Res<crate::voxel_view::VoxWorld>,
    mut player: ResMut<crate::player::Player>,
    mut wade: ResMut<Wade>,
    mut game: ResMut<crate::Game>,
) {
    let feet = player.pos;
    if too_deep(&vox.0, feet) {
        if let Some(back) = wade.last_dry {
            player.pos = back;
            player.vel = Vec3::ZERO;
            if game.caption != WADE_LINE {
                game.say(WADE_LINE);
            }
        }
        return;
    }
    let p = feet.floor().as_ivec3();
    let here = vox.0.get([p.x, p.y, p.z]).mat;
    let below = vox.0.get([p.x, p.y - 1, p.z]).mat;
    if player.grounded
        && here != voxel::mat::WATER
        && below != voxel::mat::WATER
        && voxel::materials::flags(below).solid
    {
        wade.last_dry = Some(feet);
    }
}

/// Registers block chemistry (Voxel mode only). Matter events go out as
/// `crate::MatterEvent` messages.
pub fn plugin(app: &mut App) {
    app.add_plugins(crate::fire_fx::plugin)
        .init_resource::<Keys>()
        .init_resource::<Matter>()
        .init_resource::<Wade>()
        .add_systems(
            Update,
            wade_push.after(crate::control).before(crate::aim_and_cast),
        )
        .add_systems(
            Update,
            drive
                .after(crate::projectiles)
                .before(crate::publish)
                .before(crate::story::on_event),
        );
}

/// Once a frame (integration, round 1): each `SpellLanded` from
/// `projectiles` (which already waited the flight delay) enters the sim as
/// its emit, then at most one matter step runs on the block world. The
/// cells it sets dirty their chunks, which `voxel_view` remeshes on the next
/// frame; its events go out as `crate::MatterEvent`.
pub(crate) fn drive(
    time: Res<Time>,
    player: Res<crate::player::Player>,
    mut vox: ResMut<crate::voxel_view::VoxWorld>,
    mut matter: ResMut<Matter>,
    mut keys: ResMut<Keys>,
    mut landed: MessageReader<crate::SpellLanded>,
    mut out: MessageWriter<crate::MatterEvent>,
) {
    for l in landed.read() {
        // A spell that hit nothing (zero normal) has no face to land on.
        if l.normal == [0, 0, 0] {
            continue;
        }
        let n = IVec3::new(l.normal[0].into(), l.normal[1].into(), l.normal[2].into());
        matter.world.emit(on_impact(l.spell, l.pos, n));
    }
    let now = time.elapsed_secs();
    let dt = crate::clock::frame_dt(time.delta_secs());
    let mut events = Vec::new();
    matter.tick(&mut vox.0, now, dt, player.pos, &mut keys, &mut events);
    for e in events {
        out.write(crate::MatterEvent(e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round 2: `burning` counts burning blocks within the view distance
    /// of the player (not the gas vents, which have `ventsLit`), and the
    /// workshop's count is the burning cells inside its stamp box.
    #[test]
    fn burning_counts_blocks_near_the_player_and_in_the_workshop() {
        let cells = [[10, 35, 10], [100, 35, 10], [250, 35, 250]];
        assert_eq!(
            count_near(cells.into_iter(), Vec3::new(10.0, 35.0, 10.0)),
            2
        );
        assert_eq!(
            count_near(cells.into_iter(), Vec3::new(250.0, 35.0, 250.0)),
            1
        );
        let w = voxel::generate(voxel::DEFAULT_SEED);
        assert!(workshop_burning(&w) >= 6, "the workshop burns at the start");
        let mut empty = voxel::World::new();
        assert_eq!(workshop_burning(&empty), 0);
        let mut c = voxel::Cell::new(voxel::mat::WOOD);
        c.set_flag(voxel::Cell::BURNING, true);
        empty.set([142, 36, 189], c);
        empty.set([20, 36, 20], c);
        assert_eq!(workshop_burning(&empty), 1);
    }

    /// Round 2 (spec 5.4): water two or more cells deep at the feet is too
    /// deep to wade; one cell (a puddle) or ice is fine.
    #[test]
    fn deep_water_is_too_deep_to_wade() {
        let w = voxel::generate(voxel::DEFAULT_SEED);
        assert!(
            too_deep(&w, Vec3::new(128.5, 28.0, 154.5)),
            "the ford's bed"
        );
        assert!(
            too_deep(&w, Vec3::new(128.5, 30.2, 154.5)),
            "the ford's top layer"
        );
        assert!(!too_deep(&w, Vec3::new(128.5, 35.0, 205.5)), "the plaza");
        let mut v = voxel::World::new();
        let mut water = voxel::Cell::new(voxel::mat::WATER);
        water.set_level(8);
        v.set([5, 10, 5], water);
        v.set([5, 9, 5], voxel::Cell::new(voxel::mat::STONE));
        assert!(
            !too_deep(&v, Vec3::new(5.5, 10.0, 5.5)),
            "a puddle one deep"
        );
        v.set([5, 10, 5], voxel::Cell::new(voxel::mat::ICE));
        assert!(!too_deep(&v, Vec3::new(5.5, 11.0, 5.5)), "ice");
    }

    #[test]
    fn each_spell_lands_as_its_emit() {
        let e = on_impact(Spell::Foam, Vec3::new(1.0, 2.0, 3.0), IVec3::Y);
        assert_eq!(e.what, EmitKind::Co2);
        assert_eq!(e.normal, [0, 1, 0]);
        assert_eq!(e.pos, [1.0, 2.0, 3.0]);
        assert_eq!(emit_kind(Spell::Frost), EmitKind::Frost);
        assert_eq!(emit_kind(Spell::Water), EmitKind::Water);
        assert_eq!(emit_kind(Spell::Fire), EmitKind::Fire);
    }
}
