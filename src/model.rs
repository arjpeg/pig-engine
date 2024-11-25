use crate::renderer::Render;
use glam::*;
use wgpu::{util::*, *};

/// Any format of vertex sent to the GPU.
pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable {
    /// The vertex attributes of how the data is structured.
    const ATTRIBS: &'static [VertexAttribute];

    /// Returns the wgpu vertex buffer layout of how each vertex is interpreted.
    fn desc() -> VertexBufferLayout<'static>
    where
        Self: Sized,
    {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

/// A vertex in a mesh sent to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    /// The 3d position of the vertex.
    pub pos: [f32; 3],
    /// The normal vector of the vertex.
    pub normal: [f32; 3],
    /// The index into which texture to use. The order is determined
    /// by which textures were loaded first.
    pub texture_index: u32,
}

/// A model in the world, consisting of its mesh(es) and material(s).
#[derive(Debug)]
pub struct Model {
    pub mesh: Mesh,
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

impl Model {
    /// Creates a new model with the given mesh.
    pub fn new(mesh: Mesh) -> Self {
        Self { mesh }
    }
}

impl Mesh {
    // Creates a new mesh and uploads the given vertex and index data to the GPU.
    pub fn new<T: Vertex>(vertices: &[T], indices: &[u32], device: &Device) -> Self {
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

impl Vertex for MeshVertex {
    const ATTRIBS: &'static [VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Uint32
    ];
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
