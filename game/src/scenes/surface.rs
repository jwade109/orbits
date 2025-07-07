use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::*;
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render};
use bevy::color::{palettes::css::*, Alpha, Mix, Srgba};
use bevy::prelude::{Gizmos, KeyCode};
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct SurfaceContext {
    camera: LinearCameraController,

    vehicles: Vec<Vehicle>,
    selected: HashSet<usize>,

    surface: Surface,

    particles: Vec<ThrustParticle>,

    factory: Factory,
}

const MAX_PARTICLE_AGE_SECONDS: f32 = 3.0;

const NOMINAL_DT: Nanotime = Nanotime::millis(20);

#[derive(Debug)]
struct ThrustParticle {
    pv: PV,
    birth: Nanotime,
    stamp: Nanotime,
    lifetime: Nanotime,
    color: Srgba,
    final_color: Srgba,
}

impl ThrustParticle {
    fn new(pv: PV, stamp: Nanotime, color: Srgba, final_color: Srgba) -> Self {
        Self {
            pv,
            birth: stamp,
            stamp,
            color,
            final_color,
            lifetime: Nanotime::secs_f32(MAX_PARTICLE_AGE_SECONDS * rand(0.5, 1.0)),
        }
    }

    fn step_until(&mut self, sim_time: Nanotime) {
        while self.stamp < sim_time {
            self.pv.pos += self.pv.vel * NOMINAL_DT.to_secs_f64();
            self.pv.vel *= 0.96;

            if self.pv.pos.y < 0.0 && self.pv.vel.y < 0.0 {
                let vx = self.pv.vel.x;
                let mag = self.pv.vel.y.abs() * rand(0.6, 0.95) as f64;
                let angle = rand(0.0, 0.25);
                self.pv.vel = rotate_f64(DVec2::X * mag, angle as f64);
                if rand(0.0, 1.0) < 0.5 {
                    self.pv.vel.x *= -1.0;
                }
                self.pv.vel.x += vx;
            }
            self.stamp += NOMINAL_DT;
        }
    }
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController::new(Vec2::ZERO, 1.0),
            vehicles: Vec::new(),
            surface: Surface::random(),
            selected: HashSet::new(),
            particles: Vec::new(),
            factory: example_factory(),
        }
    }
}

#[allow(unused)]
fn keyboard_control_law(input: &InputState) -> VehicleControl {
    let allow_linear_rcs: bool = input.is_pressed(KeyCode::ControlLeft);
    let control = if input.is_pressed(KeyCode::ArrowUp) {
        Vec2::X
    } else if input.is_pressed(KeyCode::ArrowDown) {
        -Vec2::X
    } else if input.is_pressed(KeyCode::ArrowLeft) && allow_linear_rcs {
        Vec2::Y
    } else if input.is_pressed(KeyCode::ArrowRight) && allow_linear_rcs {
        -Vec2::Y
    } else {
        Vec2::ZERO
    };

    let attitude = if input.is_pressed(KeyCode::ArrowLeft) && !allow_linear_rcs {
        10.0
    } else if input.is_pressed(KeyCode::ArrowRight) && !allow_linear_rcs {
        -10.0
    } else {
        0.0
    };

    VehicleControl {
        throttle: 0.4,
        linear: control,
        attitude,
        allow_linear_rcs,
        allow_attitude_rcs: true,
    }
}

pub fn to_srbga(fl: [f32; 4]) -> Srgba {
    Srgba::new(fl[0], fl[1], fl[2], fl[3])
}

impl SurfaceContext {
    pub fn add_vehicle(&mut self, mut vehicle: Vehicle) {
        let x = rand(-200.0, 200.0);
        let y = rand(40.0, 120.0);
        let target = Vec2::new(x, y);

        let policy = VehicleControlPolicy::PositionHold(target);
        vehicle.policy = policy;
        vehicle.pv.pos = (target + randvec(10.0, 100.0)).as_dvec2();
        self.vehicles.push(vehicle);
    }

    pub fn gravity_vector(&self) -> Vec2 {
        Vec2::new(0.0, -(self.surface.gravity as f32) / 10.0 * 9.81)
    }

    pub fn increase_gravity(&mut self) {}

    pub fn decrease_gravity(&mut self) {}

    pub fn mouseover_vehicle(&self, pos: Vec2) -> Option<(usize, &Vehicle)> {
        for (i, v) in self.vehicles.iter().enumerate() {
            let d = v.pv.pos_f32().distance(pos);
            let r = v.bounding_radius();
            if d < r {
                return Some((i, v));
            }
        }
        None
    }

    pub fn step(state: &mut GameState, dt: f32) {
        // if state.sim_speed > 3 {
        //     state.sim_speed = 3;
        // }

        let ctx = &mut state.surface_context;

        if state.input.just_pressed(KeyCode::KeyF) {
            ctx.factory = example_factory();
        }

        (|| -> Option<()> {
            let (pos, double) = if let Some(p) = state.input.double_click() {
                (p, true)
            } else {
                (state.input.on_frame(MouseButt::Left, FrameId::Down)?, false)
            };

            let add = state.input.is_pressed(KeyCode::ShiftLeft);
            if !add {
                ctx.selected.clear();
            }

            let pos = ctx.c2w(pos);
            let (idx, _) = ctx.mouseover_vehicle(pos)?;
            ctx.selected.insert(idx);
            if double {
                // TODO fix this
                // ctx.follow_vehicle = true;
            }
            None
        })();

        (|| -> Option<()> {
            let rc = state.input.position(MouseButt::Right, FrameId::Current)?;
            let p = ctx.c2w(rc);

            let sep = 15.0;
            let spread = sep * ctx.selected.len() as f32 - sep;
            let center = Vec2::new(spread / 2.0, 0.0);

            for (i, idx) in ctx.selected.iter().enumerate() {
                let v = ctx.vehicles.get_mut(*idx);
                if let Some(v) = v {
                    v.policy =
                        VehicleControlPolicy::PositionHold(p + Vec2::X * 15.0 * i as f32 - center);
                }
            }
            None
        })();

        ctx.factory.do_stuff(state.sim_time);

        ctx.camera.update(dt, &state.input);

        let gravity = ctx.gravity_vector();

        for v in ctx.vehicles.iter_mut() {
            let stamp = v.stamp();

            v.step(state.sim_time, PhysicsMode::RealTime, gravity);

            for t in v.thrusters_ref() {
                let mut stamp = stamp;

                if !t.variant.is_thrusting() || t.variant.model().is_rcs {
                    continue;
                }

                while stamp < state.sim_time {
                    let pos = rotate(t.center_meters(), v.angle());
                    let ve = t.variant.model().exhaust_velocity / 20.0;
                    let u = rotate(t.thrust_pointing(), v.angle());
                    let vel = randvec(2.0, 10.0) + u * -ve * rand(0.6, 1.0);
                    let pv = v.pv + PV::from_f64(pos, vel);
                    let c1 = to_srbga(t.variant.model().primary_color);
                    let c2 = to_srbga(t.variant.model().secondary_color);
                    let color = c1.mix(&c2, rand(0.0, 1.0));
                    let final_color = WHITE.mix(&DARK_GRAY, rand(0.3, 0.9)).with_alpha(0.4);
                    ctx.particles
                        .push(ThrustParticle::new(pv, stamp, color, final_color));
                    stamp += NOMINAL_DT;
                }
            }

            if v.pv.pos.y < 0.0 {
                v.pv.pos.y = 0.0;
                v.pv.vel.y = 0.0;
            }

            if v.pv.pos.y == 0.0 {
                v.pv.vel.x *= 0.98;
            }
        }

        ctx.particles
            .retain(|p| state.sim_time - p.birth < p.lifetime);

        for part in &mut ctx.particles {
            part.step_until(state.sim_time);
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
        format!("{}", state.surface_context.gravity_vector()),
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

    let main_area = Node::grow().invisible();

    let wrapper = Node::structural(350, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(show_gravity)
        .with_child(increase_gravity)
        .with_child(decrease_gravity);

    let layout = Node::new(vb.span.x, vb.span.y)
        .tight()
        .invisible()
        .down()
        .with_child(top_bar)
        .with_child(main_area.with_child(wrapper));

    Tree::new().with_layout(layout, Vec2::ZERO)
}

impl Render for SurfaceContext {
    fn background_color(state: &GameState) -> Srgba {
        let c = state.surface_context.surface.atmo_color;
        to_srbga([c[0], c[1], c[2], 1.0])
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        // draw_factory(
        //     canvas,
        //     &ctx.factory,
        //     AABB::new(Vec2::ZERO, Vec2::new(700.0, 500.0)),
        //     state.sim_time,
        // );

        {
            let bl = ctx.w2c(Vec2::new(-1000.0, -500.0));
            let tr = ctx.w2c(Vec2::new(1000.0, 0.0));
            let center = (tr + bl) / 2.0;
            let dims = tr - bl;

            if dims.x > 0.0 && dims.y > 0.0 {
                let color = ctx.surface.land_color;
                let color = to_srbga([color[0], color[1], color[2], 1.0]);

                canvas
                    .sprite(center, 0.0, "error", -10.0, dims)
                    .set_color(color);
            }
        };

        for (i, particle) in ctx.particles.iter().enumerate() {
            let p = ctx.w2c(particle.pv.pos_f32());
            let age = (state.sim_time - particle.birth).to_secs();
            let alpha = (1.0 - age / particle.lifetime.to_secs())
                .powi(3)
                .clamp(0.0, 1.0);
            let color = particle
                .color
                .mix(&particle.final_color, age.clamp(0.0, 1.0).sqrt());
            let size = 1.0 + age * 12.0;
            let stretch = (8.0 * (1.0 - age * 2.0)).max(1.0);
            let angle = particle.pv.vel.to_angle() as f32;
            canvas
                .sprite(
                    p,
                    angle,
                    "cloud",
                    1.0 + i as f32 / 10000.0,
                    Vec2::new(size * stretch, size) * ctx.scale(),
                )
                .set_color(color.with_alpha(color.alpha * alpha));
        }

        for v in &ctx.vehicles {
            let pos = ctx.w2c(v.pv.pos_f32());
            draw_vehicle(canvas, v, pos, ctx.scale(), v.angle());
        }

        (|| -> Option<()> {
            let mouse_pos = ctx.c2w(state.input.current()?);
            let (_, vehicle) = ctx.mouseover_vehicle(mouse_pos)?;
            let pos = ctx.w2c(vehicle.pv.pos_f32());
            draw_circle(
                &mut canvas.gizmos,
                pos,
                vehicle.bounding_radius() * ctx.scale() * 1.1,
                RED.with_alpha(0.3),
            );
            None
        })();

        for id in &ctx.selected {
            if let Some(v) = ctx.vehicles.get(*id) {
                let pos = ctx.w2c(v.pv.pos_f32());
                draw_circle(
                    &mut canvas.gizmos,
                    pos,
                    v.bounding_radius() * ctx.scale(),
                    ORANGE.with_alpha(0.3),
                );

                let sim = simulate_vehicle(v.clone(), ctx.gravity_vector());

                for (i, (pos, angle)) in sim.iter().enumerate() {
                    let p = ctx.w2c(*pos);
                    draw_x(&mut canvas.gizmos, p, 2.0, WHITE.with_alpha(0.3));
                    if i % 20 == 0 {
                        let q = ctx.w2c(pos + rotate(Vec2::X * 5.0, *angle));
                        canvas.gizmos.line_2d(p, q, YELLOW.with_alpha(0.7));
                    }
                }

                let target = if let VehicleControlPolicy::PositionHold(target) = v.policy {
                    target
                } else {
                    continue;
                };

                let p = ctx.w2c(v.pv.pos_f32());
                let q = ctx.w2c(target);
                draw_x(&mut canvas.gizmos, q, 2.0 * ctx.scale(), RED);

                let info = crate::scenes::craft_editor::vehicle_info(v);
                canvas.text(info, p, 0.01 * ctx.scale()).anchor_left();

                canvas.gizmos.line_2d(p, q, BLUE);
                if ctx.gravity_vector().length() > 0.0 {
                    draw_kinematic_arc(&mut canvas.gizmos, v.pv, ctx, ctx.gravity_vector());
                }
            }
        }

        let p = ctx.w2c(Vec2::new(-400.0, 0.0));
        let q = ctx.w2c(Vec2::new(400.0, 0.0));

        // grid of 10 meter increments
        for i in (-100..100).step_by(10) {
            for j in (-100..100).step_by(10) {
                let p = Vec2::new(i as f32, j as f32);
                let p = ctx.w2c(p);
                draw_cross(&mut canvas.gizmos, p, 3.0, WHITE.with_alpha(0.1));
            }
        }

        canvas.gizmos.line_2d(p, q, WHITE);

        canvas.label(crate::scenes::orbital::date_label(state));

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(surface_scene_ui(state))
    }
}
