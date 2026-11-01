//! Stamps: hand-authored block templates placed after the noise terrain at
//! fixed positions (section 4), each with its protected box.
//!
//! The ids and boxes are fixed by the contract step (section 10). Lane A may
//! move a stamp by up to 4 blocks only if it updates the constant here and
//! all three golden sections still pass. Boxes are `min` inclusive, `max`
//! exclusive, in blocks.

/// An axis-aligned box of blocks: `min` inclusive, `max` exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Aabb {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

impl Aabb {
    /// True when block `p` is inside.
    pub fn contains(&self, p: [i32; 3]) -> bool {
        (0..3).all(|a| p[a] >= self.min[a] && p[a] < self.max[a])
    }
}

/// A named puzzle or landmark volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stamp {
    /// Stable id (state keys, `PlayerEvent::InBox`).
    pub id: &'static str,
    pub min: [i32; 3],
    pub max: [i32; 3],
}

impl Stamp {
    pub const fn aabb(&self) -> Aabb {
        Aabb {
            min: self.min,
            max: self.max,
        }
    }

    /// True when block `p` is inside the stamp's box.
    pub fn contains(&self, p: [i32; 3]) -> bool {
        self.aabb().contains(p)
    }
}

const fn s(id: &'static str, min: [i32; 3], max: [i32; 3]) -> Stamp {
    Stamp { id, min, max }
}

/// The burning workshop hut of planks and wood in the Village (P1).
pub const WORKSHOP: Stamp = s("workshop", [138, 34, 186], [146, 42, 194]);
/// The river ford: 3-deep water between stepped banks (P2a).
pub const FORD: Stamp = s("ford", [122, 26, 150], [134, 36, 158]);
/// The ice gate in the Frozen Fields' cliff band (P2b).
pub const ICE_GATE: Stamp = s("ice_gate", [58, 36, 86], [62, 48, 94]);
/// The coal seam behind the ice gate (coal ore at y 36-42).
pub const COAL_SEAM: Stamp = s("coal_seam", [40, 36, 60], [80, 43, 84]);
/// The methane hollow: a boardwalk ravine with 6 gas vents, floor y 27 (P3).
pub const HOLLOW: Stamp = s("hollow", [186, 26, 96], [197, 34, 106]);
/// The electrolysis rig room in the Crystal Cave (floor y 10).
pub const RIG: Stamp = s("rig", [226, 10, 24], [236, 16, 34]);
/// The 14 m balloon shaft (P4).
pub const SHAFT: Stamp = s("shaft", [238, 10, 18], [243, 25, 23]);
/// The ledge at the top of the shaft that holds the Kiln Spark (y 24).
pub const LEDGE: Stamp = s("ledge", [243, 24, 18], [246, 27, 23]);
/// The Great Kiln on its terrace: dome, chimney to y 60, runestone door,
/// hearth (P5).
pub const KILN: Stamp = s("kiln", [120, 40, 164], [136, 61, 174]);
/// The shortcut door in the cliff between the Marsh and the Village.
pub const SHORTCUT: Stamp = s("shortcut", [160, 34, 178], [166, 40, 182]);

/// Every stamp, in contract order.
pub const ALL: [Stamp; 10] = [
    WORKSHOP, FORD, ICE_GATE, COAL_SEAM, HOLLOW, RIG, SHAFT, LEDGE, KILN, SHORTCUT,
];

/// The stamp whose box holds block `p`, if any (first in `ALL` order).
pub fn at(p: [i32; 3]) -> Option<&'static Stamp> {
    ALL.iter().find(|s| s.contains(p))
}
