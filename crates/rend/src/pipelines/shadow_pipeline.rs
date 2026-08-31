use crate::Texture;
use crate::*;
use wgpu::*;

pub struct ShadowPipeline {
    pipeline: RenderPipeline,
    mesh: Mesh,
    shader_params: BufferResource,
}

impl ShadowPipeline {
    pub fn new(rd: &Renderer) -> Self {
        let mesh = make_quad(&rd.device);

        let bgl = Texture::make_bind_group_layout(&rd.device, "ShadowPipeline Bind Group Layout");
        let shader = Shader::from_path("crates/rend/shaders/shadow_map.wgsl");

        let shader_params = BufferResource::new_array(
            &rd.device,
            1,
            ShaderParams::SIZE_IN_BYTES,
            "Shadow pipeline shader params",
        );

        let layout = BufferResource::make_layout(&rd.device);

        let mut builder = PipelineBuilder::new(&rd.device);
        builder.add_bind_group_layout(&bgl);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Shadow Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        Self {
            pipeline,
            mesh,
            shader_params,
        }
    }

    pub fn shadow_pass(
        &self,
        rp: &mut RenderPass,
        queue: &Queue,
        params: &ShaderParams,
        material: &BindGroup,
    ) {
        rp.set_pipeline(&self.pipeline);
        self.shader_params.write(queue, &params.to_bytes());
        rp.set_bind_group(0, material, &[]);
        rp.set_bind_group(1, self.shader_params.bind_group(), &[]);
        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..1);
    }
}
