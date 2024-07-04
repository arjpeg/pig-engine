#![allow(dead_code)]

use renderer::Renderer;
use wgpu::SurfaceError;
use winit::{dpi::LogicalSize, event::Event, event_loop::EventLoop, window::WindowBuilder};
use winit_input_helper::WinitInputHelper;

mod camera;
mod model;
mod renderer;

#[pollster::main]
async fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Pig Engine")
        .with_inner_size(LogicalSize::new(1920, 1080))
        .build(&event_loop)?;

    let mut input = WinitInputHelper::new();

    let mut renderer = Renderer::new(&window).await?;

    event_loop.run(|event, elwt| {
        if !input.update(&event) {
            return;
        };

        println!("{:?}", input.mouse_diff());

        if let Some(size) = input.window_resized() {
            renderer.resize(size);
        }

        match renderer.render() {
            Ok(_) => {}
            // Reconfigure the surface if it's lost
            Err(SurfaceError::Lost) => renderer.resize(window.inner_size()),
            // If we are out of memory, just quit the app
            Err(SurfaceError::OutOfMemory) => elwt.exit(),
            // For other errors, they will be gone by the next frame
            Err(error) => eprintln!("{error}"),
        };
    })?;

    Ok(())
}
