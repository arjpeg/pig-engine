use std::{
    collections::HashSet,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use glam::*;
use noise::Simplex;
use wgpu::SurfaceError;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use crate::{camera::Camera, chunk::*, renderer::Renderer};

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

    /// The only chunk in the world for now.
    chunk: crate::chunk::Chunk,

    /// Represents whether the app is currently in focus and locked or not.
    has_focus: bool,

    /// All the keys currently being held down.
    keys_held: HashSet<KeyCode>,

    /// The time of the last rendering frame.
    last_frame: std::time::Instant,

    /// The noise generator used to generate terrain, etc.
    noise: Simplex,
}

impl<'a> App<'a> {
    /// Sets up the renderer and camera.
    pub async fn new(window: &'a Window) -> Result<Self> {
        let camera = Camera::new(
            vec3(10.0, 10.0, 7.0),
            0.0,
            -20.0f32.to_radians(),
            window.inner_size(),
        );

        let seed = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let noise = Simplex::new(seed.as_secs() as u32);

        let populator = SinglesPopulator::new(vec![
            (uvec3(8, 8, 8), Voxel::Grass),
            (uvec3(4, 8, 8), Voxel::Grass),
        ])?;
        let chunk = Chunk::new(ivec2(0, 0), &populator);

        let renderer = Renderer::new(window, &camera, &chunk).await?;

        Ok(Self {
            renderer,
            camera,
            chunk,
            noise,
            has_focus: false,
            keys_held: HashSet::new(),
            last_frame: Instant::now(),
        })
    }

    /// Returns the time elapsed since the last frame, in seconds
    fn delta_time(&self) -> f32 {
        (Instant::now() - self.last_frame).as_secs_f32()
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
                WindowEvent::Resized(size) => {
                    self.renderer.resize(size);
                    self.camera.resize(size);
                }

                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            state: ElementState::Pressed,
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
                    if self.has_focus {
                        self.camera
                            .update_position(&self.keys_held, self.delta_time());
                    }

                    self.last_frame = Instant::now();

                    self.renderer.update_camera_buffer(self.camera.view_proj());
                    self.render();
                }

                _ => {}
            },

            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } if self.has_focus => {
                self.camera.update_orientation(delta, self.delta_time());
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

            self.keys_held.clear();
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
