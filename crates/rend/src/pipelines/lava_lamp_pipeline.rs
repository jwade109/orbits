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
        let layout = BufferResource::make_layout(&rd.device);

        let mesh = make_quad(&rd.device);
        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/cells.wgsl");
        builder.add_bind_group_layout(&layout);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Lava Lamp Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        let camera_data = BufferResource::new_array(&rd.device, 1, 64, "Lava lamp camera");
        let shader_params = BufferResource::new_array(
            &rd.device,
            1,
            ShaderParams::SIZE_IN_BYTES,
            "Lava lamp shader params",
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
        self.camera_data.write(queue, any_as_u8_slice(transform));
        self.shader_params.write(queue, &shader_params.to_bytes());
        rp.set_bind_group(0, self.shader_params.bind_group(), &[]);
        rp.set_bind_group(1, self.camera_data.bind_group(), &[]);
        draw_mesh(rp, &self.mesh);
    }
}
