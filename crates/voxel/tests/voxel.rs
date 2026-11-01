//! I-052 acceptance tests for the voxel crate (spec section 11, V1-V10),
//! plus the contract-step checks that pin the shared API (section 10).
//!
//! Tests marked `#[ignore]` have their body written by the lane that owns
//! the feature (spec section 12). Each ignored body is `todo!()`, so taking
//! the `#[ignore]` off without writing the body fails.

use realgame_voxel::{mat, materials, stamp, Cell, World, CHUNK, CHUNKS, WORLD};

// ---------------------------------------------------------------------------
// Contract step (section 10, item 2 and 3): green from the first merge.
// ---------------------------------------------------------------------------

#[test]
fn world_and_chunk_sizes_match_the_spec() {
    assert_eq!(CHUNK, 32);
    assert_eq!(WORLD, [256, 96, 256]);
    assert_eq!(CHUNKS, [8, 3, 8]);
    let w = World::new();
    assert_eq!(w.chunk_count(), 192);
    assert_eq!(World::chunk_index([0, 0, 0]), Some(0));
    assert_eq!(World::chunk_index([255, 95, 255]), Some(191));
    assert_eq!(World::chunk_index([32, 0, 0]), Some(1));
    assert_eq!(World::chunk_index([0, 32, 0]), Some(8));
    assert_eq!(World::chunk_index([0, 0, 32]), Some(24));
    assert_eq!(World::chunk_index([-1, 0, 0]), None);
    assert_eq!(World::chunk_index([0, 96, 0]), None);
}

#[test]
fn get_and_set_round_trip_and_mark_dirty() {
    let mut w = World::new();
    assert!(w.dirty_chunks().is_empty());
    assert_eq!(w.get([10, 40, 10]).mat, mat::AIR);
    let mut c = Cell::new(mat::STONE);
    c.set_temp_k(400.0);
    w.set([10, 40, 10], c);
    assert_eq!(w.get([10, 40, 10]), c);
    assert_eq!(
        w.dirty_chunks(),
        vec![World::chunk_index([10, 40, 10]).unwrap()]
    );
    // Outside the world reads as air and a set there is ignored.
    assert_eq!(w.get([-1, 0, 0]).mat, mat::AIR);
    w.set([300, 0, 0], c);
    assert_eq!(w.dirty_chunks().len(), 1);
    // A cell on a chunk face also dirties the neighbour that draws against it.
    let mut w = World::new();
    w.set([31, 40, 10], c);
    let mut want = vec![
        World::chunk_index([31, 40, 10]).unwrap(),
        World::chunk_index([32, 40, 10]).unwrap(),
    ];
    want.sort_unstable();
    assert_eq!(w.dirty_chunks(), want);
    assert_eq!(w.take_dirty(), want);
    assert!(w.dirty_chunks().is_empty());
}

#[test]
fn materials_json_has_every_must_row_in_mat_id_order() {
    let rows = materials::load(materials::MATERIALS_JSON).expect("content/materials.json parses");
    assert_eq!(
        rows.len(),
        mat::COUNT,
        "one row per Must material (section 5.2)"
    );
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.id as usize, i, "row {i} id");
        assert_eq!(row.name, mat::NAMES[i], "row {i} name");
    }
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "air",
            "dirt",
            "grass",
            "sand",
            "stone",
            "cliffstone",
            "runestone",
            "wood",
            "leaves",
            "planks",
            "snow",
            "ice",
            "water",
            "steam",
            "mud",
            "coal_ore",
            "iron_ore",
            "crystal",
            "glass",
            "iron",
            "brick",
            "ash",
            "torch",
            "bench",
            "invention_bench",
            "kiln_hearth",
            "gas_vent",
        ]
    );
    // Structural fields are filled in the contract; physical values may still
    // be "todo" until Lane B fills them (V10 then forbids any "todo").
    let by = |n: &str| rows.iter().find(|r| r.name == n).unwrap();
    assert_eq!(by("air").phase, materials::Phase::None);
    assert_eq!(by("water").phase, materials::Phase::Liquid);
    assert_eq!(by("steam").phase, materials::Phase::Gas);
    assert_eq!(by("stone").phase, materials::Phase::Solid);
    assert!(by("stone").solid && !by("water").solid && !by("air").solid);
    assert!(by("water").transparent && by("ice").transparent && by("glass").transparent);
    assert!(!by("stone").transparent);
}

#[test]
#[allow(clippy::needless_range_loop)] // three axes, indexed in step
fn stamp_ids_are_fixed_by_the_contract() {
    let ids: Vec<&str> = stamp::ALL.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [
            "workshop",
            "ford",
            "ice_gate",
            "coal_seam",
            "hollow",
            "rig",
            "shaft",
            "ledge",
            "kiln",
            "shortcut"
        ]
    );
    for s in stamp::ALL {
        for a in 0..3 {
            assert!(s.min[a] < s.max[a], "{} box is not empty on axis {a}", s.id);
            assert!(
                s.min[a] >= 0 && s.max[a] <= WORLD[a],
                "{} box is inside the world",
                s.id
            );
        }
    }
    assert_eq!(stamp::WORKSHOP.id, "workshop");
    assert_eq!(stamp::SHORTCUT.id, "shortcut");
}

#[test]
fn protected_boxes_are_a_world_list_not_a_cell_bit() {
    let mut w = World::new();
    assert!(w.can_break([5, 40, 5]) && w.can_place([5, 40, 5]));
    w.protect(stamp::Aabb {
        min: [0, 0, 0],
        max: [8, 96, 8],
    });
    assert!(!w.can_break([5, 40, 5]) && !w.can_place([5, 40, 5]));
    assert!(w.can_break([8, 40, 5]), "max is exclusive");
    // Writing cells does not change protection.
    w.set([5, 40, 5], Cell::new(mat::STONE));
    assert!(!w.can_break([5, 40, 5]));
    assert!(
        !w.can_break([-1, 40, 5]),
        "outside the world is never breakable"
    );
}

// ---------------------------------------------------------------------------
// Acceptance (section 11).
// ---------------------------------------------------------------------------

/// V1: `size_of::<Cell>() == 4`; level and state nibbles round-trip; temp 0
/// reads as 293.15 K.
#[test]
fn cell_packs_into_4_bytes() {
    assert_eq!(std::mem::size_of::<Cell>(), 4);
    let mut c = Cell::new(mat::WATER);
    assert_eq!(c.temp, 0);
    assert!(
        (c.temp_k() - 293.15).abs() < 1e-3,
        "temp 0 reads as ambient"
    );
    for level in 0..=8 {
        for state in 0..16u8 {
            c.set_level(level);
            c.set_state(state);
            assert_eq!(c.level(), level);
            assert_eq!(c.state(), state);
            assert_eq!(c.mat, mat::WATER);
        }
    }
    c.set_state(0);
    c.set_level(5);
    for f in [Cell::BURNING, Cell::SOURCE, Cell::SETTLED, Cell::LIT] {
        assert!(!c.has(f));
        c.set_flag(f, true);
        assert!(c.has(f));
        assert_eq!(c.level(), 5, "flags never touch the level");
    }
    c.set_flag(Cell::SOURCE, false);
    assert_eq!(c.state(), Cell::BURNING | Cell::SETTLED | Cell::LIT);
    assert_eq!(
        [Cell::BURNING, Cell::SOURCE, Cell::SETTLED, Cell::LIT],
        [1, 2, 4, 8]
    );
    c.set_temp_k(273.15);
    assert!((c.temp_k() - 273.15).abs() <= 0.05);
    assert!((c.temp_c() - 0.0).abs() <= 0.05);
    c.set_temp_k(77.0);
    assert!((c.temp_k() - 77.0).abs() <= 0.05);
    c.set_temp_k(5000.0);
    assert!(
        (c.temp_k() - 3276.75).abs() <= 0.05,
        "clamps at the top of the range"
    );
}

/// V2: `generate(S).hash() == generate(S).hash()`, and a different seed gives
/// a different hash outside the stamps but the same stamp cells.
#[test]
#[ignore = "Lane A (Sun night): body with generate()"]
fn generation_is_deterministic() {
    todo!("V2")
}

/// V3: the bench, Invention Bench, Kiln hearth, gas vents (6), rig and ledge
/// cells hold their materials at their section-4 coordinates; spawn has 2
/// air cells above a solid cell.
#[test]
#[ignore = "Lane A (Tue): body with the stamps"]
fn stamps_are_where_the_spec_says() {
    todo!("V3")
}

/// V4: `can_break` and `can_place` are false on every cell of each puzzle box
/// and of the edge ring, and true at 5 sample cells in the open.
#[test]
#[ignore = "Lane A (Tue): body with the stamps' protected boxes"]
fn protected_boxes_refuse_dig_and_place() {
    todo!("V4")
}

/// V5: a 32 x 1 x 32 slab of stone in an empty chunk gives exactly 6 quads;
/// two touching stone blocks give 10 quads (no inner faces); a water block
/// next to stone draws the stone's face, and stone does not draw a face
/// against stone; a 3D checkerboard gives at most 6 x 16384 quads.
#[test]
#[ignore = "Lane A (Mon): body with the greedy mesher"]
fn greedy_mesh_counts() {
    todo!("V5")
}

/// V6: a stone surface across the boundary of two chunks draws no faces on
/// the boundary.
#[test]
#[ignore = "Lane A (Mon): body with the greedy mesher"]
fn mesh_skips_hidden_faces_between_chunks() {
    todo!("V6")
}

/// V7: hits the right cell and normal for axis-aligned and diagonal rays, at
/// a chunk boundary, and returns `None` past `max_t`; `RayMode::Fluids` stops
/// at water while `Solids` passes through it.
#[test]
#[ignore = "Lane A (Mon): body with the DDA raycast"]
fn raycast_dda() {
    todo!("V7")
}

/// V8: at the spawn point the result includes the ground under the feet, has
/// at most 64 boxes, and includes ice cells only in `ice_boxes`.
#[test]
#[ignore = "Lane A (Mon): body with collision boxes"]
fn solid_boxes_around_player() {
    todo!("V8")
}

/// V9: random edits encode and decode to the same world; an unedited world
/// encodes to under 16 bytes.
#[test]
#[ignore = "Lane A (Mon): body with DiffLog"]
fn difflog_round_trips() {
    todo!("V9")
}

/// V10: every row has a `source` that exists as a heading key in
/// `content/science-sources.md`, and `melt_c < boil_c` where both are set.
#[test]
fn materials_table_is_sourced() {
    let todo = materials::todos(materials::MATERIALS_JSON).unwrap();
    assert!(todo.is_empty(), "values still \"todo\": {todo:?}");
    let register = include_str!("../../../content/science-sources.md");
    let headings: Vec<&str> = register
        .lines()
        .filter_map(|l| l.strip_prefix("### "))
        .map(str::trim)
        .collect();
    let rows = materials::load(materials::MATERIALS_JSON).unwrap();
    assert_eq!(rows.len(), mat::COUNT);
    for r in &rows {
        let src = r.source.as_deref().unwrap_or("");
        assert!(
            !src.is_empty() && headings.contains(&src),
            "{}: source {src:?} is not a heading in content/science-sources.md",
            r.name
        );
        if let (Some(m), Some(b)) = (r.melt_c.get(), r.boil_c.get()) {
            assert!(m < b, "{}: melt_c {m} is not below boil_c {b}", r.name);
        }
    }
}
