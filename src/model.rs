use std::mem;

use anyhow::Result;
use wgpu::{util::*, *};

use crate::renderer::Render;

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
            array_stride: mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

/// A vertex in a mesh sent to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    /// The 3d position of the vertex in the (right-hand based) world.
    pub pos: [f32; 3],
    /// The normal vector of the vertex.
    pub normal: [f32; 3],
}

/// A model in the world, consisting of its mesh(es) and material(s).
/// TODO: make meshes and materials just store indices to reduce duplicate assets
/// being loaded into memory.
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
    /// The indices uploaded to the gpu. Stored in the `u32` format.
    pub index_buffer: wgpu::Buffer,

    /// The number of vertices present in the buffer.
    pub count: u32,
}

impl Model {
    /// Creates a new model with the given mesh.
    pub fn new(mesh: Mesh) -> Self {
        Self { mesh }
    }

    /// Loads a mesh from the given path, which is interpreted as an object file
    /// and triangulated to make a mesh.
    pub fn load_from_file(file_name: &str, device: &Device) -> Result<Self> {
        let (models, _) = tobj::load_obj(file_name, &tobj::GPU_LOAD_OPTIONS)?;

        let meshes = models
            .iter()
            .map(|m| Mesh::from_tobj_mesh(&m.mesh, device))
            .collect::<Vec<_>>();

        let mesh = meshes.into_iter().nth(0).unwrap();

        Ok(Self { mesh })
    }
}

impl Mesh {
    /// Loads and uploads the mesh data from a tobj Mesh value.
    fn from_tobj_mesh(mesh: &tobj::Mesh, device: &Device) -> Self {
        let vertices = (0..mesh.positions.len() / 3)
            .map(|i| MeshVertex {
                pos: [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ],
                normal: [
                    mesh.normals[i * 3],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                ],
            })
            .collect::<Vec<_>>();

        Self::new(&vertices, &mesh.indices, device)
    }

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
    ];
}

impl<'a, 'rp> Render<'a, Model> for RenderPass<'rp>
where
    'a: 'rp,
{
    fn draw_object_instanced(&mut self, model: &'a Model, instances: std::ops::Range<u32>) {
        let Mesh {
            vertex_buffer,
            index_buffer,
            count,
        } = &model.mesh;

        self.set_vertex_buffer(0, vertex_buffer.slice(..));
        self.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);
        self.draw_indexed(0..*count, 0, instances);
    }
}
