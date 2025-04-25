#[derive(Debug, Clone, Copy)]
pub struct TelescopeContext {
    pub azimuth: f32,
    pub elevation: f32,
    pub angular_radius: f32,
}

impl TelescopeContext {
    pub fn new() -> Self {
        TelescopeContext {
            azimuth: 0.0,
            elevation: 0.0,
            angular_radius: 1.0,
        }
    }
}
