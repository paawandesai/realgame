//! Spell physics in the block world (I-051 ported to I-052): Water and CO2
//! charge while the button is held and fly a gravity arc on release (with a
//! dotted preview of the arc while charging); Fire and Frost are short cones.
//! The landing is found by stepping the arc through the voxel cells, and a
//! landed spell goes out as a [`SpellLanded`] message, which Lane B's
//! `blockchem::on_impact` turns into a matter emit. Registered only in
//! `Mode::Voxel`; the classic courtyard's cast path is not touched.

use crate::ballistics;
use realgame_voxel::{mat, World};

/// Where a spell came to rest in the block world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// The block it hit (solid or water).
    pub cell: [i32; 3],
    /// The face it came in through (the cell it was in just before).
    pub normal: [i8; 3],
    /// The point where it arrived.
    pub pos: [f32; 3],
    /// Flight time, seconds.
    pub t: f32,
}

/// Whether a flying spell stops at a block of material `m` (solid blocks
/// and water; air, steam and torches let it pass). Contract stub.
pub fn stops(m: u8) -> bool {
    let _ = m;
    false
}

/// Traces the arc from `hand` with velocity `v0` through `world` (steps of
/// `dt`, at most `max_t`) and returns where it lands, if it does. Stub.
pub fn land(world: &World, hand: [f32; 3], v0: [f32; 3], dt: f32, max_t: f32) -> Option<Landing> {
    let _ = (world, hand, v0, dt, max_t, ballistics::G, mat::AIR);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::Cell;

    /// A stone floor at y = 9 (top face at y = 10) over x, z in 0..40.
    fn floor() -> World {
        let mut w = World::new();
        for x in 0..40 {
            for z in 0..40 {
                w.set([x, 9, z], Cell::new(mat::STONE));
            }
        }
        w
    }

    fn launch(speed: f32, deg: f32) -> [f32; 3] {
        let a = deg.to_radians();
        [0.0, speed * a.sin(), -speed * a.cos()]
    }

    #[test]
    fn i051v_an_arc_lands_on_the_ground_where_the_maths_says() {
        let w = floor();
        let hand = [5.5, 11.45, 30.5];
        let v0 = launch(ballistics::launch_speed(6.5), 40.0);
        let l = land(&w, hand, v0, 1.0 / 60.0, 5.0).expect("lands");
        assert_eq!(l.cell[1], 9, "on the floor: {l:?}");
        assert_eq!(l.normal, [0, 1, 0], "from above");
        assert!((l.pos[1] - 10.0).abs() < 0.12, "{l:?}");
        // Same as the flat-ground maths from 1.45 m above the floor.
        let p = bevy::prelude::Vec3::from(hand);
        let v = bevy::prelude::Vec3::from(v0);
        let t = ballistics::landing_time(p, v, 10.0).unwrap();
        let want = ballistics::at(p, v, t);
        assert!((l.pos[2] - want.z).abs() < 0.15, "z {} vs {}", l.pos[2], want.z);
        assert!((l.t - t).abs() < 0.05);
    }

    #[test]
    fn i051v_a_wall_in_the_way_stops_the_arc_on_its_face() {
        let mut w = floor();
        // A wall 3 blocks ahead (z = 27), 4 blocks tall.
        for x in 0..12 {
            for y in 10..14 {
                w.set([x, y, 27], Cell::new(mat::STONE));
            }
        }
        let hand = [5.5, 11.45, 30.5];
        let v0 = launch(ballistics::launch_speed(6.5), 20.0);
        let l = land(&w, hand, v0, 1.0 / 60.0, 5.0).expect("lands");
        assert_eq!(l.cell[2], 27, "on the wall: {l:?}");
        assert_eq!(l.normal, [0, 0, 1], "on the face toward the player");
        assert!(l.pos[1] > 10.0, "above the floor: {l:?}");
    }

    #[test]
    fn i051v_an_arc_passes_over_a_gap_and_lands_beyond_it() {
        let mut w = floor();
        // A trench 2 blocks wide, 3 deep, right under the middle of the arc.
        for x in 0..12 {
            for z in 25..27 {
                for y in 7..10 {
                    w.set([x, y, z], Cell::new(mat::AIR));
                }
            }
        }
        let hand = [5.5, 11.45, 30.5];
        let v0 = launch(ballistics::launch_speed(10.0), 45.0);
        let l = land(&w, hand, v0, 1.0 / 60.0, 5.0).expect("lands");
        assert!(l.pos[2] < 25.0, "beyond the trench: {l:?}");
        assert_eq!(l.cell[1], 9, "on the floor past it: {l:?}");
        // Water stops it; air and steam do not.
        assert!(stops(mat::WATER) && stops(mat::STONE) && stops(mat::ICE));
        assert!(!stops(mat::AIR) && !stops(mat::STEAM));
    }
}
