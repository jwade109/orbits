use crate::game_version_two::*;

pub struct ComputerPlugin;

impl Plugin for ComputerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                (update_computers, do_random_maneuvers)
                    .run_if(on_timer(Duration::from_millis(100))),
            ),
        );
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct Computer {
    pub on: bool,
    pub status: MachineStatus,
    pub iters: u64,
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

fn do_random_maneuvers(
    grids: Query<&Children, With<SpacecraftGrid>>,
    computers: Query<(&Computer, &ChildOf)>,
    mut thrusters: Query<&mut Thruster>,
) {
    for (computer, parent) in computers {
        if !computer.on {
            continue;
        }
        if let Ok(grid) = grids.get(parent.0) {
            for child in grid {
                if let Ok(mut thruster) = thrusters.get_mut(*child) {
                    let r = rand(0.0, 1.0);
                    thruster.on = match (r, thruster.on) {
                        (0.0..0.3, true) => false,
                        (0.0..0.05, false) => true,
                        _ => thruster.on,
                    };
                }
            }
        }
    }
}
