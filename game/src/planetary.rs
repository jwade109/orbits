use bevy::prelude::*;

use starling::prelude::*;

use crate::camera_controls::*;
use crate::debug::*;
use crate::drawing::*;
use crate::mouse::*;
use crate::ui::InteractionEvent;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(400.0));

        app.add_systems(Startup, init_system);
        app.add_systems(FixedUpdate, propagate_system);

        app.add_systems(Update, log_system_info);

        app.add_systems(
            Update,
            (
                crate::keybindings::keyboard_input,
                mouse_button_input,
                handle_interactions,
                handle_camera_interactions,
                update_camera_controllers,
                todo_fix_actual_scale,
                crate::mouse::cursor_position,
                draw,
                draw_mouse_state,

                crate::drawing::draw_mouse_state,
            )
                .chain(),
        );
    }
}

#[derive(Component, Default)]
pub struct SoftController(pub Transform);

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
    commands.spawn((Camera2d, SoftController::default()));
    commands.spawn(MouseState::default());
}

fn update_camera_controllers(mut query: Query<(&SoftController, &mut Transform)>) {
    for (ctrl, mut tf) in &mut query {
        let target = ctrl.0;
        let current = *tf;
        tf.translation += (target.translation - current.translation) * 0.1;
        tf.scale += (target.scale - current.scale) * 0.1;
    }
}

fn todo_fix_actual_scale(mut state: ResMut<GameState>, query: Query<&Transform, With<Camera>>) {
    if let Ok(tf) = query.get_single() {
        state.camera.actual_scale = tf.scale.z;
    }
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
    pub camera: CameraState,
    pub control_points: Vec<Vec2>,
    pub hide_debug: bool,
    pub duty_cycle_high: bool,
    pub controllers: Vec<Controller>,
    pub follow: Option<ObjectId>,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub selection_mode: bool,
    pub target_mode: bool,
}

impl GameState {
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

    pub fn highlighted(&self) -> Vec<ObjectId> {
        if let Some(a) = self.camera.selection_region() {
            self.scenario
                .all_ids()
                .into_iter()
                .filter_map(|id| {
                    let pv = self.scenario.lup(id, self.sim_time)?.pv();
                    a.contains(pv.pos).then(|| id)
                })
                .collect()
        } else {
            vec![]
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

        GameState {
            sim_time: Nanotime::zero(),
            actual_time: Nanotime::zero(),
            physics_duration: Nanotime::secs(120),
            sim_speed: 0,
            paused: false,
            scenario: scenario.clone(),
            ids,
            track_list: Vec::new(),
            backup: Some((scenario, ids, Nanotime::zero())),
            camera: CameraState::default(),
            control_points: Vec::new(),
            hide_debug: true,
            duty_cycle_high: false,
            controllers: vec![],
            follow: None,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: false,
            selection_mode: false,
            target_mode: false,
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

    let ids = state.scenario.ids().collect::<Vec<_>>();

    state.controllers.retain(|c| {
        if !ids.contains(&c.target) {
            return false;
        }
        if let Some(end) = c.plan().map(|p| p.end()).flatten() {
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

fn process_interaction(
    inter: &InteractionEvent,
    state: &mut GameState,
    exit: &mut EventWriter<bevy::app::AppExit>,
) -> Option<()> {
    match inter {
        InteractionEvent::Delete => state.delete_objects(),
        InteractionEvent::CommitMission => {
            let plan = state.maneuver_plans();
            if !plan.is_empty() {
                for (id, plan) in &plan {
                    state.enqueue_plan(*id, &plan);
                }
                let ids = plan.iter().map(|(id, _)| id).collect::<Vec<_>>();
                let avg_dv =
                    plan.iter().map(|(_, plan)| plan.dv()).sum::<f32>() / plan.len() as f32;
                info!("Committing maneuver plan (avg dv of {avg_dv:0.2}) for {ids:?}");
            }
        }
        InteractionEvent::ToggleDebugMode => {
            state.hide_debug = !state.hide_debug;
        }
        InteractionEvent::ClearSelection => {
            state.track_list.clear();
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
        InteractionEvent::ToggleTargetMode => {
            state.target_mode = !state.target_mode;
        }
        InteractionEvent::ToggleSelectionMode => {
            state.selection_mode = !state.selection_mode;
        }
        InteractionEvent::ExitApp => {
            exit.send(bevy::app::AppExit::Success);
        }
        InteractionEvent::Save => {
            state.backup = Some((state.scenario.clone(), state.ids, state.sim_time));
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
fn mouse_button_input(buttons: Res<ButtonInput<MouseButton>>, mut state: ResMut<GameState>) {
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
    if buttons.just_pressed(MouseButton::Left) {}
    if buttons.just_released(MouseButton::Left) {
        for h in state.highlighted() {
            if !state.track_list.contains(&h) {
                state.track_list.push(h);
            }
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
