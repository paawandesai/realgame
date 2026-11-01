//! I-052 acceptance tests for the matter crate (spec section 11, M1-M11),
//! plus the contract-step checks that pin the shared API (section 10).
//!
//! Tests marked `#[ignore]` have their body written by Lane B (spec section
//! 12). Each ignored body is `todo!()`, so taking the `#[ignore]` off without
//! writing the body fails.

use realgame_matter::{lift_n, By, Emit, EmitKind, Gas, MatterEvent, World, DT, FLICKER_S};
use realgame_voxel as voxel;

// ---------------------------------------------------------------------------
// Contract step (section 10, item 4): green from the first merge.
// ---------------------------------------------------------------------------

#[test]
fn step_constants_match_the_spec() {
    assert_eq!(DT, 0.1, "10 steps per second");
    assert_eq!(FLICKER_S, 1.5, "chem.rs FLICKER_S");
}

#[test]
fn an_idle_step_on_an_empty_world_is_quiet_and_repeatable() {
    let mut vox = voxel::World::new();
    let mut m = World::new(0x52_47_31);
    let mut events: Vec<MatterEvent> = Vec::new();
    let h0 = m.hash(&vox);
    for _ in 0..10 {
        m.step(&mut vox, &mut events);
    }
    assert!(events.is_empty(), "nothing happens in an all-air world");
    assert!(
        vox.dirty_chunks().is_empty(),
        "an idle step writes no cells"
    );
    let again = World::new(0x52_47_31);
    assert_eq!(again.hash(&voxel::World::new()), h0, "same seed, same hash");
}

#[test]
fn the_emit_and_event_shapes_are_the_contract() {
    // Built by blockchem::on_impact (Lane B) and consumed by story (Lane C).
    let e = Emit {
        pos: [128.5, 35.0, 205.5],
        normal: [0, 1, 0],
        what: EmitKind::Frost,
        amount: 1.0,
    };
    let mut m = World::new(1);
    m.emit(e);
    for k in [
        EmitKind::Water,
        EmitKind::Frost,
        EmitKind::Fire,
        EmitKind::Co2,
    ] {
        let _ = Emit { what: k, ..e };
    }
    let cell = [1, 2, 3];
    let all = [
        MatterEvent::Melted { cell },
        MatterEvent::Froze { cell },
        MatterEvent::Ignited { cell },
        MatterEvent::Extinguished {
            cell,
            by: By::Water,
        },
        MatterEvent::Extinguished {
            cell,
            by: By::Frost,
        },
        MatterEvent::Extinguished {
            cell,
            by: By::NoOxygen,
        },
        MatterEvent::Smothered { vent: cell },
        MatterEvent::Relit { vent: cell },
        MatterEvent::Glass { cell },
        MatterEvent::Iron { cell },
        MatterEvent::Leidenfrost {
            pos: [0.5, 1.0, 2.5],
        },
        MatterEvent::Flowed { chunk: 7 },
    ];
    assert_eq!(all.len(), 12);
    let _ = lift_n(Gas::H2, 2.0, 1.5);
}

// ---------------------------------------------------------------------------
// Acceptance (section 11).
// ---------------------------------------------------------------------------

/// M1a: on the generated world, Frost freezes the top layer of the ford first.
#[test]
#[ignore = "Lane B (Mon): body with heat and phase"]
fn frost_freezes_top_layer_first() {
    todo!("M1 frost_freezes_top_layer_first")
}

/// M1b: 3 Frost emits along one row of the ford make a walkable line of ice
/// (`solid_boxes` cover the path from bank to bank for a 0.7 m wide body).
#[test]
#[ignore = "Lane B (Mon): body with heat and phase"]
fn ford_bridges_in_three_casts_per_row() {
    todo!("M1 ford_bridges_in_three_casts_per_row")
}

/// M2: the gate stamp's ice has no ice left after 3 Fire emits and 5 s of
/// steps; the meltwater flows downhill (some water cell is lower than the
/// gate's base after 5 s).
#[test]
#[ignore = "Lane B (Mon): body with heat, phase and water"]
fn fire_melts_ice_gate_in_three_casts() {
    todo!("M2")
}

/// M3: in a closed basin with no sources, the sum of levels is the same after
/// 500 steps; water never moves into a solid cell or upward.
#[test]
#[ignore = "Lane B (Sun night skeleton, Mon body): water flow"]
fn water_conserves_volume() {
    todo!("M3")
}

/// M4: 3 CO2 emits above the methane hollow give O2 < 15 % in every vent's gas
/// cell within 5 s, all 6 vents report `Smothered`, and the hollow's CO2 does
/// not climb over its rim (the gas cells just above the rim stay below 5 %).
#[test]
#[ignore = "Lane B (Mon): body with the gas field"]
fn co2_pools_in_hollow() {
    todo!("M4")
}

/// M4b: with the rim gap open, 3 CO2 emits leave at least 1 vent lit after 5 s
/// and CO2 shows in the gas cells outside the gap; after `set` of one stone
/// (or 1 Frost emit on the gap's water, making ice) the same 3 emits put
/// every vent out.
#[test]
#[ignore = "Lane B (Mon): body with the gas field"]
fn hollow_gap_spills_until_closed() {
    todo!("M4b")
}

/// M5: a Water or Frost emit on a vent gives `Extinguished` and then `Relit`
/// within `FLICKER_S + 0.2` s; Fire on a vent changes nothing.
#[test]
#[ignore = "Lane B (Mon): body with vents"]
fn water_and_frost_only_flicker_vents() {
    todo!("M5")
}

/// M6: an emit of each into a closed 6 m tall box puts H2 in the top gas layer
/// and CO2 in the bottom layer after 3 s.
#[test]
#[ignore = "Lane B (Mon): body with the gas field"]
fn h2_rises_co2_sinks() {
    todo!("M6")
}

/// M7: the burning workshop stamp spreads to a touching wood block within
/// 3 s; a Water emit on each burning cell puts the workshop out; a Fire emit
/// relights it.
#[test]
#[ignore = "Lane B (Mon): body with fire"]
fn fire_spreads_through_wood_and_water_puts_it_out() {
    todo!("M7")
}

/// M8: sand held at 1720 °C becomes glass and sand at 1600 °C stays sand; iron
/// ore at 1250 °C next to coal becomes iron and uses the coal; without coal
/// it stays ore.
#[test]
#[ignore = "Lane B (Tue): body with the glass and iron rules"]
fn sand_to_glass_and_ore_to_iron() {
    todo!("M8")
}

/// M9: H2 and CH4 positive, O2 and CO2 negative, for the cave balloon
/// (2 m³, 1.5 kg).
#[test]
#[ignore = "Lane B (Tue): body with lift_n"]
fn lift_sign() {
    todo!("M9")
}

/// M10: the same world and emits give the same `hash()` after 300 steps,
/// twice in a run.
#[test]
#[ignore = "Lane B (Tue): body once the step does work"]
fn deterministic_steps() {
    todo!("M10")
}

/// M11: with 20 000 burning wood cells, one step updates at most 2 000 burning
/// cells and carries the rest over.
#[test]
#[ignore = "Lane B (Tue): body with the per-pass caps"]
fn caps_hold() {
    todo!("M11")
}
