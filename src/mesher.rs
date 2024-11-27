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

const FACE_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

const FACE_VERTICES: [[glam::Vec3; 4]; 6] = [
    // up
    [
        vec3(-0.5, 0.5, -0.5),
        vec3(-0.5, 0.5, 0.5),
        vec3(0.5, 0.5, 0.5),
        vec3(0.5, 0.5, -0.5),
    ],
    // down
    [
        vec3(-0.5, -0.5, 0.5),
        vec3(-0.5, -0.5, -0.5),
        vec3(0.5, -0.5, -0.5),
        vec3(0.5, -0.5, 0.5),
    ],
    // right
    [
        vec3(0.5, 0.5, 0.5),
        vec3(0.5, -0.5, 0.5),
        vec3(0.5, -0.5, -0.5),
        vec3(0.5, 0.5, -0.5),
    ],
    // left
    [
        vec3(-0.5, 0.5, -0.5),
        vec3(-0.5, -0.5, -0.5),
        vec3(-0.5, -0.5, 0.5),
        vec3(-0.5, 0.5, 0.5),
    ],
    // front
    [
        vec3(-0.5, 0.5, 0.5),
        vec3(-0.5, -0.5, 0.5),
        vec3(0.5, -0.5, 0.5),
        vec3(0.5, 0.5, 0.5),
    ],
    // back
    [
        vec3(0.5, 0.5, -0.5),
        vec3(0.5, -0.5, -0.5),
        vec3(-0.5, -0.5, -0.5),
        vec3(-0.5, 0.5, -0.5),
    ],
];

#[rustfmt::skip]
const AMBIENT_NEIGHBOR_OFFSETS_OLD: [[[isize; 3]; 8]; 6] = [
    // top
    [
        [ -1,  1,  0 ], // left edge           0
        [ -1,  1, -1 ], // back left corner    1
        [  0,  1, -1 ], // back edge           2
        [  1,  1, -1 ], // back right corner   3
        [  1,  1,  0 ], // right edge          4
        [  1,  1,  1 ], // front right corner  5
        [  0,  1,  1 ], // front edge          6
        [ -1,  1,  1 ], // front left corner   7
    ],
    // bottom
    [
        [ -1, -1,  0 ], // left edge          8
        [ -1, -1, -1 ], // back left corner   9
        [  0, -1, -1 ], // back edge          10
        [  1, -1, -1 ], // back right corner  11
        [  1, -1,  0 ], // right edge         12
        [  1, -1,  1 ], // front right corner 13
        [  0, -1,  1 ], // front edge         14
        [ -1, -1,  1 ], // front left corner  15
    ],
    // right
    [
        [  1,  0,  1 ],
        [  1,  1,  1 ],
        [  1,  1,  0 ],
        [  1,  1, -1 ],
        [  1,  0, -1 ],
        [  1, -1, -1 ],
        [  1, -1,  0 ],
        [  1, -1,  1 ],
    ],
    // left
    [
        [ -1,  0, -1 ],
        [ -1,  1, -1 ],
        [ -1,  1,  0 ],
        [ -1,  1,  1 ],
        [ -1,  0,  1 ],
        [ -1, -1,  1 ],
        [ -1, -1,  0 ],
        [ -1, -1, -1 ],
    ],
    // front
    [
        [ -1,  0,  1 ],
        [ -1,  1,  1 ],
        [  0,  1,  1 ],
        [  1,  1,  1 ],
        [  1,  0,  1 ],
        [  1, -1,  1 ],
        [  0, -1,  1 ],
        [ -1, -1,  1 ],

    ],
    // back
    [
        [  1,  0, -1 ],
        [  1,  1, -1 ],
        [  0,  1, -1 ],
        [ -1,  1, -1 ],
        [ -1,  0, -1 ],
        [ -1, -1, -1 ],
        [  0, -1, -1 ],
        [  1, -1, -1 ],
    ]    
];

#[rustfmt::skip]
const AMBIENT_NEIGHBOR_OFFSETS: [[[isize; 3]; 8]; 6] = [
    // top
    [
        [  0,  1, -1 ], // back edge           2
        [ -1,  1, -1 ], // back left corner    1
        [ -1,  1,  0 ], // left edge           0
        [ -1,  1,  1 ], // front left corner   7
        [  0,  1,  1 ], // front edge          6
        [  1,  1,  1 ], // front right corner  5
        [  1,  1,  0 ], // right edge          4
        [  1,  1, -1 ], // back right corner   3
    ],
    // bottom
    [
        [  0, -1, -1 ],
        [ -1, -1, -1 ],
        [ -1, -1,  0 ],
        [ -1, -1,  1 ],
        [  0, -1,  1 ],
        [  1, -1,  1 ],
        [  1, -1,  0 ],
        [  1, -1, -1 ],
    ],
    // right
    [
        [  1,  1,  0 ],
        [  1,  1,  1 ],
        [  1,  0,  1 ],
        [  1, -1,  1 ],
        [  1, -1,  0 ],
        [  1, -1, -1 ],
        [  1,  0, -1 ],
        [  1,  1, -1 ],
    ],
    // left
    [
        [ -1,  1,  0 ],
        [ -1,  1, -1 ],
        [ -1,  0, -1 ],
        [ -1, -1, -1 ],
        [ -1, -1,  0 ],
        [ -1, -1,  1 ],
        [ -1,  0,  1 ],
        [ -1,  1,  1 ],
    ],
    // front
    [
        [  0,  1,  1 ],
        [ -1,  1,  1 ],
        [ -1,  0,  1 ],
        [ -1, -1,  1 ],
        [  0, -1,  1 ],
        [  1, -1,  1 ],
        [  1,  0,  1 ],
        [  1,  1,  1 ],
    ],
    // back
    [
        [  0,  1, -1 ],
        [ -1,  1, -1 ],
        [ -1,  0, -1 ],
        [ -1, -1, -1 ],
        [  0, -1, -1 ],
        [  1, -1, -1 ],
        [  1,  0, -1 ],
        [  1,  1, -1 ],
    ]
];

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

    /// Gets the ambient occlusion values for the given normal direction and position. The order of
    /// the ao values matches the order of the vertices.
    fn calculate_ambient_occlusion(&self, position: [usize; 3], normal_index: usize) -> [u32; 4] {
        // up
        // down
        // right
        // left
        // front
        // back

        let sample_directions = match normal_index {
            0 => [[-1, 0]],
        };
    }

    fn add_block(&mut self, position: [usize; 3]) {
        if !self.chunk.is_block_full(position) {
            return;
        }

        let [x, y, z] = position;
        let voxel = self.chunk.voxels[y][z][x];

        let local_position = vec3(x as f32, y as f32, z as f32);
        let chunk_offset = self.chunk.position.extend(0).xzy().as_vec3() * CHUNK_WIDTH as f32;

        for (normal_index, (face, normal)) in FACE_NORMALS.iter().enumerate() {
            if self.is_solid(self.chunk.get_world_position(position, *normal)) {
                continue;
            }

            let normal = Vec3::from_array(normal.map(|n| n as f32));
            let texture_index = get_texture_index(&voxel, face).unwrap_or_else(|| {
                panic!("could not find texture for '{voxel:?}' (face: '{face:?}')")
            });

            for (vertex_index, voxel_center_offset) in
                FACE_VERTICES[normal_index].iter().enumerate()
            {
                let world_position = voxel_center_offset + local_position + chunk_offset;

                let ambient_occlusion =
                    self.calculate_ambient_occlusion(position, normal_index, vertex_index);

                let texture_ambient = ((texture_index as u32) << 16) | ambient_occlusion;

                self.vertices.push(MeshVertex {
                    pos: world_position,
                    normal,
                    texture_ambient,
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
