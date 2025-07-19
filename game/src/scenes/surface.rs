use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::*;
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render};
use crate::thrust_particles::*;
use bevy::color::{palettes::css::*, Alpha, Srgba};
use bevy::prelude::{Gizmos, KeyCode};
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct SurfaceContext {
    camera: LinearCameraController,
    selected: HashSet<usize>,
    particles: ThrustParticleEffects,
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController::new(Vec2::Y * 30.0, 10.0, 1700.0),
            selected: HashSet::new(),
            particles: ThrustParticleEffects::new(),
        }
    }
}

pub fn to_srgba(fl: [f32; 4]) -> Srgba {
    Srgba::new(fl[0], fl[1], fl[2], fl[3])
}

impl SurfaceContext {
    pub fn camera(&self) -> &LinearCameraController {
        &self.camera
    }

    pub fn mouseover_vehicle(
        universe: &Universe,
        pos: Vec2,
    ) -> Option<(usize, &RigidBody, &Vehicle)> {
        for (i, (body, _, vehicle)) in universe.surface_vehicles.iter().enumerate() {
            let d = body.pv.pos_f32().distance(pos);
            let r = vehicle.bounding_radius();
            if d < r {
                return Some((i, body, vehicle));
            }
        }
        None
    }

    pub fn on_render_tick(&mut self, input: &InputState, universe: &mut Universe) {
        self.camera.handle_input(input);

        (|| -> Option<()> {
            let (pos, double) = if let Some(p) = input.double_click() {
                (p, true)
            } else {
                (input.on_frame(MouseButt::Left, FrameId::Down)?, false)
            };

            let add = input.is_pressed(KeyCode::ShiftLeft);
            if !add {
                self.selected.clear();
            }

            let pos = self.c2w(pos);
            let (idx, _, _) = Self::mouseover_vehicle(universe, pos)?;
            self.selected.insert(idx);
            if double {
                // TODO fix this
                // ctx.follow_vehicle = true;
            }
            None
        })();

        (|| -> Option<()> {
            let rc = input.position(MouseButt::Right, FrameId::Down)?;
            let rd = input.position(MouseButt::Right, FrameId::Current)?;
            let p = self.c2w(rc);
            let q = self.c2w(rd);

            let sep = 15.0;
            let spread = sep * self.selected.len() as f32 - sep;
            let center = Vec2::new(spread / 2.0, 0.0);

            let angle = (q - p).to_angle();

            for (i, idx) in self.selected.iter().enumerate() {
                if let Some((_, policy, _)) = universe.surface_vehicles.get_mut(*idx) {
                    let pos = p + Vec2::X * 15.0 * i as f32 - center;
                    *policy = VehicleControlPolicy::PositionHold(pos, angle);
                }
            }

            None
        })();
    }

    pub fn on_game_tick(state: &mut GameState) {
        let ctx = &mut state.surface_context;

        ctx.camera.on_game_tick();

        ctx.particles.step();

        for (_, _, vehicle) in state.universe.surface_vehicles.iter_mut() {
            for (_, part) in vehicle.parts() {
                if let Some((t, d)) = part.as_thruster() {
                    if !d.is_thrusting(t) || t.is_rcs() {
                        continue;
                    }

                    ctx.particles.add(vehicle, part);
                }
            }
        }
    }
}

impl CameraProjection for SurfaceContext {
    fn origin(&self) -> Vec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f32 {
        self.camera.scale()
    }
}

fn draw_kinematic_arc(gizmos: &mut Gizmos, mut pv: PV, ctx: &impl CameraProjection, accel: Vec2) {
    let dt = 0.25;
    for _ in 0..100 {
        if pv.pos.y < 0.0 {
            return;
        }
        let q = ctx.w2c(pv.pos_f32());
        draw_circle(gizmos, q, 2.0, GRAY);
        pv.pos += pv.vel * dt;
        pv.vel += accel.as_dvec2() * dt;
    }
}

fn surface_scene_ui(state: &GameState) -> Tree<OnClick> {
    use crate::ui::*;
    use layout::layout::*;

    let vb = state.input.screen_bounds;
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return Tree::new();
    }

    let top_bar = top_bar(state);

    let show_gravity = Node::text(
        Size::Grow,
        BUTTON_HEIGHT,
        format!("{:0.1}", state.universe.surface.external_acceleration()),
    );

    let increase_gravity = Node::button(
        "More Gravity",
        OnClick::IncreaseGravity,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let decrease_gravity = Node::button(
        "Less Gravity",
        OnClick::DecreaseGravity,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let increase_wind = Node::button(
        "More Wind",
        OnClick::IncreaseWind,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let decrease_wind = Node::button(
        "Less Wind",
        OnClick::DecreaseWind,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let main_area = Node::grow().invisible();

    let wrapper = Node::structural(350, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(show_gravity)
        .with_child(increase_gravity)
        .with_child(decrease_gravity)
        .with_child(increase_wind)
        .with_child(decrease_wind);

    let layout = Node::new(vb.span.x, vb.span.y)
        .tight()
        .invisible()
        .down()
        .with_child(top_bar)
        .with_child(main_area.with_child(wrapper));

    Tree::new().with_layout(layout, Vec2::ZERO)
}

fn draw_terrain_tile(
    canvas: &mut Canvas,
    ctx: &impl CameraProjection,
    pos: IVec2,
    chunk: &TerrainChunk,
) {
    if chunk.is_air() {
        return;
    }

    if chunk.is_deep() {
        return;
    }

    let bounds = chunk_pos_to_bounds(pos);
    let bounds = ctx.w2c_aabb(bounds);
    draw_aabb(&mut canvas.gizmos, bounds, GRAY.with_alpha(0.1));

    for (tile_pos, value) in chunk.tiles() {
        let color = match value {
            Tile::Air => continue,
            Tile::DeepStone => GRAY,
            Tile::Stone => LIGHT_GRAY,
            Tile::Sand => LIGHT_YELLOW,
            Tile::Ore => ORANGE,
            Tile::Grass => DARK_GREEN,
        };

        let bounds = tile_pos_to_bounds(pos, tile_pos);
        let bounds = ctx.w2c_aabb(bounds);
        canvas.rect(bounds, color).z_index = 1.0;
    }
}

impl Render for SurfaceContext {
    fn background_color(state: &GameState) -> Srgba {
        let c = state.universe.surface.atmo_color;
        to_srgba([c[0], c[1], c[2], 1.0])
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        for (pos, chunk) in &state.universe.surface.terrain {
            draw_terrain_tile(canvas, ctx, *pos, chunk);
        }

        {
            let mut pts = Vec::new();
            for k in &state.universe.surface.elevation {
                let p = ctx.w2c(Vec2::new(k.t, k.value));
                pts.push(p);
            }
            canvas.gizmos.linestrip_2d(pts, GRAY);
        };

        ctx.particles.draw(canvas, ctx);

        for (body, _, vehicle) in &state.universe.surface_vehicles {
            let pos = ctx.w2c(body.pv.pos_f32());
            draw_vehicle(canvas, vehicle, pos, ctx.scale(), body.angle);
        }

        (|| -> Option<()> {
            let mouse_pos = ctx.c2w(state.input.current()?);
            let (_, body, vehicle) = Self::mouseover_vehicle(&state.universe, mouse_pos)?;
            let pos = ctx.w2c(body.pv.pos_f32());
            draw_circle(
                &mut canvas.gizmos,
                pos,
                vehicle.bounding_radius() * ctx.scale() * 1.1,
                RED.with_alpha(0.3),
            );
            None
        })();

        for id in &ctx.selected {
            if let Some((body, policy, vehicle)) = state.universe.surface_vehicles.get(*id) {
                let pos = ctx.w2c(body.pv.pos_f32());
                draw_circle(
                    &mut canvas.gizmos,
                    pos,
                    vehicle.bounding_radius() * ctx.scale(),
                    ORANGE.with_alpha(0.3),
                );

                let (target, angle) =
                    if let VehicleControlPolicy::PositionHold(target, angle) = policy {
                        (target, angle)
                    } else {
                        continue;
                    };

                let p = ctx.w2c(body.pv.pos_f32());
                let q = ctx.w2c(*target);
                let r = ctx.w2c(target + rotate(Vec2::X * 5.0, *angle));
                draw_x(&mut canvas.gizmos, q, 2.0 * ctx.scale(), RED);
                canvas.gizmos.line_2d(q, r, YELLOW);

                let info = crate::scenes::craft_editor::vehicle_info(vehicle);
                canvas.text(info, p, 0.01 * ctx.scale()).anchor_left();

                canvas.gizmos.line_2d(p, q, BLUE);
                if state.universe.surface.external_acceleration().length() > 0.0 {
                    draw_kinematic_arc(
                        &mut canvas.gizmos,
                        body.pv,
                        ctx,
                        state.universe.surface.external_acceleration(),
                    );
                }
            }
        }

        // grid of 10 meter increments
        for i in (-100..100).step_by(10) {
            for j in (-100..100).step_by(10) {
                let p = Vec2::new(i as f32, j as f32);
                let p = ctx.w2c(p);
                draw_cross(&mut canvas.gizmos, p, 3.0, WHITE.with_alpha(0.1));
            }
        }

        canvas.label(crate::scenes::orbital::date_label(state));

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(surface_scene_ui(state))
    }
}
