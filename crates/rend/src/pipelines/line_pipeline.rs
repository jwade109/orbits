use crate::*;
use glam::DVec2;
use glm::Vec4;
use wgpu::*;

pub struct LinePipeline {
    pipeline: RenderPipeline,
    data: BufferResource,
    mesh: Mesh,
}

impl LinePipeline {
    pub const MAX_LINES_PER_PASS: usize = 1200;

    pub fn new(rd: &Renderer) -> Self {
        // this buffer holds 2D start and end pos, as well as color and thickness
        // so 9 f32s -> 9 * 4 = 36, plus 12 padding bytes -> 48
        let data = BufferResource::new_array(&rd.device, Self::MAX_LINES_PER_PASS, 48, "Line data");

        let layout = BufferResource::make_layout(&rd.device);

        let mesh = make_quad(&rd.device);

        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/line.wgsl");

        builder.add_bind_group_layout(&layout);
        // builder.add_bind_group_layout(&transforms.layout);
        // builder.add_bind_group_layout(&radius.layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Line Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        Self {
            pipeline,
            data,
            mesh,
        }
    }

    pub fn assign_buffer_data(&self, queue: &Queue, commands: &[LineCommand], sx: f64, sy: f64) {
        let data: Vec<u8> = commands
            .iter()
            .flat_map(|l| {
                [
                    l.start.x as f32,
                    l.start.y as f32,
                    l.end.x as f32,
                    l.end.y as f32,
                    l.color.r as f32,
                    l.color.g as f32,
                    l.color.b as f32,
                    l.color.a as f32,
                    l.thickness as f32,
                    sx as f32,
                    sy as f32,
                    0.0,
                ]
                .to_vec()
            })
            .flat_map(|f| f.to_le_bytes().to_vec())
            .collect();

        self.data.write(queue, &data);
    }

    pub fn draw_lines(&self, rp: &mut RenderPass, n: usize) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, self.data.bind_group(), &[]);
        let n = n.min(Self::MAX_LINES_PER_PASS);
        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}
