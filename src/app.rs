use std::{collections::HashSet, time::Instant};

use egui::Context;
use glam::*;
use wgpu::SurfaceError;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use crate::{camera::Camera, chunk_manager::ChunkManager, renderer::Renderer};

use anyhow::Result;

/// The main application struct that holds all the data and state of the
/// application.
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

    /// The time of the last rendering frame.
    last_frame: std::time::Instant,

    /// The chunk manager used to manage chunks around the player.
    chunk_manager: crate::chunk_manager::ChunkManager,
}

impl<'a> App<'a> {
    /// Sets up the renderer and camera.
    pub async fn new(window: &'a Window) -> Result<Self> {
        let camera = Camera::new(
            vec3(0.0, 50.0, 0.0),
            -90.0f32.to_radians(),
            -15.0f32.to_radians(),
            window.inner_size(),
        );

        let renderer = Renderer::new(window, &camera).await?;

        Ok(Self {
            renderer,
            camera,
            has_focus: false,
            keys_held: HashSet::new(),
            last_frame: Instant::now(),
            chunk_manager: ChunkManager::new(),
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

                    self.chunk_manager.update(self.camera.eye);
                    self.chunk_manager
                        .resolve_mesh_uploads(&self.renderer.device);

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
        let meshes = self.chunk_manager.loaded_meshes();

        match self.renderer.render(&meshes, |ui| {
            Self::ui(ui, &self.camera, &self.chunk_manager)
        }) {
            Ok(_) => {}
            // If we are out of memory, just quit the app
            Err(SurfaceError::OutOfMemory) => panic!("out of memory - stopping application"),
            // For other errors, they will be gone by the next frame
            Err(error) => eprintln!("{error}"),
        };
    }

    /// Renders all egui windows.
    fn ui(ui: &Context, camera: &Camera, chunk_manager: &ChunkManager) {
        use egui::*;

        Window::new("debug").show(ui, |ui| {
            ui.label(format!("position: {:?}", camera.eye));

            ui.label(format!("chunks loaded: {}", chunk_manager.chunks_loaded()));
            ui.label(format!("meshes built: {}", chunk_manager.meshes_loaded()));
        });
    }
}
