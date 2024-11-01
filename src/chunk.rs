use std::{isize, str::FromStr};

use anyhow::bail;
use glam::*;
use noise::{NoiseFn, Perlin};

/// The width of a chunk (xz length).
pub const CHUNK_WIDTH: usize = 16;
/// The height of a chunk (y length).
pub const CHUNK_HEIGHT: usize = 128;

/// A 3d grid of voxels.
pub type VoxelGrid = [[[Voxel; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_HEIGHT];

/// A filled cube within a 3d grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Voxel {
    #[default]
    Air,
    Grass,
    Dirt,
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

/// A generator for populating the voxels of a chunk.
#[derive(Debug)]
pub enum ChunkGenerator {
    /// A generator that generates a flat world.
    Flat,
    /// A generator that generates a world with some noise.
    Noise(noise::Perlin),
}

impl Chunk {
    /// Creates a new chunk at the given position.
    pub fn new(position: IVec2) -> Self {
        let voxels = Box::new([[[Voxel::Air; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_HEIGHT]);

        Self { voxels, position }
    }

    /// Fills the chunk with voxels using the provided generator.
    pub fn generate(&mut self, generator: &ChunkGenerator) {
        generator.generate(self);
    }

    /// Returns whether the provided position is in the confines of the chunk,
    /// not accounting for the chunk's position.
    pub fn in_local_bounds([x, y, z]: [usize; 3]) -> bool {
        x < CHUNK_WIDTH && z < CHUNK_WIDTH && y < CHUNK_HEIGHT
    }

    /// Returns if the voxel at the given position is non empty (not air).
    pub fn is_block_full(&self, block_pos: [usize; 3]) -> bool {
        let [x, y, z] = block_pos;

        self.voxels[y][z][x] != Voxel::Air
    }

    /// Utility to add a block position with some delta direction, and return
    /// the position if it was within the bounds of a chunk, or else None.
    pub fn get_block_in_direction(
        [x, y, z]: [usize; 3],
        [dx, dy, dz]: [isize; 3],
    ) -> Option<[usize; 3]> {
        let x = x.checked_add_signed(dx).filter(|n| *n < CHUNK_WIDTH)?;
        let z = z.checked_add_signed(dz).filter(|n| *n < CHUNK_WIDTH)?;
        let y = y.checked_add_signed(dy).filter(|n| *n < CHUNK_HEIGHT)?;

        Some([x, y, z])
    }

    /// Returns the world position of a voxel with some delta direction.
    /// This position may not be within the bounds of the chunk.
    pub fn get_world_position(
        &self,
        [x, y, z]: [usize; 3],
        [dx, dy, dz]: [isize; 3],
    ) -> [isize; 3] {
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
}

impl ChunkGenerator {
    pub fn generate(&self, chunk: &mut Chunk) {
        match self {
            Self::Flat => self.apply_flat(chunk),
            Self::Noise(noise) => self.apply_perlin(chunk, noise),
        }
    }

    fn apply_flat(&self, chunk: &mut Chunk) {
        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_WIDTH {
                for x in 0..CHUNK_WIDTH {
                    let voxel = match y {
                        0..10 => Voxel::Dirt,
                        10..11 => Voxel::Grass,
                        _ => continue,
                    };

                    chunk.voxels[y][z][x] = voxel;
                }
            }
        }
    }

    fn apply_perlin(&self, chunk: &mut Chunk, noise: &Perlin) {
        for z in 0..CHUNK_WIDTH {
            for x in 0..CHUNK_WIDTH {
                let filled = (noise.get([
                    (x as f64 + (CHUNK_WIDTH as f64 * chunk.position.x as f64)) / 250.0,
                    (z as f64 + (CHUNK_WIDTH as f64 * chunk.position.y as f64)) / 250.0,
                ]) + 1.0)
                    * 0.5
                    * CHUNK_HEIGHT as f64;

                for y in 0..(filled as usize + 1).min(CHUNK_HEIGHT - 1) {
                    chunk.voxels[y][z][x] = Voxel::Grass
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
            _ => bail!("unkown voxel type, '{s}'"),
        }
    }
}
