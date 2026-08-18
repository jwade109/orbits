use crate::*;
use glm::*;
use std::collections::HashMap;
use wgpu::SurfaceTexture;

pub struct MeshObject {
    pub position: Vec3,
    pub angle: f32,
    pub vel: f32,
    pub mesh_id: usize,
    pub should_animate: bool,
}

impl MeshObject {
    pub fn get_transform_matrix(&self) -> Matrix4<f32> {
        let eye = mat4_identity();
        let matrix = ext::translate(&eye, self.position)
            * ext::rotate(&eye, self.angle, glm::Vector3::new(0.0, 0.0, 1.0));

        matrix
    }
}

struct Pipelines {
    lava_lamp_pipeline: LavaLampPipeline,
    blur_pipeline: BlurPipeline,
    shadow_pipeline: ShadowPipeline,
    standard_3d_pipeline: Standard3DPipeline,
    single_color_pipeline: SingleColorPipeline,
    text_pipeline: TextPipeline,
    circle_pipeline: CirclePipeline,
    line_pipeline: LinePipeline,
}

pub struct RenderResources {
    pub fonts: HashMap<usize, (FontInfo, Texture)>,
    pub meshes: HashMap<usize, Mesh>,
    pub textures: HashMap<usize, Texture>,
    pub next_resource_id: usize,
}

impl RenderResources {
    pub fn spawn_mesh(&mut self, mesh: Mesh) -> usize {
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        self.meshes.insert(id, mesh);
        id
    }

    pub fn load_texture(&mut self, rd: &Renderer, path: &str) -> usize {
        let sprite = Texture::load_sprite(path, rd).unwrap();
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        self.textures.insert(id, sprite);
        id
    }

    pub fn load_font(&mut self, rd: &Renderer, name: &str) -> usize {
        println!("Loading font {name}");
        let data_path = format!("assets/font_textures/{name}/font_data.json");
        let texture_path = format!("assets/font_textures/{name}/font.png");
        let texture = Texture::load_sprite(&texture_path, rd).unwrap();
        let font = FontInfo::from_file(&data_path).unwrap();
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        self.fonts.insert(id, (font, texture));
        id
    }

    pub fn spawn_ground_plane(
        &mut self,
        device: &wgpu::Device,
        x: i32,
        z: i32,
        n_quads: u16,
    ) -> usize {
        let mesh = make_rough_ground_plane(device, Vec2::new(x as f32, z as f32), n_quads);
        self.spawn_mesh(mesh)
    }
}

pub struct RenderState<'a> {
    pub renderer: Renderer<'a>,
    pub resources: RenderResources,
    pub window: &'a mut glfw::Window,

    pipelines: Pipelines,

    invincible: Texture,
    depth_texture: Texture,
    im_tex_1: Texture,
    im_tex_2: Texture,

    common_shader_info: SingleUBO,
}

impl<'a> RenderState<'a> {
    pub async fn new(window: &'a mut glfw::Window) -> Self {
        let renderer = Renderer::new(window).await;

        let bgl = material_bind_group_layout(&renderer.device, "Texture Bind Group Layout");

        let ubo_bind_group_layout = {
            let mut builder = BindGroupLayoutBuilder::new(&renderer.device);
            builder.add_ubo();
            builder.build("UBO Bind Group Layout")
        };

        let uniform_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shader Params"),
            size: 40,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let invincible = Texture::load_sprite("assets/invincible.jpg", &renderer).unwrap();
        let depth_texture = Texture::depth_texture(&renderer, "depth_texture");
        let im_tex_1 = Texture::blank_texture(&renderer, "im_tex_1");
        let im_tex_2 = Texture::blank_texture(&renderer, "im_tex_2");

        let time_etc_data_bind_group =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("uniform_data_bind_group"),
                });

        let single_color_pipeline = SingleColorPipeline::new(&renderer.device, &renderer.config);

        let uniform_bind_group = {
            let mut builder = BindGroupBuilder::new(&renderer.device);
            builder.set_layout(&time_etc_data_bind_group);
            builder.add_buffer(&uniform_buffer, 0);
            builder.build("uniform buffer")
        };

        let text_pipeline = TextPipeline::new(&renderer);
        let blur_pipeline = BlurPipeline::new(&renderer.device, &renderer.config);
        let circle_pipeline = CirclePipeline::new(&renderer);
        let lava_lamp_pipeline = LavaLampPipeline::new(&renderer);
        let line_pipeline = LinePipeline::new(&renderer);

        let standard_3d_pipeline = Standard3DPipeline::new(
            &renderer.device,
            &ubo_bind_group_layout,
            &bgl,
            &time_etc_data_bind_group,
            &renderer.config,
        );

        let shadow_pipeline = ShadowPipeline::new(&renderer);

        let resources = RenderResources {
            fonts: HashMap::new(),
            meshes: HashMap::new(),
            textures: HashMap::new(),
            next_resource_id: 0,
        };

        let pipelines = Pipelines {
            lava_lamp_pipeline,
            blur_pipeline,
            shadow_pipeline,
            standard_3d_pipeline,
            single_color_pipeline,
            text_pipeline,
            circle_pipeline,
            line_pipeline,
        };

        Self {
            renderer,
            window,
            pipelines,
            resources,
            common_shader_info: SingleUBO {
                buffer: uniform_buffer,
                bind_group: uniform_bind_group,
            },
            invincible,
            depth_texture,
            im_tex_1,
            im_tex_2,
        }
    }

    pub fn resize(&mut self, new_size: (i32, i32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.renderer.config.width = new_size.0 as u32;
            self.renderer.config.height = new_size.1 as u32;
            self.renderer
                .surface
                .configure(&self.renderer.device, &self.renderer.config);
            self.depth_texture = Texture::depth_texture(&self.renderer, "depth_texture");
            self.im_tex_1 = Texture::blank_texture(&self.renderer, "im_tex_1");
            self.im_tex_2 = Texture::blank_texture(&self.renderer, "im_tex_2");
        }
    }

    pub fn update_surface(&mut self) {
        self.renderer.surface = self
            .renderer
            .instance
            .create_surface(self.window.render_context())
            .unwrap();
    }

    fn draw_lava(&self, view: &wgpu::TextureView, time: f32) {
        let mut command_encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

        let mouse_pos = self.window.get_cursor_pos();

        let shader_params = ShaderParams {
            mouse: (mouse_pos.0 as f32, mouse_pos.1 as f32),
            time,
            resolution: (
                self.window.get_size().0 as f32,
                self.window.get_size().1 as f32,
            ),
        };

        let transform = mat4_identity();
        self.pipelines.lava_lamp_pipeline.draw(
            &mut rp,
            &transform,
            &shader_params,
            &self.renderer.queue,
        );

        drop(rp);
        self.renderer
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    fn clear(&self, view: &wgpu::TextureView, color: Color) {
        let mut command_encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let rp = self.get_render_pass(&mut command_encoder, Some(color.to_wgpu()), &view);

        drop(rp);

        self.renderer
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    fn draw_3d(&self, meshes: &[MeshObject], view: &wgpu::TextureView) {
        let mut command_encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

        rp.set_pipeline(self.pipelines.standard_3d_pipeline.pipeline());
        rp.set_bind_group(2, &self.common_shader_info.bind_group, &[]);
        self.pipelines.standard_3d_pipeline.set_bindings(&mut rp);

        for i in 0..meshes.len() {
            let matrix: Matrix4<f32> = meshes[i].get_transform_matrix();
            self.pipelines.standard_3d_pipeline.upload_transform(
                i as u64,
                &matrix,
                &self.renderer.queue,
            );
        }

        rp.set_bind_group(
            1,
            &self.resources.textures.values().next().unwrap().bind_group,
            &[],
        );

        for i in 0..meshes.len() {
            let bg = self
                .pipelines
                .standard_3d_pipeline
                .transforms()
                .bind_group(i);

            let Some(mesh) = self.resources.meshes.get(&meshes[i].mesh_id) else {
                println!("Failed to get mesh of type {:?}", meshes[i].mesh_id);
                continue;
            };

            mesh.set_as_active(&mut rp);

            rp.set_bind_group(0, bg, &[]);
            rp.draw_indexed(0..mesh.index_count(), 0, 0..1);
        }

        drop(rp);

        self.renderer
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    fn draw_circles(&self, view: &wgpu::TextureView, commands: &[CircleCommand]) {
        let (sx, sy) = self.window.get_size();
        let screen = Vec2d::new(sx as f64, sy as f64);

        for chunk in commands.chunks(CirclePipeline::MAX_CHARS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

            self.pipelines
                .circle_pipeline
                .assign_buffer_data(&self.renderer.queue, chunk, screen);

            self.pipelines
                .circle_pipeline
                .draw_circles(&mut rp, chunk.len());

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }
    }

    fn draw_lines(&self, view: &wgpu::TextureView, commands: &[LineCommand]) {
        let (sx, sy) = self.window.get_size();
        for chunk in commands.chunks(LinePipeline::MAX_LINES_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

            self.pipelines.line_pipeline.assign_buffer_data(
                &self.renderer.queue,
                chunk,
                sx as f64,
                sy as f64,
            );

            self.pipelines
                .line_pipeline
                .draw_lines(&mut rp, chunk.len());

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }
    }

    fn draw_rectangles(&self, view: &wgpu::TextureView, commands: &[RectCommand]) {
        let (sx, sy) = self.window.get_size();
        let screen = Vec2d::new(sx as f64, sy as f64);

        for cmd in commands {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

            let tf = screen_space_transform(cmd.pos, cmd.dims, screen, cmd.angle);

            let mesh = make_quad(&self.renderer.device);

            self.pipelines.single_color_pipeline.draw(
                &mut rp,
                &mesh,
                &tf,
                &cmd.color,
                &self.renderer.queue,
            );

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }
    }

    fn draw_ui(&self, view: &wgpu::TextureView, font_id: usize, commands: &[CharCommand]) {
        let (sx, sy) = self.window.get_size();
        let screen = Vec2d::new(sx as f64, sy as f64);

        let (font, material) = self.resources.fonts.get(&font_id).unwrap();

        for chunk in commands.chunks(TextPipeline::MAX_CHARS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

            self.pipelines.text_pipeline.assign_buffer_data(
                &self.renderer.queue,
                chunk,
                font,
                screen,
            );
            self.pipelines
                .text_pipeline
                .draw_text(&mut rp, material, chunk.len());

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }
    }

    pub fn update(&mut self, view_proj: Mat4, time: f32) {
        self.pipelines
            .standard_3d_pipeline
            .upload_camera_matrix(&view_proj, &self.renderer.queue);

        let mouse_pos = self.window.get_cursor_pos();

        let shader_params = ShaderParams {
            mouse: (mouse_pos.0 as f32, mouse_pos.1 as f32),
            time,
            resolution: (
                self.window.get_size().0 as f32,
                self.window.get_size().1 as f32,
            ),
        };

        self.renderer.queue.write_buffer(
            &self.common_shader_info.buffer,
            0,
            &shader_params.to_bytes(),
        );
    }

    fn get_render_pass<'b>(
        &self,
        command_encoder: &'b mut wgpu::CommandEncoder,
        clear_color: Option<wgpu::Color>,
        view: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'b> {
        self.renderer
            .get_render_pass(command_encoder, clear_color, view, &self.depth_texture)
    }

    fn blur_pass(&self, incoming: &Texture, outgoing: &wgpu::TextureView) {
        let mut command_encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut rp = self.get_render_pass(&mut command_encoder, None, &outgoing);

        self.pipelines
            .blur_pipeline
            .blur_pass(&mut rp, &incoming.bind_group);

        drop(rp);
        self.renderer
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    pub fn apply_geometry_commands(&self, commands: &RenderCommands, view: &wgpu::TextureView) {
        for cmd in commands.commands() {
            match cmd {
                BatchRenderCommand::Char(font_id, c) => self.draw_ui(&view, *font_id, &c),
                // BatchRenderCommand::Rect(c) => self.draw_rectangles(view, &c),
                BatchRenderCommand::Circle(c) => self.draw_circles(view, &c),
                BatchRenderCommand::Line(c) => self.draw_lines(view, &c),
                _ => (),
            }
        }
    }

    pub fn render(
        &mut self,
        commands: &RenderCommands,
    ) -> Result<Option<SurfaceTexture>, wgpu::SurfaceError> {
        let (w, h) = self.window.get_size();

        if w == 0 || h == 0 {
            return Ok(None);
        }

        self.renderer.device.poll(wgpu::Maintain::wait());

        {
            let event = self.renderer.queue.submit([]);
            let maintain = wgpu::Maintain::WaitForSubmissionIndex(event);
            self.renderer.device.poll(maintain);
        }

        let drawable = self.renderer.surface.get_current_texture()?;

        let view = drawable
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.clear(&view, Color::rgb(117, 186, 255, 1.0));
        // self.draw_lava(&view, 0.0);
        // self.draw_3d(&[], &view);
        // self.blur_pass(&self.im_tex_1, &view);
        self.apply_geometry_commands(commands, &view);

        self.renderer.device.poll(wgpu::Maintain::wait());

        Ok(Some(drawable))
    }
}
