const PI: f32 = 3.141592653589793;
const TAU: f32 = 6.283185307179586;

const N_B: f32 = 1024.0;
const N_PHI: f32 = 512.0;
const SPLIT: f32 = 256.0;
const B_CRIT: f32 = 2.598076211353316;
const B_MAX: f32 = 64.0;

const RADIAL_EPSILON: f32 = 1.0e-4;
const PLANE_EPSILON: f32 = 1.0e-5;
const CAMERA_PHI_ITERATIONS: i32 = 20;
const MAX_DISK_CROSSINGS: i32 = 12;

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
@group(2) @binding(0) var lut_u: texture_2d<f32>;
@group(2) @binding(1) var lut_phi_max: texture_2d<f32>;

struct TraceResult {
    kind: u32,
    color: vec3<f32>,
    direction: vec3<f32>,
};

fn ray_direction(uv: vec2<f32>) -> vec3<f32> {
    let ndc = uv * 2.0 - 1.0;
    return normalize(u.cam_forward + ndc.x * u.cam_right + ndc.y * u.cam_up);
}

fn lut_index_from_b(b: f32) -> f32 {
    if b <= B_CRIT {
        return (b / B_CRIT) * SPLIT;
    }
    let e = sqrt((b - B_CRIT) / (B_MAX - B_CRIT));
    return SPLIT + e * (N_B - 1.0 - SPLIT);
}

fn fetch_phi_max(fi: f32) -> f32 {
    let i0 = i32(floor(fi));
    let i1 = min(i0 + 1, i32(N_B) - 1);
    let tx = fi - floor(fi);
    let a = textureLoad(lut_phi_max, vec2<i32>(i0, 0), 0).r;
    let b = textureLoad(lut_phi_max, vec2<i32>(i1, 0), 0).r;
    return mix(a, b, tx);
}

fn fetch_u(fi: f32, phi: f32, phi_max: f32) -> f32 {
    let fj = clamp(phi / phi_max, 0.0, 1.0) * (N_PHI - 1.0);
    let i0 = i32(floor(fi));
    let i1 = min(i0 + 1, i32(N_B) - 1);
    let tx = fi - floor(fi);
    let j0 = i32(floor(fj));
    let j1 = min(j0 + 1, i32(N_PHI) - 1);
    let ty = fj - floor(fj);

    let u00 = textureLoad(lut_u, vec2<i32>(i0, j0), 0).r;
    let u10 = textureLoad(lut_u, vec2<i32>(i1, j0), 0).r;
    let u01 = textureLoad(lut_u, vec2<i32>(i0, j1), 0).r;
    let u11 = textureLoad(lut_u, vec2<i32>(i1, j1), 0).r;

    return mix(mix(u00, u10, tx), mix(u01, u11, tx), ty);
}

fn camera_phi(fi: f32, phi_max: f32, u_camera: f32, captured: bool) -> f32 {
    var lo = 0.0;
    var hi = select(phi_max * 0.5, phi_max, captured);
    for (var k = 0; k < CAMERA_PHI_ITERATIONS; k = k + 1) {
        let mid = 0.5 * (lo + hi);
        if fetch_u(fi, mid, phi_max) < u_camera {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    return 0.5 * (lo + hi);
}

fn trace_ray(origin: vec3<f32>, dir: vec3<f32>) -> TraceResult {
    var result: TraceResult;
    result.kind = 0u;
    result.direction = dir;

    let rs = u.schwarzschild_radius;
    let r_camera = length(origin);
    let radial = origin / r_camera;
    let cos_psi = dot(radial, dir);
    let tangent_vec = dir - cos_psi * radial;
    let sin_psi = length(tangent_vec);

    if sin_psi < RADIAL_EPSILON {
        if cos_psi < 0.0 {
            result.kind = 2u;
            result.color = vec3<f32>(0.0);
        }
        return result;
    }

    let tangent = tangent_vec / sin_psi;
    let impact = r_camera * sin_psi;
    let b = impact / rs;

    if b >= B_MAX {
        return result;
    }

    let u_camera = rs / r_camera;
    let captured = b < B_CRIT;
    let inward = cos_psi < 0.0;
    let travel_sign = select(-1.0, 1.0, inward);

    let fi = lut_index_from_b(b);
    let phi_max = fetch_phi_max(fi);
    let phi_camera = camera_phi(fi, phi_max, u_camera, captured);
    let theta_total = select(phi_camera, phi_max - phi_camera, inward);

    let plane_a = radial.y;
    let plane_b = tangent.y;
    if abs(plane_a) > PLANE_EPSILON || abs(plane_b) > PLANE_EPSILON {
        var theta = atan2(-plane_a, plane_b);
        theta = theta - PI * floor(theta / PI);
        if theta <= RADIAL_EPSILON {
            theta = theta + PI;
        }
        for (var k = 0; k < MAX_DISK_CROSSINGS; k = k + 1) {
            if theta > theta_total {
                break;
            }
            let phi = phi_camera + travel_sign * theta;
            if phi >= 0.0 && phi <= phi_max {
                let u_hit = fetch_u(fi, phi, phi_max);
                if u_hit > RADIAL_EPSILON {
                    let r_hit = rs / u_hit;
                    if r_hit >= u.disk_inner && r_hit <= u.disk_outer {
                        result.kind = 1u;
                        result.color = disk_emission(r_hit);
                        return result;
                    }
                }
            }
            theta = theta + PI;
        }
    }

    if captured && inward {
        result.kind = 2u;
        result.color = vec3<f32>(0.0);
        return result;
    }

    result.direction = normalize(cos(theta_total) * radial + sin(theta_total) * tangent);
    return result;
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

fn disk_emission(radius: f32) -> vec3<f32> {
    let t = clamp((radius - u.disk_inner) / (u.disk_outer - u.disk_inner), 0.0, 1.0);
    let inner_color = vec3<f32>(1.0, 0.9, 0.6);
    let outer_color = vec3<f32>(0.75, 0.16, 0.03);
    let brightness = mix(1.6, 0.25, t);
    return mix(inner_color, outer_color, t) * brightness;
}
