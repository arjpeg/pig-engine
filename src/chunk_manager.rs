use std::collections::HashMap;

use bracket_noise::prelude::*;
use glam::{ivec2, IVec2, Vec3};
use wgpu::Device;

use crate::{
    chunk::{Chunk, SimplexPopulator, CHUNK_WIDTH},
    model::{ChunkMeshBuilder, Mesh},
};

pub const CHUNK_LOAD_RADIUS: usize = 10;

/// Manages the loading and unloading of chunks around the player.
pub struct ChunkManager {
    /// The noise generator used to generate terrain, etc.
    noise: bracket_noise::prelude::FastNoise,

    /// The chunks that are currently loaded.
    chunks: HashMap<glam::IVec2, crate::chunk::Chunk>,
    /// The meshes of the chunks that have been made.
    meshes: HashMap<glam::IVec2, MeshLoadState>,
}

/// Represents the state of the loaded mesh - either built and not uploaded,
/// or built and uploaded
#[derive(Debug)]
enum MeshLoadState {
    Uploaded(crate::model::Mesh),
    Todo((Vec<crate::model::MeshVertex>, Vec<u32>)),
}

impl ChunkManager {
    /// Creates a new chunk manager.
    pub fn new() -> Self {
        let seed = 30;
        let mut noise = FastNoise::seeded(seed);

        noise.set_noise_type(NoiseType::PerlinFractal);
        noise.set_fractal_type(FractalType::FBM);
        noise.set_fractal_octaves(8);
        noise.set_fractal_gain(0.5);
        noise.set_fractal_lacunarity(2.0);
        noise.set_frequency(2.0);

        Self {
            noise,
            chunks: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    /// Updates the chunk manager with the latest player position.
    pub fn update(&mut self, position: Vec3) {
        let chunk = ivec2(
            (position.x as i32).div_euclid(CHUNK_WIDTH as i32),
            (position.z as i32).div_euclid(CHUNK_WIDTH as i32),
        );

        let queue = Self::get_chunks_around(chunk, CHUNK_LOAD_RADIUS + 1);

        for chunk in queue {
            if !self.chunks.contains_key(&chunk) {
                self.load_chunk(chunk);
                self.build_mesh(chunk);
            }
        }
    }

    pub fn resolve_mesh_uploads(&mut self, device: &Device) {
        self.meshes.values_mut().for_each(|m| match m {
            MeshLoadState::Uploaded(_) => {}
            MeshLoadState::Todo((vertices, indices)) => {
                *m = MeshLoadState::Uploaded(Mesh::new(&vertices, &indices, device));
            }
        });
    }

    /// Gets the chunks around a chunk in the provided radius.
    fn get_chunks_around(position: IVec2, radius: usize) -> Vec<IVec2> {
        let mut chunks = Vec::with_capacity(radius * radius);

        let radius = radius as i32;

        for x in -radius..=radius {
            for z in -radius..=radius {
                chunks.push(position + ivec2(x, z));
            }
        }

        chunks
    }

    fn load_chunk(&mut self, chunk: IVec2) {
        let populator = SimplexPopulator::new(&self.noise);
        self.chunks.insert(chunk, Chunk::new(chunk, &populator));
    }

    /// Returns the number of chunks currently loaded.
    pub fn chunks_loaded(&self) -> usize {
        self.chunks.len()
    }

    /// Returns the number of meshes currently built.
    pub fn meshes_loaded(&self) -> usize {
        self.meshes.len()
    }

    /// Builds the mesh for the chunks around the player.
    fn build_mesh(&mut self, chunk: IVec2) {
        let mesh = ChunkMeshBuilder::new(
            self.chunks
                .get(&chunk)
                .expect("cannot build mesh for unloaded chunk"),
        )
        .build();

        self.meshes.insert(chunk, MeshLoadState::Todo(mesh));
    }

    pub fn loaded_meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.meshes.values().filter_map(|m| match m {
            MeshLoadState::Todo(_) => None,
            MeshLoadState::Uploaded(mesh) => Some(mesh),
        })
    }
}
