use crate::{Shader, Texture, Vertex};

pub struct PipelineBuilder<'a> {
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    device: &'a wgpu::Device,
    draw_wireframes: bool,
    has_vertex_buffer: bool,
}

fn make_shader_module(device: &wgpu::Device, shader: &Shader, label: &str) -> wgpu::ShaderModule {
    let desc = wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader.contents.clone().into()),
    };
    device.create_shader_module(desc)
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            bind_group_layouts: Vec::new(),
            device: device,
            draw_wireframes: false,
            has_vertex_buffer: true,
        }
    }

    pub fn add_bind_group_layout(&mut self, layout: &'a wgpu::BindGroupLayout) {
        self.bind_group_layouts.push(layout);
    }

    pub fn wireframes(&mut self) {
        self.draw_wireframes = true;
    }

    pub fn no_vertex_buffer(&mut self) {
        self.has_vertex_buffer = false;
    }

    pub fn build_pipeline<T: Vertex>(
        self,
        label: &str,
        shader: &Shader,
        pixel_format: wgpu::TextureFormat,
        has_depth_stencil: bool,
        cull_backface: bool,
    ) -> wgpu::RenderPipeline {
        let vertex_buffer_layouts = if self.has_vertex_buffer {
            vec![T::get_layout()]
        } else {
            vec![]
        };

        let shader_module = make_shader_module(&self.device, shader, "Shader Module");

        let pipeline_layout = {
            let pipeline_layout_descriptor = wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            };
            self.device
                .create_pipeline_layout(&pipeline_layout_descriptor)
        };

        let depth_stencil = has_depth_stencil.then(|| wgpu::DepthStencilState {
            format: Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),

            cache: None,

            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some(&shader.vertex_entry),
                buffers: &vertex_buffer_layouts,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: cull_backface.then(|| wgpu::Face::Back),
                polygon_mode: if self.draw_wireframes {
                    wgpu::PolygonMode::Line
                } else {
                    wgpu::PolygonMode::Fill
                },
                unclipped_depth: false,
                conservative: false,
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some(&shader.fragment_entry),
                targets: &[Some(wgpu::ColorTargetState {
                    format: pixel_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            depth_stencil,

            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        };

        let pipeline = self
            .device
            .create_render_pipeline(&render_pipeline_descriptor);

        pipeline
    }
}
