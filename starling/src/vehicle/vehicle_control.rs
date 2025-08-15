use crate::math::*;
use crate::orbits::Body;
use crate::orbits::SparseOrbit;
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
    pub attitude: f64,
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

fn zero_gravity_control_law(
    target: DVec2,
    target_angle: f64,
    body: &RigidBody,
    vehicle: &Vehicle,
) -> (VehicleControl, VehicleControlStatus) {
    let mut ctrl = VehicleControl::NULLOPT;

    let pos = body.pv.pos;
    let vel = body.pv.vel;
    let error = target - pos;
    let distance = error.length();

    let error_hat = error.normalize_or_zero();

    let vel_along_error = error_hat * error_hat.dot(vel);
    let bad_vel = vel - vel_along_error;

    let status = if distance < 20.0 {
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

        let error = rotate_f64(target - body.pv.pos, -body.angle);
        let error_rate = rotate_f64(body.pv.vel, -body.angle);

        let ax = vehicle
            .docking_linear_controller
            .apply(error.x, error_rate.x);
        let ay = vehicle
            .docking_linear_controller
            .apply(error.y, error_rate.y);

        if ax > 0.0 {
            ctrl.plus_x.throttle = ax.abs() as f32;
        } else {
            ctrl.neg_x.throttle = ax.abs() as f32;
        }

        if ay > 0.0 {
            ctrl.plus_y.throttle = ay.abs() as f32;
        } else {
            ctrl.neg_y.throttle = ay.abs() as f32;
        }

        ctrl.plus_x.use_rcs = true;
        ctrl.plus_y.use_rcs = true;
        ctrl.neg_x.use_rcs = true;
        ctrl.neg_y.use_rcs = true;
        0
    } else if bad_vel.length() > 2.0 && vel.length() > 5.0 {
        let target_angle = (-bad_vel).to_angle();
        let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        if angle_error.abs() < 0.2 {
            ctrl.plus_x.throttle = 0.4;
        }
        1
    } else if distance > 100.0 {
        let target_angle = error.to_angle();
        let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        if angle_error.abs() < 0.05 && vel.length() < 4.0 {
            ctrl.plus_x.throttle = 0.2;
        }

        let bad_vel = rotate_f64(bad_vel, -body.angle);
        if bad_vel.y > 0.1 {
            ctrl.neg_y.throttle = 1.0;
            ctrl.neg_y.use_rcs = true;
        } else if bad_vel.y < 0.1 {
            ctrl.plus_y.throttle = 1.0;
            ctrl.plus_y.use_rcs = true;
        }
        2
    } else if vel.length() > 3.0 {
        let forward = vehicle.max_forward_thrust();
        let backward = vehicle.max_backwards_thrust();

        if forward > 0.0 && backward / forward > 0.5 {
            let target_angle = vel.to_angle();
            let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
            ctrl.attitude =
                compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
            if angle_error.abs() < 0.05 {
                ctrl.neg_x.throttle = 0.2;
            }
            3
        } else {
            // flip and burn
            let target_angle = (-vel).to_angle();
            let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
            ctrl.attitude =
                compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
            if angle_error.abs() < 0.05 {
                ctrl.plus_x.throttle = 0.2;
            }
            4
        }
    } else if vel.length() < 1.0 {
        let target_angle = error.to_angle();
        let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
        ctrl.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        if angle_error.abs() < 0.1 {
            ctrl.plus_x.throttle = 0.2;
        }
        5
    } else {
        ctrl.attitude = compute_attitude_control(body, 0.0, &vehicle.attitude_controller);
        6
    };

    (ctrl, VehicleControlStatus::ZeroGRendezvous(status))
}

fn compute_attitude_control(body: &RigidBody, target_angle: f64, pid: &PDCtrl) -> f64 {
    let attitude_error = wrap_pi_npi_f64(target_angle - body.angle);
    pid.apply(attitude_error, body.angular_velocity)
}

fn hover_control_law(
    target: DVec2,
    gravity: DVec2,
    vehicle: &Vehicle,
    body: &RigidBody,
) -> (VehicleControl, VehicleControlStatus) {
    let upright_angle = DVec2::new(-gravity.x, -gravity.y).to_angle();

    let target = if target.distance(body.pv.pos) > 250.0 {
        let d = target - body.pv.pos;
        d.normalize_or_zero() * 250.0 + body.pv.pos
    } else {
        target
    };

    let horizontal_control = vehicle
        .horizontal_controller
        .apply(target.x - body.pv.pos.x as f64, body.pv.vel.x as f64);

    // attitude controller
    let target_angle = upright_angle - horizontal_control.clamp(-PI_64 / 6.0, PI_64 / 6.0);
    let attitude_error = (body.angle - target_angle).abs();
    let attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

    let thrust = vehicle.max_forward_thrust();
    let accel = thrust / vehicle.total_mass().to_kg_f64();
    let pct = gravity.length() / accel;

    // vertical controller
    let error = vehicle
        .vertical_controller
        .apply(target.y - body.pv.pos.y as f64, body.pv.vel.y as f64);

    let throttle = pct + error;

    let mut ctrl = VehicleControl::NULLOPT;

    let status = if attitude_error < 0.7 {
        ctrl.plus_x.throttle = throttle as f32;
        VehicleControlStatus::TurningToHover
    } else {
        VehicleControlStatus::Hovering
    };

    ctrl.attitude = attitude;

    (ctrl, status)
}

pub fn position_hold_control_law(
    target: Pose,
    body: &RigidBody,
    vehicle: &Vehicle,
    gravity: DVec2,
) -> (VehicleControl, VehicleControlStatus) {
    if gravity.length() > 0.0 {
        hover_control_law(target.0, gravity, vehicle, body)
    } else {
        zero_gravity_control_law(target.0, target.1, body, vehicle)
    }
}

pub fn velocity_control_law(
    vel: DVec2,
    body: &RigidBody,
    vehicle: &Vehicle,
    gravity: DVec2,
) -> VehicleControl {
    if gravity.length() == 0.0 {
        return VehicleControl::NULLOPT;
    }

    let mut cmd = VehicleControl::NULLOPT;

    let upright_angle = vel.to_angle();
    let velocity_error = vel - body.pv.vel;
    let heading_dir = velocity_error - gravity;
    let target_angle = heading_dir
        .to_angle()
        .clamp(upright_angle - 0.2 * PI_64, upright_angle + 0.2 * PI_64);

    // attitude controller
    // let attitude_error = (body.angle - target_angle).abs();
    let attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);

    let vmag = body.pv.vel.length();
    let vmag_error = vel.length() - vmag;

    let pid = PDCtrl::new(0.3, 30.0);

    let extra_throttle = pid.apply(vmag_error, 0.0);

    let thrust = vehicle.max_forward_thrust();
    let accel = thrust / vehicle.total_mass().to_kg_f64();
    let pct = gravity.length() / accel + extra_throttle;

    cmd.attitude = attitude;
    cmd.plus_x.throttle = pct as f32;

    cmd
}

#[derive(Debug, Clone, Copy)]
pub enum VehicleControlStatus {
    Done,
    WaitingForInput,
    UnderExternalControl,
    ZeroGRendezvous(u32),
    RaisingOrbit,
    StopFalling,
    RaisingPeriapsis(i32),
    ExecutingLaunchProgram,
    CoastingToApoapsis(i32),
    Idling,
    Whatever,
    InProgress,
    TurningToHover,
    Hovering,
}

fn to_int_percent(x: f64) -> i32 {
    (100.0 * x).round() as i32
}

pub fn enter_orbit_control_law(
    planet: &Body,
    body: &RigidBody,
    vehicle: &Vehicle,
    orbit: Option<&SparseOrbit>,
    target_altitude: f64,
) -> (VehicleControl, VehicleControlStatus) {
    let target_apoapsis = target_altitude + 10000.0;
    let target_periapsis = target_altitude;

    let altitude = body.pv.pos.length() - planet.radius;
    let vertical = body.pv.pos.to_angle();
    let vertical_velocity = body.pv.vel.dot(body.pv.pos.normalize_or_zero());
    let gravity = planet.gravity(body.pv.pos).length();

    let (apoapsis_altitude, periapsis_altitude, circular) = if let Some(orbit) = orbit {
        (
            orbit.apoapsis_r() - planet.radius,
            orbit.periapsis_r() - planet.radius,
            match orbit.class() {
                crate::orbits::OrbitClass::Circular => true,
                crate::orbits::OrbitClass::NearCircular => true,
                _ => false,
            },
        )
    } else {
        (
            kinematic_apoapis(altitude, vertical_velocity, gravity),
            0.0,
            false,
        )
    };

    let att_and_throttle = |target_angle: f64, throttle: f32| {
        let mut cmd = VehicleControl::NULLOPT;
        cmd.attitude = compute_attitude_control(body, target_angle, &vehicle.attitude_controller);
        // let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
        // if angle_error.abs() < 0.1 {
        cmd.plus_x.throttle = throttle;
        // }
        cmd
    };

    let near_ground = altitude < 20_000.0;
    let falling = vertical_velocity < 0.0;
    let periapsis_above_target = periapsis_altitude > target_periapsis;
    let apoapsis_above_target = apoapsis_altitude > target_apoapsis;
    let above_target = altitude > target_altitude;

    let launch_program_target_angle = {
        let start_altitude = 1000.0;
        let end_altitude = 12000.0;
        let s = ((altitude - start_altitude) / end_altitude).clamp(0.0, 1.0);
        let off_vertical = s * PI_64 / 2.0;
        vertical + off_vertical
    };

    let circularization_angle = {
        let off_horizontal_angle = (vertical_velocity / 100.0).clamp(-PI_64 / 5.0, PI_64 / 5.0);
        vertical + PI_64 / 2.0 + off_horizontal_angle
    };

    let (cmd, status) = if apoapsis_above_target && periapsis_above_target {
        (VehicleControl::NULLOPT, VehicleControlStatus::Done)
    } else if near_ground && falling {
        (
            att_and_throttle(vertical, 1.0),
            VehicleControlStatus::StopFalling,
        )
    } else if apoapsis_above_target || above_target {
        if !above_target {
            (
                att_and_throttle(body.pv.vel.to_angle(), 0.0),
                VehicleControlStatus::CoastingToApoapsis(to_int_percent(
                    altitude / target_altitude,
                )),
            )
        } else if !periapsis_above_target || !circular {
            (
                att_and_throttle(circularization_angle, 0.2),
                VehicleControlStatus::RaisingPeriapsis(to_int_percent(
                    periapsis_altitude / target_periapsis,
                )),
            )
        } else {
            (VehicleControl::NULLOPT, VehicleControlStatus::InProgress)
        }
    } else {
        let max_accel = vehicle.max_forward_thrust() / vehicle.total_mass().to_kg_f64();
        let target_accel = 16.0;
        let throttle = target_accel / max_accel;
        (
            att_and_throttle(launch_program_target_angle, throttle as f32),
            VehicleControlStatus::ExecutingLaunchProgram,
        )
    };

    (cmd, status)
}

#[derive(Debug, Clone)]
pub enum VehicleControlPolicy {
    Idle,
    External,
    PositionHold(Vec<(DVec2, f64)>),
    LaunchToOrbit(f64),
}

#[derive(Debug, Clone)]
pub struct VehicleController {
    status: VehicleControlStatus,
    mode: VehicleControlPolicy,
}

pub type Pose = (DVec2, f64);

impl VehicleController {
    pub fn idle() -> Self {
        Self {
            status: VehicleControlStatus::Done,
            mode: VehicleControlPolicy::Idle,
        }
    }

    pub fn external() -> Self {
        Self {
            status: VehicleControlStatus::WaitingForInput,
            mode: VehicleControlPolicy::External,
        }
    }

    pub fn launch() -> Self {
        Self {
            status: VehicleControlStatus::InProgress,
            mode: VehicleControlPolicy::LaunchToOrbit(rand(300_000.0, 700_000.0) as f64),
        }
    }

    pub fn position_hold(pose: Pose) -> Self {
        Self {
            status: VehicleControlStatus::InProgress,
            mode: VehicleControlPolicy::PositionHold(vec![pose]),
        }
    }

    pub fn mission(poses: Vec<Pose>) -> Self {
        Self {
            status: VehicleControlStatus::InProgress,
            mode: VehicleControlPolicy::PositionHold(poses),
        }
    }

    pub fn set_status(&mut self, status: VehicleControlStatus) {
        self.status = status;
    }

    pub fn status(&self) -> VehicleControlStatus {
        self.status
    }

    pub fn set_idle(&mut self) {
        self.mode = VehicleControlPolicy::Idle;
    }

    pub fn enqueue_target_pose(&mut self, pose: Pose, clear_queue: bool) {
        if let VehicleControlPolicy::PositionHold(queue) = &mut self.mode {
            if clear_queue {
                queue.clear();
            }
            queue.push(pose);
        } else {
            self.mode = VehicleControlPolicy::PositionHold(vec![pose]);
        }
    }

    pub fn mode(&self) -> &VehicleControlPolicy {
        &self.mode
    }

    pub fn is_idle(&self) -> bool {
        match self.mode {
            VehicleControlPolicy::Idle => true,
            _ => false,
        }
    }

    pub fn get_target_pose(&self) -> Option<Pose> {
        self.get_target_queue().next()
    }

    pub fn get_target_queue(&self) -> impl Iterator<Item = Pose> + use<'_> {
        match &self.mode {
            VehicleControlPolicy::PositionHold(queue) => queue.iter().cloned(),
            _ => [].iter().cloned(),
        }
    }

    pub fn go_to_next_mode(&mut self) {
        self.mode = match self.mode {
            VehicleControlPolicy::Idle => VehicleControlPolicy::External,
            VehicleControlPolicy::External => VehicleControlPolicy::PositionHold(vec![]),
            VehicleControlPolicy::PositionHold(_) => {
                VehicleControlPolicy::LaunchToOrbit(rand(300_000.0, 700_000.0) as f64)
            }
            VehicleControlPolicy::LaunchToOrbit(_) => VehicleControlPolicy::Idle,
        };
    }

    fn mark_target_achieved(&mut self) {
        if let VehicleControlPolicy::PositionHold(queue) = &mut self.mode {
            if queue.len() > 1 {
                queue.remove(0);
            }
        }
    }

    pub fn check_target_achieved(&mut self, body: &RigidBody, ignore_angle: bool) {
        let (pos, angle) = match self.get_target_pose() {
            Some(p) => p,
            None => return,
        };

        let d = pos.distance(body.pv.pos).abs();
        let v = body.pv.vel.length().abs();
        let a = wrap_pi_npi_f64(angle - body.angle).abs();

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
