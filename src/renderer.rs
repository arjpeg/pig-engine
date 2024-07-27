use std::ops::Range;

use glam::Mat4;

use wgpu::*;
use winit::{dpi::PhysicalSize, window::Window};

use anyhow::Result;

use crate::{
    camera::Camera,
    chunk::Chunk,
    model::{ChunkMeshBuilder, Mesh, MeshVertex, Model, Vertex},
    texture::Texture,
};

/// A trait to be implemented by a render pass to render any arbitrary object.
pub trait Render<'a, T> {
    /// Render a single instance of this value.
    fn draw_object(&mut self, value: &'a T) {
        self.draw_object_instanced(value, 0..1);
    }

    /// Renders the object in the range of instances.
    fn draw_object_instanced(&mut self, value: &'a T, instances: Range<u32>);
}

#[derive(Debug)]
pub struct Renderer<'s> {
    /// The actual physical device responsible for rendering things (most likely the GPU).
    device: wgpu::Device,
    /// The queue of commands being staged to be sent to the `device`.
    queue: wgpu::Queue,
    /// The series of steps that data takes while moving through the rendering process.
    pipeline: wgpu::RenderPipeline,
    /// The depth texture is used to properly sort the pixels while rendering and fix z-index.
    depth_texture: Texture,

    /// A reference to the surface being rendered onto.
    surface: wgpu::Surface<'s>,
    /// The configuration of the `surface`.
    surface_config: wgpu::SurfaceConfiguration,

    /// The model currently being rendered.
    model: crate::model::Model,

    /// A uniform buffer to hold the camera's view-projection matrix.
    camera_uniform: wgpu::Buffer,
    /// The uniform bind group to which the camera's uniform is stored.
    camera_bind_group: wgpu::BindGroup,

    /// The bind group to hold the texture being rendered.
    texture_bind_group: wgpu::BindGroup,
}

impl<'s> Renderer<'s> {
    /// Creates a new renderer given a window as the surface.
    pub async fn new(
        window: &'s Window,
        camera: &Camera,
        chunk: &Chunk,
        texture_bytes: &[u8],
    ) -> Result<Self> {
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
                    required_features: Features::POLYGON_MODE_LINE,
                    required_limits: Limits::default(),
                },
                None,
            )
            .await?;

        let surface_config = Self::get_surface_config(&adapter, &surface, window.inner_size());
        surface.configure(&device, &surface_config);

        let (camera_uniform, camera_bind_group_layout, camera_bind_group) =
            camera.create_buffers(&device);

        let texture = Texture::from_bytes(&device, &queue, texture_bytes, Some("Texture"))?;
        let (texture_bind_group_layout, texture_bind_group) = texture.create_bind_group(&device);

        let depth_texture = Texture::create_depth_texture(&device, &surface_config);

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
        let pipeline = Self::create_pipeline(
            &device,
            &surface_config,
            shader,
            &[&camera_bind_group_layout, &texture_bind_group_layout],
        );

        let (vertices, indices) = ChunkMeshBuilder::new(chunk).build();

        println!("num vertices: {}", vertices.len());
        println!("num indices: {}", indices.len());

        let model = Model::new(Mesh::new(&vertices, &indices, &device));

        Ok(Self {
            device,
            queue,
            pipeline,
            surface,
            surface_config,
            model,
            camera_uniform,
            camera_bind_group,
            texture_bind_group,
            depth_texture,
        })
    }

    /// Creates the rendering pipeline.
    fn create_pipeline(
        device: &Device,
        surface_config: &SurfaceConfiguration,
        shader: ShaderModule,
        bind_group_layouts: &[&BindGroupLayout],
    ) -> RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            push_constant_ranges: &[],
            bind_group_layouts,
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[MeshVertex::desc()],
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
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }

    /// Creates a surface configuration given an adapter, surface, and surface size.
    /// Does not apply the created config to the surface
    fn get_surface_config(
        adapter: &Adapter,
        surface: &Surface,
        size: PhysicalSize<u32>,
    ) -> SurfaceConfiguration {
        let PhysicalSize { width, height } = size;
        let surface_caps = surface.get_capabilities(adapter);

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
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let PhysicalSize { width, height } = size;

        assert!(width > 0, "cannot resize to zero width");
        assert!(height > 0, "cannot resize to zero height");

        self.surface_config.width = width;
        self.surface_config.height = height;

        self.depth_texture = Texture::create_depth_texture(&self.device, &self.surface_config);

        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Updates the camera's uniform buffer with the given view projection matrix.
    pub fn update_camera_buffer(&mut self, view_proj: Mat4) {
        self.queue.write_buffer(
            &self.camera_uniform,
            0,
            bytemuck::cast_slice(&view_proj.to_cols_array()),
        );
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
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.texture_bind_group, &[]);

            render_pass.draw_object(&self.model);
        };

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
