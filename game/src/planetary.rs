use bevy::prelude::*;
use starling::planning::EncounterDir;
use starling::planning::SepTracker;
use std::time::Duration;

use starling::core::*;
use starling::examples::*;

use crate::debug::*;
use crate::drawing::*;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(Update, (draw, keyboard_input, handle_zoom));
        app.add_systems(FixedUpdate, (propagate_system, draw_separation_tracker));
        app.add_systems(Update, (log_system_info, update_camera, process_commands));
    }
}

fn draw(gizmos: Gizmos, res: Res<GameState>) {
    draw_orbital_system(gizmos, res)
}

fn draw_separation_tracker(gizmos: Gizmos, mut state: ResMut<GameState>) {
    let a = state.primary_object;
    let b = state.secondary_object;

    let t = state.system.epoch;

    let pva = match state.system.transform_from_id(Some(a), t) {
        Some(p) => p,
        _ => return,
    };
    let pvb = match state.system.transform_from_id(Some(b), t) {
        Some(p) => p,
        _ => return,
    };

    let sep = pva.pos.distance(pvb.pos);

    state.tracker.update(t, sep);
}

#[derive(Resource)]
pub struct GameState {
    pub sim_time: Duration,
    pub sim_speed: f32,
    pub show_orbits: bool,
    pub show_potential_field: bool,
    pub show_gravity_field: bool,
    pub show_primary_body: bool,
    pub paused: bool,
    pub system: OrbitalSystem,
    pub backup: Option<OrbitalSystem>,
    pub primary_object: ObjectId,
    pub secondary_object: ObjectId,
    pub follow_object: bool,
    pub target_scale: f32,
    pub camera_easing: Vec2,
    pub camera_switch: bool,

    pub tracker: SepTracker,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            sim_time: Duration::default(),
            sim_speed: 1.0,
            show_orbits: true,
            show_potential_field: false,
            show_gravity_field: false,
            show_primary_body: false,
            paused: false,
            system: default_example(),
            primary_object: ObjectId(10),
            secondary_object: ObjectId(20),
            backup: None,
            follow_object: false,
            target_scale: 4.0,
            camera_easing: Vec2::ZERO,
            camera_switch: false,
            tracker: SepTracker::default(),
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
}

fn propagate_system(time: Res<Time>, mut state: ResMut<GameState>) {
    if state.paused {
        return;
    }
    let sp = state.sim_speed;
    state.sim_time += Duration::from_nanos((time.delta().as_nanos() as f32 * sp) as u64);
    state.system.epoch = state.sim_time;
}

fn log_system_info(state: Res<GameState>, mut evt: EventWriter<DebugLog>) {
    send_log(
        &mut evt,
        &format!("Epoch: {:0.2}", state.system.epoch.as_secs_f32()),
    );
    send_log(&mut evt, &format!("Scale: {:0.3}", state.target_scale));
    send_log(&mut evt, &format!("{} objects", state.system.objects.len()));
    if state.paused {
        send_log(&mut evt, "Paused");
    }
    send_log(&mut evt, &format!("Sim speed: {:0.2}", state.sim_speed));
    send_log(&mut evt, &format!("Show (o)orbits: {}", state.show_orbits));
    send_log(
        &mut evt,
        &format!("Show (p)otential: {}", state.show_potential_field),
    );
    send_log(
        &mut evt,
        &format!("Show (g)ravity: {}", state.show_gravity_field),
    );
    send_log(
        &mut evt,
        &format!("Show primary (b)ody: {}", state.show_primary_body),
    );
    send_log(&mut evt, &format!("Primary: {:?}", state.primary_object));
    send_log(
        &mut evt,
        &format!("Secondary: {:?}", state.secondary_object),
    );
    send_log(
        &mut evt,
        &format!("Follow tracked: {:?}", state.follow_object),
    );

    if let Some(obj) = state.system.lookup_ref(state.primary_object) {
        send_log(&mut evt, &format!("{:#?}", obj));
    }
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut exit: ResMut<Events<bevy::app::AppExit>>,
    cstate: Res<CommandsState>,
) {
    if cstate.active {
        return;
    }

    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Period => {
                state.sim_speed = f32::clamp(state.sim_speed * 10.0, 0.01, 1000.0);
            }
            KeyCode::Comma => {
                state.sim_speed = f32::clamp(state.sim_speed / 10.0, 0.01, 2000.0);
            }
            KeyCode::KeyF => {
                state.camera_switch = true;
            }
            KeyCode::Equal => {
                state.target_scale /= 1.5;
            }
            KeyCode::Minus => {
                state.target_scale *= 1.5;
            }
            KeyCode::KeyS => {
                state.paused = true;
                state.system.epoch += Duration::from_millis(10);
                state.sim_time = state.system.epoch;
            }
            _ => (),
        }
    }

    if keys.just_pressed(KeyCode::KeyM) || keys.all_pressed([KeyCode::KeyM, KeyCode::ShiftLeft]) {
        let ObjectId(max) = state.system.max_id().unwrap_or(ObjectId(0));
        let ObjectId(mut id) = state.primary_object;
        id += 1;
        while !state.system.has_object(ObjectId(id)) && id < max {
            id += 1
        }
        state.primary_object = ObjectId(id.min(max));
    }
    if keys.just_pressed(KeyCode::KeyN) || keys.all_pressed([KeyCode::KeyN, KeyCode::ShiftLeft]) {
        let ObjectId(min) = state.system.min_id().unwrap_or(ObjectId(0));
        let ObjectId(mut id) = state.primary_object;
        id -= 1;
        while !state.system.has_object(ObjectId(id)) && id > min {
            id -= 1;
        }
        state.primary_object = ObjectId(id.max(min));
    }
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(bevy::app::AppExit::Success);
    }
}

fn load_new_scenario(state: &mut GameState, new_system: OrbitalSystem) {
    state.backup = Some(new_system.clone());
    state.system = new_system;
    state.sim_time = Duration::default();
}

fn on_command(state: &mut GameState, cmd: &Vec<String>) {
    let starts_with = |s: &'static str| -> bool { cmd.first() == Some(&s.to_string()) };

    if starts_with("load") {
        let system = match cmd.get(1).map(|s| s.as_str()) {
            Some("earth") => earth_moon_example_one(),
            Some("moon") => patched_conics_scenario(),
            Some("jupiter") => sun_jupiter_lagrange(),
            _ => {
                return;
            }
        };
        load_new_scenario(state, system);
    } else if starts_with("toggle") {
        match cmd.get(1).map(|s| s.as_str()) {
            Some("gravity") => {
                state.show_gravity_field = !state.show_gravity_field;
            }
            Some("potential") => {
                state.show_potential_field = !state.show_potential_field;
            }
            Some("primary") => {
                state.show_primary_body = !state.show_primary_body;
            }
            Some("orbit") => {
                state.show_orbits = !state.show_orbits;
            }
            _ => {
                return;
            }
        }
    } else if starts_with("restore") {
        if let Some(sys) = &state.backup {
            state.system = sys.clone();
            state.sim_time = state.system.epoch;
        }
    } else if starts_with("save") {
        state.backup = Some(state.system.clone());
    } else if starts_with("pri") {
        if let Some(n) = cmd.get(1).map(|s| s.parse::<i64>().ok()).flatten() {
            state.primary_object = ObjectId(n)
        }
    } else if starts_with("sec") {
        if let Some(n) = cmd.get(1).map(|s| s.parse::<i64>().ok()).flatten() {
            state.secondary_object = ObjectId(n)
        }
    } else if starts_with("swap") {
        let x = state.primary_object;
        state.primary_object = state.secondary_object;
        state.secondary_object = x;
    }
}

fn process_commands(mut evts: EventReader<DebugCommand>, mut state: ResMut<GameState>) {
    for DebugCommand(cmd) in evts.read() {
        on_command(&mut state, cmd);
    }
}

fn handle_zoom(state: Res<GameState>, mut tf: Query<&mut Transform, With<Camera>>) {
    let mut transform = tf.single_mut();
    let ds = (state.target_scale - transform.scale) * 0.2;
    transform.scale += ds;
}

fn update_camera(mut query: Query<&mut Transform, With<Camera>>, mut state: ResMut<GameState>) {
    let mut tf = query.single_mut();

    if state.camera_switch {
        state.follow_object = !state.follow_object;
    }

    let current_pos = tf.translation.xy();

    let target_pos = if state.follow_object {
        state
            .system
            .lookup(state.primary_object)
            .map(|o| state.system.global_transform(&o.prop, state.system.epoch))
            .flatten()
            .map(|p| p.pos)
            .unwrap_or(Vec2::ZERO)
    } else {
        Vec2::ZERO
    };

    if state.camera_switch {
        state.camera_easing = current_pos - target_pos;
    }

    state.camera_switch = false;

    *tf = tf.with_translation((target_pos + state.camera_easing).extend(0.0));
    state.camera_easing *= 0.85;
}
