use std::{collections::HashSet, f32::consts::FRAC_PI_2};

use glam::*;
use winit::{dpi::PhysicalSize, keyboard::KeyCode};

/// The normal speed of the camera in space.
pub const CAMERA_NORMAL_SPEED: f32 = 5.0;
/// The speed of the camera when the boost speed key (L_CTRL) is pressed.
pub const CAMERA_BOOST_SPEED: f32 = 10.0;
/// The speed of the camera when the slow modifier key (L_ALT) is pressed.
pub const CAMERA_SLOW_SPEED: f32 = 1.25;

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

        self.pitch = self.pitch.clamp(-FRAC_PI_2, FRAC_PI_2);
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
}
