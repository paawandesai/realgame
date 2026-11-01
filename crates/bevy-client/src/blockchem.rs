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
#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `activeChunks`: chunks the matter step is working on.
    pub active_chunks: u32,
    /// `matterMs`: CPU time of the last matter step.
    pub matter_ms: f32,
    /// `gasCO2Max`: the largest CO2 fraction in the player's chunk.
    pub gas_co2_max: f32,
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
        true
    }
}

/// Registers block chemistry (Voxel mode only). Matter events go out as
/// `crate::MatterEvent` messages.
pub fn plugin(app: &mut App) {
    app.init_resource::<Keys>()
        .init_resource::<Matter>()
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
fn drive(
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
