use crate::mouse::InputState;
use crate::mouse::{FrameId, MouseButt};
use crate::planetary::GameState;
use crate::scenes::{CameraProjection, Interactive, Render, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct EditorContext {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,

    aabbs: Vec<AABB>,
    points: HashSet<IVec2>,
    lines: Vec<Vec<Vec2>>,

    parts: Vec<IVec2>,
}

impl Render for EditorContext {
    fn text_labels(_state: &GameState) -> Vec<TextLabel> {
        vec![]
    }

    fn sprites(state: &GameState) -> Vec<StaticSpriteDescriptor> {
        let parts = ["frame", "tank11", "tank21", "tank22", "motor", "antenna"];
        let ctx = &state.editor_context;
        let mut ret: Vec<_> = parts
            .iter()
            .enumerate()
            .map(|(i, path)| StaticSpriteDescriptor {
                position: ctx.w2c(Vec2::X * i as f32 * 40.0),
                path: format!("embedded://game/../assets/parts/{}.png", path),
                scale: ctx.scale(),
                z_index: 1.0,
            })
            .collect();

        if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
            let p = ctx.c2w(p);
            ret.push(StaticSpriteDescriptor {
                position: ctx.w2c(vround(p).as_vec2()),
                path: "embedded://game/../assets/parts/frame.png".to_string(),
                scale: ctx.scale(),
                z_index: 10.0,
            });
        }

        ret.extend(ctx.parts.iter().map(|p| StaticSpriteDescriptor {
            position: ctx.w2c(p.as_vec2()),
            path: "embedded://game/../assets/parts/frame.png".to_string(),
            scale: ctx.scale(),
            z_index: 10.0,
        }));

        ret
    }

    fn background_color(_state: &GameState) -> bevy::color::Srgba {
        GRAY.with_luminance(0.06)
    }
}

fn discrete_points_in_bounds(aabb: &AABB) -> Vec<IVec2> {
    let lower = vround(aabb.lower());
    let upper = vround(aabb.upper());
    let mut ret = Vec::new();
    for x in lower.x..=upper.x {
        for y in lower.y..=upper.y {
            let v = IVec2::new(x, y);
            ret.push(v);
        }
    }
    ret
}

fn marching_squares(
    p: (IVec2, bool),
    a: (IVec2, bool),
    b: (IVec2, bool),
    c: (IVec2, bool),
) -> Option<Vec<Vec2>> {
    let lerp4 = |w: IVec2, x: IVec2, y: IVec2, z: IVec2| {
        Some(vec![
            w.as_vec2().lerp(x.as_vec2(), 0.5),
            y.as_vec2().lerp(z.as_vec2(), 0.5),
        ])
    };

    let (p, pb) = p;
    let (a, ab) = a;
    let (b, bb) = b;
    let (c, cb) = c;

    let (ab, bb, cb) = if !pb { (ab, bb, cb) } else { (!ab, !bb, !cb) };

    // pb is implicitly false
    match (ab, bb, cb) {
        (false, false, false) => None,
        (true, false, false) => lerp4(p, a, a, b),
        (false, true, false) => lerp4(a, b, b, c),
        (true, false, true) => {
            if pb {
                let mut x = lerp4(p, a, a, b).unwrap();
                x.push(Vec2::NAN);
                x.extend(lerp4(p, c, c, b).unwrap());
                Some(x)
            } else {
                let mut x = lerp4(p, a, p, c).unwrap();
                x.push(Vec2::NAN);
                x.extend(lerp4(a, b, c, b).unwrap());
                Some(x)
            }
        }
        (true, true, false) => lerp4(p, a, b, c),
        (false, false, true) => lerp4(b, c, c, p),
        (true, true, true) => lerp4(p, a, p, c),
        (false, true, true) => lerp4(a, b, p, c),
    }
}

impl EditorContext {
    pub fn new() -> Self {
        let mut x: EditorContext = EditorContext {
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 20.0,
            target_scale: 18.0,
            aabbs: Vec::new(),
            points: HashSet::new(),
            lines: Vec::new(),
            parts: Vec::new(),
        };

        x.update();

        x
    }

    pub fn cursor_box(&self, input: &InputState) -> Option<AABB> {
        let p1 = input.position(MouseButt::Left, FrameId::Down)?;
        let p2 = input.position(MouseButt::Left, FrameId::Current)?;
        Some(AABB::from_arbitrary(
            vround(self.c2w(p1)).as_vec2(),
            vround(self.c2w(p2)).as_vec2(),
        ))
    }

    pub fn aabbs(&self) -> impl Iterator<Item = &AABB> {
        self.aabbs.iter()
    }

    pub fn lines(&self) -> impl Iterator<Item = &Vec<Vec2>> {
        self.lines.iter()
    }

    fn update(&mut self) {
        self.points.clear();
        for aabb in &self.aabbs {
            let pts = discrete_points_in_bounds(aabb);
            self.points.extend(pts);
        }

        self.lines.clear();
        for x in -100..=100 {
            for y in -100..=100 {
                let p = IVec2::new(x, y);
                let a = IVec2::new(x + 1, y);
                let b = IVec2::new(x + 1, y + 1);
                let c = IVec2::new(x, y + 1);
                let pb = self.points.contains(&p);
                let ab = self.points.contains(&a);
                let bb = self.points.contains(&b);
                let cb = self.points.contains(&c);
                if let Some(line) = marching_squares((p, pb), (a, ab), (b, bb), (c, cb)) {
                    self.lines.push(line);
                }
            }
        }
    }
}

impl CameraProjection for EditorContext {
    fn origin(&self) -> Vec2 {
        self.center
    }

    fn scale(&self) -> f32 {
        self.scale
    }
}

impl Interactive for EditorContext {
    fn step(&mut self, input: &InputState, dt: f32) {
        let speed = 16.0 * dt * 100.0;

        if input.is_pressed(KeyCode::KeyC) {
            self.aabbs.clear();
            self.update();
        }
        if input.just_pressed(KeyCode::KeyQ) {
            if let Some(c) = input.position(MouseButt::Hover, FrameId::Current) {
                let p = vround(self.c2w(c));
                if !self.parts.contains(&p) {
                    self.parts.push(p);
                }
            }
        }

        if let Some(c) = input.position(MouseButt::Right, FrameId::Current) {
            let c = vround(self.c2w(c));
            self.parts.retain(|p| *p != c);
        }

        if input.is_scroll_down() {
            self.target_scale /= 1.5;
        }
        if input.is_scroll_up() {
            self.target_scale *= 1.5;
        }

        if input.is_pressed(KeyCode::Equal) {
            self.target_scale *= 1.03;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.target_scale /= 1.03;
        }
        if input.is_pressed(KeyCode::KeyD) {
            self.target_center.x += speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_center.x -= speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_center.y += speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_center.y -= speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_center = Vec2::ZERO;
            self.target_scale = 1.0;
        }

        self.scale += (self.target_scale - self.scale) * 0.1;
        self.center += (self.target_center - self.center) * 0.1;
    }
}
