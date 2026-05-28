const PI: f32 = 3.141592653589793;
const TAU: f32 = 6.283185307179586;

struct Uniforms {
    cam_position: vec3<f32>,
    cam_right: vec3<f32>,
    cam_up: vec3<f32>,
    cam_forward: vec3<f32>,
    resolution: vec2<f32>,
    time: f32,
    schwarzschild_radius: f32,
    disk_inner: f32,
    disk_outer: f32,
    exposure: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var sky_texture: texture_2d<f32>;
@group(1) @binding(1) var sky_sampler: sampler;

fn ray_direction(uv: vec2<f32>) -> vec3<f32> {
    let ndc = uv * 2.0 - 1.0;
    return normalize(u.cam_forward + ndc.x * u.cam_right + ndc.y * u.cam_up);
}

fn intersect_sphere(origin: vec3<f32>, dir: vec3<f32>, radius: f32) -> f32 {
    let b = dot(origin, dir);
    let c = dot(origin, origin) - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return -1.0;
    }
    let root = sqrt(discriminant);
    let near = -b - root;
    if near > 0.0 {
        return near;
    }
    let far = -b + root;
    if far > 0.0 {
        return far;
    }
    return -1.0;
}

fn intersect_disk(origin: vec3<f32>, dir: vec3<f32>) -> f32 {
    if abs(dir.y) < 1e-6 {
        return -1.0;
    }
    let t = -origin.y / dir.y;
    if t <= 0.0 {
        return -1.0;
    }
    let radius = length((origin + t * dir).xz);
    if radius < u.disk_inner || radius > u.disk_outer {
        return -1.0;
    }
    return t;
}

fn sample_sky(dir: vec3<f32>) -> vec3<f32> {
    let longitude = atan2(dir.z, dir.x) / TAU + 0.5;
    let latitude = acos(clamp(dir.y, -1.0, 1.0)) / PI;
    return textureSampleLevel(sky_texture, sky_sampler, vec2<f32>(longitude, latitude), 0.0).rgb;
}

fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn disk_emission(hit: vec3<f32>) -> vec3<f32> {
    let radius = length(hit.xz);
    let t = clamp((radius - u.disk_inner) / (u.disk_outer - u.disk_inner), 0.0, 1.0);
    let inner_color = vec3<f32>(1.0, 0.9, 0.6);
    let outer_color = vec3<f32>(0.75, 0.16, 0.03);
    let brightness = mix(1.6, 0.25, t);
    return mix(inner_color, outer_color, t) * brightness;
}
