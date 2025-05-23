use std::str::FromStr;

use anyhow::bail;
use glam::*;
use noise::NoiseFn;

/// The width of a chunk (xz length).
pub const CHUNK_WIDTH: usize = 16;
/// The height of a chunk (y length).
pub const CHUNK_HEIGHT: usize = 256;

/// The scale factor used to sample noise values for chunk generation.
const NOISE_SCALE: f64 = 1.0 / 500.0;

/// A 3d grid of voxels.
pub type VoxelGrid = [[[Voxel; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_HEIGHT];

/// A filled cube within a 3d grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Voxel {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    Snow,
}

/// Creates a noise function that can be used to create interesting terrain.
pub fn create_noise_generator(seed: u32) -> impl NoiseFn<f64, 2> {
    use noise::*;

    /// The number of octaves used for noise generation.
    const NUM_OCTAVES: usize = 8;

    let continents_fbm = Fbm::<Perlin>::new(seed)
        .set_frequency(0.2)
        .set_lacunarity(2.05)
        .set_persistence(0.5)
        .set_octaves(NUM_OCTAVES);

    Curve::new(continents_fbm)
        .add_control_point(-1.5, -1.675)
        .add_control_point(-1.0, -1.375)
        .add_control_point(0.0, -0.375)
        .add_control_point(0.0625, 0.125)
        .add_control_point(0.125, 0.25)
        .add_control_point(0.25, 1.0)
        .add_control_point(0.5, 0.2)
        .add_control_point(1.0, 0.5)
        .add_control_point(2.0, 0.25)
        .add_control_point(2.0, 0.9)
}

/// A collection of voxels grouped within a AABB rectangle to increase performance
/// with regards to rendering.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The list of voxels stored contiguously in memory.
    pub voxels: Box<VoxelGrid>,
    /// The position of the chunk within the world along the xz axis.
    pub position: glam::IVec2,
}

impl Chunk {
    /// Creates a new chunk at the given position.
    pub fn new(position: IVec2) -> Self {
        let voxels = Box::new([[[Voxel::Air; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_HEIGHT]);

        Self { voxels, position }
    }

    /// Returns whether the provided position is in the confines of the chunk,
    /// not accounting for the chunk's position.
    pub fn in_local_bounds([x, y, z]: [usize; 3]) -> bool {
        x < CHUNK_WIDTH && z < CHUNK_WIDTH && y < CHUNK_HEIGHT
    }

    /// Returns a list of the neighbor chunks' positions.
    pub fn neighbors(&self) -> impl Iterator<Item = IVec2> {
        let position = self.position;

        (-1..=1).flat_map(move |x| (-1..=1).map(move |z| (IVec2::new(x, z) + position)))
    }

    /// Returns if the voxel at the given position is non empty (not air).
    pub fn is_block_full(&self, block_pos: [usize; 3]) -> bool {
        let [x, y, z] = block_pos;

        self.voxels[y][z][x] != Voxel::Air
    }

    /// Returns the world position of a voxel with some delta direction.
    /// This position may not be within the bounds of the chunk.
    pub fn offset_local_in_direction(
        &self,
        local_position: [usize; 3],
        direction: [isize; 3],
    ) -> [isize; 3] {
        let [x, y, z] = local_position;
        let [dx, dy, dz] = direction;

        let chunk_x_offset = self.position.x as isize * CHUNK_WIDTH as isize;
        let chunk_z_offset = self.position.y as isize * CHUNK_WIDTH as isize;

        [
            x as isize + dx + chunk_x_offset,
            y as isize + dy,
            z as isize + dz + chunk_z_offset,
        ]
    }

    /// Returns the local position of a voxel, given a world position.
    pub fn get_local_position(&self, [x, y, z]: [isize; 3]) -> Option<[usize; 3]> {
        let chunk_x_offset = self.position.x as isize * CHUNK_WIDTH as isize;
        let chunk_z_offset = self.position.y as isize * CHUNK_WIDTH as isize;

        let x = x - chunk_x_offset;
        let z = z - chunk_z_offset;

        if y < 0 {
            None
        } else {
            Some([x as usize, y as usize, z as usize])
        }
    }

    /// Fills the chunk in using noise values.
    pub fn fill_perlin(&mut self, noise: impl NoiseFn<f64, 2>) {
        let chunk_position = (self.position * CHUNK_WIDTH as i32).as_vec2();

        for z in 0..CHUNK_WIDTH {
            for x in 0..CHUNK_WIDTH {
                let local_position = vec2(x as f32, z as f32);
                let position = (chunk_position + local_position).as_dvec2() * NOISE_SCALE;

                let height = {
                    let sample = noise.get(position.to_array());
                    let sample = (sample + 1.0) / 2.0; // put sample in [0, 1]

                    ((sample * CHUNK_HEIGHT as f64) as usize).clamp(0, CHUNK_HEIGHT - 2)
                };

                for y in 0..=height {
                    self.voxels[y][z][x] = match y {
                        200..=CHUNK_HEIGHT => Voxel::Snow,
                        150.. => Voxel::Stone,
                        _ if y == height => Voxel::Grass,
                        _ => Voxel::Dirt,
                    };
                }
            }
        }
    }
}

impl FromStr for Voxel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "air" => Ok(Self::Air),
            "grass" => Ok(Self::Grass),
            "dirt" => Ok(Self::Dirt),
            "stone" => Ok(Self::Stone),
            "snow" => Ok(Self::Snow),
            _ => bail!("unkown voxel type, '{s}'"),
        }
    }
}
