#![allow(unused)]

use crate::game_version_two::*;

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorWorldPosition(None));
        app.add_systems(Update, (update_mouse_world_pos, draw_cursor_pos));
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct CursorWorldPosition(Option<Vec2>);

impl CursorWorldPosition {
    pub fn get(&self) -> Option<Vec2> {
        self.0
    }
}

fn draw_cursor_pos(
    mut painter: ShapePainter,
    pos: Res<CursorWorldPosition>,
    camera: Single<&Transform, With<Camera>>,
) {
    if let Some(p) = pos.get() {
        painter.reset();
        painter.set_translation(p.extend(100.0));
        painter.set_color(Srgba::gray(0.4).with_alpha(0.6));
        painter.circle(3.0 * camera.scale.x);
    }
}

fn update_mouse_world_pos(
    mut coords: ResMut<CursorWorldPosition>,
    // query to get the window (so we can read the current cursor position)
    window: Single<&Window>,
    // query to get camera transform
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = *camera;

    coords.0 = if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
    {
        Some(world_position)
    } else {
        None
    };
}
