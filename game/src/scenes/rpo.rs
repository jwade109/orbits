use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::InputState;
use crate::onclick::OnClick;
use crate::scenes::Render;
use crate::scenes::{CameraProjection, TextLabel};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use layout::layout::Tree;

#[derive(Debug)]
pub struct DockingContext {
    camera: LinearCameraController,
    following: Option<usize>,
}

impl CameraProjection for DockingContext {
    fn origin(&self) -> Vec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f32 {
        self.camera.scale()
    }
}

fn relative_info_labels(state: &GameState) -> Option<TextLabel> {
    let target = state.targeting()?;
    let ownship = state.piloting()?;
    let pvt = state
        .universe
        .lup_orbiter(target, state.universe.stamp())?
        .pv();
    let pvo = state
        .universe
        .lup_orbiter(ownship, state.universe.stamp())?
        .pv();
    let relpos = pvo - pvt;

    let str = format!(
        "Separation {:0.1} m / Velocity {:0.1} m/s",
        relpos.pos.length() * 1000.0,
        relpos.vel.length() * 1000.0
    );

    let dims = state.input.screen_bounds.span;

    Some(TextLabel::new(str, Vec2::Y * (dims.y / 2.0 - 100.0), 1.0))
}

impl Render for DockingContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.05)
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.docking_context;
        let target = state.targeting()?;
        let piloting = state.piloting()?;

        draw_piloting_overlay(canvas, state);

        let origin = state
            .universe
            .lup_orbiter(target, state.universe.stamp())?
            .pv();

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

        for (id, (_, _, vehicle)) in &state.universe.orbital_vehicles {
            let lup = match state.universe.lup_orbiter(*id, state.universe.stamp()) {
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

            draw_vehicle(canvas, vehicle, ctx.w2c(pv.pos_f32()), ctx.scale(), 0.0);
        }

        {
            // TODO this is terrible
            let po = state.get_orbit(piloting)?;
            let to = state.get_orbit(target)?;

            let (_, _, mut relpos) = make_separation_graph(&po.1, &to.1, state.universe.stamp());
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
                state.docking_context.scale()
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

        let sidebar = Node::column(300).with_color(UI_BACKGROUND_COLOR);

        let world_viewport = Node::grow()
            .invisible()
            .down()
            .with_child(sim_time)
            .with_child(throttle);

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

impl DockingContext {
    pub fn new() -> Self {
        Self {
            camera: LinearCameraController::new(Vec2::ZERO, 1.0, 1100.0),
            following: None,
        }
    }

    pub fn following(&self) -> Option<usize> {
        self.following
    }

    pub fn on_game_tick(&mut self) {
        self.camera.on_game_tick();
    }

    pub fn handle_input(&mut self, input: &InputState) {
        self.camera.handle_input(input);
    }
}
