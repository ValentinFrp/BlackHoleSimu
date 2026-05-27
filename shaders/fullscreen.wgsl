struct Uniforms {
    time: f32,
    _pad: f32,
    resolution: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    let uv = vec2<f32>(f32((vid << 1u) & 2u), f32(vid & 2u));
    var out: VsOut;
    out.uv = uv;
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let r = 0.5 + 0.5 * sin(u.time);
    let g = 0.5 + 0.5 * sin(u.time + 2.094395);
    let b = 0.5 + 0.5 * sin(u.time + 4.188790);
    return vec4<f32>(r, g, b, 1.0);
}
