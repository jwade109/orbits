use crate::game_version_two::*;

pub struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            ((update_computers, do_maneuvers).run_if(on_timer(Duration::from_millis(100))),),
        )
        .add_systems(Update, (draw_computers, human_control))
        .insert_resource(ManualControl::default());
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct Computer {
    pub on: bool,
    pub status: MachineStatus,
    pub iters: u64,
    pub mode: ComputerMode,
    pub attitude: f32,
    pub position: Vec2,
    pub vehicle_control: VehicleControl,
    pub control_status: VehicleControlStatus,
}

#[derive(Sequence, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputerMode {
    #[default]
    None,
    Manual,
    AttitudeHold,
    PositionHold,
}

impl Computer {
    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}

fn update_computers(computers: Query<&mut Computer>) {
    for mut computer in computers {
        computer.status = match computer.on {
            true => MachineStatus::Running,
            false => MachineStatus::Off,
        };
        if computer.on {
            computer.iters += 1;
        }
    }
}

fn draw_computers(mut painter: ShapePainter, computers: Query<(&Computer, &GlobalTransform)>) {
    for (computer, transform) in computers {
        if !computer.on {
            continue;
        }

        painter.reset();
        painter.set_translation(transform.translation().with_z(60.0));
        painter.set_rotation(transform.rotation());

        let color = if computer.on {
            YELLOW.with_alpha(0.3)
        } else {
            GRAY.with_alpha(0.3)
        };

        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.thickness = 3.0;
        painter.set_color(color);
        painter.circle(0.3);
        painter.set_color(RED);
        painter.line(Vec3::ZERO, Vec3::X * 2.0);
        painter.set_color(GREEN);
        painter.line(Vec3::ZERO, Vec3::Y * 2.0);

        let pointing = transform.translation().xy() + Vec2::from_angle(computer.attitude) * 10.0;

        // painter.set_color(TEAL);
        // painter.set_translation(Vec3::ZERO);
        // painter.line(transform.translation().with_z(60.0), target);
        // painter.set_translation(target);
        // painter.circle(0.5);

        painter.reset();
        painter.set_color(TEAL);
        painter.set_translation(Vec3::ZERO);
        painter.line(transform.translation().with_z(60.0), pointing.extend(60.0));
    }
}

fn do_maneuvers(
    grids: Query<(&Children, &SpacecraftGrid)>,
    computers: Query<(&mut Computer, &GlobalTransform, &ChildOf)>,
    mut thrusters: Query<(&mut Thruster, &Transform, &PartInstance)>,
    manual: Res<ManualControl>,
) {
    for (mut computer, tf, parent) in computers {
        if !computer.on {
            continue;
        }

        let (yaw, _pitch, _roll) = tf.rotation().to_euler(EulerRot::ZYX);

        let angular_velocity = match grids.get(parent.0) {
            Ok((_, grid)) => grid.angular_velocity,
            Err(e) => {
                error!(?e);
                0.0
            }
        };

        let body = RigidBody {
            pv: PV::pos(tf.translation().xy()),
            angle: yaw as f64,
            angular_velocity,
        };

        let pd = PDCtrl::new(20.0, 50.0);

        let (ctrl, status) = match computer.mode {
            ComputerMode::None => {
                continue;
            }
            ComputerMode::Manual => (manual.0, VehicleControlStatus::UnderExternalControl),
            _ => attitude_control_law(computer.attitude as f64, &pd, &body),
        };

        computer.vehicle_control = ctrl;
        computer.control_status = status;

        if let Ok((children, grid)) = grids.get(parent.0) {
            for child in children {
                if let Ok((mut thruster, transform, part)) = thrusters.get_mut(*child) {
                    let (thrust, torque) =
                        body_frame_thrust(&thruster, transform, grid.center_of_mass);
                    if torque.abs() > 0.5 && ctrl.attitude.abs() > 0.5 && thruster.is_rcs {
                        thruster.on = torque.signum() as f64 == ctrl.attitude.signum();
                    } else if !thruster.is_rcs {
                        let unit = transform.local_x().round().as_i8vec3();
                        let tac = match part.rotation() {
                            Rotation::East => ctrl.plus_x,
                            Rotation::North => ctrl.neg_y,
                            Rotation::West => ctrl.neg_x,
                            Rotation::South => ctrl.plus_y,
                        };
                        thruster.on = tac.throttle > 0.0;
                    } else {
                        thruster.on = false;
                    }
                }
            }
        }
    }
}

#[derive(Resource, Debug, Default)]
struct ManualControl(VehicleControl);

fn keyboard_control_law(keys: &ButtonInput<KeyCode>) -> VehicleControl {
    let mut ctrl = VehicleControl::NULLOPT;

    let docking_mode = keys.pressed(KeyCode::ControlLeft);

    if docking_mode {
        ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
        ctrl.plus_y.throttle = keys.pressed(KeyCode::ArrowLeft) as u8 as f32;
        ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;
        ctrl.neg_y.throttle = keys.pressed(KeyCode::ArrowRight) as u8 as f32;
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
