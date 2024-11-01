use std::{collections::HashSet, f32::consts::FRAC_PI_2};

use glam::*;
use wgpu::{util::*, *};

use winit::{dpi::PhysicalSize, keyboard::KeyCode};

/// The normal speed of the camera in space.
pub const CAMERA_NORMAL_SPEED: f32 = 20.0;
/// The speed of the camera when the boost speed key (L_CTRL) is pressed.
pub const CAMERA_BOOST_SPEED: f32 = 350.0;
/// The speed of the camera when the slow modifier key (L_ALT) is pressed.
pub const CAMERA_SLOW_SPEED: f32 = 10.0;

/// The sensitivity of the camera.
pub const CAMERA_SENSITIVITY: f32 = 0.15;

/// A perspective camera with a position and orientation in 3D space.
#[derive(Debug)]
pub struct Camera {
    /// The actual position of the camera.
    pub eye: glam::Vec3,
    /// The "forward" vector, representing the direction the camera is looking to.
    pub forward: glam::Vec3,
    /// The vector representing the up direction of the camera
    pub up: glam::Vec3,

    /// The yaw of the camera in radians.
    yaw: f32,
    /// The pitch of the camera in radians. Clamped to [-pi/2, pi/2].
    pitch: f32,

    /// The aspect ratio of the surface.
    aspect: f32,
    /// The vertical field of view of the camera in radians.
    fovy: f32,
    /// The near clipping plane of the camera's frustum.
    znear: f32,
}

impl Camera {
    /// Calculates the forward vector from the yaw and pitch of the camera.
    pub fn calculate_forward(yaw: f32, pitch: f32) -> Vec3 {
        vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
    }

    /// Creates a new camera at the given position, looking at the target, and window size to
    /// calculate the aspect ratio.
    pub fn new(eye: Vec3, yaw: f32, pitch: f32, window_size: PhysicalSize<u32>) -> Self {
        let PhysicalSize { width, height } = window_size;

        let forward = Self::calculate_forward(yaw, pitch);

        Self {
            eye,
            forward,
            up: Vec3::Y,
            aspect: width as f32 / height as f32,
            fovy: 45.0f32.to_radians(),
            znear: 0.01,
            yaw,
            pitch,
        }
    }

    /// Returns the view-projection matrix of the camera to transform vertices.
    /// Follows the canonical WebGPU coordinate depth in range [0, 1], unlike OpenGL's [-1, 1]
    /// range.
    pub fn view_proj(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.forward + self.eye, self.up);
        let proj = Mat4::perspective_infinite_rh(self.fovy, self.aspect, self.znear);

        proj * view
    }

    /// Updates the camera's orientation (yaw/pitch) based on the mouse move delta
    pub fn update_orientation(&mut self, delta: (f64, f64), dt: f32) {
        let (dx, dy) = delta;

        self.yaw += dt * dx as f32 * CAMERA_SENSITIVITY;
        self.pitch -= dt * dy as f32 * CAMERA_SENSITIVITY;

        self.pitch = self.pitch.clamp(-FRAC_PI_2 + 0.001, FRAC_PI_2 - 0.001);
        self.forward = Self::calculate_forward(self.yaw, self.pitch);
    }

    /// Updates the camera's position based on the keys held.
    pub fn update_position(&mut self, keys_held: &HashSet<KeyCode>, dt: f32) {
        let forward = self.forward;
        let right = forward.cross(self.up);

        let mut delta_pos = Vec3::ZERO;

        if keys_held.contains(&KeyCode::KeyW) {
            delta_pos += forward;
        }
        if keys_held.contains(&KeyCode::KeyS) {
            delta_pos -= forward;
        }
        if keys_held.contains(&KeyCode::KeyD) {
            delta_pos += right;
        }
        if keys_held.contains(&KeyCode::KeyA) {
            delta_pos -= right;
        }

        delta_pos.y = 0.0;

        if keys_held.contains(&KeyCode::Space) {
            delta_pos += Vec3::Y;
        }
        if keys_held.contains(&KeyCode::ShiftLeft) {
            delta_pos -= Vec3::Y;
        }

        delta_pos = delta_pos.normalize_or_zero();

        let speed = if keys_held.contains(&KeyCode::ControlLeft) {
            CAMERA_BOOST_SPEED
        } else if keys_held.contains(&KeyCode::AltLeft) {
            CAMERA_SLOW_SPEED
        } else {
            CAMERA_NORMAL_SPEED
        };

        self.eye += dt * speed * delta_pos;
    }

    /// Recalculates the aspect ratio given the new window size
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let PhysicalSize { width, height } = size;

        self.aspect = width as f32 / height as f32;
    }

    /// Creates a camera uniform buffer, and binding group (layout).
    pub fn create_buffers(&self, device: &Device) -> (Buffer, BindGroupLayout, BindGroup) {
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&self.view_proj().to_cols_array()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        (uniform_buffer, bind_group_layout, bind_group)
    }
}
