use crate::{render_world::RenderWorld, *};
use glam::DVec2;
use wgpu::SurfaceTexture;

struct Pipelines {
    lava_lamp_pipeline: LavaLampPipeline,
    blur_pipeline: BlurPipeline,
    #[allow(unused)]
    shadow_pipeline: ShadowPipeline,
    text_pipeline: TextPipeline,
    circle_pipeline: CirclePipeline,
    line_pipeline: LinePipeline,
    rectangle_pipeline: RectanglePipeline,
    chunk_pipeline: RectanglePipeline,
    sprite_pipeline: SpritePipeline,
}

pub struct RenderState<'a> {
    pub renderer: Renderer<'a>,
    pub world: RenderWorld,
    pub window: &'a mut glfw::Window,

    pipelines: Pipelines,

    depth_texture: Texture,
    im_tex_1: Texture,
    im_tex_2: Texture,
}

impl<'a> RenderState<'a> {
    pub async fn new(window: &'a mut glfw::Window) -> Self {
        let renderer = Renderer::new(window).await;

        let uniform_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shader Params"),
            size: 40,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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

        let rect_data = RectDataBuffer::new(&renderer, RectanglePipeline::RECTS_PER_PASS);

        let (rectangle_pipeline, _) =
            RectanglePipeline::new(&renderer, "crates/rend/shaders/rectangle.wgsl");

        let (chunk_pipeline, height_data_chunks) =
            RectanglePipeline::new(&renderer, "crates/rend/shaders/chunk_terrain.wgsl");

        let sprite_pipeline = SpritePipeline::new(&renderer);

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

        let shadow_pipeline = ShadowPipeline::new(&renderer);

        let world = RenderWorld::new(rect_data, height_data_chunks);

        let pipelines = Pipelines {
            lava_lamp_pipeline,
            blur_pipeline,
            shadow_pipeline,
            text_pipeline,
            circle_pipeline,
            line_pipeline,
            rectangle_pipeline,
            chunk_pipeline,
            sprite_pipeline,
        };

        Self {
            renderer,
            window,
            pipelines,
            world,
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

    #[allow(unused)]
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

    fn draw_circles(&self, view: &wgpu::TextureView, commands: &[CircleCommand]) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let mut passes = 0;

        for chunk in commands.chunks(CirclePipeline::MAX_CIRCLES_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

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

        passes
    }

    fn draw_lines(&self, view: &wgpu::TextureView, commands: &[LineCommand]) -> usize {
        let (sx, sy) = self.window.get_size();

        let mut passes = 0;

        for chunk in commands.chunks(LinePipeline::MAX_LINES_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

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

        passes
    }

    fn draw_rectangles(&self, view: &wgpu::TextureView, commands: &[RectCommand]) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let mut passes = 0;

        for cmds in commands.chunks(RectanglePipeline::RECTS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

            self.world
                .rect_data
                .write(&self.renderer.queue, cmds, screen);

            self.pipelines.rectangle_pipeline.draw(
                &mut rp,
                cmds.len(),
                &[
                    self.world.rect_data.buffer(),
                    &self.world.height_data_chunks,
                ],
            );

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn draw_chunks(&self, view: &wgpu::TextureView, commands: &[ChunkCommand]) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let mut passes = 0;

        for cmds in commands.chunks(RectanglePipeline::RECTS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let height_data: Vec<u8> = cmds
                .iter()
                .map(|c| c.height.to_vec())
                .flat_map(|c| c.iter().map(|c| c.to_le_bytes()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
                .concat();

            self.world
                .height_data_chunks
                .write(&self.renderer.queue, &height_data);

            let rcmds: Vec<_> = cmds
                .iter()
                .map(|c| RectCommand {
                    pos: c.pos,
                    dims: c.dims,
                    angle: c.angle,
                    fill: RectFill::Color(Color::PURPLE.alpha(0.3)),
                })
                .collect();

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view);

            self.world
                .rect_data
                .write(&self.renderer.queue, &rcmds, screen);

            self.pipelines.chunk_pipeline.draw(
                &mut rp,
                cmds.len(),
                &[
                    self.world.rect_data.buffer(),
                    &self.world.height_data_chunks,
                ],
            );

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn draw_ui(&self, view: &wgpu::TextureView, font_id: Ent, commands: &[CharCommand]) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let (font, material) = self.world.fonts.get(font_id).unwrap();

        let mut passes = 0;

        for chunk in commands.chunks(TextPipeline::MAX_CHARS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

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

        passes
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

    #[allow(unused)]
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

    pub fn apply_geometry_commands(
        &self,
        commands: &RenderCommands,
        view: &wgpu::TextureView,
    ) -> usize {
        let mut passes = 0;
        for cmd in commands.commands() {
            passes += match cmd {
                // BatchRenderCommand::Char(font_id, c) => self.draw_ui(&view, *font_id, &c),
                // BatchRenderCommand::Rect(c) => self.draw_rectangles(view, &c),
                // BatchRenderCommand::Circle(c) => self.draw_circles(view, &c),
                BatchRenderCommand::Line(c) => self.draw_lines(view, &c),
                // BatchRenderCommand::Chunk(c) => self.draw_chunks(view, c),
                _ => 0,
            }
        }
        passes
    }

    fn draw_sprites(&self, view: &wgpu::TextureView, commands: &RenderCommands) {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        for batch in commands.commands() {
            let BatchRenderCommand::Rect(cmds) = batch else {
                continue;
            };

            let sprite_commands: Vec<(&RectCommand, Ent)> = cmds
                .iter()
                .filter_map(|c| {
                    if let RectFill::Sprite(id) = c.fill {
                        Some((c, id))
                    } else {
                        None
                    }
                })
                .collect();

            for (chunk, id) in sprite_commands {
                let mut command_encoder = self
                    .renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let mut rp = self.get_render_pass(&mut command_encoder, None, view);

                self.world
                    .rect_data
                    .write(&self.renderer.queue, &[*chunk], screen);

                let texture = self.world.textures.get(id).unwrap();

                self.pipelines.sprite_pipeline.draw(
                    &mut rp,
                    &texture.bind_group,
                    &self.world.rect_data,
                    1,
                );
                drop(rp);

                self.renderer
                    .queue
                    .submit(std::iter::once(command_encoder.finish()));
            }
        }
    }

    pub fn render(
        &mut self,
        commands: &RenderCommands,
    ) -> Result<Option<(SurfaceTexture, usize)>, wgpu::SurfaceError> {
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
        self.draw_sprites(&view, commands);
        let passes = self.apply_geometry_commands(commands, &view);

        self.renderer.device.poll(wgpu::Maintain::wait());

        Ok(Some((drawable, passes)))
    }
}
