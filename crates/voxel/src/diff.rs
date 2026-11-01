//! The save format (section 5.8): per chunk, a sorted list of the cells that
//! differ from the seed's generated chunk. `encode` writes a version byte,
//! then run-length data; the format is fixed now so a save never needs a
//! migration. Storing it (IndexedDB, OPFS) is a Should.
//!
//! Byte layout (little-endian), version 1:
//!
//! ```text
//! u8  version (1)
//! u16 chunk count
//! per chunk, in index order:
//!   u16 chunk index
//!   u16 run count
//!   per run, in cell order: u16 first cell index, u16 length, then the
//!   cell (u8 mat, u8 level_state, u16 temp), the same for every cell in
//!   the run
//! ```

use crate::{Cell, World, CHUNK_CELLS};

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
    /// A chunk or cell index out of range, or out of order.
    Invalid,
}

impl DiffLog {
    /// The cells of `world` that differ from `base` (the same seed's
    /// `generate`).
    pub fn between(base: &World, world: &World) -> DiffLog {
        let mut out = DiffLog::default();
        for ci in 0..world.chunk_count() {
            let (a, b) = (base.chunk(ci), world.chunk(ci));
            if a == b {
                continue;
            }
            let o = World::chunk_origin(ci);
            let mut cells = Vec::new();
            for i in 0..CHUNK_CELLS {
                let p = cell_pos(o, i);
                let c = world.get(p);
                if c != base.get(p) {
                    cells.push((i as u16, c));
                }
            }
            if !cells.is_empty() {
                out.chunks.push((ci as u16, cells));
            }
        }
        out
    }

    /// Writes every edit into `world` (a fresh `generate` of the same seed).
    pub fn apply(&self, world: &mut World) {
        for (ci, cells) in &self.chunks {
            if *ci as usize >= world.chunk_count() {
                continue;
            }
            let o = World::chunk_origin(*ci as usize);
            for &(i, c) in cells {
                world.set(cell_pos(o, i as usize), c);
            }
        }
    }

    /// Version byte, then run-length data.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![VERSION];
        out.extend((self.chunks.len() as u16).to_le_bytes());
        for (ci, cells) in &self.chunks {
            let mut runs: Vec<(u16, u16, Cell)> = Vec::new();
            for &(i, c) in cells {
                match runs.last_mut() {
                    Some((start, len, rc))
                        if *rc == c && *start as u32 + *len as u32 == i as u32 =>
                    {
                        *len += 1;
                    }
                    _ => runs.push((i, 1, c)),
                }
            }
            out.extend(ci.to_le_bytes());
            out.extend((runs.len() as u16).to_le_bytes());
            for (start, len, c) in runs {
                out.extend(start.to_le_bytes());
                out.extend(len.to_le_bytes());
                out.push(c.mat);
                out.push(c.level_state);
                out.extend(c.temp.to_le_bytes());
            }
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<DiffLog, DecodeError> {
        match bytes.first() {
            None => return Err(DecodeError::Empty),
            Some(&VERSION) => {}
            Some(&v) => return Err(DecodeError::Version(v)),
        }
        let mut r = Reader { b: bytes, at: 1 };
        let n = r.u16()?;
        let chunk_count = (crate::CHUNKS[0] * crate::CHUNKS[1] * crate::CHUNKS[2]) as u16;
        let mut out = DiffLog::default();
        let mut last_chunk: Option<u16> = None;
        for _ in 0..n {
            let ci = r.u16()?;
            if ci >= chunk_count || last_chunk.is_some_and(|l| ci <= l) {
                return Err(DecodeError::Invalid);
            }
            last_chunk = Some(ci);
            let runs = r.u16()?;
            let mut cells = Vec::new();
            let mut next_free = 0u32;
            for _ in 0..runs {
                let start = r.u16()? as u32;
                let len = r.u16()? as u32;
                let c = Cell {
                    mat: r.u8()?,
                    level_state: r.u8()?,
                    temp: r.u16()?,
                };
                if len == 0 || start < next_free || start + len > CHUNK_CELLS as u32 {
                    return Err(DecodeError::Invalid);
                }
                cells.extend((start..start + len).map(|i| (i as u16, c)));
                next_free = start + len;
            }
            out.chunks.push((ci, cells));
        }
        if r.at != bytes.len() {
            return Err(DecodeError::Invalid);
        }
        Ok(out)
    }
}

fn cell_pos(o: [i32; 3], i: usize) -> [i32; 3] {
    let c = crate::CHUNK as usize;
    [
        o[0] + (i % c) as i32,
        o[1] + (i / c % c) as i32,
        o[2] + (i / (c * c)) as i32,
    ]
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Result<u8, DecodeError> {
        let v = *self.b.get(self.at).ok_or(DecodeError::Truncated)?;
        self.at += 1;
        Ok(v)
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes([self.u8()?, self.u8()?]))
    }
}
