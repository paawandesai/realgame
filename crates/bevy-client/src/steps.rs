//! Game feel inputs for the page (submission P7): what the player stands on,
//! how far they have walked, and each landing with its speed, so
//! `web/feel.js` can play a footstep per ground type and a landing thud.
//! Published as voxel-mode state keys `stepMat`, `stride`, `lands` and
//! `landV`. Voxel mode only (registered by `blockchem`).

use bevy::prelude::*;
use realgame_voxel::{mat, materials, World};

use crate::player::Player;
use crate::voxel_view::VoxWorld;

/// A fall faster than this (m/s) when it ends counts as a landing.
pub const LAND_MIN_V: f32 = 3.5;

#[derive(Resource, Debug, Clone, Default)]
pub struct Keys {
    /// `stepMat`: the material under the feet ("" in the air).
    pub step_mat: &'static str,
    /// `stride`: metres walked on the ground so far.
    pub stride: f32,
    /// `lands`: landings so far.
    pub lands: u32,
    /// `landV`: the last landing's speed, m/s.
    pub land_v: f32,
    /// `hits`: spells landed so far (for the page's positioned sounds).
    pub hits: u32,
    /// `hitAt`: where the last spell landed, and `hitSpell` which (0-3).
    pub hit_at: [f32; 3],
    pub hit_spell: u8,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Keys>()
        .add_systems(Update, track.before(crate::publish));
}

/// The material the feet at `feet` stand on: the cell just below them
/// (or the one they are in, for a thin layer like snow or a puddle).
pub fn under(world: &World, feet: Vec3) -> u8 {
    let x = feet.x.floor() as i32;
    let z = feet.z.floor() as i32;
    for dy in [-0.15, -0.6, 0.2] {
        let m = world.get([x, (feet.y + dy).floor() as i32, z]).mat;
        if m != mat::AIR {
            return m;
        }
    }
    mat::AIR
}

/// A landing's speed, if this frame ends a fall: the body was in the air
/// moving down at `prev_vy` and is now on the ground.
pub fn landing(was_grounded: bool, grounded: bool, prev_vy: f32) -> Option<f32> {
    (!was_grounded && grounded && -prev_vy >= LAND_MIN_V).then_some(-prev_vy)
}

fn track(
    player: Res<Player>,
    mut landed: MessageReader<crate::SpellLanded>,
    world: Option<Res<VoxWorld>>,
    mut keys: ResMut<Keys>,
    mut last: Local<Option<(bool, f32, Vec3)>>,
) {
    for l in landed.read() {
        keys.hits += 1;
        keys.hit_at = l.pos.to_array();
        keys.hit_spell = crate::chem::Spell::ALL
            .iter()
            .position(|s| *s == l.spell)
            .unwrap_or(0) as u8;
    }
    let Some(world) = world else { return };
    let (was, prev_vy, prev_pos) = last.unwrap_or((player.grounded, 0.0, player.pos));
    if let Some(v) = landing(was, player.grounded, prev_vy) {
        keys.lands += 1;
        keys.land_v = v;
    }
    if player.grounded {
        // A teleport is not a walk.
        let d = (player.pos - prev_pos).xz().length();
        if d < 2.0 {
            keys.stride += d;
        }
        let m = under(&world.0, player.pos);
        keys.step_mat = if m == mat::AIR {
            ""
        } else {
            materials::name(m)
        };
    } else {
        keys.step_mat = "";
    }
    *last = Some((player.grounded, player.vel.y, player.pos));
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::Cell;

    #[test]
    fn the_ground_under_the_feet_names_the_footstep() {
        let mut w = World::new();
        w.set([4, 9, 4], Cell::new(mat::STONE));
        assert_eq!(under(&w, Vec3::new(4.5, 10.0, 4.5)), mat::STONE);
        w.set([4, 10, 4], Cell::new(mat::SNOW));
        assert_eq!(
            under(&w, Vec3::new(4.5, 10.3, 4.5)),
            mat::SNOW,
            "a snow layer"
        );
        assert_eq!(under(&w, Vec3::new(8.5, 10.0, 8.5)), mat::AIR);
        assert_eq!(materials::name(mat::GRASS), "grass");
    }

    #[test]
    fn only_a_real_fall_counts_as_a_landing() {
        assert_eq!(landing(false, true, -6.0), Some(6.0));
        assert_eq!(landing(false, true, -1.0), None, "a step down");
        assert_eq!(landing(true, true, -6.0), None, "already down");
        assert_eq!(landing(false, false, -6.0), None, "still falling");
    }
}
