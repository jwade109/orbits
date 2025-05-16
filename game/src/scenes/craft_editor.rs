use crate::drawing::*;
use crate::mouse::InputState;
use crate::mouse::{FrameId, MouseButt};
use crate::parts::{part_sprite_path, PartProto};
use crate::planetary::GameState;
use crate::scenes::{CameraProjection, Render, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use enum_iterator::{next_cycle, Sequence};
use starling::prelude::*;

#[derive(Debug, Clone, Copy, Sequence)]
pub enum Rotation {
    East,
    North,
    West,
    South,
}

impl Rotation {
    fn to_angle(&self) -> f32 {
        match self {
            Self::East => 0.0,
            Self::North => PI * 0.5,
            Self::West => PI,
            Self::South => PI * 1.5,
        }
    }
}

#[derive(Debug)]
pub struct EditorContext {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
    parts: Vec<(IVec2, Rotation, PartProto)>,
    pub current_part_index: usize,
    rotation: Rotation,
}

impl EditorContext {
    pub fn new() -> Self {
        EditorContext {
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 20.0,
            target_scale: 18.0,
            parts: Vec::new(),
            current_part_index: 2,
            rotation: Rotation::East,
        }
    }

    pub fn cursor_box(&self, input: &InputState) -> Option<AABB> {
        let p1 = input.position(MouseButt::Left, FrameId::Down)?;
        let p2 = input.position(MouseButt::Left, FrameId::Current)?;
        Some(AABB::from_arbitrary(
            vround(self.c2w(p1)).as_vec2(),
            vround(self.c2w(p2)).as_vec2(),
        ))
    }

    fn occupied_pixels(pos: IVec2, rot: Rotation, part: &PartProto) -> Vec<IVec2> {
        let w2 = part.width / 2;
        let h2 = part.height / 2;
        let offset = match rot {
            Rotation::East | Rotation::West => UVec2::new(w2, h2),
            Rotation::North | Rotation::South => UVec2::new(h2, w2),
        }
        .as_ivec2();
        let mut ret = vec![];
        for w in 0..part.width {
            for h in 0..part.height {
                let wh = match rot {
                    Rotation::East | Rotation::West => UVec2::new(w, h),
                    Rotation::North | Rotation::South => UVec2::new(h, w),
                };
                let p = pos + wh.as_ivec2() - offset;
                ret.push(p);
            }
        }
        ret
    }

    fn current_part(&self) -> Option<PartProto> {
        crate::parts::ALL_PARTS
            .get(self.current_part_index)
            .cloned()
            .cloned()
    }

    fn try_place_part(&mut self, p: IVec2, new_part: PartProto) -> Option<()> {
        let new_pixels = Self::occupied_pixels(p, self.rotation, &new_part);
        for (pos, rot, part) in &self.parts {
            if part.layer != new_part.layer {
                continue;
            }
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            for q in pixels {
                if new_pixels.contains(&q) {
                    return None;
                }
            }
        }
        self.parts.push((p, self.rotation, new_part));
        Some(())
    }

    fn remove_part_at(&mut self, p: IVec2) {
        self.parts.retain(|(pos, rot, part)| {
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            !pixels.contains(&p)
        });
    }

    fn current_part_and_cursor_position(state: &GameState) -> Option<(IVec2, PartProto)> {
        let part = state.editor_context.current_part()?;
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = vround(state.editor_context.c2w(pos));
        Some((pos, part))
    }
}

impl Render for EditorContext {
    fn text_labels(state: &GameState) -> Vec<TextLabel> {
        vec![TextLabel {
            text: format!(
                "{} parts / {:?}",
                state.editor_context.parts.len(),
                state.editor_context.rotation,
            ),
            position: Vec2::new(0.0, 30.0 - state.input.screen_bounds.span.y * 0.5),
            size: 0.7,
        }]
    }

    fn sprites(state: &GameState) -> Vec<StaticSpriteDescriptor> {
        let ctx = &state.editor_context;
        let mut ret = Vec::new();

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            ret.push(StaticSpriteDescriptor {
                position: ctx.w2c(p.as_vec2()),
                angle: ctx.rotation.to_angle(),
                path: part_sprite_path(current_part.path),
                scale: ctx.scale(),
                z_index: 12.0,
            });

            let current_pixels = Self::occupied_pixels(p, ctx.rotation, &current_part);

            for (pos, rot, part) in &ctx.parts {
                if part.layer != current_part.layer {
                    continue;
                }
                let pixels = Self::occupied_pixels(*pos, *rot, part);
                for q in pixels {
                    if current_pixels.contains(&q) {
                        ret.push(StaticSpriteDescriptor {
                            position: ctx.w2c(q.as_vec2() + Vec2::ONE / 2.0),
                            angle: rot.to_angle(),
                            path: "embedded://game/../assets/collision_pixel.png".into(),
                            scale: ctx.scale(),
                            z_index: part.to_z_index(),
                        });
                    }
                }
            }
        }

        ret.extend(ctx.parts.iter().enumerate().map(|(i, (pos, rot, part))| {
            StaticSpriteDescriptor {
                position: ctx.w2c(pos.as_vec2()),
                angle: rot.to_angle(),
                path: part_sprite_path(part.path),
                scale: ctx.scale(),
                z_index: part.to_z_index(),
            }
        }));

        ret
    }

    fn background_color(_state: &GameState) -> bevy::color::Srgba {
        GRAY.with_luminance(0.06)
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        let ctx = &state.editor_context;
        draw_cross(gizmos, ctx.w2c(Vec2::ZERO), 10.0, GRAY);

        let cursor = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let c = ctx.c2w(cursor);
        let discrete = IVec2::new(
            (c.x / 8.0).round() as i32 * 8,
            (c.y / 8.0).round() as i32 * 8,
        );

        if let Some((p, _)) = Self::current_part_and_cursor_position(state) {
            draw_cross(gizmos, ctx.w2c(p.as_vec2()), 0.8 * ctx.scale(), TEAL);
        }

        for (p, _, _) in &ctx.parts {
            draw_cross(gizmos, ctx.w2c(p.as_vec2()), 0.5 * ctx.scale(), RED);
        }

        for dx in (-80..=80).step_by(8) {
            for dy in (-80..=80).step_by(8) {
                let s = IVec2::new(dx, dy);
                let p = discrete - s;
                let d = (s.length_squared() as f32).sqrt();
                let alpha = 0.2 * (1.0 - d / 80.0);
                draw_diamond(gizmos, ctx.w2c(p.as_vec2()), 7.0, GRAY.with_alpha(alpha));
            }
        }

        Some(())
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

impl EditorContext {
    pub fn step(&mut self, input: &InputState, dt: f32, is_hovering_over_ui: bool) {
        let speed = 16.0 * dt * 100.0;

        if !is_hovering_over_ui {
            if let Some(p) = input.position(MouseButt::Left, FrameId::Current) {
                let p = vround(self.c2w(p));
                if let Some(part) = self.current_part() {
                    self.try_place_part(p, part);
                }
            }

            if let Some(p) = input.position(MouseButt::Right, FrameId::Current) {
                let p = vround(self.c2w(p));
                self.remove_part_at(p);
            }
        }

        if input.is_scroll_down() {
            self.target_scale /= 1.5;
        }
        if input.is_scroll_up() {
            self.target_scale *= 1.5;
        }

        if input.just_pressed(KeyCode::KeyR) {
            self.rotation = next_cycle(&self.rotation);
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

        self.scale += (self.target_scale - self.scale) * 0.1;
        self.center += (self.target_center - self.center) * 0.1;
    }
}
