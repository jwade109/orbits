use crate::planetary::GameState;
use bevy::prelude::*;

pub fn cursor_position(
    q_windows: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    state: Res<GameState>,
) {
    let (camera, transform) = *camera;
    if let Some(position) = q_windows.cursor_position() {
        if let Ok(p) = camera.viewport_to_world_2d(transform, position) {
            if let Some((n, b)) = state
                .scenario
                .relevant_body(p, state.sim_time)
                .map(|id| Some(state.scenario.lup(id, state.sim_time)?.named_body()?))
                .flatten()
            {
                println!("{:?} {:?} {} {:?}", position, p, n, b);
            }
        }
    } else {
        println!("Cursor is not in the game window.");
    }
}
