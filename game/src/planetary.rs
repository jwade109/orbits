use crate::mouse::{FrameId, InputState, MouseButt};
use crate::notifications::*;
use crate::scenes::{
    CameraProjection, CursorMode, EnumIter, OrbitalContext, RPOContext, Scene, SceneType,
    TelescopeContext,
};
use crate::ui::{InteractionEvent, OnClick};
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::window::WindowMode;
use rfd::FileDialog;
use starling::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);

        app.add_systems(
            Update,
            (
                // egui
                crate::ui::do_text_labels,
                // physics
                step_system,
                // inputs
                crate::keybindings::keyboard_input,
                track_highlighted_objects,
                handle_interactions,
                crate::mouse::update_input_state,
                // rendering
                crate::sprites::make_new_sprites,
                crate::sprites::update_planet_sprites,
                crate::sprites::update_shadow_sprites,
                crate::sprites::update_background_sprite,
                crate::sprites::update_spacecraft_sprites,
                crate::drawing::draw_game_state,
            )
                .chain(),
        );
    }
}

/// TODO get rid of this thing.
#[derive(Component, Debug)]
pub struct DingusController;

fn init_system(mut commands: Commands) {
    commands.insert_resource(GameState::default());
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(BLACK.with_alpha(0.0).into()),
            ..default()
        },
        Bloom {
            intensity: 0.2,
            ..Bloom::OLD_SCHOOL
        },
        DingusController,
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(BLACK.with_alpha(0.0).into()),
            ..default()
        },
        RenderLayers::layer(1),
    ));
}

#[derive(Resource)]
pub struct GameState {
    pub current_frame_no: u32,

    /// Contains all states related to window size, mouse clicks and positions,
    /// and button presses and holds.
    pub input: InputState,

    /// Stores information and provides an API for interacting with the simulation
    /// from the perspective of a global solar/planetary system view.
    ///
    /// Additional information allows the user to select spacecraft and
    /// direct them to particular orbits, or manually pilot them.
    pub orbital_context: OrbitalContext,

    pub telescope_context: TelescopeContext,

    pub rpo_context: RPOContext,

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

    /// Stupid thing to generate unique increasing IDs for
    /// planets and orbiters
    pub ids: ObjectIdTracker,

    pub controllers: Vec<Controller>,
    pub constellations: HashMap<OrbiterId, GroupId>,
    pub starfield: Vec<(Vec3, Srgba, f32, f32)>,
    pub rpos: Vec<RPO>,

    pub scenes: Vec<Scene>,
    pub current_scene_idx: usize,
    pub current_orbit: Option<usize>,

    pub current_hover: Option<OnClick>,
    pub redraw_requested: bool,
    pub last_redraw: Nanotime,

    pub notifications: Vec<Notification>,
    pub text_labels: Vec<(Vec2, String)>,
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

fn generate_rpos() -> Vec<RPO> {
    vec![RPO::example()]
}

impl Default for GameState {
    fn default() -> Self {
        let (scenario, ids) = default_example();

        GameState {
            current_frame_no: 0,
            input: InputState::default(),
            orbital_context: OrbitalContext::new(PlanetId(0)),
            telescope_context: TelescopeContext::new(),
            rpo_context: RPOContext::new(),
            sim_time: Nanotime::zero(),
            wall_time: Nanotime::zero(),
            physics_duration: Nanotime::days(7),
            sim_speed: 0,
            paused: false,
            scenario: scenario.clone(),
            ids,
            controllers: vec![],
            constellations: HashMap::new(),
            starfield: generate_starfield(),
            rpos: generate_rpos(),
            scenes: vec![
                Scene::orbital("Earth System", PlanetId(0)),
                Scene::orbital("Luna System", PlanetId(1)),
                Scene::docking("Docking", OrbiterId(0)),
                Scene::telescope(),
                Scene::main_menu(),
            ],
            current_scene_idx: 0,
            current_orbit: None,
            current_hover: None,
            redraw_requested: true,
            last_redraw: Nanotime::zero(),
            notifications: Vec::new(),
            text_labels: Vec::new(),
        }
    }
}

impl GameState {
    pub fn redraw(&mut self) {
        self.redraw_requested = true;
        self.last_redraw = Nanotime::zero()
    }

    pub fn current_scene(&self) -> &Scene {
        &self.scenes[self.current_scene_idx]
    }

    pub fn current_scene_mut(&mut self) -> &mut Scene {
        &mut self.scenes[self.current_scene_idx]
    }

    pub fn toggle_track(&mut self, id: OrbiterId) {
        if self.orbital_context.selected.contains(&id) {
            self.orbital_context.selected.retain(|e| *e != id);
        } else {
            self.orbital_context.selected.insert(id);
        }
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
        let ov = self.current_scene().orbital_view(&self.input)?;
        ov.selection_region(self)
    }

    pub fn measuring_tape(&self) -> Option<(Vec2, Vec2, Vec2)> {
        if self.orbital_context.selection_mode != CursorMode::Measure {
            return None;
        }

        let scene = self.current_scene();
        let ov = scene.orbital_view(&self.input)?;
        ov.measuring_tape(self)
    }

    pub fn right_cursor_orbit(&self) -> Option<GlobalOrbit> {
        let scene = self.current_scene();
        let ov = scene.orbital_view(&self.input)?;
        ov.right_cursor_orbit(self)
    }

    pub fn piloting(&self) -> Option<OrbiterId> {
        self.orbital_context.follow?.orbiter()
    }

    pub fn spawn_at(&mut self, global: &GlobalOrbit) -> Option<()> {
        let GlobalOrbit(parent, orbit) = global;
        let pv_local = orbit.pv(self.sim_time).ok()?;
        let perturb = PV::new(
            randvec(pv_local.pos.length() * 0.005, pv_local.pos.length() * 0.02),
            randvec(pv_local.vel.length() * 0.005, pv_local.vel.length() * 0.02),
        );
        let orbit = SparseOrbit::from_pv(pv_local + perturb, orbit.body, self.sim_time)?;
        let id = self.ids.next();
        self.scenario.add_object(id, *parent, orbit, self.sim_time);
        Some(())
    }

    pub fn spawn_new(&mut self) -> Option<()> {
        let orbit = self.right_cursor_orbit()?;
        self.spawn_at(&orbit)
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

    pub fn highlighted(&self) -> HashSet<OrbiterId> {
        if let Some(a) = self.selection_region() {
            self.scenario
                .orbiter_ids()
                .into_iter()
                .filter_map(|id| {
                    let pv = self.scenario.lup_orbiter(id, self.sim_time)?.pv();
                    a.contains(pv.pos).then(|| id)
                })
                .collect()
        } else {
            HashSet::new()
        }
    }

    pub fn turn(&mut self, dir: i8) -> Option<()> {
        let id = self.piloting()?;
        let orbiter = self.scenario.orbiter_mut(id)?;
        orbiter.vehicle.turn(dir as f32 * 0.03);
        Some(())
    }

    pub fn thrust_prograde(&mut self) -> Option<()> {
        let id = self.piloting()?;

        let orbiter = self.scenario.lup_orbiter(id, self.sim_time)?.orbiter()?;
        let dv = orbiter.vehicle.pointing() * 0.005;

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
        info!(
            "Commanding {} orbiters to {}",
            self.orbital_context.selected.len(),
            next,
        );
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
        let orbiter = self.scenario.lup_orbiter(id, self.sim_time)?.orbiter()?;
        if !orbiter.vehicle.is_controllable() {
            self.notify(id, NotificationType::NotControllable, None);
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

    pub fn notify(
        &mut self,
        parent: impl Into<ObjectId>,
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
        let angle = self.sim_time.to_secs() / 1000.0;
        rotate(Vec2::X, angle + PI) * 1000000.0
    }

    pub fn save(&self) -> Option<()> {
        let orbiters: Vec<_> = self
            .orbital_context
            .selected
            .iter()
            .filter_map(|id| {
                self.scenario
                    .lup_orbiter(*id, self.sim_time)
                    .map(|lup| lup.orbiter())
                    .flatten()
            })
            .collect();

        let dir = FileDialog::new().set_directory("/").pick_folder()?;

        for orbiter in orbiters {
            let mut file = dir.clone();
            file.push(format!("{}.strl", orbiter.id()));
            info!("Saving {}", file.display());
            starling::file_export::to_strl_file(orbiter, &file).ok()?;
        }

        Some(())
    }

    pub fn on_button_event(&mut self, id: crate::ui::OnClick) -> Option<()> {
        use crate::ui::OnClick;

        match id {
            OnClick::CurrentBody(id) => self.orbital_context.follow = Some(ObjectId::Planet(id)),
            OnClick::Orbiter(id) => self.orbital_context.follow = Some(ObjectId::Orbiter(id)),
            OnClick::ToggleDrawMode => self.orbital_context.draw_mode.to_next(),
            OnClick::ClearTracks => self.orbital_context.selected.clear(),
            OnClick::ClearOrbits => self.orbital_context.queued_orbits.clear(),
            OnClick::Group(gid) => self.toggle_group(&gid),
            OnClick::CreateGroup => self.create_group(GroupId(get_random_name())),
            OnClick::DisbandGroup(gid) => self.disband_group(&gid),
            OnClick::CommitMission => {
                self.commit_mission();
            }
            OnClick::Exit => std::process::exit(0),
            OnClick::SimSpeed(s) => {
                self.sim_speed = s;
            }
            OnClick::DeleteOrbit(i) => {
                self.orbital_context.queued_orbits.remove(i);
            }
            OnClick::TogglePause => self.paused = !self.paused,
            OnClick::GlobalOrbit(i) => {
                let orbit = self.orbital_context.queued_orbits.get(i)?;
                self.orbital_context.follow = Some(ObjectId::Planet(orbit.0));
                self.current_orbit = Some(i);
            }
            OnClick::World => (),
            OnClick::Nullopt => (),
            OnClick::Save => {
                self.save();
            }
            OnClick::Load => {
                let file = FileDialog::new()
                    .add_filter("text", &["starling", "strl"])
                    .set_directory("/")
                    .pick_file();
                dbg!(&file);
                if let Some(file) = file {
                    let obj = starling::file_export::load_strl_file(&file);
                    let _ = dbg!(obj);
                }
            }
            OnClick::CursorMode => self.orbital_context.selection_mode.to_next(),
            OnClick::AutopilotingCount => {
                self.orbital_context.selected =
                    self.controllers.iter().map(|c| c.target()).collect();
            }
            OnClick::GoToScene(i) => {
                self.set_current_scene(i);
            }
            _ => info!("Unhandled button event: {id:?}"),
        };

        Some(())
    }

    pub fn set_current_scene(&mut self, i: usize) -> Option<()> {
        if i == self.current_scene_idx {
            return Some(());
        }
        self.scenes.get(i)?;
        self.current_scene_idx = i;
        Some(())
    }

    pub fn current_hover_ui(&self) -> Option<&crate::ui::OnClick> {
        let wb = self.input.screen_bounds.span;
        let scene = self.current_scene();
        let p = self.input.position(MouseButt::Hover, FrameId::Current)?;
        scene.ui().at(p, wb).map(|n| n.id()).flatten()
    }

    fn handle_click_events(&mut self) {
        use FrameId::*;
        use MouseButt::*;

        let wb = self.input.screen_bounds.span;

        if self.input.on_frame(Left, Down, self.current_frame_no) {
            let scene = self.current_scene();
            let p = self.input.position(Left, Down);
            if let Some(p) = p {
                if let Some(n) = scene.ui().at(p, wb).map(|n| n.id()).flatten() {
                    self.on_button_event(n.clone());
                }
                self.redraw();
            }
        }

        if self.input.on_frame(Right, Down, self.current_frame_no) {
            self.redraw();
        }

        if self.input.on_frame(Left, Up, self.current_frame_no) {
            self.redraw();
        }

        if self.input.on_frame(Right, Up, self.current_frame_no) {
            self.redraw();
        }
    }

    pub fn step(&mut self, time: &Time) {
        let old_sim_time = self.sim_time;
        self.wall_time += Nanotime::nanos(time.delta().as_nanos() as i64);
        if !self.paused {
            let sp = 10.0f32.powi(self.sim_speed);
            self.sim_time += Nanotime::nanos((time.delta().as_nanos() as f32 * sp) as i64);
        }

        let c = self.current_hover_ui().cloned();
        if c != self.current_hover {
            self.redraw();
        }

        match self.current_scene().kind() {
            SceneType::OrbitalView(_) => self.orbital_context.step(&self.input),
            SceneType::TelescopeView(_) => self.telescope_context.step(&self.input),
            SceneType::DockingView(_) => self.rpo_context.step(&self.input),
            _ => (),
        }

        for rpo in &mut self.rpos {
            rpo.step(self.wall_time);
        }

        // handle discrete physics events
        for orbiter in self.scenario.orbiters_mut() {
            // controversial
            orbiter.vehicle.main(false);
            orbiter.step(self.wall_time);
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

        self.text_labels.clear();

        if let Some((m1, m2, corner)) = self.measuring_tape() {
            for (a, b) in [(m1, m2), (m1, corner), (m2, corner)] {
                let middle = (a + b) / 2.0;
                let middle = self.orbital_context.w2c(middle);
                let d = format!("{:0.2}", a.distance(b));
                self.text_labels.push((middle, d));
            }
        }

        self.current_frame_no += 1;
    }
}

fn step_system(time: Res<Time>, mut state: ResMut<GameState>) {
    state.step(&time);
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
            state.orbital_context.selection_mode.to_next();
        }
        InteractionEvent::DrawMode => {
            state.orbital_context.draw_mode.to_next();
        }
        InteractionEvent::RedrawGui => {
            state.redraw();
        }
        InteractionEvent::Orbits => {
            state.orbital_context.show_orbits.to_next();
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
        InteractionEvent::DoubleClick => {
            let p = state.input.position(MouseButt::Left, FrameId::Down)?;
            let w = state.orbital_context.c2w(p);
            let id = state.scenario.nearest(w, state.sim_time)?;
            state.orbital_context.follow = Some(id);
            state.notify(id, NotificationType::Following(id), None);
        }
        InteractionEvent::ExitApp => {
            std::process::exit(0);
        }
        InteractionEvent::ContextDependent => {
            if let Some(o) = state.right_cursor_orbit() {
                info!("Enqueued orbit {}", &o);
                state.orbital_context.queued_orbits.push(o);
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
            state.toggle_track(*id);
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

// TODO get rid of this
fn track_highlighted_objects(buttons: Res<ButtonInput<MouseButton>>, mut state: ResMut<GameState>) {
    if buttons.just_released(MouseButton::Left) || buttons.just_released(MouseButton::Middle) {
        let h = state.highlighted();
        state.orbital_context.selected.extend(h.into_iter());
    }
}

fn load_new_scenario(state: &mut GameState, scen: Scenario, ids: ObjectIdTracker) {
    state.scenario = scen;
    state.ids = ids;
    state.sim_time = Nanotime::zero();
    state.orbital_context.selected.clear();
}
