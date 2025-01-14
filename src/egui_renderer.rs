use std::sync::Arc;

use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;

use wgpu::*;
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

/// Ties together egui with wgpu, by providing a renderer that can render egui
/// textures and meshes.
pub struct EguiRenderer {
    /// The egui context that holds the state of the gui.
    context: egui::Context,
    /// The actual renderer that renders the egui context.
    renderer: egui_wgpu::Renderer,

    /// The window that the renderer is rendering to.
    window: Arc<winit::window::Window>,
    /// egui's internal state of the window.
    state: egui_winit::State,
}

impl EguiRenderer {
    /// Creates a new egui renderer with the given window.
    pub fn new(window: Arc<Window>, device: &Device, texture_format: TextureFormat) -> Self {
        let context = Context::default();
        let id = context.viewport_id();

        let state = State::new(
            context.clone(),
            id,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );

        let renderer = Renderer::new(device, texture_format, None, 1);

        Self {
            context,
            renderer,
            window,
            state,
        }
    }

    /// Updates egui with the latest events.
    pub fn handle_input(&mut self, event: &WindowEvent) -> bool {
        self.state.on_window_event(&self.window, event).consumed
    }

    /// Renders all egui content on to the surface.
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        ui: impl FnOnce(&Context),
    ) {
        let input = self.state.take_egui_input(&self.window);
        let full_output = self.context.run(input, ui);

        self.state
            .handle_platform_output(&self.window, full_output.platform_output);

        let tris = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        let PhysicalSize { width, height } = self.window.inner_size();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("Egui Render Pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
        }

        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}
