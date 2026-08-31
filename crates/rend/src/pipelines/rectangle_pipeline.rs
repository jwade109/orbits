use std::collections::BTreeMap;

use crate::*;
use glam::DVec2;
use wgpu::*;

pub struct RectanglePipeline {
    pipeline: RenderPipeline,
    rect_data: BufferResource,
    resources: BTreeMap<String, BufferResource>,
    mesh: Mesh,
}

const RECT_DATA_F32_COUNT: usize = 12;

fn to_packed_array(cmd: &RectCommand, screen_size: DVec2) -> [f32; RECT_DATA_F32_COUNT] {
    [
        cmd.pos.x as f32,
        cmd.pos.y as f32,
        cmd.dims.x as f32,
        cmd.dims.y as f32,
        cmd.color.r as f32,
        cmd.color.g as f32,
        cmd.color.b as f32,
        cmd.color.a as f32,
        cmd.angle as f32,
        screen_size.x as f32,
        screen_size.y as f32,
        0.0, // padding
    ]
}

impl RectanglePipeline {
    pub const RECTS_PER_PASS: usize = 400;

    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        shader_path: &str,
        named_resources: &[(String, usize)],
    ) -> Self {
        let rect_data = make_array_resource(
            device,
            Self::RECTS_PER_PASS,
            RECT_DATA_F32_COUNT * 4,
            "Rectangle data",
        );

        let mesh = make_quad(device);

        let mut builder = PipelineBuilder::new(&device);
        let shader = Shader::from_path(shader_path);
        builder.add_bind_group_layout(&rect_data.layout);

        let mut resources = BTreeMap::new();

        for (name, size) in named_resources {
            let arr = make_resource(device, *size, name);
            resources.insert(name.clone(), arr);
        }

        for (_name, buffer) in &resources {
            builder.add_bind_group_layout(&buffer.layout);
        }

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Lava Lamp Pipeline",
            &shader,
            config.format,
            true,
            true,
        );

        Self {
            pipeline,
            rect_data,
            resources,
            mesh,
        }
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn upload(&self, queue: &Queue, buffer_name: &str, data: &[u8]) {
        if let Some(buffer) = self.resources.get(buffer_name) {
            queue.write_buffer(&buffer.buffer, 0, data);
        } else {
            println!("Bad resource name {buffer_name}");
        }
    }

    fn set_data(&self, queue: &Queue, index: usize, data: [f32; RECT_DATA_F32_COUNT]) {
        let stride = RECT_DATA_F32_COUNT * 4;
        queue.write_buffer(
            &self.rect_data.buffer,
            (stride * index) as u64,
            any_as_u8_slice(&data),
        )
    }

    pub fn assign_buffer_data(&self, queue: &Queue, commands: &[RectCommand], screen: DVec2) {
        for (i, cmd) in commands.iter().enumerate() {
            let data = to_packed_array(cmd, screen);
            self.set_data(queue, i, data);
        }
    }

    pub fn draw(&self, rp: &mut RenderPass, n: usize) {
        rp.set_pipeline(self.pipeline());
        rp.set_bind_group(0, &self.rect_data.bind_group, &[]);

        for (_name, buffer) in &self.resources {
            rp.set_bind_group(1, &buffer.bind_group, &[]);
        }

        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}
