use crate::camera_controller::*;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::graph::Graph;
use crate::input::InputState;
use crate::input::{FrameId, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::{Render, TextLabel};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct TelescopeContext {
    camera: LinearCameraController,
}

impl CameraProjection for TelescopeContext {
    fn origin(&self) -> DVec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f64 {
        self.camera.scale()
    }

    fn offset(&self) -> DVec2 {
        self.camera.offset()
    }

    fn parent(&self) -> EntityId {
        self.camera.parent()
    }
}

pub fn to_azel(p: Vec3) -> (f64, f64) {
    let p = p.as_dvec3();
    let az = f64::atan2(p.y, p.x);
    let el = f64::atan2(p.z, p.xy().length());
    (az, el)
}

impl TelescopeContext {
    pub fn new() -> Self {
        TelescopeContext {
            camera: LinearCameraController::new(DVec2::ZERO, 1.1, 0.3),
        }
    }

    pub fn azimuth(&self) -> f64 {
        self.camera.origin().x
    }

    pub fn elevation(&self) -> f64 {
        self.camera.origin().y
    }

    pub fn on_game_tick(&mut self) {
        self.camera.on_game_tick();
    }

    pub fn on_render_tick(&mut self, input: &InputState) {
        self.camera.handle_input(input);
    }

    pub fn screen_radius(state: &GameState) -> f32 {
        state.input.screen_bounds.span.min_element() / 2.0 * 1.1
    }

    pub fn screen_position(az: f64, el: f64, state: &GameState) -> (Vec2, f32, f64) {
        let screen_radius = Self::screen_radius(state);

        let azel = state.telescope_context.origin();
        let daz = az - azel.x;
        let del = el - azel.y;

        // assumes x is on the domain [0, 1].
        // moves x towards 1, but doesn't move 0
        let scale = |x: f64| -> f64 {
            let xmag = x.abs();
            (1.0 - (1.0 - xmag).powf(3.0)) * x.signum()
        };

        let daz = wrap_pi_npi_f64(daz);
        let del = wrap_pi_npi_f64(del * 2.0) / 2.0;

        let angular_offset = DVec2::new(daz, del);
        let angular_distance = angular_offset.length();

        let scaled_distance = scale((angular_distance * state.telescope_context.scale()).min(1.0));

        let alpha = 1.0 - scaled_distance.powi(3);

        (
            graphics_cast(angular_offset.normalize_or_zero() * scaled_distance) * screen_radius,
            gcast(alpha),
            angular_distance,
        )
    }
}

fn get_frequency_spectrum(x: f32, d: f32, fc: f32) -> f32 {
    let rsq = (d * -20.0).exp();
    let blackbody = 0.7 / (x / 250.0);
    let noise = rand(0.0, 0.01);
    let emissions = 0.5 * (1.0 / (1.0 + ((x - fc) / 100.0).powi(2)));
    rsq * (blackbody + noise + emissions)
}

impl Render for TelescopeContext {
    fn background_color(_state: &GameState) -> Srgba {
        GRAY.with_luminance(0.12)
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(crate::ui::basic_scenes_layout(state))
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let screen_radius = TelescopeContext::screen_radius(state);
        draw_circle(&mut canvas.gizmos, Vec2::ZERO, screen_radius, WHITE);
        draw_circle(&mut canvas.gizmos, Vec2::ZERO, screen_radius + 5.0, WHITE);

        draw_cross(&mut canvas.gizmos, Vec2::ZERO, 5.0, GRAY);

        let mut graph = Graph::linspace(250.0, 2500.0, 100);

        graph.add_point(250.0, 0.0, true);
        graph.add_point(250.0, 1.0, true);
        graph.add_point(2500.0, 0.0, true);

        for (star, color, radius, fc) in &state.starfield {
            let (az, el) = to_azel(*star);
            let (p, alpha, d) = TelescopeContext::screen_position(az, el, state);
            if d < 0.2 {
                graph.add_func(
                    |x: f64| get_frequency_spectrum(x as f32, d as f32, *fc as f32) as f64,
                    color.with_alpha(0.3),
                );
            }
            draw_circle(&mut canvas.gizmos, p, *radius, color.with_alpha(alpha));
        }

        draw_graph(
            canvas,
            &graph,
            state.input.screen_bounds.with_center(Vec2::ZERO),
            Some(&state.input),
        );

        let cursor = state.input.position(MouseButt::Hover, FrameId::Current)?;

        for (p, _, _, freq) in &state.starfield {
            let (az, el) = to_azel(*p);
            let (q, alpha, _) = Self::screen_position(az, el, state);
            if alpha > 0.4 && q.distance(cursor) < 50.0 {
                canvas.label(TextLabel::new(
                    format!(
                        "AZEL {:0.0}/{:0.0}\n{:0.1} LYR\n{:0.1} K",
                        az.to_degrees(),
                        el.to_degrees(),
                        p.length() / 600.0,
                        freq
                    ),
                    q + 30.0 * Vec2::Y,
                    0.7,
                ));
            }
        }

        Some(())
    }
}
