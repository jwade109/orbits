// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone)]
pub struct ShaderParams {
    pub mouse: (f32, f32),
    pub resolution: (f32, f32),
    pub time: f32,
}

impl ShaderParams {
    pub const SIZE_IN_BYTES: usize = 40;

    pub fn to_bytes(&self) -> Vec<u8> {
        [
            self.mouse.0.to_le_bytes(),
            self.mouse.1.to_le_bytes(),
            self.resolution.0.to_le_bytes(),
            self.resolution.1.to_le_bytes(),
            self.time.to_le_bytes(),
            [0; 4],
            [0; 4],
            [0; 4],
            [0; 4],
            [0; 4],
        ]
        .concat()
    }
}
