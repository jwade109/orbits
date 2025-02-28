use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use starling::prelude::*;
use starling::scenario::OrbiterOrBody;

use crate::button::Button;
use crate::camera_controls::*;
use crate::debug::*;
use crate::drawing::*;
use crate::sprites::PlanetSpritePlugin;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlanetSpritePlugin {});
        app.add_systems(Startup, (init_system, set_camera_scale));
        app.add_systems(
            Update,
            (
                log_system_info,
                process_commands,
                keyboard_input,
                mouse_button_input,
                propagate_system,
                manage_orbiter_labels,
                update_text,
                draw,
            )
                .chain(),
        );
    }
}

fn set_camera_scale(mut query: Query<&mut Transform, With<Camera>>) {
    for mut cam in query.iter_mut() {
        cam.scale *= 6.5;
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
    let s = 0.02;
    commands.insert_resource(ClearColor(Color::linear_rgb(s, s, s)));
}

fn manage_orbiter_labels(
    mut commands: Commands,
    state: Res<GameState>,
    text: Query<(Entity, &FollowObject)>,
) {
    for tid in &state.track_list {
        let has_txt = text.iter().any(|(_, f)| f.0 == *tid);
        if !has_txt {
            commands.spawn((
                Text2d::new(""),
                FollowObject(*tid),
                bevy::sprite::Anchor::TopLeft,
            ));
        }
    }

    for (e, f) in text.iter() {
        if !state.track_list.contains(&f.0) {
            commands.entity(e).despawn();
        }
    }
}

const TEXT_LABELS_Z_INDEX: f32 = 100.0;

fn update_text(res: Res<GameState>, mut text: Query<(&mut Transform, &mut Text2d, &FollowObject)>) {
    let scale = res.camera.actual_scale.min(1.0);
    let zoomed_out = scale == 1.0;
    let mut height = -40.0;
    let count = text.iter().count();
    let _ = text
        .iter_mut()
        .filter_map(|(mut tr, mut text, follow)| {
            let id = follow.0;
            let obj = res.scenario.objects.iter().find(|o| o.id() == id)?;
            let pvl = obj.pvl(res.sim_time)?;
            let pv = obj.pv(res.sim_time, &res.scenario.system)?;
            let prop = obj.propagator_at(res.sim_time)?;
            let (_, _, _, parent) = res.scenario.system.lookup(prop.parent, res.sim_time)?;
            let warn_str = if obj.will_collide() && res.duty_cycle_high {
                " COLLISION IMMINENT"
            } else if id == res.primary() {
                " PRIMARY"
            } else {
                ""
            };

            let event_lines = obj
                .props()
                .iter()
                .filter_map(|p| {
                    let dt = (p.end - res.sim_time).to_secs();
                    if let Some(e) = p.event {
                        Some(format!("\n{:?} in {:0.1}s", e, dt))
                    } else {
                        Some(format!("\nC {:0.1}s {:0.2}s", dt, p.dt.to_secs()))
                    }
                })
                .collect::<String>();

            let prop = obj.propagator_at(res.sim_time)?;

            let p_line = prop
                .orbit
                .t_next_p(res.sim_time)
                .map(|nt| format!("\nP {:0.1}s", (nt - res.sim_time).to_secs()))
                .unwrap_or("".into());

            let altitude = pvl.pos.length() - prop.orbit.body.radius;

            let txt = if count < 8 {
                format!(
                    "{:?}{}\nOrbiting {}{}\nA {:0.1} V {:0.1}\n{:?}{}",
                    id,
                    warn_str,
                    parent.name,
                    p_line,
                    altitude,
                    pvl.vel.length(),
                    prop.orbit.class(),
                    event_lines,
                )
            } else {
                format!(
                    "{:?}{}\nA {:0.1} V {:0.1}",
                    id,
                    warn_str,
                    altitude,
                    pvl.vel.length(),
                )
            };

            let window = res.camera.game_bounds();
            let n = txt.lines().collect::<Vec<_>>().len();
            *text = txt.into();

            if zoomed_out || !window.contains(pv.pos) {
                let s = res.camera.actual_scale;
                let h = 23.0 * (n + 1) as f32;
                let ur = window.center + window.span / 2.0 + Vec2::new(-300.0, height) * s;
                height -= h;
                tr.translation = ur.extend(TEXT_LABELS_Z_INDEX);
                tr.scale = Vec3::new(s, s, s);
            } else {
                tr.translation =
                    (pv.pos + Vec2::new(40.0 * scale, 40.0 * scale)).extend(TEXT_LABELS_Z_INDEX);
                tr.scale = Vec3::new(scale, scale, scale);
            }
            Some(())
        })
        .collect::<Vec<_>>();
}

fn draw(gizmos: Gizmos, res: Res<GameState>) {
    draw_game_state(gizmos, &res)
}

#[derive(Component)]
struct FollowObject(ObjectId);

#[derive(Resource)]
pub struct GameState {
    pub sim_time: Nanotime,
    pub actual_time: Nanotime,
    pub physics_duration: Nanotime,
    pub sim_speed: i32,
    pub paused: bool,
    pub scenario: Scenario,
    pub ids: ObjectIdTracker,
    pub backup: Option<(Scenario, ObjectIdTracker, Nanotime)>,
    pub track_list: Vec<ObjectId>,
    pub highlighted_list: Vec<ObjectId>,
    pub draw_levels: Vec<i32>,
    pub camera: CameraState,
    pub control_points: Vec<Vec2>,
    pub hide_debug: bool,
    pub duty_cycle_high: bool,
    pub controllers: Vec<Controller>,
    pub follow: Option<ObjectId>,
    // pub topo_map: TopoMap,

    // buttons!
    pub show_orbits: Button,
    pub show_potential_field: Button,
    pub show_animations: Button,
    pub spawn_new_craft: Button,
}

impl GameState {
    pub fn buttons(&self) -> Vec<&Button> {
        vec![
            &self.show_orbits,
            &self.show_potential_field,
            &self.show_animations,
            &self.spawn_new_craft,
        ]
    }

    pub fn update_buttons(&mut self, pos: Vec2, clicked: bool) -> bool {
        self.show_orbits.update(pos, clicked)
            | self.show_potential_field.update(pos, clicked)
            | self.show_animations.update(pos, clicked)
            | self.spawn_new_craft.update(pos, clicked)
    }

    pub fn primary(&self) -> ObjectId {
        *self.track_list.first().unwrap_or(&ObjectId(-1))
    }

    pub fn toggle_track(&mut self, id: ObjectId) {
        if self.track_list.contains(&id) {
            self.track_list.retain(|e| *e != id);
        } else {
            self.track_list.insert(0, id);
        }
    }

    pub fn tracked_aabb(&self) -> Option<AABB> {
        let pos = self
            .track_list
            .iter()
            .filter_map(|id| Some(self.scenario.lookup(*id, self.sim_time)?.pv().pos))
            .collect::<Vec<_>>();
        AABB::from_list(&pos).map(|aabb| aabb.padded(60.0))
    }

    pub fn cursor_pv(&self) -> Option<PV> {
        let p1 = self.control_points.get(0);
        let p2 = self
            .control_points
            .get(1)
            .map(|e| *e)
            .or(self.camera.mouse_pos());

        if let Some((p1, p2)) = p1.zip(p2) {
            if p1.distance(p2) < 10.0 {
                return None;
            }

            let v = (self.scenario.system.body.mu() / p1.length()).sqrt();

            Some(PV::new(*p1, (p2 - p1) * v / p1.length()))
        } else {
            None
        }
    }

    pub fn target_orbit(&self) -> Option<SparseOrbit> {
        let pv = self.cursor_pv()?;
        SparseOrbit::from_pv(pv, self.scenario.system.body, self.sim_time)
    }

    pub fn primary_orbit(&self) -> Option<SparseOrbit> {
        let lup = self.scenario.lookup(self.primary(), self.sim_time)?;
        if let OrbiterOrBody::Orbiter(o) = lup.inner {
            Some(o.propagator_at(self.sim_time)?.orbit)
        } else {
            None
        }
    }

    pub fn follow_position(&self) -> Option<Vec2> {
        let id = self.follow?;
        let lup = self.scenario.lookup(id, self.sim_time)?;
        Some(lup.pv().pos)
    }

    pub fn spawn_new(&mut self) -> Option<()> {
        let t = self.target_orbit().or_else(|| self.primary_orbit())?;

        let (parent_id, body) = if self.target_orbit().is_some() {
            (self.scenario.system.id, self.scenario.system.body)
        } else {
            let pri = self.primary_object()?;
            let prop = pri.propagator_at(self.sim_time)?;
            let (body, _, _, _) = self.scenario.system.lookup(prop.parent, self.sim_time)?;
            (prop.parent, body)
        };

        let pv = t.pv_at_time_fallible(self.sim_time).ok()?;
        let perturb = PV::new(
            randvec(pv.pos.length() * 0.005, pv.pos.length() * 0.02),
            randvec(pv.vel.length() * 0.005, pv.vel.length() * 0.02),
        );
        let orbit = SparseOrbit::from_pv(pv + perturb, body, self.sim_time)?;
        let id = self.ids.next();
        self.toggle_track(id);
        self.scenario
            .add_object(id, parent_id, orbit, self.sim_time);
        Some(())
    }

    pub fn delete_objects(&mut self) {
        self.track_list.iter().for_each(|i| {
            self.scenario.remove_object(*i);
        });
    }

    pub fn primary_object_mut(&mut self) -> Option<&mut Orbiter> {
        let pri = self.primary();
        self.scenario.objects.iter_mut().find(|o| o.id() == pri)
    }

    pub fn primary_object(&self) -> Option<&Orbiter> {
        let pri = self.primary();
        self.scenario.objects.iter().find(|o| o.id() == pri)
    }

    pub fn do_maneuver(&mut self, dv: Vec2) -> Option<()> {
        if self.paused {
            return Some(());
        }
        let s = self.sim_time;
        let d = self.physics_duration;
        let p = self.scenario.system.clone();
        let obj = self.primary_object_mut()?;
        match obj.dv(s, dv) {
            Some(()) => (),
            None => {
                println!("Failed to maneuver");
            }
        };
        let res = obj.propagate_to(s, d, &p);
        match res {
            Ok(_) => Some(()),
            Err(p) => {
                dbg!(p);
                None
            }
        }
    }

    pub fn maneuver_plans(&self) -> Vec<ManeuverPlan> {
        let res = (|| -> Option<(SparseOrbit, SparseOrbit)> {
            let (id, dst) = (self.scenario.system.id, self.target_orbit()?);
            let prop = self
                .scenario
                .objects
                .iter()
                .find(|o| o.id() == self.primary())?
                .propagator_at(self.sim_time)?;

            (prop.parent == id).then_some((prop.orbit, dst))
        })();

        if let Some((src, dst)) = res {
            generate_maneuver_plans(&src, &dst, self.sim_time)
        } else {
            vec![]
        }
    }

    pub fn enqueue_plan(&mut self, id: ObjectId, plan: &ManeuverPlan) {
        if self.controllers.iter().find(|c| c.target == id).is_some() {
            println!("Controller already exists for object {}", id);
            return;
        }

        let c = Controller::with_plan(id, plan.clone());
        self.controllers.push(c);
    }
}

impl Default for GameState {
    fn default() -> Self {
        let (scenario, ids) = default_example();

        let mut button_idx = 0;

        let mut next_button = |name: &'static str| -> Button {
            let w = 50.0;
            let h = 40.0;
            let s = 6.0;
            let start = Vec2::new(30.0, 60.0);

            let p1 = start + Vec2::X * (w + s) * button_idx as f32;
            let p2 = p1 + Vec2::new(w, h);
            let b = Button::new(&name, p1, p2, true);
            button_idx += 1;
            b
        };

        GameState {
            sim_time: Nanotime::zero(),
            actual_time: Nanotime::zero(),
            physics_duration: Nanotime::secs(120),
            sim_speed: 0,
            paused: false,
            scenario: scenario.clone(),
            ids,
            track_list: Vec::new(),
            highlighted_list: Vec::new(),
            backup: Some((scenario, ids, Nanotime::zero())),
            draw_levels: (-1000..=1000).step_by(50).collect(),
            camera: CameraState::default(),
            control_points: Vec::new(),
            hide_debug: true,
            duty_cycle_high: false,
            controllers: vec![],
            follow: Some(ObjectId(12)),
            // topo_map: test_topo(),

            // buttons
            show_orbits: next_button("Show Orbits"),
            show_potential_field: next_button("Show Potential"),
            show_animations: next_button("Show Animations"),
            spawn_new_craft: next_button("Spawn New Craft"),
        }
    }
}

fn propagate_system(time: Res<Time>, mut state: ResMut<GameState>) {
    state.actual_time += Nanotime::nanos(time.delta().as_nanos() as i64);
    if !state.paused {
        let sp = 10.0f32.powi(state.sim_speed);
        state.sim_time += Nanotime::nanos((time.delta().as_nanos() as f32 * sp) as i64);
    }

    state.duty_cycle_high = time.elapsed().as_millis() % 1000 < 500;

    let s = state.sim_time;
    let d = state.physics_duration;

    for (id, ri) in state.scenario.simulate(s, d) {
        if let Some(ri) = ri {
            println!(
                "Object {} removed at time {:?} due to {:?}",
                id, ri.stamp, ri.reason
            );
        } else {
            println!("Object {} removed for unknown reason", id);
        }
    }

    if let Some(a) = state.camera.selection_region() {
        state.highlighted_list = state
            .scenario
            .objects
            .iter()
            .filter_map(|o| {
                let pv = state.scenario.lookup(o.id(), state.sim_time)?.pv();
                a.contains(pv.pos).then(|| o.id())
            })
            .collect();
    } else {
        state.highlighted_list.clear();
    }

    let mut track_list = state.track_list.clone();
    track_list.retain(|o| state.scenario.lookup(*o, state.sim_time).is_some());
    state.track_list = track_list;

    let ids = state.scenario.ids();

    state.controllers.retain(|c| {
        if !ids.contains(&c.target) {
            return false;
        }
        if let Some(end) = c.plan().map(|p| p.end()).flatten() {
            end > s
        } else {
            true
        }
    });

    // let scalar = move |p: Vec2| -> f32 { (p.length() + s.to_secs()) % 1000.0 };

    // let levels = (100..=900)
    //     .step_by(100)
    //     .map(|i| i as f32)
    //     .collect::<Vec<_>>();

    // if s - state.topo_map.last_updated > Nanotime::secs(1) {
    //     state.topo_map.update(s, &scalar, &levels);
    // }

    if state.spawn_new_craft.state() {
        state.spawn_new();
        state.spawn_new_craft.set(false);
    }
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
    let logs = [
        "",
        "Look around - [W][A][S][D]",
        "Control orbiter - Arrow Keys",
        "  Increase thrust - hold [LSHIFT]",
        "  Decrease thrust - hold [LCTRL]",
        "Zoom in/out - +/-, [Scroll]",
        "Select spacecraft - Left click and drag",
        "Toggle debug info - [H]",
        "Increase sim speed - [.]",
        "Decrease sim speed - [,]",
        "Pause - [SPACE]",
        "",
    ];

    for log in logs {
        send_log(&mut evt, log);
    }

    send_log(&mut evt, &format!("Epoch: {:?}", state.sim_time));

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

    if state.hide_debug {
        return;
    }

    if state.track_list.len() > 15 {
        send_log(&mut evt, &format!("Tracks: lots of em"));
    } else {
        send_log(&mut evt, &format!("Tracks: {:?}", state.track_list));
    }
    send_log(&mut evt, &format!("Physics: {:?}", state.physics_duration));
    send_log(
        &mut evt,
        &format!("Scale: {:0.3}", state.camera.actual_scale),
    );
    send_log(
        &mut evt,
        &format!("{} objects", state.scenario.objects.len()),
    );

    if let Some(pv) = state.cursor_pv() {
        send_log(&mut evt, &format!("{:0.3}", pv));
    }

    for plan in state.maneuver_plans() {
        send_log(&mut evt, &format!("{}", plan));
    }

    send_log(&mut evt, &format!("Ctlrs: {}", state.controllers.len()));

    {
        let mut dvs = vec![];
        for ctrl in &state.controllers {
            if let Some(plan) = ctrl.plan() {
                for node in &plan.nodes {
                    dvs.push((ctrl.target, node.stamp, node.impulse.vel));
                }
            }
        }
        dvs.sort_by_key(|(_, t, _)| t.inner());
        for (id, t, dv) in dvs {
            send_log(&mut evt, &format!("- {} {:?} {}", id, t, dv))
        }
    }

    let prop_count: usize = state.scenario.objects.iter().map(|o| o.props().len()).sum();
    send_log(&mut evt, &format!("Propagators: {}", prop_count));

    if let Some(lup) = state.scenario.lookup(state.primary(), state.sim_time) {
        match lup.inner {
            OrbiterOrBody::Orbiter(o) => {
                if let Some(prop) = o.propagator_at(state.sim_time) {
                    send_log(&mut evt, &format!("- [{}]", prop));
                    send_log(&mut evt, &format!("{:#?}", prop.orbit));
                    send_log(
                        &mut evt,
                        &format!("Next p: {:?}", prop.orbit.t_next_p(state.sim_time)),
                    );
                    send_log(&mut evt, &format!("Period: {:?}", prop.orbit.period()));
                    send_log(
                        &mut evt,
                        &format!("Orbit count: {:?}", prop.orbit.orbit_number(state.sim_time)),
                    );
                }
            }
            OrbiterOrBody::Body(b) => {
                send_log(&mut evt, &format!("BD: {:?}", b));
            }
        }
    }
}

// I dislike bevy and so I'm lumping all input events into a single function
// because I am ungovernable
fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: EventReader<MouseWheel>,
    mut state: ResMut<GameState>,
    mut exit: ResMut<Events<bevy::app::AppExit>>,
    cstate: Res<CommandsState>,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    query: Query<&mut Transform, With<Camera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if cstate.active {
        return;
    }

    state.camera.on_keys(&keys, time.delta_secs());
    if !keys.pressed(KeyCode::ShiftLeft) {
        state.camera.on_scroll(scroll);
    }
    state.camera.on_mouse_click(&buttons);
    state.camera.on_mouse_move(windows);

    update_camera_transform(query, &mut state.camera);

    if let Some(p) = state.follow_position() {
        state.camera.track(p, CameraTracking::ExternalTrack);
    } else {
        let s = state.camera.cursor;
        state.camera.track(s, CameraTracking::TrackingCursor);
    }

    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Period => {
                state.sim_speed = i32::clamp(state.sim_speed + 1, -10, 4);
            }
            KeyCode::Comma => {
                state.sim_speed = i32::clamp(state.sim_speed - 1, -10, 4);
            }
            KeyCode::Delete => {
                state.delete_objects();
            }
            KeyCode::KeyH => {
                state.hide_debug = !state.hide_debug;
            }
            KeyCode::KeyK => {
                state.spawn_new();
            }
            KeyCode::KeyM => {
                let plan = state.maneuver_plans();
                if let Some(plan) = plan.iter().min_by_key(|e| (e.dv() * 1000.0) as i64) {
                    let id = state.primary();
                    state.enqueue_plan(id, plan);
                }
            }
            _ => (),
        }
    }

    let dv = if keys.pressed(KeyCode::ControlLeft) {
        0.002
    } else if keys.pressed(KeyCode::ShiftLeft) {
        0.5
    } else {
        0.03
    };

    let mut man = Vec2::ZERO;

    if keys.pressed(KeyCode::ArrowUp) {
        man += Vec2::Y * dv;
    }

    if keys.pressed(KeyCode::ArrowDown) {
        man -= Vec2::Y * dv;
    }

    if keys.pressed(KeyCode::ArrowLeft) {
        man -= Vec2::X * dv;
    }

    if keys.pressed(KeyCode::ArrowRight) {
        man += Vec2::X * dv;
    }

    if man != Vec2::ZERO {
        state.do_maneuver(man);
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
    let clicked = buttons.pressed(MouseButton::Left);
    let button_interact = if let Some(p) = state.camera.mouse_screen_pos {
        state.update_buttons(p, clicked)
    } else {
        false
    };

    if button_interact {
        return;
    }

    if buttons.just_pressed(MouseButton::Right) {
        state.control_points.clear();
        if let Some(p) = state.camera.mouse_pos() {
            state.control_points.push(p);
        }
    }
    if buttons.just_released(MouseButton::Right) {
        if let Some(p) = state.camera.mouse_pos() {
            state.control_points.push(p);
        }
    }
    if buttons.just_released(MouseButton::Left) {
        let hl = state.highlighted_list.clone();
        if keys.pressed(KeyCode::ShiftLeft) {
            // add to track list
            for h in hl {
                if !state.track_list.contains(&h) {
                    state.track_list.push(h);
                }
            }
        } else if keys.pressed(KeyCode::KeyX) {
            // remove from track list
            state.track_list.retain(|id| !hl.contains(id))
        } else {
            // start from scratch
            state.track_list.clear();
            state.track_list = hl;
        }
    }
}

fn load_new_scenario(state: &mut GameState, scen: Scenario, ids: ObjectIdTracker) {
    state.backup = Some((scen.clone(), ids, Nanotime::zero()));
    state.camera.target_scale = 0.001 * scen.system.body.soi;
    state.scenario = scen;
    state.sim_time = Nanotime::zero();
}

fn on_command(state: &mut GameState, cmd: &Vec<String>) {
    let starts_with = |s: &'static str| -> bool { cmd.first() == Some(&s.to_string()) };

    if starts_with("load") {
        let (system, ids) = match cmd.get(1).map(|s| s.as_str()) {
            Some("grid") => consistency_example(),
            Some("earth") => earth_moon_example_one(),
            Some("earth2") => earth_moon_example_two(),
            Some("moon") => just_the_moon(),
            Some("jupiter") => sun_jupiter_lagrange(),
            _ => {
                return;
            }
        };
        load_new_scenario(state, system, ids);
    } else if starts_with("restore") {
        if let Some((sys, ids, time)) = &state.backup {
            state.scenario = sys.clone();
            state.sim_time = *time;
            state.ids = *ids;
        }
    } else if starts_with("save") {
        state.backup = Some((state.scenario.clone(), state.ids, state.sim_time));
    } else if starts_with("follow") {
        state.follow = cmd.get(1).map(|s| s.parse::<i64>().ok()).flatten().map(|n| ObjectId(n));
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
    } else if starts_with("rm") {
        state.delete_objects();
    } else if starts_with("spawn") {
        state.spawn_new();
    } else if starts_with("export") {
        if let Some(orbit) = state.target_orbit() {
            let filename = format!("orbit-{:?}", state.sim_time);
            let filename = std::path::Path::new(&filename);
            match export_orbit_data(&orbit, filename) {
                Ok(_) => println!("Exported orbit data to {}", filename.display()),
                Err(e) => println!("Failed to export: {:?}", e),
            }
        } else {
            println!("No orbit to export.");
        }
    }
}

fn process_commands(mut evts: EventReader<DebugCommand>, mut state: ResMut<GameState>) {
    for DebugCommand(cmd) in evts.read() {
        on_command(&mut state, cmd);
    }
}
