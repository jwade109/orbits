use crate::{
    system_sets::{DrawSet, SimulationSet},
    *,
};
use bary_core::prelude::*;
use std::time::Duration;

#[derive(Component, Debug, Clone, Copy)]
pub struct Thruster {
    pub on: bool,
    pub status: MachineStatus,
    pub max_thrust: f32,
    pub is_rcs: bool,
    pub burn_time: Duration,
}

impl Thruster {
    pub fn new(max_thrust: f32, is_rcs: bool) -> Self {
        Self {
            on: false,
            status: MachineStatus::Off,
            max_thrust,
            is_rcs,
            burn_time: Duration::ZERO,
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

pub fn body_frame_thrust(thruster: &Thruster, transform: &Transform, com: Vec2) -> (Vec2, f32) {
    let u = transform.right().xy();
    let location = transform.translation.xy();
    let lever_arm = location - com;
    let thrust = thruster.max_thrust * u;
    let torque = cross2d(lever_arm, thrust);
    (thrust, torque as f32)
}

// PLUGIN AND SYSTEMS

pub fn thruster_plugin(app: &mut App) {
    app.add_systems(Update, draw_thrusters.in_set(DrawSet));
    app.add_systems(
        SimTick,
        (consume_fuel, apply_thrust_to_grids)
            .chain()
            .in_set(SimulationSet::Thruster),
    );
    app.add_systems(FixedUpdate, update_volume);
}

fn draw_thrusters(
    mut painter: ShapePainter,
    thrusters: Query<(&GlobalTransform, &Thruster, &PartInstance)>,
    settings: Res<Settings>,
) {
    if !settings.draw_thruster_states {
        return;
    }

    for (location, thruster, part) in &thrusters {
        painter.reset();

        let color = match thruster.on {
            true => Srgba::RED.with_alpha(0.7),
            false => Srgba::GREEN.with_alpha(0.02),
        };

        painter.set_color(color);
        painter.set_translation(location.translation());
        painter.set_rotation(location.rotation());
        let dims = part.placement.part_aligned_dims().to_meters();
        painter.translate(-dims.x * Vec2::X.extend(0.0));
        painter.rect(dims);
    }
}

fn consume_fuel(
    mut thrusters: Query<(&mut Thruster, &mut PartContainers)>,
    mut slots: Query<&mut InvSlot>,
    settings: Res<Settings>,
) {
    for (mut thruster, containers) in &mut thrusters {
        // only draw from first container for now
        let container = containers.get(0).expect("Expected at least one container");
        let mut slot = slots.get_mut(*container).expect("Expected a container");

        thruster.status = if thruster.on {
            if settings.infinite_fuel {
                MachineStatus::Running
            } else {
                if slot.take(Item::H2, 1) {
                    MachineStatus::Running
                } else {
                    MachineStatus::Starved
                }
            }
        } else {
            MachineStatus::Off
        };

        if thruster.status == MachineStatus::Running {
            thruster.burn_time += Duration::from_millis(20);
        }
    }
}

fn update_volume(query: Query<(&Thruster, &mut SpatialAudioSink)>) {
    for (thruster, mut sink) in query {
        let target_volume = if thruster.on { 0.5 } else { 0.0 };
        let actual_volume = sink.volume().to_linear();
        let delta = (target_volume - actual_volume).clamp(-0.02, 0.02);
        let new_volume = (actual_volume + delta).clamp(0.0, 0.5);
        sink.set_volume(bevy::audio::Volume::Linear(new_volume));
    }
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
