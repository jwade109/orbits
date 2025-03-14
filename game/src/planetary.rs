use bevy::prelude::*;

use starling::prelude::*;

use crate::camera_controls::*;
use crate::debug::*;
use crate::ui::InteractionEvent;
use bevy::core_pipeline::bloom::Bloom;
use std::collections::{HashMap, HashSet};

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        // app.add_systems(FixedUpdate, );
        app.add_systems(Update, log_system_info);

        app.add_systems(
            Update,
            (
                propagate_system,

                update_camera_controllers,

                crate::keybindings::keyboard_input,
                track_highlighted_objects,
                handle_interactions,
                handle_camera_interactions,
                update_mouse_interactions,
                todo_fix_actual_scale,

                update_camera_controllers,

                crate::sprites::make_new_sprites,
                crate::sprites::update_planet_sprites,
                crate::sprites::update_shadow_sprites,
                crate::sprites::update_background_sprite,
                crate::sprites::update_spacecraft_sprites,

                update_camera_controllers,

                crate::mouse::update_mouse_state,
                crate::drawing::draw_mouse_state,
                crate::drawing::draw_game_state,

                update_camera_controllers,
            )
                .chain(),
        );
    }
}

#[derive(Component, Default)]
pub struct SoftController(pub Transform);

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            ..default()
        },
        SoftController::default(),
        Bloom {
            intensity: 0.5,
            ..Bloom::OLD_SCHOOL
        },
    ));
    commands.spawn(crate::mouse::MouseState::default());
}

fn update_camera_controllers(mut query: Query<(&SoftController, &mut Transform)>) {
    for (ctrl, mut tf) in &mut query {
        let target = ctrl.0;
        let current = *tf;
        tf.translation += (target.translation - current.translation) * 0.1;
        tf.scale += (target.scale - current.scale) * 0.1;
    }
}

fn todo_fix_actual_scale(
    mut state: ResMut<GameState>,
    query: Query<&Transform, With<Camera>>,
    window: Single<&Window>,
) {
    if let Ok(tf) = query.get_single() {
        state.camera.actual_scale = tf.scale.z;
        state.camera.world_center = tf.translation.xy();
        state.camera.window_dims = Vec2::new(window.width(), window.height());
    }
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
    pub track_list: HashSet<ObjectId>,
    pub camera: CameraState,
    pub control_points: Vec<Vec2>,
    pub hide_debug: bool,
    pub duty_cycle_high: bool,
    pub controllers: Vec<Controller>,
    pub follow: Option<ObjectId>,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub selection_region: Option<Region>,
    pub queued_orbits: Vec<(ObjectId, SparseOrbit)>,
    pub constellations: HashMap<GroupId, HashSet<ObjectId>>,
}

impl Default for GameState {
    fn default() -> Self {
        let (scenario, ids) = default_example();

        GameState {
            sim_time: Nanotime::zero(),
            actual_time: Nanotime::zero(),
            physics_duration: Nanotime::secs(120),
            sim_speed: 0,
            paused: false,
            scenario: scenario.clone(),
            ids,
            track_list: HashSet::new(),
            backup: Some((scenario, ids, Nanotime::zero())),
            camera: CameraState::default(),
            control_points: Vec::new(),
            hide_debug: true,
            duty_cycle_high: false,
            controllers: vec![],
            follow: None,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: false,
            selection_region: None,
            queued_orbits: Vec::new(),
            constellations: HashMap::from([(
                GroupId(45),
                HashSet::from([ObjectId(12), ObjectId(30), ObjectId(60)]),
            )]),
        }
    }
}

impl GameState {
    pub fn primary(&self) -> Option<ObjectId> {
        self.track_list.iter().next().cloned()
    }

    pub fn toggle_track(&mut self, id: ObjectId) {
        if self.track_list.contains(&id) {
            self.track_list.retain(|e| *e != id);
        } else {
            self.track_list.insert(id);
        }
    }

    pub fn is_tracked(&self, id: ObjectId) -> bool {
        self.track_list.contains(&id)
    }

    pub fn toggle_group(&mut self, gid: GroupId) -> Option<()> {
        // - if any of the orbiters in the group are not selected,
        //   select all of them
        // - if all of them are already selected, deselect all of them

        let members = self.constellations.get(&gid)?;

        dbg!(members);

        let all_selected = members.iter().all(|id| self.is_tracked(*id));

        dbg!(all_selected);

        for id in members {
            if all_selected {
                self.track_list.remove(id);
            } else {
                self.track_list.insert(*id);
            }
        }

        Some(())
    }

    pub fn planned_maneuvers(&self, after: Nanotime) -> Vec<(ObjectId, Nanotime, Vec2)> {
        let mut dvs = vec![];
        for ctrl in &self.controllers {
            if let Some(plan) = ctrl.plan() {
                for (stamp, impulse) in plan.future_dvs(after) {
                    dvs.push((ctrl.target, stamp, impulse));
                }
            }
        }
        dvs.sort_by_key(|(_, t, _)| t.inner());
        dvs
    }

    pub fn cursor_pv(&self) -> Option<PV> {
        let p1 = *self.control_points.get(0)?;
        let p2 = *self.control_points.get(1)?;

        if p1.distance(p2) < 20.0 {
            return None;
        }

        let wrt_id = self.scenario.relevant_body(p1, self.sim_time)?;
        let parent = self.scenario.lup(wrt_id, self.sim_time)?;

        let r = p1.distance(parent.pv().pos);
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::new(p1, (p2 - p1) * v / r))
    }

    pub fn cursor_orbit(&self) -> Option<(ObjectId, SparseOrbit)> {
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
        let (parent, orbit) = self.cursor_orbit().or_else(|| self.primary_orbit())?;
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

    pub fn highlighted(&self) -> HashSet<ObjectId> {
        if let Some(a) = self.selection_region {
            self.scenario
                .all_ids()
                .into_iter()
                .filter_map(|id| {
                    let pv = self.scenario.lup(id, self.sim_time)?.pv();
                    a.contains(pv.pos).then(|| id)
                })
                .collect()
        } else {
            HashSet::new()
        }
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
                        info!("{:?} - Failed to maneuver orbiter {}", self.sim_time, id);
                    }
                }
            };
        }
        self.scenario.simulate(self.sim_time, self.physics_duration);
        Some(())
    }

    pub fn command_selected(&mut self, next: &SparseOrbit) {
        for id in self.track_list.clone() {
            self.command(id, next);
        }
    }

    pub fn command(&mut self, id: ObjectId, next: &SparseOrbit) -> Option<()> {
        if self.controllers.iter().find(|c| c.target == id).is_none() {
            self.controllers.push(Controller::idle(id));
        }

        if let Some(c) = self.controllers.iter_mut().find(|c| c.target == id) {
            if c.is_idle() {
                let orbiter = self.scenario.lup(c.target(), self.sim_time)?.orbiter()?;
                let current = orbiter.propagator_at(self.sim_time)?.orbit;
                c.activate(&current, next, self.sim_time);
            } else {
                c.enqueue(next);
            }
        }

        Some(())
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
            let perturb = randvec(0.01, 0.05);
            state.scenario.simulate(*t, d);
            state.scenario.dv(*id, *t, dv + perturb);
        } else {
            break;
        }
        man.remove(0);
    }

    for (id, ri) in state.scenario.simulate(s, d) {
        if let Some(ri) = ri {
            info!(
                "Object {} removed at time {:?} due to {:?}",
                id, ri.stamp, ri.reason
            );
        } else {
            info!("Object {} removed for unknown reason", id);
        }
    }

    let mut track_list = state.track_list.clone();
    track_list.retain(|o| state.scenario.lup(*o, state.sim_time).is_some());
    state.track_list = track_list;

    let ids: Vec<_> = state.scenario.orbiter_ids().collect();

    for (_, members) in &mut state.constellations {
        members.retain(|id| ids.contains(id));
    }

    state
        .constellations
        .retain(|_, members| !members.is_empty());

    state.controllers.retain(|c| {
        if !ids.contains(&c.target) {
            return false;
        }
        if c.is_idle() {
            info!("Vehicle {} has idle controller", c.target);
            return false;
        }
        if let Some(end) = c.plan().map(|p| p.end()) {
            let retain = end > s;
            if !retain {
                info!("Maneuver completed by vehicle {}", c.target);
            }
            retain
        } else {
            true
        }
    });
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

    let mut show_id_list = |ids: &HashSet<ObjectId>, name: &str| {
        if ids.len() > 15 {
            log(&format!("{}: {} ...", name, ids.len()));
        } else {
            log(&format!("{}: {} {:?}", name, ids.len(), ids));
        }
    };

    show_id_list(&state.track_list, "Tracks");
    show_id_list(&state.highlighted(), "Select");

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

fn update_mouse_interactions(
    mut state: ResMut<GameState>,
    mouse: Single<&crate::mouse::MouseState>,
) {
    state.control_points = mouse
        .right_world()
        .into_iter()
        .chain(mouse.current_world().into_iter())
        .collect();

    state.selection_region = if let Some((a, b)) = mouse.left_world().zip(mouse.current_world()) {
        Some(Region::aabb(a, b))
    } else if let Some((a, b)) = mouse.middle_world().zip(mouse.current_world()) {
        Some(Region::range(a, b))
    } else {
        None
    }
}

fn process_interaction(
    inter: &InteractionEvent,
    state: &mut GameState,
    exit: &mut EventWriter<bevy::app::AppExit>,
) -> Option<()> {
    match inter {
        InteractionEvent::Delete => state.delete_objects(),
        InteractionEvent::CommitMission => {
            for (_, orbit) in state.queued_orbits.clone() {
                state.command_selected(&orbit)
            }
        }
        InteractionEvent::ToggleDebugMode => {
            state.hide_debug = !state.hide_debug;
        }
        InteractionEvent::ClearSelection => {
            state.track_list.clear();
        }
        InteractionEvent::ClearOrbitQueue => {
            state.queued_orbits.clear();
        }
        InteractionEvent::SimSlower => {
            state.sim_speed = i32::clamp(state.sim_speed - 1, -10, 4);
        }
        InteractionEvent::SimFaster => {
            state.sim_speed = i32::clamp(state.sim_speed + 1, -10, 4);
        }
        InteractionEvent::SimPause => {
            state.paused = !state.paused;
        }
        InteractionEvent::Follow => {
            state.follow = state.primary();
        }
        InteractionEvent::Orbits => {
            state.show_orbits.next();
        }
        InteractionEvent::Spawn => {
            state.spawn_new();
        }
        InteractionEvent::DoubleClick(p) => {
            let w = state
                .camera
                .viewport_bounds()
                .map(state.camera.world_bounds(), *p);
            let id = state.scenario.nearest(w, state.sim_time);
            if let Some(id) = id {
                state.follow = Some(id)
            }
        }
        InteractionEvent::ExitApp => {
            exit.send(bevy::app::AppExit::Success);
        }
        InteractionEvent::Save => {
            state.backup = Some((state.scenario.clone(), state.ids, state.sim_time));
        }
        InteractionEvent::QueueOrbit => {
            if let Some(o) = state.cursor_orbit() {
                state.queued_orbits.push(o);
            }
        }
        InteractionEvent::Restore => {
            if let Some((sys, ids, time)) = &state.backup {
                state.scenario = sys.clone();
                state.sim_time = *time;
                state.ids = *ids;
            }
        }
        InteractionEvent::Load(name) => {
            let (system, ids) = match name.as_str() {
                "grid" => Some(consistency_example()),
                "earth" => Some(earth_moon_example_one()),
                "earth2" => Some(earth_moon_example_two()),
                "moon" => Some(just_the_moon()),
                "jupiter" => Some(sun_jupiter_lagrange()),
                _ => {
                    error!("No scenario named {}", name);
                    None
                }
            }?;
            load_new_scenario(state, system, ids);
        }
        InteractionEvent::ToggleObject(id) => {
            state.toggle_track(*id);
        }
        InteractionEvent::ToggleGroup(gid) => {
            state.toggle_group(*gid);
        }
        InteractionEvent::ThrustUp => {
            state.do_maneuver(Vec2::Y * 0.03);
        }
        InteractionEvent::ThrustDown => {
            state.do_maneuver(-Vec2::Y * 0.03);
        }
        InteractionEvent::ThrustLeft => {
            state.do_maneuver(-Vec2::X * 0.03);
        }
        InteractionEvent::ThrustRight => {
            state.do_maneuver(Vec2::X * 0.03);
        }
        InteractionEvent::Reset
        | InteractionEvent::MoveLeft
        | InteractionEvent::MoveRight
        | InteractionEvent::MoveUp
        | InteractionEvent::MoveDown => state.follow = None,
        _ => (),
    };
    Some(())
}

fn handle_interactions(
    mut events: EventReader<InteractionEvent>,
    mut state: ResMut<GameState>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    for e in events.read() {
        info!("Interaction event: {e:?}");
        process_interaction(e, &mut state, &mut exit);
    }
}

fn handle_camera_interactions(
    mut events: EventReader<InteractionEvent>,
    mut query: Query<&mut SoftController>,
    state: Res<GameState>,
    time: Res<Time>,
) {
    let mut ctrl = match query.get_single_mut() {
        Ok(c) => c,
        Err(e) => {
            error!("{:?}", e);
            return;
        }
    };

    let cursor_delta = 1400.0 * time.delta_secs() * ctrl.0.scale.z;
    let scale_scalar = 1.5;

    if let Some(p) = state.follow_position() {
        ctrl.0.translation = p.extend(0.0);
    }

    for e in events.read() {
        match e {
            InteractionEvent::MoveLeft => ctrl.0.translation.x -= cursor_delta,
            InteractionEvent::MoveRight => ctrl.0.translation.x += cursor_delta,
            InteractionEvent::MoveUp => ctrl.0.translation.y += cursor_delta,
            InteractionEvent::MoveDown => ctrl.0.translation.y -= cursor_delta,
            InteractionEvent::ZoomIn => ctrl.0.scale /= scale_scalar,
            InteractionEvent::ZoomOut => ctrl.0.scale *= scale_scalar,
            InteractionEvent::Reset => ctrl.0 = Transform::IDENTITY,
            _ => (),
        }
    }
}

// TODO get rid of this
fn track_highlighted_objects(buttons: Res<ButtonInput<MouseButton>>, mut state: ResMut<GameState>) {
    if buttons.just_released(MouseButton::Left) || buttons.just_released(MouseButton::Middle) {
        let h = state.highlighted();
        state.track_list.extend(h.into_iter());
    }
}

fn load_new_scenario(state: &mut GameState, scen: Scenario, ids: ObjectIdTracker) {
    state.backup = Some((scen.clone(), ids, Nanotime::zero()));
    state.scenario = scen;
    state.ids = ids;
    state.sim_time = Nanotime::zero();
    state.track_list.clear();
}
