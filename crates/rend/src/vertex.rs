use glm::*;

pub trait Vertex {
    fn get_layout() -> wgpu::VertexBufferLayout<'static>;

    fn to_bytes(&self) -> Vec<u8>;
}

#[repr(C)]
pub struct TexQuadVertex {
    position: Vec3,
    uv: Vec2,
}

impl TexQuadVertex {
    pub fn new(position: Vec3, uv: Vec2) -> Self {
        Self { position, uv }
    }
}

impl Vertex for TexQuadVertex {
    fn get_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FullVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let arr = [
            self.position.x,
            self.position.y,
            self.position.z,
            self.uv.x,
            self.uv.y,
        ];

        bytemuck::bytes_of(&arr).to_vec()
    }
}

#[repr(C)]
pub struct FullVertex {
    position: Vec3,
    color: Vec4,
    tex_coord: Vec2,
}

impl FullVertex {
    pub fn new(position: Vec3, color: Vec4, tex_coord: Vec2) -> Self {
        Self {
            position,
            color,
            tex_coord,
        }
    }
}

impl Vertex for FullVertex {
    fn get_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FullVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let arr = [
            self.position.x,
            self.position.y,
            self.position.z,
            self.color.x,
            self.color.y,
            self.color.z,
            self.color.w,
            self.tex_coord.x,
            self.tex_coord.y,
        ];

        bytemuck::bytes_of(&arr).to_vec()
    }
}
