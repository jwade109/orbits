use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::*;
use crate::onclick::OnClick;
use crate::scenes::rpo::LinearCameraController;
use crate::scenes::{CameraProjection, Render, TextLabel};
use bevy::color::{palettes::css::*, Alpha, Luminance, Mix, Srgba};
use bevy::prelude::{Gizmos, KeyCode};
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct SurfaceContext {
    camera: LinearCameraController,
    // real stuff
    plants: Vec<Plant>,
    wind_offset: f32,
    vehicles: Vec<Vehicle>,
    selected: HashSet<usize>,
    manual_control: bool,
    /// in 10ths of a G
    gravity: u32,
    particles: Vec<(PV, Nanotime, Srgba)>,
}

impl Default for SurfaceContext {
    fn default() -> Self {
        SurfaceContext {
            camera: LinearCameraController {
                center: Vec2::ZERO,
                target_center: Vec2::ZERO,
                scale: 1.0,
                target_scale: 1.0,
            },
            plants: generate_plants(),
            wind_offset: 0.0,
            vehicles: Vec::new(),
            selected: HashSet::new(),
            manual_control: false,
            gravity: 5,
            particles: Vec::new(),
        }
    }
}

fn generate_plants() -> Vec<Plant> {
    let ret = Vec::new();

    // for _ in 0..30 {
    //     let root = Vec2::new(rand(-100.0, 100.0), rand(-40.0, -10.0));

    //     let mut segments = Vec::new();
    //     if rand(0.0, 1.0) < 0.2 {
    //         let n_segments = randint(5, 7);
    //         for _ in 0..n_segments {
    //             let angle = rand(-0.4, 0.4);
    //             let length = rand(1.2, 2.3);
    //             segments.push((angle, length));
    //         }
    //     } else {
    //         for _ in 0..5 {
    //             let angle = rand(-0.4, 0.4);
    //             let length = rand(0.3, 0.9);
    //             segments.push((angle, length));
    //         }
    //     }

    //     let p = Plant::new(root, segments);

    //     ret.push(p);
    // }

    ret
}

pub fn keyboard_control_law(input: &InputState) -> VehicleControl {
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
        Vec2::new(0.0, -(self.gravity as f32) / 10.0 * 9.81)
    }

    pub fn increase_gravity(&mut self) {
        self.gravity += 1;
    }

    pub fn decrease_gravity(&mut self) {
        if self.gravity > 0 {
            self.gravity -= 1;
        }
    }

    // pub fn follow_position(&self) -> Option<Vec2> {
    //     let idx = self.follow_vehicle.then(|| self.ownship)??;
    //     let v = self.vehicles.get(idx)?;
    //     Some(v.pv.pos_f32())
    // }

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

    pub fn randomize(&mut self) {
        for v in &mut self.vehicles {
            let x = rand(-200.0, 200.0);
            let y = rand(40.0, 120.0);
            let target = Vec2::new(x, y);
            v.policy = VehicleControlPolicy::PositionHold(target);
        }
    }

    pub fn step(state: &mut GameState, dt: f32) {
        if state.sim_speed > 2 {
            state.sim_speed = 2;
        }

        let ctx = &mut state.surface_context;

        // if state.input.just_pressed(KeyCode::KeyF) {
        //     ctx.follow_vehicle = !ctx.follow_vehicle;
        // }

        if state.input.just_pressed(KeyCode::KeyM) {
            ctx.manual_control = !ctx.manual_control;
        }

        if state.input.just_pressed(KeyCode::KeyG) {
            ctx.randomize();
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

        ctx.camera.update(dt, &state.input);

        let gravity = ctx.gravity_vector();

        for v in ctx.vehicles.iter_mut() {
            v.step(state.sim_time, PhysicsMode::RealTime, gravity);

            for t in v.thrusters() {
                if !t.is_thrusting() || t.proto.is_rcs {
                    continue;
                }

                for _ in 0..3 {
                    if rand(0.0, 1.0) < t.throttle() {
                        let pos = rotate(t.pos, v.angle());
                        let vel = randvec(2.0, 10.0) + v.pointing() * -70.0;
                        let pv = v.pv + PV::from_f64(pos, vel);
                        let color = YELLOW.mix(&RED, rand(0.0, 1.0));
                        ctx.particles.push((pv, state.wall_time, color));
                    }
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
            .retain(|(_, t, _)| state.wall_time - *t < Nanotime::millis(2000));

        for (p, _, _) in &mut ctx.particles {
            p.pos += p.vel * dt as f64;
            p.vel *= 0.98;
        }

        // if let Some(v) = ctx.follow_position() {
        //     ctx.camera.center = v;
        //     ctx.camera.target_center = ctx.camera.center;
        // }

        for p in &mut ctx.plants {
            p.step(dt, ctx.wind_offset);
        }

        // if state.input.is_pressed(KeyCode::KeyM) {
        //     ctx.wind_offset += 0.01;
        // } else if state.input.is_pressed(KeyCode::KeyN) {
        //     ctx.wind_offset -= 0.01;
        // }

        // ctx.wind_offset = ctx.wind_offset.clamp(-0.4, 0.4);
    }
}

impl CameraProjection for SurfaceContext {
    fn origin(&self) -> Vec2 {
        self.camera.center
    }

    fn scale(&self) -> f32 {
        self.camera.scale
    }
}

fn draw_plant(gizmos: &mut Gizmos, p: &Plant, ctx: &impl CameraProjection) {
    for (p, q) in p.segments() {
        let p = ctx.w2c(p);
        let q = ctx.w2c(q);
        gizmos.line_2d(p, q, ORANGE);
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
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.3)
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        for (particle, t, color) in &ctx.particles {
            let p = ctx.w2c(particle.pos_f32());
            let age = (state.wall_time - *t).to_secs().clamp(0.0, 1.0);
            let color = color.mix(&BLACK, age);
            let size = 0.3 + age;
            canvas
                .sprite(p, 0.0, "error", 1.0, Vec2::splat(size) * ctx.scale())
                .set_color(color.with_alpha((1.0 - age).clamp(0.1, 1.0)));
        }

        for v in &ctx.vehicles {
            let target = if let VehicleControlPolicy::PositionHold(target) = v.policy {
                target
            } else {
                continue;
            };

            let p = ctx.w2c(v.pv.pos_f32());
            let q = ctx.w2c(target);
            draw_x(&mut canvas.gizmos, q, 2.0 * ctx.scale(), RED);

            // let info = crate::scenes::craft_editor::vehicle_info(v);
            // canvas.label(TextLabel::new(info, p, 0.01 * ctx.scale()));

            canvas.gizmos.line_2d(p, q, BLUE);
            if ctx.gravity_vector().length() > 0.0 {
                draw_kinematic_arc(&mut canvas.gizmos, v.pv, ctx, ctx.gravity_vector());
            }
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

        for p in &ctx.plants {
            draw_plant(&mut canvas.gizmos, p, ctx);
        }

        canvas.label(TextLabel::new(
            format!("{:0.2}", ctx.wind_offset),
            Vec2::splat(-300.0),
            1.0,
        ));

        canvas.label(crate::scenes::orbital::date_label(state));

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(surface_scene_ui(state))
    }
}
