use crate::{Color, FontInfo};
use bary_core::prelude::{rotate_f64, Isometry2d};
use glam::DVec2;
use std::collections::BTreeMap;

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
    pub pos: DVec2,
    pub dims: DVec2,
    pub angle: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct CharCommand {
    pub pos: DVec2,
    pub dims: DVec2,
    pub angle: f64,
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
    pub start: DVec2,
    pub end: DVec2,
    pub thickness: f64,
    pub color: Color,
}

pub struct RenderCommands {
    pub fonts: BTreeMap<usize, FontInfo>,
    commands: Vec<BatchRenderCommand>,
    pub current_font_id: usize,
}

impl RenderCommands {
    pub fn new(fonts: BTreeMap<usize, FontInfo>) -> Self {
        let current_font_id = *fonts.keys().next().unwrap();
        Self {
            fonts,
            commands: Vec::new(),
            current_font_id,
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

    pub fn apply(&mut self, cmd: BatchRenderCommand) {
        self.commands.push(cmd);
    }

    pub fn rect(&mut self, p: impl Into<Isometry2d>) -> RectBuilder<'_> {
        let p = p.into();
        RectBuilder::new(self, p)
    }

    pub fn circle(&mut self, p: impl Into<DVec2>) -> CircleBuilder<'_> {
        let p = p.into();
        let builder: CircleBuilder<'_> = CircleBuilder::new(self, p.x, p.y);
        builder
    }

    pub fn line(&mut self, start: impl Into<DVec2>, end: impl Into<DVec2>) -> LineBuilder<'_> {
        let builder = LineBuilder::new(self, start.into(), end.into());
        builder
    }

    pub fn linestring(&mut self, points: Vec<DVec2>) -> LineStringBuilder<'_> {
        let builder = LineStringBuilder::new(self, points);
        builder
    }

    pub fn isometry(&mut self, iso: impl Into<Isometry2d>, length: f64) {
        let iso = iso.into();
        let c = iso.translation.as_dvec2();
        let x = c + rotate_f64(DVec2::X, iso.rotation as f64) * length;
        let y = c + rotate_f64(DVec2::Y, iso.rotation as f64) * length;

        self.line(c, x).thickness(6.0).color(Color::RED);
        self.line(c, y).thickness(6.0).color(Color::GREEN);
    }

    pub fn text(
        &mut self,
        iso: impl Into<Isometry2d>,
        text: impl AsRef<str>,
        font_size: f64,
        color: Color,
    ) -> (BatchRenderCommand, DVec2) {
        let iso = iso.into();
        self.isometry(iso, 30.0);
        self.paragraph(
            iso,
            self.current_font_id,
            font_size,
            text.as_ref(),
            None,
            color,
        )
    }

    pub fn frame(&mut self, p: impl Into<DVec2>, q: impl Into<DVec2>) -> LineStringBuilder<'_> {
        let p = p.into();
        let q = q.into();
        let a = DVec2::new(p.x, q.y);
        let b = DVec2::new(q.x, p.y);
        self.linestring(vec![p, a, q, b, p])
    }

    pub fn paragraph(
        &self,
        iso: impl Into<Isometry2d>,
        font_id: usize,
        font_size: f64,
        text: &str,
        layout_width: Option<f64>,
        color: Color,
    ) -> (BatchRenderCommand, DVec2) {
        let font = self.fonts.get(&font_id).unwrap();
        TextBuilder::new(iso, font, font_id, font_size, text, layout_width, color)
    }
}

pub struct TextBuilder<'a> {
    commands: &'a mut RenderCommands,
    command: BatchRenderCommand,
    extent: DVec2,
}

impl<'a> TextBuilder<'a> {
    pub fn new(
        iso: impl Into<Isometry2d>,
        font: &FontInfo,
        font_id: usize,
        font_size: f64,
        text: &str,
        layout_width: Option<f64>,
        color: Color,
    ) -> (BatchRenderCommand, DVec2) {
        let iso = iso.into();

        let right = iso.local_x().as_dvec2();
        let down = -iso.local_y().as_dvec2();

        let mut col_offset = 0;
        let mut row = 0;

        let mut cursor = iso.translation.as_dvec2() + down * font_size;

        let font_size = font_size / font.size as f64;

        let mut char_commands = Vec::new();
        let mut sum_x = 0.0;
        let mut max_sum_x: f64 = 0.0;

        for ch in text.chars() {
            if ch == '\n' {
                row += 1;
                cursor = iso.translation.as_dvec2()
                    + down * (row + 1) as f64 * font.size as f64 * font_size;
                col_offset = 0;
                sum_x = 0.0;
                continue;
            }

            let Some(data) = font.characters.get(&ch) else {
                continue;
            };

            if ch == ' ' && col_offset == 0 {
                continue;
            }

            if ch != ' ' {
                let dims = DVec2::new(data.width as f64, data.height as f64) * font_size;
                let yoff = (data.origin_y as f64 - data.height as f64) * font_size;
                let xoff = data.origin_x as f64 * font_size;
                let pos = cursor - yoff * down - xoff * right;
                char_commands.push(CharCommand {
                    pos,
                    dims,
                    c: ch,
                    color,
                    angle: iso.rotation as f64,
                });
            }

            col_offset += 1;

            let delta_x = data.advance as f64 * font_size;
            cursor += right * delta_x;
            sum_x += delta_x;

            max_sum_x = max_sum_x.max(sum_x);

            if let Some(layout_width) = layout_width {
                if ch == ' ' && sum_x > layout_width {
                    row += 1;
                    cursor = iso.translation.as_dvec2() + down * (row + 1) as f64;
                    col_offset = 0;
                    sum_x = 0.0;
                }
            }
        }

        let cmd = BatchRenderCommand::Char(font_id, char_commands);

        let extent = DVec2::new(max_sum_x, (row + 1) as f64 * (font.size as f64 * font_size));

        (cmd, extent)
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
            inner_radius: -4.0,
            outer_radius: 25.0,
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

    pub fn radii(&mut self, inner: f64, outer: f64) -> &mut Self {
        self.inner_radius = inner;
        self.outer_radius = outer;
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
    start: DVec2,
    end: DVec2,
    thickness: f64,
    color: Color,
}

impl<'a> LineBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, start: DVec2, end: DVec2) -> Self {
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

pub struct RectBuilder<'a> {
    commands: &'a mut RenderCommands,
    pos: DVec2,
    dims: DVec2,
    angle: f64,
    color: Color,
    centered: bool,
}

impl<'a> RectBuilder<'a> {
    pub fn new(commands: &'a mut RenderCommands, iso: impl Into<Isometry2d>) -> Self {
        let iso = iso.into();
        Self {
            commands,
            pos: iso.translation.as_dvec2(),
            dims: DVec2::splat(70.0),
            angle: iso.rotation as f64,
            color: Color::new(0.2, 1.0, 0.2, 1.0),
            centered: false,
        }
    }

    pub fn dims(&mut self, dims: impl Into<DVec2>) -> &mut Self {
        self.dims = dims.into();
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn angle(&mut self, angle: f64) -> &mut Self {
        self.angle = angle;
        self
    }

    pub fn centered(&mut self) -> &mut Self {
        self.centered = true;
        self
    }
}

impl<'a> Drop for RectBuilder<'a> {
    fn drop(&mut self) {
        let pos = if self.centered {
            let iso = Isometry2d::ZERO.with_rotation(self.angle as f32);
            let x = iso.local_x().as_dvec2();
            let y = iso.local_y().as_dvec2();
            let d = self.dims / 2.0;
            let off = x * d.x + y * d.y;
            self.pos - off
        } else {
            self.pos
        };

        let cmd = RectCommand {
            pos,
            dims: self.dims,
            angle: self.angle,
            color: self.color,
        };
        self.commands.enqueue(RenderCommand::Rect(cmd));

        // uncomment for debugging
        // let iso = Isometry2d::new(self.pos.as_vec2(), self.angle as f32);
        // self.commands.isometry(iso, 50.0);
    }
}

pub struct LineStringBuilder<'a> {
    commands: &'a mut RenderCommands,
    points: Vec<DVec2>,
    color: Color,
    thickness: f64,
}

impl<'a> LineStringBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, points: Vec<DVec2>) -> Self {
        Self {
            commands,
            points,
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

impl<'a> Drop for LineStringBuilder<'a> {
    fn drop(&mut self) {
        for points in self.points.windows(2) {
            self.commands
                .line(points[0], points[1])
                .color(self.color)
                .thickness(self.thickness);
        }
    }
}
