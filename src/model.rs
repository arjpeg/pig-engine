use crate::renderer::Render;
use wgpu::{util::*, *};

/// A vertex in a mesh sent to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    /// The 3d position of the vertex.
    pub pos: glam::Vec3,
    /// The normal vector of the vertex.
    pub normal: glam::Vec3,
    /// The first sixteen bits are an index into which texture layer to use, then the latter 16
    /// bits represent the ambient occlusion value for this vertex.
    pub texture_ambient: u32,
}

/// A mesh consists of a set of vertices connected by edges in triangles
/// (the indices).
#[derive(Debug)]
pub struct Mesh {
    /// The vertices uploaded to the gpu.
    pub vertex_buffer: wgpu::Buffer,
    /// The indices uploaded to the gpu. Stored as a list of `u32`s.
    pub index_buffer: wgpu::Buffer,

    /// The number of vertices present in the buffer.
    pub count: u32,
}

impl Mesh {
    // Creates a new mesh and uploads the given vertex and index data to the GPU.
    pub fn new(vertices: &[MeshVertex], indices: &[u32], device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX,
        });

        let count = indices.len() as u32;

        Self {
            vertex_buffer,
            index_buffer,
            count,
        }
    }
}

impl MeshVertex {
    /// The vertex attributes of how the data is structured.
    const ATTRIBS: &'static [VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Uint32
    ];

    /// Returns the wgpu vertex buffer layout of how each vertex is interpreted.
    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

impl<'a, 'rp> Render<'a, Mesh> for RenderPass<'rp>
where
    'a: 'rp,
{
    fn draw_object_instanced(&mut self, mesh: &'a Mesh, instances: std::ops::Range<u32>) {
        let Mesh {
            vertex_buffer,
            index_buffer,
            count,
        } = mesh;

        self.set_vertex_buffer(0, vertex_buffer.slice(..));
        self.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);

        self.draw_indexed(0..*count, 0, instances);
    }
}
