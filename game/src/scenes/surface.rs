use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::*;
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render};
use crate::sounds::*;
use crate::z_index::*;
use bevy::color::{palettes::css::*, Alpha, Srgba};
use bevy::prelude::{Gizmos, KeyCode};
use bevy_vector_shapes::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct SurfaceContext {
    camera: LinearCameraController,
    pub selected: HashSet<EntityId>,
    pub current_surface: EntityId,

    left_click_world_pos: Option<DVec2>,
    right_click_world_pos: Option<DVec2>,

    follow: Option<EntityId>,
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController::new(DVec2::Y * 30.0, 0.001, 1100.0),
            selected: HashSet::new(),
            current_surface: EntityId(0),
            left_click_world_pos: None,
            right_click_world_pos: None,
            follow: None,
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

    pub fn mouseover_vehicle<'a>(
        &'a self,
        universe: &'a Universe,
        pos: DVec2,
    ) -> Option<(EntityId, &'a SurfaceSpacecraftEntity)> {
        for (id, sv) in universe.surface_vehicles(self.current_surface) {
            let veh_screen_pos = self.w2c(sv.body.pv.pos);
            let mouse_screen_pos = self.w2c(pos);

            if veh_screen_pos.distance(mouse_screen_pos) < 25.0 {
                return Some((*id, sv));
            }

            let d = sv.body.pv.pos.distance(pos);
            let r = sv.vehicle.bounding_radius();
            if d < r {
                return Some((*id, sv));
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
        Some(AABB::from_arbitrary(
            aabb_stopgap_cast(p),
            aabb_stopgap_cast(q),
        ))
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
                .surface_vehicles(self.current_surface)
                .filter_map(|(id, sv)| {
                    bounds
                        .contains(aabb_stopgap_cast(sv.body.pv.pos))
                        .then(|| *id)
                })
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
            let (idx, _) = self.mouseover_vehicle(universe, pos)?;
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

            let angle = PI_64 / 2.0;

            let ns = self.selected.len();
            let width = (ns as f32).sqrt().ceil() as usize;

            let mut separation: f64 = 5.0;

            let mut selected: Vec<_> = self.selected.iter().collect();
            selected.sort();

            for idx in &self.selected {
                if let Some(sv) = universe.surface_vehicles.get_mut(idx) {
                    separation = separation.max(sv.vehicle.bounding_radius() * 4.0);
                }
            }

            for (i, idx) in selected.into_iter().enumerate() {
                if let Some(sv) = universe.surface_vehicles.get_mut(idx) {
                    let xi = i % width;
                    let yi = i / width;
                    let oblique = yi as f64 * 0.33;
                    let pos = p + DVec2::new(xi as f64 + oblique, yi as f64) * separation * 2.0;
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

        if input.just_pressed(KeyCode::KeyF) {
            if self.follow.is_none() {
                if let Some(id) = self.selected.iter().next() {
                    self.follow = Some(*id);
                }
            } else {
                self.follow = None;
            }
        }

        if input.just_pressed(KeyCode::KeyQ) {
            if let Some((_, sv)) = universe.surface_vehicles.iter().next().iter().next() {
                universe.add_surface_vehicle(
                    self.current_surface,
                    sv.vehicle.clone(),
                    rand(0.0, PI) as f64,
                    100.0,
                );
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
            for id in &self.selected {
                universe.remove(*id);
            }
        }
    }

    pub fn on_game_tick(state: &mut GameState) {
        let ctx = &mut state.surface_context;

        ctx.camera.on_game_tick();

        if let Some(follow) = ctx.follow {
            if let Some(sv) = state
                .universe
                .lup_surface_vehicle(follow, ctx.current_surface)
            {
                let p = sv.body.pv.pos;
                ctx.camera.follow(p);
            }
        }

        // ctx.particles.step();
    }
}

impl CameraProjection for SurfaceContext {
    fn origin(&self) -> DVec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f64 {
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
    // let dt = 0.25;
    // for _ in 0..100 {
    //     if pv.pos.y < surface.elevation(pv.pos.x as f32) as f64 {
    //         return;
    //     }
    //     let q = ctx.w2c(pv.pos);
    //     draw_circle(gizmos, q, 2.0, GRAY);
    //     pv.pos += pv.vel * dt;
    //     pv.vel += accel.as_dvec2() * dt;
    // }
}

fn draw_tracks(
    canvas: &mut Canvas,
    ctx: &impl CameraProjection,
    tracks: &HashMap<EntityId, Vec<(Nanotime, DVec2)>>,
    selected: &HashSet<EntityId>,
) {
    for (id, track) in tracks {
        let color = if selected.contains(id) { ORANGE } else { GRAY };

        for (_, p) in track {
            let p = ctx.w2c(*p);
            canvas.circle(p, 3.0, color);
        }
    }
}

fn surface_scene_ui(state: &GameState) -> Option<Tree<OnClick>> {
    use crate::ui::*;
    use layout::layout::*;

    let ctx = &state.surface_context;

    let surface_id = ctx.current_surface;

    let vb = state.input.screen_bounds;
    if vb.span.x == 0.0 || vb.span.y == 0.0 {
        return None;
    }

    let ls = state.universe.landing_sites.get(&ctx.current_surface)?;

    let top_bar: Node<OnClick> = top_bar(state);

    let show_gravity = Node::text(
        Size::Grow,
        state.settings.ui_button_height,
        format!(
            "{:0.2}",
            ls.surface.body.gravity(ls.surface.body.radius * DVec2::Y)
        ),
    );

    let main_area = Node::grow().invisible();

    let wrapper = Node::structural(350, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(show_gravity);

    let surfaces = Node::structural(350, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(
            Node::row(state.settings.ui_button_height)
                .with_text("Landing Sites")
                .with_color(UI_BACKGROUND_COLOR)
                .enabled(false),
        )
        .with_children(state.universe.landing_sites.iter().map(|(e, ls)| {
            let text = format!("{}-{}", e, ls.name);
            let onclick = OnClick::GoToSurface(*e);
            Node::button(text, onclick, Size::Grow, state.settings.ui_button_height)
                .enabled(state.surface_context.current_surface != *e)
        }));

    let layout = Node::new(vb.span.x, vb.span.y)
        .tight()
        .invisible()
        .down()
        .with_child(top_bar)
        .with_child(main_area.down().with_child(wrapper).with_child(surfaces));

    let ctx = &state.surface_context;

    let mut tree = Tree::new().with_layout(layout, Vec2::ZERO);

    if let Some(sv) = (ctx.selected.len() == 1)
        .then(|| {
            ctx.selected
                .iter()
                .next()
                .map(|id| state.universe.lup_surface_vehicle(*id, surface_id))
                .flatten()
        })
        .flatten()
    {
        let mut n = Node::structural(Size::Fit, Size::Fit)
            .with_color([0.0, 0.0, 0.0, 0.0])
            .tight()
            .down();
        let text = vehicle_info(&sv.vehicle);
        let text = format!(
            "{}Mode: {:?}\nStatus: {:?}\nP: {:0.2}\nV: {:0.2}",
            text,
            sv.controller.mode(),
            sv.controller.status(),
            sv.body.pv.pos,
            sv.body.pv.vel,
        );
        for line in text.lines() {
            n.add_child(
                Node::text(600, state.settings.ui_button_height, line)
                    .enabled(false)
                    .with_color([0.1, 0.1, 0.1, 0.8]),
            );
        }
        let pos = ctx.w2c(sv.body.pv.pos + DVec2::X * sv.vehicle.bounding_radius());
        let dims = state.input.screen_bounds.span;
        let pos = dims / 2.0 + Vec2::new(pos.x + 20.0, -pos.y);
        tree.add_layout(n, pos);
    };

    Some(tree)
}

fn vehicle_mouseover_radius(vehicle: &Vehicle, ctx: &impl CameraProjection) -> f32 {
    (vehicle.bounding_radius() * ctx.scale()).max(20.0) as f32
}

impl Render for SurfaceContext {
    fn background_color(state: &GameState) -> Srgba {
        if let Some(ls) = state
            .universe
            .landing_sites
            .get(&state.surface_context.current_surface)
        {
            let c = ls.surface.atmo_color;
            to_srgba([c[0], c[1], c[2], 1.0])
        } else {
            LIGHT_BLUE
        }
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        draw_camera_info(canvas, ctx, state.input.screen_bounds.span);

        let surface_id = state.surface_context.current_surface;

        if let Some(ls) = state.universe.landing_sites.get(&surface_id) {
            let surface = &ls.surface;

            let body_center = DVec2::ZERO;

            for (altitude, color) in [
                (-200.0, WHITE.with_alpha(0.2)),
                (-100.0, WHITE.with_alpha(0.6)),
                (0.0, WHITE),
                (5000.0, RED.with_alpha(0.3)),
                (10000.0, TEAL.with_alpha(0.3)),
                (50000.0, GREEN.with_alpha(0.3)),
                (100000.0, TEAL.with_alpha(0.3)),
                (300000.0, GREEN.with_alpha(0.3)),
            ] {
                canvas
                    .circle(
                        ctx.w2c(body_center),
                        gcast((ls.surface.body.radius + altitude) * ctx.scale()),
                        color,
                    )
                    .resolution(1000);
                canvas.sprite(
                    ctx.w2c(body_center),
                    0.0,
                    "Luna",
                    ZOrdering::Planet,
                    graphics_cast(DVec2::splat(ls.surface.body.radius) * 2.0 * ctx.scale()),
                );
            }

            let p = DVec2::new(-30.0, 200.0) + DVec2::Y * surface.body.radius;
            let p = ctx.w2c(p);
            let text = format!(
                "Landing Site\nLS-{} \"{}\"\n{}",
                surface_id,
                ls.name,
                landing_site_info(ls)
            );
            canvas.text(text, p, gcast(5.0 * ctx.scale())).color.alpha = 0.2;

            crate::craft_editor::draw_particles(canvas, ctx, &surface.particles);

            draw_tracks(canvas, ctx, &ls.tracks, &ctx.selected);

            for (id, sv) in state.universe.surface_vehicles(surface_id) {
                let pos = ctx.w2c(sv.body.pv.pos);
                draw_vehicle(
                    canvas,
                    &sv.vehicle,
                    pos,
                    gcast(ctx.scale()),
                    gcast(sv.body.angle),
                    false,
                    true,
                );

                let color: Srgba = crate::sprites::hashable_to_color(&sv.controller.mode()).into();

                canvas.circle(
                    pos,
                    7.0,
                    color.with_alpha(gcast((1.0 - ctx.scale() / 4.0).clamp(0.0, 1.0))),
                );

                let ground_track = sv.body.pv.pos.normalize_or_zero() * ls.surface.body.radius;

                for (p, q) in [(ground_track, sv.body.pv.pos)] {
                    let p = ctx.w2c(p);
                    let q = ctx.w2c(q);
                    canvas.gizmos.line_2d(p, q, RED.with_alpha(0.1));
                }

                let altitude = sv.body.pv.pos.length() - ls.surface.body.radius;
                if altitude < 2000.0 {
                    continue;
                }

                let color = if ctx.selected.contains(id) {
                    ORANGE
                } else {
                    GRAY.with_alpha(0.1)
                };
                if let Some(orbit) = sv.orbit {
                    crate::drawing::draw_orbit(canvas, &orbit, body_center, color, ctx);
                }
            }
        }

        crate::drawing::draw_piloting_overlay(canvas, state, ctx.selected.iter().next().cloned());

        (|| -> Option<()> {
            let p = state.input.current()?;
            let mouse_world_pos = ctx.c2w(p);
            let (_, sv) = ctx.mouseover_vehicle(&state.universe, mouse_world_pos)?;
            let pos = ctx.w2c(sv.body.pv.pos);
            let r = vehicle_mouseover_radius(&sv.vehicle, ctx) * 1.1;
            draw_circle(&mut canvas.gizmos, pos, r, RED.with_alpha(0.3));
            let title = sv.vehicle.title();
            canvas.text(title, pos + Vec2::new(0.0, r + 40.0), 0.8);
            None
        })();

        for (e, sv) in state.universe.surface_vehicles(surface_id) {
            if !ctx.selected.contains(e) {
                continue;
            }
            let pos = ctx.w2c(sv.body.pv.pos);
            draw_circle(
                &mut canvas.gizmos,
                pos,
                vehicle_mouseover_radius(&sv.vehicle, ctx),
                ORANGE.with_alpha(0.3),
            );
        }

        for (id, sv) in state.universe.surface_vehicles(surface_id) {
            let selected = ctx.selected.contains(id);
            let mut p = ctx.w2c(sv.body.pv.pos);

            for pose in sv.controller.get_target_queue() {
                let q = ctx.w2c(pose.0);
                let r = ctx.w2c(pose.0 + rotate_f64(DVec2::X * 5.0, pose.1));
                draw_x(&mut canvas.gizmos, q, gcast(2.0 * ctx.scale()), RED);
                if selected {
                    canvas.gizmos.line_2d(q, r, YELLOW);
                }

                let color = if selected { BLUE } else { GRAY.with_alpha(0.2) };
                canvas.gizmos.line_2d(p, q, color);
                p = q;
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
            for (_, sv) in state.universe.surface_vehicles(surface_id) {
                let p = sv.body.pv.pos;
                if bounds.contains(aabb_stopgap_cast(p)) {
                    draw_circle(
                        &mut canvas.gizmos,
                        ctx.w2c(p),
                        gcast(sv.vehicle.bounding_radius() * ctx.scale()),
                        GRAY.with_alpha(0.6),
                    );
                }
            }

            let bounds = ctx.w2c_aabb(bounds);
            draw_aabb(canvas, bounds, RED.with_alpha(0.6));
        }

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        surface_scene_ui(state)
    }
}
