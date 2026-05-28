struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var out: VsOut;
    out.uv = uv;
    out.clip_position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let origin = u.cam_position;
    let dir = ray_direction(in.uv);

    let t_horizon = intersect_sphere(origin, dir, u.schwarzschild_radius);
    let t_disk = intersect_disk(origin, dir);

    let disk_visible = t_disk > 0.0 && (t_horizon < 0.0 || t_disk < t_horizon);

    var color: vec3<f32>;
    if disk_visible {
        color = disk_emission(origin + t_disk * dir);
    } else if t_horizon > 0.0 {
        color = vec3<f32>(0.0);
    } else {
        color = sample_sky(dir);
    }

    return vec4<f32>(aces_tonemap(color * u.exposure), 1.0);
}
