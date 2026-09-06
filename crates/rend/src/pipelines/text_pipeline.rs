use crate::Texture;
use crate::*;
use bary_core::prelude::rand;
use glam::DVec2;
use glm::{Mat4, Vec3, Vec4};
use log::info;
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
        let shader = Shader::from_path("crates/rend/shaders/text_shader.wgsl").unwrap();

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

    pub fn assign_buffer_data(
        &self,
        queue: &Queue,
        commands: &[CharCommand],
        font: &FontInfo,
        screen: DVec2,
    ) {
        let color_data: Vec<u8> = commands
            .iter()
            .map(|cmd| any_as_u8_slice(&cmd.color.to_vec()).to_vec())
            .collect::<Vec<Vec<u8>>>()
            .concat();

        let transform_data = commands
            .iter()
            .map(|cmd| {
                let transform = TextTransform {
                    x: cmd.pos.x as f32,
                    y: cmd.pos.y as f32,
                    width: cmd.dims.x as f32,
                    height: cmd.dims.y as f32,
                    sx: screen.x as f32,
                    sy: screen.y as f32,
                    angle: cmd.angle as f32,
                };

                any_as_u8_slice(&transform.to_gpu()).to_vec()
            })
            .collect::<Vec<Vec<u8>>>()
            .concat();

        let range_data = commands
            .iter()
            .map(|cmd| {
                let range = font.get_sample_range(cmd.c).unwrap();
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
                any_as_u8_slice(&gpu).to_vec()
            })
            .collect::<Vec<Vec<u8>>>()
            .concat();

        self.colors.write(queue, &color_data);
        self.transforms.write(queue, &transform_data);
        self.range_info.write(queue, &range_data);
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
