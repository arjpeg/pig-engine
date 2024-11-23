use std::collections::{HashMap, VecDeque};

use glam::{ivec2, IVec2, Vec3};
use noise::NoiseFn;
use wgpu::Device;

use crate::{chunk::*, model::*};

/// The radius around the player in which chunks are loaded. One extra chunk
/// in both the x and z axes are loaded as padding for mesh generation.
pub const CHUNK_LOAD_RADIUS: usize = 32;
/// The size of the padding around loaded chunks. These padding chunks only have
/// their voxel data generated; without their meshes being built.
pub const CHUNK_LOAD_PADDING: usize = 1;

/// The maximum number of chunks whose voxel generation can be built per frame.
pub const MAX_CHUNK_GENERATION_PER_FRAME: usize = 32;
/// The maximum number of chunks whose mesh can be built per frame.
pub const MAX_CHUNK_MESH_GENERATION_PER_FRAME: usize = 10;

/// Manages the loading and unloading of chunks around the player.
pub struct ChunkManager {
    /// The noise generator used to generate terrain, etc.
    noise: Box<dyn NoiseFn<f64, 2>>,

    /// The chunks that are currently loaded.
    chunks: HashMap<glam::IVec2, Chunk>,
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
    Uploaded(Mesh),
    /// The mesh has been built but not uploaded to the GPU.
    Todo((Vec<MeshVertex>, Vec<u32>)),
}

impl ChunkManager {
    /// Creates a new chunk manager.
    pub fn new() -> Self {
        let noise = Box::new(create_noise_generator(0));

        Self {
            noise,
            chunks: HashMap::new(),
            meshes: HashMap::new(),
            load_queue: VecDeque::new(),
            build_queue: VecDeque::new(),
        }
    }

    /// Updates the chunk manager with the latest player position.
    pub fn update(&mut self, player_position: Vec3) {
        let player_chunk = ivec2(
            (player_position.x as i32).div_euclid(CHUNK_WIDTH as i32),
            (player_position.z as i32).div_euclid(CHUNK_WIDTH as i32),
        );

        let mut neighbors =
            Self::get_chunks_around(player_chunk, CHUNK_LOAD_RADIUS + CHUNK_LOAD_PADDING)
                .map(|chunk| {
                    // the manhattan distance between this chunk and the player
                    let distance = (player_chunk.x - chunk.x)
                        .abs()
                        .max((player_chunk.y - chunk.y).abs())
                        as usize;

                    (chunk, distance)
                })
                .collect::<Vec<_>>();

        // prioritize "padding" chunks (i.e. chunks outside the load-radius), then sort from
        // closest to the player
        neighbors.sort_by(|(_, dist_a), (_, dist_b)| {
            let a_inside = *dist_a <= CHUNK_LOAD_RADIUS;
            let b_inside = *dist_b <= CHUNK_LOAD_RADIUS;

            // Inside chunks first, then by distance ascending
            b_inside.cmp(&a_inside).then_with(|| dist_a.cmp(dist_b))
        });

        for (neighbor, distance) in neighbors {
            if !self.load_queue.contains(&neighbor) && !self.chunks.contains_key(&neighbor) {
                self.load_queue.push_back(neighbor);
            }

            if distance > CHUNK_LOAD_RADIUS {
                continue;
            }

            if !self.build_queue.contains(&neighbor) && !self.meshes.contains_key(&neighbor) {
                self.build_queue.push_back(neighbor);
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
    fn get_chunks_around(position: IVec2, radius: usize) -> impl Iterator<Item = IVec2> {
        let radius = radius as i32;

        (-radius..=radius)
            .flat_map(move |x| (-radius..=radius).map(move |z| position + ivec2(x, z)))
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
