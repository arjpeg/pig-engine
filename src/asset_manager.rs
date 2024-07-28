use std::fs;

use crate::chunk::Voxel;
use anyhow::Context;
use regex::Regex;
use wgpu::*;

/// The direction in which a particular voxel face is being rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceDirection {
    /// Facing up (0, 1, 0).
    Up,
    /// Facing down (0, -1, 0).
    Down,
    /// Facing perpendicular to the y-axis.
    Side,
}

/// Associates voxels with their texture for each face.
#[derive(Debug)]
pub struct AssetRegistry {
    /// All textures uploaded to the GPU.
    textures: Vec<(Voxel, FaceDirection)>,
}

impl AssetRegistry {
    /// Finds all of the voxel assets in the `assets` directory at the root of the project,
    /// and uploads them to the GPU.
    pub fn upload_all() -> anyhow::Result<()> {
        let assets = fs::read_dir("assets").context("loading voxel textures")?;

        let assets = assets
            .filter_map(|entry| entry.map(|entry| entry.path()).ok())
            .filter(|path| path.is_file())
            .filter_map(|path| path.file_name()?.to_str().map(String::from));

        let re = Regex::new(r"(\w+)_(\w+).png")?;

        for asset_file in assets {
            for (_, [voxel, face]) in re.captures_iter(&asset_file).map(|c| c.extract()) {
                println!("{:?}", (voxel, face));
            }
        }

        todo!()
    }
}
