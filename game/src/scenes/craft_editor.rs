use crate::mouse::InputState;
use crate::mouse::{FrameId, MouseButt};
use crate::scenes::{CameraProjection, Interactive};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::Srgba;
use enum_iterator::{all, next_cycle, Sequence};
use starling::math::{IVec2, Vec2};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Sequence)]
pub enum EditorColor {
    Orange,
    Teal,
    Red,
    White,
}

impl EditorColor {
    pub fn to_color(&self) -> Srgba {
        match self {
            Self::Orange => ORANGE,
            Self::Teal => TEAL,
            Self::Red => RED,
            Self::White => WHITE,
        }
    }
}

pub struct EditorContext {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,

    points: HashMap<IVec2, EditorColor>,
    lines: HashMap<EditorColor, Vec<Vec<Vec2>>>,
    color: EditorColor,
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
            points: HashMap::new(),
            lines: HashMap::new(),
            color: EditorColor::Orange,
        };

        x.update();

        x
    }

    pub fn color(&self) -> EditorColor {
        self.color
    }

    pub fn cursor_size(&self) -> i32 {
        ((20.0 / self.scale).round() as i32).max(0)
    }

    pub fn paintbrush(&self, camera_pos: Vec2) -> HashSet<IVec2> {
        let w = self.c2w(camera_pos);
        let i = IVec2::new(w.x.round() as i32, w.y.round() as i32);
        let mut ret = HashSet::new();
        for dx in -self.cursor_size()..=self.cursor_size() {
            for dy in -self.cursor_size()..=self.cursor_size() {
                let d = IVec2::new(dx, dy);
                ret.insert(i + d);
            }
        }
        ret
    }

    pub fn points(&self) -> impl Iterator<Item = (&IVec2, &EditorColor)> {
        self.points.iter()
    }

    pub fn lines(&self, color: EditorColor) -> impl Iterator<Item = &Vec<Vec2>> {
        match self.lines.get(&color) {
            Some(x) => x.iter(),
            None => [].iter(),
        }
    }

    fn update(&mut self) {
        self.lines.clear();
        for color in all::<EditorColor>() {
            let mut lines = Vec::new();
            for x in -100..=100 {
                for y in -100..=100 {
                    let p = IVec2::new(x, y);
                    let a = IVec2::new(x + 1, y);
                    let b = IVec2::new(x + 1, y + 1);
                    let c = IVec2::new(x, y + 1);
                    let pb = self.points.get(&p) == Some(&color);
                    let ab = self.points.get(&a) == Some(&color);
                    let bb = self.points.get(&b) == Some(&color);
                    let cb = self.points.get(&c) == Some(&color);
                    if let Some(line) = marching_squares((p, pb), (a, ab), (b, bb), (c, cb)) {
                        lines.push(line);
                    }
                }
            }
            self.lines.insert(color, lines);
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
            self.points.clear();
            self.update();
        }

        if input.just_pressed(KeyCode::KeyP) {
            self.color = next_cycle(&self.color);
        }

        if let Some(c) = input.position(MouseButt::Left, FrameId::Current) {
            let pb = self.paintbrush(c);
            for v in pb {
                self.points.insert(v, self.color);
            }
            self.update();
        } else if let Some(c) = input.position(MouseButt::Right, FrameId::Current) {
            let pb = self.paintbrush(c);
            for v in pb {
                self.points.remove(&v);
            }
            self.update();
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
