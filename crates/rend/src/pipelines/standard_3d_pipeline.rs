use crate::Texture;
use crate::*;
use wgpu::util::DeviceExt;
use wgpu::*;

pub struct Standard3DPipeline {
    standard: RenderPipeline,
    wireframe: RenderPipeline,
    draw_wireframes: bool,
    camera_ubo: UBO<glm::Mat4>,
    lighting_ubo: SingleUBO,
    transforms_ubo: UBO<glm::Mat4>,
}

impl Standard3DPipeline {
    pub fn new(
        device: &Device,
        ubo_bind_group_layout: &BindGroupLayout,
        time_etc_data_bind_group: &BindGroupLayout,
        config: &SurfaceConfiguration,
    ) -> Self {
        let bgl = Texture::make_bind_group_layout(device, "Texture Bind Group Layout");

        let camera_projection_bind_group_layout = {
            let mut builder = BindGroupLayoutBuilder::new(&device);
            builder.add_ubo();
            builder.build("Camera Projection UBO")
        };

        let lighting_bind_group_layout = {
            let mut builder = BindGroupLayoutBuilder::new(&device);
            builder.add_ubo();
            builder.build("Lighting UBO")
        };

        let mut builder = PipelineBuilder::new(&device);
        let shader = Shader::from_path("crates/rend/shaders/texture.wgsl").unwrap();
        builder.add_bind_group_layout(ubo_bind_group_layout);
        builder.add_bind_group_layout(&bgl);
        builder.add_bind_group_layout(time_etc_data_bind_group);
        builder.add_bind_group_layout(&camera_projection_bind_group_layout);
        builder.add_bind_group_layout(&lighting_bind_group_layout);
        let standard = builder.build_pipeline::<FullVertex>(
            "Standard 3D Pipeline",
            &shader,
            config.format,
            true,
            true,
        );

        let wireframe = {
            let mut builder = PipelineBuilder::new(&device);
            builder.add_bind_group_layout(&ubo_bind_group_layout);
            builder.add_bind_group_layout(&bgl);
            builder.add_bind_group_layout(&time_etc_data_bind_group);
            builder.add_bind_group_layout(&camera_projection_bind_group_layout);
            builder.add_bind_group_layout(&lighting_bind_group_layout);
            builder.wireframes();
            builder.build_pipeline::<FullVertex>(
                "Texture Pipeline",
                &shader,
                config.format,
                true,
                false,
            )
        };

        let camera_ubo = UBO::new(
            &device,
            1,
            camera_projection_bind_group_layout,
            "Standard 3D pipeline camera UBO",
        );

        let light_source: [f32; 3] = [4.0, 3.0, 5.0];

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shader Params"),
            contents: &bytemuck::bytes_of(&light_source),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lighting_bind_group = {
            let mut builder = BindGroupBuilder::new(&device);
            builder.set_layout(&lighting_bind_group_layout);
            builder.add_buffer(&buffer, 0);
            builder.build("uniform buffer")
        };

        let ubo_bind_group_layout = {
            let mut builder = BindGroupLayoutBuilder::new(&device);
            builder.add_ubo();
            builder.build("UBO Bind Group Layout")
        };

        let transforms_ubo = UBO::new(
            &device,
            250,
            ubo_bind_group_layout,
            "Standard 3D pipeline transforms UBO",
        );

        Self {
            standard,
            wireframe,
            draw_wireframes: false,
            camera_ubo,
            lighting_ubo: SingleUBO {
                buffer,
                bind_group: lighting_bind_group,
            },
            transforms_ubo,
        }
    }

    pub fn transforms(&self) -> &UBO<glm::Mat4> {
        &self.transforms_ubo
    }

    pub fn upload_transform(&self, i: u64, matrix: &glm::Mat4, queue: &Queue) {
        self.transforms_ubo.upload(i, matrix, queue);
    }

    pub fn set_draw_wireframes(&mut self, wireframes: bool) {
        self.draw_wireframes = wireframes;
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        if self.draw_wireframes {
            &self.wireframe
        } else {
            &self.standard
        }
    }

    pub fn set_bindings(&self, rp: &mut RenderPass) {
        rp.set_bind_group(3, self.camera_ubo.bind_group(0), &[]);
        rp.set_bind_group(4, &self.lighting_ubo.bind_group, &[]);
    }

    pub fn upload_camera_matrix(&mut self, view_proj: &glm::Mat4, queue: &Queue) {
        self.camera_ubo.upload(0, view_proj, queue);
    }
}
