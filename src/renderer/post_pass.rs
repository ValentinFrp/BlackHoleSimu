use bytemuck::{Pod, Zeroable};

use crate::renderer::offscreen::{RenderTargets, HDR_FORMAT};

#[derive(Clone, Copy)]
pub struct PostSettings {
    pub exposure: f32,
    pub bloom_strength: f32,
    pub bloom_threshold: f32,
}

impl Default for PostSettings {
    fn default() -> Self {
        Self {
            exposure: 0.5,
            bloom_strength: 0.6,
            bloom_threshold: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostParams {
    exposure: f32,
    bloom_strength: f32,
    bloom_threshold: f32,
    _pad: f32,
}

pub struct PostPass {
    bright: wgpu::RenderPipeline,
    blur_h: wgpu::RenderPipeline,
    blur_v: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    sample_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

impl PostPass {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let post_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/post.wgsl").into()),
        });
        let composite_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/composite.wgsl").into()),
        });

        let sample_layout = sample_bind_group_layout(device);
        let composite_layout = composite_bind_group_layout(device);

        let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post layout"),
            bind_group_layouts: &[Some(&sample_layout)],
            immediate_size: 0,
        });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("composite layout"),
                bind_group_layouts: &[Some(&composite_layout)],
                immediate_size: 0,
            });

        let bright = fullscreen_pipeline(
            device,
            &post_layout,
            &post_module,
            "fs_bright",
            HDR_FORMAT,
        );
        let blur_h = fullscreen_pipeline(
            device,
            &post_layout,
            &post_module,
            "fs_blur_h",
            HDR_FORMAT,
        );
        let blur_v = fullscreen_pipeline(
            device,
            &post_layout,
            &post_module,
            "fs_blur_v",
            HDR_FORMAT,
        );
        let composite = fullscreen_pipeline(
            device,
            &composite_pipeline_layout,
            &composite_module,
            "fs_composite",
            surface_format,
        );

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post params"),
            size: std::mem::size_of::<PostParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            bright,
            blur_h,
            blur_v,
            composite,
            sample_layout,
            composite_layout,
            params_buffer,
            sampler,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, settings: &PostSettings) {
        let params = PostParams {
            exposure: settings.exposure,
            bloom_strength: settings.bloom_strength,
            bloom_threshold: settings.bloom_threshold,
            _pad: 0.0,
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn record(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        targets: &RenderTargets,
        output: &wgpu::TextureView,
    ) {
        let bright_group = self.sample_group(device, &targets.scene_view);
        let blur_h_group = self.sample_group(device, &targets.bloom_views[0]);
        let blur_v_group = self.sample_group(device, &targets.bloom_views[1]);
        let composite_group = self.composite_group(device, &targets.scene_view, &targets.bloom_views[0]);

        run(encoder, &self.bright, &bright_group, &targets.bloom_views[0]);
        run(encoder, &self.blur_h, &blur_h_group, &targets.bloom_views[1]);
        run(encoder, &self.blur_v, &blur_v_group, &targets.bloom_views[0]);
        run(encoder, &self.composite, &composite_group, output);
    }

    fn sample_group(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post sample group"),
            layout: &self.sample_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn composite_group(
        &self,
        device: &wgpu::Device,
        scene: &wgpu::TextureView,
        bloom: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite group"),
            layout: &self.composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(bloom),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        })
    }
}

fn run(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    target: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("post pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}

fn sample_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("post sample layout"),
        entries: &[
            texture_entry(0),
            sampler_entry(1),
            uniform_entry(2),
        ],
    })
}

fn composite_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("composite layout"),
        entries: &[
            texture_entry(0),
            texture_entry(1),
            sampler_entry(2),
            uniform_entry(3),
        ],
    })
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    fn validate(source: &str) {
        let module = naga::front::wgsl::parse_str(source).expect("le WGSL doit parser");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("le WGSL doit être valide");
    }

    #[test]
    fn post_shaders_compile() {
        validate(include_str!("../../shaders/post.wgsl"));
        validate(include_str!("../../shaders/composite.wgsl"));
    }
}

fn fullscreen_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    fragment_entry: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("post pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_fullscreen"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
