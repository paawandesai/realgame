//! The I-052 puzzle checks (M1, M2, M4, M4b, M5, M7) on hand-built fixtures
//! of the stamps (spec section 4 coordinates), so the matter rules are tested
//! before Lane A's generator exists. `tests/matter.rs` runs the same checks on
//! `voxel::generate` once the stamps land. See `common/mod.rs`.

mod common;

use common::*;

#[test]
fn fixture_m1a_frost_freezes_top_layer_first() {
    check_frost_freezes_top_layer_first(fixture_world());
}

#[test]
fn fixture_m1b_ford_bridges_in_three_casts_per_row() {
    check_ford_bridges_in_three_casts_per_row(fixture_world(), false);
}

#[test]
fn fixture_m2_fire_melts_ice_gate_in_three_casts() {
    check_fire_melts_ice_gate_in_three_casts(fixture_world());
}

#[test]
fn fixture_m4_co2_pools_in_hollow() {
    check_co2_pools_in_hollow(fixture_world());
}

#[test]
fn fixture_m4b_hollow_gap_spills_until_closed_with_stone() {
    check_hollow_gap_spills_until_closed(fixture_world(), close_with_stone);
}

#[test]
fn fixture_m4b_hollow_gap_spills_until_closed_with_an_ice_dam() {
    let mut vox = fixture_world();
    build_hollow(&mut vox, true);
    check_hollow_gap_spills_until_closed(vox, close_with_frost);
}

#[test]
fn generated_m5_water_and_frost_only_flicker_vents() {
    check_water_and_frost_only_flicker_vents(generated_world());
}

#[test]
fn fixture_m7_fire_spreads_through_wood_and_water_puts_it_out() {
    check_fire_spreads_through_wood_and_water_puts_it_out(fixture_world());
}

/// Tuning guard (not in the spec's list): with the gap closed, two CO2 casts
/// are not enough, so the hollow really takes three (spec 5.5, level 3's
/// "fills in three casts").
#[test]
fn fixture_two_co2_casts_leave_a_vent_burning() {
    use realgame_matter::{EmitKind, World};
    use realgame_voxel::{mat, Cell};
    let mut vox = fixture_world();
    vox.set(GAP, Cell::new(mat::STONE));
    let h = hollow(&vox);
    let mut m = World::new(1);
    let mut ev = Vec::new();
    step_n(&mut m, &mut vox, 2, &mut ev);
    for _ in 0..2 {
        emit(&mut m, h.hit, [0, 1, 0], EmitKind::Co2);
        step_n(&mut m, &mut vox, steps(0.5), &mut ev);
    }
    step_n(&mut m, &mut vox, steps(4.0), &mut ev);
    let lit = h
        .vents
        .iter()
        .filter(|&&v| vox.get(v).has(Cell::BURNING))
        .count();
    assert!(lit >= 1, "two casts leave at least one vent burning");
}

/// Round 2 (P1): a fuel store, a flammable block flagged `SOURCE` (a stack
/// of charcoal and logs, content/science-sources.md "p1-fuel-store"), keeps
/// burning long past one block's burn time, the way the river's `SOURCE`
/// cells keep refilling. Water cools it below ignition and it stays out;
/// Fire lights it again. A plain burning log beside it burns to ash.
#[test]
fn fixture_a_fuel_store_burns_until_water_cools_it() {
    use realgame_matter::{By, EmitKind, MatterEvent, World};
    use realgame_voxel::{mat, Cell};
    let mut vox = realgame_voxel::World::new();
    fill(&mut vox, [0, 0, 0], [24, 11, 24], Cell::new(mat::STONE));
    let store_at = [8, 11, 8];
    let log_at = [16, 11, 16];
    let mut store = Cell::new(mat::WOOD);
    store.set_flag(Cell::BURNING, true);
    store.set_flag(Cell::SOURCE, true);
    vox.set(store_at, store);
    let mut log = Cell::new(mat::WOOD);
    log.set_flag(Cell::BURNING, true);
    vox.set(log_at, log);
    let mut m = World::new(1);
    let mut ev = Vec::new();
    // Three times a log's burn time (wood: 60 s).
    let long = 3 * m.burn_full(mat::WOOD) as usize;
    step_n(&mut m, &mut vox, long, &mut ev);
    let c = vox.get(store_at);
    assert_eq!(
        c.mat,
        mat::WOOD,
        "the store is still wood after {long} steps"
    );
    assert!(
        c.has(Cell::BURNING),
        "the store still burns after {long} steps"
    );
    assert_eq!(
        vox.get(log_at).mat,
        mat::ASH,
        "a plain log burns to ash in its burn time"
    );
    // One Water cast on its top face puts it out, and it stays out.
    emit(&mut m, [8.5, 12.0, 8.5], [0, 1, 0], EmitKind::Water);
    step_n(&mut m, &mut vox, 1, &mut ev);
    assert!(!vox.get(store_at).has(Cell::BURNING), "water put it out");
    assert!(ev.iter().any(|e| matches!(
        e,
        MatterEvent::Extinguished { cell, by: By::Water } if *cell == store_at
    )));
    step_n(&mut m, &mut vox, steps(30.0), &mut ev);
    let c = vox.get(store_at);
    assert!(
        !c.has(Cell::BURNING) && c.mat == mat::WOOD,
        "out for good: below ignition, nothing relights it ({c:?})"
    );
    assert!(m.temp_c(&vox, store_at) < 100.0);
    // Fire lights it again (M7's relight), and it burns on as a store.
    emit(&mut m, [8.5, 12.0, 8.5], [0, 1, 0], EmitKind::Fire);
    step_n(&mut m, &mut vox, steps(2.0), &mut ev);
    assert!(
        vox.get(store_at).has(Cell::BURNING),
        "Fire relights the store"
    );
}

/// Round 2 (P2): the river outside the ford flows (its cells are `SOURCE`,
/// refilled from upstream), and flowing water does not freeze from a Frost
/// cast; the still ford does. So the ford is the one way north, and the
/// gate at its mouth is the one gate (content/science-sources.md,
/// "p2-river-flows").
#[test]
fn fixture_flowing_river_water_does_not_freeze_but_the_still_ford_does() {
    use realgame_matter::{EmitKind, World};
    use realgame_voxel::{mat, Cell};
    let mut vox = realgame_voxel::World::new();
    fill(&mut vox, [0, 0, 0], [40, 27, 20], Cell::new(mat::STONE));
    let mut river = Cell::new(mat::WATER);
    river.set_level(8);
    river.set_flag(Cell::SOURCE, true);
    let mut still = Cell::new(mat::WATER);
    still.set_level(8);
    fill(&mut vox, [2, 27, 2], [12, 30, 18], river);
    fill(&mut vox, [22, 27, 2], [32, 30, 18], still);
    let mut m = World::new(1);
    let mut ev = Vec::new();
    step_n(&mut m, &mut vox, 2, &mut ev);
    for _ in 0..3 {
        emit(&mut m, [7.5, 30.0, 10.5], [0, 1, 0], EmitKind::Frost);
        emit(&mut m, [27.5, 30.0, 10.5], [0, 1, 0], EmitKind::Frost);
        step_n(&mut m, &mut vox, 5, &mut ev);
    }
    step_n(&mut m, &mut vox, steps(2.0), &mut ev);
    assert_eq!(
        vox.get([7, 29, 10]).mat,
        mat::WATER,
        "the flowing river stays liquid"
    );
    assert_eq!(
        vox.get([27, 29, 10]).mat,
        mat::ICE,
        "the still ford freezes"
    );
}
