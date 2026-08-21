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
    pub const MAX_LINES_PER_PASS: usize = 600;

    pub fn new(rd: &Renderer) -> Self {
        // this buffer holds 2D start and end pos, as well as color and thickness
        // so 9 f32s -> 9 * 4 = 36, plus 12 padding bytes -> 48
        let data = make_array_resource(&rd.device, Self::MAX_LINES_PER_PASS, 48, "Line data");

        let mesh = make_quad(&rd.device);

        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/line.wgsl");

        builder.add_bind_group_layout(&data.layout);
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

    pub fn set_data(
        &self,
        queue: &Queue,
        i: usize,
        start: DVec2,
        end: DVec2,
        color: Vec4,
        t: f64,
        sx: f64,
        sy: f64,
    ) {
        let as_floats: [f32; 12] = [
            start.x as f32,
            start.y as f32,
            end.x as f32,
            end.y as f32,
            color.x,
            color.y,
            color.z,
            color.w,
            t as f32,
            sx as f32,
            sy as f32,
            0.0,
        ];

        queue.write_buffer(
            &self.data.buffer,
            48 * i as u64,
            any_as_u8_slice(&as_floats),
        );
    }

    pub fn assign_buffer_data(&self, queue: &Queue, commands: &[LineCommand], sx: f64, sy: f64) {
        for (i, cmd) in commands.iter().enumerate() {
            self.set_data(
                queue,
                i,
                cmd.start,
                cmd.end,
                cmd.color.to_vec(),
                cmd.thickness,
                sx,
                sy,
            );
        }
    }

    pub fn draw_lines(&self, rp: &mut RenderPass, n: usize) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, &self.data.bind_group, &[]);
        let n = n.min(Self::MAX_LINES_PER_PASS);
        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}
