use crate::math::*;
use crate::pid::PDCtrl;
use crate::vehicle::*;

#[derive(Default, Debug, Clone, Copy)]
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

#[derive(Default, Debug, Clone, Copy)]
pub struct VehicleControl {
    pub plus_x: ThrustAxisControl,
    pub plus_y: ThrustAxisControl,
    pub neg_x: ThrustAxisControl,
    pub neg_y: ThrustAxisControl,
    pub attitude: f32,
}

impl VehicleControl {
    pub const NULLOPT: VehicleControl = VehicleControl {
        plus_x: ThrustAxisControl::NULLOPT,
        plus_y: ThrustAxisControl::NULLOPT,
        neg_x: ThrustAxisControl::NULLOPT,
        neg_y: ThrustAxisControl::NULLOPT,
        attitude: 0.0,
    };
}

#[derive(Default, Debug, Clone, Copy)]
pub enum VehicleControlPolicy {
    #[default]
    Idle,
    External,
    PositionHold(Vec2),
}

const ATTITUDE_CONTROLLER: PDCtrl = PDCtrl::new(40.0, 60.0);

const VERTICAL_CONTROLLER: PDCtrl = PDCtrl::new(0.2, 1.0);

const HORIZONTAL_CONTROLLER: PDCtrl = PDCtrl::new(0.01, 0.08);

const DOCKING_LINEAR_CONTROLLER: PDCtrl = PDCtrl::new(30.0, 300.0);

fn zero_gravity_control_law(target: Vec2, body: &RigidBody) -> VehicleControl {
    VehicleControl::NULLOPT
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
    let future_alt = kinematic_apoapis(body, gravity.length() as f64) as f32;

    let target = if target.distance(body.pv.pos_f32()) > 250.0 {
        let d = target - body.pv.pos_f32();
        d.normalize_or_zero() * 250.0 + body.pv.pos_f32()
    } else {
        target
    };

    let horizontal_control =
        HORIZONTAL_CONTROLLER.apply(target.x - body.pv.pos.x as f32, body.pv.vel.x as f32);

    // attitude controller
    let target_angle = PI * 0.5 - horizontal_control.clamp(-PI / 6.0, PI / 6.0);
    let attitude_error = (body.angle - target_angle).abs();
    let attitude = compute_attitude_control(body, target_angle, &ATTITUDE_CONTROLLER);

    let thrust = vehicle.max_thrust_along_heading(0.0, false);
    let accel = thrust / vehicle.current_mass().to_kg_f32();
    let pct = gravity.length() / accel;

    // vertical controller
    let error = VERTICAL_CONTROLLER.apply(target.y - future_alt, body.pv.vel.y as f32);

    let throttle = pct + error;

    let mut ctrl = VehicleControl::NULLOPT;

    if attitude_error < 0.7 {
        ctrl.plus_x.throttle = throttle;
    }

    ctrl.attitude = attitude;

    ctrl
}

pub fn position_hold_control_law(
    target: Vec2,
    body: &RigidBody,
    vehicle: &Vehicle,
    gravity: Vec2,
) -> VehicleControl {
    if gravity.length() > 0.0 {
        hover_control_law(target, gravity, vehicle, body)
    } else {
        zero_gravity_control_law(target, body)
    }
}
