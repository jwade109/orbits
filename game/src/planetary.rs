use crate::args::ProgramContext;
use crate::mouse::{FrameId, InputState, MouseButt};
use crate::notifications::*;
use crate::onclick::OnClick;
use crate::scenes::{
    CameraProjection, CommsContext, CursorMode, EditorContext, Interactive, OrbitalContext,
    RPOContext, Render, Scene, SceneType, StaticSpriteDescriptor, TelescopeContext, TextLabel,
};
use crate::ui::InteractionEvent;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::window::WindowMode;
use clap::Parser;
use enum_iterator::next_cycle;
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);

        app.add_systems(
            Update,
            (
                crate::keybindings::keyboard_input,
                handle_interactions,
                crate::mouse::update_input_state,
                // physics
                step_system,
                // rendering
                crate::ui::do_text_labels,
                crate::sprites::update_static_sprites,
                crate::sprites::update_shadow_sprites,
                crate::sprites::update_background_sprite,
                crate::sprites::update_spacecraft_sprites,
                crate::drawing::draw_game_state,
            )
                .chain(),
        );
    }
}

#[derive(Component, Debug)]
pub struct BackgroundCamera;

fn init_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let g = GameState::default();

    let path = std::fs::canonicalize(g.args.audio_dir().join("button-down.ogg")).unwrap();

    commands.spawn((AudioPlayer::new(asset_server.load(path)),));

    commands.insert_resource(g);
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            order: 0,
            clear_color: ClearColorConfig::Custom(BLACK.with_alpha(0.0).into()),
            ..default()
        },
        Bloom {
            intensity: 0.2,
            ..Bloom::OLD_SCHOOL
        },
        BackgroundCamera,
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            order: 1,
            clear_color: ClearColorConfig::Custom(BLACK.with_alpha(0.0).into()),
            ..default()
        },
        RenderLayers::layer(1),
    ));
}

#[derive(Resource, Debug)]
pub struct GameState {
    /// Contains all states related to window size, mouse clicks and positions,
    /// and button presses and holds.
    pub input: InputState,

    /// Contains CLI arguments
    pub args: ProgramContext,

    /// Stores information and provides an API for interacting with the simulation
    /// from the perspective of a global solar/planetary system view.
    ///
    /// Additional information allows the user to select spacecraft and
    /// direct them to particular orbits, or manually pilot them.
    pub orbital_context: OrbitalContext,

    pub telescope_context: TelescopeContext,

    pub rpo_context: RPOContext,

    pub editor_context: EditorContext,

    pub coms_context: CommsContext,

    /// Simulation clock
    pub sim_time: Nanotime,

    /// Wall clock, i.e. time since program began.
    pub wall_time: Nanotime,

    pub physics_duration: Nanotime,
    pub sim_speed: i32,
    pub paused: bool,

    /// Representation of the solar system and all of the spacecraft
    /// and other objects contained therein.
    ///
    /// TODO replace this with a flat data structure that can be
    /// expanded during runtime, and store multiple (potentially
    /// disjoint) solar systems.
    pub scenario: Scenario,

    /// Map of names to parts to their definitions. Loaded from
    /// the assets/parts directory
    pub part_database: HashMap<String, PartProto>,

    /// Stupid thing to generate unique increasing IDs for
    /// planets and orbiters
    pub ids: ObjectIdTracker,

    pub controllers: Vec<Controller>,
    pub constellations: HashMap<OrbiterId, GroupId>,
    pub orbital_vehicles: HashMap<OrbiterId, Vehicle>,
    pub starfield: Vec<(Vec3, Srgba, f32, f32)>,
    pub rpos: HashMap<OrbiterId, RPO>,

    pub scenes: Vec<Scene>,
    pub current_scene_idx: usize,
    pub current_orbit: Option<usize>,

    pub redraw_requested: bool,
    pub last_redraw: Nanotime,
    pub ui: Tree<OnClick>,
    last_hover_ui: Option<OnClick>,

    pub notifications: Vec<Notification>,

    pub is_exit_prompt: bool,
}

fn generate_starfield() -> Vec<(Vec3, Srgba, f32, f32)> {
    (0..1000)
        .map(|_| {
            let s = rand(0.0, 2.0);
            let color = if s < 1.0 {
                RED.mix(&YELLOW, s)
            } else {
                WHITE.mix(&TEAL, s - 1.0)
            };
            (
                randvec3(1000.0, 12000.0),
                color,
                rand(3.0, 9.0),
                rand(700.0, 1900.0),
            )
        })
        .collect()
}

impl Default for GameState {
    fn default() -> Self {
        let (scenario, ids) = default_example();

        let args = match ProgramContext::try_parse() {
            Ok(a) => a,
            Err(e) => {
                dbg!(e);
                ProgramContext::default()
            }
        };

        let mut g = GameState {
            input: InputState::default(),
            args: args.clone(),
            orbital_context: OrbitalContext::new(PlanetId(0)),
            telescope_context: TelescopeContext::new(),
            rpo_context: RPOContext::new(),
            editor_context: EditorContext::new(),
            coms_context: CommsContext::default(),
            sim_time: Nanotime::zero(),
            wall_time: Nanotime::zero(),
            physics_duration: Nanotime::days(7),
            sim_speed: 0,
            paused: false,
            scenario: scenario.clone(),
            part_database: load_parts_from_dir(&args.parts_dir()),
            ids,
            controllers: vec![],
            orbital_vehicles: HashMap::new(),
            constellations: HashMap::new(),
            starfield: generate_starfield(),
            rpos: HashMap::new(),
            scenes: vec![
                Scene::main_menu(),
                Scene::orbital(),
                Scene::docking(),
                Scene::telescope(),
                Scene::editor(),
                Scene::comms(),
            ],
            current_scene_idx: 0,
            current_orbit: None,
            redraw_requested: true,
            ui: Tree::new(),
            last_hover_ui: None,
            last_redraw: Nanotime::zero(),
            notifications: Vec::new(),
            is_exit_prompt: false,
        };

        let orbit = SparseOrbit::new(
            17000.0,
            12000.0,
            -0.3,
            Body::with_mu(EARTH_RADIUS, EARTH_MU, EARTH_SOI),
            Nanotime::zero(),
            false,
        )
        .unwrap();
        let go = GlobalOrbit(PlanetId(0), orbit);

        for _ in 0..60 {
            g.spawn_with_random_perturbance(&go);
        }

        g.set_piloting(OrbiterId(3));
        g.set_targeting(OrbiterId(25));

        g
    }
}

impl Render for GameState {
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        match state.current_scene().kind() {
            SceneType::Orbital => OrbitalContext::text_labels(state),
            SceneType::TelescopeView => TelescopeContext::text_labels(state),
            SceneType::Editor => EditorContext::text_labels(state),
            SceneType::DockingView => RPOContext::text_labels(state),
            SceneType::CommsPanel => CommsContext::text_labels(state),
            SceneType::MainMenu => None,
        }
    }

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        match state.current_scene().kind() {
            SceneType::Editor => EditorContext::sprites(state),
            SceneType::DockingView => RPOContext::sprites(state),
            SceneType::MainMenu | SceneType::Orbital => OrbitalContext::sprites(state),
            SceneType::CommsPanel => CommsContext::sprites(state),
            SceneType::TelescopeView => None,
        }
    }

    fn background_color(state: &GameState) -> Srgba {
        match state.current_scene().kind() {
            SceneType::Orbital => OrbitalContext::background_color(state),
            SceneType::Editor => EditorContext::background_color(state),
            SceneType::TelescopeView => TelescopeContext::background_color(state),
            SceneType::DockingView => RPOContext::background_color(state),
            SceneType::MainMenu => GRAY.with_luminance(0.09),
            SceneType::CommsPanel => CommsContext::background_color(state),
        }
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        match state.current_scene().kind() {
            SceneType::Orbital => OrbitalContext::draw_gizmos(gizmos, state),
            SceneType::Editor => EditorContext::draw_gizmos(gizmos, state),
            SceneType::TelescopeView => TelescopeContext::draw_gizmos(gizmos, state),
            SceneType::DockingView => RPOContext::draw_gizmos(gizmos, state),
            SceneType::MainMenu => None,
            SceneType::CommsPanel => CommsContext::draw_gizmos(gizmos, state),
        }
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        match state.current_scene().kind() {
            SceneType::CommsPanel => CommsContext::ui(state),
            _ => None,
        }
    }
}

impl GameState {
    pub fn redraw(&mut self) {
        self.redraw_requested = true;
        self.last_redraw = Nanotime::zero()
    }

    pub fn set_piloting(&mut self, id: OrbiterId) {
        self.orbital_context.piloting = Some(id);
    }

    pub fn set_targeting(&mut self, id: OrbiterId) {
        self.orbital_context.targeting = Some(id);
    }

    pub fn current_scene(&self) -> &Scene {
        &self.scenes[self.current_scene_idx]
    }

    pub fn is_tracked(&self, id: OrbiterId) -> bool {
        self.orbital_context.selected.contains(&id)
    }

    pub fn get_group_members(&mut self, gid: &GroupId) -> Vec<OrbiterId> {
        self.constellations
            .iter()
            .filter_map(|(id, g)| (g == gid).then(|| *id))
            .collect()
    }

    pub fn group_membership(&self, id: &OrbiterId) -> Option<&GroupId> {
        self.constellations.get(id)
    }

    pub fn unique_groups(&self) -> Vec<&GroupId> {
        let mut s: Vec<&GroupId> = self
            .constellations
            .iter()
            .map(|(_, gid)| gid)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        s.sort();
        s
    }

    pub fn toggle_group(&mut self, gid: &GroupId) {
        // - if any of the orbiters in the group are not selected,
        //   select all of them
        // - if all of them are already selected, deselect all of them

        let members = self.get_group_members(gid);

        let all_selected = members.iter().all(|id| self.is_tracked(*id));

        for id in members {
            if all_selected {
                self.orbital_context.selected.remove(&id);
            } else {
                self.orbital_context.selected.insert(id);
            }
        }
    }

    pub fn disband_group(&mut self, gid: &GroupId) {
        self.constellations.retain(|_, g| g != gid);
    }

    pub fn create_group(&mut self, gid: GroupId) {
        for id in &self.orbital_context.selected {
            self.constellations.insert(*id, gid.clone());
        }
    }

    pub fn planned_maneuvers(&self, after: Nanotime) -> Vec<(OrbiterId, Nanotime, Vec2)> {
        let mut dvs = vec![];
        for ctrl in &self.controllers {
            if let Some(plan) = ctrl.plan() {
                for (stamp, impulse) in plan.future_dvs(after) {
                    dvs.push((ctrl.target(), stamp, impulse));
                }
            }
        }
        dvs.sort_by_key(|(_, t, _)| t.inner());
        dvs
    }

    pub fn selection_region(&self) -> Option<Region> {
        OrbitalContext::selection_region(self)
    }

    pub fn measuring_tape(&self) -> Option<(Vec2, Vec2, Vec2)> {
        if self.orbital_context.cursor_mode != CursorMode::MeasuringTape {
            return None;
        }

        OrbitalContext::measuring_tape(self)
    }

    pub fn protractor(&self) -> Option<(Vec2, Vec2, Option<Vec2>)> {
        if self.orbital_context.cursor_mode != CursorMode::Protractor {
            return None;
        }

        OrbitalContext::protractor(self)
    }

    pub fn left_cursor_orbit(&self) -> Option<GlobalOrbit> {
        OrbitalContext::left_cursor_orbit(self)
    }

    pub fn cursor_orbit_if_mode(&self) -> Option<GlobalOrbit> {
        if self.orbital_context.cursor_mode == CursorMode::AddOrbit {
            self.left_cursor_orbit()
        } else {
            None
        }
    }

    pub fn piloting(&self) -> Option<OrbiterId> {
        self.orbital_context.piloting
    }

    pub fn targeting(&self) -> Option<OrbiterId> {
        self.orbital_context.targeting
    }

    pub fn get_orbit(&self, id: OrbiterId) -> Option<GlobalOrbit> {
        let lup = self.scenario.lup_orbiter(id, self.sim_time)?;
        let orbiter = lup.orbiter()?;
        let prop = orbiter.propagator_at(self.sim_time)?;
        Some(prop.orbit)
    }

    pub fn spawn_with_random_perturbance(&mut self, global: &GlobalOrbit) -> Option<()> {
        let GlobalOrbit(parent, orbit) = global;
        let pv_local = orbit.pv(self.sim_time).ok()?;
        let perturb = PV::new(
            randvec(pv_local.pos.length() * 0.005, pv_local.pos.length() * 0.02),
            randvec(pv_local.vel.length() * 0.005, pv_local.vel.length() * 0.02),
        );
        let orbit = SparseOrbit::from_pv(pv_local + perturb, orbit.body, self.sim_time)?;
        let id = self.ids.next();
        self.scenario.add_object(id, *parent, orbit, self.sim_time);
        self.notice(format!("Spawned {id} in orbit around {}", global.0));
        Some(())
    }

    pub fn spawn_new(&mut self) -> Option<()> {
        let orbit = self.cursor_orbit_if_mode()?;
        self.spawn_with_random_perturbance(&orbit)
    }

    pub fn delete_orbiter(&mut self, id: OrbiterId) -> Option<()> {
        let lup = self.scenario.lup_orbiter(id, self.sim_time)?;
        let _orbiter = lup.orbiter()?;
        let parent = lup.parent(self.sim_time)?;
        let pv = lup.pv().pos;
        let plup = self.scenario.lup_planet(parent, self.sim_time)?;
        let pvp = plup.pv().pos;
        let pvl = pv - pvp;
        self.scenario.remove_orbiter(id)?;
        self.notify(
            ObjectId::Planet(parent),
            NotificationType::OrbiterDeleted(id),
            pvl,
        );
        Some(())
    }

    pub fn delete_objects(&mut self) {
        self.orbital_context
            .selected
            .clone()
            .into_iter()
            .for_each(|id| {
                self.delete_orbiter(id);
            });
    }

    pub fn current_orbit(&self) -> Option<&GlobalOrbit> {
        self.orbital_context.queued_orbits.get(self.current_orbit?)
    }

    pub fn commit_mission(&mut self) -> Option<()> {
        let orbit = self.current_orbit()?.clone();
        self.command_selected(&orbit);
        Some(())
    }

    pub fn turn(&mut self, dir: i8) -> Option<()> {
        let id = self.piloting()?;
        let vehicle = self.orbital_vehicles.get_mut(&id)?;
        vehicle.turn(dir as f32 * 0.03);
        Some(())
    }

    pub fn thrust_prograde(&mut self) -> Option<()> {
        let id = self.piloting()?;

        let vehicle = match self.orbital_vehicles.get(&id) {
            Some(v) => {
                if v.is_controllable() {
                    v
                } else {
                    let notif = NotificationType::NotControllable(id);
                    self.notify(ObjectId::Orbiter(id), notif, None);
                    return None;
                }
            }
            None => {
                let notif = NotificationType::NotControllable(id);
                self.notify(ObjectId::Orbiter(id), notif, None);
                return None;
            }
        };

        let throttle = self.orbital_context.throttle.to_ratio();

        let dv = vehicle.pointing() * 0.005 * throttle;

        let notif = if self
            .scenario
            .impulsive_burn(id, self.sim_time, dv)
            .is_none()
        {
            NotificationType::ManeuverFailed(id)
        } else {
            NotificationType::OrbitChanged(id)
        };

        self.notify(ObjectId::Orbiter(id), notif, None);

        self.scenario.simulate(self.sim_time, self.physics_duration);
        Some(())
    }

    pub fn command_selected(&mut self, next: &GlobalOrbit) {
        if self.orbital_context.selected.is_empty() {
            return;
        }
        self.notice(format!(
            "Commanding {} orbiters to {}",
            self.orbital_context.selected.len(),
            next,
        ));
        for id in self.orbital_context.selected.clone() {
            self.command(id, next);
        }
    }

    pub fn release_selected(&mut self) {
        let tracks = self.orbital_context.selected.clone();
        self.controllers.retain(|c| !tracks.contains(&c.target()));
    }

    pub fn command(&mut self, id: OrbiterId, next: &GlobalOrbit) -> Option<()> {
        let tracks = self.orbital_context.selected.clone();
        let vehicle = self.orbital_vehicles.get(&id)?;
        if !vehicle.is_controllable() {
            self.notify(
                ObjectId::Orbiter(id),
                NotificationType::NotControllable(id),
                None,
            );
            return None;
        }

        if self.controllers.iter().find(|c| c.target() == id).is_none() {
            self.controllers.push(Controller::idle(id));
        }

        self.controllers.iter_mut().for_each(|c| {
            if tracks.contains(&c.target()) {
                let ret = c.set_destination(*next, self.sim_time);
                if let Err(_e) = ret {
                    // dbg!(e);
                }
            }
        });

        Some(())
    }

    pub fn notice(&mut self, s: String) {
        info!("Notice: {s}");
        self.notify(None, NotificationType::Notice(s), None)
    }

    pub fn notify(
        &mut self,
        parent: impl Into<Option<ObjectId>>,
        kind: NotificationType,
        offset: impl Into<Option<Vec2>>,
    ) {
        self.redraw();

        let notif = Notification {
            parent: parent.into(),
            offset: offset.into().unwrap_or(Vec2::ZERO),
            jitter: Vec2::ZERO,
            sim_time: self.sim_time,
            wall_time: self.wall_time,
            extra_time: Nanotime::secs_f32(rand(0.0, 1.0)),
            kind,
        };

        if self.notifications.iter().any(|e| notif.is_duplicate(e)) {
            return;
        }

        self.notifications.push(notif);
    }

    pub fn light_source(&self) -> Vec2 {
        let angle = 2.0 * PI * self.sim_time.to_secs() / Nanotime::days(365).to_secs();
        rotate(Vec2::X, angle + PI) * 1000000.0
    }

    pub fn save(&mut self) -> Option<()> {
        match self.current_scene().kind() {
            SceneType::Editor => EditorContext::save_to_file(self),
            SceneType::Orbital => OrbitalContext::save_to_file(self),
            _ => None,
        }
    }

    pub fn load(&mut self) -> Option<()> {
        match self.current_scene().kind() {
            SceneType::Editor => EditorContext::load_from_file(self),
            _ => None,
        }
    }

    pub fn on_button_event(&mut self, id: OnClick) -> Option<()> {
        match id {
            OnClick::CurrentBody(id) => self.orbital_context.following = Some(ObjectId::Planet(id)),
            OnClick::Orbiter(id) => self.orbital_context.following = Some(ObjectId::Orbiter(id)),
            OnClick::ToggleDrawMode => {
                self.orbital_context.draw_mode = next_cycle(&self.orbital_context.draw_mode)
            }
            OnClick::ClearTracks => self.orbital_context.selected.clear(),
            OnClick::ClearOrbits => self.orbital_context.queued_orbits.clear(),
            OnClick::Group(gid) => self.toggle_group(&gid),
            OnClick::CreateGroup => self.create_group(GroupId(get_random_name())),
            OnClick::DisbandGroup(gid) => self.disband_group(&gid),
            OnClick::CommitMission => {
                self.commit_mission();
            }
            OnClick::Exit => self.shutdown_with_prompt(),
            OnClick::SimSpeed(s) => {
                self.sim_speed = s;
            }
            OnClick::DeleteOrbit(i) => {
                self.orbital_context.queued_orbits.remove(i);
            }
            OnClick::TogglePause => self.paused = !self.paused,
            OnClick::GlobalOrbit(i) => {
                let orbit = self.orbital_context.queued_orbits.get(i)?;
                self.orbital_context.following = Some(ObjectId::Planet(orbit.0));
                self.current_orbit = Some(i);
            }
            OnClick::Nullopt => (),
            OnClick::Save => {
                self.save();
            }
            OnClick::Load => {
                self.load();
            }
            OnClick::CursorMode(c) => self.orbital_context.cursor_mode = c,
            OnClick::AutopilotingCount => {
                self.orbital_context.selected =
                    self.controllers.iter().map(|c| c.target()).collect();
            }
            OnClick::GoToScene(i) => {
                self.set_current_scene(i);
            }
            OnClick::ThrustLevel(throttle) => {
                self.orbital_context.throttle = throttle;
                self.notice(format!("Throttle set to {:?}", throttle));
            }
            OnClick::ClearPilot => self.orbital_context.piloting = None,
            OnClick::ClearTarget => self.orbital_context.targeting = None,
            OnClick::SetPilot(p) => self.orbital_context.piloting = Some(p),
            OnClick::SetTarget(p) => self.orbital_context.targeting = Some(p),
            OnClick::SelectPart(name) => EditorContext::set_current_part(self, &name),
            OnClick::ToggleLayer(layer) => self.editor_context.toggle_layer(layer),
            OnClick::LoadVehicle(path) => _ = EditorContext::load_vehicle(&path, self),
            OnClick::ConfirmExitDialog => self.shutdown(),
            OnClick::DismissExitDialog => self.is_exit_prompt = false,
            _ => info!("Unhandled button event: {id:?}"),
        };

        Some(())
    }

    pub fn shutdown_with_prompt(&mut self) {
        if self.is_exit_prompt {
            self.shutdown()
        } else {
            self.is_exit_prompt = true;
        }
    }

    pub fn shutdown(&self) {
        // for a sensation of weightiness
        std::thread::sleep(core::time::Duration::from_millis(50));
        std::process::exit(0)
    }

    pub fn set_current_scene(&mut self, i: usize) -> Option<()> {
        if i == self.current_scene_idx {
            return Some(());
        }
        self.scenes.get(i)?;
        self.current_scene_idx = i;
        Some(())
    }

    pub fn current_hover_ui(&self) -> Option<&OnClick> {
        let wb = self.input.screen_bounds.span;
        let p = self.input.position(MouseButt::Hover, FrameId::Current)?;
        self.ui.at(p, wb).map(|n| n.on_click()).flatten()
    }

    pub fn is_hovering_over_ui(&self) -> bool {
        let wb = self.input.screen_bounds.span;
        let p = match self.input.position(MouseButt::Hover, FrameId::Current) {
            Some(p) => p,
            None => return false,
        };
        self.ui.at(p, wb).map(|n| n.is_visible()).unwrap_or(false)
    }

    pub fn is_currently_left_clicked_on_ui(&self) -> bool {
        let wb = self.input.screen_bounds.span;
        if self
            .input
            .position(MouseButt::Left, FrameId::Current)
            .is_none()
        {
            return false;
        }
        let p = match self.input.position(MouseButt::Left, FrameId::Down) {
            Some(p) => p,
            None => return false,
        };
        self.ui.at(p, wb).map(|n| n.is_visible()).unwrap_or(false)
    }

    fn handle_click_events(&mut self) {
        use FrameId::*;
        use MouseButt::*;

        let wb = self.input.screen_bounds.span;

        if self.input.on_frame(Left, Down).is_some() {
            self.redraw();
        }

        if self.input.on_frame(Left, Up).is_some() {
            self.redraw();
            let p = self.input.position(Left, Down);
            let q = self.input.position(Left, Up);
            if let Some((p, q)) = p.zip(q) {
                let n = self.ui.at(p, wb).map(|n| n.on_click()).flatten();
                let m = self.ui.at(q, wb).map(|n| n.on_click()).flatten();
                if let Some((n, m)) = n.zip(m) {
                    if n == m {
                        self.on_button_event(n.clone());
                    }
                }
            }
        }

        if self.input.on_frame(Right, Down).is_some() {
            self.redraw();
        }

        if self.input.on_frame(Left, Up).is_some() {
            let h = &self.orbital_context.highlighted;
            self.orbital_context.selected.extend(h.into_iter());
            self.orbital_context.highlighted.clear();
            self.redraw();
        }

        if self.input.on_frame(Right, Up).is_some() {
            self.redraw();
        }
    }

    pub fn step(&mut self, delta_time: Nanotime) {
        let dt = delta_time.to_secs();
        let old_sim_time = self.sim_time;
        self.wall_time += delta_time;
        if !self.paused {
            let sp = 10.0f32.powi(self.sim_speed);
            self.sim_time += delta_time * sp;
        }

        let current_ui = self.current_hover_ui().cloned();
        if current_ui != self.last_hover_ui {
            self.redraw();
            self.last_hover_ui = current_ui;
        }

        || -> Option<()> {
            if let Some(p) = self.input.double_click() {
                if let SceneType::Orbital = self.current_scene().kind() {
                    ()
                } else {
                    return None;
                }
                if self.is_hovering_over_ui() {
                    return None;
                }
                let w = self.orbital_context.c2w(p);
                let id = self.scenario.nearest(w, self.sim_time)?;
                self.orbital_context.following = Some(id);
                self.notice(format!("Now following {id}"));
            }
            Some(())
        }();

        for (_, rpo) in &mut self.rpos {
            rpo.step(self.wall_time);
        }

        // handle discrete physics events
        for (_, vehicle) in self.orbital_vehicles.iter_mut() {
            // controversial
            vehicle.main(false);
            vehicle.step(self.wall_time);
        }

        self.handle_click_events();

        let s = self.sim_time;
        let d = self.physics_duration;

        let mut man = self.planned_maneuvers(old_sim_time);
        while let Some((id, t, dv)) = man.first() {
            if s > *t {
                let perturb = 0.0 * randvec(0.01, 0.05);
                self.scenario.simulate(*t, d);
                self.scenario.impulsive_burn(*id, *t, dv + perturb);
                self.notify(
                    ObjectId::Orbiter(*id),
                    NotificationType::OrbitChanged(*id),
                    None,
                );
            } else {
                break;
            }
            man.remove(0);
        }

        for (id, ri) in self.scenario.simulate(s, d) {
            info!("{} {:?}", id, &ri);
            if let Some(pv) = ri.orbit.pv(ri.stamp).ok() {
                let notif = match ri.reason {
                    EventType::Collide(_) => NotificationType::OrbiterCrashed(id),
                    EventType::Encounter(_) => continue,
                    EventType::Escape(_) => NotificationType::OrbiterEscaped(id),
                    EventType::Impulse(_) => continue,
                    EventType::NumericalError => NotificationType::NumericalError(id),
                };
                self.notify(ObjectId::Planet(ri.parent), notif, pv.pos);
            }
        }

        let mut track_list = self.orbital_context.selected.clone();
        track_list.retain(|o| self.scenario.lup_orbiter(*o, self.sim_time).is_some());
        self.orbital_context.selected = track_list;

        let ids: Vec<_> = self.scenario.orbiter_ids().collect();

        self.constellations.retain(|id, _| ids.contains(id));

        let mut notifs = vec![];

        self.controllers.iter_mut().for_each(|c| {
            if !c.needs_update(s) {
                return;
            }

            let lup = self.scenario.lup_orbiter(c.target(), s);
            let orbiter = lup.map(|lup| lup.orbiter()).flatten();
            let prop = orbiter.map(|orb| orb.propagator_at(s)).flatten();

            if let Some(prop) = prop {
                let res = c.update(s, prop.orbit);
                if let Err(_) = res {
                    notifs.push((c.target(), NotificationType::ManeuverFailed(c.target())));
                }
            }
        });

        for id in ids {
            if !self.orbital_vehicles.contains_key(&id) && !self.rpos.contains_key(&id) {
                if rand(0.0, 1.0) < 0.7 {
                    let vehicle = Vehicle::random(self.sim_time);
                    self.orbital_vehicles.insert(id, vehicle);
                } else {
                    let rpo = RPO::example(self.wall_time);
                    self.rpos.insert(id, rpo);
                }
            }
        }

        notifs
            .into_iter()
            .for_each(|(t, n)| self.notify(ObjectId::Orbiter(t), n, None));

        let mut finished_ids = Vec::<OrbiterId>::new();

        self.controllers.retain(|c| {
            if c.is_idle() {
                finished_ids.push(c.target());
                false
            } else {
                true
            }
        });

        finished_ids.into_iter().for_each(|id| {
            self.notify(
                ObjectId::Orbiter(id),
                NotificationType::ManeuverComplete(id),
                None,
            )
        });

        self.notifications.iter_mut().for_each(|n| n.jitter());

        self.notifications
            .retain(|n| n.wall_time + n.duration() > self.wall_time);

        match self.current_scene().kind() {
            SceneType::Orbital => {
                self.orbital_context.highlighted = OrbitalContext::highlighted(self);
                if let Some(p) = self.orbital_context.follow_position(self) {
                    self.orbital_context.go_to(p);
                }
                self.orbital_context.step(&self.input, dt);
            }
            SceneType::TelescopeView => self.telescope_context.step(&self.input, dt),
            SceneType::DockingView => {
                self.rpo_context.step(&self.input, dt);
                if let Some((_, rpo)) = self.rpos.iter().next() {
                    self.rpo_context.handle_follow(&self.input, rpo);
                }
            }
            SceneType::Editor => {
                EditorContext::step(self, dt);
            }
            _ => (),
        }
    }
}

fn step_system(time: Res<Time>, mut state: ResMut<GameState>) {
    let dt = Nanotime::secs_f32(time.delta_secs());
    state.step(dt);
}

fn process_interaction(
    inter: &InteractionEvent,
    state: &mut GameState,
    window: &mut Window,
) -> Option<()> {
    match inter {
        InteractionEvent::Delete => state.delete_objects(),
        InteractionEvent::CommitMission => {
            state.commit_mission();
        }
        InteractionEvent::ClearMissions => {
            state.release_selected();
        }
        InteractionEvent::ClearSelection => {
            state.orbital_context.selected.clear();
        }
        InteractionEvent::ClearOrbitQueue => {
            state.orbital_context.queued_orbits.clear();
        }
        InteractionEvent::SimSlower => {
            state.sim_speed = i32::clamp(state.sim_speed - 1, -4, 4);
            state.redraw();
        }
        InteractionEvent::SimFaster => {
            state.sim_speed = i32::clamp(state.sim_speed + 1, -4, 4);
            state.redraw();
        }
        InteractionEvent::SimPause => {
            state.paused = !state.paused;
        }
        InteractionEvent::CursorMode => {
            state.orbital_context.cursor_mode = next_cycle(&state.orbital_context.cursor_mode);
        }
        InteractionEvent::DrawMode => {
            state.orbital_context.draw_mode = next_cycle(&state.orbital_context.draw_mode);
        }
        InteractionEvent::Orbits => {
            state.orbital_context.show_orbits = next_cycle(&state.orbital_context.show_orbits);
        }
        InteractionEvent::Spawn => {
            state.spawn_new();
        }
        InteractionEvent::ToggleFullscreen => {
            let fs = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
            window.mode = if window.mode == fs {
                WindowMode::Windowed
            } else {
                fs
            };
        }
        InteractionEvent::Escape => {
            if !state.is_exit_prompt {
                state.is_exit_prompt = true;
            } else {
                state.shutdown()
            }
        }
        InteractionEvent::ContextDependent => {
            if let Some(o) = state.cursor_orbit_if_mode() {
                state.notice(format!("Enqueued orbit {}", &o));
                state.orbital_context.queued_orbits.push(o);
            } else if state.orbital_context.following.is_some() {
                state.orbital_context.following = None;
            } else if !state.orbital_context.selected.is_empty() {
                state.orbital_context.selected.clear();
            }
        }
        InteractionEvent::Load(name) => {
            let (system, ids) = match name.as_str() {
                "grid" => Some(consistency_example()),
                "earth" => Some(earth_moon_example_one()),
                "earth2" => Some(earth_moon_example_two()),
                "moon" => Some(just_the_moon()),
                "jupiter" => Some(sun_jupiter()),
                _ => {
                    error!("No scenario named {}", name);
                    None
                }
            }?;
            load_new_scenario(state, system, ids);
        }
        InteractionEvent::ToggleObject(id) => {
            state.orbital_context.toggle_track(*id);
        }
        InteractionEvent::ToggleGroup(gid) => {
            state.toggle_group(gid);
        }
        InteractionEvent::DisbandGroup(gid) => {
            state.disband_group(gid);
        }
        InteractionEvent::CreateGroup => {
            let gid = GroupId(get_random_name());
            state.create_group(gid.clone());
        }
        InteractionEvent::ThrustForward => {
            state.thrust_prograde();
        }
        InteractionEvent::TurnLeft => {
            state.turn(1);
        }
        InteractionEvent::TurnRight => {
            state.turn(-1);
        }
        InteractionEvent::Reset
        | InteractionEvent::MoveLeft
        | InteractionEvent::MoveRight
        | InteractionEvent::MoveUp
        | InteractionEvent::MoveDown
        | _ => (),
    };
    state.redraw();
    Some(())
}

fn handle_interactions(
    mut events: EventReader<InteractionEvent>,
    mut state: ResMut<GameState>,
    mut window: Single<&mut Window>,
) {
    for e in events.read() {
        debug!("Interaction event: {e:?}");
        process_interaction(e, &mut state, &mut window);
    }
}

fn load_new_scenario(state: &mut GameState, scen: Scenario, ids: ObjectIdTracker) {
    state.scenario = scen;
    state.ids = ids;
    state.sim_time = Nanotime::zero();
    state.orbital_context.selected.clear();
}
