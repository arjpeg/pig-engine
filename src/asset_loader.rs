use regex::Regex;
use std::{fs, str::FromStr, sync::OnceLock};
use wgpu::*;

use anyhow::{bail, Context};

use crate::{chunk::Voxel, texture::Texture};

static TEXTURE_UPLOAD_ORDER: OnceLock<Vec<(Voxel, Face)>> = OnceLock::new();

/// The side to which this face is oriented towards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Towards positive y.
    Up,
    /// Towards negative y.
    Down,
    /// Perpendicular to the y-axis.
    Side,
}

/// Loads all textures from the `assets` directory and uploads them to the GPU.
/// All textures are uploaded onto the same texture, in seperate layers. The order
/// in which the images are stored are saved in `TEXTURE_UPLOAD_ORDER`.
pub fn load_textures(device: &Device, queue: &Queue) -> anyhow::Result<Texture> {
    let assets = fs::read_dir("assets").context("loading voxel textures")?;

    let assets = assets
        .filter_map(|entry| entry.map(|entry| entry.path()).ok())
        .filter(|path| path.is_file())
        .filter_map(|path| path.file_name()?.to_str().map(String::from));

    let re = Regex::new(r"(\w+)_(\w+).png")?;

    let mut images = Vec::new();
    let mut order = Vec::new();

    for asset_file in assets {
        for (_, [voxel, face]) in re.captures_iter(&asset_file).map(|c| c.extract()) {
            println!("loading {:?}", format!("assets/{asset_file}"));

            let image = image::open(format!("assets/{asset_file}"))
                .context(format!("loading {asset_file}"))?;

            let voxel = Voxel::from_str(voxel)?;
            let face = Face::from_str(face)?;

            images.push(image);
            order.push((voxel, face));
        }
    }

    let texture = Texture::from_images(device, queue, &images, Some("Voxel Textures"))?;

    TEXTURE_UPLOAD_ORDER.get_or_init(|| order);

    Ok(texture)
}

/// Gets the appropriate texture index for a given voxel oriented in this face direction.
pub fn get_texture_index(voxel: &Voxel, face: &Face) -> Option<u16> {
    TEXTURE_UPLOAD_ORDER
        .get()?
        .iter()
        .position(|(v, f)| voxel == v && face == f)
        .map(|index| index as u16)
}

impl FromStr for Face {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "side" => Ok(Self::Side),
            _ => bail!("unkown face direction, '{s}'"),
        }
    }
}
