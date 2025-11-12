use crate::game_version_two::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Thruster {
    pub on: bool,
    pub status: MachineStatus,
    pub max_thrust: f32,
}

impl Thruster {
    pub fn default() -> Self {
        Self {
            on: false,
            status: MachineStatus::Off,
            max_thrust: 40000.0,
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
        app.add_systems(Update, draw_thrusters);
        app.add_systems(FixedUpdate, (consume_fuel, apply_thrust_to_grids));
    }
}

fn draw_thrusters(
    mut painter: ShapePainter,
    thrusters: Query<(&GlobalTransform, &PartInstance), With<Thruster>>,
) {
    for (location, part) in &thrusters {
        painter.reset();
        painter.set_color(Srgba::RED);
        painter.set_translation(location.translation());
        painter.set_rotation(location.rotation());
        painter.hollow = true;
        painter.thickness_type = ThicknessType::Pixels;
        painter.thickness = 3.0;
        let dims = part.prototype().dims_meters();
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

fn apply_thrust_to_grids(
    thrusters: Query<(&Thruster, &Transform, &ChildOf)>,
    mut grids: Query<&mut SpacecraftGrid>,
) {
    for mut grid in &mut grids {
        grid.body_frame_acceleration = DVec2::ZERO;
    }

    for (thruster, transform, parent) in thrusters {
        if !thruster.on {
            continue;
        }

        let u = transform.right().xy();

        match grids.get_mut(parent.0) {
            Ok(mut grid) => {
                grid.apply_body_frame_thrust(thruster.current_thrust() * u);
            }
            Err(e) => {
                error!(?e);
            }
        }
    }
}
