use crate::Color;
use crate::*;
use wgpu::*;

pub struct SingleColorPipeline {
    pipeline: RenderPipeline,
    transforms_ubo: UBO<glm::Mat4>,
    color_ubo: UBO<glm::Vec4>,
}

fn make_ubo_layout(device: &Device, label: &str) -> BindGroupLayout {
    let mut builder = BindGroupLayoutBuilder::new(&device);
    builder.add_ubo();
    builder.build(label)
}

impl SingleColorPipeline {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let transforms_ubo_bind_group_layout = make_ubo_layout(device, "Shader Params");
        let color_ubo_bind_group_layout = make_ubo_layout(device, "Shader Params");

        let mut builder = PipelineBuilder::new(&device);
        let shader = Shader::from_path("crates/rend/shaders/single_color.wgsl");
        builder.add_bind_group_layout(&transforms_ubo_bind_group_layout);
        builder.add_bind_group_layout(&color_ubo_bind_group_layout);
        let pipeline = builder.build_pipeline::<FullVertex>(
            "Lava Lamp Pipeline",
            &shader,
            config.format,
            true,
            true,
        );

        let transforms_ubo = UBO::new(
            &device,
            1,
            transforms_ubo_bind_group_layout,
            "Single color pipeline transforms UBO",
        );
        let color_ubo = UBO::new(
            &device,
            1,
            color_ubo_bind_group_layout,
            "Single color pipeline color UBO",
        );

        Self {
            pipeline,
            transforms_ubo,
            color_ubo,
        }
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn draw(
        &self,
        rp: &mut RenderPass,
        mesh: &Mesh,
        transform: &glm::Mat4,
        color: &Color,
        queue: &Queue,
    ) {
        rp.set_pipeline(self.pipeline());
        self.transforms_ubo.upload(0, transform, queue);
        self.color_ubo.upload(0, &color.to_vec(), queue);
        rp.set_bind_group(0, self.transforms_ubo.bind_group(0 as usize), &[]);
        rp.set_bind_group(1, self.color_ubo.bind_group(0 as usize), &[]);
        draw_mesh(rp, mesh);
    }
}
