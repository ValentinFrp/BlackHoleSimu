mod blackhole_pass;
mod context;
mod lut_texture;
mod sky;
mod texture;
mod uniforms;

use std::sync::Arc;

use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use crate::camera::OrbitCamera;
use crate::physics::{BlackbodyLut, DeflectionLut};
use crate::scene::Scene;
use blackhole_pass::BlackHolePass;
use context::GpuContext;
use uniforms::Uniforms;

#[cfg(not(target_arch = "wasm32"))]
const SKY_SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/milkyway.png");
#[cfg(target_arch = "wasm32")]
const SKY_SOURCE: &str = "assets/milkyway.png";

const DEFAULT_EXPOSURE: f32 = 0.5;

pub struct Renderer {
    ctx: GpuContext,
    pass: BlackHolePass,
    exposure: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, display_handle: OwnedDisplayHandle) -> Self {
        let ctx = GpuContext::new(window, display_handle).await;
        let sky = load_sky().await;
        let lut = DeflectionLut::build();
        let blackbody = BlackbodyLut::build();
        let pass = BlackHolePass::new(
            ctx.device(),
            ctx.queue(),
            ctx.surface_format(),
            &sky,
            &lut,
            &blackbody,
        );
        Self {
            ctx,
            pass,
            exposure: DEFAULT_EXPOSURE,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
    }

    pub fn render(&mut self, camera: &OrbitCamera, scene: &Scene, time: f32) {
        let (width, height) = self.ctx.size();
        let basis = camera.basis(self.ctx.aspect());
        let uniforms = Uniforms::new(
            &basis,
            scene,
            [width as f32, height as f32],
            time,
            self.exposure,
        );
        self.pass.update(self.ctx.queue(), &uniforms);

        let Some(frame) = self.ctx.acquire() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });
        self.pass.record(&mut encoder, &view);
        self.ctx.queue().submit(Some(encoder.finish()));
        self.ctx.window().pre_present_notify();
        frame.present();
    }
}

async fn load_sky() -> sky::SkyImage {
    match sky::load(SKY_SOURCE).await {
        Ok(image) => image,
        Err(error) => {
            log::error!("Fond non chargé ({error}) — fond de secours sombre");
            sky::SkyImage::fallback()
        }
    }
}
