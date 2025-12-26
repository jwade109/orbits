use crate::game_version_two::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (control_camera, track_selected_spacecraft).chain())
            // update_mouse_world_pos
            .add_systems(PostUpdate, update_mouse_world_pos.in_set(Sets::PostPhysics))
            // draw the mouse cursor
            .add_systems(PostUpdate, draw_cursor_pos.in_set(Sets::Draw))
            .insert_resource(CursorWorldPosition::default());
    }
}

fn control_camera(
    mut camera: Single<&mut Transform, With<Camera>>,
    key: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<MouseWheel>,
) {
    let speed = 9.0 * camera.scale.x;

    if key.pressed(KeyCode::KeyW) {
        camera.translation.y += speed;
    }
    if key.pressed(KeyCode::KeyS) {
        camera.translation.y -= speed;
    }
    if key.pressed(KeyCode::KeyA) {
        camera.translation.x -= speed;
    }
    if key.pressed(KeyCode::KeyD) {
        camera.translation.x += speed;
    }
    if key.pressed(KeyCode::Equal) {
        camera.scale /= 1.02;
    }
    if key.pressed(KeyCode::Minus) {
        camera.scale *= 1.02;
    }

    use bevy::input::mouse::MouseScrollUnit;

    for ev in scroll.read() {
        if ev.y > 0.0 {
            camera.scale /= 1.15;
        } else {
            camera.scale *= 1.15;
        }

        camera.scale.z = 1.0;
    }
}

fn track_selected_spacecraft(
    cursor: Res<SelectedSpacecraft>,
    grids: Query<&GlobalTransform, With<SpacecraftGrid>>,
    parts: Query<&ChildOf, With<PartInstance>>,
    settings: Res<Settings>,
    mut camera: Single<&mut Transform, With<Camera>>,
) {
    if !settings.follow_selected {
        return;
    }

    let id = some_or_return!(cursor.selected);
    let part = ok_or_return!(parts.get(id));
    let grid = ok_or_return!(grids.get(part.0));
    camera.translation = grid.translation();
}

#[derive(Resource, Default, Debug)]
pub struct CursorWorldPosition {
    pos: Option<Vec2>,
    pub on_egui: bool,
}

impl CursorWorldPosition {
    pub fn get(&self) -> Option<Vec2> {
        (!self.on_egui).then(|| self.pos).flatten()
    }

    pub fn get_anyway(&self) -> Option<Vec2> {
        self.pos
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
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = *camera;

    coords.pos = if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
    {
        Some(world_position)
    } else {
        None
    };
}
