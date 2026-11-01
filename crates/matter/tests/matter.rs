//! I-052 acceptance tests for the matter crate (spec section 11, M1-M11),
//! plus the contract-step checks that pin the shared API (section 10).
//!
//! Tests marked `#[ignore]` have their body written by Lane B (spec section
//! 12). Each ignored body is `todo!()`, so taking the `#[ignore]` off without
//! writing the body fails.

mod common;

use common::*;
use realgame_matter::{lift_n, By, Emit, EmitKind, Gas, MatterEvent, World, DT, FLICKER_S};
use realgame_voxel::{self as voxel, mat, Cell};

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
//
// M1, M2, M4, M4b and M7 run "on the generated world". Their checks live in
// `common` and run here on `voxel::generate`, which stays ignored until Lane
// A's stamps exist (V3); `tests/rules.rs` runs the same checks on hand-built
// fixtures of those stamps (section 4 coordinates), green from Lane B's
// first round.
// ---------------------------------------------------------------------------

/// M1a: on the generated world, Frost freezes the top layer of the ford first.
#[test]
#[ignore = "waits on Lane A's generate() stamps (V3); same check green on the fixture in rules.rs"]
fn frost_freezes_top_layer_first() {
    check_frost_freezes_top_layer_first(generated_world());
}

/// M1b: 3 Frost emits along one row of the ford make a walkable line of ice
/// (`solid_boxes` cover the path from bank to bank for a 0.7 m wide body).
#[test]
#[ignore = "waits on Lane A's generate() stamps (V3) and ice_boxes (V8); same check green on the fixture in rules.rs"]
fn ford_bridges_in_three_casts_per_row() {
    check_ford_bridges_in_three_casts_per_row(generated_world(), true);
}

/// M2: the gate stamp's ice has no ice left after 3 Fire emits and 5 s of
/// steps; the meltwater flows downhill (some water cell is lower than the
/// gate's base after 5 s).
#[test]
#[ignore = "waits on Lane A's generate() stamps (V3); same check green on the fixture in rules.rs"]
fn fire_melts_ice_gate_in_three_casts() {
    check_fire_melts_ice_gate_in_three_casts(generated_world());
}

/// M3: in a closed basin with no sources, the sum of levels is the same after
/// 500 steps; water never moves into a solid cell or upward.
#[test]
fn water_conserves_volume() {
    let mut vox = voxel::World::new();
    // A closed stone basin, inner floor at y 11, inner walls x/z 11..21.
    fill(&mut vox, [10, 10, 10], [22, 20, 22], Cell::new(mat::STONE));
    fill(&mut vox, [11, 11, 11], [21, 20, 21], Cell::AIR);
    // A step inside the basin, so water has to fall and spread.
    fill(&mut vox, [11, 11, 11], [15, 13, 21], Cell::new(mat::STONE));
    // A 3 x 3 x 3 block of water up high, with odd levels, not settled.
    let mut sum0 = 0u32;
    for z in 12..15 {
        for y in 15..18 {
            for x in 12..15 {
                let mut c = Cell::new(mat::WATER);
                let lv = 1 + ((x + y + z) % 8) as u8;
                c.set_level(lv);
                sum0 += lv as u32;
                vox.set([x, y, z], c);
            }
        }
    }
    let solid0: Vec<[i32; 3]> = cells(&vox, [10, 10, 10], [22, 20, 22], |c| c.mat == mat::STONE);
    let mut m = World::new(3);
    let mut ev = Vec::new();
    let mut top = 17;
    for _ in 0..500 {
        m.step(&mut vox, &mut ev);
        let water = cells(&vox, [0, 0, 0], [40, 40, 40], |c| c.mat == mat::WATER);
        let sum: u32 = water.iter().map(|&p| vox.get(p).level() as u32).sum();
        assert_eq!(sum, sum0, "the sum of levels is conserved");
        let t = water.iter().map(|p| p[1]).max().unwrap_or(0);
        assert!(t <= top, "water never moves upward ({t} > {top})");
        top = t;
        for &p in &solid0 {
            assert_eq!(
                vox.get(p).mat,
                mat::STONE,
                "water never enters a solid cell"
            );
        }
    }
    assert!(top <= 13, "the water fell to the floor (top {top})");
}

fn cells(
    vox: &voxel::World,
    min: [i32; 3],
    max: [i32; 3],
    f: impl Fn(Cell) -> bool,
) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for z in min[2]..max[2] {
        for y in min[1]..max[1] {
            for x in min[0]..max[0] {
                if f(vox.get([x, y, z])) {
                    out.push([x, y, z]);
                }
            }
        }
    }
    out
}

/// M4: 3 CO2 emits above the methane hollow give O2 < 15 % in every vent's gas
/// cell within 5 s, all 6 vents report `Smothered`, and the hollow's CO2 does
/// not climb over its rim (the gas cells just above the rim stay below 5 %).
#[test]
#[ignore = "waits on Lane A's generate() stamps (V3); same check green on the fixture in rules.rs"]
fn co2_pools_in_hollow() {
    check_co2_pools_in_hollow(generated_world());
}

/// M4b: with the rim gap open, 3 CO2 emits leave at least 1 vent lit after 5 s
/// and CO2 shows in the gas cells outside the gap; after `set` of one stone
/// (or 1 Frost emit on the gap's water, making ice) the same 3 emits put
/// every vent out.
#[test]
#[ignore = "waits on Lane A's generate() stamps (V3); same check green on the fixture in rules.rs"]
fn hollow_gap_spills_until_closed() {
    check_hollow_gap_spills_until_closed(generated_world(), close_with_stone);
    let vox = generated_world();
    if vox.get(GAP).mat == mat::WATER {
        check_hollow_gap_spills_until_closed(vox, close_with_frost);
    }
}

/// M5: a Water or Frost emit on a vent gives `Extinguished` and then `Relit`
/// within `FLICKER_S + 0.2` s; Fire on a vent changes nothing.
#[test]
fn water_and_frost_only_flicker_vents() {
    check_water_and_frost_only_flicker_vents(fixture_world());
}

/// M6: an emit of each into a closed 6 m tall box puts H2 in the top gas layer
/// and CO2 in the bottom layer after 3 s.
#[test]
fn h2_rises_co2_sinks() {
    let mut vox = voxel::World::new();
    // Stone all round; the inside is gas cells x 1..3, y 1..4, z 1..3
    // (blocks x 2..6, y 2..8, z 2..6): 6 m tall, 3 gas layers.
    fill(&mut vox, [0, 0, 0], [8, 10, 8], Cell::new(mat::STONE));
    fill(&mut vox, [2, 2, 2], [6, 8, 6], Cell::AIR);
    let mut m = World::new(5);
    let mut ev = Vec::new();
    m.add_gas(&vox, [4.0, 4.5, 4.0], Gas::H2, 2.0);
    m.emit(Emit {
        pos: [4.0, 5.0, 4.0],
        normal: [0, 1, 0],
        what: EmitKind::Co2,
        amount: 1.0,
    });
    step_n(&mut m, &mut vox, steps(3.0), &mut ev);
    let layer = |gy: i32, g: usize| -> f32 {
        let y = gy * 2;
        let mut s = 0.0;
        for z in [2, 4] {
            for x in [2, 4] {
                s += m.gas_at([x, y, z])[g];
            }
        }
        s
    };
    let (h_bot, h_mid, h_top) = (layer(1, 1), layer(2, 1), layer(3, 1));
    let (c_bot, c_mid, c_top) = (layer(1, 0), layer(2, 0), layer(3, 0));
    assert!(h_top > 0.0, "H2 is still in the box");
    assert!(h_top > h_mid + h_bot, "H2 rose: {h_bot} {h_mid} {h_top}");
    assert!(c_bot > 0.0, "CO2 is still in the box");
    assert!(c_bot > c_mid + c_top, "CO2 sank: {c_bot} {c_mid} {c_top}");
}

/// M7: the burning workshop stamp spreads to a touching wood block within
/// 3 s; a Water emit on each burning cell puts the workshop out; a Fire emit
/// relights it.
#[test]
#[ignore = "waits on Lane A's generate() stamps (V3); same check green on the fixture in rules.rs"]
fn fire_spreads_through_wood_and_water_puts_it_out() {
    check_fire_spreads_through_wood_and_water_puts_it_out(generated_world());
}

fn hot(m: u8, c: f32) -> Cell {
    let mut cell = Cell::new(m);
    cell.set_temp_k(c + 273.15);
    cell
}

/// M8: sand held at 1720 °C becomes glass and sand at 1600 °C stays sand; iron
/// ore at 1250 °C next to coal becomes iron and uses the coal; without coal
/// it stays ore.
#[test]
fn sand_to_glass_and_ore_to_iron() {
    let mut vox = voxel::World::new();
    let (s1, s2, ore1, coal, ore2) = (
        [10, 10, 10],
        [20, 10, 10],
        [30, 10, 10],
        [31, 10, 10],
        [40, 10, 10],
    );
    vox.set(s1, hot(mat::SAND, 1720.0));
    vox.set(s2, hot(mat::SAND, 1600.0));
    vox.set(ore1, hot(mat::IRON_ORE, 1250.0));
    vox.set(coal, Cell::new(mat::COAL_ORE));
    vox.set(ore2, hot(mat::IRON_ORE, 1250.0));
    let mut m = World::new(8);
    let mut ev = Vec::new();
    step_n(&mut m, &mut vox, steps(1.0), &mut ev);
    assert_eq!(vox.get(s1).mat, mat::GLASS, "sand at 1720 °C made glass");
    assert!(ev.contains(&MatterEvent::Glass { cell: s1 }));
    assert_eq!(vox.get(s2).mat, mat::SAND, "sand at 1600 °C stays sand");
    assert_eq!(vox.get(ore1).mat, mat::IRON, "ore next to coal made iron");
    assert!(ev.contains(&MatterEvent::Iron { cell: ore1 }));
    assert_eq!(vox.get(coal).mat, mat::AIR, "the coal was used up");
    assert_eq!(
        vox.get(ore2).mat,
        mat::IRON_ORE,
        "ore without coal stays ore"
    );
}

/// M9: H2 and CH4 positive, O2 and CO2 negative, for the cave balloon
/// (2 m³, 1.5 kg).
#[test]
fn lift_sign() {
    assert!(lift_n(Gas::H2, 2.0, 1.5) > 0.0);
    assert!(lift_n(Gas::Ch4, 2.0, 1.5) > 0.0);
    assert!(lift_n(Gas::O2, 2.0, 1.5) < 0.0);
    assert!(lift_n(Gas::Co2, 2.0, 1.5) < 0.0);
    // Section 5.7's worked numbers: about +7.2 N for H2, -17.3 N for O2.
    assert!((lift_n(Gas::H2, 2.0, 1.5) - 7.2).abs() < 0.3);
    assert!((lift_n(Gas::O2, 2.0, 1.5) + 17.3).abs() < 0.5);
}

fn run_scripted(seed: u64) -> u64 {
    let mut vox = fixture_world();
    let mut m = World::new(seed);
    let mut ev = Vec::new();
    let h = hollow(&vox);
    for t in 0..300 {
        match t {
            5 => emit(&mut m, [128.5, 30.0, 154.5], [0, 1, 0], EmitKind::Frost),
            10 => emit(&mut m, [60.0, 39.5, 94.0], [0, 0, 1], EmitKind::Fire),
            20 => emit(&mut m, h.hit, [0, 1, 0], EmitKind::Co2),
            30 => emit(&mut m, [141.5, 37.5, 185.0], [0, 0, -1], EmitKind::Water),
            40 => m.add_gas(&vox, [191.0, 29.0, 100.0], Gas::H2, 2.0),
            _ => {}
        }
        m.step(&mut vox, &mut ev);
    }
    m.hash(&vox)
}

/// M10: the same world and emits give the same `hash()` after 300 steps,
/// twice in a run.
#[test]
fn deterministic_steps() {
    let a = run_scripted(42);
    let b = run_scripted(42);
    assert_eq!(a, b, "same world and emits, same hash");
    let fresh = World::new(42).hash(&fixture_world());
    assert_ne!(a, fresh, "the 300 steps changed the world");
}

/// M11: with 20 000 burning wood cells, one step updates at most 2 000 burning
/// cells and carries the rest over.
#[test]
fn caps_hold() {
    let mut vox = voxel::World::new();
    let mut c = Cell::new(mat::WOOD);
    c.set_flag(Cell::BURNING, true);
    fill(&mut vox, [0, 40, 0], [200, 41, 100], c);
    let mut m = World::new(9);
    let mut ev = Vec::new();
    m.step(&mut vox, &mut ev);
    let s = m.stats();
    assert!(s.fire <= 2000, "one step updated {} burning cells", s.fire);
    assert_eq!(s.fire + s.fire_carried, 20_000, "the rest carried over");
    let first = m.burn_left([0, 40, 0]);
    for _ in 0..9 {
        m.step(&mut vox, &mut ev);
        assert!(m.stats().fire <= 2000);
    }
    // After 10 steps every burning cell has been updated at least once.
    let full = m.burn_left([199, 40, 99]);
    assert!(first.is_some() && full.is_some());
    assert!(
        full.unwrap() < m.burn_full(mat::WOOD),
        "the last cell was reached by carry-over"
    );
}
