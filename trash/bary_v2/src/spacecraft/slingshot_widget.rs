use bary_core::prelude::Blueprint;
use bary_v1::z_index::ZOrdering;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;

use crate::{
    Computer, ComputerMode, CursorWorldPosition, DeltaVelocity, PartsResource, SelectedSpacecraft,
    SpacecraftGrid, SpawnAnimText, draw_blueprint,
};

#[derive(Resource, Debug, Default)]
pub struct SlingshotWidget {
    pos_down: Option<Vec2>,
    pos_up: Option<Vec2>,
}

impl SlingshotWidget {
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

    pub fn delta(&self) -> Option<Vec2> {
        let down = self.pos_down?;
        let up = self.pos_up?;
        Some(up - down)
    }

    fn clear(&mut self) {
        self.pos_down = None;
        self.pos_up = None;
    }
}

pub fn update_slingshot_widget_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorWorldPosition>,
    mut widget: ResMut<SlingshotWidget>,
    sel: Res<SelectedSpacecraft>,
) {
    let commands_key = KeyCode::KeyQ;

    let pos = cursor.get();

    if keys.pressed(commands_key) {
        widget.update_cursor_pos(pos);
    }

    if keys.just_released(commands_key) {
        if let Some(delta_velocity) = widget.delta() {
            if let Some(primary) = sel.primary {
                commands.trigger(DeltaVelocity {
                    target_grid: primary.grid.entity,
                    delta_velocity,
                });
            }
        }
    }

    if !keys.pressed(commands_key) {
        widget.clear();
    }
}

pub fn draw_slingshot_widget(mut painter: ShapePainter, widget: Res<SlingshotWidget>) {
    let z = ZOrdering::Debug2.as_f32();

    painter.reset();
    painter.set_color(RED_400);
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
}

pub fn on_delta_v_observer(
    command: On<DeltaVelocity>,
    mut commands: Commands,
    mut grids: Query<&mut SpacecraftGrid>,
) -> Result {
    let mut grid = grids.get_mut(command.target_grid)?;
    grid.velocity += command.delta_velocity.as_dvec2();
    let s = format!("dv: {:0.2}", command.delta_velocity);
    commands.write_message(SpawnAnimText::new(s));
    Ok(())
}
