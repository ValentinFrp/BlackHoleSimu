use half::f16;

pub const BB_N: usize = 1024;
pub const BB_T_MIN: f64 = 1000.0;
pub const BB_T_MAX: f64 = 40000.0;

const LAMBDA_MIN_NM: f64 = 380.0;
const LAMBDA_MAX_NM: f64 = 780.0;
const LAMBDA_STEP_NM: f64 = 1.0;
const RADIATION_C2: f64 = 1.438776877e-2;

pub struct BlackbodyLut {
    pub rgba_f16: Vec<u16>,
}

impl BlackbodyLut {
    pub fn build() -> Self {
        let mut rgba_f16 = Vec::with_capacity(BB_N * 4);
        for i in 0..BB_N {
            let t = BB_T_MIN + (BB_T_MAX - BB_T_MIN) * (i as f64 / (BB_N - 1) as f64);
            let [r, g, b] = color_for_temperature(t);
            rgba_f16.push(f16::from_f64(r).to_bits());
            rgba_f16.push(f16::from_f64(g).to_bits());
            rgba_f16.push(f16::from_f64(b).to_bits());
            rgba_f16.push(f16::from_f64(1.0).to_bits());
        }
        Self { rgba_f16 }
    }
}

fn color_for_temperature(temperature: f64) -> [f64; 3] {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    let mut lambda = LAMBDA_MIN_NM;
    while lambda <= LAMBDA_MAX_NM {
        let radiance = planck(lambda, temperature);
        x += radiance * cie_x(lambda);
        y += radiance * cie_y(lambda);
        z += radiance * cie_z(lambda);
        lambda += LAMBDA_STEP_NM;
    }

    if y <= 0.0 {
        return [0.0, 0.0, 0.0];
    }

    x /= y;
    z /= y;
    y = 1.0;

    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;

    [r.max(0.0), g.max(0.0), b.max(0.0)]
}

fn planck(lambda_nm: f64, temperature: f64) -> f64 {
    let lambda = lambda_nm * 1.0e-9;
    let exponent = RADIATION_C2 / (lambda * temperature);
    1.0 / (lambda.powi(5) * (exponent.exp() - 1.0))
}

fn gaussian(lambda: f64, mu: f64, sigma_low: f64, sigma_high: f64) -> f64 {
    let sigma = if lambda < mu { sigma_low } else { sigma_high };
    let t = (lambda - mu) / sigma;
    (-0.5 * t * t).exp()
}

fn cie_x(lambda: f64) -> f64 {
    1.056 * gaussian(lambda, 599.8, 37.9, 31.0)
        + 0.362 * gaussian(lambda, 442.0, 16.0, 26.7)
        - 0.065 * gaussian(lambda, 501.1, 20.4, 26.2)
}

fn cie_y(lambda: f64) -> f64 {
    0.821 * gaussian(lambda, 568.8, 46.9, 40.5) + 0.286 * gaussian(lambda, 530.9, 16.3, 31.1)
}

fn cie_z(lambda: f64) -> f64 {
    1.217 * gaussian(lambda, 437.0, 11.8, 36.0) + 0.681 * gaussian(lambda, 459.0, 26.0, 13.8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cool_is_reddish() {
        let [r, _, b] = color_for_temperature(2000.0);
        assert!(r > b, "r = {r}, b = {b}");
    }

    #[test]
    fn hot_is_bluish() {
        let [r, _, b] = color_for_temperature(20000.0);
        assert!(b > r, "r = {r}, b = {b}");
    }

    #[test]
    fn daylight_is_roughly_neutral() {
        let [r, g, b] = color_for_temperature(6500.0);
        assert!(r > 0.0 && g > 0.0 && b > 0.0);
        assert!((r - b).abs() < 0.5, "r = {r}, b = {b}");
    }

    #[test]
    fn lut_has_expected_length() {
        let lut = BlackbodyLut::build();
        assert_eq!(lut.rgba_f16.len(), BB_N * 4);
    }
}
