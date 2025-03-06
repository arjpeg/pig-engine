use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::mpsc,
};

use glam::{ivec2, IVec2, Vec3};
use noise::NoiseFn;
use rayon::ThreadPoolBuilder;
use wgpu::Device;

use crate::{chunk::*, mesher::ChunkMesher, model::*};

/// The radius around the player in which chunks are loaded. One extra chunk
/// in both the x and z axes are loaded as padding for mesh generation.
pub const CHUNK_LOAD_RADIUS: usize = 64;
/// The size of the padding around loaded chunks. These padding chunks only have
/// their voxel data generated; without their meshes being built.
pub const CHUNK_LOAD_PADDING: usize = 2;

/// The maximum number of chunks whose voxel data can be generated per frame.
pub const MAX_CHUNK_DATA_GENERATION_PER_FRAME: usize = 32;
/// The maximum number of chunks whose meshes can be built per frame.
pub const MAX_CHUNK_MESH_GENERATION_PER_FRAME: usize = 16;

type UnUploadedMesh = (Vec<MeshVertex>, Vec<u32>);

/// Manages the loading and unloading of chunks around the player.
pub struct ChunkManager {
    /// The noise generator used to generate terrain, etc.
    noise: Box<dyn NoiseFn<f64, 2> + Send + Sync>,

    /// The chunks that are currently loaded.
    chunks: HashMap<glam::IVec2, Chunk>,
    /// The meshes of the chunks that have been made and uploaded to the GPU.
    uploaded_meshes: HashMap<glam::IVec2, Mesh>,
    /// The meshes of the chunks that have been made but not yet been uploaded to the GPU.
    unuploaded_meshes: HashMap<glam::IVec2, UnUploadedMesh>,

    /// A queue of chunks to load.
    load_queue: VecDeque<glam::IVec2>,
    /// A queue of chunks to build meshes for.
    build_queue: VecDeque<glam::IVec2>,

    /// A list of chunks that are currently having their voxel data generated.
    currently_generating: HashSet<glam::IVec2>,
    /// A list of chunks that are currently having their meshes built.
    currently_meshing: HashSet<glam::IVec2>,

    /// A thread pool to manage chunks voxel data to be built.
    chunk_thread_pool: rayon::ThreadPool,
    /// The producer end of the `std::sync::mpsc::channel` to communicate with workers.
    chunk_tx: std::sync::mpsc::Sender<Chunk>,
    /// The consumer end of the `std::sync::mpsc::channel` to communicate with workers.
    chunk_rx: std::sync::mpsc::Receiver<Chunk>,

    /// A thread pool to manage chunks meshes to be built.
    mesh_thread_pool: rayon::ThreadPool,
    /// The producer end of the `std::sync::mpsc::channel` to communicate with workers.
    mesh_tx: std::sync::mpsc::Sender<(glam::IVec2, UnUploadedMesh)>,
    /// The consumer end of the `std::sync::mpsc::channel` to communicate with workers.
    mesh_rx: std::sync::mpsc::Receiver<(glam::IVec2, UnUploadedMesh)>,

    /// The (current) chunk the player is in.
    current_chunk: Option<glam::IVec2>,
}

impl ChunkManager {
    /// Creates a new chunk manager.
    pub fn new() -> Self {
        let noise = Box::new(create_noise_generator(129));

        let chunk_thread_pool = ThreadPoolBuilder::new()
            .num_threads(16)
            .build()
            .expect("could not create chunk voxel builder thread pool");

        let (chunk_tx, chunk_rx) = mpsc::channel();

        let mesh_thread_pool = ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .expect("could not create mesh builder thread pool");

        let (mesh_tx, mesh_rx) = mpsc::channel();

        Self {
            noise,
            chunks: HashMap::new(),
            unuploaded_meshes: HashMap::new(),
            uploaded_meshes: HashMap::new(),
            load_queue: VecDeque::new(),
            build_queue: VecDeque::new(),
            currently_generating: HashSet::new(),
            currently_meshing: HashSet::new(),
            chunk_thread_pool,
            chunk_tx,
            chunk_rx,
            mesh_thread_pool,
            mesh_tx,
            mesh_rx,
            current_chunk: None,
        }
    }

    /// Updates the chunk manager with the latest player position.
    pub fn update(&mut self, player_position: Vec3) {
        self.load_chunks();
        self.build_meshes();

        let player_chunk = ivec2(
            (player_position.x as i32).div_euclid(CHUNK_WIDTH as i32),
            (player_position.z as i32).div_euclid(CHUNK_WIDTH as i32),
        );

        if let Some(chunk) = self.current_chunk {
            if chunk == player_chunk {
                return;
            };
        }

        self.current_chunk = Some(player_chunk);
        self.queue_surrounding_chunks();
    }

    /// Uploads any meshes that have built but not uploaded.
    pub fn resolve_mesh_uploads(&mut self, device: &Device) {
        for (position, (vertices, indices)) in self.unuploaded_meshes.drain() {
            let mesh = Mesh::new(&vertices, &indices, device);
            self.uploaded_meshes.insert(position, mesh);
        }
    }

    /// Adds the chunks that are not currently being built or have not already been generated (mesh
    /// or voxel data) onto the respective queues.
    fn queue_surrounding_chunks(&mut self) {
        let Some(player_chunk) = self.current_chunk else {
            return;
        };

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
            if !(self.chunks.contains_key(&neighbor)
                || self.load_queue.contains(&neighbor)
                || self.currently_generating.contains(&neighbor))
            {
                self.load_queue.push_back(neighbor);
            }

            if distance > CHUNK_LOAD_RADIUS {
                continue;
            }

            let mesh_built = self.unuploaded_meshes.contains_key(&neighbor)
                || self.uploaded_meshes.contains_key(&neighbor);

            if !(mesh_built
                || self.build_queue.contains(&neighbor)
                || self.currently_meshing.contains(&neighbor))
            {
                self.build_queue.push_back(neighbor);
            }
        }
    }

    /// Loads upto `MAX_CHUNK_GENERATION_PER_FRAME` chunks that are currently in the load queue.
    fn load_chunks(&mut self) {
        for chunk in self.chunk_rx.try_iter() {
            self.currently_generating.remove(&chunk.position);
            self.chunks.insert(chunk.position, chunk);
        }

        for position in self
            .load_queue
            .drain(..MAX_CHUNK_DATA_GENERATION_PER_FRAME.min(self.load_queue.len()))
        {
            let tx = self.chunk_tx.clone();

            self.currently_generating.insert(position);

            self.chunk_thread_pool.scope(|_| {
                let mut chunk = Chunk::new(position);
                chunk.fill_perlin(&*self.noise);

                tx.send(chunk).unwrap();
            });
        }
    }

    /// Builds upto `MAX_CHUNK_MESH_GENERATION_PER_FRAME` meshes that are currently in the build
    /// queue.
    pub fn build_meshes(&mut self) {
        for (position, mesh) in self.mesh_rx.try_iter() {
            self.currently_meshing.remove(&position);

            assert!(
                self.unuploaded_meshes.insert(position, mesh).is_none()
                    && !self.uploaded_meshes.contains_key(&position),
                "should not build duplicate meshes"
            );
        }

        let mut reinsert = Vec::new();

        for position in self
            .build_queue
            .drain(..MAX_CHUNK_MESH_GENERATION_PER_FRAME.min(self.build_queue.len()))
        {
            let tx = self.mesh_tx.clone();
            let chunks = &self.chunks;

            if !self.chunks.contains_key(&position) {
                reinsert.push(position);
                continue;
            }

            self.currently_meshing.insert(position);

            self.mesh_thread_pool.scope(move |_| {
                let mesh = ChunkMesher::new(&chunks, position).build();
                tx.send((position, mesh)).unwrap();
            });
        }

        self.build_queue.extend(reinsert);
    }

    /// Gets the chunks around a chunk in the provided radius.
    fn get_chunks_around(position: IVec2, radius: usize) -> impl Iterator<Item = IVec2> {
        let radius = radius as i32;

        (-radius..=radius)
            .flat_map(move |x| (-radius..=radius).map(move |z| position + ivec2(x, z)))
    }

    /// Returns all the meshes that have been uploaded to the GPU, and
    /// are ready for rendering.
    pub fn loaded_meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.uploaded_meshes.values()
    }

    /// Returns the number of chunks currently loaded.
    pub fn chunks_loaded(&self) -> usize {
        self.chunks.len()
    }

    /// Returns the number of meshes currently built.
    pub fn meshes_loaded(&self) -> usize {
        self.uploaded_meshes.len() + self.unuploaded_meshes.len()
    }
}
