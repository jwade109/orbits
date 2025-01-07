use bevy::input::mouse::MouseWheel;
use bevy::math::VectorSpace;
use bevy::prelude::*;

// use bevy_egui::{egui, EguiContexts, EguiPlugin};

use starling::core::*;
use starling::examples::*;
use starling::orbit::*;
use starling::planning::*;

use crate::debug::*;
use crate::drawing::*;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(Update, (draw, keyboard_input, handle_zoom));
        app.add_systems(FixedUpdate, propagate_system);
        app.add_systems(
            Update,
            (
                log_system_info,
                update_camera,
                process_commands,
                scroll_events,
            ),
        );
        // app.add_plugins(EguiPlugin).add_systems(Update, ui_system);
    }
}

// fn ui_system(mut contexts: EguiContexts, mut state: ResMut<GameState>) {
//     egui::Window::new("Settings")
//         .resizable(true)
//         .show(contexts.ctx_mut(), |ui| {
//             if state.paused {
//                 if ui.add(egui::Button::new("Unpause")).clicked() {
//                     state.paused = false;
//                 }
//             } else {
//                 if ui.add(egui::Button::new("Pause")).clicked() {
//                     state.paused = true;
//                 }
//             }

//             ui.add_space(10.0);
//             ui.add(egui::Label::new("Sim Speed"));
//             ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
//                 for speed in [0.01, 0.1, 1.0, 10.0, 100.0, 1000.0] {
//                     let en = state.sim_speed != speed;
//                     let button = egui::Button::new(format!("{:0.2}", speed));
//                     if ui.add_enabled(en, button).clicked() {
//                         state.sim_speed = speed;
//                     }
//                 }
//             });

//             ui.add_space(10.0);
//             if ui.add(egui::Button::new("Toggle Follow")).clicked() {
//                 state.camera_switch = true;
//             }
//             if ui.add(egui::Button::new("Toggle Orbits")).clicked() {
//                 state.show_orbits = !state.show_orbits;
//             }
//             if ui.add(egui::Button::new("Toggle Potential")).clicked() {
//                 state.show_potential_field = !state.show_potential_field;
//             }

//             ui.add_space(10.0);
//             ui.add(egui::Label::new("Scenarios"));
//             ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
//                 if ui.add(egui::Button::new("Earth-Moon")).clicked() {
//                     load_new_scenario(&mut state, earth_moon_example_one());
//                 }
//                 if ui.add(egui::Button::new("Moon")).clicked() {
//                     load_new_scenario(&mut state, just_the_moon());
//                 }
//                 if ui.add(egui::Button::new("Jupiter")).clicked() {
//                     load_new_scenario(&mut state, sun_jupiter_lagrange());
//                 }
//             });

//             ui.add_space(10.0);
//             ui.add(egui::Label::new(format!(
//                 "Primary ({})",
//                 state.primary_object.0
//             )));
//             ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
//                 if ui.add(egui::Button::new("<<<")).clicked() {
//                     state.primary_object.0 -= 100;
//                 }
//                 if ui.add(egui::Button::new("<<")).clicked() {
//                     state.primary_object.0 -= 10;
//                 }
//                 if ui.add(egui::Button::new("<")).clicked() {
//                     state.primary_object.0 -= 1;
//                 }
//                 if ui.add(egui::Button::new(">")).clicked() {
//                     state.primary_object.0 += 1;
//                 }
//                 if ui.add(egui::Button::new(">>")).clicked() {
//                     state.primary_object.0 += 10;
//                 }
//                 if ui.add(egui::Button::new(">>>")).clicked() {
//                     state.primary_object.0 += 100;
//                 }
//             });

//             ui.add_space(10.0);
//             ui.add(egui::Label::new(format!(
//                 "Secondary ({})",
//                 state.secondary_object.0
//             )));
//             ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
//                 if ui.add(egui::Button::new("<<<")).clicked() {
//                     state.secondary_object.0 -= 100;
//                 }
//                 if ui.add(egui::Button::new("<<")).clicked() {
//                     state.secondary_object.0 -= 10;
//                 }
//                 if ui.add(egui::Button::new("<")).clicked() {
//                     state.secondary_object.0 -= 1;
//                 }
//                 if ui.add(egui::Button::new(">")).clicked() {
//                     state.secondary_object.0 += 1;
//                 }
//                 if ui.add(egui::Button::new(">>")).clicked() {
//                     state.secondary_object.0 += 10;
//                 }
//                 if ui.add(egui::Button::new(">>>")).clicked() {
//                     state.secondary_object.0 += 100;
//                 }
//             });

//             ui.add_space(20.0);
//             ui.heading("Orbital Info");
//             ui.add_space(10.0);
//             if let Some((orbit, _)) = state.system.lookup_subsystem(state.primary_object) {
//                 ui.add(egui::Label::new(format!(
//                     "Epoch: {:?}\nOrbit: {:#?}",
//                     state.system.epoch, orbit
//                 )));
//             }

//             ui.allocate_space(ui.available_size());
//         });
// }

fn draw(gizmos: Gizmos, res: Res<GameState>) {
    draw_game_state(gizmos, res)
}

#[derive(Resource)]
pub struct GameState {
    pub sim_time: Nanotime,
    pub sim_speed: i32,
    pub show_orbits: bool,
    pub show_potential_field: bool,
    pub paused: bool,
    pub system: OrbitalSystem,
    pub backup: Option<OrbitalSystem>,
    pub primary_object: ObjectId,
    pub secondary_object: ObjectId,
    pub follow_cursor: bool,
    pub target_scale: f32,
    pub actual_scale: f32,
    pub camera_easing: Vec2,
    pub camera_switch: bool,
    pub draw_levels: Vec<i32>,
    pub cursor: Vec2,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            sim_time: Nanotime(0),
            sim_speed: 0,
            show_orbits: false,
            show_potential_field: false,
            paused: false,
            system: default_example(),
            primary_object: ObjectId(-1),
            secondary_object: ObjectId(-1),
            backup: Some(default_example()),
            follow_cursor: false,
            target_scale: 4.0,
            actual_scale: 4.0,
            camera_easing: Vec2::ZERO,
            camera_switch: false,
            draw_levels: (-70000..=-10000)
                .step_by(10000)
                .chain((-5000..-3000).step_by(250))
                .collect(),
            cursor: Vec2::ZERO,
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
    let s = 0.02;
    commands.insert_resource(ClearColor(Color::linear_rgb(s, s, s)));
}

fn propagate_system(time: Res<Time>, mut state: ResMut<GameState>) {
    if state.paused {
        return;
    }
    let sp = 10.0f32.powi(state.sim_speed);
    state.sim_time += Nanotime((time.delta().as_nanos() as f32 * sp) as i64);
    state.system.epoch = state.sim_time;
    let s = state.system.epoch;
    for (_, _, sys) in state.system.subsystems.iter_mut() {
        sys.epoch = s;
    }
    state.system.rebalance();
}

fn log_system_info(state: Res<GameState>, mut evt: EventWriter<DebugLog>) {
    send_log(&mut evt, &format!("Epoch: {:?}", state.system.epoch));
    send_log(&mut evt, &format!("Scale: {:0.3}", state.actual_scale));
    send_log(&mut evt, &format!("{} objects", state.system.objects.len()));
    if state.paused {
        send_log(&mut evt, "Paused");
    }
    send_log(&mut evt, &format!("Sim speed: 10^{}", state.sim_speed));
    send_log(&mut evt, &format!("Primary: {:?}", state.primary_object));
    send_log(
        &mut evt,
        &format!(
            "Object type: {:?}",
            state.system.otype(state.primary_object)
        ),
    );
    send_log(
        &mut evt,
        &format!("Secondary: {:?}", state.secondary_object),
    );
    send_log(
        &mut evt,
        &format!("Follow tracked: {:?}", state.follow_cursor),
    );

    if let Some((obj, _)) = state.system.lookup_subsystem(state.primary_object) {
        send_log(&mut evt, &format!("{:#?}", obj));
        let ta = obj.ta_at_time(state.system.epoch);
        let ea = true_to_eccentric(ta, obj.eccentricity);
        let ma = eccentric_to_mean(ea, obj.eccentricity);
        let ea2 = mean_to_eccentric(ma, obj.eccentricity)
            .unwrap_or(Anomaly::with_ecc(obj.eccentricity, 0.3777));
        let ta2 = eccentric_to_true(ea2, obj.eccentricity);

        let mm = obj.mean_motion();

        let dt = ma.as_f32() / mm;

        send_log(
            &mut evt,
            &format!(
                "TA: {:?}\nEA: {:?}\nMA: {:?}\nEA: {:?}\nTA: {:?}\nTP: {:0.3}",
                ta, ea, ma, ea2, ta2, dt
            ),
        );

        send_log(
            &mut evt,
            &format!("Consistent: {}", obj.is_consistent(state.system.epoch)),
        );

        send_log(
            &mut evt,
            &format!("Next p: {:?}", obj.t_next_p(state.system.epoch)),
        );

        send_log(&mut evt, &format!("Period: {:?}", obj.period()));
        send_log(
            &mut evt,
            &format!("Orbit count: {:?}", obj.orbit_number(state.system.epoch)),
        );
    }

    if let Some(dat) = state.system.lookup_metadata(state.primary_object) {
        send_log(&mut evt, &format!("{:#?}", dat));
    }
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut exit: ResMut<Events<bevy::app::AppExit>>,
    cstate: Res<CommandsState>,
    time: Res<Time>,
) {
    if cstate.active {
        return;
    }

    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Period => {
                state.sim_speed = i32::clamp(state.sim_speed + 1, -6, 4);
            }
            KeyCode::Comma => {
                state.sim_speed = i32::clamp(state.sim_speed - 1, -6, 4);
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
                state.system.epoch.0 += 1000000;
                state.sim_time = state.system.epoch;
            }
            _ => (),
        }
    }

    let dt = time.delta().as_secs_f32();
    let cursor_rate = 500.0 * state.actual_scale;

    if keys.pressed(KeyCode::ArrowLeft) {
        state.cursor.x -= cursor_rate * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        state.cursor.x += cursor_rate * dt;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        state.cursor.y += cursor_rate * dt;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        state.cursor.y -= cursor_rate * dt;
    }

    if keys.just_pressed(KeyCode::KeyM) || keys.all_pressed([KeyCode::KeyM, KeyCode::ShiftLeft]) {
        let i = state.primary_object.0;
        state.primary_object = ObjectId(i + 1);
    }
    if keys.just_pressed(KeyCode::KeyN) || keys.all_pressed([KeyCode::KeyN, KeyCode::ShiftLeft]) {
        let i = state.primary_object.0;
        state.primary_object = ObjectId(i - 1);
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
    state.target_scale = 0.001 * new_system.primary.soi;
    state.system = new_system;
    state.sim_time = Nanotime::default();
}

fn on_command(state: &mut GameState, cmd: &Vec<String>) {
    let starts_with = |s: &'static str| -> bool { cmd.first() == Some(&s.to_string()) };

    if starts_with("load") {
        let system = match cmd.get(1).map(|s| s.as_str()) {
            Some("earth") => earth_moon_example_one(),
            Some("earth2") => earth_moon_example_two(),
            Some("jupiter") => sun_jupiter_lagrange(),
            _ => {
                return;
            }
        };
        load_new_scenario(state, system);
    } else if starts_with("toggle") {
        match cmd.get(1).map(|s| s.as_str()) {
            Some("potential") => {
                state.show_potential_field = !state.show_potential_field;
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
    } else if starts_with("level") {
        if Some(&"clear".to_string()) == cmd.get(1) {
            state.draw_levels.clear();
        } else {
            state.draw_levels.extend(
                cmd.iter()
                    .skip(1)
                    .filter_map(|s| Some(-(s.parse::<i32>().ok()?))),
            );
        }
    } else if starts_with("remove") {
        if let Some(n) = cmd.get(1).map(|s| s.parse::<i64>().ok()).flatten() {
            state.system.remove_object(ObjectId(n));
        }
    } else if starts_with("spawn") {
        dbg!(cmd);
        if let Some(coords) = cmd
            .get(1..5)
            .map(|strs| {
                strs.iter()
                    .map(|s| s.parse::<f32>().ok())
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
        {
            let r = Vec2::new(coords[0], coords[1]);
            let v = Vec2::new(coords[2], coords[3]);
            let orbit = Orbit::from_pv(r, v, state.system.primary.mass, state.system.epoch);
            println!("New object: {:?}", orbit);
            let id = ObjectId(1) + state.system.high_water_mark;
            state.primary_object = id;
            state.system.add_object(id, orbit);
        }
    } else if starts_with("clear") {
        state.system.objects.clear();
    } else if starts_with("near") {
        let id = state
            .system
            .objects
            .iter()
            .map(|(id, orbit)| {
                let pos = orbit.pv_at_time(state.sim_time).pos;
                let d = pos.distance(state.cursor);
                (*id, d)
            })
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2));
        if let Some((id, _)) = id {
            state.primary_object = id;
        }
    }
}

fn process_commands(mut evts: EventReader<DebugCommand>, mut state: ResMut<GameState>) {
    for DebugCommand(cmd) in evts.read() {
        on_command(&mut state, cmd);
    }
}

fn handle_zoom(mut state: ResMut<GameState>, mut tf: Query<&mut Transform, With<Camera>>) {
    let mut transform = tf.single_mut();
    let ds = (state.target_scale - transform.scale) * 0.1;
    transform.scale += ds;
    state.actual_scale = transform.scale.x;
}

fn update_camera(mut query: Query<&mut Transform, With<Camera>>, mut state: ResMut<GameState>) {
    let mut tf = query.single_mut();

    if state.camera_switch {
        state.follow_cursor = !state.follow_cursor;
    }

    let current_pos = tf.translation.xy();

    let target_pos = if state.follow_cursor {
        state.cursor
        // state
        //     .system
        //     .transform_from_id(state.primary_object, state.system.epoch)
        //     .map(|p| p.pos)
        //     .unwrap_or(Vec2::ZERO)
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

fn scroll_events(mut evr_scroll: EventReader<MouseWheel>, mut state: ResMut<GameState>) {
    for ev in evr_scroll.read() {
        if ev.y > 0.0 {
            state.target_scale /= 1.5;
        } else {
            state.target_scale *= 1.5;
        }
    }
}
