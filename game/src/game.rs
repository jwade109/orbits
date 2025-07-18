use crate::args::ProgramContext;
use crate::canvas::Canvas;
use crate::debug_console::DebugConsole;
use crate::generate_ship_sprites::*;
use crate::input::{FrameId, InputState, MouseButt};
use crate::notifications::*;
use crate::onclick::OnClick;
use crate::scenes::{
    CameraProjection, CursorMode, EditorContext, MainMenuContext, OrbitalContext, RPOContext,
    Render, Scene, SceneType, StaticSpriteDescriptor, SurfaceContext, TelescopeContext, TextLabel,
};
use crate::ui::InteractionEvent;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::smaa::Smaa;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::RenderLayers;
use bevy::window::WindowMode;
use clap::Parser;
use enum_iterator::next_cycle;
use image::DynamicImage;
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);

        app.insert_resource(Time::<Fixed>::from_duration(
            PHYSICS_CONSTANT_DELTA_TIME.to_duration(),
        ));

        app.add_systems(
            Update,
            (
                crate::keybindings::keyboard_input,
                crate::input::update_input_state,
                on_render_tick,
                crate::drawing::draw_game_state,
                crate::sprites::update_static_sprites,
                crate::sprites::update_background_color,
            )
                .chain(),
        );

        app.add_systems(
            FixedUpdate,
            (
                handle_interactions,
                // physics
                on_game_tick,
                // rendering
                crate::ui::do_text_labels,
                crate::sounds::sound_system,
            )
                .chain(),
        );
    }
}

#[derive(Component, Debug)]
pub struct BackgroundCamera;

fn init_system(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let args = match ProgramContext::try_parse() {
        Ok(args) => args,
        Err(e) => {
            _ = e.print();
            ProgramContext::default()
        }
    };

    let mut g = GameState::new(args);

    g.load_sprites(&mut images);

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
        Smaa::default(),
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

#[derive(Resource)]
pub struct GameState {
    pub game_ticks: u64,
    pub render_ticks: u64,

    /// Contains all states related to window size, mouse clicks and positions,
    /// and button presses and holds.
    pub input: InputState,

    pub console: DebugConsole,

    /// Contains CLI arguments
    pub args: ProgramContext,

    /// All the game entities and logic therein. This should be able to run
    /// autonomously without any user input with on_sim_tick.
    pub universe: Universe,

    /// Stores information and provides an API for interacting with the simulation
    /// from the perspective of a global solar/planetary system view.
    ///
    /// Additional information allows the user to select spacecraft and
    /// direct them to particular orbits, or manually pilot them.
    pub orbital_context: OrbitalContext,

    pub telescope_context: TelescopeContext,

    pub rpo_context: RPOContext,

    pub editor_context: EditorContext,

    pub surface_context: SurfaceContext,

    /// Wall clock, i.e. time since program began.
    pub wall_time: Nanotime,

    pub physics_duration: Nanotime,
    pub universe_ticks_per_game_tick: u32,
    pub paused: bool,
    pub exec_time: std::time::Duration,

    /// Map of names to parts to their definitions. Loaded from
    /// the assets/parts directory
    pub part_database: HashMap<String, PartPrototype>,

    /// Stupid thing to generate unique increasing IDs for
    /// planets and orbiters
    pub ids: ObjectIdTracker,

    pub controllers: Vec<Controller>,
    pub starfield: Vec<(Vec3, Srgba, f32, f32)>,
    pub favorites: HashSet<EntityId>,

    pub scenes: Vec<Scene>,
    pub current_scene_idx: usize,
    pub current_orbit: Option<usize>,

    pub ui: Tree<OnClick>,

    pub notifications: Vec<Notification>,

    pub is_exit_prompt: bool,
    pub button_was_pressed: bool,

    pub text_labels: Vec<TextLabel>,
    pub sprites: Vec<StaticSpriteDescriptor>,
    pub image_handles: HashMap<String, (Handle<Image>, UVec2)>,
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

impl GameState {
    pub fn new(args: ProgramContext) -> Self {
        let (planets, ids) = default_example();

        let mut g = GameState {
            render_ticks: 0,
            game_ticks: 0,
            input: InputState::default(),
            args: args.clone(),
            universe: Universe::new(planets.clone()),
            console: DebugConsole::new(),
            orbital_context: OrbitalContext::new(EntityId(0)),
            telescope_context: TelescopeContext::new(),
            rpo_context: RPOContext::new(),
            editor_context: EditorContext::new(),
            surface_context: SurfaceContext::default(),
            wall_time: Nanotime::zero(),
            physics_duration: Nanotime::days(7),
            universe_ticks_per_game_tick: 1,
            paused: false,
            exec_time: std::time::Duration::new(0, 0),
            part_database: load_parts_from_dir(&args.parts_dir()),
            ids,
            controllers: vec![],
            starfield: generate_starfield(),
            favorites: HashSet::new(),
            scenes: vec![
                Scene::main_menu(),
                Scene::orbital(),
                Scene::telescope(),
                Scene::editor(),
                Scene::surface(),
            ],
            current_scene_idx: 3,
            current_orbit: None,
            ui: Tree::new(),
            notifications: Vec::new(),
            is_exit_prompt: false,
            button_was_pressed: true,
            text_labels: Vec::new(),
            sprites: Vec::new(),
            image_handles: HashMap::new(),
        };

        for model in [
            // "icecream",
            "jubilee", "lander", // "mule",
            "pollux", "remora", "remora", "remora",
            "remora",
            // "glutton",
            // "Lord of Democracy",
        ] {
            if let Some(v) = g.get_vehicle_by_model(model) {
                g.universe.add_surface_vehicle(v);
            }
        }

        let t = g.universe.stamp();

        let get_random_orbit = |pid: EntityId| {
            let r1 = rand(11000.0, 40000.0) as f64;
            let r2 = rand(11000.0, 40000.0) as f64;
            let argp = rand(0.0, 2.0 * PI) as f64;
            let body = planets.lookup(pid, t)?.0;
            let orbit = SparseOrbit::new(r1.max(r2), r1.min(r2), argp, body, t, false)?;
            Some(GlobalOrbit(pid, orbit))
        };

        for _ in 0..30 {
            let vehicle = g.get_random_vehicle();
            let orbit = get_random_orbit(EntityId(0));
            if let (Some(orbit), Some(vehicle)) = (orbit, vehicle) {
                g.spawn_with_random_perturbance(orbit, vehicle);
            }
        }

        // for _ in 0..20 {
        //     let n = randint(3, 7);
        //     let vehicles = (0..n).filter_map(|_| g.get_random_vehicle()).collect();
        //     let rpo = RPO::example(g.sim_time, vehicles);
        //     let orbit = get_random_orbit(EntityId(0));
        //     if let Some(orbit) = orbit {
        //         g.spawn_new_rpo(orbit, rpo);
        //     }
        // }

        for _ in 0..40 {
            let vehicle = g.get_random_vehicle();
            let orbit = get_random_orbit(EntityId(1));
            if let (Some(orbit), Some(vehicle)) = (orbit, vehicle) {
                g.spawn_with_random_perturbance(orbit, vehicle);
            }
        }

        for (id, _) in &g.universe.orbiters {
            if g.favorites.len() < 5 && rand(0.0, 1.0) < 0.05 {
                g.favorites.insert(*id);
            }
        }

        g
    }

    pub fn load_sprites(&mut self, images: &mut Assets<Image>) {
        let mut handles = HashMap::new();
        if let Some(v) = crate::scenes::get_list_of_vehicles(self) {
            for (model, _) in v {
                if let Some(v) = self.get_vehicle_by_model(&model) {
                    let parts_dir = self.args.parts_dir();
                    let img = generate_ship_sprite(&v, &parts_dir, false);
                    if let Some(img) = img {
                        let dims = img.size();
                        let handle = images.add(img);
                        handles.insert(model, (handle, dims));
                    }
                }
            }
        }

        for (name, _) in &self.part_database {
            let path = self.args.part_sprite_path(name);
            if let Some(img) = crate::generate_ship_sprites::read_image(Path::new(&path)) {
                let mut img = Image::from_dynamic(
                    DynamicImage::ImageRgba8(img),
                    true,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                img.sampler = bevy::image::ImageSampler::nearest();
                let dims = img.size();
                let handle = images.add(img.clone());
                handles.insert(name.to_string(), (handle.clone(), dims));

                for pct in (0..=9).rev() {
                    for w in 0..img.width() {
                        for h in 0..img.height() {
                            if rand(0.0, 1.0) < 0.5 {
                                if let Some(pixel) = img.pixel_bytes_mut(UVec3::new(w, h, 0)) {
                                    pixel[3] = pixel[3].min(10);
                                    pixel[2] = 255;
                                }
                            }
                        }
                    }
                    let handle = images.add(img.clone());
                    handles.insert(format!("{}-building-{}", name, pct), (handle, dims));
                }
            } else {
                error!("Failed to load sprite for part {}", name);
            }
        }

        for name in [
            "cloud",
            "diamond",
            // items
            "item-bread",
            "item-corn",
            "item-h2",
            "item-ice",
            "item-methane",
            "item-o2",
            "item-potato",
            "item-wheat",
            "Earth",
            "Luna",
            "Asteroid",
            "conbot",
        ] {
            let path = self.args.install_dir.join(format!("{}.png", name));
            if let Some(img) = crate::generate_ship_sprites::read_image(&path) {
                let mut img = Image::from_dynamic(
                    DynamicImage::ImageRgba8(img),
                    true,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                img.sampler = bevy::image::ImageSampler::nearest();
                let dims = img.size();
                let handle = images.add(img);
                handles.insert(name.into(), (handle, dims));
            } else {
                error!("Failed to load sprite: {}", path.display());
            }
        }

        let image = generate_error_sprite();
        let dims = image.size();
        let handle = images.add(image);
        handles.insert("error".to_string(), (handle, dims));

        self.image_handles = handles;
    }
}

impl Render for GameState {
    fn background_color(state: &GameState) -> Srgba {
        match state.current_scene().kind() {
            SceneType::Orbital => OrbitalContext::background_color(state),
            SceneType::Editor => EditorContext::background_color(state),
            SceneType::Telescope => TelescopeContext::background_color(state),
            SceneType::DockingView => RPOContext::background_color(state),
            SceneType::MainMenu => BLACK,
            SceneType::Surface => SurfaceContext::background_color(state),
        }
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        match state.current_scene().kind() {
            SceneType::Surface => SurfaceContext::ui(state),
            _ => None,
        }
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        // canvas.text(
        //     format!(
        //         "Wall time: {}\nUniverse time: {}\nUniverse ticks per game tick: {}\nRender ticks: {}\nGame ticks: {}\nUniverse ticks: {}\nExec time: {} ns",
        //         state.wall_time,
        //         state.universe.stamp(),
        //         state.universe_ticks_per_game_tick,
        //         state.render_ticks,
        //         state.game_ticks,
        //         state.universe.ticks(),
        //         state.exec_time.as_micros(),
        //     ),
        //     Vec2::ZERO,
        //     0.8,
        // );

        match state.current_scene().kind() {
            SceneType::Orbital => OrbitalContext::draw(canvas, state),
            SceneType::Editor => EditorContext::draw(canvas, state),
            SceneType::Telescope => TelescopeContext::draw(canvas, state),
            SceneType::DockingView => RPOContext::draw(canvas, state),
            SceneType::MainMenu => MainMenuContext::draw(canvas, state),
            SceneType::Surface => SurfaceContext::draw(canvas, state),
        }
    }
}

fn keyboard_control_law(input: &InputState) -> Option<VehicleControl> {
    let mut ctrl = VehicleControl::NULLOPT;

    let docking_mode = input.is_pressed(KeyCode::ControlLeft);

    if docking_mode {
        ctrl.plus_x.throttle = input.is_pressed(KeyCode::ArrowUp) as u8 as f32;
        ctrl.plus_y.throttle = input.is_pressed(KeyCode::ArrowLeft) as u8 as f32;
        ctrl.neg_x.throttle = input.is_pressed(KeyCode::ArrowDown) as u8 as f32;
        ctrl.neg_y.throttle = input.is_pressed(KeyCode::ArrowRight) as u8 as f32;
    } else {
        ctrl.plus_x.throttle = input.is_pressed(KeyCode::ArrowUp) as u8 as f32;
        ctrl.neg_x.throttle = input.is_pressed(KeyCode::ArrowDown) as u8 as f32;

        ctrl.attitude = if input.is_pressed(KeyCode::ArrowLeft) {
            10.0
        } else if input.is_pressed(KeyCode::ArrowRight) {
            -10.0
        } else {
            0.0
        };
    }

    ctrl.plus_x.use_rcs = docking_mode;
    ctrl.plus_y.use_rcs = docking_mode;
    ctrl.neg_x.use_rcs = docking_mode;
    ctrl.neg_y.use_rcs = docking_mode;

    Some(ctrl)
}

impl GameState {
    pub fn reload(&mut self) {
        *self = GameState::new(self.args.clone());
    }

    pub fn set_piloting(&mut self, id: EntityId) {
        self.orbital_context.piloting = Some(id);
    }

    pub fn set_targeting(&mut self, id: EntityId) {
        self.orbital_context.targeting = Some(id);
    }

    pub fn current_scene(&self) -> &Scene {
        &self.scenes[self.current_scene_idx]
    }

    pub fn is_tracked(&self, id: EntityId) -> bool {
        self.orbital_context.selected.contains(&id)
    }

    pub fn toggle_group(&mut self, gid: EntityId) {
        // - if any of the orbiters in the group are not selected,
        //   select all of them
        // - if all of them are already selected, deselect all of them

        let members = self.universe.get_group_members(gid);

        let all_selected = members.iter().all(|id| self.is_tracked(*id));

        for id in members {
            if all_selected {
                self.orbital_context.selected.remove(&id);
            } else {
                self.orbital_context.selected.insert(id);
            }
        }
    }

    pub fn disband_group(&mut self, gid: EntityId) {
        self.universe.constellations.retain(|_, g| *g != gid);
    }

    pub fn create_group(&mut self, gid: EntityId) {
        for id in &self.orbital_context.selected {
            self.universe.constellations.insert(*id, gid.clone());
        }
    }

    pub fn get_vehicle_by_model(&self, name: &str) -> Option<Vehicle> {
        let vehicles = crate::scenes::get_list_of_vehicles(self)?;

        if vehicles.is_empty() {
            return None;
        }

        let (_, path) = vehicles.iter().find(|(model, _)| model == name)?;

        let sat = EditorContext::load_from_vehicle_file(&path)?;

        let mut parts = vec![];
        for part in sat.parts {
            let proto = self.part_database.get(&part.partname)?;
            parts.push((part.pos, part.rot, proto.clone()));
        }

        let mut vehicle = Vehicle::from_parts(name.to_string(), parts);

        vehicle.build_all();

        Some(vehicle)
    }

    pub fn planned_maneuvers(&self, after: Nanotime) -> Vec<(EntityId, Nanotime, Vec2)> {
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

    pub fn piloting(&self) -> Option<EntityId> {
        self.orbital_context.piloting
    }

    pub fn targeting(&self) -> Option<EntityId> {
        self.orbital_context.targeting
    }

    pub fn get_orbit(&self, id: EntityId) -> Option<GlobalOrbit> {
        let lup = self.universe.lup_orbiter(id, self.universe.stamp())?;
        let orbiter = lup.orbiter()?;
        let prop = orbiter.propagator_at(self.universe.stamp())?;
        Some(prop.orbit)
    }

    pub fn spawn_with_random_perturbance(
        &mut self,
        global: GlobalOrbit,
        vehicle: Vehicle,
    ) -> Option<()> {
        let GlobalOrbit(parent, orbit) = global;
        let pv_local = orbit.pv(self.universe.stamp()).ok()?;
        let perturb = PV::from_f64(
            randvec(
                pv_local.pos_f32().length() * 0.005,
                pv_local.pos_f32().length() * 0.02,
            ),
            randvec(
                pv_local.vel_f32().length() * 0.005,
                pv_local.vel_f32().length() * 0.02,
            ),
        );
        let orbit = SparseOrbit::from_pv(pv_local + perturb, orbit.body, self.universe.stamp())?;
        let id = self.ids.next();
        self.universe.orbiters.insert(
            id,
            Orbiter::new(GlobalOrbit(parent, orbit), self.universe.stamp()),
        );
        let name = vehicle.name().to_string();
        self.universe.vehicles.insert(id, vehicle);
        self.notice(format!(
            "Spawned {} {} in orbit around {}",
            name, id, global.0
        ));
        Some(())
    }

    pub fn spawn_new(&mut self) -> Option<()> {
        let orbit = self.cursor_orbit_if_mode()?;
        let vehicle = self.get_random_vehicle()?;
        self.spawn_with_random_perturbance(orbit, vehicle)
    }

    pub fn delete_orbiter(&mut self, id: EntityId) -> Option<()> {
        let lup = self.universe.lup_orbiter(id, self.universe.stamp())?;
        let _orbiter = lup.orbiter()?;
        let parent = lup.parent(self.universe.stamp())?;
        let pv = lup.pv().pos_f32();
        let plup = self.universe.lup_planet(parent, self.universe.stamp())?;
        let pvp = plup.pv().pos_f32();
        let pvl = pv - pvp;
        self.universe.orbiters.remove(&id)?;
        self.universe.vehicles.remove(&id);
        self.notify(
            ObjectId::Planet(parent),
            NotificationType::OrbiterDeleted(id),
            pvl,
        );
        Some(())
    }

    pub fn nearest(&self, pos: Vec2, stamp: Nanotime) -> Option<ObjectId> {
        let results = crate::scenes::all_orbital_ids(self)
            .filter_map(|id| {
                let lup = match id {
                    ObjectId::Orbiter(id) => self.universe.lup_orbiter(id, stamp),
                    ObjectId::Planet(id) => self.universe.lup_planet(id, stamp),
                }?;
                let p = lup.pv().pos_f32();
                let d = pos.distance(p);
                Some((d, id))
            })
            .collect::<Vec<_>>();
        results
            .into_iter()
            .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
            .map(|(_, id)| id)
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

    pub fn impulsive_burn(&mut self, id: EntityId, stamp: Nanotime, dv: Vec2) -> Option<()> {
        let obj = self.universe.orbiters.get_mut(&id)?;
        obj.try_impulsive_burn(stamp, dv)?;
        Some(())
    }

    pub fn swap_ownship_target(&mut self) {
        let tmp = self.orbital_context.targeting;
        self.orbital_context.targeting = self.orbital_context.piloting;
        self.orbital_context.piloting = tmp;
    }

    pub fn write_editor_to_ownship(&mut self) -> Option<()> {
        let id = match self.piloting() {
            Some(p) => p,
            None => {
                self.notice("No ownship to write to");
                return None;
            }
        };

        let vehicle = match self.universe.vehicles.get_mut(&id) {
            Some(v) => v,
            None => {
                self.notice(format!("Failed to find vehicle for id {}", id));
                return None;
            }
        };

        let new_vehicle = self.editor_context.vehicle().clone();

        let old_title = vehicle.name().to_string();
        let new_title = new_vehicle.name().to_string();

        *vehicle = new_vehicle;

        self.notice(format!(
            "Successfully overwrite vehicle {}, \"{}\" -> \"{}\"",
            id, old_title, new_title
        ));

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

    pub fn command(&mut self, id: EntityId, next: &GlobalOrbit) -> Option<()> {
        let tracks = self.orbital_context.selected.clone();
        let vehicle = self.universe.vehicles.get(&id)?;
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
                let ret = c.set_destination(*next, self.universe.stamp());
                if let Err(_e) = ret {
                    // dbg!(e);
                }
            }
        });

        Some(())
    }

    pub fn notice(&mut self, s: impl Into<String>) {
        let s = s.into();
        info!("Notice: {s}");
        self.console.log(s);
    }

    pub fn notify(
        &mut self,
        parent: impl Into<Option<ObjectId>>,
        kind: NotificationType,
        offset: impl Into<Option<Vec2>>,
    ) {
        let notif = Notification {
            parent: parent.into(),
            offset: offset.into().unwrap_or(Vec2::ZERO),
            jitter: Vec2::ZERO,
            sim_time: self.universe.stamp(),
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
        let angle = 2.0 * PI * self.universe.stamp().to_secs() / Nanotime::days(365).to_secs();
        rotate(Vec2::X, angle + PI) * 1000000.0
    }

    pub fn save(&mut self) -> Option<()> {
        match self.current_scene().kind() {
            SceneType::Editor => EditorContext::save_to_file(self),
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
        self.button_was_pressed = true;

        match id {
            OnClick::CurrentBody(id) => self.orbital_context.following = Some(ObjectId::Planet(id)),
            OnClick::Orbiter(id) => self.orbital_context.following = Some(ObjectId::Orbiter(id)),
            OnClick::ToggleDrawMode => {
                self.orbital_context.draw_mode = next_cycle(&self.orbital_context.draw_mode)
            }
            OnClick::ClearTracks => self.orbital_context.selected.clear(),
            OnClick::ClearOrbits => self.orbital_context.queued_orbits.clear(),
            OnClick::Group(gid) => self.toggle_group(gid),
            OnClick::CreateGroup => {
                let id = self.ids.next();
                self.create_group(id);
            }
            OnClick::DisbandGroup(gid) => self.disband_group(gid),
            OnClick::CommitMission => {
                self.commit_mission();
            }
            OnClick::Exit => self.shutdown_with_prompt(),
            OnClick::SimSpeed(s) => {
                self.universe_ticks_per_game_tick = s;
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
            OnClick::ThrottleLevel(throttle) => {
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
            OnClick::TogglePartsMenuCollapsed => {
                self.editor_context.parts_menu_collapsed = !self.editor_context.parts_menu_collapsed
            }
            OnClick::ToggleVehiclesMenuCollapsed => {
                self.editor_context.vehicles_menu_collapsed =
                    !self.editor_context.vehicles_menu_collapsed
            }
            OnClick::ToggleLayersMenuCollapsed => {
                self.editor_context.layers_menu_collapsed =
                    !self.editor_context.layers_menu_collapsed
            }
            OnClick::IncrementThrottle(d) => {
                self.orbital_context.throttle.increment(d);
            }
            OnClick::OpenNewCraft => {
                self.editor_context.new_craft();
            }
            OnClick::WriteVehicleToImage => {
                self.editor_context.write_image_to_file(&self.args);
            }
            OnClick::RotateCraft => {
                self.editor_context.rotate_craft();
            }
            OnClick::ToggleVehicleInfo => {
                self.editor_context.show_vehicle_info = !self.editor_context.show_vehicle_info;
            }
            OnClick::WriteToOwnship => {
                self.write_editor_to_ownship();
            }
            OnClick::NormalizeCraft => self.editor_context.normalize_coordinates(),
            OnClick::SwapOwnshipTarget => _ = self.swap_ownship_target(),
            OnClick::AddToFavorites(id) => _ = self.favorites.insert(id),
            OnClick::RemoveFromFavorites(id) => _ = self.favorites.remove(&id),
            OnClick::ReloadGame => _ = self.reload(),
            OnClick::IncreaseGravity => self.universe.surface.increase_gravity(),
            OnClick::DecreaseGravity => self.universe.surface.decrease_gravity(),

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

    pub fn get_random_vehicle(&self) -> Option<Vehicle> {
        let vehicles = crate::scenes::get_list_of_vehicles(self).unwrap_or(vec![]);

        if vehicles.is_empty() {
            return None;
        }

        let choice = randint(0, vehicles.len() as i32);
        let (name, path) = vehicles.get(choice as usize)?;

        let sat = EditorContext::load_from_vehicle_file(&path)?;

        let mut parts = vec![];
        for part in sat.parts {
            let proto = self.part_database.get(&part.partname)?;
            parts.push((part.pos, part.rot, proto.clone()));
        }

        let vehicle = Vehicle::from_parts(name.to_string(), parts);

        Some(vehicle)
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

    fn maybe_trigger_click_event(&mut self) -> Option<()> {
        use FrameId::*;
        use MouseButt::*;

        let wb = self.input.screen_bounds.span;

        let p = self.input.position(Left, Down)?;
        let q = self.input.position(Left, Up)?;
        let n = self.ui.at(p, wb)?;
        let m = self.ui.at(q, wb)?;
        if !n.is_enabled() || !m.is_enabled() {
            return None;
        }
        let n = n.on_click()?;
        let m = m.on_click()?;
        if n == m {
            self.on_button_event(n.clone());
        }
        return Some(());
    }

    fn handle_click_events(&mut self) {
        use FrameId::*;
        use MouseButt::*;

        if self.input.on_frame(Left, Up).is_some() {
            self.maybe_trigger_click_event();
        }

        if self.input.on_frame(Left, Up).is_some() {
            let h = &self.orbital_context.highlighted;
            self.orbital_context.selected.extend(h.into_iter());
            self.orbital_context.highlighted.clear();
        }

        if self.input.on_frame(Right, Up).is_some() {}
    }

    pub fn on_render_tick(&mut self) {
        self.render_ticks += 1;

        if let Some((decl, args)) = self.console.process_input(&mut self.input) {
            decl.execute(self, args);
        }

        self.handle_click_events();

        match self.current_scene().kind() {
            SceneType::DockingView => self.rpo_context.handle_input(&self.input),
            SceneType::Editor => EditorContext::on_render_tick(self),
            SceneType::MainMenu => (),
            SceneType::Orbital => self.orbital_context.handle_input(&self.input),
            SceneType::Surface => self
                .surface_context
                .on_render_tick(&self.input, &mut self.universe),
            SceneType::Telescope => self.telescope_context.handle_input(&self.input),
        }
    }

    pub fn on_game_tick(&mut self) {
        self.game_ticks += 1;

        let start = std::time::Instant::now();

        let mut signals = ControlSignals::new();

        signals.gravity = if self.input.is_pressed(KeyCode::KeyG) {
            80.0
        } else {
            5.0
        };

        signals.piloting = keyboard_control_law(&self.input);
        signals.toggle_mode = self.input.just_pressed(KeyCode::KeyM);

        if !self.paused {
            self.universe
                .on_sim_ticks(self.universe_ticks_per_game_tick, &signals);
        }

        self.exec_time = std::time::Instant::now() - start;

        let old_sim_time = self.universe.stamp();

        self.wall_time += PHYSICS_CONSTANT_DELTA_TIME;

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
                let id = self.nearest(w, self.universe.stamp())?;
                self.orbital_context.following = Some(id);
                self.notice(format!("Now following {:?}", id));
            }
            Some(())
        }();

        let s = self.universe.stamp();
        let d = self.physics_duration;

        let planets = self.universe.planets.clone();

        let mut man = self.planned_maneuvers(old_sim_time);
        while let Some((id, t, dv)) = man.first() {
            if s > *t {
                let perturb = 0.0 * randvec(0.01, 0.05);
                simulate(&mut self.universe.orbiters, &planets, *t, d);
                self.impulsive_burn(*id, *t, dv + perturb);
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

        for (id, ri) in simulate(&mut self.universe.orbiters, &planets, s, d) {
            info!("{} {:?}", id, &ri);
            if let Some(pv) = ri.orbit.pv(ri.stamp).ok() {
                let notif = match ri.reason {
                    EventType::Collide(_) => NotificationType::OrbiterCrashed(id),
                    EventType::Encounter(_) => continue,
                    EventType::Escape(_) => NotificationType::OrbiterEscaped(id),
                    EventType::Impulse(_) => continue,
                    EventType::NumericalError => NotificationType::NumericalError(id),
                };
                self.notify(ObjectId::Planet(ri.parent), notif, pv.pos_f32());
            }
        }

        let mut track_list = self.orbital_context.selected.clone();
        track_list.retain(|o| {
            self.universe
                .lup_orbiter(*o, self.universe.stamp())
                .is_some()
        });
        self.orbital_context.selected = track_list;

        let mut notifs = vec![];
        let mut controller_updates = vec![];

        self.controllers.iter().enumerate().for_each(|(i, c)| {
            if !c.needs_update(s) {
                return;
            }

            let lup = self.universe.lup_orbiter(c.target(), s);
            let orbiter = lup.map(|lup| lup.orbiter()).flatten();
            let prop = orbiter.map(|orb| orb.propagator_at(s)).flatten();

            if let Some(prop) = prop {
                controller_updates.push((i, prop.orbit));
            }
        });

        for (i, orbit) in controller_updates {
            if let Some(c) = self.controllers.get_mut(i) {
                let res = c.update(s, orbit);
                if let Err(_) = res {
                    notifs.push((c.target(), NotificationType::ManeuverFailed(c.target())));
                }
            }
        }

        let ids: Vec<_> = self.universe.orbiter_ids().collect();

        for id in ids {
            if !self.universe.vehicles.contains_key(&id) {
                if let Some(v) = self.get_random_vehicle() {
                    self.universe.vehicles.insert(id, v);
                }
            }
        }

        notifs
            .into_iter()
            .for_each(|(t, n)| self.notify(ObjectId::Orbiter(t), n, None));

        let mut finished_ids = Vec::<EntityId>::new();

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
                self.orbital_context.on_game_tick();
            }
            SceneType::Telescope => {
                self.telescope_context.on_game_tick();
            }
            SceneType::DockingView => {
                self.rpo_context.on_game_tick();
            }
            SceneType::Editor => {
                EditorContext::on_game_tick(self);
            }
            SceneType::Surface => {
                SurfaceContext::on_game_tick(self);
            }
            _ => (),
        }
    }
}

fn on_game_tick(mut state: ResMut<GameState>, mut images: ResMut<Assets<Image>>) {
    state.on_game_tick();

    if state.image_handles.is_empty() {
        state.load_sprites(&mut images)
    }
}

fn on_render_tick(mut state: ResMut<GameState>) {
    state.on_render_tick();
}

pub const MIN_SIM_SPEED: u32 = 0;
pub const MAX_SIM_SPEED: u32 = 30;

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
            if state.universe_ticks_per_game_tick > 0 {
                state.universe_ticks_per_game_tick = u32::clamp(
                    state.universe_ticks_per_game_tick - 1,
                    MIN_SIM_SPEED,
                    MAX_SIM_SPEED,
                );
            }
        }
        InteractionEvent::SimFaster => {
            state.universe_ticks_per_game_tick = u32::clamp(
                state.universe_ticks_per_game_tick + 1,
                MIN_SIM_SPEED,
                MAX_SIM_SPEED,
            );
        }
        InteractionEvent::SetSim(s) => {
            state.universe_ticks_per_game_tick = u32::clamp(*s, MIN_SIM_SPEED, MAX_SIM_SPEED);
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
        InteractionEvent::ToggleDebugConsole => {
            state.console.toggle();
        }
        InteractionEvent::Escape => {
            if state.console.is_active() {
                state.console.hide()
            } else if !state.is_exit_prompt {
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
        InteractionEvent::ToggleObject(id) => {
            state.orbital_context.toggle_track(*id);
        }
        InteractionEvent::ToggleGroup(gid) => {
            state.toggle_group(*gid);
        }
        InteractionEvent::DisbandGroup(gid) => {
            state.disband_group(*gid);
        }
        InteractionEvent::CreateGroup => {
            let gid = state.ids.next();
            state.create_group(gid);
        }
        _ => (),
    };
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
