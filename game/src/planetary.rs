use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

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
        app.add_systems(
            Update,
            (
                keyboard_input,
                mouse_button_input,
                handle_zoom,
                scroll_events,
                update_cursor,
                update_camera,
                draw,
            )
                .chain(),
        );
        app.add_systems(FixedUpdate, propagate_system);
        app.add_systems(Update, (log_system_info, process_commands));
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
//                 state.primary().0
//             )));
//             ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
//                 if ui.add(egui::Button::new("<<<")).clicked() {
//                     state.primary().0 -= 100;
//                 }
//                 if ui.add(egui::Button::new("<<")).clicked() {
//                     state.primary().0 -= 10;
//                 }
//                 if ui.add(egui::Button::new("<")).clicked() {
//                     state.primary().0 -= 1;
//                 }
//                 if ui.add(egui::Button::new(">")).clicked() {
//                     state.primary().0 += 1;
//                 }
//                 if ui.add(egui::Button::new(">>")).clicked() {
//                     state.primary().0 += 10;
//                 }
//                 if ui.add(egui::Button::new(">>>")).clicked() {
//                     state.primary().0 += 100;
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
//             if let Some((orbit, _)) = state.system.lookup_subsystem(state.primary()) {
//                 ui.add(egui::Label::new(format!(
//                     "Epoch: {:?}\nOrbit: {:#?}",
//                     state.sim_time, orbit
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
    pub backup: Option<(OrbitalSystem, Nanotime)>,
    pub track_list: Vec<ObjectId>,
    pub highlighted_list: Vec<ObjectId>,
    pub target_scale: f32,
    pub actual_scale: f32,
    pub camera_easing: Vec2,
    pub camera_switch: bool,
    pub draw_levels: Vec<i32>,
    pub cursor: Vec2,
    pub center: Vec2,
    pub mouse_screen_pos: Option<Vec2>,
    pub mouse_down_pos: Option<Vec2>,
    pub window_dims: Vec2,
}

impl GameState {
    pub fn game_bounds(&self) -> AABB {
        AABB::from_center(self.center, self.window_dims * self.actual_scale)
    }

    pub fn window_bounds(&self) -> AABB {
        AABB(Vec2::ZERO, self.window_dims)
    }

    pub fn primary(&self) -> ObjectId {
        *self.track_list.first().unwrap_or(&ObjectId(-1))
    }

    pub fn mouse_pos(&self) -> Option<Vec2> {
        let gb = self.game_bounds();
        let wb = self.window_bounds();
        Some(AABB::map(wb, gb, self.mouse_screen_pos?))
    }

    pub fn mouse_down_pos(&self) -> Option<Vec2> {
        let p = self.mouse_down_pos?;
        let gb = self.game_bounds();
        let wb = self.window_bounds();
        Some(AABB::map(wb, gb, p))
    }

    pub fn selection_region(&self) -> Option<AABB> {
        Some(AABB::from_arbitrary(
            self.mouse_pos()?,
            self.mouse_down_pos()?,
        ))
    }

    pub fn toggle_track(&mut self, id: ObjectId) {
        if self.track_list.contains(&id) {
            self.track_list.retain(|e| *e != id);
        } else {
            self.track_list.push(id);
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            sim_time: Nanotime(0),
            sim_speed: 0,
            show_orbits: true,
            show_potential_field: false,
            paused: false,
            system: default_example(),
            track_list: Vec::new(),
            highlighted_list: Vec::new(),
            backup: Some((default_example(), Nanotime::default())),
            target_scale: 4.0,
            actual_scale: 4.0,
            camera_easing: Vec2::ZERO,
            camera_switch: false,
            draw_levels: (-70000..=-10000)
                .step_by(10000)
                .chain((-5000..-3000).step_by(250))
                .collect(),
            cursor: Vec2::ZERO,
            center: Vec2::ZERO,
            mouse_screen_pos: None,
            mouse_down_pos: None,
            window_dims: Vec2::ZERO,
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

    let s = state.sim_time;
    let mut to_apply = vec![];
    for obj in &mut state.system.objects {
        obj.events.retain(|e| {
            if e.stamp <= s {
                to_apply.push(*e);
                false
            } else {
                true
            }
        })
    }
    for e in to_apply {
        state.system.apply(e);
    }

    if let Some(a) = state.selection_region() {
        state.highlighted_list = state
            .system
            .ids()
            .iter()
            .filter_map(|id| {
                let pos = state.system.pv(*id, state.sim_time)?.pos;
                a.contains(pos).then(|| *id)
            })
            .collect();
    } else {
        state.highlighted_list.clear();
    }

    let mut track_list = state.track_list.clone();
    track_list.retain(|o| state.system.lookup(*o).is_some());
    state.track_list = track_list;
}

fn sim_speed_str(speed: i32) -> String {
    if speed == 0 {
        ">".to_owned()
    } else if speed > 0 {
        (0..speed.abs() * 2).map(|_| '>').collect()
    } else {
        (0..speed.abs() * 2).map(|_| '<').collect()
    }
}

fn log_system_info(state: Res<GameState>, mut evt: EventWriter<DebugLog>) {
    send_log(&mut evt, &format!("Tracks: {:?}", state.track_list));
    send_log(&mut evt, &format!("Epoch: {:?}", state.sim_time));
    send_log(&mut evt, &format!("Scale: {:0.3}", state.actual_scale));
    send_log(&mut evt, &format!("{} objects", state.system.objects.len()));
    if state.paused {
        send_log(&mut evt, "Paused");
    }
    send_log(
        &mut evt,
        &format!(
            "Sim speed: 10^{} [{}]",
            state.sim_speed,
            sim_speed_str(state.sim_speed)
        ),
    );
    send_log(
        &mut evt,
        &format!("Object type: {:?}", state.system.otype(state.primary())),
    );

    if let Some(obj) = state.system.lookup(state.primary()) {
        let pv = obj.orbit.pv_at_time(state.sim_time);
        send_log(&mut evt, &format!("{:#?}", obj));
        send_log(&mut evt, &format!("{:#?}", pv));
        let ta = obj.orbit.ta_at_time(state.sim_time);
        let ea = true_to_eccentric(ta, obj.orbit.eccentricity);
        let ma = eccentric_to_mean(ea, obj.orbit.eccentricity);
        let ea2 = mean_to_eccentric(ma, obj.orbit.eccentricity)
            .unwrap_or(Anomaly::with_ecc(obj.orbit.eccentricity, 0.3777));
        let ta2 = eccentric_to_true(ea2, obj.orbit.eccentricity);

        let mm = obj.orbit.mean_motion();

        let dt = ma.as_f32() / mm;

        // send_log(
        //     &mut evt,
        //     &format!(
        //         "TA: {:?}\nEA: {:?}\nMA: {:?}\nEA: {:?}\nTA: {:?}\nTP: {:0.3}",
        //         ta, ea, ma, ea2, ta2, dt
        //     ),
        // );

        send_log(
            &mut evt,
            &format!("Consistent: {}", obj.orbit.is_consistent(state.sim_time)),
        );

        send_log(
            &mut evt,
            &format!("Next p: {:?}", obj.orbit.t_next_p(state.sim_time)),
        );

        send_log(&mut evt, &format!("Period: {:?}", obj.orbit.period()));
        send_log(
            &mut evt,
            &format!("Orbit count: {:?}", obj.orbit.orbit_number(state.sim_time)),
        );
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
            KeyCode::KeyP => {
                run_physics_predictions(&mut state);
            }
            _ => (),
        }
    }

    let dt = time.delta().as_secs_f32();
    let cursor_rate = 1400.0 * state.actual_scale;

    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        state.cursor.x -= cursor_rate * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        state.cursor.x += cursor_rate * dt;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        state.cursor.y += cursor_rate * dt;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        state.cursor.y -= cursor_rate * dt;
    }
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(bevy::app::AppExit::Success);
    }
}

fn mouse_button_input(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        state.mouse_down_pos = state.mouse_screen_pos;
    }
    if buttons.just_released(MouseButton::Left) {
        state.mouse_down_pos = None;
        if !keys.pressed(KeyCode::ShiftLeft) {
            state.track_list.clear();
        }
        for hl in state.highlighted_list.clone() {
            state.toggle_track(hl);
        }
    }
    // we can check multiple at once with `.any_*`
    if buttons.any_just_pressed([MouseButton::Left, MouseButton::Middle]) {
        // Either the left or the middle (wheel) button was just pressed
    }
}

fn run_physics_predictions(state: &mut GameState) {
    _ = state
        .track_list
        .iter()
        .filter_map(|id| {
            let obj = state.system.lookup(*id)?;

            if !obj.events.is_empty() {
                return None;
            }

            let start = obj
                .computed_until
                .unwrap_or(state.sim_time)
                .max(state.sim_time);
            let end = start + Nanotime::secs(100);

            let future = get_future_path(&state.system, *id, start, end);

            let (pos, crashtime) = match future {
                Err(_) => {
                    println!("Prediction failed: {}, {:?}", *id, future);
                    return None;
                }
                Ok((pos, crashtime)) => (pos, crashtime),
            };

            let object = state.system.lookup_orbiter_mut(*id)?;

            if let Some(crash) = crashtime {
                let e: OrbitalEvent = OrbitalEvent::collision(*id, crash);
                object.events.push(e);
                object.computed_until = Some(e.stamp);
                object.sample_points.extend_from_slice(&pos);
            } else {
                object.computed_until = Some(end);
                object.sample_points.clear();
            }

            Some(())
        })
        .collect::<Vec<_>>();
}

fn update_cursor(
    mut state: ResMut<GameState>,
    q: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let (w, p) = match q.get_single() {
        Ok(w) => (w, w.cursor_position()),
        Err(_) => {
            state.mouse_screen_pos = None;
            return;
        }
    };
    state.mouse_screen_pos = p.map(|p| Vec2::new(p.x, w.height() - p.y));
    state.window_dims = Vec2::new(w.width(), w.height());
}

fn load_new_scenario(state: &mut GameState, new_system: OrbitalSystem) {
    state.backup = Some((new_system.clone(), Nanotime::default()));
    state.target_scale = 0.001 * new_system.primary.soi;
    state.system = new_system;
    state.sim_time = Nanotime::default();
}

fn on_command(state: &mut GameState, cmd: &Vec<String>) {
    let starts_with = |s: &'static str| -> bool { cmd.first() == Some(&s.to_string()) };

    if starts_with("load") {
        let system = match cmd.get(1).map(|s| s.as_str()) {
            Some("grid") => consistency_example(),
            Some("earth") => earth_moon_example_one(),
            Some("earth2") => earth_moon_example_two(),
            Some("moon") => just_the_moon(),
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
        if let Some((sys, time)) = &state.backup {
            state.system = sys.clone();
            state.sim_time = *time;
        }
    } else if starts_with("save") {
        state.backup = Some((state.system.clone(), state.sim_time));
    } else if starts_with("track") {
        for n in cmd.iter().skip(1).filter_map(|s| s.parse::<i64>().ok()) {
            let id = ObjectId(n);
            state.toggle_track(id);
        }
    } else if starts_with("untrack") {
        state.track_list.clear();
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
            let orbit = Orbit::from_pv(r, v, state.system.primary.mass, state.sim_time);
            let id = ObjectId((rand(0.0, 1.0) * 10000.0 + 1000.0) as i64);
            state.toggle_track(id);
            state.system.add_object(id, orbit);
        }
    } else if starts_with("clear") {
        state.system.objects.clear();
    } else if starts_with("maneuver") {
        _ = state.track_list.iter().filter_map(|id| {
            let t = Nanotime::secs_f32(cmd.get(1)?.parse().ok()?);
            let dx = cmd.get(2)?.parse::<f32>().ok()?;
            let dy = cmd.get(3)?.parse::<f32>().ok()?;
            let evt = OrbitalEvent::maneuver(*id, Vec2::new(dx, dy), t);
            let obj = state.system.lookup_orbiter_mut(*id)?;
            obj.events.push(evt);
            Some(())
        }).collect::<Vec<_>>();
    }
}

fn process_commands(mut evts: EventReader<DebugCommand>, mut state: ResMut<GameState>) {
    for DebugCommand(cmd) in evts.read() {
        on_command(&mut state, cmd);
    }
}

fn handle_zoom(mut state: ResMut<GameState>, mut tf: Query<&mut Transform, With<Camera>>) {
    let mut transform = tf.single_mut();
    let ds = (state.target_scale - transform.scale) * 0.5;
    transform.scale += ds;
    state.actual_scale = transform.scale.x;
}

fn update_camera(mut query: Query<&mut Transform, With<Camera>>, mut state: ResMut<GameState>) {
    let mut tf = query.single_mut();

    let current_pos = tf.translation.xy();

    let target_pos = state.cursor;

    if state.camera_switch {
        state.camera_easing = current_pos - target_pos;
    }

    state.camera_switch = false;

    state.center = target_pos + state.camera_easing;

    *tf = tf.with_translation(state.center.extend(0.0));
    state.camera_easing *= 0.85;
}

fn scroll_events(
    mut evr_scroll: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
) {
    if keys.pressed(KeyCode::ShiftLeft) {
        for ev in evr_scroll.read() {
            if ev.y > 0.0 {
                state.sim_speed = i32::clamp(state.sim_speed + 1, -6, 4);
            } else {
                state.sim_speed = i32::clamp(state.sim_speed - 1, -6, 4);
            }
        }
    } else {
        for ev in evr_scroll.read() {
            if ev.y > 0.0 {
                state.target_scale /= 1.3;
            } else {
                state.target_scale *= 1.3;
            }
        }
    }
}
