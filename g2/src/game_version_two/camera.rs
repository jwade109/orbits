use crate::game_version_two::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (control_camera, track_selected_spacecraft).chain());
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
    cursor: Res<CursorInfo>,
    grids: Query<&GlobalTransform, With<SpacecraftGrid>>,
    parts: Query<&ChildOf, With<PartInstance>>,
    mut camera: Single<&mut Transform, With<Camera>>,
) {
    let id = some_or_return!(cursor.selected);
    let part = ok_or_return!(parts.get(id));
    let grid = ok_or_return!(grids.get(part.0));
    camera.translation = grid.translation();
}
