use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use starling::prelude::*;

use crate::button::Button;
use crate::camera_controls::*;
use crate::debug::*;
use crate::drawing::*;
use crate::sprites::PlanetSpritePlugin;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlanetSpritePlugin {});
        app.add_systems(Startup, init_system);
        app.add_systems(FixedUpdate, propagate_system);
        app.add_systems(
            Update,
            (
                log_system_info,
                process_commands,
                keyboard_input,
                mouse_button_input,
                update_camera,
                draw,
            )
                .chain(),
        );
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
}

fn draw(gizmos: Gizmos, res: Res<GameState>) {
    draw_game_state(gizmos, &res)
}

#[derive(Debug, Clone, Copy)]
pub enum ShowOrbitsState {
    None,
    Focus,
    All,
}

impl ShowOrbitsState {
    fn next(&mut self) {
        let n = match self {
            ShowOrbitsState::None => ShowOrbitsState::Focus,
            ShowOrbitsState::Focus => ShowOrbitsState::All,
            ShowOrbitsState::All => ShowOrbitsState::None,
        };
        *self = n;
    }
}

fn update_camera(query: Query<&mut Transform, With<Camera>>, mut state: ResMut<GameState>) {
    update_camera_transform(query, &mut state.camera);
}

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
    pub topo_map: TopoMap,
    pub show_orbits: ShowOrbitsState,

    // buttons!
    pub show_potential_field: Button,
    pub show_animations: Button,
}

impl GameState {
    pub fn buttons(&self) -> Vec<&Button> {
        vec![&self.show_potential_field, &self.show_animations]
    }

    pub fn update_buttons(&mut self, pos: Vec2, clicked: bool) -> bool {
        self.show_potential_field.update(pos, clicked) | self.show_animations.update(pos, clicked)
    }

    pub fn primary(&self) -> Option<ObjectId> {
        self.track_list.first().cloned()
    }

    pub fn toggle_track(&mut self, id: ObjectId) {
        if self.track_list.contains(&id) {
            self.track_list.retain(|e| *e != id);
        } else {
            self.track_list.insert(0, id);
        }
    }

    pub fn planned_maneuvers(&self, after: Nanotime) -> Vec<(ObjectId, Nanotime, Vec2)> {
        let mut dvs = vec![];
        for ctrl in &self.controllers {
            if let Some(plan) = ctrl.plan() {
                for node in &plan.nodes {
                    if node.stamp > after {
                        dvs.push((ctrl.target, node.stamp, node.impulse.vel));
                    }
                }
            }
        }
        dvs.sort_by_key(|(_, t, _)| t.inner());
        dvs
    }

    pub fn cursor_pv(&self) -> Option<PV> {
        let p1 = *self.control_points.get(0)?;
        let p2 = self
            .control_points
            .get(1)
            .map(|e| *e)
            .or(self.camera.mouse_pos())?;

        if p1.distance(p2) < 20.0 {
            return None;
        }

        let wrt_id = self.scenario.relevant_body(p1, self.sim_time)?;
        let parent = self.scenario.lup(wrt_id, self.sim_time)?;

        let r = p1.distance(parent.pv().pos);
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::new(p1, (p2 - p1) * v / r))
    }

    pub fn target_orbit(&self) -> Option<(ObjectId, SparseOrbit)> {
        let pv = self.cursor_pv()?;
        let parent_id = self.scenario.relevant_body(pv.pos, self.sim_time)?;
        let parent = self.scenario.lup(parent_id, self.sim_time)?;
        let parent_pv = parent.pv();
        let pv = pv - PV::pos(parent_pv.pos);
        let body = parent.body()?;
        Some((parent_id, SparseOrbit::from_pv(pv, body, self.sim_time)?))
    }

    pub fn primary_orbit(&self) -> Option<(ObjectId, SparseOrbit)> {
        let lup = self.scenario.lup(self.primary()?, self.sim_time)?;
        if let Some(o) = lup.orbiter() {
            let prop = o.propagator_at(self.sim_time)?;
            Some((prop.parent, prop.orbit))
        } else {
            None
        }
    }

    pub fn follow_position(&self) -> Option<Vec2> {
        let id = self.follow?;
        let lup = self.scenario.lup(id, self.sim_time)?;
        Some(lup.pv().pos)
    }

    pub fn spawn_new(&mut self) -> Option<()> {
        let (parent, orbit) = self.target_orbit().or_else(|| self.primary_orbit())?;
        let pv_local = orbit.pv(self.sim_time).ok()?;
        let perturb = PV::new(
            randvec(pv_local.pos.length() * 0.005, pv_local.pos.length() * 0.02),
            randvec(pv_local.vel.length() * 0.005, pv_local.vel.length() * 0.02),
        );
        let orbit = SparseOrbit::from_pv(pv_local + perturb, orbit.body, self.sim_time)?;
        let id = self.ids.next();
        self.scenario.add_object(id, parent, orbit, self.sim_time);
        Some(())
    }

    pub fn delete_objects(&mut self) {
        self.track_list.iter().for_each(|i| {
            self.scenario.remove_object(*i);
        });
    }

    pub fn do_maneuver(&mut self, dv: Vec2) -> Option<()> {
        if self.paused {
            return Some(());
        }
        for id in &self.track_list {
            match self.scenario.dv(*id, self.sim_time, dv) {
                Some(()) => (),
                None => {
                    if self
                        .scenario
                        .lup(*id, self.sim_time)
                        .map(|lup| lup.orbiter())
                        .flatten()
                        .is_some()
                    {
                        println!("{:?} - Failed to maneuver orbiter {}", self.sim_time, id);
                    }
                }
            };
        }
        self.scenario.simulate(self.sim_time, self.physics_duration);
        Some(())
    }

    pub fn maneuver_plans(&self) -> Vec<(ObjectId, ManeuverPlan)> {
        self.track_list
            .iter()
            .filter_map(|id| {
                let (parent_id, dst) = self.target_orbit()?;
                let orbiter = self.scenario.lup(*id, self.sim_time)?.orbiter()?;
                let prop = orbiter.propagator_at(self.sim_time)?;
                let src = (prop.parent == parent_id).then_some(prop.orbit)?;

                let mut plans = generate_maneuver_plans(&src, &dst, self.sim_time);

                plans.sort_by_key(|m| (m.dv() * 1000.0) as i32);

                let ret = plans.first()?;
                Some((*id, ret.clone()))
            })
            .collect::<Vec<_>>()
    }

    pub fn enqueue_plan(&mut self, id: ObjectId, plan: &ManeuverPlan) {
        self.controllers.retain(|c| c.target != id);
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

        let mut topo_map = TopoMap::new(250.0);
        for x in -50..=50 {
            for y in -50..=50 {
                topo_map.add_bin(IVec2::new(x, y));
            }
        }

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
            draw_levels: vec![150],
            camera: CameraState::default(),
            control_points: Vec::new(),
            hide_debug: true,
            duty_cycle_high: false,
            controllers: vec![],
            follow: None,
            topo_map,
            show_orbits: ShowOrbitsState::Focus,

            // buttons
            show_potential_field: next_button("Show Potential"),
            show_animations: next_button("Show Animations"),
        }
    }
}

fn propagate_system(time: Res<Time>, mut state: ResMut<GameState>) {
    let old_sim_time = state.sim_time;
    state.actual_time += Nanotime::nanos(time.delta().as_nanos() as i64);
    if !state.paused {
        let sp = 10.0f32.powi(state.sim_speed);
        state.sim_time += Nanotime::nanos((time.delta().as_nanos() as f32 * sp) as i64);
    }

    state.duty_cycle_high = time.elapsed().as_millis() % 1000 < 500;

    let s = state.sim_time;
    let d = state.physics_duration;

    let mut man = state.planned_maneuvers(old_sim_time);
    while let Some((id, t, dv)) = man.first() {
        if s > *t {
            let perturb = randvec(0.1, 0.3);
            state.scenario.simulate(*t, d);
            state.scenario.dv(*id, *t, dv + perturb);
        } else {
            break;
        }
        man.remove(0);
    }

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
            .all_ids()
            .into_iter()
            .filter_map(|id| {
                let pv = state.scenario.lup(id, state.sim_time)?.pv();
                a.contains(pv.pos).then(|| id)
            })
            .collect();
    } else {
        state.highlighted_list.clear();
    }

    let mut track_list = state.track_list.clone();
    track_list.retain(|o| state.scenario.lup(*o, state.sim_time).is_some());
    state.track_list = track_list;

    let ids = state.scenario.ids().collect::<Vec<_>>();

    state.controllers.retain(|c| {
        if !ids.contains(&c.target) {
            return false;
        }
        if let Some(end) = c.plan().map(|p| p.end()).flatten() {
            let retain = end > s;
            if !retain {
                println!("Maneuver completed by vehicle {}", c.target);
            }
            retain
        } else {
            true
        }
    });

    if state.show_potential_field.state() {
        let orbits = state
            .track_list
            .iter()
            .filter_map(|id| {
                let lup = state.scenario.lup(*id, state.sim_time)?.orbiter()?;
                Some(lup.propagator_at(state.sim_time)?.orbit)
            })
            .collect::<Vec<_>>();

        let scalar_field = |p: Vec2| -> f32 {
            orbits
                .iter()
                .map(|o| (o.sdf(p) * 1000.0) as i32)
                .min()
                .unwrap_or(0) as f32
                / 1000.0
        };

        if orbits.is_empty() {
            state.topo_map.clear();
        }

        let a = state.actual_time;
        if s - state.topo_map.last_updated > Nanotime::millis(1) {
            state.topo_map.update(a, &scalar_field, &[100.0, 150.0]);
        }
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
    let mut log = |str: &str| {
        send_log(&mut evt, str);
    };

    log("Show/hide info - [H]");

    if state.hide_debug {
        return;
    }

    let logs = [
        "",
        "Look around - [W][A][S][D]",
        "Control orbiter - Arrow Keys",
        "  Increase thrust - hold [LSHIFT]",
        "  Decrease thrust - hold [LCTRL]",
        "Zoom in/out - +/-, [Scroll]",
        "Select spacecraft - Left click and drag",
        "Set target orbit - Right click and drag",
        "Send spacecraft to orbit - [ENTER]",
        "Toggle orbit draw modes - [TAB]",
        "Increase sim speed - [.]",
        "Decrease sim speed - [,]",
        "Pause - [SPACE]",
        "",
    ];

    for s in logs {
        log(s);
    }

    log(&format!("Epoch: {:?}", state.sim_time));

    if state.paused {
        log("Paused");
    }
    log(&format!(
        "Sim speed: 10^{} [{}]",
        state.sim_speed,
        sim_speed_str(state.sim_speed)
    ));

    if state.track_list.len() > 15 {
        log(&format!("Tracks: lots of em"));
    } else {
        log(&format!("Tracks: {:?}", state.track_list));
    }
    log(&format!("Physics: {:?}", state.physics_duration));
    log(&format!("Scale: {:0.3}", state.camera.actual_scale));

    if let Some(pv) = state.cursor_pv() {
        log(&format!("{:0.3}", pv));
    }

    log(&format!("Ctlrs: {}", state.controllers.len()));

    {
        for (id, t, dv) in state.planned_maneuvers(state.sim_time) {
            log(&format!("- {} {:?} {}", id, t, dv))
        }
    }

    log(&format!("Orbiters: {}", state.scenario.orbiter_count()));
    log(&format!("Propagators: {}", state.scenario.prop_count()));

    if let Some(lup) = state
        .primary()
        .map(|id| state.scenario.lup(id, state.sim_time))
        .flatten()
    {
        if let Some(o) = lup.orbiter() {
            for prop in o.props() {
                log(&format!("- [{}]", prop));
            }
            if let Some(prop) = o.propagator_at(state.sim_time) {
                log(&format!("{:#?}", prop.orbit));
                log(&format!(
                    "Next p: {:?}",
                    prop.orbit.t_next_p(state.sim_time)
                ));
                log(&format!("Period: {:?}", prop.orbit.period()));
                log(&format!(
                    "Orbit count: {:?}",
                    prop.orbit.orbit_number(state.sim_time)
                ));
            }
        } else if let Some(b) = lup.body() {
            log(&format!("BD: {:?}", b));
        }
    }
}

// I dislike bevy and so I'm lumping all input events into a single function
// because I am ungovernable
fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<MouseWheel>,
    mut state: ResMut<GameState>,
    mut exit: ResMut<Events<bevy::app::AppExit>>,
    cstate: Res<CommandsState>,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if cstate.active {
        return;
    }

    let scroll_events = scroll.read().collect::<Vec<_>>();

    state.camera.on_keys(&keys, time.delta_secs());
    if !keys.pressed(KeyCode::ShiftLeft) {
        state.camera.on_scroll(&scroll_events);
    }
    state.camera.on_mouse_click(&buttons);
    state.camera.on_mouse_move(windows);

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
            KeyCode::KeyF => state.follow = state.primary(),
            KeyCode::Enter => {
                let plan = state.maneuver_plans();
                if !plan.is_empty() {
                    for (id, plan) in &plan {
                        state.enqueue_plan(*id, &plan);
                    }
                    let ids = plan.iter().map(|(id, _)| id).collect::<Vec<_>>();
                    let avg_dv =
                        plan.iter().map(|(_, plan)| plan.dv()).sum::<f32>() / plan.len() as f32;
                    println!("Committing maneuver plan (avg dv of {avg_dv:0.2}) for {ids:?}");
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

    if keys.pressed(KeyCode::ShiftLeft) {
        for ev in scroll_events {
            if ev.y > 0.0 {
                state.sim_speed = i32::clamp(state.sim_speed + 1, -10, 4);
            } else {
                state.sim_speed = i32::clamp(state.sim_speed - 1, -10, 4);
            }
        }
    }

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

    if keys.pressed(KeyCode::KeyK) {
        state.spawn_new();
    }

    if keys.just_pressed(KeyCode::Tab) {
        state.show_orbits.next();
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
    state.camera.center = Vec2::ZERO;
    state.scenario = scen;
    state.ids = ids;
    state.sim_time = Nanotime::zero();
    state.track_list.clear();
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
        state.follow = cmd
            .get(1)
            .map(|s| s.parse::<i64>().ok())
            .flatten()
            .map(|n| ObjectId(n));
    } else if starts_with("track") {
        for n in cmd.iter().skip(1).filter_map(|s| s.parse::<i64>().ok()) {
            let id = ObjectId(n);
            state.toggle_track(id);
        }
    } else if starts_with("spawn") {
        state.spawn_new();
    }
}

fn process_commands(mut evts: EventReader<DebugCommand>, mut state: ResMut<GameState>) {
    for DebugCommand(cmd) in evts.read() {
        on_command(&mut state, cmd);
    }
}
