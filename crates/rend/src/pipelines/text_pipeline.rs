use crate::Texture;
use crate::*;
use glam::DVec2;
use glm::{Mat4, Vec3, Vec4};
use wgpu::*;

pub struct TextPipeline {
    pipeline: RenderPipeline,
    colors: BufferResource,
    range_info: BufferResource,
    transforms: BufferResource,
    mesh: Mesh,
}

struct TextTransform {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    angle: f32,
    sx: f32,
    sy: f32,
}

const NUM_F32_IN_TEXT_TRANSFORM: usize = 8;

impl TextTransform {
    fn to_gpu(&self) -> [f32; NUM_F32_IN_TEXT_TRANSFORM] {
        [
            self.x,
            self.y,
            self.width,
            self.height,
            self.angle,
            self.sx,
            self.sy,
            0.0,
        ]
    }
}

pub struct GpuSampleInfo {
    pub origin_x: u32,
    pub origin_y: u32,
    pub sample_width: u32,
    pub sample_height: u32,
    pub image_width: u32,
    pub image_height: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl TextPipeline {
    pub const MAX_CHARS_PER_PASS: usize = 1400;

    pub fn new(rd: &Renderer) -> Self {
        let mesh = make_quad(&rd.device);
        let size = std::mem::size_of::<GpuSampleInfo>();
        assert!(size == 4 * 8);
        let range_info = BufferResource::new_array(
            &rd.device,
            Self::MAX_CHARS_PER_PASS,
            size,
            "Text range info",
        );

        let colors =
            BufferResource::new_array(&rd.device, Self::MAX_CHARS_PER_PASS, 16, "Text colors");

        let transforms = BufferResource::new_array(
            &rd.device,
            Self::MAX_CHARS_PER_PASS,
            NUM_F32_IN_TEXT_TRANSFORM * 4,
            "Text transforms",
        );

        let bgl = Texture::make_bind_group_layout(&rd.device, "Texture Bind Group Layout");

        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/text_shader.wgsl");

        let layout = BufferResource::make_layout(&rd.device);

        builder.add_bind_group_layout(&bgl);
        builder.add_bind_group_layout(&layout);
        builder.add_bind_group_layout(&layout);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Single Texture Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        for i in 0..Self::MAX_CHARS_PER_PASS {
            let color = [1.0f32, 1.0, 1.0, 1.0];

            colors.upload(&rd.queue, 16 * i as u64, any_as_u8_slice(&color));
        }

        Self {
            pipeline,
            colors,
            range_info,
            transforms,
            mesh,
        }
    }

    fn set_color(&self, queue: &Queue, i: usize, color: Vec4) {
        self.colors
            .upload(queue, 16 * i as u64, any_as_u8_slice(&color));
    }

    fn set_transform(&self, queue: &Queue, i: usize, transform: TextTransform) {
        self.transforms
            .upload(queue, 32 * i as u64, any_as_u8_slice(&transform.to_gpu()));
    }

    pub fn set_range(&self, queue: &Queue, i: usize, range: &TextureSampleRange) {
        let gpu = GpuSampleInfo {
            origin_x: range.origin_x,
            origin_y: range.origin_y,
            sample_width: range.sample_width,
            sample_height: range.sample_height,
            image_width: range.image_width,
            image_height: range.image_height,
            _pad1: 0,
            _pad2: 0,
        };

        self.range_info
            .upload(queue, 32 * i as u64, any_as_u8_slice(&gpu));
    }

    pub fn assign_buffer_data(
        &self,
        queue: &Queue,
        commands: &[CharCommand],
        font: &FontInfo,
        screen: DVec2,
    ) {
        for (i, text) in commands.iter().enumerate() {
            let range = font.get_sample_range(text.c).unwrap();

            let transform = TextTransform {
                x: text.pos.x as f32,
                y: text.pos.y as f32,
                width: text.dims.x as f32,
                height: text.dims.y as f32,
                sx: screen.x as f32,
                sy: screen.y as f32,
                angle: text.angle as f32,
            };

            self.set_range(queue, i, &range);
            self.set_transform(queue, i, transform);
            self.set_color(queue, i, text.color.to_vec())
        }
    }

    pub fn draw_text(&self, rp: &mut RenderPass, material: &Texture, n: usize) {
        rp.set_pipeline(&self.pipeline);

        rp.set_bind_group(0, &material.bind_group, &[]);
        rp.set_bind_group(1, self.colors.bind_group(), &[]);
        rp.set_bind_group(2, self.range_info.bind_group(), &[]);
        rp.set_bind_group(3, self.transforms.bind_group(), &[]);

        let n = n.min(Self::MAX_CHARS_PER_PASS);

        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}

pub fn screen_space_transform(pos: DVec2, dims: DVec2, screen: DVec2, _angle: f64) -> Mat4 {
    // let aspect_ratio = (sx / sy) as f32;

    let width_scale = dims.x / screen.x;
    let height_scale = dims.y / screen.y;

    let xoff = 2.0 * (pos.x + dims.x / 2.0) / screen.x - 1.0;
    let yoff = -(2.0 * (pos.y + dims.y / 2.0) / screen.y - 1.0);

    translation_matrix(Vec3::new(xoff as f32, yoff as f32, 0.0))
        * mat4_diagonal(width_scale as f32, height_scale as f32, 1.0, 1.0)
}
