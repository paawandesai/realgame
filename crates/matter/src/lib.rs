//! Block chemistry on the voxel world (I-052, spec section 5.3-5.7): heat,
//! phase changes, fire, volume-conserving water, the coarse 2 m gas field
//! (CO2 sinks, H2 and CH4 rise), flame smothering, the glass and iron rules
//! and balloon lift. Pure Rust, no Bevy, deterministic: the same seed plus
//! the same emits give a bit-identical `hash` on native and wasm.
//!
//! **Determinism rules** (section 3): fixed iteration order (chunk index,
//! then cell index); no `HashMap` iteration in a step; no `sin`, `exp` or
//! `powf` inside a step (tables and `+ - * /` only); the seeded xorshift RNG
//! lives in `World`.
//!
//! **Contract rules** (section 10): cells are read and written only through
//! `realgame_voxel::World::{get, set}`; this crate never meshes. Reaction
//! ids and gas densities come from `realgame_sim` (`reactions`, `species`).
//!
//! Contract step: the types and signatures are fixed here; `step` and
//! `lift_n` are stubs that Lane B fills (spec section 12).

#![deny(unsafe_code)]

use realgame_voxel as voxel;

/// Fixed step: 10 steps per second.
pub const DT: f32 = 0.1;
/// A Water or Frost hit knocks a gas vent's flame down for this long, then it
/// relights (the value from the client's `chem.rs`).
pub const FLICKER_S: f32 = 1.5;

/// What a landed spell adds to the world (section 5.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmitKind {
    /// Liquid water: 8 liquid levels at the landing cell (24 per cast at most).
    Water,
    /// Liquid nitrogen (−196 °C): cools a 1.5-block radius by a set energy.
    Frost,
    /// Methane flame: heats a 1-block radius to 1100 °C for 0.5 s and adds
    /// CO2 and H2O to the gas cell.
    Fire,
    /// CO2 foam: 1 m³ of cold CO2 at the hit, which sinks.
    Co2,
}

/// One landed spell, built by the client's `blockchem::on_impact`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emit {
    /// Hit point in world metres.
    pub pos: [f32; 3],
    /// The face that was hit (unit axis vector).
    pub normal: [i8; 3],
    pub what: EmitKind,
    /// Scale of the cast (1.0 for one ordinary cast).
    pub amount: f32,
}

/// What put a fire out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum By {
    Water,
    Frost,
    /// Its gas cell's O2 fell below 15 %.
    NoOxygen,
}

/// Something the matter step did, for captions, sounds and the quest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatterEvent {
    Melted { cell: [i32; 3] },
    Froze { cell: [i32; 3] },
    Ignited { cell: [i32; 3] },
    Extinguished { cell: [i32; 3], by: By },
    Smothered { vent: [i32; 3] },
    Relit { vent: [i32; 3] },
    Glass { cell: [i32; 3] },
    Iron { cell: [i32; 3] },
    Leidenfrost { pos: [f32; 3] },
    Flowed { chunk: usize },
}

/// The gases in the coarse field (they are not blocks).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gas {
    Co2,
    H2,
    Ch4,
    O2,
}

/// The matter simulation's own state: the gas field, pending emits, the
/// active-chunk set, carry-over work and the RNG. The blocks themselves live
/// in `realgame_voxel::World`.
#[derive(Clone, Debug)]
pub struct World {
    rng: u64,
    ticks: u64,
    pending: Vec<Emit>,
}

impl World {
    /// A matter world with no gas and nothing active, seeded for its RNG.
    pub fn new(seed: u64) -> World {
        World {
            // xorshift must not start at 0.
            rng: seed ^ 0x9e37_79b9_7f4a_7c15,
            ticks: 0,
            pending: Vec::new(),
        }
    }

    /// Queues a landed spell; it enters the world on the next `step`.
    pub fn emit(&mut self, e: Emit) {
        self.pending.push(e);
    }

    /// One fixed step of `DT` on the active chunks (heat, phase, fire, water,
    /// gas, in that order, each with its cap), appending what happened.
    ///
    /// Stub (contract step): consumes pending emits and does nothing else.
    pub fn step(&mut self, vox: &mut voxel::World, events: &mut Vec<MatterEvent>) {
        let _ = (vox, events);
        self.pending.clear();
        self.ticks += 1;
    }

    /// Steps taken so far.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// The test hook (M10): a hash of the blocks and the matter state. Equal
    /// on native and wasm for the same seed and emits.
    pub fn hash(&self, vox: &voxel::World) -> u64 {
        let mut h = vox.hash();
        for v in [self.rng, self.pending.len() as u64] {
            h ^= v;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
    }
}

/// Net upward force in newtons on a balloon of `volume_m3` filled with
/// `gas` carrying `payload_kg`, in air at 20 °C (section 5.7):
/// `(ρ_air − ρ_gas) · V · g − payload · g`.
///
/// Stub (contract step): returns 0. Lane B fills it (M9).
pub fn lift_n(gas: Gas, volume_m3: f32, payload_kg: f32) -> f32 {
    let _ = (gas, volume_m3, payload_kg);
    0.0
}
