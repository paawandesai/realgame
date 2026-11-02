//! The workshop fire is a puzzle, never a soft-lock (I-052 powers lane).
//!
//! Two rules keep it fair, whatever the matter sim does with heat:
//! - only the workshop's fuel store can burn: every other flammable block
//!   near the workshop (its wood corner posts, its plank workbench, any
//!   grass or leaves next to it) is made fireproof when the world loads, so
//!   the fire can never spread to the walls or the plaza;
//! - a Water cast that lands near the fire runs to it: the water also lands
//!   on the burning block nearest to where it hit, so every cast puts out
//!   part of the fire and the store goes out in a handful of casts;
//! - the store blocks a Water cast soaks stop being an endless fuel source
//!   (break-2 B1): one block left burning can no longer relight the whole
//!   pile, it only burns its own fuel.

use bevy::prelude::*;
use realgame_matter as matter;
use realgame_voxel::{gen::WORKSHOP_STORE, mat, stamp::WORKSHOP, Cell, World};

use crate::chem::Spell;

/// Blocks around the workshop's box that are made fireproof.
pub const ZONE_PAD: i32 = 4;
/// How far (m) a landed Water cast runs to the nearest burning block.
pub const RUN_TO_FIRE: f32 = 5.0;

fn in_store(p: [i32; 3]) -> bool {
    (0..3).all(|a| p[a] >= WORKSHOP_STORE.0[a] && p[a] < WORKSHOP_STORE.1[a])
}

/// The fireproof stand-in for a flammable block: wood and planks become
/// brick and stone (posts and a stone workbench), plants become dirt or air.
fn fireproof(m: u8) -> Cell {
    match m {
        mat::WOOD => Cell::new(mat::BRICK),
        mat::GRASS => Cell::new(mat::DIRT),
        mat::LEAVES => Cell::AIR,
        _ => Cell::new(mat::STONE),
    }
}

/// Makes every flammable block near the workshop, except the fuel store,
/// fireproof. Returns how many blocks changed.
pub fn fireproof_workshop(w: &mut World) -> usize {
    let props = matter::props::table();
    let mut n = 0;
    for x in WORKSHOP.min[0] - ZONE_PAD..WORKSHOP.max[0] + ZONE_PAD {
        for y in WORKSHOP.min[1] - ZONE_PAD..WORKSHOP.max[1] + ZONE_PAD {
            for z in WORKSHOP.min[2] - ZONE_PAD..WORKSHOP.max[2] + ZONE_PAD {
                let p = [x, y, z];
                let c = w.get(p);
                let burns = props
                    .get(c.mat as usize)
                    .is_some_and(|p| p.ignite_k.is_some());
                if burns && !in_store(p) {
                    w.set(p, fireproof(c.mat));
                    n += 1;
                }
            }
        }
    }
    n
}

/// Where a Water cast that landed at `pos` also lands: the burning block
/// nearest to it, when one is within `RUN_TO_FIRE` and not already inside
/// the cast's own soak radius.
pub fn run_to_fire(sim: &matter::World, pos: Vec3) -> Option<Vec3> {
    sim.burning_cells()
        .map(|c| Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 0.5, c[2] as f32 + 0.5))
        .map(|c| (c.distance(pos), c))
        .filter(|(d, _)| *d <= RUN_TO_FIRE && *d > matter::WATER_SOAK_RADIUS * 0.5)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c)
}

/// A Water cast landed at `pos`: it also lands on the nearest burning
/// block, and when either landing reaches the fuel store the water sheets
/// over the whole store: every burning store block is wetted and the store
/// stops being a `SOURCE`.
pub fn douse_at(sim: &mut matter::World, w: &mut World, pos: Vec3) {
    let mut at = vec![pos];
    at.extend(run_to_fire(sim, pos));
    if let Some(&to) = at.get(1) {
        sim.emit(crate::blockchem::on_impact(Spell::Water, to, IVec3::Y));
    }
    // Water that reaches the pile sheets over it: the back of the store is
    // cooled with the front, goes out and stops being a source (content/
    // science-sources.md, p1-fuel-store), so a soaked block cannot be
    // relit by one the cast missed.
    let r = matter::WATER_SOAK_RADIUS + 0.5;
    let store: Vec<[i32; 3]> = (WORKSHOP_STORE.0[0]..WORKSHOP_STORE.1[0])
        .flat_map(|x| (WORKSHOP_STORE.0[1]..WORKSHOP_STORE.1[1]).map(move |y| (x, y)))
        .flat_map(|(x, y)| (WORKSHOP_STORE.0[2]..WORKSHOP_STORE.1[2]).map(move |z| [x, y, z]))
        .collect();
    let centre = |p: [i32; 3]| Vec3::new(p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5);
    if !store
        .iter()
        .any(|&p| at.iter().any(|a| a.distance(centre(p)) <= r))
    {
        return;
    }
    for p in store {
        let mut cell = w.get(p);
        if cell.has(Cell::BURNING) {
            sim.emit(crate::blockchem::on_impact(
                Spell::Water,
                centre(p),
                IVec3::Y,
            ));
        }
        if cell.has(Cell::SOURCE) {
            cell.set_flag(Cell::SOURCE, false);
            w.set(p, cell);
        }
    }
}

/// A landed Water cast also wets the nearest burning block.
pub(crate) fn douse(
    mut landed: MessageReader<crate::SpellLanded>,
    matter: Option<ResMut<crate::blockchem::Matter>>,
    vox: Option<ResMut<crate::voxel_view::VoxWorld>>,
) {
    let (Some(mut matter), Some(mut vox)) = (matter, vox) else {
        landed.clear();
        return;
    };
    for l in landed.read() {
        if l.spell == Spell::Water && l.normal != [0, 0, 0] {
            douse_at(&mut matter.world, &mut vox.0, l.pos);
        }
    }
}

pub fn plugin(app: &mut App) {
    if let Some(mut w) = app
        .world_mut()
        .get_resource_mut::<crate::voxel_view::VoxWorld>()
    {
        fireproof_workshop(&mut w.0);
    }
    app.add_systems(Update, douse);
}

#[cfg(test)]
mod tests {
    use super::*;
    use realgame_voxel::{gen, DEFAULT_SEED};

    fn world() -> World {
        let mut w = gen::generate(DEFAULT_SEED);
        fireproof_workshop(&mut w);
        w
    }

    fn water(sim: &mut matter::World, w: &mut World, at: Vec3, normal: IVec3) {
        sim.emit(crate::blockchem::on_impact(Spell::Water, at, normal));
        douse_at(sim, w, at);
    }

    #[test]
    fn only_the_fuel_store_ever_burns() {
        let mut w = world();
        let mut sim = matter::World::new(DEFAULT_SEED);
        let mut ev = Vec::new();
        for i in 0..6000 {
            sim.step(&mut w, &mut ev);
            if i % 100 == 0 {
                for c in sim.burning_cells() {
                    assert!(in_store(c), "the fire spread to {c:?} after {i} steps");
                }
            }
        }
        assert!(sim.burning_cells().count() > 0, "the store still burns");
    }

    #[test]
    fn a_handful_of_water_casts_always_put_it_out() {
        let base = world();
        // Wherever a player's casts land near the fire, including break-2
        // B1's hold-0 worked cast (the store's front face at z 190, normal
        // +z, and the front row's top edge), however long it burned first,
        // and however long between casts (a Sprout takes 10 s or more in the
        // builder; the world holds while it is open, but not between casts).
        let spots = [
            (Vec3::new(141.5, 35.0, 192.0), IVec3::Y),
            (Vec3::new(142.5, 37.0, 189.0), IVec3::Y),
            (Vec3::new(142.5, 36.5, 190.0), IVec3::Z),
            (Vec3::new(141.5, 37.0, 189.8), IVec3::Y),
            (Vec3::new(139.0, 35.0, 191.0), IVec3::Y),
            (Vec3::new(145.0, 36.0, 187.0), IVec3::Y),
        ];
        for gap in [10usize, 50, 100] {
            for delay in [0usize, 300, 1500, 6000] {
                for (k, &(spot, normal)) in spots.iter().enumerate() {
                    let at = format!("gap {gap}, delay {delay}, spot {k}");
                    let mut w = base.clone();
                    let mut sim = matter::World::new(DEFAULT_SEED);
                    let mut ev = Vec::new();
                    for _ in 0..delay.max(20) {
                        sim.step(&mut w, &mut ev);
                    }
                    let mut casts = 0;
                    while sim.burning_cells().count() > 0 {
                        assert!(casts < 5, "{at}: still burning after 5 casts");
                        let before = sim.burning_cells().count();
                        water(&mut sim, &mut w, spot, normal);
                        casts += 1;
                        for _ in 0..10 {
                            sim.step(&mut w, &mut ev);
                        }
                        let after = sim.burning_cells().count();
                        assert!(after < before, "{at}: cast {casts} won no ground");
                        for _ in 10..gap {
                            sim.step(&mut w, &mut ev);
                        }
                    }
                    // And it stays out.
                    for _ in 0..600 {
                        sim.step(&mut w, &mut ev);
                    }
                    assert_eq!(sim.burning_cells().count(), 0, "{at}: relit");
                }
            }
        }
    }
}
