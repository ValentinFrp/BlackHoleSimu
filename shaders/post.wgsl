struct PostParams {
    exposure: f32,
    bloom_strength: f32,
    bloom_threshold: f32,
    _pad: f32,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var out: VsOut;
    out.uv = uv;
    out.clip_position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: PostParams;

const WEIGHTS = array<f32, 5>(0.227027, 0.194594, 0.121622, 0.054054, 0.016216);

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn fs_bright(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, in.uv).rgb;
    let lum = luminance(color);
    let contribution = max(lum - params.bloom_threshold, 0.0) / max(lum, 1.0e-4);
    return vec4<f32>(color * contribution, 1.0);
}

fn blur(uv: vec2<f32>, direction: vec2<f32>) -> vec3<f32> {
    let texel = direction / vec2<f32>(textureDimensions(source_texture));
    var result = textureSample(source_texture, source_sampler, uv).rgb * WEIGHTS[0];
    for (var i = 1; i < 5; i = i + 1) {
        let offset = texel * f32(i);
        result += textureSample(source_texture, source_sampler, uv + offset).rgb * WEIGHTS[i];
        result += textureSample(source_texture, source_sampler, uv - offset).rgb * WEIGHTS[i];
    }
    return result;
}

@fragment
fn fs_blur_h(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(blur(in.uv, vec2<f32>(1.0, 0.0)), 1.0);
}

@fragment
fn fs_blur_v(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(blur(in.uv, vec2<f32>(0.0, 1.0)), 1.0);
}
