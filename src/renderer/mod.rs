mod blackhole_pass;
mod context;
mod lut_texture;
mod offscreen;
mod post_pass;
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
use offscreen::{RenderTargets, HDR_FORMAT};
use post_pass::PostPass;
use uniforms::Uniforms;

pub use post_pass::PostSettings;

#[cfg(not(target_arch = "wasm32"))]
const SKY_SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/milkyway.png");
#[cfg(target_arch = "wasm32")]
const SKY_SOURCE: &str = "assets/milkyway.png";

const DEFAULT_SUPERSAMPLE: u32 = 2;

pub struct Renderer {
    ctx: GpuContext,
    pass: BlackHolePass,
    post: PostPass,
    targets: RenderTargets,
    settings: PostSettings,
    supersample: u32,
    egui: egui_wgpu::Renderer,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, display_handle: OwnedDisplayHandle) -> Self {
        let ctx = GpuContext::new(window, display_handle).await;
        let sky = load_sky().await;
        let lut = DeflectionLut::build();
        let blackbody = BlackbodyLut::build();
        let pass = BlackHolePass::new(ctx.device(), ctx.queue(), HDR_FORMAT, &sky, &lut, &blackbody);
        let post = PostPass::new(ctx.device(), ctx.surface_format());
        let supersample = DEFAULT_SUPERSAMPLE;
        let targets = RenderTargets::new(ctx.device(), ctx.size(), supersample);
        let egui = egui_wgpu::Renderer::new(
            ctx.device(),
            ctx.surface_format(),
            egui_wgpu::RendererOptions::default(),
        );
        Self {
            ctx,
            pass,
            post,
            targets,
            settings: PostSettings::default(),
            supersample,
            egui,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
        self.targets = RenderTargets::new(self.ctx.device(), self.ctx.size(), self.supersample);
    }

    pub fn settings_mut(&mut self) -> &mut PostSettings {
        &mut self.settings
    }

    pub fn supersample(&self) -> u32 {
        self.supersample
    }

    pub fn set_supersample(&mut self, supersample: u32) {
        if supersample != self.supersample {
            self.supersample = supersample;
            self.targets = RenderTargets::new(self.ctx.device(), self.ctx.size(), supersample);
        }
    }

    pub fn render(
        &mut self,
        camera: &OrbitCamera,
        scene: &Scene,
        time: f32,
        ui_jobs: Vec<egui::ClippedPrimitive>,
        ui_textures: egui::TexturesDelta,
        pixels_per_point: f32,
    ) {
        let (width, height) = self.ctx.size();
        let basis = camera.basis(self.ctx.aspect());
        let uniforms = Uniforms::new(&basis, scene, [width as f32, height as f32], time);
        self.pass.update(self.ctx.queue(), &uniforms);
        self.post.update(self.ctx.queue(), &self.settings);

        let Some(frame) = self.ctx.acquire() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let device = self.ctx.device();
        let queue = self.ctx.queue();
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });
        self.pass.record(&mut encoder, &self.targets.scene_view);
        self.post.record(device, &mut encoder, &self.targets, &view);

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };
        for (id, delta) in &ui_textures.set {
            self.egui.update_texture(device, queue, *id, delta);
        }
        let ui_buffers = self
            .egui
            .update_buffers(device, queue, &mut encoder, &ui_jobs, &screen);
        {
            let mut ui_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui.render(&mut ui_pass, &ui_jobs, &screen);
        }

        queue.submit(ui_buffers.into_iter().chain(std::iter::once(encoder.finish())));
        self.ctx.window().pre_present_notify();
        frame.present();
        for id in &ui_textures.free {
            self.egui.free_texture(id);
        }
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
