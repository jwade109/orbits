use bary_core::prelude::*;

use crate::*;

pub struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, human_control);
        app.add_systems(PostUpdate, draw_computers);
        app.insert_resource(ManualControl::default());

        app.add_observer(handle_hold_here_commands);
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct Computer {
    pub on: bool,
    pub status: MachineStatus,
    pub ticks_this_cycle: u32,
    pub ticks_per_cycle: u32,
    pub fired_this_tick: bool,
    pub iters: u64,
    pub mode: ComputerMode,
    pub attitude: f32,
    pub velocity: Vec2,
    pub position: Vec2,
    pub vehicle_control: VehicleControl,
    pub control_status: VehicleControlStatus,
}

#[derive(Sequence, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputerMode {
    #[default]
    Idle,
    Manual,
    AttitudeHold,
    VelocityHold,
    PositionHold,
}

impl ComputerMode {
    pub fn needs_attitude(&self) -> bool {
        match self {
            ComputerMode::Idle => false,
            ComputerMode::Manual => false,
            ComputerMode::AttitudeHold => true,
            ComputerMode::VelocityHold => true,
            ComputerMode::PositionHold => true,
        }
    }

    pub fn needs_velocity(&self) -> bool {
        match self {
            ComputerMode::Idle => false,
            ComputerMode::Manual => false,
            ComputerMode::AttitudeHold => false,
            ComputerMode::VelocityHold => true,
            ComputerMode::PositionHold => false,
        }
    }

    pub fn needs_position(&self) -> bool {
        match self {
            ComputerMode::Idle => false,
            ComputerMode::Manual => false,
            ComputerMode::AttitudeHold => false,
            ComputerMode::VelocityHold => false,
            ComputerMode::PositionHold => true,
        }
    }

    pub fn needs_isometry(&self) -> bool {
        self.needs_attitude() && self.needs_position()
    }
}

impl Computer {
    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}

pub fn update_computers(computers: Query<&mut Computer>) {
    for mut computer in computers {
        computer.status = match computer.on {
            true => MachineStatus::Running,
            false => MachineStatus::Off,
        };
        if computer.on {
            computer.ticks_this_cycle += 1;
            computer.fired_this_tick = computer.ticks_this_cycle == computer.ticks_per_cycle;
            if computer.fired_this_tick {
                computer.ticks_this_cycle = 0;
                computer.iters += 1;
            }
        } else {
            computer.fired_this_tick = false;
        }
    }
}

fn draw_waypoint(
    painter: &mut ShapePainter,
    pos: Vec3,
    attitude: Option<f32>,
    color: Srgba,
    scale: f32,
) {
    painter.reset();
    painter.set_translation(pos);
    painter.set_color(color);
    painter.hollow = true;
    painter.thickness_type = ThicknessType::Pixels;
    painter.thickness = 3.0;
    painter.circle(16.0 * scale);
    painter.hollow = false;
    painter.circle(4.0 * scale);

    if let Some(attitude) = attitude {
        let pointing = Vec2::from_angle(attitude) * 75.0 * scale;
        painter.reset();
        painter.set_color(color);
        painter.set_translation(pos);
        painter.thickness_type = ThicknessType::Pixels;
        painter.thickness = 3.0;
        painter.line(Vec3::ZERO, pointing.extend(pos.z));
    }
}

fn draw_computers(
    mut painter: ShapePainter,
    blueprints: Query<&Blueprint>,
    computers: Query<(&Computer, &GlobalTransform, &ChildOf)>,
    camera: Single<&Transform, With<Camera>>,
    parts: Res<PartsResource>,
    mut gizmos: Gizmos,
) {
    for (computer, transform, grid_id) in computers {
        if !computer.on {
            continue;
        }

        const COMPUTER_AXIS_Z: f32 = 120.0;

        {
            // axes
            painter.reset();
            painter.set_translation(transform.translation().with_z(COMPUTER_AXIS_Z));
            painter.set_rotation(transform.rotation());
            painter.hollow = true;
            painter.thickness_type = ThicknessType::Pixels;
            painter.thickness = 3.0;
            painter.set_color(YELLOW.with_alpha(0.3));
            painter.circle(0.3);
            painter.set_color(RED);
            painter.line(Vec3::ZERO, Vec3::X * 2.0);
            painter.set_color(GREEN);
            painter.line(Vec3::ZERO, Vec3::Y * 2.0);
        }

        let z = 60.0;

        if computer.mode.needs_position() {
            draw_waypoint(
                &mut painter,
                computer.position.extend(z),
                computer.mode.needs_attitude().then(|| computer.attitude),
                LIME,
                camera.scale.x,
            );
        } else if computer.mode.needs_attitude() {
            draw_waypoint(
                &mut painter,
                transform.translation().with_z(COMPUTER_AXIS_Z),
                Some(computer.attitude),
                LIME,
                camera.scale.x,
            );
        }

        if computer.mode.needs_isometry() {
            let iso = Isometry2d::new(computer.position, computer.attitude.into());
            let Ok(bp) = blueprints.get(grid_id.0) else {
                continue;
            };
            draw_blueprint(&mut gizmos, bp, iso, &parts);
        }
    }
}

pub fn do_maneuvers(
    grids: Query<(&Children, &SpacecraftGrid, &Transform)>,
    computers: Query<(&mut Computer, &GlobalTransform, &ChildOf)>,
    mut thrusters: Query<(&mut Thruster, &Transform, &PartInstance)>,
    manual: Res<ManualControl>,
) {
    for (mut computer, _, parent) in computers {
        if !computer.fired_this_tick {
            continue;
        }

        let (_, grid, transform) = ok_or_continue!(grids.get(parent.0));

        let (yaw, _pitch, _roll) = transform.rotation.to_euler(EulerRot::ZYX);

        let body = RigidBody {
            pv: PV::from_f64(transform.translation.xy(), grid.velocity),
            angle: yaw as f64,
            angular_velocity: grid.angular_velocity,
        };

        let pd = PDCtrl::new(20.0, 50.0);

        let (ctrl, status) = match computer.mode {
            ComputerMode::Idle => (VehicleControl::NULLOPT, VehicleControlStatus::Idling),
            ComputerMode::Manual => (manual.0, VehicleControlStatus::UnderExternalControl),
            ComputerMode::AttitudeHold => {
                attitude_control_law(computer.attitude as f64, &pd, &body)
            }
            ComputerMode::VelocityHold => zero_gravity_velocity_control_law(
                computer.velocity.as_dvec2(),
                computer.attitude as f64,
                &body,
                &pd,
            ),
            ComputerMode::PositionHold => zero_gravity_control_law(
                PV::pos(computer.position),
                computer.attitude as f64,
                &body,
                &pd,
            ),
        };

        computer.vehicle_control = ctrl;
        computer.control_status = status;

        if let Ok((children, grid, _)) = grids.get(parent.0) {
            for child in children {
                if let Ok((mut thruster, transform, part)) = thrusters.get_mut(*child) {
                    let tac = match part.rotation() {
                        Rotation::East => ctrl.plus_x,
                        Rotation::North => ctrl.neg_y,
                        Rotation::West => ctrl.neg_x,
                        Rotation::South => ctrl.plus_y,
                    };
                    let (_thrust, torque) =
                        body_frame_thrust(&thruster, transform, grid.center_of_mass);
                    if thruster.is_rcs {
                        let can_torque = torque.abs() > 0.5 && ctrl.attitude.abs() > 0.5;
                        let is_torque =
                            can_torque && torque.signum() as f64 == ctrl.attitude.signum();
                        let is_linear = tac.throttle > 0.0 && tac.use_rcs;
                        thruster.on = is_linear || is_torque;
                    } else {
                        thruster.on = !tac.use_rcs && tac.throttle > 0.0;
                    }
                }
            }
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct ManualControl(VehicleControl);

fn keyboard_control_law(keys: &ButtonInput<KeyCode>) -> VehicleControl {
    let mut ctrl = VehicleControl::NULLOPT;

    let docking_mode = keys.pressed(KeyCode::ControlLeft);

    if docking_mode {
        ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
        ctrl.plus_y.throttle = keys.pressed(KeyCode::ArrowRight) as u8 as f32;
        ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;
        ctrl.neg_y.throttle = keys.pressed(KeyCode::ArrowLeft) as u8 as f32;
    } else {
        ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
        ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;

        ctrl.attitude = if keys.pressed(KeyCode::ArrowLeft) {
            10.0
        } else if keys.pressed(KeyCode::ArrowRight) {
            -10.0
        } else {
            0.0
        };
    }

    ctrl.plus_x.use_rcs = docking_mode;
    ctrl.plus_y.use_rcs = docking_mode;
    ctrl.neg_x.use_rcs = docking_mode;
    ctrl.neg_y.use_rcs = docking_mode;

    ctrl
}

fn human_control(keys: Res<ButtonInput<KeyCode>>, mut ctrl: ResMut<ManualControl>) {
    ctrl.0 = keyboard_control_law(&keys);
}

fn handle_hold_here_commands(
    command: On<HoldHereCommand>,
    grids: Query<&GlobalTransform, With<SpacecraftGrid>>,
    mut computers: Query<(&mut Computer, &ChildOf)>,
) -> Result {
    info!("Hold here issued: {:?}", command);

    let (mut computer, parent) = computers.get_mut(command.0)?;

    info!("Parent vehicle is {}", parent.0);

    let transform = grids.get(parent.0)?;
    let pos = transform.translation();
    let (yaw, _, _) = transform.rotation().to_euler(EulerRot::ZYX);

    computer.position = pos.xy();
    computer.attitude = yaw;
    computer.mode = ComputerMode::PositionHold;
    computer.on = true;

    Ok(())
}
