use crate::*;
use wgpu::*;

pub struct LavaLampPipeline {
    pipeline: RenderPipeline,
    camera_data: BufferResource,
    shader_params: BufferResource,
    mesh: Mesh,
}

impl LavaLampPipeline {
    pub fn new(rd: &Renderer) -> Self {
        let camera_data = make_array_resource(&rd.device, 1, 64, "Lava lamp camera");
        let shader_params = make_array_resource(
            &rd.device,
            1,
            ShaderParams::SIZE_IN_BYTES,
            "Lava lamp shader params",
        );

        let mesh = make_quad(&rd.device);
        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/cells.wgsl");
        builder.add_bind_group_layout(&shader_params.layout);
        builder.add_bind_group_layout(&camera_data.layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Lava Lamp Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        Self {
            pipeline,
            camera_data,
            shader_params,
            mesh,
        }
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn draw(
        &self,
        rp: &mut RenderPass,
        transform: &glm::Mat4,
        shader_params: &ShaderParams,
        queue: &Queue,
    ) {
        rp.set_pipeline(self.pipeline());
        queue.write_buffer(&self.camera_data.buffer, 0, any_as_u8_slice(transform));
        queue.write_buffer(&self.shader_params.buffer, 0, &shader_params.to_bytes());
        rp.set_bind_group(0, &self.shader_params.bind_group, &[]);
        rp.set_bind_group(1, &self.camera_data.bind_group, &[]);
        draw_mesh(rp, &self.mesh);
    }
}
