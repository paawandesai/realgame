//! Shared helpers for the matter acceptance tests (I-052 Lane B).
//!
//! Each puzzle check (M1, M2, M4, M4b, M5, M7) is written once as a function
//! over a `voxel::World`, then run twice:
//!
//! - on `fixture_world()`: the puzzle stamps built by hand here, inside the
//!   same stamp boxes and at the coordinates of spec section 4, so the matter
//!   rules are tested before Lane A's generator exists;
//! - on `voxel::generate(DEFAULT_SEED)` (the spec's "generated world"), which
//!   stays `#[ignore]` until Lane A's stamps land (V3).
//!
//! The checks only look inside the stamp boxes (they find the water, ice,
//! vents and burning cells there), so they do not depend on the fixture's
//! exact shapes. What they do need from the generated world is listed in
//! `docs/loop/items/I-052/lanes/B/round-1.md`.

#![allow(dead_code)] // each test binary uses a different subset

use realgame_matter::{By, Emit, EmitKind, MatterEvent, World, DT, FLICKER_S};
use realgame_voxel::{self as voxel, mat, stamp, Cell};

/// The methane hollow's rim gap (spec section 5.5): x 196, z 101, y 28.
pub const GAP: [i32; 3] = [196, 28, 101];

pub fn fill(vox: &mut voxel::World, min: [i32; 3], max: [i32; 3], cell: Cell) {
    for z in min[2]..max[2] {
        for y in min[1]..max[1] {
            for x in min[0]..max[0] {
                vox.set([x, y, z], cell);
            }
        }
    }
}

fn water(level: u8) -> Cell {
    let mut c = Cell::new(mat::WATER);
    c.set_level(level);
    c.set_flag(Cell::SETTLED, true);
    c
}

/// The river ford (P2a): 3-deep still water (y 27-29, surface y 30) across
/// z 150..158, banks with their top block at y 30 on both sides, walled ends.
pub fn build_ford(vox: &mut voxel::World) {
    let stone = Cell::new(mat::STONE);
    fill(vox, [110, 20, 140], [146, 31, 168], stone);
    fill(
        vox,
        [110, 31, 140],
        [122, 41, 168],
        Cell::new(mat::CLIFFSTONE),
    );
    fill(
        vox,
        [134, 31, 140],
        [146, 41, 168],
        Cell::new(mat::CLIFFSTONE),
    );
    fill(vox, [122, 27, 150], [134, 30, 158], water(8));
}

/// The ice gate (P2b): a protected cliff band z 86..94 with a 4-wide,
/// 3-tall opening at x 58..62, plugged by one layer of ice at z 93. Ground
/// top at y 37; south of the band the ground falls away, so meltwater runs
/// downhill.
pub fn build_ice_gate(vox: &mut voxel::World) {
    let stone = Cell::new(mat::STONE);
    fill(vox, [44, 26, 76], [76, 38, 94], stone);
    for z in 94..124 {
        let top = 37 - (z - 94) / 3;
        fill(vox, [44, 26, z], [76, top + 1, z + 1], stone);
    }
    fill(vox, [44, 38, 86], [76, 50, 94], Cell::new(mat::CLIFFSTONE));
    fill(vox, [58, 38, 86], [62, 41, 94], Cell::AIR);
    fill(vox, [58, 38, 93], [62, 41, 94], Cell::new(mat::ICE));
}

/// The methane hollow (P3): a ravine x 187..196, z 97..105, floor block at
/// y 27, rim top at y 31, six burning vents on the floor, and the one-block
/// rim gap at `GAP` that opens into a trench and a sunken marsh pit to the
/// east. `gap_water` puts shallow water (level 3) in the gap.
pub fn build_hollow(vox: &mut voxel::World, gap_water: bool) {
    fill(vox, [176, 20, 86], [226, 32, 118], Cell::new(mat::MUD));
    fill(vox, [187, 28, 97], [196, 32, 105], Cell::AIR);
    let mut vent = Cell::new(mat::GAS_VENT);
    vent.set_flag(Cell::BURNING, true);
    for p in vents_spec() {
        vox.set(p, vent);
    }
    vox.set(GAP, if gap_water { water(3) } else { Cell::AIR });
    // East of the one-block wall at x 196: a sunken marsh pool, floor block
    // at y 21, open to the sky, that the gap's CO2 pours down into.
    fill(vox, [197, 22, 96], [216, 32, 106], Cell::AIR);
}

/// The six vent cells of the fixture hollow.
pub fn vents_spec() -> [[i32; 3]; 6] {
    [
        [188, 28, 98],
        [191, 28, 98],
        [194, 28, 98],
        [188, 28, 103],
        [191, 28, 103],
        [194, 28, 103],
    ]
}

/// The burning workshop (P1): a planks hut x 138..146, z 186..194 on ground
/// whose top block is at y 33, wood corner posts, a planks roof at y 38, a
/// door gap on the south side, and two burning planks in the north wall.
pub fn build_workshop(vox: &mut voxel::World) {
    fill(vox, [130, 26, 178], [154, 34, 202], Cell::new(mat::DIRT));
    let planks = Cell::new(mat::PLANKS);
    for y in 34..38 {
        for x in 138..146 {
            vox.set([x, y, 186], planks);
            vox.set([x, y, 193], planks);
        }
        for z in 186..194 {
            vox.set([138, y, z], planks);
            vox.set([145, y, z], planks);
        }
    }
    for (x, z) in [(138, 186), (145, 186), (138, 193), (145, 193)] {
        fill(vox, [x, 34, z], [x + 1, 39, z + 1], Cell::new(mat::WOOD));
    }
    fill(vox, [139, 38, 187], [145, 39, 193], planks);
    fill(vox, [141, 34, 193], [143, 36, 194], Cell::AIR);
    let mut burning = planks;
    burning.set_flag(Cell::BURNING, true);
    vox.set([141, 37, 186], burning);
    vox.set([142, 37, 186], burning);
}

/// All four puzzle fixtures in one otherwise empty world.
pub fn fixture_world() -> voxel::World {
    let mut vox = voxel::World::new();
    build_ford(&mut vox);
    build_ice_gate(&mut vox);
    build_hollow(&mut vox, false);
    build_workshop(&mut vox);
    vox
}

/// The generated world (Lane A).
pub fn generated_world() -> voxel::World {
    voxel::generate(voxel::DEFAULT_SEED)
}

pub fn step_n(m: &mut World, vox: &mut voxel::World, n: usize, ev: &mut Vec<MatterEvent>) {
    for _ in 0..n {
        m.step(vox, ev);
    }
}

pub fn steps(secs: f32) -> usize {
    (secs / DT).round() as usize
}

/// Every cell of `mat` inside a stamp box, in z, y, x order.
pub fn cells_in(vox: &voxel::World, s: &stamp::Stamp, m: u8) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for z in s.min[2]..s.max[2] {
        for y in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                if vox.get([x, y, z]).mat == m {
                    out.push([x, y, z]);
                }
            }
        }
    }
    out
}

pub fn emit(m: &mut World, pos: [f32; 3], normal: [i8; 3], what: EmitKind) {
    m.emit(Emit {
        pos,
        normal,
        what,
        amount: 1.0,
    });
}

fn add(p: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [p[0] + d[0], p[1] + d[1], p[2] + d[2]]
}

const FACES: [[i32; 3]; 6] = [
    [0, 1, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
    [0, -1, 0],
];

/// The centre of one of `p`'s faces that opens onto air (top first), as an
/// emit position and normal; `None` when the cell is enclosed.
pub fn air_face(vox: &voxel::World, p: [i32; 3]) -> Option<([f32; 3], [i8; 3])> {
    FACES.iter().find_map(|&d| {
        let q = add(p, d);
        (vox.get(q).mat == mat::AIR).then(|| {
            let pos = [0, 1, 2].map(|a| p[a] as f32 + 0.5 + 0.5 * d[a] as f32);
            (pos, d.map(|v| v as i8))
        })
    })
}

// ---------------------------------------------------------------------------
// The checks, one per acceptance test.
// ---------------------------------------------------------------------------

/// Top-layer water cells of the ford (water with no water above), and the
/// column the checks use: the middle x, with its z extent.
fn ford_row(vox: &voxel::World) -> (i32, i32, i32, i32) {
    let water = cells_in(vox, &stamp::FORD, mat::WATER);
    let tops: Vec<[i32; 3]> = water
        .iter()
        .copied()
        .filter(|&p| vox.get(add(p, [0, 1, 0])).mat != mat::WATER)
        .collect();
    assert!(!tops.is_empty(), "the ford stamp holds surface water");
    let xmin = tops.iter().map(|p| p[0]).min().unwrap();
    let xmax = tops.iter().map(|p| p[0]).max().unwrap();
    let x = (xmin + xmax) / 2;
    let row: Vec<[i32; 3]> = tops.iter().copied().filter(|p| p[0] == x).collect();
    let y = row[0][1];
    let zmin = row.iter().map(|p| p[2]).min().unwrap();
    let zmax = row.iter().map(|p| p[2]).max().unwrap();
    (x, y, zmin, zmax)
}

/// M1a: one Frost emit on the ford's surface freezes the top cell first; the
/// water under it is still liquid.
pub fn check_frost_freezes_top_layer_first(mut vox: voxel::World) {
    let (x, y, zmin, zmax) = ford_row(&vox);
    let z = (zmin + zmax) / 2;
    assert_eq!(vox.get([x, y - 1, z]).mat, mat::WATER, "the ford is deep");
    let mut m = World::new(1);
    let mut ev = Vec::new();
    emit(
        &mut m,
        [x as f32 + 0.5, (y + 1) as f32, z as f32 + 0.5],
        [0, 1, 0],
        EmitKind::Frost,
    );
    step_n(&mut m, &mut vox, steps(1.0), &mut ev);
    assert_eq!(vox.get([x, y, z]).mat, mat::ICE, "the surface froze");
    assert_eq!(
        vox.get([x, y - 1, z]).mat,
        mat::WATER,
        "the water under the ice is still liquid (top layer first)"
    );
    assert!(
        ev.contains(&MatterEvent::Froze { cell: [x, y, z] }),
        "a Froze event for the surface cell"
    );
}

/// M1b: three Frost emits along one row make a line of ice from bank to bank
/// under a 0.7 m wide body. `with_boxes` also checks `voxel::ice_boxes`
/// (Lane A's collision boxes) along the path.
pub fn check_ford_bridges_in_three_casts_per_row(mut vox: voxel::World, with_boxes: bool) {
    let (x, y, zmin, zmax) = ford_row(&vox);
    let mut m = World::new(1);
    let mut ev = Vec::new();
    let mut z = zmin + 1;
    for _ in 0..3 {
        emit(
            &mut m,
            [x as f32 + 0.5, (y + 1) as f32, z as f32 + 0.5],
            [0, 1, 0],
            EmitKind::Frost,
        );
        step_n(&mut m, &mut vox, steps(1.0), &mut ev);
        z += 3;
    }
    step_n(&mut m, &mut vox, steps(1.0), &mut ev);
    // A 0.7 m wide body walking along x + 0.5 stays inside column x.
    for z in zmin..=zmax {
        let c = vox.get([x, y, z]);
        assert_eq!(c.mat, mat::ICE, "ice at {:?}", [x, y, z]);
    }
    // The banks at both ends are solid ground at the same height or 1 higher.
    for z in [zmin - 1, zmax + 1] {
        let bank = (y..=y + 1).any(|yy| {
            let c = vox.get([x, yy, z]);
            c.mat != mat::AIR && c.mat != mat::WATER
        });
        assert!(bank, "a bank at z {z}");
    }
    if with_boxes {
        let path = [
            x as f32 + 0.15,
            y as f32 + 0.5,
            zmin as f32,
            x as f32 + 0.85,
            (y + 1) as f32,
            (zmax + 1) as f32,
        ];
        let boxes = voxel::ice_boxes(&vox, path);
        for z in zmin..=zmax {
            let covered = boxes.iter().any(|b| {
                b[0] <= x as f32 + 0.15
                    && b[3] >= x as f32 + 0.85
                    && b[2] <= z as f32
                    && b[5] >= (z + 1) as f32
                    && b[4] >= (y + 1) as f32 - 0.01
            });
            assert!(covered, "ice_boxes cover the path at z {z}");
        }
    }
}

/// M2: three Fire emits on the ice gate and 5 s of steps leave no ice in the
/// gate stamp; some meltwater ends lower than the gate's base.
pub fn check_fire_melts_ice_gate_in_three_casts(mut vox: voxel::World) {
    let ice = cells_in(&vox, &stamp::ICE_GATE, mat::ICE);
    assert!(!ice.is_empty(), "the gate stamp holds ice");
    // The face the player meets: the ice cells with air to the south (+z).
    let front: Vec<[i32; 3]> = ice
        .iter()
        .copied()
        .filter(|&p| vox.get(add(p, [0, 0, 1])).mat == mat::AIR)
        .collect();
    assert!(!front.is_empty(), "the gate has a face to the south");
    let zf = front.iter().map(|p| p[2]).max().unwrap();
    let x0 = front.iter().map(|p| p[0]).min().unwrap() as f32;
    let x1 = front.iter().map(|p| p[0]).max().unwrap() as f32 + 1.0;
    let y0 = front.iter().map(|p| p[1]).min().unwrap();
    let y1 = front.iter().map(|p| p[1]).max().unwrap() as f32 + 1.0;
    let ymid = (y0 as f32 + y1) / 2.0;
    let mut m = World::new(1);
    let mut ev = Vec::new();
    for k in [1.0, 3.0, 5.0] {
        let x = x0 + (x1 - x0) * k / 6.0;
        emit(
            &mut m,
            [x, ymid, (zf + 1) as f32],
            [0, 0, 1],
            EmitKind::Fire,
        );
        step_n(&mut m, &mut vox, 3, &mut ev);
    }
    step_n(&mut m, &mut vox, steps(5.0), &mut ev);
    let left = cells_in(&vox, &stamp::ICE_GATE, mat::ICE);
    assert!(left.is_empty(), "no ice left in the gate: {left:?}");
    assert!(
        ev.iter().any(|e| matches!(e, MatterEvent::Melted { .. })),
        "Melted events"
    );
    let g = stamp::ICE_GATE;
    let mut lower = false;
    for z in g.min[2] - 12..g.max[2] + 12 {
        for x in g.min[0] - 12..g.max[0] + 12 {
            for y in y0 - 12..y0 {
                lower |= vox.get([x, y, z]).mat == mat::WATER;
            }
        }
    }
    assert!(lower, "some meltwater flowed below the gate's base y {y0}");
}

/// The hollow's vents, its floor-centre emit point and its rim top.
pub struct Hollow {
    pub vents: Vec<[i32; 3]>,
    pub hit: [f32; 3],
    pub rim_top: i32,
}

pub fn hollow(vox: &voxel::World) -> Hollow {
    let s = stamp::HOLLOW;
    let vents = cells_in(vox, &s, mat::GAS_VENT);
    assert_eq!(vents.len(), 6, "six vents in the hollow");
    let cx = vents.iter().map(|p| p[0]).sum::<i32>() / 6;
    let cz = vents.iter().map(|p| p[2]).sum::<i32>() / 6;
    let mut y = s.max[1];
    while y > s.min[1] && vox.get([cx, y - 1, cz]).mat == mat::AIR {
        y -= 1;
    }
    let mut rim_top = s.min[1];
    for z in s.min[2]..s.max[2] {
        for yy in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                let c = vox.get([x, yy, z]);
                if c.mat != mat::AIR && c.mat != mat::WATER {
                    rim_top = rim_top.max(yy);
                }
            }
        }
    }
    Hollow {
        vents,
        hit: [cx as f32 + 0.5, y as f32, cz as f32 + 0.5],
        rim_top,
    }
}

fn lit(vox: &voxel::World, p: [i32; 3]) -> bool {
    vox.get(p).has(Cell::BURNING)
}

/// The O2 fraction at a vent's flame (the cell above the vent).
fn vent_o2(m: &World, v: [i32; 3]) -> f32 {
    m.o2_at(add(v, [0, 1, 0]))
}

fn cast_co2_three_times(
    m: &mut World,
    vox: &mut voxel::World,
    h: &Hollow,
    ev: &mut Vec<MatterEvent>,
) {
    for _ in 0..3 {
        emit(m, h.hit, [0, 1, 0], EmitKind::Co2);
        step_n(m, vox, steps(0.5), ev);
    }
}

/// M4: with the rim gap closed, three CO2 emits put every vent out within 5 s
/// (O2 < 15 % at each, a Smothered event each), and the gas cells just above
/// the rim stay below 5 % CO2.
pub fn check_co2_pools_in_hollow(mut vox: voxel::World) {
    vox.set(GAP, Cell::new(mat::STONE));
    let h = hollow(&vox);
    let mut m = World::new(1);
    let mut ev = Vec::new();
    m.touch(&vox, GAP);
    step_n(&mut m, &mut vox, 2, &mut ev);
    assert!(h.vents.iter().all(|&v| lit(&vox, v)), "vents burn at first");
    cast_co2_three_times(&mut m, &mut vox, &h, &mut ev);
    step_n(&mut m, &mut vox, steps(5.0) - steps(1.5), &mut ev);
    for &v in &h.vents {
        assert!(vent_o2(&m, v) < 0.15, "O2 {} at vent {v:?}", vent_o2(&m, v));
        assert!(!lit(&vox, v), "vent {v:?} is out");
        assert!(
            ev.contains(&MatterEvent::Smothered { vent: v }),
            "Smothered for {v:?}"
        );
    }
    let s = stamp::HOLLOW;
    for z in (s.min[2]..s.max[2]).step_by(2) {
        for x in (s.min[0]..s.max[0]).step_by(2) {
            let co2 = m.gas_at([x, h.rim_top + 1, z])[0];
            assert!(co2 < 0.05, "CO2 {co2} above the rim at x {x} z {z}");
        }
    }
}

/// M4b: with the gap open, three CO2 emits leave at least one vent lit after
/// 5 s and CO2 shows outside the gap; once the gap is closed (`close`), the
/// same three emits put every vent out.
pub fn check_hollow_gap_spills_until_closed(
    mut vox: voxel::World,
    close: fn(&mut World, &mut voxel::World, &mut Vec<MatterEvent>),
) {
    let h = hollow(&vox);
    let mut m = World::new(1);
    let mut ev = Vec::new();
    step_n(&mut m, &mut vox, 2, &mut ev);
    cast_co2_three_times(&mut m, &mut vox, &h, &mut ev);
    step_n(&mut m, &mut vox, steps(5.0) - steps(1.5), &mut ev);
    let lit_n = h.vents.iter().filter(|&&v| lit(&vox, v)).count();
    assert!(
        lit_n >= 1,
        "with the gap open at least one vent still burns"
    );
    // Outside the gap: the cells east of it, at the gap's height and below
    // (spilled CO2 keeps sinking).
    let outside = (GAP[0] + 2..GAP[0] + 12)
        .flat_map(|x| (GAP[1] - 8..=GAP[1]).map(move |y| [x, y, GAP[2]]))
        .map(|p| m.gas_at(p)[0])
        .fold(0.0f32, f32::max);
    assert!(
        outside > 0.02,
        "CO2 spilled out through the gap ({outside})"
    );
    close(&mut m, &mut vox, &mut ev);
    let g = vox.get(GAP);
    assert!(
        g.mat != mat::AIR && g.mat != mat::WATER,
        "the gap is closed (mat {})",
        g.mat
    );
    ev.clear();
    cast_co2_three_times(&mut m, &mut vox, &h, &mut ev);
    step_n(&mut m, &mut vox, steps(5.0) - steps(1.5), &mut ev);
    for &v in &h.vents {
        assert!(!lit(&vox, v), "vent {v:?} is out once the gap is closed");
        assert!(vent_o2(&m, v) < 0.15, "O2 {} at vent {v:?}", vent_o2(&m, v));
    }
}

/// Closes the gap with a stone block (what a player places).
pub fn close_with_stone(m: &mut World, vox: &mut voxel::World, _ev: &mut Vec<MatterEvent>) {
    vox.set(GAP, Cell::new(mat::STONE));
    m.touch(vox, GAP);
}

/// Closes the gap with one Frost emit on the shallow water in it.
pub fn close_with_frost(m: &mut World, vox: &mut voxel::World, ev: &mut Vec<MatterEvent>) {
    let c = vox.get(GAP);
    assert_eq!(c.mat, mat::WATER, "shallow water in the gap");
    let top = GAP[1] as f32 + c.level() as f32 / 8.0;
    emit(
        m,
        [GAP[0] as f32 + 0.5, top, GAP[2] as f32 + 0.5],
        [0, 1, 0],
        EmitKind::Frost,
    );
    step_n(m, vox, steps(1.0), ev);
}

/// M5: a Water or Frost emit on a vent gives `Extinguished` and then `Relit`
/// within `FLICKER_S + 0.2` s; Fire on a vent changes nothing.
pub fn check_water_and_frost_only_flicker_vents(mut vox: voxel::World) {
    let h = hollow(&vox);
    let v = h.vents[0];
    let mut m = World::new(1);
    let mut ev = Vec::new();
    step_n(&mut m, &mut vox, 2, &mut ev);
    assert!(lit(&vox, v));
    let hit = [v[0] as f32 + 0.5, (v[1] + 1) as f32, v[2] as f32 + 0.5];
    for (kind, by) in [(EmitKind::Water, By::Water), (EmitKind::Frost, By::Frost)] {
        ev.clear();
        emit(&mut m, hit, [0, 1, 0], kind);
        m.step(&mut vox, &mut ev);
        assert!(
            ev.contains(&MatterEvent::Extinguished { cell: v, by }),
            "{kind:?} knocks the flame down: {ev:?}"
        );
        assert!(!lit(&vox, v), "{kind:?}: out for now");
        let limit = ((FLICKER_S + 0.2) / DT).round() as usize;
        let mut relit = false;
        for _ in 0..limit {
            m.step(&mut vox, &mut ev);
            if ev.contains(&MatterEvent::Relit { vent: v }) {
                relit = true;
                break;
            }
        }
        assert!(relit, "{kind:?}: relit within FLICKER_S + 0.2 s");
        assert!(lit(&vox, v));
    }
    ev.clear();
    emit(&mut m, hit, [0, 1, 0], EmitKind::Fire);
    step_n(&mut m, &mut vox, steps(2.0), &mut ev);
    let touched = ev.iter().any(|e| {
        matches!(e, MatterEvent::Extinguished { cell, .. } if *cell == v)
            || *e == MatterEvent::Smothered { vent: v }
            || *e == MatterEvent::Relit { vent: v }
    });
    assert!(!touched, "Fire on a vent changes nothing: {ev:?}");
    assert!(lit(&vox, v), "still burning after Fire");
}

fn flammable(m: u8) -> bool {
    m == mat::WOOD || m == mat::PLANKS
}

fn burning_in_workshop(vox: &voxel::World) -> Vec<[i32; 3]> {
    let s = stamp::WORKSHOP;
    let mut out = Vec::new();
    for z in s.min[2]..s.max[2] {
        for y in s.min[1]..s.max[1] {
            for x in s.min[0]..s.max[0] {
                if vox.get([x, y, z]).has(Cell::BURNING) {
                    out.push([x, y, z]);
                }
            }
        }
    }
    out
}

/// M7: the burning workshop spreads to a touching wood block within 3 s; a
/// Water emit on each burning cell puts it out; a Fire emit relights it.
pub fn check_fire_spreads_through_wood_and_water_puts_it_out(mut vox: voxel::World) {
    let b0 = burning_in_workshop(&vox);
    assert!(!b0.is_empty(), "the workshop burns at the start");
    let mut m = World::new(1);
    let mut ev = Vec::new();
    let touching: Vec<[i32; 3]> = b0
        .iter()
        .flat_map(|&p| FACES.iter().map(move |&d| add(p, d)))
        .filter(|&q| flammable(vox.get(q).mat) && !b0.contains(&q))
        .collect();
    assert!(!touching.is_empty(), "wood touches the fire");
    let mut spread = false;
    for _ in 0..steps(3.0) {
        m.step(&mut vox, &mut ev);
        if touching.iter().any(|&q| vox.get(q).has(Cell::BURNING)) {
            spread = true;
            break;
        }
    }
    assert!(
        spread,
        "the fire spread to a touching wood block within 3 s"
    );
    assert!(ev.iter().any(|e| matches!(e, MatterEvent::Ignited { .. })));
    // Water on each burning cell, round by round.
    let mut rounds = 0;
    loop {
        let burning = burning_in_workshop(&vox);
        if burning.is_empty() {
            break;
        }
        rounds += 1;
        assert!(rounds <= 20, "still burning after 20 rounds: {burning:?}");
        for p in burning {
            let (pos, n) = air_face(&vox, p).expect("a burning cell faces air");
            emit(&mut m, pos, n, EmitKind::Water);
        }
        m.step(&mut vox, &mut ev);
    }
    step_n(&mut m, &mut vox, steps(3.0), &mut ev);
    assert!(burning_in_workshop(&vox).is_empty(), "the workshop is out");
    assert!(ev
        .iter()
        .any(|e| matches!(e, MatterEvent::Extinguished { by: By::Water, .. })));
    // Relight: a dry planks or wood cell with air beside it and no water near.
    let s = stamp::WORKSHOP;
    let mut target = None;
    'find: for z in s.min[2]..s.max[2] {
        for y in (s.min[1]..s.max[1]).rev() {
            for x in s.min[0]..s.max[0] {
                let p = [x, y, z];
                if !flammable(vox.get(p).mat) {
                    continue;
                }
                let wet = (-2..=2).any(|dz: i32| {
                    (-2..=2).any(|dy: i32| {
                        (-2..=2).any(|dx: i32| vox.get(add(p, [dx, dy, dz])).mat == mat::WATER)
                    })
                });
                if !wet {
                    if let Some(f) = air_face(&vox, p) {
                        target = Some((p, f));
                        break 'find;
                    }
                }
            }
        }
    }
    let (p, (pos, n)) = target.expect("a dry wood block to relight");
    emit(&mut m, pos, n, EmitKind::Fire);
    let mut relit = false;
    for _ in 0..steps(2.0) {
        m.step(&mut vox, &mut ev);
        relit |= !burning_in_workshop(&vox).is_empty();
    }
    assert!(relit, "a Fire emit relights the workshop (at {p:?})");
}
