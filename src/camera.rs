use glam::*;
use winit::dpi::PhysicalSize;

/// The speed of the camera in space.
pub const CAMERA_SPEED: f32 = 2.0;

/// A perspective camera with a position and orientation in 3D space.
#[derive(Debug)]
pub struct Camera {
    /// The actual position of the camera.
    pub eye: glam::Vec3,
    /// The "forward" vector, representing the direction the camera is looking to.
    pub forward: glam::Vec3,
    /// The vector representing the up direction of the camera
    pub up: glam::Vec3,

    /// The aspect ratio of the surface.
    aspect: f32,
    /// The vertical field of view of the camera in radians.
    fovy: f32,
    /// The near clipping plane of the camera's frustum.
    znear: f32,
}

impl Camera {
    /// Creates a new camera at the given position, target, and window size.
    pub fn new(eye: Vec3, forward: Vec3, window_size: PhysicalSize<u32>) -> Self {
        let PhysicalSize { width, height } = window_size;

        Self {
            eye,
            forward,
            up: Vec3::Y,
            aspect: width as f32 / height as f32,
            fovy: 45.0f32.to_radians(),
            znear: 0.1,
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
}
