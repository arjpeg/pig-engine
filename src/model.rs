use anyhow::Result;
use std::mem;

use crate::{chunk::*, renderer::Render};
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
    /// The 3d position of the vertex.
    pub pos: [f32; 3],
    /// The normal vector of the vertex.
    pub normal: [f32; 3],
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

/// Generates a mesh for a given chunk.
#[derive(Debug)]
pub struct ChunkMeshBuilder<'a> {
    /// The chunk whose mesh is being built.
    chunk: &'a crate::chunk::Chunk,

    /// The vertices generated so far.
    vertices: Vec<MeshVertex>,
    /// The indices generated so far.
    indices: Vec<u32>,
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

impl<'c> ChunkMeshBuilder<'c> {
    const FACE_NORMALS: [[isize; 3]; 6] = [
        [0, 1, 0],  // up
        [0, -1, 0], // down
        [1, 0, 0],  // right
        [-1, 0, 0], // left
        [0, 0, 1],  // front
        [0, 0, -1], // back
    ];

    const FACE_VERTICES: [[[f32; 3]; 4]; 6] = [
        // up
        [
            [-0.5, 0.5, -0.5],
            [-0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [0.5, 0.5, -0.5],
        ],
        // down
        [
            [-0.5, -0.5, 0.5],  // 1
            [-0.5, -0.5, -0.5], // 0
            [0.5, -0.5, -0.5],  // 3
            [0.5, -0.5, 0.5],   // 2
        ],
        // right
        [
            [0.5, 0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
        ],
        // left
        [
            [-0.5, 0.5, 0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, -0.5],
            [-0.5, -0.5, 0.5],
        ],
        // front
        [
            [-0.5, 0.5, 0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
        ],
        // back
        [
            [-0.5, -0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
            [0.5, -0.5, -0.5],
        ],
    ];

    /// Creates a new chunk mesh builder given a chunk.
    pub fn new(chunk: &'c Chunk) -> Self {
        Self {
            chunk,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Builds the vertices and indices for the chunk.
    pub fn build(mut self) -> (Vec<MeshVertex>, Vec<u32>) {
        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_WIDTH {
                for x in 0..CHUNK_WIDTH {
                    self.add_block([x, y, z]);
                }
            }
        }

        (self.vertices, self.indices)
    }

    fn add_block(&mut self, block_pos: [usize; 3]) {
        if !self.chunk.is_block_full(block_pos) {
            return;
        }

        let [x, y, z] = block_pos;

        for (index, face_normal) in Self::FACE_NORMALS.iter().enumerate() {
            if let Some(neighbor) = Chunk::get_block_in_direction(block_pos, *face_normal) {
                if self.chunk.is_block_full(neighbor) {
                    // the neighbor was out of bounds
                    continue;
                }
            };

            let [fx, fy, fz] = *face_normal;
            let normal = [fx as f32, fy as f32, fz as f32];

            let Some(vertices) = Self::FACE_VERTICES.get(index) else { continue; };

            for vertex in vertices {
                // the local position offset of the vertex relative
                // to its center
                let [lx, ly, lz] = vertex;

                let pos = [x as f32 + lx, y as f32 + ly, z as f32 + lz];

                self.vertices.push(MeshVertex { pos, normal });
            }

            let offset = self
                .indices
                .get(self.indices.len().saturating_sub(2))
                .copied()
                .map(|i| i + 1)
                .unwrap_or(0) as u32;

            self.indices.extend([0, 1, 2, 2, 3, 0].map(|i| i + offset));
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
