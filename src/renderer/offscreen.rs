pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const BLOOM_DOWNSCALE: u32 = 4;

pub struct RenderTargets {
    pub scene_view: wgpu::TextureView,
    pub bloom_views: [wgpu::TextureView; 2],
    _scene: wgpu::Texture,
    _bloom: [wgpu::Texture; 2],
}

impl RenderTargets {
    pub fn new(device: &wgpu::Device, surface_size: (u32, u32), supersample: u32) -> Self {
        let render_size = (
            (surface_size.0 * supersample).max(1),
            (surface_size.1 * supersample).max(1),
        );
        let bloom_size = (
            (surface_size.0 / BLOOM_DOWNSCALE).max(1),
            (surface_size.1 / BLOOM_DOWNSCALE).max(1),
        );

        let scene = color_target(device, "scene hdr", render_size);
        let bloom0 = color_target(device, "bloom 0", bloom_size);
        let bloom1 = color_target(device, "bloom 1", bloom_size);

        Self {
            scene_view: scene.create_view(&Default::default()),
            bloom_views: [
                bloom0.create_view(&Default::default()),
                bloom1.create_view(&Default::default()),
            ],
            _scene: scene,
            _bloom: [bloom0, bloom1],
        }
    }
}

fn color_target(device: &wgpu::Device, label: &str, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
