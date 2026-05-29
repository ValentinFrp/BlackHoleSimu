use std::f64::consts::PI;

pub const B_CRIT: f64 = 2.598076211353316;

const D_PHI: f64 = 0.005;
const MAX_PHI: f64 = 12.0 * PI;
const HORIZON_U: f64 = 1.0;
const MIN_IMPACT: f64 = 1.0e-3;

pub struct Trajectory {
    pub phi: Vec<f64>,
    pub u: Vec<f64>,
    pub captured: bool,
}

fn binet(u: f64) -> f64 {
    -u + 1.5 * u * u
}

pub fn integrate(impact: f64) -> Trajectory {
    let b = impact.max(MIN_IMPACT);
    let mut phi = 0.0_f64;
    let mut u = 0.0_f64;
    let mut du = 1.0 / b;
    let mut phis = vec![0.0_f64];
    let mut us = vec![0.0_f64];

    loop {
        let k1u = du;
        let k1d = binet(u);
        let k2u = du + 0.5 * D_PHI * k1d;
        let k2d = binet(u + 0.5 * D_PHI * k1u);
        let k3u = du + 0.5 * D_PHI * k2d;
        let k3d = binet(u + 0.5 * D_PHI * k2u);
        let k4u = du + D_PHI * k3d;
        let k4d = binet(u + D_PHI * k3u);

        let u_next = u + (D_PHI / 6.0) * (k1u + 2.0 * k2u + 2.0 * k3u + k4u);
        let du_next = du + (D_PHI / 6.0) * (k1d + 2.0 * k2d + 2.0 * k3d + k4d);
        let phi_next = phi + D_PHI;

        if u_next >= HORIZON_U {
            let t = (HORIZON_U - u) / (u_next - u);
            phis.push(phi + t * D_PHI);
            us.push(HORIZON_U);
            return Trajectory { phi: phis, u: us, captured: true };
        }

        if du_next < 0.0 && u_next <= 0.0 {
            let t = u / (u - u_next);
            phis.push(phi + t * D_PHI);
            us.push(0.0);
            return Trajectory { phi: phis, u: us, captured: false };
        }

        if phi_next > MAX_PHI {
            phis.push(phi_next);
            us.push(u_next);
            return Trajectory { phi: phis, u: us, captured: true };
        }

        u = u_next;
        du = du_next;
        phi = phi_next;
        phis.push(phi);
        us.push(u);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b_crit_matches_photon_sphere() {
        assert!((B_CRIT - 1.5 * 3.0_f64.sqrt()).abs() < 1.0e-9);
    }

    #[test]
    fn below_critical_is_captured() {
        assert!(integrate(2.0).captured);
        assert!(integrate(B_CRIT - 0.05).captured);
    }

    #[test]
    fn above_critical_escapes() {
        assert!(!integrate(B_CRIT + 0.2).captured);
        assert!(!integrate(6.0).captured);
    }

    #[test]
    fn weak_field_deflection_matches_2_over_b() {
        let traj = integrate(20.0);
        assert!(!traj.captured);
        let phi_max = *traj.phi.last().unwrap();
        let deflection = phi_max - PI;
        assert!((deflection - 0.1).abs() < 0.02, "deflection = {deflection}");
    }

    #[test]
    fn radius_stays_outside_horizon_on_escape() {
        let traj = integrate(5.0);
        let u_max = traj.u.iter().cloned().fold(0.0_f64, f64::max);
        assert!(u_max < 2.0 / 3.0 + 1.0e-3, "u_max = {u_max}");
    }
}
