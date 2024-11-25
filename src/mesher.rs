use std::collections::HashMap;

use crate::{
    asset_loader::{get_texture_index, Face},
    chunk::*,
    model::MeshVertex,
};

use glam::*;

const FACE_NORMALS: [(Face, [isize; 3]); 6] = [
    (Face::Up, [0, 1, 0]),    // up
    (Face::Down, [0, -1, 0]), // down
    (Face::Side, [1, 0, 0]),  // right
    (Face::Side, [-1, 0, 0]), // left
    (Face::Side, [0, 0, 1]),  // front
    (Face::Side, [0, 0, -1]), // back
];

const FACE_VERTICES: [[[f32; 3]; 4]; 6] = [
    // up
    [
        [-0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, -0.5],
    ],
    // down
    [
        [-0.5, -0.5, 0.5],
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
    ],
    // right
    [
        [0.5, 0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
    ],
    // left
    [
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ],
    // front
    [
        [-0.5, 0.5, 0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
    ],
    // back
    [
        [0.5, 0.5, -0.5],
        [0.5, -0.5, -0.5],
        [-0.5, -0.5, -0.5],
        [-0.5, 0.5, -0.5],
    ],
];

const FACE_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

/// Generates a mesh for a given chunk.
#[derive(Debug)]
pub struct ChunkMesher<'a> {
    /// The chunk whose mesh is being built.
    chunk: &'a crate::chunk::Chunk,
    /// A list of the chunks surrounding the chunk.
    chunks: &'a HashMap<glam::IVec2, crate::chunk::Chunk>,

    /// The vertices generated so far.
    vertices: Vec<MeshVertex>,
    /// The indices generated so far.
    indices: Vec<u32>,
}

impl<'c> ChunkMesher<'c> {
    /// Creates a new chunk mesh builder given a chunk.
    pub fn new(chunks: &'c HashMap<IVec2, Chunk>, chunk: IVec2) -> Self {
        Self {
            chunk: chunks
                .get(&chunk)
                .expect("cannot build mesh for unloaded chunk"),
            chunks,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Builds the vertices and indices for the chunk.
    pub fn build(mut self) -> (Vec<MeshVertex>, Vec<u32>) {
        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_WIDTH {
                for x in 0..CHUNK_WIDTH {
                    self.add_block([x, y, z]);
                }
            }
        }

        (self.vertices, self.indices)
    }

    /// Returns whether the given voxel position (in world space) is solid.
    fn is_solid(&self, [x, y, z]: [isize; 3]) -> bool {
        let position = ivec2(
            x.div_euclid(CHUNK_WIDTH as isize) as i32,
            z.div_euclid(CHUNK_WIDTH as isize) as i32,
        );

        let Some(chunk) = self.chunks.get(&position) else {
            // the neighbor chunk hasn't been loaded yet.
            return false;
        };

        chunk.is_block_full(match chunk.get_local_position([x, y, z]) {
            Some(pos) => pos,
            None => return false,
        })
    }

    fn add_block(&mut self, block_pos: [usize; 3]) {
        if !self.chunk.is_block_full(block_pos) {
            return;
        }

        let [x, y, z] = block_pos;
        let voxel = self.chunk.voxels[y][z][x];

        let chunk_x_offset = self.chunk.position.x as f32 * CHUNK_WIDTH as f32;
        let chunk_z_offset = self.chunk.position.y as f32 * CHUNK_WIDTH as f32;

        for (index, (face, face_normal)) in FACE_NORMALS.iter().enumerate() {
            if self.is_solid(self.chunk.get_world_position(block_pos, *face_normal)) {
                continue;
            }

            let [fx, fy, fz] = *face_normal;
            let normal = [fx as f32, fy as f32, fz as f32];

            let vertices = FACE_VERTICES[index];

            for position in vertices {
                // the local position offset of the vertex relative
                // to its center
                let [lx, ly, lz] = position;

                let pos = [
                    x as f32 + lx + chunk_x_offset,
                    y as f32 + ly,
                    z as f32 + lz + chunk_z_offset,
                ];

                self.vertices.push(MeshVertex {
                    pos,
                    normal,
                    texture_index: get_texture_index(&voxel, face).unwrap_or_else(|| {
                        panic!("could not find texture for '{voxel:?}' (face: '{face:?}')")
                    }),
                });
            }

            let offset = self
                .indices
                .get(self.indices.len().saturating_sub(2))
                .copied()
                .map(|i| i + 1)
                .unwrap_or(0);

            self.indices.extend(FACE_INDICES.map(|i| i + offset));
        }
    }
}
