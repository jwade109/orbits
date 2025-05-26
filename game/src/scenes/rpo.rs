use crate::drawing::*;
use crate::mouse::InputState;
use crate::planetary::GameState;
use crate::scenes::craft_editor::part_sprite_path;
use crate::scenes::Render;
use crate::scenes::{CameraProjection, Interactive, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use starling::prelude::*;

#[derive(Debug)]
pub struct RPOContext {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
    following: Option<usize>,
}

impl CameraProjection for RPOContext {
    fn origin(&self) -> Vec2 {
        self.center
    }

    fn scale(&self) -> f32 {
        self.scale
    }
}

impl Render for RPOContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.05)
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        let ctx = &state.rpo_context;
        let target = state.targeting()?;
        let piloting = state.piloting()?;

        let origin = state.scenario.lup_orbiter(target, state.sim_time)?.pv();

        draw_circle(gizmos, Vec2::ZERO, 4.0, GRAY);
        draw_circle(gizmos, ctx.w2c(Vec2::ZERO), 4.0, TEAL);
        draw_circle(gizmos, ctx.w2c(Vec2::ZERO), 1000.0 * ctx.scale(), GRAY);
        draw_circle(
            gizmos,
            ctx.w2c(Vec2::ZERO),
            5000.0 * ctx.scale(),
            GRAY.with_alpha(0.4),
        );

        for id in [target, piloting] {
            let lup = match state.scenario.lup_orbiter(id, state.sim_time) {
                Some(lup) => lup,
                None => continue,
            };

            let pv = lup.pv() - origin;
            draw_circle(gizmos, ctx.w2c(pv.pos), 4.0, WHITE);

            if let Some(v) = state.orbital_vehicles.get(&id) {
                draw_vehicle(gizmos, v, ctx.w2c(pv.pos), ctx.scale() / 1000.0);
            }
        }

        Some(())
    }

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        None
        // let ctx = &state.rpo_context;

        // let mut ret = vec![];

        // for id in state.scenario.orbiter_ids() {
        //     let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
        //     let vehicle = match state.orbital_vehicles.get(&id) {
        //         Some(v) => v,
        //         None => continue,
        //     };

        //     for (_, _, part) in &vehicle.parts {
        //         let path = part_sprite_path(&state.args, &part.path);
        //         let desc = StaticSpriteDescriptor::new(
        //             ctx.w2c(lup.pv().pos),
        //             vehicle.angle(),
        //             path,
        //             ctx.scale(),
        //             10.0,
        //         );
        //         ret.push(desc);
        //     }
        // }

        // Some(ret)
    }

    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let half_span = state.input.screen_bounds.span / 2.0;
        Some(vec![TextLabel::new(
            format!(
                "Target: {:?} scale: {}",
                state.orbital_context.targeting,
                state.rpo_context.scale()
            ),
            (40.0 - half_span.y) * Vec2::Y,
            0.6,
        )])
    }

    fn ui(_state: &GameState) -> Option<layout::layout::Tree<crate::onclick::OnClick>> {
        todo!()
    }
}

impl RPOContext {
    pub fn new() -> Self {
        Self {
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 1.0,
            target_scale: 1.0,
            following: None,
        }
    }

    pub fn following(&self) -> Option<usize> {
        self.following
    }

    pub fn handle_follow(&mut self, input: &InputState, rpo: &RPO) -> Option<()> {
        let p = self.c2w(input.double_click()?);
        let idx = rpo.nearest(p)?;
        self.following = Some(idx);
        Some(())
    }
}

impl Interactive for RPOContext {
    fn step(&mut self, input: &InputState, dt: f32) {
        let speed = 16.0 * dt * 100.0;

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
