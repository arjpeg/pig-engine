use glam::Vec3;
use wgpu::{util::*, *};

/// A light source in a scene.
#[derive(Clone, Copy)]
pub struct LightSource {
    /// The position of the source.
    pub position: glam::Vec3,
    /// The color emmited by the source.
    pub color: glam::Vec3,
    /// Padding to make this value 32 bytes.
    __padding: [u8; 6],
}

unsafe impl bytemuck::Pod for LightSource {}
unsafe impl bytemuck::Zeroable for LightSource {}

impl LightSource {
    /// Creates a new light source with the given position and color.
    pub fn new(position: Vec3, color: Vec3) -> Self {
        Self {
            position,
            color,
            __padding: [0; 6],
        }
    }

    /// Creates a camera uniform buffer, and binding group (layout).
    pub fn create_buffers(&self, device: &Device) -> (Buffer, BindGroupLayout, BindGroup) {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Light Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Light Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        (uniform_buffer, bind_group_layout, bind_group)
    }
}
