use crate::math::*;
use crate::pid::PDCtrl;
use crate::vehicle::*;
use enum_iterator::{next_cycle, Sequence};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct ThrustAxisControl {
    pub use_rcs: bool,
    pub throttle: f32,
}

impl ThrustAxisControl {
    pub const NULLOPT: ThrustAxisControl = ThrustAxisControl {
        use_rcs: false,
        throttle: 0.0,
    };
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct VehicleControl {
    pub plus_x: ThrustAxisControl,
    pub plus_y: ThrustAxisControl,
    pub neg_x: ThrustAxisControl,
    pub neg_y: ThrustAxisControl,
    pub attitude: f32,
}

impl VehicleControl {
    pub const NULLOPT: Self = Self {
        plus_x: ThrustAxisControl::NULLOPT,
        plus_y: ThrustAxisControl::NULLOPT,
        neg_x: ThrustAxisControl::NULLOPT,
        neg_y: ThrustAxisControl::NULLOPT,
        attitude: 0.0,
    };

    pub const FORWARD: Self = Self {
        plus_x: ThrustAxisControl {
            use_rcs: false,
            throttle: 1.0,
        },
        plus_y: ThrustAxisControl::NULLOPT,
        neg_x: ThrustAxisControl::NULLOPT,
        neg_y: ThrustAxisControl::NULLOPT,
        attitude: 0.0,
    };
}

// const ATTITUDE_CONTROLLER: PDCtrl = PDCtrl::new(40.0, 60.0);

// const VERTICAL_CONTROLLER: PDCtrl = PDCtrl::new(0.03, 0.3);

// const vehicle.horizontal_controller: PDCtrl = PDCtrl::new(0.01, 0.08);

// const DOCKING_LINEAR_CONTROLLER: PDCtrl = PDCtrl::new(10.0, 300.0);

fn zero_gravity_control_law(
    target: Vec2,
    target_angle: f32,
    body: &RigidBody,
    vehicle: &Vehicle,
) -> VehicleControl {
    let mut ctrl = VehicleControl::NULLOPT;

    let pos = body.pv.pos_f32();
    let vel = body.pv.vel_f32();
    let error = target - pos;
    let distance = error.length();

    let error_hat = error.normalize_or_zero();

    let vel_along_error = error_hat * error_hat.dot(vel);
    let bad_vel = vel - vel_along_error;

    if distance < 20.0 {
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

        let error = rotate(target - body.pv.pos_f32(), -body.angle);
        let error_rate = rotate(body.pv.vel_f32(), -body.angle);

        let ax = vehicle
            .docking_linear_controller
            .apply(error.x, error_rate.x);
        let ay = vehicle
            .docking_linear_controller
            .apply(error.y, error_rate.y);

        if ax > 0.0 {
            ctrl.plus_x.throttle = ax.abs();
        } else {
            ctrl.neg_x.throttle = ax.abs();
        }

        if ay > 0.0 {
            ctrl.plus_y.throttle = ay.abs();
        } else {
            ctrl.neg_y.throttle = ay.abs();
        }

        ctrl.plus_x.use_rcs = true;
        ctrl.plus_y.use_rcs = true;
        ctrl.neg_x.use_rcs = true;
        ctrl.neg_y.use_rcs = true;
    } else if bad_vel.length() > 2.0 && vel.length() > 5.0 {
        let target_angle = (-bad_vel).to_angle();
        let angle_error = wrap_pi_npi(target_angle - body.angle);
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        if angle_error.abs() < 0.2 {
            ctrl.plus_x.throttle = 0.4;
        }
    } else if distance > 100.0 {
        let target_angle = error.to_angle();
        let angle_error = wrap_pi_npi(target_angle - body.angle);
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        if angle_error.abs() < 0.05 && vel.length() < 4.0 {
            ctrl.plus_x.throttle = 0.2;
        }

        let bad_vel = rotate(bad_vel, -body.angle);
        if bad_vel.y > 0.1 {
            ctrl.neg_y.throttle = 1.0;
            ctrl.neg_y.use_rcs = true;
        } else if bad_vel.y < 0.1 {
            ctrl.plus_y.throttle = 1.0;
            ctrl.plus_y.use_rcs = true;
        }
    } else if vel.length() > 3.0 {
        let forward = vehicle.max_forward_thrust();
        let backward = vehicle.max_backwards_thrust();

        if forward > 0.0 && backward / forward > 0.5 {
            let target_angle = vel.to_angle();
            let angle_error = wrap_pi_npi(target_angle - body.angle);
            ctrl.attitude =
                compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
            if angle_error.abs() < 0.05 {
                ctrl.neg_x.throttle = 0.2;
            }
        } else {
            // flip and burn
            let target_angle = (-vel).to_angle();
            let angle_error = wrap_pi_npi(target_angle - body.angle);
            ctrl.attitude =
                compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
            if angle_error.abs() < 0.05 {
                ctrl.plus_x.throttle = 0.2;
            }
        }
    } else if vel.length() < 1.0 {
        let target_angle = error.to_angle();
        let angle_error = wrap_pi_npi(target_angle - body.angle);
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        if angle_error.abs() < 0.1 {
            ctrl.plus_x.throttle = 0.2;
        }
    } else {
        ctrl.attitude = compute_attitude_control(body, 0.0, &vehicle.attitude_controller);
    }

    ctrl
}

fn compute_attitude_control(body: &RigidBody, target_angle: f32, pid: &PDCtrl) -> f32 {
    let attitude_error = wrap_pi_npi(target_angle - body.angle);
    pid.apply(attitude_error, body.angular_velocity)
}

fn hover_control_law(
    target: Vec2,
    gravity: Vec2,
    vehicle: &Vehicle,
    body: &RigidBody,
) -> VehicleControl {
    let upright_angle = Vec2::new(-gravity.x, -gravity.y).to_angle();

    let target = if target.distance(body.pv.pos_f32()) > 250.0 {
        let d = target - body.pv.pos_f32();
        d.normalize_or_zero() * 250.0 + body.pv.pos_f32()
    } else {
        target
    };

    let horizontal_control = vehicle
        .horizontal_controller
        .apply(target.x - body.pv.pos.x as f32, body.pv.vel.x as f32);

    // attitude controller
    let target_angle = upright_angle - horizontal_control.clamp(-PI / 6.0, PI / 6.0);
    let attitude_error = (body.angle - target_angle).abs();
    let attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

    let thrust = vehicle.max_forward_thrust();
    let accel = thrust / vehicle.total_mass().to_kg_f32();
    let pct = gravity.length() / accel;

    // vertical controller
    let error = vehicle
        .vertical_controller
        .apply(target.y - body.pv.pos.y as f32, body.pv.vel.y as f32);

    let throttle = pct + error;

    let mut ctrl = VehicleControl::NULLOPT;

    if attitude_error < 0.7 {
        ctrl.plus_x.throttle = throttle;
    }

    ctrl.attitude = attitude;

    ctrl
}

pub fn position_hold_control_law(
    target: Pose,
    body: &RigidBody,
    vehicle: &Vehicle,
    gravity: Vec2,
) -> VehicleControl {
    if gravity.length() > 0.0 {
        hover_control_law(target.0, gravity, vehicle, body)
    } else {
        zero_gravity_control_law(target.0, target.1, body, vehicle)
    }
}

pub fn velocity_control_law(
    vel: Vec2,
    body: &RigidBody,
    vehicle: &Vehicle,
    gravity: Vec2,
) -> VehicleControl {
    if gravity.length() == 0.0 {
        return VehicleControl::NULLOPT;
    }

    let mut cmd = VehicleControl::NULLOPT;

    let upright_angle = vel.to_angle();
    let velocity_error = vel - body.pv.vel_f32();
    let heading_dir = velocity_error - gravity;
    let target_angle = heading_dir
        .to_angle()
        .clamp(upright_angle - 0.2 * PI, upright_angle + 0.2 * PI);

    // attitude controller
    // let attitude_error = (body.angle - target_angle).abs();
    let attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

    let vmag = body.pv.vel_f32().length();
    let vmag_error = vel.length() - vmag;

    let pid = PDCtrl::new(0.3, 30.0);

    let extra_throttle = pid.apply(vmag_error, 0.0);

    let thrust = vehicle.max_forward_thrust();
    let accel = thrust / vehicle.total_mass().to_kg_f32();
    let pct = gravity.length() / accel + extra_throttle;

    cmd.attitude = attitude;
    cmd.plus_x.throttle = pct;

    cmd
}

pub fn enter_orbit_control_law(
    body: &RigidBody,
    vehicle: &Vehicle,
    _gravity: Vec2,
) -> VehicleControl {
    let altitude_km = body.pv.pos_f32().y / 1000.0;

    let off_normal = if altitude_km < 15.0 {
        0.0
    } else if altitude_km < 40.0 {
        0.25 * PI
    } else {
        0.5 * PI
    };

    let target_angle = PI / 2.0 - off_normal;

    let mut cmd = VehicleControl::NULLOPT;
    cmd.plus_x.throttle = 1.0;
    cmd.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

    cmd
}

#[derive(Debug, Clone, Copy, Sequence, PartialEq, Eq)]
pub enum VehicleControlPolicy {
    Idle,
    External,
    PositionHold,
    Velocity,
}

#[derive(Debug, Clone)]
pub struct VehicleController {
    mode: VehicleControlPolicy,
    position_queue: Vec<(Vec2, f32)>,
}

pub type Pose = (Vec2, f32);

impl VehicleController {
    pub fn idle() -> Self {
        Self {
            mode: VehicleControlPolicy::Idle,
            position_queue: Vec::new(),
        }
    }

    pub fn external() -> Self {
        Self {
            mode: VehicleControlPolicy::External,
            position_queue: Vec::new(),
        }
    }

    pub fn velocity() -> Self {
        Self {
            mode: VehicleControlPolicy::Velocity,
            position_queue: Vec::new(),
        }
    }

    pub fn position_hold(pose: Pose) -> Self {
        Self {
            mode: VehicleControlPolicy::PositionHold,
            position_queue: vec![pose],
        }
    }

    pub fn mission(poses: Vec<Pose>) -> Self {
        Self {
            mode: VehicleControlPolicy::PositionHold,
            position_queue: poses,
        }
    }

    pub fn set_idle(&mut self) {
        self.mode = VehicleControlPolicy::Idle;
    }

    pub fn enqueue_target_pose(&mut self, pose: Pose, clear_queue: bool) {
        if clear_queue {
            self.position_queue.clear();
        }
        self.position_queue.push(pose);
        self.mode = VehicleControlPolicy::PositionHold;
    }

    pub fn mode(&self) -> VehicleControlPolicy {
        self.mode
    }

    pub fn is_idle(&self) -> bool {
        self.mode == VehicleControlPolicy::Idle
    }

    pub fn get_target_pose(&self) -> Option<Pose> {
        self.position_queue.get(0).cloned()
    }

    pub fn get_target_queue(&self) -> impl Iterator<Item = Pose> + use<'_> {
        self.position_queue
            .iter()
            .filter(|_| self.mode == VehicleControlPolicy::PositionHold)
            .cloned()
    }

    pub fn go_to_next_mode(&mut self) {
        self.mode = next_cycle(&self.mode);
    }

    pub fn clear_queue(&mut self) {
        self.position_queue.clear();
    }

    fn mark_target_achieved(&mut self) {
        if self.position_queue.len() > 1 {
            self.position_queue.remove(0);
        }
    }

    pub fn check_target_achieved(&mut self, body: &RigidBody, ignore_angle: bool) {
        let (pos, angle) = match self.get_target_pose() {
            Some(p) => p,
            None => return,
        };

        if self.position_queue.len() <= 1 {
            return;
        }

        let d = pos.distance(body.pv.pos_f32()).abs();
        let v = body.pv.vel_f32().length().abs();
        let a = wrap_pi_npi(angle - body.angle).abs();

        if d > 2.0 {
            return;
        }

        if v > 5.0 {
            return;
        }

        if a > 0.1 && !ignore_angle {
            return;
        }

        self.mark_target_achieved();
    }
}
