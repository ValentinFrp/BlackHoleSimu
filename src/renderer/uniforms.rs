use bytemuck::{Pod, Zeroable};

use crate::camera::CameraBasis;
use crate::scene::Scene;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    cam_position: [f32; 3],
    _pad0: f32,
    cam_right: [f32; 3],
    _pad1: f32,
    cam_up: [f32; 3],
    _pad2: f32,
    cam_forward: [f32; 3],
    _pad3: f32,
    resolution: [f32; 2],
    time: f32,
    schwarzschild_radius: f32,
    disk_inner: f32,
    disk_outer: f32,
    exposure: f32,
    disk_temperature: f32,
}

impl Uniforms {
    pub fn new(
        camera: &CameraBasis,
        scene: &Scene,
        resolution: [f32; 2],
        time: f32,
        exposure: f32,
    ) -> Self {
        Self {
            cam_position: camera.position.to_array(),
            _pad0: 0.0,
            cam_right: camera.right.to_array(),
            _pad1: 0.0,
            cam_up: camera.up.to_array(),
            _pad2: 0.0,
            cam_forward: camera.forward.to_array(),
            _pad3: 0.0,
            resolution,
            time,
            schwarzschild_radius: scene.black_hole.schwarzschild_radius,
            disk_inner: scene.disk.inner_radius,
            disk_outer: scene.disk.outer_radius,
            exposure,
            disk_temperature: scene.disk.peak_temperature,
        }
    }
}
