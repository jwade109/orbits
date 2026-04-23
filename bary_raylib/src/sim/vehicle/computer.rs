use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::Components,
    sim::{Part, VehicleGrid},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Instruction {
    Ctrl(VehicleControl),
    HoldPosition(Isometry2d),
    HoldAttitude(Angle),
    PointAt(Vec2),
    Drift,
    DeltaV(Vec2),
}

impl Instruction {
    pub fn rcs_left() -> Self {
        Self::Ctrl(VehicleControl::rcs(false, true, false, false))
    }

    pub fn rcs_right() -> Self {
        Self::Ctrl(VehicleControl::rcs(false, false, true, false))
    }

    pub fn rcs_forward() -> Self {
        Self::Ctrl(VehicleControl::rcs(true, false, false, false))
    }

    pub fn rcs_backward() -> Self {
        Self::Ctrl(VehicleControl::rcs(false, false, false, true))
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Ctrl(_vehicle_control) => {
                write!(f, "CTRL")
            }
            Instruction::HoldPosition(iso) => {
                write!(
                    f,
                    "HP {:0.2} {:0.2} {:0.2}",
                    iso.translation.x, iso.translation.y, iso.rotation
                )
            }
            Instruction::Drift => {
                write!(f, "DRIFT")
            }
            Instruction::HoldAttitude(angle) => {
                write!(f, "HDG {:0.2}", angle)
            }
            Instruction::PointAt(pos) => {
                write!(f, "POINT {:0.2}", pos)
            }
            Instruction::DeltaV(dv) => {
                write!(f, "DV {:0.2}", dv)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TimedInstruction {
    pub duration: Option<u64>,
    pub instruction: Instruction,
}

impl TimedInstruction {
    pub fn perp(instruction: Instruction) -> Self {
        Self {
            duration: None,
            instruction,
        }
    }

    pub fn timed(ticks: u64, instruction: Instruction) -> Self {
        Self {
            duration: Some(ticks),
            instruction,
        }
    }

    pub fn tick(&mut self) {
        if let Some(dur) = &mut self.duration {
            if *dur > 0 {
                *dur -= 1;
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.duration.map(|d| d == 0).unwrap_or(false)
    }
}

impl std::fmt::Display for TimedInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ticks) = self.duration {
            write!(f, "{}, for {} ticks", self.instruction, ticks)
        } else {
            write!(f, "{}", self.instruction)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Computer {
    pub on: bool,
    pub status: MachineStatus,
    pub ticks_this_cycle: u32,
    pub ticks_per_cycle: u32,
    pub fired_this_tick: bool,
    pub iters: u64,
    pub vehicle_control: VehicleControl,
    pub prototype: Ent,
    pub command_queue: Vec<TimedInstruction>,
}

impl Computer {
    pub fn new(prototype: Ent) -> Self {
        Self {
            on: true,
            status: MachineStatus::Off,
            ticks_this_cycle: 0,
            ticks_per_cycle: 5,
            fired_this_tick: false,
            iters: 0,
            vehicle_control: VehicleControl::NULLOPT,
            prototype,
            command_queue: vec![
                // TimedInstruction::timed(300, Instruction::rcs_forward()),
                // TimedInstruction::timed(150, Instruction::rcs_right()),
                // TimedInstruction::timed(150, Instruction::rcs_left()),
                // TimedInstruction::timed(80, Instruction::rcs_backward()),
                // TimedInstruction::timed(20000, Instruction::Drift),
                // TimedInstruction::timed(20000, Instruction::HoldPosition((0.0, 0.0, 0.0).into())),
                // TimedInstruction::perp(Instruction::HoldPosition((100.0, 100.0, 0.0).into())),
            ],
        }
    }

    pub fn tick_forward(&mut self) {
        self.status = match self.on {
            true => MachineStatus::Running,
            false => MachineStatus::Off,
        };

        if self.on {
            self.ticks_this_cycle += 1;
            self.fired_this_tick = self.ticks_this_cycle == self.ticks_per_cycle;
            if self.fired_this_tick {
                self.ticks_this_cycle = 0;
                self.iters += 1;
            }
            if let Some(ins) = self.command_queue.first_mut() {
                ins.tick();
                if ins.is_complete() {
                    self.command_queue.remove(0);
                }
            }
        } else {
            self.fired_this_tick = false;
        }
    }

    pub fn current_instruction(&self) -> Option<Instruction> {
        let cmd = self.command_queue.first()?;
        Some(cmd.instruction)
    }

    pub fn current_instruction_mut(&mut self) -> Option<&mut Instruction> {
        let cmd = self.command_queue.first_mut()?;
        Some(&mut cmd.instruction)
    }

    pub fn current_control(&self) -> Option<VehicleControl> {
        let cmd = self.command_queue.first()?;
        if let Instruction::Ctrl(ctrl) = cmd.instruction {
            Some(ctrl)
        } else if let Instruction::Drift = cmd.instruction {
            Some(VehicleControl::NULLOPT)
        } else {
            None
        }
    }

    pub fn current_angle(&self) -> Option<Angle> {
        if let Some(wp) = self.current_waypoint() {
            return Some(Angle::radians(wp.rotation));
        }
        let cmd = self.command_queue.first()?;

        match cmd.instruction {
            Instruction::HoldAttitude(hdg) => Some(hdg),
            Instruction::HoldPosition(iso) => Some(Angle::radians(iso.rotation)),
            _ => None,
        }
    }

    pub fn delta_v(&self) -> Option<Vec2> {
        let cmd = self.current_instruction()?;
        match cmd {
            Instruction::DeltaV(dv) => Some(dv),
            _ => None,
        }
    }

    pub fn delta_v_mut(&mut self) -> Option<&mut Vec2> {
        let cmd = self.current_instruction_mut()?;
        match cmd {
            Instruction::DeltaV(dv) => Some(dv),
            _ => None,
        }
    }

    pub fn current_waypoint(&self) -> Option<Isometry2d> {
        let cmd = self.command_queue.first()?;
        if let Instruction::HoldPosition(wp) = cmd.instruction {
            Some(wp)
        } else {
            None
        }
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}

pub fn sys_update_computers(
    computers: &mut Components<Computer>,
    parts: &Components<Part>,
    grids: &Components<VehicleGrid>,
) {
    for (cpu_id, computer) in computers.iter_mut() {
        computer.tick_forward();

        if !computer.fired_this_tick {
            continue;
        }

        if let Some(ctrl) = computer.current_control() {
            computer.vehicle_control = ctrl;
        } else if let Some(target_pose) = computer.current_waypoint() {
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };

            let pose = grid.particle_location;

            let target = PV::from_f64(target_pose.translation, Vec2::ZERO);
            let actual = PV::from_f64(pose.translation, grid.velocity.translation);

            let body = RigidBody {
                pv: actual,
                angle: pose.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };

            let (ctrl, _status) =
                position_hold_control_law(target, target_pose.rotation as f64, &body, DVec2::ZERO);

            computer.vehicle_control = ctrl;
        } else if let Some(target) = computer.current_angle() {
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };

            let actual = Angle::radians(grid.particle_location.rotation);

            let body = RigidBody {
                pv: PV::ZERO,
                angle: actual.as_rad() as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };

            let pid = PDCtrl::new(20.0, 50.0);

            let (ctrl, _status) = attitude_control_law(target.as_rad() as f64, &pid, &body);

            computer.vehicle_control = ctrl;
        } else if let Some(dv) = computer.delta_v_mut() {
            *dv -= Vec2::splat(0.1);
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };
            let target = dv.to_angle();
            let pid = PDCtrl::new(20.0, 50.0);
            let body = RigidBody {
                pv: PV::ZERO,
                angle: grid.particle_location.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };
            let (ctrl, _status) = attitude_control_law(target as f64, &pid, &body);

            computer.vehicle_control = ctrl;
        }
    }
}

// pub fn do_maneuvers(
//     grids: Components<VehicleGrid>,
//     computers: Query<(&mut Computer, &GlobalTransform, &ChildOf)>,
//     mut thrusters: Query<(&mut Thruster, &Transform, &PartInstance)>,
//     manual: Res<ManualControl>,
// ) {
//     for (mut computer, _, parent) in computers {
//         if !computer.fired_this_tick {
//             continue;
//         }

//         let (_, grid, transform) = ok_or_continue!(grids.get(parent.0));

//         let (yaw, _pitch, _roll) = transform.rotation.to_euler(EulerRot::ZYX);

//         let body = RigidBody {
//             pv: PV::from_f64(transform.translation.xy(), grid.velocity),
//             angle: yaw as f64,
//             angular_velocity: grid.angular_velocity,
//         };

//         let pd = PDCtrl::new(20.0, 50.0);

//         let (ctrl, status) = match computer.mode {
//             ComputerMode::Idle => (VehicleControl::NULLOPT, VehicleControlStatus::Idling),
//             ComputerMode::Manual => (manual.0, VehicleControlStatus::UnderExternalControl),
//             ComputerMode::AttitudeHold => {
//                 attitude_control_law(computer.attitude as f64, &pd, &body)
//             }
//             ComputerMode::VelocityHold => zero_gravity_velocity_control_law(
//                 computer.velocity.as_dvec2(),
//                 computer.attitude as f64,
//                 &body,
//                 &pd,
//             ),
//             ComputerMode::PositionHold => zero_gravity_control_law(
//                 PV::pos(computer.position),
//                 computer.attitude as f64,
//                 &body,
//                 &pd,
//             ),
//         };

//         computer.vehicle_control = ctrl;
//         computer.control_status = status;

//         if let Ok((children, grid, _)) = grids.get(parent.0) {
//             for child in children {
//                 if let Ok((mut thruster, transform, part)) = thrusters.get_mut(*child) {
//                     let tac = match part.rotation() {
//                         Rotation::East => ctrl.plus_x,
//                         Rotation::North => ctrl.neg_y,
//                         Rotation::West => ctrl.neg_x,
//                         Rotation::South => ctrl.plus_y,
//                     };
//                     let (_thrust, torque) =
//                         body_frame_thrust(&thruster, transform, grid.center_of_mass);
//                     if thruster.is_rcs {
//                         let can_torque = torque.abs() > 0.5 && ctrl.attitude.abs() > 0.5;
//                         let is_torque =
//                             can_torque && torque.signum() as f64 == ctrl.attitude.signum();
//                         let is_linear = tac.throttle > 0.0 && tac.use_rcs;
//                         thruster.on = is_linear || is_torque;
//                     } else {
//                         thruster.on = !tac.use_rcs && tac.throttle > 0.0;
//                     }
//                 }
//             }
//         }
//     }
// }

// fn keyboard_control_law(keys: &ButtonInput<KeyCode>) -> VehicleControl {
//     let mut ctrl = VehicleControl::NULLOPT;

//     let docking_mode = keys.pressed(KeyCode::ControlLeft);

//     if docking_mode {
//         ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
//         ctrl.plus_y.throttle = keys.pressed(KeyCode::ArrowRight) as u8 as f32;
//         ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;
//         ctrl.neg_y.throttle = keys.pressed(KeyCode::ArrowLeft) as u8 as f32;
//     } else {
//         ctrl.plus_x.throttle = keys.pressed(KeyCode::ArrowUp) as u8 as f32;
//         ctrl.neg_x.throttle = keys.pressed(KeyCode::ArrowDown) as u8 as f32;

//         ctrl.attitude = if keys.pressed(KeyCode::ArrowLeft) {
//             10.0
//         } else if keys.pressed(KeyCode::ArrowRight) {
//             -10.0
//         } else {
//             0.0
//         };
//     }

//     ctrl.plus_x.use_rcs = docking_mode;
//     ctrl.plus_y.use_rcs = docking_mode;
//     ctrl.neg_x.use_rcs = docking_mode;
//     ctrl.neg_y.use_rcs = docking_mode;

//     ctrl
// }

// fn human_control(keys: Res<ButtonInput<KeyCode>>, mut ctrl: ResMut<ManualControl>) {
//     ctrl.0 = keyboard_control_law(&keys);
// }

// fn handle_hold_here_commands(
//     command: On<HoldHereCommand>,
//     grids: Query<&GlobalTransform, With<SpacecraftGrid>>,
//     mut computers: Query<(&mut Computer, &ChildOf)>,
// ) -> Result {
//     println!("Hold here issued: {:?}", command);

//     let (mut computer, parent) = computers.get_mut(command.0)?;

//     println!("Parent vehicle is {}", parent.0);

//     let transform = grids.get(parent.0)?;
//     let pos = transform.translation();
//     let (yaw, _, _) = transform.rotation().to_euler(EulerRot::ZYX);

//     computer.position = pos.xy();
//     computer.attitude = yaw;
//     computer.mode = ComputerMode::PositionHold;
//     computer.on = true;

//     Ok(())
// }
