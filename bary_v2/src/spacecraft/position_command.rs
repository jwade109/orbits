use bary_core::prelude::Blueprint;
use bary_v1::z_index::ZOrdering;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;

use crate::{
    Computer, ComputerMode, CursorWorldPosition, PartsResource, SelectedSpacecraft, draw_blueprint,
};

#[derive(Resource, Debug, Default)]
pub struct CursorPositionCommandWidget {
    pos_down: Option<Vec2>,
    pos_up: Option<Vec2>,
}

impl CursorPositionCommandWidget {
    fn get_command(&self) -> Option<PositionHoldCommand> {
        let down = self.pos_down?;
        let up = self.pos_up?;
        let angle = (up - down).to_angle();
        Some(PositionHoldCommand { pos: down, angle })
    }

    fn update_cursor_pos(&mut self, p: Option<Vec2>) {
        if p.is_none() {
            self.pos_down = None;
            self.pos_up = None;
        } else {
            if self.pos_down.is_none() {
                self.pos_down = p;
            }
            self.pos_up = p;
        }
    }

    pub fn isometry(&self) -> Option<Isometry2d> {
        let down = self.pos_down?;
        let up = self.pos_up?;
        let angle = (up - down).to_angle();
        Some(Isometry2d::new(down, angle.into()))
    }

    fn clear(&mut self) {
        self.pos_down = None;
        self.pos_up = None;
    }
}

#[derive(Event, Debug)]
pub struct PositionHoldCommand {
    pos: Vec2,
    angle: f32,
}

pub fn update_position_command_widget_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorWorldPosition>,
    mut widget: ResMut<CursorPositionCommandWidget>,
) {
    let commands_key = KeyCode::KeyE;

    let pos = cursor.get();

    if keys.pressed(commands_key) {
        widget.update_cursor_pos(pos);
    }

    if keys.just_released(commands_key) {
        if let Some(msg) = widget.get_command() {
            info!("Command: {:?}", msg);
            commands.trigger(msg);
        }
    }

    if !keys.pressed(commands_key) {
        widget.clear();
    }
}

pub fn draw_position_command_widget(
    mut painter: ShapePainter,
    widget: Res<CursorPositionCommandWidget>,
    sel: Res<SelectedSpacecraft>,
    blueprints: Query<&Blueprint>,
    mut gizmos: Gizmos,
    parts: Res<PartsResource>,
) {
    let z = ZOrdering::Debug2.as_f32();

    painter.reset();
    painter.set_color(TEAL_300);
    painter.hollow = true;
    painter.thickness = 0.1;

    if let Some(down) = widget.pos_down {
        painter.set_translation(down.extend(z));
        painter.circle(1.0);
        if let Some(up) = widget.pos_up {
            painter.set_translation(Vec3::ZERO.with_z(z));
            painter.line(up.extend(0.0), down.extend(0.0));
        }
    }

    let Some(grid_id) = sel.primary else {
        return;
    };
    let Some(iso) = widget.isometry() else {
        return;
    };
    let Ok(bp) = blueprints.get(grid_id.grid.entity) else {
        return;
    };

    draw_blueprint(&mut gizmos, bp, iso, &parts);
}

pub fn on_position_commands_system(
    command: On<PositionHoldCommand>,
    selected: Res<SelectedSpacecraft>,
    mut computers: Query<&mut Computer>,
) -> Option<()> {
    let id = selected.primary?.part;
    let mut cpu = computers.get_mut(id.entity).ok()?;
    cpu.mode = ComputerMode::PositionHold;
    cpu.position = command.pos;
    cpu.attitude = command.angle;
    cpu.on = true;
    Some(())
}
