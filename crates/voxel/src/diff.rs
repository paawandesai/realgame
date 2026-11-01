//! The save format (section 5.8): per chunk, a sorted list of the cells that
//! differ from the seed's generated chunk. `encode` writes a version byte,
//! then run-length data; the format is fixed now so a save never needs a
//! migration. Storing it (IndexedDB, OPFS) is a Should.
//!
//! Stub (contract step): `between`, `apply`, `encode` and `decode` keep
//! the types; Lane A fills the bodies (V9).

use crate::{Cell, World};

/// Format version written as the first byte of `encode`.
pub const VERSION: u8 = 1;

/// Edits against a generated world: `(chunk index, sorted (cell index, cell))`,
/// chunks in index order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffLog {
    pub chunks: Vec<(u16, Vec<(u16, Cell)>)>,
}

/// Why `decode` refused its input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    Version(u8),
    Truncated,
}

impl DiffLog {
    /// The cells of `world` that differ from `base` (the same seed's
    /// `generate`).
    pub fn between(base: &World, world: &World) -> DiffLog {
        let _ = (base, world);
        DiffLog::default()
    }

    /// Writes every edit into `world` (a fresh `generate` of the same seed).
    pub fn apply(&self, world: &mut World) {
        let _ = world;
    }

    /// Version byte, then run-length data.
    pub fn encode(&self) -> Vec<u8> {
        vec![VERSION]
    }

    pub fn decode(bytes: &[u8]) -> Result<DiffLog, DecodeError> {
        match bytes.first() {
            None => Err(DecodeError::Empty),
            Some(&VERSION) => Ok(DiffLog::default()),
            Some(&v) => Err(DecodeError::Version(v)),
        }
    }
}
