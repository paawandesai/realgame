//! The greedy mesher (section 5.8): per face direction over 32 x 32 slices,
//! per-vertex AO by the 3-neighbour rule, water tops lowered by level.
//!
//! Plain vectors, no Bevy types, so a server or the VR client can reuse it.
//! Stub (contract step): returns empty meshes. Lane A fills it (V5, V6).

use crate::World;

/// One mesh's vertex streams and indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// AO and tint, linear RGBA.
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Quads in the mesh (4 vertices, 6 indices each).
    pub fn quads(&self) -> usize {
        self.indices.len() / 6
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// A chunk's two passes: `opaque` (alpha-tested atlas) and `clear` (water,
/// ice, glass and crystal, alpha blend).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkMesh {
    pub opaque: MeshData,
    pub clear: MeshData,
}

/// Meshes chunk `chunk` (an index from `World::chunk_index`). Positions are
/// in world blocks.
pub fn mesh_chunk(world: &World, chunk: usize) -> ChunkMesh {
    let _ = (world, chunk);
    ChunkMesh::default()
}
