//! I-052 acceptance tests for the voxel crate (spec section 11, V1-V10),
//! plus the contract-step checks that pin the shared API (section 10).
//!
//! Tests marked `#[ignore]` have their body written by the lane that owns
//! the feature (spec section 12). Each ignored body is `todo!()`, so taking
//! the `#[ignore]` off without writing the body fails.

use realgame_voxel::{
    generate, ice_boxes, mat, materials, mesh_chunk, raycast, solid_boxes, stamp, Cell, DiffLog,
    MeshData, RayMode, World, CHUNK, CHUNKS, DEFAULT_SEED, SPAWN, WORLD,
};

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
fn generation_is_deterministic() {
    let a = generate(DEFAULT_SEED);
    let b = generate(DEFAULT_SEED);
    assert_eq!(a.hash(), b.hash(), "the same seed gives the same world");
    assert_eq!(a.protected(), b.protected());
    let c = generate(DEFAULT_SEED ^ 0x0bad_5eed);
    assert_ne!(
        a.hash(),
        c.hash(),
        "a different seed gives a different world"
    );
    assert_eq!(
        a.protected(),
        c.protected(),
        "protection does not depend on the seed"
    );
    // The difference is outside the stamps: every stamp cell is the same.
    let mut outside = 0usize;
    for z in 0..WORLD[2] {
        for x in 0..WORLD[0] {
            for y in 0..WORLD[1] {
                let p = [x, y, z];
                if a.get(p) == c.get(p) {
                    continue;
                }
                assert!(
                    stamp::at(p).is_none(),
                    "stamp cell {p:?} ({}) differs between seeds",
                    stamp::at(p).unwrap().id
                );
                outside += 1;
            }
        }
    }
    assert!(outside > 1000, "the seeds differ in only {outside} cells");
    // Every stamp writes something: no stamp box is all air.
    for s in stamp::ALL {
        let mut solid = 0;
        for z in s.min[2]..s.max[2] {
            for y in s.min[1]..s.max[1] {
                for x in s.min[0]..s.max[0] {
                    solid += usize::from(a.get([x, y, z]).mat != mat::AIR);
                }
            }
        }
        assert!(solid > 0, "{} is empty", s.id);
    }
    assert_eq!(a.seed, DEFAULT_SEED);
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
fn greedy_mesh_counts() {
    // A 32 x 1 x 32 slab of stone in an empty chunk: exactly 6 quads.
    let mut w = World::new();
    for z in 0..32 {
        for x in 0..32 {
            w.set([x, 40, z], Cell::new(mat::STONE));
        }
    }
    let c = World::chunk_index([0, 40, 0]).unwrap();
    let m = mesh_chunk(&w, c);
    assert_eq!(m.opaque.quads() + m.clear.quads(), 6, "slab");
    assert_eq!(area(&m.opaque), 2.0 * 32.0 * 32.0 + 4.0 * 32.0);
    check_streams(&m.opaque);

    // Two touching stone blocks: no inner faces. The spec says "10 quads",
    // which is the count before greedy merging (and contradicts the slab
    // clause above, where merging is required); the merged mesh covers
    // exactly the 10 outer unit faces and no inner face (lane A notes).
    let mut w = World::new();
    w.set([4, 40, 4], Cell::new(mat::STONE));
    w.set([5, 40, 4], Cell::new(mat::STONE));
    let m = mesh_chunk(&w, c);
    assert_eq!(area(&m.opaque), 10.0, "10 outer unit faces");
    assert!(m.opaque.quads() <= 10);
    assert_eq!(
        faces_on_plane(&m.opaque, 0, 5.0),
        0.0,
        "no face between the two blocks"
    );

    // A water block next to stone draws the stone's face; stone does not
    // draw a face against stone.
    let mut w = World::new();
    w.set([8, 40, 8], Cell::new(mat::STONE));
    let mut water = Cell::new(mat::WATER);
    water.set_level(8);
    w.set([9, 40, 8], water);
    w.set([7, 40, 8], Cell::new(mat::STONE));
    let m = mesh_chunk(&w, c);
    assert_eq!(
        faces_on_plane(&m.opaque, 0, 9.0),
        1.0,
        "the stone face toward the water is drawn"
    );
    assert_eq!(
        faces_on_plane(&m.clear, 0, 9.0),
        0.0,
        "the water face against the stone is not drawn"
    );
    assert_eq!(
        faces_on_plane(&m.opaque, 0, 8.0),
        0.0,
        "no face between the two stones"
    );
    assert_eq!(m.clear.quads(), 5, "the water block's other 5 faces");

    // A 3D checkerboard: at most 6 x 16384 quads.
    let mut w = World::new();
    for z in 0..32 {
        for y in 32..64 {
            for x in 0..32 {
                if (x + y + z) % 2 == 0 {
                    w.set([x, y, z], Cell::new(mat::STONE));
                }
            }
        }
    }
    let m = mesh_chunk(&w, c);
    assert!(m.opaque.quads() <= 6 * 16384, "{}", m.opaque.quads());
    assert_eq!(area(&m.opaque), 6.0 * 16384.0);
}

/// V6: a stone surface across the boundary of two chunks draws no faces on
/// the boundary.
#[test]
fn mesh_skips_hidden_faces_between_chunks() {
    // A stone slab from x 16 to 47 crosses the boundary at x = 32.
    let mut w = World::new();
    for z in 0..32 {
        for x in 16..48 {
            w.set([x, 40, z], Cell::new(mat::STONE));
        }
    }
    let a = mesh_chunk(&w, World::chunk_index([0, 40, 0]).unwrap());
    let b = mesh_chunk(&w, World::chunk_index([32, 40, 0]).unwrap());
    for m in [&a, &b] {
        assert_eq!(faces_on_plane(&m.opaque, 0, 32.0), 0.0, "boundary face");
        assert!(m.opaque.quads() > 0);
    }
    // Each chunk draws its own half of the top and bottom.
    assert_eq!(area(&a.opaque), 2.0 * 16.0 * 32.0 + 32.0 + 2.0 * 16.0);
    assert_eq!(area(&b.opaque), 2.0 * 16.0 * 32.0 + 32.0 + 2.0 * 16.0);
}

/// V7: hits the right cell and normal for axis-aligned and diagonal rays, at
/// a chunk boundary, and returns `None` past `max_t`; `RayMode::Fluids` stops
/// at water while `Solids` passes through it.
#[test]
fn raycast_dda() {
    let mut w = World::new();
    w.set([10, 40, 10], Cell::new(mat::STONE));
    w.set([20, 40, 20], Cell::new(mat::STONE));
    w.set([32, 40, 5], Cell::new(mat::STONE));
    let mut water = Cell::new(mat::WATER);
    water.set_level(8);
    w.set([5, 40, 10], water);
    w.set([5, 40, 13], Cell::new(mat::STONE));
    let s = RayMode::Solids;

    // Axis-aligned, +z.
    let h = raycast(&w, [10.5, 40.5, 5.5], [0.0, 0.0, 1.0], 20.0, s).unwrap();
    assert_eq!((h.cell, h.normal), ([10, 40, 10], [0, 0, -1]));
    assert!((h.t - 4.5).abs() < 1e-4, "t {}", h.t);
    // Axis-aligned, straight down onto the top face.
    let h = raycast(&w, [10.5, 45.5, 10.5], [0.0, -1.0, 0.0], 20.0, s).unwrap();
    assert_eq!((h.cell, h.normal), ([10, 40, 10], [0, 1, 0]));
    assert!((h.t - 4.5).abs() < 1e-4);
    // A direction that is not unit length is normalised.
    let h = raycast(&w, [10.5, 40.5, 15.5], [0.0, 0.0, -3.0], 20.0, s).unwrap();
    assert_eq!((h.cell, h.normal), ([10, 40, 10], [0, 0, 1]));
    assert!((h.t - 4.5).abs() < 1e-4);
    // Diagonal: crosses z = 20 first (an air cell), then enters through -x.
    let d = std::f32::consts::FRAC_1_SQRT_2;
    let h = raycast(&w, [17.2, 40.5, 17.9], [d, 0.0, d], 20.0, s).unwrap();
    assert_eq!((h.cell, h.normal), ([20, 40, 20], [-1, 0, 0]));
    assert!(
        (h.t - 2.8 * std::f32::consts::SQRT_2).abs() < 1e-3,
        "t {}",
        h.t
    );
    // At a chunk boundary (x = 32).
    let h = raycast(&w, [28.5, 40.5, 5.5], [1.0, 0.0, 0.0], 20.0, s).unwrap();
    assert_eq!((h.cell, h.normal), ([32, 40, 5], [-1, 0, 0]));
    assert!((h.t - 3.5).abs() < 1e-4);
    // Nothing past max_t.
    assert_eq!(
        raycast(&w, [28.5, 40.5, 5.5], [1.0, 0.0, 0.0], 3.0, s),
        None
    );
    // Nothing at all along an empty ray, and nothing outside the world.
    assert_eq!(
        raycast(&w, [100.5, 40.5, 100.5], [0.0, 1.0, 0.0], 50.0, s),
        None
    );
    assert_eq!(
        raycast(&w, [-5.0, 40.5, 10.5], [-1.0, 0.0, 0.0], 50.0, s),
        None
    );
    // A ray starting outside the world still finds blocks inside it.
    let h = raycast(&w, [10.5, 40.5, -3.5], [0.0, 0.0, 1.0], 30.0, s).unwrap();
    assert_eq!(h.cell, [10, 40, 10]);
    // Fluids stops at water; Solids passes through it.
    let h = raycast(&w, [5.5, 40.5, 7.5], [0.0, 0.0, 1.0], 20.0, s).unwrap();
    assert_eq!(h.cell, [5, 40, 13]);
    let h = raycast(&w, [5.5, 40.5, 7.5], [0.0, 0.0, 1.0], 20.0, RayMode::Fluids).unwrap();
    assert_eq!((h.cell, h.normal), ([5, 40, 10], [0, 0, -1]));
}

/// V8: at the spawn point the result includes the ground under the feet, has
/// at most 64 boxes, and includes ice cells only in `ice_boxes`.
#[test]
fn solid_boxes_around_player() {
    let mut w = generate(DEFAULT_SEED);
    let [x, y, z] = SPAWN;
    // The player's box (0.35 m radius, 1.75 m tall) swept by up to 0.6 m.
    let r = 0.35 + 0.6;
    let aabb = [x - r, y - 0.6, z - r, x + r, y + 1.75 + 0.6, z + r];
    let boxes = solid_boxes(&w, aabb);
    assert!(boxes.len() <= 64, "{} boxes", boxes.len());
    assert!(
        boxes.iter().any(|b| (b[4] - y).abs() < 1e-6
            && b[0] <= x
            && x <= b[3]
            && b[2] <= z
            && z <= b[5]),
        "the ground under the feet: {boxes:?}"
    );
    for b in &boxes {
        assert!(b[0] < aabb[3] && b[3] > aabb[0] && b[1] < aabb[4] && b[4] > aabb[1]);
        assert!(b[2] < aabb[5] && b[5] > aabb[2]);
    }
    assert!(ice_boxes(&w, aabb).is_empty(), "no ice at spawn");
    // An ice block next to the player is in ice_boxes only.
    let ice = [x.floor() as i32 - 1, y as i32, z.floor() as i32];
    w.set(ice, Cell::new(mat::ICE));
    let fx = ice.map(|v| v as f32);
    let want = [fx[0], fx[1], fx[2], fx[0] + 1.0, fx[1] + 1.0, fx[2] + 1.0];
    assert_eq!(ice_boxes(&w, aabb), vec![want]);
    assert!(!solid_boxes(&w, aabb).contains(&want));
    // Water is never solid.
    let mut water = Cell::new(mat::WATER);
    water.set_level(8);
    w.set(ice, water);
    assert!(ice_boxes(&w, aabb).is_empty());
    assert!(!solid_boxes(&w, aabb).contains(&want));
}

/// V9: random edits encode and decode to the same world; an unedited world
/// encodes to under 16 bytes.
#[test]
fn difflog_round_trips() {
    let base = generate(DEFAULT_SEED);
    assert!(
        DiffLog::between(&base, &base).encode().len() < 16,
        "an unedited world encodes to under 16 bytes"
    );
    let mut w = base.clone();
    let mut rng: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    for k in 0..2000 {
        let r = next();
        let p = [
            (r % 256) as i32,
            ((r >> 8) % 96) as i32,
            ((r >> 16) % 256) as i32,
        ];
        let mut c = Cell::new(((r >> 24) % mat::COUNT as u64) as u8);
        c.set_level(((r >> 32) % 9) as u8);
        c.set_state(((r >> 36) % 16) as u8);
        if k % 3 == 0 {
            c.set_temp_k(200.0 + ((r >> 40) % 1000) as f32);
        }
        w.set(p, c);
        // Runs: a few rows of the same cell.
        if k % 50 == 0 {
            for dx in 0..20 {
                w.set([dx, 60, p[2]], Cell::new(mat::GLASS));
            }
        }
    }
    let d = DiffLog::between(&base, &w);
    assert!(!d.chunks.is_empty());
    let bytes = d.encode();
    assert_eq!(bytes[0], realgame_voxel::diff::VERSION);
    let back = DiffLog::decode(&bytes).expect("decodes");
    assert_eq!(back, d);
    let mut w2 = base.clone();
    back.apply(&mut w2);
    assert_eq!(w2.hash(), w.hash(), "the decoded edits rebuild the world");
    // A cell set back to its generated value is not an edit.
    let mut w3 = base.clone();
    let p = [100, 50, 100];
    w3.set(p, Cell::new(mat::GLASS));
    w3.set(p, base.get(p));
    assert!(DiffLog::between(&base, &w3).chunks.is_empty());
    // Bad input is refused, never misread.
    assert!(DiffLog::decode(&[]).is_err());
    assert!(DiffLog::decode(&[99, 0, 0]).is_err());
    assert!(DiffLog::decode(&bytes[..bytes.len() - 1]).is_err());
}

/// V10: every row has a `source` that exists as a heading key in
/// `content/science-sources.md`, and `melt_c < boil_c` where both are set.
#[test]
#[ignore = "Lane B (Tue): un-ignored when the material values are sourced"]
fn materials_table_is_sourced() {
    todo!("V10")
}

// ---------------------------------------------------------------------------
// Helpers for the mesh tests.
// ---------------------------------------------------------------------------

/// Total face area of a mesh, in unit block faces (each quad is 4 vertices).
fn area(m: &MeshData) -> f32 {
    m.positions
        .chunks(4)
        .map(|q| {
            let e1 = sub(q[1], q[0]);
            let e2 = sub(q[3], q[0]);
            let c = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt()
        })
        .sum()
}

/// Face area lying in the plane `axis = at`.
fn faces_on_plane(m: &MeshData, axis: usize, at: f32) -> f32 {
    let mut sum = 0.0;
    for (k, q) in m.positions.chunks(4).enumerate() {
        if q.iter().all(|v| (v[axis] - at).abs() < 1e-5) {
            let one = MeshData {
                positions: q.to_vec(),
                ..MeshData::default()
            };
            sum += area(&one);
            let n = m.normals[k * 4];
            assert!(
                (n[axis].abs() - 1.0).abs() < 1e-6,
                "a face in the plane faces the axis"
            );
        }
    }
    sum
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Every stream has one entry per vertex and every index is in range.
fn check_streams(m: &MeshData) {
    let n = m.positions.len();
    assert_eq!(n % 4, 0);
    assert_eq!(m.normals.len(), n);
    assert_eq!(m.uvs.len(), n);
    assert_eq!(m.colors.len(), n);
    assert_eq!(m.indices.len(), n / 4 * 6);
    assert!(m.indices.iter().all(|&i| (i as usize) < n));
}
