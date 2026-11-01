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
#[ignore = "waits on Lane A's generate() stamps (V3)"]
fn generated_m5_water_and_frost_only_flicker_vents() {
    check_water_and_frost_only_flicker_vents(generated_world());
}

#[test]
fn fixture_m7_fire_spreads_through_wood_and_water_puts_it_out() {
    check_fire_spreads_through_wood_and_water_puts_it_out(fixture_world());
}
