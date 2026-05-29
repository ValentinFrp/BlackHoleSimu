#[derive(Clone, Copy)]
pub struct BlackHole {
    pub schwarzschild_radius: f32,
}

#[derive(Clone, Copy)]
pub struct AccretionDisk {
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub peak_temperature: f32,
}

#[derive(Clone, Copy)]
pub struct Scene {
    pub black_hole: BlackHole,
    pub disk: AccretionDisk,
}

impl Default for Scene {
    fn default() -> Self {
        let r_s = 1.0;
        Self {
            black_hole: BlackHole {
                schwarzschild_radius: r_s,
            },
            disk: AccretionDisk {
                inner_radius: 3.0 * r_s,
                outer_radius: 11.0 * r_s,
                peak_temperature: 6500.0,
            },
        }
    }
}
