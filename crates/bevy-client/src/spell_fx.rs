//! Spell effects in the block world (submission P5): what each spell looks
//! like on its way and where it lands, drawn with the shared particle pool
//! (`fx`), so they batch and cost nothing when idle.
//!
//! - **Water**: a trail of drops along the arc, a splash where it lands, and
//!   steam where it meets fire.
//! - **Frost**: cold mist and falling ice shards, a crystal burst on landing.
//! - **Fire**: a trail of flame with embers, sparks on landing, and embers
//!   drifting up from everything that burns.
//! - **CO2**: a cold fog that spreads low and sinks (CO2 is about 1.5 times
//!   as dense as air).
//! - **Every spell**: a short bright flash where it lands.
//!
//! Voxel mode only (registered by `blockchem`).

use bevy::prelude::*;
use realgame_matter::{By, MatterEvent as M};

use crate::blockchem::Matter;
use crate::chem::Spell;
use crate::fx::{Fx, P};
use crate::player::Player;

/// Embers per second from each of the nearest burning cells.
const EMBER_RATE: f32 = 2.5;
/// How many burning cells shed embers (the nearest).
const EMBER_CELLS: usize = 10;
/// A landing this close (m) to a burning cell counts as hitting the fire.
const NEAR_FIRE: f32 = 1.8;

/// One burst: kind, count, speed (m/s), size (m).
pub type Burst = (P, usize, f32, f32);

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, |mut fx: ResMut<Fx>| fx.floors = true)
        .add_systems(
            Update,
            (
                show.after(crate::control).before(crate::aim_and_cast),
                landings.before(crate::blockchem::drive),
                (embers, steam).after(crate::blockchem::drive),
            )
                .before(crate::publish),
        );
}

/// `?debug` only: `fxshow <spell 0-3>` plays that spell's landing burst
/// 4 m in front of the eye, slowed and held for a few seconds, so a review
/// capture can see it at any frame rate.
fn show(mut web: ResMut<crate::bridge::WebIn>, player: Res<Player>, mut fx: ResMut<Fx>) {
    if crate::param("debug").is_none() || web.cmds.is_empty() {
        return;
    }
    let mut keep = Vec::new();
    for c in std::mem::take(&mut web.cmds) {
        // `fxshow <n> [fire]`: `fire` adds the steam of a spell on a fire.
        let mut a = c.split_whitespace();
        let (Some("fxshow"), Some(Ok(n))) = (a.next(), a.next().map(str::parse::<usize>)) else {
            keep.push(c);
            continue;
        };
        let spell = Spell::ALL[n.min(3)];
        let f = player.rotation() * Vec3::NEG_Z;
        let at = player.eye() + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 4.0 - Vec3::Y * 0.8;
        let before = fx.pending.len();
        emit_impact(&mut fx, spell, at, [0, 1, 0], a.next() == Some("fire"));
        for p in &mut fx.pending[before..] {
            p.2 *= 0.25;
            p.4 = 3.0;
        }
    }
    web.cmds = keep;
}

/// The particles a spell leaves along its flight, per frame: kind, the
/// chance of one this frame, size and life (s).
pub fn trail(spell: Spell) -> [(P, f32, f32, f32); 2] {
    match spell {
        Spell::Water => [(P::Water, 0.95, 0.16, 0.5), (P::Water, 0.5, 0.1, 0.35)],
        Spell::Frost => [(P::Frost, 0.8, 0.26, 0.6), (P::Shard, 0.6, 0.1, 0.5)],
        Spell::Fire => [(P::Fire, 0.95, 0.22, 0.35), (P::Ember, 0.6, 0.07, 0.6)],
        Spell::Foam => [(P::Frost, 0.6, 0.14, 0.6), (P::Foam, 0.5, 0.08, 0.4)],
    }
}

/// What a spell leaves where it lands (`fire`: it landed on a fire).
pub fn impact(spell: Spell, fire: bool) -> Vec<Burst> {
    let mut v = match spell {
        Spell::Water => vec![(P::Water, 30, 3.4, 0.32), (P::Foam, 10, 2.2, 0.16)],
        Spell::Frost => vec![(P::Shard, 20, 3.2, 0.16), (P::Frost, 14, 1.6, 0.55)],
        Spell::Fire => vec![
            (P::Fire, 16, 2.4, 0.42),
            (P::Spark, 12, 5.0, 0.1),
            (P::Ember, 12, 1.5, 0.1),
        ],
        Spell::Foam => vec![(P::Fog, 18, 1.8, 0.6), (P::Foam, 10, 2.0, 0.16)],
    };
    if fire && spell != Spell::Fire {
        // Water (or a cold spell) on a fire: a burst of steam.
        v.push((P::Steam, 14, 1.8, 0.5));
    }
    v.push((P::Flash, 1, 0.0, flash_size(spell)));
    v
}

fn flash_size(spell: Spell) -> f32 {
    match spell {
        Spell::Fire => 0.6,
        Spell::Foam => 0.4,
        _ => 0.45,
    }
}

/// Puffy kinds (mist, fog) grow; nearer than this (m) to where the cast left
/// the hand they would fill the screen, so the trail starts them later.
const PUFF_CLEAR: f32 = 2.5;

/// Emits a spell's trail at `pos` for one frame (`from`: where it left the
/// hand).
pub fn emit_trail(fx: &mut Fx, spell: Spell, pos: Vec3, from: Vec3) {
    for (k, chance, size, life) in trail(spell) {
        if matches!(k, P::Frost | P::Fog | P::Steam) && pos.distance(from) < PUFF_CLEAR {
            continue;
        }
        if fx.rand() < chance {
            let v = fx.rand3() * 0.4 + Vec3::Y * if k == P::Ember { 0.6 } else { 0.0 };
            let at = pos + fx.rand3() * 0.08;
            let sz = size * (0.7 + 0.6 * fx.rand());
            fx.emit(k, at, v, sz, life);
        }
    }
}

/// Emits a landing's bursts at `pos`, lifted off the face it hit.
pub fn emit_impact(fx: &mut Fx, spell: Spell, pos: Vec3, normal: [i8; 3], fire: bool) {
    let n = Vec3::new(normal[0] as f32, normal[1] as f32, normal[2] as f32);
    let at = pos + n * 0.15;
    for (k, count, speed, size) in impact(spell, fire) {
        match k {
            P::Flash => fx.emit(P::Flash, at, Vec3::ZERO, size, 0.2),
            P::Fog => {
                // Fog rolls out low along the ground instead of up.
                for _ in 0..count {
                    let mut v = fx.rand3() * speed;
                    v.y = 0.1 * fx.rand();
                    let p = at + Vec3::Y * 0.4 + fx.rand3() * 0.3;
                    let sz = size * (0.7 + 0.6 * fx.rand());
                    let life = 1.6 + fx.rand() * 1.2;
                    fx.emit(P::Fog, p, v, sz, life);
                }
            }
            _ => fx.burst(k, at, count, speed, size),
        }
    }
}

fn near_fire(matter: &Matter, pos: Vec3) -> bool {
    matter.world.burning_cells().any(|c| {
        let centre = Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 0.5, c[2] as f32 + 0.5);
        centre.distance(pos) < NEAR_FIRE
    })
}

fn landings(
    mut landed: MessageReader<crate::SpellLanded>,
    mut fx: ResMut<Fx>,
    matter: Option<Res<Matter>>,
) {
    for l in landed.read() {
        let fire = matter.as_ref().is_some_and(|m| near_fire(m, l.pos));
        emit_impact(&mut fx, l.spell, l.pos, l.normal, fire);
    }
}

/// Embers drift up from the nearest burning cells.
fn embers(
    time: Res<Time>,
    player: Res<Player>,
    matter: Option<Res<Matter>>,
    mut fx: ResMut<Fx>,
    mut acc: Local<f32>,
) {
    let Some(matter) = matter else { return };
    let cells: Vec<[i32; 3]> = matter.world.burning_cells().collect();
    if cells.is_empty() {
        *acc = 0.0;
        return;
    }
    let cells = crate::fire_fx::nearest(cells, player.eye(), EMBER_CELLS);
    *acc += time.delta_secs().min(0.1) * EMBER_RATE * cells.len() as f32;
    while *acc >= 1.0 {
        *acc -= 1.0;
        let i = ((fx.rand() * cells.len() as f32) as usize).min(cells.len() - 1);
        let c = cells[i];
        let at = Vec3::new(
            c[0] as f32 + fx.rand(),
            c[1] as f32 + 1.1,
            c[2] as f32 + fx.rand(),
        );
        let v = Vec3::new(fx.rand() - 0.5, 1.2 + fx.rand(), fx.rand() - 0.5);
        let size = 0.04 + 0.04 * fx.rand();
        let life = 0.9 + 0.8 * fx.rand();
        fx.emit(P::Ember, at, v, size, life);
    }
}

/// A burst of steam particles where water puts a fire out (on top of
/// `fire_fx`'s steam puffs).
fn steam(mut events: MessageReader<crate::MatterEvent>, mut fx: ResMut<Fx>) {
    for e in events.read() {
        if let M::Extinguished {
            cell,
            by: By::Water,
        } = e.0
        {
            let at = Vec3::new(
                cell[0] as f32 + 0.5,
                cell[1] as f32 + 1.0,
                cell[2] as f32 + 0.5,
            );
            fx.burst(P::Steam, at, 6, 1.4, 0.4);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spell_has_its_own_look_and_a_flash() {
        for s in Spell::ALL {
            let v = impact(s, false);
            assert!(v.iter().any(|b| b.0 == P::Flash), "{s:?} flashes");
            assert!(
                v.iter().all(|b| b.0 != P::Steam),
                "{s:?}: no steam without fire"
            );
        }
        assert!(
            impact(Spell::Water, false).iter().any(|b| b.0 == P::Water),
            "a splash"
        );
        assert!(
            impact(Spell::Frost, false).iter().any(|b| b.0 == P::Shard),
            "ice crystals"
        );
        assert!(
            impact(Spell::Fire, false).iter().any(|b| b.0 == P::Ember),
            "embers"
        );
        assert!(
            impact(Spell::Foam, false).iter().any(|b| b.0 == P::Fog),
            "CO2 fog"
        );
        assert!(
            trail(Spell::Water).iter().all(|t| t.0 == P::Water),
            "a trail of drops"
        );
    }

    #[test]
    fn water_on_fire_makes_steam_but_fire_on_fire_does_not() {
        assert!(impact(Spell::Water, true).iter().any(|b| b.0 == P::Steam));
        assert!(!impact(Spell::Fire, true).iter().any(|b| b.0 == P::Steam));
    }

    #[test]
    fn co2_fog_sinks_and_embers_and_steam_rise() {
        use crate::fx::gravity;
        assert!(gravity(P::Fog) < 0.0, "CO2 is denser than air");
        assert!(gravity(P::Ember) > 0.0 && gravity(P::Steam) > 0.0);
        assert!(gravity(P::Shard) < 0.0 && gravity(P::Water) < 0.0);
    }
}
