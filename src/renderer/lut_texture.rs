use crate::physics::{BlackbodyLut, DeflectionLut, BB_N, N_B, N_PHI};

pub struct LutTextures {
    pub u_view: wgpu::TextureView,
    pub phi_max_view: wgpu::TextureView,
    pub blackbody_view: wgpu::TextureView,
}

impl LutTextures {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lut: &DeflectionLut,
        blackbody: &BlackbodyLut,
    ) -> Self {
        let u_view = upload(
            device,
            queue,
            "deflection lut u",
            N_B as u32,
            N_PHI as u32,
            &lut.u,
        );
        let phi_max_view = upload(
            device,
            queue,
            "deflection lut phi_max",
            N_B as u32,
            1,
            &lut.phi_max,
        );
        let blackbody_view = upload_rgba16f(
            device,
            queue,
            "blackbody lut",
            BB_N as u32,
            &blackbody.rgba_f16,
        );

        Self {
            u_view,
            phi_max_view,
            blackbody_view,
        }
    }
}

fn upload_rgba16f(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    data: &[u16],
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width,
        height: 1,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: Some(1),
        },
        size,
    );

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    data: &[f32],
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        size,
    );

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
