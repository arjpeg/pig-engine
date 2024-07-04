use renderer::Renderer;
use wgpu::SurfaceError;
use winit::{dpi::LogicalSize, event::Event, event_loop::EventLoop, window::WindowBuilder};

mod renderer;

#[pollster::main]
async fn main() -> anyhow::Result<()> {
    use winit::event::WindowEvent as WE;

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Pig Engine")
        .with_inner_size(LogicalSize::new(1920, 1080))
        .build(&event_loop)?;

    let mut renderer = Renderer::new(&window).await?;

    event_loop.run(|event, elwt| match event {
        Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
            WE::RedrawRequested => {
                match renderer.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost
                    Err(SurfaceError::Lost) => renderer.resize(window.inner_size()),
                    // If we are out of memory, just quit the app
                    Err(SurfaceError::OutOfMemory) => elwt.exit(),
                    // For other errors, they will be gone by the next frame
                    Err(error) => eprintln!("{error}"),
                };
            }

            WE::Resized(size) => renderer.resize(size),

            _ => {}
        },

        Event::AboutToWait => {
            window.request_redraw();
        }

        _ => {}
    })?;

    Ok(())
}
