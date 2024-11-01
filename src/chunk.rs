use std::{isize, str::FromStr};

use anyhow::bail;
use glam::*;
use noise::NoiseFn;

/// The width of a chunk (xz length).
pub const CHUNK_WIDTH: usize = 16;
/// The height of a chunk (y length).
pub const CHUNK_HEIGHT: usize = 128;

/// The scale factor used to sample noise values for chunk generation.
pub const NOISE_SCALE: f32 = 1.0 / 250.0;

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

    /// Fills the chunk in using noise values.
    pub fn fill_perlin(&mut self, noise: impl NoiseFn<f64, 2>) {
        let global_position = (self.position * CHUNK_WIDTH as i32).as_vec2();

        for z in 0..CHUNK_WIDTH {
            for x in 0..CHUNK_WIDTH {
                let local_position = vec2(x as f32, z as f32);
                let true_position = (global_position + local_position) * NOISE_SCALE;

                let filled = (noise.get(true_position.as_dvec2().to_array()) + 1.0)
                    * 0.5
                    * CHUNK_HEIGHT as f64;

                for y in 0..(filled as usize + 1).min(CHUNK_HEIGHT - 1) {
                    self.voxels[y][z][x] = Voxel::Grass
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
