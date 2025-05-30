use crate::drawing::*;
use crate::mouse::InputState;
use crate::onclick::OnClick;
use crate::planetary::GameState;
use crate::scenes::Render;
use crate::scenes::{CameraProjection, Interactive, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use layout::layout::Tree;
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

        draw_piloting_overlay(gizmos, state);

        let origin = state.scenario.lup_orbiter(target, state.sim_time)?.pv();

        draw_circle(gizmos, ctx.w2c(Vec2::ZERO), 7.0, TEAL);

        for km in 1..=5 {
            let km = km as f32;
            let alpha = 0.8 - 0.14 * km as f32;
            draw_circle(
                gizmos,
                ctx.w2c(Vec2::ZERO),
                km * 1000.0 * ctx.scale(),
                GRAY.with_alpha(alpha),
            );
        }

        for meters in (10..=90).step_by(10).chain((100..=900).step_by(100)) {
            let alpha = 0.2;
            draw_circle(
                gizmos,
                ctx.w2c(Vec2::ZERO),
                meters as f32 * ctx.scale(),
                GRAY.with_alpha(alpha),
            );
        }

        for id in [target, piloting] {
            let lup = match state.scenario.lup_orbiter(id, state.sim_time) {
                Some(lup) => lup,
                None => continue,
            };

            let pv = (lup.pv() - origin) * 1000.0f32;

            if id == piloting {
                draw_circle(gizmos, ctx.w2c(pv.pos_f32()), 7.0, RED);
            }

            if let Some(v) = state.orbital_vehicles.get(&id) {
                draw_vehicle(gizmos, v, ctx.w2c(pv.pos_f32()), ctx.scale());
            }
        }

        {
            // TODO this is terrible
            let po = state.get_orbit(piloting)?;
            let to = state.get_orbit(target)?;

            let (_, _, mut relpos) = make_separation_graph(&po.1, &to.1, state.sim_time);
            relpos.iter_mut().for_each(|p| *p = ctx.w2c(*p * 1000.0));
            gizmos.linestrip_2d(relpos, WHITE);
        }

        Some(())
    }

    fn sprites(_state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        None
        // let ctx = &state.rpo_context;

        // let mut ret = vec![];

        // let targeting = state.targeting()?;
        // let piloting = state.piloting()?;

        // let p1 = state.scenario.lup_orbiter(targeting, state.sim_time)?.pv();
        // let p2 = state.scenario.lup_orbiter(piloting, state.sim_time)?.pv();

        // for (id, offset) in [
        //     (targeting, DVec2::ZERO),
        //     (piloting, (p2.pos - p1.pos) * 1000.0),
        // ] {
        //     let vehicle = state.orbital_vehicles.get(&id)?;

        //     for (pos, rot, part) in &vehicle.parts {
        //         let path = crate::scenes::craft_editor::part_sprite_path(&state.args, &part.path);
        //         let dims = meters_with_rotation(*rot, part);
        //         let p = pos.as_vec2() / starling::parts::parts::PIXELS_PER_METER;
        //         let angle = rot.to_angle() + vehicle.angle() - PI / 2.0;
        //         let center = rotate(p + dims / 2.0, vehicle.angle() - PI / 2.0);
        //         let scale = ctx.scale() / PIXELS_PER_METER;
        //         let position = ctx.w2c(offset.as_vec2() + center);
        //         let z_index = match part.data.layer {
        //             PartLayer::Exterior => 12.0,
        //             PartLayer::Internal => 9.0,
        //             PartLayer::Structural => 11.0,
        //         };
        //         let desc = StaticSpriteDescriptor::new(position, angle, path, scale, z_index);
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

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        use crate::ui::*;
        use layout::layout::Node;

        let dims = state.input.screen_bounds.span;

        let bar = top_bar(state);
        let sim_time = sim_time_toolbar(state);
        let throttle = throttle_controls(state);
        let thruster = crate::scenes::orbital::thruster_control_dialogue(state);

        let sidebar = Node::column(300).with_color(UI_BACKGROUND_COLOR);

        let mut world_viewport = Node::grow()
            .invisible()
            .down()
            .with_child(sim_time)
            .with_child(throttle);

        if let Some(t) = thruster {
            world_viewport.add_child(t);
        }

        let main_content = Node::grow()
            .invisible()
            .tight()
            .with_child(sidebar)
            .with_child(world_viewport);

        let wrapper = Node::new(dims.x, dims.y)
            .down()
            .tight()
            .invisible()
            .with_child(bar)
            .with_child(main_content);

        Some(Tree::new().with_layout(wrapper, Vec2::ZERO))
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
