use std::collections::HashSet;

use glam::{vec3, Vec3};
use wgpu::SurfaceError;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    camera::{Camera, CAMERA_SPEED},
    renderer::Renderer,
};

use anyhow::Result;

/// The main application struct that holds all the data and state of the
/// application.
#[derive(Debug)]
pub struct App<'a> {
    /// The renderer responsible for interacting with wgpu and setting up the
    /// rendering device, and drawing out a scene.
    renderer: crate::renderer::Renderer<'a>,

    /// The camera in 3d space representing the player.
    camera: crate::camera::Camera,

    /// Represents whether the app is currently in focus and locked or not.
    has_focus: bool,

    /// All the keys currently being held down.
    keys_held: HashSet<KeyCode>,
}

impl<'a> App<'a> {
    /// Sets up the renderer and camera.
    pub async fn new(window: &'a Window) -> Result<Self> {
        let camera = Camera::new(vec3(2.5, 0.5, 0.0), Vec3::NEG_X, window.inner_size());

        let renderer = Renderer::new(window, &camera).await?;

        Ok(Self {
            renderer,
            camera,
            has_focus: false,
            keys_held: HashSet::new(),
        })
    }

    /// Updates the app with the latest input state, and renders
    /// onto the surface.
    pub fn update(
        &mut self,
        event: Event<()>,
        elwt: &EventLoopWindowTarget<()>,
        window: &Window,
    ) -> Result<()> {
        match event {
            Event::AboutToWait => {
                window.request_redraw();
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => self.renderer.resize(size),

                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: key,
                            state,
                            ..
                        },
                    ..
                } => {
                    let PhysicalKey::Code(code) = key else {
                            eprintln!("unknown key code, {key:?}");
                            return Ok(());
                    };

                    match state {
                        ElementState::Pressed => self.keys_held.insert(code),
                        ElementState::Released => self.keys_held.remove(&code),
                    };
                }

                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::RedrawRequested => {
                    let _ = 2;

                    self.update_camera_position(0.01);

                    self.render();
                }

                _ => {}
            },

            _ => {}
        }

        Ok(())
    }

    /// Updates the camera's position based on the latest keyboard inputs.
    fn update_camera_position(&mut self, dt: f32) {
        let forward = self.camera.forward;
        let right = forward.cross(self.camera.up);

        let mut delta_pos = Vec3::ZERO;

        if self.keys_held.contains(&KeyCode::KeyW) {
            delta_pos += forward;
        }
        if self.keys_held.contains(&KeyCode::KeyS) {
            delta_pos -= forward;
        }
        if self.keys_held.contains(&KeyCode::KeyD) {
            delta_pos += right;
        }
        if self.keys_held.contains(&KeyCode::KeyA) {
            delta_pos -= right;
        }

        delta_pos = delta_pos.normalize_or_zero();

        self.camera.eye += dt * CAMERA_SPEED * delta_pos;
        self.renderer.update_camera_buffer(self.camera.view_proj());
    }

    /// Renders everything onto the surface.
    fn render(&mut self) {
        match self.renderer.render() {
            Ok(_) => {}
            // If we are out of memory, just quit the app
            Err(SurfaceError::OutOfMemory) => panic!("out of memory - stopping application"),
            // For other errors, they will be gone by the next frame
            Err(error) => eprintln!("{error}"),
        };
    }
}
