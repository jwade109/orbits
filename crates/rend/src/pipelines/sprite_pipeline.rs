use crate::Texture;
use crate::*;
use wgpu::*;

pub struct SpritePipeline {
    pipeline: RenderPipeline,
    mesh: Mesh,
}

impl SpritePipeline {
    pub fn new(rd: &Renderer) -> Self {
        let mut builder = PipelineBuilder::new(&rd.device);

        let bgl = Texture::make_bind_group_layout(&rd.device, "SpritePipeline Bind Group Layout");
        let shader = Shader::from_path("crates/rend/shaders/sprite_shader.wgsl").unwrap();

        let layout = BufferResource::make_layout(&rd.device);

        let mesh = make_quad(&rd.device);

        builder.add_bind_group_layout(&bgl);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Sprite Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        Self { pipeline, mesh }
    }

    pub fn draw(
        &self,
        rp: &mut RenderPass,
        material: &BindGroup,
        rect_data: &RectDataBuffer,
        n: usize,
    ) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, material, &[]);
        rp.set_bind_group(1, rect_data.buffer().bind_group(), &[]);
        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}
