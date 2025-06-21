use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::{FrameId, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::rpo::LinearCameraController;
use crate::scenes::{CameraProjection, Render, TextLabel};
use bevy::color::{palettes::css::*, Luminance};
use bevy::prelude::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug)]
pub struct SurfaceContext {
    // camera stuff
    camera: LinearCameraController,
    // real stuff
    plants: Vec<Plant>,
    wind_offset: f32,
    vehicles: Vec<(Vehicle, Vec2)>,
    follow_vehicle: bool,
    ownship: Option<usize>,
    manual_control: bool,
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
            follow_vehicle: false,
            ownship: None,
            manual_control: false,
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

const ACCELERATION_DUE_TO_GRAVITY: Vec2 = Vec2::new(0.0, -3.2); // m/s^2;

fn hover_control_law(gravity: Vec2, target: Vec2, vehicle: &Vehicle) -> VehicleControl {
    let future_alt = vehicle.kinematic_apoapis(gravity.length() as f64) as f32;

    let horizontal_control = {
        // if (target.y - vehicle.pv.pos.y as f32).abs() < 10.0 && vehicle.pv.vel.y.abs() < 10.0 {
        // horizontal controller
        let kp = 0.01;
        let kd = 0.08;

        // positive means to the right, which corresponds to a negative heading correction
        kp * (target.x - vehicle.pv.pos.x as f32) - kd * vehicle.pv.vel.x as f32
        // } else {
        //     0.0
        // };
    };

    // attitude controller
    let kp = 30.0;
    let kd = 50.0;
    let target_angle = PI * 0.5 - horizontal_control.clamp(-PI / 4.0, PI / 4.0);

    let attitude_error = wrap_pi_npi(target_angle - vehicle.angle());
    let attitude = kp * attitude_error - kd * vehicle.angular_velocity();

    let thrust = vehicle.max_thrust_along_heading(0.0, false);
    let accel = thrust / vehicle.wet_mass();
    let pct = gravity.length() / accel;

    // vertical controller
    let kp = 0.5;
    let kd = 2.0;
    let altitude_error = target.y - future_alt;
    let error = kp * altitude_error - kd * vehicle.pv.vel.y as f32;

    let linear = if attitude_error.abs() < 0.5 || vehicle.pv.pos.y > 10.0 {
        Vec2::X
    } else {
        Vec2::ZERO
    };

    let throttle = pct + error;

    // let throttle = if future_alt < target.y && attitude_error.abs() < PI / 5.0 {
    //     error.max(0.25)
    // } else {
    //     0.0
    // };

    VehicleControl {
        throttle,
        linear,
        attitude,
        is_rcs: false,
    }
}

impl SurfaceContext {
    pub fn add_vehicle(&mut self, vehicle: Vehicle) {
        let x = rand(-200.0, 200.0);
        let y = rand(40.0, 120.0);
        self.vehicles.push((vehicle, Vec2::new(x, y)));
    }

    pub fn ownship(&self) -> Option<(usize, &Vehicle)> {
        let idx = self.ownship?;
        self.vehicles.get(idx).map(|(v, _)| (idx, v))
    }

    pub fn follow_position(&self) -> Option<Vec2> {
        let idx = self.follow_vehicle.then(|| self.ownship)??;
        let v = self.vehicles.get(idx).map(|(v, _)| v)?;
        Some(v.pv.pos_f32())
    }

    pub fn mouseover_vehicle(&self, pos: Vec2) -> Option<(usize, &Vehicle)> {
        for (i, (v, _)) in self.vehicles.iter().enumerate() {
            let d = v.pv.pos_f32().distance(pos);
            let r = v.bounding_radius();
            if d < r {
                return Some((i, v));
            }
        }
        None
    }

    pub fn randomize(&mut self) {
        let land = rand(0.0, 1.0) < 0.2;
        for (_, target) in &mut self.vehicles {
            if land {
                *target = target.with_y(5.0);
                continue;
            }
            let x = rand(-200.0, 200.0);
            let y = rand(40.0, 120.0);
            *target = Vec2::new(x, y);
        }
    }

    pub fn step(state: &mut GameState, dt: f32) {
        state.sim_speed = 0;

        let ctx = &mut state.surface_context;

        if state.input.just_pressed(KeyCode::KeyF) {
            ctx.follow_vehicle = !ctx.follow_vehicle;
        }

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
                (state.input.position(MouseButt::Left, FrameId::Down)?, false)
            };

            let pos = ctx.c2w(pos);
            let (idx, _) = ctx.mouseover_vehicle(pos)?;
            ctx.ownship = Some(idx);
            if double {
                ctx.follow_vehicle = true;
            }
            None
        })();

        (|| -> Option<()> {
            let rc = state.input.position(MouseButt::Right, FrameId::Current)?;
            let p = ctx.c2w(rc);
            let (_, t) = ctx.vehicles.get_mut(ctx.ownship?)?;
            *t = p;
            None
        })();

        ctx.camera.update(dt, &state.input);

        for (i, (v, target)) in ctx.vehicles.iter_mut().enumerate() {
            let control = if ctx.manual_control && ctx.ownship == Some(i) {
                let is_rcs = state.input.is_pressed(KeyCode::ControlLeft);
                let control = if state.input.is_pressed(KeyCode::ArrowUp) {
                    Vec2::X
                } else if state.input.is_pressed(KeyCode::ArrowDown) {
                    -Vec2::X
                } else if state.input.is_pressed(KeyCode::ArrowLeft) && is_rcs {
                    Vec2::Y
                } else if state.input.is_pressed(KeyCode::ArrowRight) && is_rcs {
                    -Vec2::Y
                } else {
                    Vec2::ZERO
                };

                let attitude = if state.input.is_pressed(KeyCode::ArrowLeft) && !is_rcs {
                    10.0
                } else if state.input.is_pressed(KeyCode::ArrowRight) && !is_rcs {
                    -10.0
                } else {
                    0.0
                };

                VehicleControl {
                    throttle: 0.4,
                    linear: control,
                    attitude,
                    is_rcs,
                }
            } else {
                hover_control_law(ACCELERATION_DUE_TO_GRAVITY, *target, v)
            };

            // control.attitude += attitude;

            v.step(
                state.sim_time,
                control,
                PhysicsMode::RealTime,
                ACCELERATION_DUE_TO_GRAVITY,
            );

            if v.pv.pos.y < 0.0 {
                v.pv.pos.y = 0.0;
                v.pv.vel.y = 0.0;
            }
            if v.pv.pos.y == 0.0 {
                v.pv.vel.x *= 0.99;
            }
        }

        if let Some(v) = ctx.follow_position() {
            ctx.camera.center = v;
            ctx.camera.target_center = ctx.camera.center;
        }

        for p in &mut ctx.plants {
            p.step(dt, ctx.wind_offset);
        }

        if state.input.is_pressed(KeyCode::KeyM) {
            ctx.wind_offset += 0.01;
        } else if state.input.is_pressed(KeyCode::KeyN) {
            ctx.wind_offset -= 0.01;
        }

        ctx.wind_offset = ctx.wind_offset.clamp(-0.4, 0.4);
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

impl Render for SurfaceContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.3)
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.surface_context;

        {
            let sprites: String = state
                .image_handles
                .iter()
                .map(|(n, _)| format!("{}\n", n))
                .collect();
            canvas.text(sprites, Vec2::splat(-500.0), 1.0);
        }

        for (v, target) in &ctx.vehicles {
            let p = ctx.w2c(v.pv.pos_f32());
            let q = ctx.w2c(*target);
            draw_circle(
                &mut canvas.gizmos,
                q,
                2.0 * ctx.scale(),
                PURPLE.with_alpha(0.3),
            );
            canvas.gizmos.line_2d(p, q, BLUE);
            draw_kinematic_arc(&mut canvas.gizmos, v.pv, ctx, ACCELERATION_DUE_TO_GRAVITY);
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

        (|| -> Option<()> {
            let (_, v) = ctx.ownship()?;
            let pos = ctx.w2c(v.pv.pos_f32());
            draw_circle(
                &mut canvas.gizmos,
                pos,
                v.bounding_radius() * ctx.scale(),
                ORANGE.with_alpha(0.3),
            );
            None
        })();

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

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        Some(crate::ui::basic_scenes_layout(state))
    }
}
