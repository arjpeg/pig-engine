use anyhow::bail;
use glam::*;
use noise::{NoiseFn, Simplex};

/// The width of a chunk (xz length).
pub const CHUNK_WIDTH: usize = 16;
/// The height of a chunk (y length).
pub const CHUNK_HEIGHT: usize = 256;

/// A 3d grid of voxels.
pub type VoxelGrid = [[[Voxel; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_HEIGHT];

/// A filled cube within a 3d grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Voxel {
    #[default]
    Air,
    Grass,
}

/// A collection of voxels grouped within a AABB rectangle to increase performance
/// with regards to rendering.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The list of voxels stored contiguously in memory.
    pub voxels: Box<VoxelGrid>,
    /// The position of the chunk within the world along the xz axis.
    position: glam::IVec2,
}

/// A strategy to populate the voxels of a given chunk.
pub trait Populator {
    /// Fills in (some) of the voxels of the chunk given its position,
    /// based on some strategy.
    ///
    /// All voxels are pressumed to be initialized to `Voxel::Air`.
    fn populate(&self, voxels: &mut VoxelGrid, chunk_position: Vec2);
}

impl Chunk {
    /// Creates a new chunk with the given generation strategy, and position.
    pub fn new(position: IVec2, generator: &impl Populator) -> Self {
        let mut voxels = Box::new([[[Voxel::Air; CHUNK_WIDTH]; CHUNK_WIDTH]; CHUNK_HEIGHT]);
        generator.populate(&mut voxels, position.as_vec2());

        Self { voxels, position }
    }

    /// Returns whether the provided position is in the confines of the chunk,
    /// not accounting for the chunk's position.
    pub fn in_local_bounds(position: UVec3) -> bool {
        let UVec3 { x, y, z } = position;

        (x as usize) < CHUNK_WIDTH && (z as usize) < CHUNK_WIDTH && (y as usize) < CHUNK_HEIGHT
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
}

/// A chunk populator where the provided positions are populated with the given
/// Voxel variants.
pub struct SinglesPopulator(Vec<(UVec3, Voxel)>);

impl SinglesPopulator {
    /// Creates a new generator with the given voxel positions (relative to the chunk),
    /// and voxel types. Returns an error if any of the positions are outside of the
    /// bounds of the chunk.
    pub fn new(voxel_data: Vec<(UVec3, Voxel)>) -> anyhow::Result<Self> {
        let out_of_bounds = voxel_data
            .iter()
            .any(|(pos, _)| !Chunk::in_local_bounds(*pos));

        if out_of_bounds {
            bail!("voxel position was out of bounds")
        } else {
            Ok(Self(voxel_data))
        }
    }
}

impl Populator for SinglesPopulator {
    fn populate(&self, voxels: &mut VoxelGrid, _: Vec2) {
        for (position, voxel) in self.0.clone() {
            let UVec3 { x, y, z } = position;

            voxels[y as usize][z as usize][x as usize] = voxel;
        }
    }
}

/// A chunk populator where all blocks below the given height are set to
/// the specified voxel variant.
pub struct FlatFillPopulator(usize, Voxel);

impl FlatFillPopulator {
    /// Creates a new populator with the given voxel and height.
    /// Everything below the specified height are set to the voxel.
    /// Returns an error if the height is above the `CHUNK_HEIGHT`.
    pub fn new(height: usize, voxel: Voxel) -> anyhow::Result<Self> {
        if height >= CHUNK_HEIGHT {
            bail!("specified height was greater than max chunk height")
        } else {
            Ok(Self(height, voxel))
        }
    }
}

impl Populator for FlatFillPopulator {
    fn populate(&self, voxels: &mut VoxelGrid, _: Vec2) {
        for y in 0..self.0 {
            for z in 0..CHUNK_WIDTH {
                for x in 0..CHUNK_WIDTH {
                    voxels[y][z][x] = self.1;
                }
            }
        }
    }
}

/// A chunk populator where the voxel data is sampled using 3d simplex noise.
pub struct SimplexPopulator<'a>(&'a Simplex);

impl<'a> SimplexPopulator<'a> {
    /// Creates a new populator given the noise sampler.
    pub fn new(sampler: &'a Simplex) -> Self {
        Self(sampler)
    }
}

impl Populator for SimplexPopulator<'_> {
    fn populate(&self, voxels: &mut VoxelGrid, chunk_position: Vec2) {
        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_WIDTH {
                for x in 0..CHUNK_WIDTH {
                    let position = [
                        x as f64 + chunk_position.x as f64,
                        y as f64,
                        z as f64 + chunk_position.y as f64,
                    ];

                    if self.0.get(position) > 0.5 {
                        voxels[y][z][x] = Voxel::Grass;
                    }
                }
            }
        }
    }
}
