use crate::game_version_two::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Thruster {
    pub on: bool,
    pub status: MachineStatus,
    pub max_thrust: f32,
    pub is_rcs: bool,
}

impl Thruster {
    pub fn new(max_thrust: f32, is_rcs: bool) -> Self {
        Self {
            on: false,
            status: MachineStatus::Off,
            max_thrust,
            is_rcs,
        }
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
    }

    pub fn current_thrust(&self) -> f32 {
        match (self.on, self.status) {
            (true, MachineStatus::Running) => self.max_thrust,
            _ => 0.0,
        }
    }
}

#[derive(Default)]
pub struct ThrusterPlugin;

impl Plugin for ThrusterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw_thrusters.run_if(in_state(DebugThrusters::Drawn)),
        );
        app.add_systems(FixedUpdate, (consume_fuel, apply_thrust_to_grids));
        app.insert_state(DebugThrusters::Drawn);
    }
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum DebugThrusters {
    Hidden,
    Drawn,
}

fn draw_thrusters(
    mut painter: ShapePainter,
    thrusters: Query<(&GlobalTransform, &Thruster, &PartInstance)>,
) {
    for (location, thruster, part) in &thrusters {
        painter.reset();

        let color = match thruster.on {
            true => Srgba::RED.with_alpha(0.7),
            false => Srgba::GREEN.with_alpha(0.02),
        };

        painter.set_color(color);
        painter.set_translation(location.translation());
        painter.set_rotation(location.rotation());
        let dims = part.prototype().dims_meters();
        painter.translate(-dims.x * Vec2::X.extend(0.0));
        painter.rect(dims);
    }
}

fn consume_fuel(mut thrusters: Query<(&mut Thruster, &mut Inventory)>) {
    for (mut thruster, mut inv) in &mut thrusters {
        thruster.status = if thruster.on {
            if inv.take(Item::H2, 1) {
                MachineStatus::Running
            } else {
                MachineStatus::Starved
            }
        } else {
            MachineStatus::Off
        };
    }
}

pub fn body_frame_thrust(thruster: &Thruster, transform: &Transform, com: Vec2) -> (Vec2, f32) {
    let u = transform.right().xy();
    let location = transform.translation.xy();
    let lever_arm = location - com;
    let thrust = thruster.max_thrust * u;
    let torque = cross2d(lever_arm, thrust);
    (thrust, torque as f32)
}

fn apply_thrust_to_grids(
    thrusters: Query<(&Thruster, &Transform, &ChildOf)>,
    mut grids: Query<&mut SpacecraftGrid>,
) {
    for mut grid in &mut grids {
        grid.body_frame_acceleration = DVec2::ZERO;
        grid.angular_acceleration = 0.0;
    }

    for (thruster, transform, parent) in thrusters {
        if !thruster.status.is_running() {
            continue;
        }

        match grids.get_mut(parent.0) {
            Ok(mut grid) => {
                let (thrust, torque) = body_frame_thrust(thruster, transform, grid.center_of_mass);
                grid.apply_body_frame_thrust(thrust, torque);
            }
            Err(e) => {
                error!(?e);
            }
        }
    }
}
