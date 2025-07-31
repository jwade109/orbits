use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::*;
use crate::onclick::OnClick;
use crate::scenes::craft_editor::vehicle_info;
use crate::scenes::{CameraProjection, Render};
use crate::sounds::*;
use crate::thrust_particles::*;
use bevy::color::{palettes::css::*, Alpha, Srgba};
use bevy::prelude::{Gizmos, KeyCode};
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct SurfaceContext {
    camera: LinearCameraController,
    selected: HashSet<EntityId>,
    particles: ThrustParticleEffects,

    left_click_world_pos: Option<Vec2>,
    right_click_world_pos: Option<Vec2>,
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController::new(Vec2::Y * 30.0, 10.0, 1100.0),
            selected: HashSet::new(),
            particles: ThrustParticleEffects::new(),
            left_click_world_pos: None,
            right_click_world_pos: None,
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
    ) -> Option<(EntityId, &RigidBody, &Vehicle)> {
        for (id, sv) in &universe.surface_vehicles {
            let d = sv.body.pv.pos_f32().distance(pos);
            let r = sv.vehicle.bounding_radius();
            if d < r {
                return Some((*id, &sv.body, &sv.vehicle));
            }
        }
        None
    }

    pub fn selection_region(&self, mouse_pos: Option<Vec2>) -> Option<AABB> {
        let (p, q) = self.left_click_world_pos.zip(mouse_pos)?;
        let q = self.c2w(q);
        if p.distance(q) < 4.0 {
            return None;
        }
        Some(AABB::from_arbitrary(p, q))
    }

    pub fn on_render_tick(
        &mut self,
        input: &InputState,
        universe: &mut Universe,
        sounds: &mut EnvironmentSounds,
    ) {
        self.camera.handle_input(input);

        if let Some(bounds) = self.selection_region(input.on_frame(MouseButt::Left, FrameId::Up)) {
            self.selected = universe
                .surface_vehicles
                .iter()
                .filter_map(|(id, sv)| bounds.contains(sv.body.pv.pos_f32()).then(|| *id))
                .collect();
        }

        if input.position(MouseButt::Left, FrameId::Current).is_some() {
            if let Some(p) = input.position(MouseButt::Left, FrameId::Down) {
                if self.left_click_world_pos.is_none() {
                    self.left_click_world_pos = Some(self.c2w(p));
                }
            }
        } else {
            self.left_click_world_pos = None;
        }

        if input.position(MouseButt::Right, FrameId::Current).is_some() {
            if let Some(p) = input.position(MouseButt::Right, FrameId::Down) {
                if self.right_click_world_pos.is_none() {
                    self.right_click_world_pos = Some(self.c2w(p));
                }
            }
        } else {
            self.right_click_world_pos = None;
        }

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
            sounds.play_once("soft-pulse.ogg", 1.0);
            self.selected.insert(idx);
            if double {
                // TODO fix this
                // ctx.follow_vehicle = true;
            }
            None
        })();

        (|| -> Option<()> {
            let rc = input.on_frame(MouseButt::Right, FrameId::Down)?;
            let p = self.c2w(rc);

            sounds.play_once("soft-pulse-higher.ogg", 0.6);

            let clear_queue = !input.is_pressed(KeyCode::ShiftLeft);

            let angle = PI / 2.0;

            let ns = self.selected.len();
            let width = (ns as f32).sqrt().ceil() as usize;

            let mut separation: f32 = 5.0;

            let mut selected: Vec<_> = self.selected.iter().collect();
            selected.sort();

            for idx in &self.selected {
                if let Some(sv) = universe.surface_vehicles.get_mut(idx) {
                    separation = separation.max(sv.vehicle.bounding_radius());
                }
            }

            for (i, idx) in selected.into_iter().enumerate() {
                if let Some(sv) = universe.surface_vehicles.get_mut(idx) {
                    let xi = i % width;
                    let yi = i / width;
                    let pos = p + Vec2::new(xi as f32, yi as f32) * separation * 2.0;
                    let pose: Pose = (pos, angle);
                    sv.controller.enqueue_target_pose(pose, clear_queue);
                }
            }

            None
        })();

        if input.just_pressed(KeyCode::KeyN) {
            for idx in &self.selected {
                if let Some(sv) = universe.surface_vehicles.get_mut(idx) {
                    sv.controller.go_to_next_mode();
                }
            }
        }

        if input.just_pressed(KeyCode::KeyC) {
            for idx in &self.selected {
                if let Some(sv) = universe.surface_vehicles.get_mut(idx) {
                    sv.controller.clear_queue();
                }
            }
        }

        if input.just_pressed(KeyCode::Delete) {
            universe
                .surface_vehicles
                .retain(|id, _| !self.selected.contains(id))
        }
    }

    pub fn on_game_tick(state: &mut GameState) {
        let ctx = &mut state.surface_context;

        ctx.camera.on_game_tick();

        ctx.particles.step();

        for (_, sv) in state.universe.surface_vehicles.iter_mut() {
            for (_, part) in sv.vehicle.parts() {
                if let Some((t, d)) = part.as_thruster() {
                    if !d.is_thrusting(t) || t.is_rcs() {
                        continue;
                    }

                    ctx.particles.add(&sv.vehicle, part);
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

#[allow(unused)]
fn draw_kinematic_arc(
    gizmos: &mut Gizmos,
    mut pv: PV,
    ctx: &impl CameraProjection,
    accel: Vec2,
    surface: &Surface,
) {
    let dt = 0.25;
    for _ in 0..100 {
        if pv.pos.y < surface.elevation(pv.pos.x as f32) as f64 {
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
        state.settings.ui_button_height,
        format!("{:0.1}", state.universe.surface.external_acceleration()),
    );

    let increase_gravity = Node::button(
        "+Y",
        OnClick::IncreaseGravity,
        Size::Grow,
        state.settings.ui_button_height,
    );

    let decrease_gravity = Node::button(
        "-Y",
        OnClick::DecreaseGravity,
        Size::Grow,
        state.settings.ui_button_height,
    );

    let increase_wind = Node::button(
        "+X",
        OnClick::IncreaseWind,
        Size::Grow,
        state.settings.ui_button_height,
    );

    let decrease_wind = Node::button(
        "-X",
        OnClick::DecreaseWind,
        Size::Grow,
        state.settings.ui_button_height,
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

    let ctx = &state.surface_context;

    let mut tree = Tree::new().with_layout(layout, Vec2::ZERO);

    if let Some(sv) = (ctx.selected.len() == 1)
        .then(|| {
            ctx.selected
                .iter()
                .next()
                .map(|id| state.universe.surface_vehicles.get(id))
                .flatten()
        })
        .flatten()
    {
        let mut n = Node::structural(Size::Fit, Size::Fit)
            .with_color(UI_BACKGROUND_COLOR)
            .down();
        let text = vehicle_info(&sv.vehicle);
        for line in text.lines() {
            n.add_child(Node::text(300, state.settings.ui_button_height, line));
        }
        let pos = ctx.w2c(sv.body.pv.pos_f32() + Vec2::X * sv.vehicle.bounding_radius());
        let dims = state.input.screen_bounds.span;
        let pos = dims / 2.0 + Vec2::new(pos.x, -pos.y);
        tree.add_layout(n, pos);
    };

    tree
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

fn vehicle_mouseover_radius(vehicle: &Vehicle, ctx: &impl CameraProjection) -> f32 {
    (vehicle.bounding_radius() * ctx.scale()).max(20.0)
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

        for (_, sv) in &state.universe.surface_vehicles {
            let pos = ctx.w2c(sv.body.pv.pos_f32());
            draw_vehicle(canvas, &sv.vehicle, pos, ctx.scale(), sv.body.angle);

            canvas.circle(
                pos,
                7.0,
                RED.with_alpha((1.0 - ctx.scale() / 4.0).clamp(0.0, 1.0)),
            );
        }

        (|| -> Option<()> {
            let mouse_pos = ctx.c2w(state.input.current()?);
            let (_, body, vehicle) = Self::mouseover_vehicle(&state.universe, mouse_pos)?;
            let pos = ctx.w2c(body.pv.pos_f32());
            draw_circle(
                &mut canvas.gizmos,
                pos,
                vehicle_mouseover_radius(vehicle, ctx) * 1.1,
                RED.with_alpha(0.3),
            );
            None
        })();

        for id in &ctx.selected {
            if let Some(sv) = state.universe.surface_vehicles.get(id) {
                let pos = ctx.w2c(sv.body.pv.pos_f32());
                draw_circle(
                    &mut canvas.gizmos,
                    pos,
                    vehicle_mouseover_radius(&sv.vehicle, ctx),
                    ORANGE.with_alpha(0.3),
                );
            }
        }

        for (id, sv) in &state.universe.surface_vehicles {
            let selected = ctx.selected.contains(id);
            let mut p = ctx.w2c(sv.body.pv.pos_f32());
            for pose in sv.controller.get_target_queue() {
                let q = ctx.w2c(pose.0);
                let r = ctx.w2c(pose.0 + rotate(Vec2::X * 5.0, pose.1));
                draw_x(&mut canvas.gizmos, q, 2.0 * ctx.scale(), RED);
                if selected {
                    canvas.gizmos.line_2d(q, r, YELLOW);
                }

                let color = if selected { BLUE } else { GRAY.with_alpha(0.2) };
                canvas.gizmos.line_2d(p, q, color);
                p = q;
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

        if let Some(p) = ctx.left_click_world_pos {
            canvas.circle(ctx.w2c(p), 10.0, GREEN);
        }
        if let Some(p) = ctx.right_click_world_pos {
            canvas.circle(ctx.w2c(p), 10.0, BLUE);
        }

        if let Some(bounds) =
            ctx.selection_region(state.input.position(MouseButt::Left, FrameId::Current))
        {
            for (_, sv) in &state.universe.surface_vehicles {
                let p = sv.body.pv.pos_f32();
                if bounds.contains(p) {
                    draw_circle(
                        &mut canvas.gizmos,
                        ctx.w2c(p),
                        sv.vehicle.bounding_radius() * ctx.scale(),
                        GRAY.with_alpha(0.6),
                    );
                }
            }

            let bounds = ctx.w2c_aabb(bounds);
            draw_aabb(&mut canvas.gizmos, bounds, RED.with_alpha(0.6));
        }

        let mut p = -state.input.screen_bounds.span / 2.0;
        let h = 6.0;

        let bar = |lower: Vec2, w: f32| {
            let upper = lower + Vec2::new(w, h);
            AABB::from_arbitrary(lower, upper)
        };

        for id in &ctx.selected {
            p += Vec2::Y * (h + 1.0);
            if let Some(sv) = state.universe.surface_vehicles.get(id) {
                let c1 = crate::sprites::hashable_to_color(id);
                for (t, d) in sv.vehicle.thrusters() {
                    let color = c1.with_saturation(if t.is_rcs { 0.3 } else { 1.0 });
                    let w = d.seconds_remaining() * 15.0;
                    let aabb = bar(p, w);
                    canvas.rect(aabb, color).z_index = 100.0;
                    p += Vec2::Y * (h + 1.0);
                }
            }
        }

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(surface_scene_ui(state))
    }
}
