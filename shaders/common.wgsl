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

const BB_N: f32 = 1024.0;
const BB_T_MIN: f32 = 1000.0;
const BB_T_MAX: f32 = 40000.0;

const NT_PEAK: f32 = 0.4877986;
const TURB_FREQUENCY: f32 = 0.7;
const TURB_WARP: f32 = 0.6;

const DISK_MARCH_STEPS: i32 = 10;
const DISK_ABSORPTION: f32 = 4.0;
const FLARE_STRENGTH: f32 = 0.6;
const VERT_SIGMAS: f32 = 3.0;
const MAX_VERT_WINDOW: f32 = 0.45;

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
    disk_temperature: f32,
    disk_intensity: f32,
    disk_spin: f32,
    disk_rotation_speed: f32,
    disk_turbulence: f32,
    disk_thickness: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var sky_texture: texture_2d<f32>;
@group(1) @binding(1) var sky_sampler: sampler;
@group(2) @binding(0) var lut_u: texture_2d<f32>;
@group(2) @binding(1) var lut_phi_max: texture_2d<f32>;
@group(2) @binding(2) var blackbody_lut: texture_2d<f32>;

struct DiskSample {
    emission: vec3<f32>,
    alpha: f32,
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

fn trace_ray(origin: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    let rs = u.schwarzschild_radius;
    let r_camera = length(origin);
    let radial = origin / r_camera;
    let cos_psi = dot(radial, dir);
    let tangent_vec = dir - cos_psi * radial;
    let sin_psi = length(tangent_vec);

    if sin_psi < RADIAL_EPSILON {
        if cos_psi < 0.0 {
            return vec3<f32>(0.0);
        }
        return sample_sky(dir);
    }

    let tangent = tangent_vec / sin_psi;
    let impact = r_camera * sin_psi;
    let b = impact / rs;

    if b >= B_MAX {
        return sample_sky(dir);
    }

    let u_camera = rs / r_camera;
    let captured = b < B_CRIT;
    let inward = cos_psi < 0.0;
    let travel_sign = select(-1.0, 1.0, inward);

    let fi = lut_index_from_b(b);
    let phi_max = fetch_phi_max(fi);
    let phi_camera = camera_phi(fi, phi_max, u_camera, captured);
    let theta_total = select(phi_camera, phi_max - phi_camera, inward);

    var background = vec3<f32>(0.0);
    if !(captured && inward) {
        background = sample_sky(normalize(cos(theta_total) * radial + sin(theta_total) * tangent));
    }

    var accum = vec3<f32>(0.0);
    var transmittance = 1.0;

    let plane_a = radial.y;
    let plane_b = tangent.y;
    if abs(plane_a) > PLANE_EPSILON || abs(plane_b) > PLANE_EPSILON {
        var theta = atan2(-plane_a, plane_b);
        theta = theta - PI * floor(theta / PI);
        if theta <= RADIAL_EPSILON {
            theta = theta + PI;
        }
        for (var k = 0; k < MAX_DISK_CROSSINGS; k = k + 1) {
            if theta > theta_total || transmittance < 0.01 {
                break;
            }
            let sample = march_crossing(
                theta,
                phi_camera,
                phi_max,
                fi,
                b,
                captured,
                travel_sign,
                radial,
                tangent,
            );
            accum += transmittance * sample.emission;
            transmittance = transmittance * (1.0 - sample.alpha);
            theta = theta + PI;
        }
    }

    return accum + transmittance * background;
}

fn sample_sky(dir: vec3<f32>) -> vec3<f32> {
    let longitude = atan2(dir.z, dir.x) / TAU + 0.5;
    let latitude = acos(clamp(dir.y, -1.0, 1.0)) / PI;
    return textureSampleLevel(sky_texture, sky_sampler, vec2<f32>(longitude, latitude), 0.0).rgb;
}

fn blackbody_color(temperature: f32) -> vec3<f32> {
    let t = clamp((temperature - BB_T_MIN) / (BB_T_MAX - BB_T_MIN), 0.0, 1.0) * (BB_N - 1.0);
    let i0 = i32(floor(t));
    let i1 = min(i0 + 1, i32(BB_N) - 1);
    let f = t - floor(t);
    let c0 = textureLoad(blackbody_lut, vec2<i32>(i0, 0), 0).rgb;
    let c1 = textureLoad(blackbody_lut, vec2<i32>(i1, 0), 0).rgb;
    return mix(c0, c1, f);
}

fn disk_temperature(radius: f32) -> f32 {
    let x = u.disk_inner / radius;
    let shape = pow(max(0.0, x * x * x * (1.0 - sqrt(x))), 0.25);
    return u.disk_temperature * shape / NT_PEAK;
}

fn doppler_g(
    radius: f32,
    theta: f32,
    u_hit: f32,
    b: f32,
    branch_sign: f32,
    travel_sign: f32,
    radial: vec3<f32>,
    tangent: vec3<f32>,
) -> f32 {
    let rs = u.schwarzschild_radius;
    let lapse = 1.0 - rs / radius;

    let du_dphi = branch_sign * sqrt(max(0.0, 1.0 / (b * b) - u_hit * u_hit + u_hit * u_hit * u_hit));
    let dr_dtheta = -rs / (u_hit * u_hit) * du_dphi * travel_sign;

    let ct = cos(theta);
    let st = sin(theta);
    let radius_dir = ct * radial + st * tangent;
    let transverse_dir = -st * radial + ct * tangent;

    let photon_proper = (dr_dtheta / sqrt(lapse)) * radius_dir + radius * transverse_dir;
    let emit_dir = normalize(-photon_proper);

    let azimuth = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), radius_dir)) * u.disk_spin;
    let beta = sqrt((rs * 0.5) / (radius - rs));
    let gamma = 1.0 / sqrt(1.0 - beta * beta);
    let g_doppler = 1.0 / (gamma * (1.0 - beta * dot(azimuth, emit_dir)));

    let r_camera = length(u.cam_position);
    let g_gravity = sqrt(lapse / (1.0 - rs / r_camera));

    return g_gravity * g_doppler;
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let weight = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, weight.x), mix(c, d, weight.x), weight.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0; i < 5; i = i + 1) {
        value = value + amplitude * value_noise(p * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }
    return value;
}

fn fbm_warped(p: vec2<f32>) -> f32 {
    let warp = vec2<f32>(fbm(p), fbm(p + vec2<f32>(5.2, 1.3)));
    return fbm(p + TURB_WARP * warp);
}

fn disk_density(radius: f32, position: vec3<f32>) -> f32 {
    let omega = sqrt(u.schwarzschild_radius / (2.0 * radius * radius * radius));
    let delta = u.disk_spin * omega * u.time * u.disk_rotation_speed;
    let plane = vec2<f32>(position.x, position.z);
    let c = cos(delta);
    let s = sin(delta);
    let rotated = vec2<f32>(plane.x * c - plane.y * s, plane.x * s + plane.y * c);
    let n = fbm_warped(rotated * TURB_FREQUENCY);
    return mix(1.0 - u.disk_turbulence, 1.0, n);
}

fn march_crossing(
    theta_center: f32,
    phi_camera: f32,
    phi_max: f32,
    fi: f32,
    b: f32,
    captured: bool,
    travel_sign: f32,
    radial: vec3<f32>,
    tangent: vec3<f32>,
) -> DiskSample {
    var sample: DiskSample;
    sample.emission = vec3<f32>(0.0);
    sample.alpha = 0.0;

    let rs = u.schwarzschild_radius;
    let phi_center = phi_camera + travel_sign * theta_center;
    if phi_center < 0.0 || phi_center > phi_max {
        return sample;
    }
    let u_center = fetch_u(fi, phi_center, phi_max);
    if u_center <= RADIAL_EPSILON {
        return sample;
    }
    let radius = rs / u_center;
    if radius < u.disk_inner || radius > u.disk_outer {
        return sample;
    }

    let branch_sign = select(-1.0, 1.0, captured || phi_center < phi_max * 0.5);
    let g = doppler_g(radius, theta_center, u_center, b, branch_sign, travel_sign, radial, tangent);
    let base_temperature = disk_temperature(radius);
    let height = max(u.disk_thickness * radius, 1.0e-3);

    let ct = cos(theta_center);
    let st = sin(theta_center);
    let vertical_rate = max(abs(-st * radial.y + ct * tangent.y), 0.08);
    let half_window = min(VERT_SIGMAS * height / (radius * vertical_rate), MAX_VERT_WINDOW);
    let segment = (2.0 * half_window / f32(DISK_MARCH_STEPS)) * radius;
    let intensity_base = pow(g, 4.0) * u.disk_intensity;

    var local_transmittance = 1.0;
    for (var m = 0; m < DISK_MARCH_STEPS; m = m + 1) {
        let frac = ((f32(m) + 0.5) / f32(DISK_MARCH_STEPS)) * 2.0 - 1.0;
        let theta_s = theta_center + frac * half_window;
        let position = radius * (cos(theta_s) * radial + sin(theta_s) * tangent);
        let vertical = position.y / height;
        let falloff = exp(-0.5 * vertical * vertical);
        let density = falloff * disk_density(radius, position);
        if density > 1.0e-4 {
            let temperature = base_temperature * (1.0 + FLARE_STRENGTH * (density - 0.5) * 2.0);
            let ratio = temperature / u.disk_temperature;
            let glow = blackbody_color(g * temperature)
                * intensity_base
                * ratio * ratio * ratio * ratio;
            let optical_depth = density * DISK_ABSORPTION * segment;
            let coverage = 1.0 - exp(-optical_depth);
            sample.emission += local_transmittance * glow * coverage;
            local_transmittance = local_transmittance * (1.0 - coverage);
        }
    }

    sample.alpha = 1.0 - local_transmittance;
    return sample;
}
