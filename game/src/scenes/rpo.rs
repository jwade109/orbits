use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::InputState;
use crate::onclick::OnClick;
use crate::scenes::Render;
use crate::scenes::{CameraProjection, Interactive, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug)]
pub struct RPOContext {
    camera: LinearCameraController,
    following: Option<usize>,
}

impl CameraProjection for RPOContext {
    fn origin(&self) -> Vec2 {
        self.camera.center
    }

    fn scale(&self) -> f32 {
        self.camera.scale
    }
}

fn relative_info_labels(state: &GameState) -> Option<TextLabel> {
    let target = state.targeting()?;
    let ownship = state.piloting()?;
    let pvt = state.lup_orbiter(target, state.sim_time)?.pv();
    let pvo = state.lup_orbiter(ownship, state.sim_time)?.pv();
    let relpos = pvo - pvt;

    let str = format!(
        "Separation {:0.1} m / Velocity {:0.1} m/s",
        relpos.pos.length() * 1000.0,
        relpos.vel.length() * 1000.0
    );

    let dims = state.input.screen_bounds.span;

    Some(TextLabel::new(str, Vec2::Y * (dims.y / 2.0 - 100.0), 1.0))
}

impl Render for RPOContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.05)
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.rpo_context;
        let target = state.targeting()?;
        let piloting = state.piloting()?;

        draw_piloting_overlay(&mut canvas.gizmos, state);

        let origin = state.lup_orbiter(target, state.sim_time)?.pv();

        draw_circle(&mut canvas.gizmos, ctx.w2c(Vec2::ZERO), 7.0, TEAL);

        for km in 1..=5 {
            let km = km as f32;
            let alpha = 0.8 - 0.14 * km as f32;
            draw_circle(
                &mut canvas.gizmos,
                ctx.w2c(Vec2::ZERO),
                km * 1000.0 * ctx.scale(),
                GRAY.with_alpha(alpha),
            );
        }

        for meters in (10..=90).step_by(10).chain((100..=900).step_by(100)) {
            let alpha = 0.2;
            draw_circle(
                &mut canvas.gizmos,
                ctx.w2c(Vec2::ZERO),
                meters as f32 * ctx.scale(),
                GRAY.with_alpha(alpha),
            );
        }

        for (id, _) in &state.orbiters {
            let lup = match state.lup_orbiter(*id, state.sim_time) {
                Some(lup) => lup,
                None => continue,
            };

            let pv = (lup.pv() - origin) * 1000.0f32;

            if pv.pos.length() > 10000.0 {
                continue;
            }

            if *id != target {
                draw_circle(&mut canvas.gizmos, ctx.w2c(pv.pos_f32()), 7.0, RED);
            }

            if let Some(v) = state.vehicles.get(&id) {
                draw_vehicle(
                    &mut canvas.gizmos,
                    v,
                    ctx.w2c(pv.pos_f32()),
                    ctx.scale(),
                    v.angle(),
                );
            }
        }

        {
            // TODO this is terrible
            let po = state.get_orbit(piloting)?;
            let to = state.get_orbit(target)?;

            let (_, _, mut relpos) = make_separation_graph(&po.1, &to.1, state.sim_time);
            relpos.iter_mut().for_each(|p| *p = ctx.w2c(*p * 1000.0));
            canvas.gizmos.linestrip_2d(relpos, WHITE);
        }

        let half_span = state.input.screen_bounds.span / 2.0;

        if let Some(info) = relative_info_labels(state) {
            canvas.label(info);
        }

        canvas.label(TextLabel::new(
            format!(
                "Target: {:?} scale: {}",
                state.orbital_context.targeting,
                state.rpo_context.scale()
            ),
            (40.0 - half_span.y) * Vec2::Y,
            0.6,
        ));

        Some(())
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
            camera: LinearCameraController {
                center: Vec2::ZERO,
                target_center: Vec2::ZERO,
                scale: 1.0,
                target_scale: 1.0,
            },
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

#[derive(Debug)]
pub struct LinearCameraController {
    pub center: Vec2,
    pub target_center: Vec2,
    pub scale: f32,
    pub target_scale: f32,
}

impl LinearCameraController {
    pub fn update(&mut self, dt: f32, input: &InputState) {
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

impl Interactive for RPOContext {
    fn step(&mut self, input: &InputState, dt: f32) {
        self.camera.update(dt, input);
    }
}
