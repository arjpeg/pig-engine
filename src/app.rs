use std::collections::HashSet;

use glam::vec3;
use wgpu::SurfaceError;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use crate::{camera::Camera, renderer::Renderer};

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
        let camera = Camera::new(
            vec3(2.5, 0.0, 0.0),
            180.0f32.to_radians(),
            0.0,
            window.inner_size(),
        );

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
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    self.toggle_focus(window);
                }

                WindowEvent::MouseInput { .. } if !self.has_focus => {
                    self.toggle_focus(window);
                }

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
                    self.camera.update_position(&self.keys_held, 0.01);

                    //println!("{:?}", self.camera.forward);

                    self.renderer.update_camera_buffer(self.camera.view_proj());
                    self.render();
                }

                _ => {}
            },

            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } if self.has_focus => {
                self.camera.update_orientation(delta, 0.01);
            }

            _ => {}
        }

        Ok(())
    }

    /// Toggles the current focus state of the app.
    fn toggle_focus(&mut self, window: &Window) {
        self.has_focus = !self.has_focus;

        if self.has_focus {
            window.set_cursor_visible(false);
            window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
        } else {
            window.set_cursor_visible(true);
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
        }
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
