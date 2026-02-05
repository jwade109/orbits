use crate::system_sets::CameraSet;
use crate::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // control_camera,
                track_followed_entity,
                set_camera_command,
                draw_camera_debug,
                update_mouse_world_pos,
                draw_cursor_pos,
            )
                .in_set(CameraSet),
        );

        app.add_observer(on_follow_event);

        app.add_systems(FixedUpdate, propagate_camera_physics);

        app.insert_resource(CursorWorldPosition::default());

        app.insert_resource(CameraState::default());
    }
}

#[derive(Event)]
pub struct FollowEvent {
    pub entity: Entity,
}

#[derive(Debug, Resource)]
struct CameraState {
    following: Option<Entity>,
    target_pos: Vec2,
    target_scale: f32,
    command: IVec3,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            following: None,
            target_pos: Vec2::ZERO,
            target_scale: 0.1,
            command: IVec3::ZERO,
        }
    }
}

fn on_follow_event(event: On<FollowEvent>, mut state: ResMut<CameraState>) {
    state.following = Some(event.entity);
}

fn draw_camera_debug(
    settings: Res<Settings>,
    mut gizmos: Gizmos,
    state: ResMut<CameraState>,
    camera: Single<&Transform, With<Camera>>,
    transforms: TransformHelper,
) {
    if !settings.draw_camera_debug {
        return;
    }

    gizmos.circle_2d(
        Isometry2d::from_translation(camera.translation.xy()),
        20.0 * camera.scale.x,
        RED,
    );

    gizmos.circle_2d(
        Isometry2d::from_translation(state.target_pos),
        20.0 * state.target_scale,
        TEAL,
    );

    if let Some(entity) = state.following {
        if let Ok(pos) = transforms.compute_global_transform(entity) {
            gizmos.circle_2d(
                Isometry2d::from_translation(pos.translation().xy()),
                35.0 * camera.scale.x,
                YELLOW,
            );
        }
    }
}

fn set_camera_command(
    key: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CameraState>,
    mut scroll: MessageReader<MouseWheel>,
) {
    state.command = IVec3::ZERO;

    if key.pressed(KeyCode::KeyW) {
        state.command.y = 1;
        state.following = None;
    }
    if key.pressed(KeyCode::KeyS) {
        state.command.y = -1;
        state.following = None;
    }
    if key.pressed(KeyCode::KeyA) {
        state.command.x = -1;
        state.following = None;
    }
    if key.pressed(KeyCode::KeyD) {
        state.command.x = 1;
        state.following = None;
    }
    if key.pressed(KeyCode::Equal) {
        state.command.z = 1;
    }
    if key.pressed(KeyCode::Minus) {
        state.command.z = -1;
    }

    use bevy::input::mouse::MouseScrollUnit;

    for ev in scroll.read() {
        if ev.y > 0.0 {
            state.target_scale /= 1.05_f32.powi(3);
        } else {
            state.target_scale *= 1.05_f32.powi(3);
        }
    }
}

fn propagate_camera_physics(
    mut state: ResMut<CameraState>,
    mut camera: Single<&mut Transform, With<Camera>>,
) {
    let speed = 20.0 * camera.scale.x;
    let delta_target_pos = state.command.xy().as_vec2() * speed;
    state.target_pos += delta_target_pos;

    let delta = state.target_pos - camera.translation.xy();

    let approach_scalar = if state.following.is_some() { 1.0 } else { 0.15 };

    camera.translation.x += delta.x * approach_scalar;
    camera.translation.y += delta.y * approach_scalar;

    let scale_scalar = match state.command.z.cmp(&0) {
        std::cmp::Ordering::Less => 1.05,
        std::cmp::Ordering::Equal => 1.0,
        std::cmp::Ordering::Greater => 1.0 / 1.05,
    };

    state.target_scale *= scale_scalar;

    camera.scale.x += (state.target_scale - camera.scale.x) * 0.2;
    camera.scale.y += (state.target_scale - camera.scale.y) * 0.2;
}

fn track_followed_entity(transforms: TransformHelper, mut state: ResMut<CameraState>) {
    let Some(entity) = state.following else {
        return;
    };

    let Ok(global) = transforms.compute_global_transform(entity) else {
        return;
    };

    state.target_pos = global.translation().xy();
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
