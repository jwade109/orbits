use glam::DVec2;

use crate::{Color, FontInfo};
use std::collections::BTreeMap;

pub type Vec2d = glm::Vector2<f64>;

#[derive(Debug, Clone)]
pub enum RenderCommand {
    Char(CharCommand),
    Rect(RectCommand),
    Circle(CircleCommand),
    Line(LineCommand),
}

#[derive(Debug, Clone)]
pub enum BatchRenderCommand {
    Char(usize, Vec<CharCommand>),
    Rect(Vec<RectCommand>),
    Circle(Vec<CircleCommand>),
    Line(Vec<LineCommand>),
}

impl std::fmt::Display for BatchRenderCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Char(_, c) => write!(f, "BatchRenderCommand::Char({} elements)", c.len()),
            Self::Rect(c) => write!(f, "BatchRenderCommand::Rect({} elements)", c.len()),
            Self::Circle(c) => write!(f, "BatchRenderCommand::Circ({} elements)", c.len()),
            Self::Line(c) => write!(f, "BatchRenderCommand::Line({} elements)", c.len()),
        }
    }
}

impl BatchRenderCommand {
    fn new(command: RenderCommand) -> Self {
        match command {
            RenderCommand::Char(_) => unimplemented!(),
            RenderCommand::Rect(c) => Self::Rect(vec![c]),
            RenderCommand::Circle(c) => Self::Circle(vec![c]),
            RenderCommand::Line(c) => Self::Line(vec![c]),
        }
    }

    fn try_enqueue(&mut self, command: RenderCommand) -> bool {
        match (self, command) {
            (BatchRenderCommand::Char(_, s), RenderCommand::Char(c)) => {
                s.push(c);
                true
            }
            (BatchRenderCommand::Rect(s), RenderCommand::Rect(c)) => {
                s.push(c);
                true
            }
            (BatchRenderCommand::Circle(s), RenderCommand::Circle(c)) => {
                s.push(c);
                true
            }
            (BatchRenderCommand::Line(s), RenderCommand::Line(c)) => {
                s.push(c);
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RectCommand {
    pub pos: Vec2d,
    pub dims: Vec2d,
    pub angle: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct CharCommand {
    pub pos: Vec2d,
    pub dims: Vec2d,
    pub c: char,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct CircleCommand {
    pub x: f64,
    pub y: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct LineCommand {
    pub start: Vec2d,
    pub end: Vec2d,
    pub thickness: f64,
    pub color: Color,
}

pub struct RenderCommands {
    pub fonts: BTreeMap<usize, FontInfo>,
    commands: Vec<BatchRenderCommand>,
}

impl RenderCommands {
    pub fn new(fonts: BTreeMap<usize, FontInfo>) -> Self {
        Self {
            fonts,
            commands: Vec::new(),
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = &BatchRenderCommand> {
        self.commands.iter()
    }

    pub fn enqueue(&mut self, command: RenderCommand) {
        let is_batched = self
            .commands
            .last_mut()
            .map(|last| last.try_enqueue(command.clone()))
            .unwrap_or(false);

        if !is_batched {
            let b = BatchRenderCommand::new(command);
            self.commands.push(b);
        }
    }

    pub fn rect(&mut self, pos: Vec2d, dims: Vec2d, angle: f64, color: Color) {
        self.enqueue(RenderCommand::Rect(RectCommand {
            pos,
            dims,
            angle,
            color,
        }));
    }

    pub fn circle(&mut self, x: f64, y: f64) -> CircleBuilder<'_> {
        let builder: CircleBuilder<'_> = CircleBuilder::new(self, x, y);
        builder
    }

    pub fn circle_new(&mut self, p: DVec2) -> CircleBuilder<'_> {
        let builder: CircleBuilder<'_> = CircleBuilder::new(self, p.x, p.y);
        builder
    }

    pub fn line(&mut self, start: impl Into<Vec2d>, end: impl Into<Vec2d>) -> LineBuilder<'_> {
        let builder = LineBuilder::new(self, start.into(), end.into());
        builder
    }

    pub fn text(&mut self, p: Vec2d, text: impl AsRef<str>, font_size: f64) -> DVec2 {
        let font_id = self.fonts.keys().next().unwrap();
        self.paragraph(*font_id, font_size, p.x, p.y, text.as_ref(), None)
    }

    pub fn frame(&mut self, p: Vec2d, q: Vec2d, t: f64) {
        let a = Vec2d::new(p.x, q.y);
        let b = Vec2d::new(q.x, p.y);
        self.line(p, a).thickness(t);
        self.line(a, q).thickness(t);
        self.line(q, b).thickness(t);
        self.line(b, p).thickness(t);
    }

    pub fn paragraph(
        &mut self,
        font_id: usize,
        font_size: f64,
        x_origin: f64,
        y_origin: f64,
        text: &str,
        layout_width: Option<f64>,
    ) -> DVec2 {
        // TODO this is terrible
        let font = self.fonts.get(&font_id).unwrap().clone();

        let font_size = font_size / font.size as f64;

        let mut col_offset = 0;

        let mut x = x_origin;
        let mut y = y_origin;

        let mut char_commands = Vec::new();

        let mut extent = DVec2::ZERO;

        for ch in text.chars() {
            if ch == '\n' {
                y += font.size as f64 * font_size;
                x = x_origin;
                col_offset = 0;
                continue;
            }

            let Some(data) = font.characters.get(&ch) else {
                continue;
            };

            if ch == ' ' && col_offset == 0 {
                continue;
            }

            let w = data.width as f64 * font_size;
            let h = data.height as f64 * font_size;

            let xt = x - data.origin_x as f64 * font_size;
            let yt = y - data.origin_y as f64 * font_size + font.size as f64 * font_size;

            let color = Color::WHITE;

            let pos = Vec2d::new(xt, yt);
            let dims = Vec2d::new(w, h);

            extent.x = extent.x.max(pos.x + dims.x);
            extent.y = extent.y.max(pos.y + dims.y);

            if ch != ' ' {
                char_commands.push(CharCommand {
                    pos,
                    dims,
                    c: ch,
                    color,
                });
            }

            col_offset += 1;

            x += data.advance as f64 * font_size;

            if let Some(layout_width) = layout_width {
                if ch == ' ' && x + w > x_origin + layout_width {
                    y += font.size as f64 * font_size;
                    x = x_origin;
                    col_offset = 0;
                }
            }
        }

        let batch = BatchRenderCommand::Char(font_id, char_commands);

        self.commands.push(batch);

        extent
    }
}

pub struct CircleBuilder<'a> {
    commands: &'a mut RenderCommands,
    x: f64,
    y: f64,
    inner_radius: f64,
    outer_radius: f64,
    color: Color,
}

impl<'a> CircleBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, x: f64, y: f64) -> Self {
        Self {
            commands,
            x,
            y,
            inner_radius: -20.0,
            outer_radius: 50.0,
            color: Color::new(0.0, 0.3, 1.0, 0.8),
        }
    }

    pub fn radius(&mut self, radius: f64) -> &mut Self {
        self.outer_radius = radius;
        self
    }

    pub fn inner_radius(&mut self, radius: f64) -> &mut Self {
        self.inner_radius = radius;
        self
    }

    pub fn diameter(&mut self, diameter: f64) -> &mut Self {
        self.outer_radius = diameter / 2.0;
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }
}

impl<'a> Drop for CircleBuilder<'a> {
    fn drop(&mut self) {
        let circle = CircleCommand {
            x: self.x,
            y: self.y,
            inner_radius: self.inner_radius,
            outer_radius: self.outer_radius,
            color: self.color,
        };

        if self.inner_radius < self.outer_radius && self.outer_radius > 0.0 {
            self.commands.enqueue(RenderCommand::Circle(circle));
        }
    }
}

pub struct LineBuilder<'a> {
    commands: &'a mut RenderCommands,
    start: Vec2d,
    end: Vec2d,
    thickness: f64,
    color: Color,
}

impl<'a> LineBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, start: Vec2d, end: Vec2d) -> Self {
        Self {
            commands,
            start,
            end,
            thickness: 10.0,
            color: Color::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn thickness(&mut self, thickness: f64) -> &mut Self {
        self.thickness = thickness;
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }
}

impl<'a> Drop for LineBuilder<'a> {
    fn drop(&mut self) {
        let line = LineCommand {
            start: self.start,
            end: self.end,
            thickness: self.thickness,
            color: self.color,
        };
        self.commands.enqueue(RenderCommand::Line(line));
    }
}
