use glm::*;

pub fn translation_matrix(p: Vec3) -> Mat4 {
    Matrix4::new(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(p.x, p.y, p.z, 1.0),
    )
}

pub fn mat4_identity() -> glm::Mat4 {
    mat4_diagonal(1.0, 1.0, 1.0, 1.0)
}

pub fn mat4_z_rotation(alpha: f64) -> glm::Mat4 {
    let c = alpha.cos() as f32;
    let s = alpha.sin() as f32;

    let c0 = glm::Vec4::new(c, s, 0.0, 0.0);
    let c1 = glm::Vec4::new(-s, c, 0.0, 0.0);
    let c2 = glm::Vec4::new(0.0, 0.0, 1.0, 0.0);
    let c3 = glm::Vec4::new(0.0, 0.0, 0.0, 1.0);
    glm::Matrix4::new(c0, c1, c2, c3)
}

pub fn mat4_diagonal(a: f32, b: f32, c: f32, d: f32) -> glm::Mat4 {
    let c0 = glm::Vec4::new(a, 0.0, 0.0, 0.0);
    let c1 = glm::Vec4::new(0.0, b, 0.0, 0.0);
    let c2 = glm::Vec4::new(0.0, 0.0, c, 0.0);
    let c3 = glm::Vec4::new(0.0, 0.0, 0.0, d);
    glm::Matrix4::new(c0, c1, c2, c3)
}

pub fn mat4_lerp(a: &Mat4, b: &Mat4, t: f32) -> Mat4 {
    *a + (*b - *a) * t
}
