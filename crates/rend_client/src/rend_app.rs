use rend::RenderCommands;

pub trait RendApp {
    fn update(&mut self);

    fn emit_render_commands(&self) -> RenderCommands;

    fn on_event(&mut self, event: &glfw::WindowEvent);

    fn render(&mut self, commands: &RenderCommands);

    fn should_close(&self) -> bool;
}
