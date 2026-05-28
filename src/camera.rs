use std::f32::consts::PI;

use glam::Vec3;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

const POLAR_EPSILON: f32 = 0.01;

#[derive(Clone, Copy)]
pub struct CameraBasis {
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

pub struct OrbitCamera {
    azimuth: f32,
    polar: f32,
    radius: f32,
    fov_y: f32,
    min_radius: f32,
    max_radius: f32,
}

impl OrbitCamera {
    pub fn new(radius: f32, fov_y_degrees: f32) -> Self {
        Self {
            azimuth: 0.0,
            polar: 1.3,
            radius,
            fov_y: fov_y_degrees.to_radians(),
            min_radius: 2.5,
            max_radius: 200.0,
        }
    }

    pub fn orbit(&mut self, delta_azimuth: f32, delta_polar: f32) {
        self.azimuth += delta_azimuth;
        self.polar = (self.polar + delta_polar).clamp(POLAR_EPSILON, PI - POLAR_EPSILON);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.radius = (self.radius * factor).clamp(self.min_radius, self.max_radius);
    }

    fn position(&self) -> Vec3 {
        let sin_polar = self.polar.sin();
        Vec3::new(
            self.radius * sin_polar * self.azimuth.cos(),
            self.radius * self.polar.cos(),
            self.radius * sin_polar * self.azimuth.sin(),
        )
    }

    pub fn basis(&self, aspect: f32) -> CameraBasis {
        let position = self.position();
        let forward = (-position).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);

        let half_height = (self.fov_y * 0.5).tan();
        CameraBasis {
            position,
            right: right * half_height * aspect,
            up: up * half_height,
            forward,
        }
    }
}

pub struct CameraController {
    dragging: bool,
    last_cursor: Option<(f64, f64)>,
    orbit_sensitivity: f32,
    zoom_sensitivity: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            dragging: false,
            last_cursor: None,
            orbit_sensitivity: 0.005,
            zoom_sensitivity: 0.1,
        }
    }
}

impl CameraController {
    pub fn on_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left {
            self.dragging = state == ElementState::Pressed;
            if !self.dragging {
                self.last_cursor = None;
            }
        }
    }

    pub fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>, camera: &mut OrbitCamera) {
        let current = (position.x, position.y);
        if self.dragging {
            if let Some((last_x, last_y)) = self.last_cursor {
                let delta_x = (current.0 - last_x) as f32;
                let delta_y = (current.1 - last_y) as f32;
                camera.orbit(
                    delta_x * self.orbit_sensitivity,
                    -delta_y * self.orbit_sensitivity,
                );
            }
        }
        self.last_cursor = Some(current);
    }

    pub fn on_scroll(&mut self, delta: MouseScrollDelta, camera: &mut OrbitCamera) {
        let amount = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => position.y as f32 * 0.01,
        };
        camera.zoom(1.0 - amount * self.zoom_sensitivity);
    }
}
