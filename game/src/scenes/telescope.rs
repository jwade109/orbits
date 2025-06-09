use crate::drawing::*;
use crate::game::GameState;
use crate::graph::Graph;
use crate::input::InputState;
use crate::input::{FrameId, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct TelescopeContext {
    azimuth: f32,
    elevation: f32,
    angular_radius: f32,
    target_az: f32,
    target_el: f32,
    target_angular_radius: f32,
}

impl CameraProjection for TelescopeContext {
    fn origin(&self) -> Vec2 {
        Vec2::new(self.azimuth, self.elevation)
    }

    fn scale(&self) -> f32 {
        1.0 / self.angular_radius
    }
}

impl TelescopeContext {
    pub fn new() -> Self {
        TelescopeContext {
            azimuth: 0.0,
            elevation: 0.0,
            target_az: 0.0,
            target_el: 0.0,
            angular_radius: 1.1,
            target_angular_radius: 1.0,
        }
    }

    pub fn step(&mut self, input: &InputState, dt: f32) {
        if input.is_scroll_down() {
            self.target_angular_radius *= 1.5;
        }
        if input.is_scroll_up() {
            self.target_angular_radius /= 1.5;
        }

        if input.is_pressed(KeyCode::Equal) {
            self.target_angular_radius *= 0.96;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.target_angular_radius /= 0.96;
        }

        let angular_speed = 0.004 * dt * 100.0;

        if input.is_pressed(KeyCode::KeyD) {
            self.target_az += angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_az -= angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_el += angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_el -= angular_speed * self.angular_radius;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_el = 0.0;
            self.target_az = 0.0;
            self.target_angular_radius = 1.0;
        }

        self.target_angular_radius = self.target_angular_radius.clamp(0.05, PI / 2.0);

        self.angular_radius += (self.target_angular_radius - self.angular_radius) * 0.03;
        self.azimuth += (self.target_az - self.azimuth) * 0.03;
        self.elevation += (self.target_el - self.elevation) * 0.03;
    }

    pub fn to_azel(p: Vec3) -> (f32, f32) {
        let az = f32::atan2(p.y, p.x);
        let el = f32::atan2(p.z, p.xy().length());
        (az, el)
    }

    pub fn screen_radius(state: &GameState) -> f32 {
        state.input.screen_bounds.span.min_element() / 2.0 * 1.1
    }

    pub fn screen_position(az: f32, el: f32, state: &GameState) -> (Vec2, f32, f32) {
        let screen_radius = Self::screen_radius(state);
        let map = |az: f32, el: f32| -> (Vec2, f32, f32) {
            let azel = state.telescope_context.origin();
            let daz = az - azel.x;
            let del = el - azel.y;

            // assumes x is on the domain [0, 1].
            // moves x towards 1, but doesn't move 0
            let scale = |x: f32| -> f32 {
                let xmag = x.abs();
                (1.0 - (1.0 - xmag).powf(3.0)) * x.signum()
            };

            let daz = wrap_pi_npi(daz);
            let del = wrap_pi_npi(del * 2.0) / 2.0;

            let angular_offset = Vec2::new(daz, del);
            let angular_distance = angular_offset.length();

            let scaled_distance =
                scale((angular_distance * state.telescope_context.scale()).min(1.0));

            let alpha = 1.0 - scaled_distance.powi(3);

            (
                angular_offset.normalize_or_zero() * scaled_distance * screen_radius,
                alpha,
                angular_distance,
            )
        };

        map(az, el)
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
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let cursor = state.input.position(MouseButt::Hover, FrameId::Current)?;

        let mut ret = Vec::new();

        for (p, _, _, freq) in &state.starfield {
            let (az, el) = Self::to_azel(*p);
            let (q, alpha, _) = Self::screen_position(az, el, state);
            if alpha > 0.4 && q.distance(cursor) < 50.0 {
                ret.push(TextLabel::new(
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

        Some(ret)
    }

    fn sprites(_state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        None
    }

    fn background_color(_state: &GameState) -> Srgba {
        GRAY.with_luminance(0.12)
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        let screen_radius = TelescopeContext::screen_radius(state);
        draw_circle(gizmos, Vec2::ZERO, screen_radius, WHITE);
        draw_circle(gizmos, Vec2::ZERO, screen_radius + 5.0, WHITE);

        draw_cross(gizmos, Vec2::ZERO, 5.0, GRAY);

        let mut graph = Graph::linspace(250.0, 2500.0, 100);

        graph.add_point(250.0, 0.0, true);
        graph.add_point(250.0, 1.0, true);
        graph.add_point(2500.0, 0.0, true);

        for (star, color, radius, fc) in &state.starfield {
            let (az, el) = TelescopeContext::to_azel(*star);
            let (p, alpha, d) = TelescopeContext::screen_position(az, el, state);
            if d < 0.2 {
                graph.add_func(
                    |x: f32| get_frequency_spectrum(x, d, *fc),
                    color.with_alpha(0.3),
                );
            }
            draw_circle(gizmos, p, *radius, color.with_alpha(alpha));
        }

        draw_graph(
            gizmos,
            &graph,
            state.input.screen_bounds.with_center(Vec2::ZERO),
        );

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(crate::ui::basic_scenes_layout(state))
    }
}
