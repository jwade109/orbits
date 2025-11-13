use crate::game_version_two::*;

pub struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            ((update_computers, do_maneuvers).run_if(on_timer(Duration::from_millis(100))),),
        )
        .add_systems(Update, draw_computers);
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct Computer {
    pub on: bool,
    pub status: MachineStatus,
    pub iters: u64,
    pub position_hold: Vec2,
    pub attitude_hold: f32,
    pub vehicle_control: VehicleControl,
    pub control_status: VehicleControlStatus,
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
        painter.reset();
        painter.set_translation(transform.translation().with_z(60.0));
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

        if !computer.on {
            continue;
        }

        let target = computer.position_hold.extend(60.0);

        let pointing =
            transform.translation().xy() + Vec2::from_angle(computer.attitude_hold) * 10.0;

        painter.set_color(TEAL);
        painter.set_translation(Vec3::ZERO);
        painter.line(transform.translation().with_z(60.0), target);
        painter.set_translation(target);
        painter.circle(0.5);

        painter.set_color(GREEN);
        painter.set_translation(Vec3::ZERO);
        painter.line(transform.translation().with_z(60.0), pointing.extend(60.0));
    }
}

fn do_maneuvers(
    grids: Query<(&Children, &SpacecraftGrid)>,
    computers: Query<(&mut Computer, &GlobalTransform, &ChildOf)>,
    mut thrusters: Query<(&mut Thruster, &Transform)>,
) {
    for (mut computer, tf, parent) in computers {
        if !computer.on {
            continue;
        }

        let angle = tf.rotation().to_axis_angle().1 as f64;

        let angular_velocity = match grids.get(parent.0) {
            Ok((_, grid)) => grid.angular_velocity,
            Err(e) => {
                error!(?e);
                0.0
            }
        };

        let body = RigidBody {
            pv: PV::pos(tf.translation().xy()),
            angle,
            angular_velocity,
        };

        let pd = PDCtrl::new(20.0, 50.0);
        let target = computer.attitude_hold as f64;

        let (ctrl, status) = attitude_control_law(target, &pd, &body);

        computer.vehicle_control = ctrl;
        computer.control_status = status;

        if let Ok((children, grid)) = grids.get(parent.0) {
            for child in children {
                if let Ok((mut thruster, transform)) = thrusters.get_mut(*child) {
                    let (thrust, torque) =
                        body_frame_thrust(&thruster, transform, grid.center_of_mass);
                    if torque.abs() > 0.5 && ctrl.attitude.abs() > 0.5 && thruster.is_rcs {
                        thruster.on = torque.signum() as f64 == ctrl.attitude.signum();
                    } else {
                        thruster.on = false;
                    }
                }
            }
        }
    }
}
