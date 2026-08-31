use crate::Color;
use crate::*;
use glam::DVec2;
use glm::Mat4;
use wgpu::*;

pub struct CirclePipeline {
    pipeline: RenderPipeline,
    colors: BufferResource,
    transforms: BufferResource,
    radius: BufferResource,
    mesh: Mesh,
}

impl CirclePipeline {
    pub const MAX_CIRCLES_PER_PASS: usize = 480;

    pub fn new(rd: &Renderer) -> Self {
        let colors =
            BufferResource::new_array(&rd.device, Self::MAX_CIRCLES_PER_PASS, 16, "Circle colors");
        let transforms = BufferResource::new_array(
            &rd.device,
            Self::MAX_CIRCLES_PER_PASS,
            64,
            "Circle transforms",
        );

        let layout = BufferResource::make_layout(&rd.device);

        // the stride is 16 here because that's the minimum.
        // the element is 4 bytes large
        let radius =
            BufferResource::new_array(&rd.device, Self::MAX_CIRCLES_PER_PASS, 16, "Circle radii");

        let mesh = make_quad(&rd.device);

        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/circle.wgsl");

        builder.add_bind_group_layout(&layout);
        builder.add_bind_group_layout(&layout);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Circle Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        Self {
            pipeline,
            colors,
            transforms,
            radius,
            mesh,
        }
    }

    pub fn set_color(&self, queue: &Queue, i: usize, color: Color) {
        self.colors
            .upload(queue, 16 * i as u64, any_as_u8_slice(&color.to_vec()))
    }

    pub fn set_transform(&self, queue: &Queue, i: usize, transform: &Mat4) {
        self.transforms
            .upload(queue, 64 * i as u64, any_as_u8_slice(transform));
    }

    pub fn set_radius(&self, queue: &Queue, i: usize, inner_radius: f32, outer_radius: f32) {
        // the stride is 16 here because that's the minimum.
        // the element is 8 bytes large
        let data: [f32; 2] = [inner_radius, outer_radius];

        self.radius
            .upload(queue, 16 * i as u64, any_as_u8_slice(&data))
    }

    pub fn assign_buffer_data(&self, queue: &Queue, commands: &[CircleCommand], screen: DVec2) {
        for (i, cmd) in commands.iter().enumerate() {
            let ul_x = cmd.x - cmd.outer_radius;
            let ul_y = cmd.y - cmd.outer_radius;
            let pos = DVec2::new(ul_x, ul_y);
            let dims = DVec2::new(cmd.outer_radius, cmd.outer_radius) * 2.0;
            let pos = pos.with_y(screen.y - pos.y - cmd.outer_radius * 2.0);
            let transform = screen_space_transform(pos, dims, screen, 0.0);
            self.set_transform(queue, i, &transform);
            self.set_color(queue, i, cmd.color);
            self.set_radius(queue, i, cmd.inner_radius as f32, cmd.outer_radius as f32);
        }
    }

    pub fn draw_circles(&self, rp: &mut RenderPass, n: usize) {
        rp.set_pipeline(&self.pipeline);

        rp.set_bind_group(0, self.colors.bind_group(), &[]);
        rp.set_bind_group(1, self.transforms.bind_group(), &[]);
        rp.set_bind_group(2, self.radius.bind_group(), &[]);

        let n = n.min(Self::MAX_CIRCLES_PER_PASS);

        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}
