use crate::*;
use wgpu::*;

pub struct BlurPipeline {
    pipeline: RenderPipeline,
    mesh: Mesh,
}

pub fn material_bind_group_layout(device: &Device, label: &str) -> BindGroupLayout {
    let mut builder = BindGroupLayoutBuilder::new(&device);
    builder.add_material();
    builder.build(label)
}

impl BlurPipeline {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let mesh = make_quad(device);
        let bgl = material_bind_group_layout(device, "BlurPipeline Bind Group Layout");
        let shader = Shader::from_path("crates/rend/shaders/blur_shader.wgsl");

        let mut builder = PipelineBuilder::new(&device);
        builder.add_bind_group_layout(&bgl);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Single Texture Pipeline",
            &shader,
            config.format,
            true,
            true,
        );

        Self { pipeline, mesh }
    }

    pub fn blur_pass(&self, rp: &mut RenderPass, material: &BindGroup) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, material, &[]);
        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..1);
    }
}
