#![allow(dead_code)]

use app::App;
use winit::{dpi::LogicalSize, event_loop::EventLoop, window::WindowBuilder};

mod app;
mod asset_loader;
mod camera;
mod chunk;
mod chunk_manager;
mod egui_renderer;
mod model;
mod renderer;
mod texture;

#[pollster::main]
async fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Pig Engine")
        .with_inner_size(LogicalSize::new(1920, 1080))
        .build(&event_loop)?;

    let mut app = App::new(&window).await?;

    event_loop.run(|event, elwt| app.update(event, elwt, &window).unwrap())?;

    Ok(())
}
