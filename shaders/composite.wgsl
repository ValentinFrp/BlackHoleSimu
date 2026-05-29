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

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var bloom_texture: texture_2d<f32>;
@group(0) @binding(2) var composite_sampler: sampler;
@group(0) @binding(3) var<uniform> params: PostParams;

fn aces(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_texture, composite_sampler, in.uv).rgb;
    let bloom = textureSample(bloom_texture, composite_sampler, in.uv).rgb;
    let hdr = (scene + bloom * params.bloom_strength) * params.exposure;
    return vec4<f32>(aces(hdr), 1.0);
}
