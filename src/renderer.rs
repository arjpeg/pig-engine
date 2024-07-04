use wgpu::*;
use winit::{dpi::PhysicalSize, window::Window};

use anyhow::Result;

#[derive(Debug)]
pub struct Renderer<'s> {
    /// The actual physical device responsible for rendering things (most likely the GPU).
    device: wgpu::Device,
    /// The queue of commands being staged to be sent to the `device`.
    queue: wgpu::Queue,
    /// The series of steps that data takes while moving through the rendering process.
    pipeline: wgpu::RenderPipeline,

    /// A reference to the surface being rendered onto.
    surface: wgpu::Surface<'s>,
    /// The configuration of the `surface`.
    surface_config: wgpu::SurfaceConfiguration,
}

impl<'s> Renderer<'s> {
    /// Creates a new renderer given a window as the surface.
    pub async fn new(window: &'s Window) -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: InstanceFlags::empty(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Request Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                },
                None,
            )
            .await?;

        let surface_config = Self::get_surface_config(&adapter, &surface, window.inner_size());
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            surface,
            surface_config,
        })
    }

    /// Gets the surface configuration given an adapter, surface, and surface size
    fn get_surface_config(
        adapter: &Adapter,
        surface: &Surface,
        PhysicalSize { width, height }: PhysicalSize<u32>,
    ) -> SurfaceConfiguration {
        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .cloned()
            .find(TextureFormat::is_srgb)
            .unwrap();

        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        }
    }

    /// Resizes the renderer's `config` to match the new given size.
    pub fn resize(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        assert!(width > 0, "cannot resize to zero width");
        assert!(height > 0, "cannot resize to zero height");

        self.surface_config.width = width;
        self.surface_config.height = height;

        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Renders the currently bound vertex buffer onto the `surface`.
    pub fn render(&self) -> std::result::Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor {
            label: Some("Rendering View"),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..3, 0..1);
        };

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
