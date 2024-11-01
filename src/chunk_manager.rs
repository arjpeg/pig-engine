use std::collections::{HashMap, VecDeque};

use glam::{ivec2, IVec2, Vec3};
use noise::Perlin;
use wgpu::Device;

use crate::{chunk::*, model::*};

/// The radius around the player in which chunks are loaded. One extra chunk
/// in both the x and z axes are loaded as padding for mesh generation.
pub const CHUNK_LOAD_RADIUS: usize = 32;

/// The maximum number of chunks whose voxel generation can be built per frame.
pub const MAX_CHUNK_GENERATION_PER_FRAME: usize = 20;
/// The maximum number of chunks whose mesh can be built per frame.
pub const MAX_CHUNK_MESH_GENERATION_PER_FRAME: usize = 10;

/// Manages the loading and unloading of chunks around the player.
pub struct ChunkManager {
    /// The noise generator used to generate terrain, etc.
    noise: Perlin,

    /// The chunks that are currently loaded.
    chunks: HashMap<glam::IVec2, crate::chunk::Chunk>,
    /// The meshes of the chunks that have been made.
    meshes: HashMap<glam::IVec2, MeshLoadState>,

    /// A queue of chunks to load.
    load_queue: VecDeque<glam::IVec2>,
    /// A queue of chunks to build meshes for.
    build_queue: VecDeque<glam::IVec2>,
}

/// Represents the state of the loaded mesh - either built and not uploaded,
/// or built and uploaded
#[derive(Debug)]
enum MeshLoadState {
    /// The mesh has been uploaded to the GPU.
    Uploaded(crate::model::Mesh),
    /// The mesh has been built but not uploaded to the GPU.
    Todo((Vec<crate::model::MeshVertex>, Vec<u32>)),
}

impl ChunkManager {
    /// Creates a new chunk manager.
    pub fn new() -> Self {
        let noise = Perlin::new(1);

        Self {
            noise,
            chunks: HashMap::new(),
            meshes: HashMap::new(),
            load_queue: VecDeque::new(),
            build_queue: VecDeque::new(),
        }
    }

    /// Updates the chunk manager with the latest player position.
    pub fn update(&mut self, position: Vec3) {
        let chunk = ivec2(
            (position.x as i32).div_euclid(CHUNK_WIDTH as i32),
            (position.z as i32).div_euclid(CHUNK_WIDTH as i32),
        );

        // generate voxel data for padding chunks, should be higher in priority than building the
        // mesh for other chunks
        for z in [-3, 3] {
            for x in [-3, 3] {
                let chunk = ivec2(
                    chunk.x + x * CHUNK_LOAD_RADIUS as i32,
                    chunk.y + z * CHUNK_LOAD_RADIUS as i32,
                );

                if !self.load_queue.contains(&chunk) && !self.chunks.contains_key(&chunk) {
                    self.load_queue.push_back(chunk);
                }
            }
        }

        let chunks = Self::get_chunks_around(chunk, CHUNK_LOAD_RADIUS);

        for chunk in chunks {
            if !self.load_queue.contains(&chunk) && !self.chunks.contains_key(&chunk) {
                self.load_queue.push_back(chunk);
            }

            if !self.build_queue.contains(&chunk) && !self.meshes.contains_key(&chunk) {
                self.build_queue.push_back(chunk);
            }
        }

        self.load_chunks();
        self.build_meshes();
    }

    /// Uploads any meshes that have built but not uploaded.
    pub fn resolve_mesh_uploads(&mut self, device: &Device) {
        self.meshes.values_mut().for_each(|m| match m {
            MeshLoadState::Uploaded(_) => {}
            MeshLoadState::Todo((vertices, indices)) => {
                *m = MeshLoadState::Uploaded(Mesh::new(vertices, indices, device));
            }
        });
    }

    /// Loads upto `MAX_CHUNK_GENERATION_PER_FRAME` chunks that are currently in the load queue.
    fn load_chunks(&mut self) {
        for _ in 0..MAX_CHUNK_GENERATION_PER_FRAME {
            let Some(position) = self.load_queue.pop_front() else {
                break;
            };

            let mut chunk = Chunk::new(position);
            chunk.fill_perlin(&self.noise);

            self.chunks.insert(position, chunk);
        }
    }

    /// Builds upto `MAX_CHUNK_MESH_GENERATION_PER_FRAME` meshes that are currently in the build
    /// queue.
    pub fn build_meshes(&mut self) {
        let mut queue = Vec::new();

        for _ in 0..MAX_CHUNK_MESH_GENERATION_PER_FRAME {
            let Some(position) = self.build_queue.pop_front() else {
                break;
            };

            if !self.chunks.contains_key(&position) {
                // put the chunk back in the queue, it's voxel data hasn't been loaded
                queue.push(position);
                continue;
            }

            let mesh = ChunkMeshBuilder::new(&self.chunks, position).build();
            self.meshes.insert(position, MeshLoadState::Todo(mesh));
        }

        self.build_queue.extend(queue);
    }

    /// Gets the chunks around a chunk in the provided radius.
    fn get_chunks_around(position: IVec2, radius: usize) -> Vec<IVec2> {
        let mut positions = Vec::with_capacity(radius * radius);

        let radius = radius as i32;

        for x in -radius..=radius {
            for z in -radius..=radius {
                positions.push(position + ivec2(x, z));
            }
        }

        positions
    }

    /// Returns all the meshes that have been uploaded to the GPU, and
    /// are ready for rendering.
    pub fn loaded_meshes(&self) -> Vec<&Mesh> {
        self.meshes
            .values()
            .filter_map(|m| match m {
                MeshLoadState::Todo(_) => None,
                MeshLoadState::Uploaded(mesh) => Some(mesh),
            })
            .collect()
    }

    /// Returns the number of chunks currently loaded.
    pub fn chunks_loaded(&self) -> usize {
        self.chunks.len()
    }

    /// Returns the number of meshes currently built.
    pub fn meshes_loaded(&self) -> usize {
        self.meshes.len()
    }
}
