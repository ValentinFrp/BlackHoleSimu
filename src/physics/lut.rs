use super::geodesic::{integrate, Trajectory, B_CRIT};

pub const N_B: usize = 1024;
pub const N_PHI: usize = 512;
pub const SPLIT: usize = 256;
pub const B_MAX: f64 = 64.0;

pub struct DeflectionLut {
    pub u: Vec<f32>,
    pub phi_max: Vec<f32>,
}

pub fn b_from_index(i: usize) -> f64 {
    if i <= SPLIT {
        B_CRIT * (i as f64 / SPLIT as f64)
    } else {
        let e = (i - SPLIT) as f64 / ((N_B - 1 - SPLIT) as f64);
        B_CRIT + (B_MAX - B_CRIT) * e * e
    }
}

impl DeflectionLut {
    pub fn build() -> Self {
        let mut u = vec![0.0_f32; N_B * N_PHI];
        let mut phi_max = vec![0.0_f32; N_B];

        for i in 0..N_B {
            let traj = integrate(b_from_index(i));
            let pmax = *traj.phi.last().unwrap();
            phi_max[i] = pmax as f32;

            for j in 0..N_PHI {
                let target = pmax * (j as f64 / (N_PHI - 1) as f64);
                u[j * N_B + i] = sample_u(&traj, target) as f32;
            }
        }

        Self { u, phi_max }
    }
}

fn sample_u(traj: &Trajectory, target: f64) -> f64 {
    let phi = &traj.phi;
    let u = &traj.u;
    let last = phi.len() - 1;

    if target <= phi[0] {
        return u[0];
    }
    if target >= phi[last] {
        return u[last];
    }

    let mut lo = 0_usize;
    let mut hi = last;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if phi[mid] <= target {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let t = (target - phi[lo]) / (phi[hi] - phi[lo]);
    u[lo] + t * (u[hi] - u[lo])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_map_is_continuous_at_split() {
        assert!((b_from_index(SPLIT) - B_CRIT).abs() < 1.0e-9);
    }

    #[test]
    fn index_map_spans_full_range() {
        assert!(b_from_index(0) < 1.0e-6);
        assert!((b_from_index(N_B - 1) - B_MAX).abs() < 1.0e-6);
    }

    #[test]
    fn all_u_within_unit_range() {
        let lut = DeflectionLut::build();
        for value in &lut.u {
            assert!(*value >= 0.0 && *value <= 1.0001, "u = {value}");
        }
    }

    #[test]
    fn phi_max_decreases_with_impact_in_escape_region() {
        let lut = DeflectionLut::build();
        let near = lut.phi_max[SPLIT + 16];
        let far = lut.phi_max[SPLIT + 400];
        assert!(near > far, "near = {near}, far = {far}");
    }
}
