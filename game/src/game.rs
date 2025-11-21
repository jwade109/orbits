use crate::prelude::*;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::smaa::Smaa;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::RenderLayers;
use clap::Parser;
use image::DynamicImage;
use layout::layout::Tree;
use starling::prelude::*;
use std::collections::HashMap;
use std::path::Path;

pub struct GamePlugin;

fn get_entity_info(world: &mut World) {
    let q = world.iter_entities();

    let mut entity_info = HashMap::new();

    for e in q {
        if let Ok(info) = world.inspect_entity(e.id()) {
            for ci in info {
                let s = ci.name().to_string();
                let count: u64 = *entity_info.get(&s).unwrap_or(&0);
                entity_info.insert(s, count + 1);
            }
        }
    }

    let mut game = world.get_resource_mut::<GameState>().unwrap();

    game.entity_info = entity_info;
}

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
                crate::ui::do_text_labels,
            )
                .chain(),
        );

        app.add_systems(
            FixedUpdate,
            (
                // physics
                on_game_tick,
                // rendering
                crate::sounds::sound_system,
                // whatever
                get_entity_info,
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
    pub entity_count: u64,
    pub entity_info: HashMap<String, u64>,

    pub cursor_position: Vec2,

    pub settings: Settings,

    pub sounds: EnvironmentSounds,

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

    pub editor_context: Editor,

    /// Wall clock, i.e. time since program began.
    pub wall_time: Nanotime,

    pub physics_duration: Nanotime,
    pub universe_ticks_per_game_tick: SimRate,
    pub paused: bool,
    pub exec_time: std::time::Duration,
    pub actual_universe_ticks_per_game_tick: u32,
    pub using_batch_mode: bool,
    pub force_batch_mode: bool,

    /// Map of names to parts to their definitions. Loaded from
    /// the assets/parts directory
    pub part_database: HashMap<String, PartPrototype>,

    pub scene: SceneType,

    pub current_orbit: Option<usize>,

    pub ui: Tree<OnClick>,

    pub notifications: Vec<Notification>,

    pub is_exit_prompt: bool,

    pub text_labels: Vec<TextLabel>,
    pub sprites: Vec<StaticSpriteDescriptor>,
    pub image_handles: HashMap<String, (Handle<Image>, UVec2)>,

    pub vehicle_names: Vec<String>,
}

impl GameState {
    pub fn new(args: ProgramContext) -> Self {
        let universe = rss();

        let part_database = match load_parts_from_dir(&args.parts_dir()) {
            Ok(d) => d,
            Err(s) => {
                error!("Failed to load parts: {s}");
                HashMap::new()
            }
        };

        let settings = match load_settings_from_file(&args.settings_path()) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to load settings: {e}");
                Settings::default()
            }
        };

        let mut sounds = EnvironmentSounds::new();
        sounds.play_loop("winter-morning-sea-smoke-bass.ogg", 0.3, TrackTag::Bass);
        sounds.play_loop("winter-morning-sea-smoke-mids.ogg", 0.5, TrackTag::Mids);
        sounds.play_loop("winter-morning-sea-smoke-high.ogg", 0.5, TrackTag::High);
        sounds.play_loop("thrust-noise.ogg", 0.0, TrackTag::Thrust);

        let vehicle_names = match load_names_from_file(&args.names_path()) {
            Ok(n) => n,
            Err(e) => {
                error!("Failed to load vehicle names: {e}");
                Vec::new()
            }
        };

        let mut g = GameState {
            render_ticks: 0,
            game_ticks: 0,
            entity_count: 0,
            entity_info: HashMap::new(),
            cursor_position: Vec2::ZERO,
            settings,
            sounds,
            input: InputState::default(),
            args: args.clone(),
            universe,
            console: DebugConsole::new(),
            orbital_context: OrbitalContext::new(),
            editor_context: Editor::new(),
            wall_time: Nanotime::zero(),
            physics_duration: Nanotime::days(7),
            universe_ticks_per_game_tick: SimRate::RealTime,
            actual_universe_ticks_per_game_tick: 0,
            using_batch_mode: false,
            force_batch_mode: false,
            paused: false,
            exec_time: std::time::Duration::new(0, 0),
            part_database,
            scene: SceneType::Editor,
            current_orbit: None,
            ui: Tree::new(),
            notifications: Vec::new(),
            is_exit_prompt: false,
            text_labels: Vec::new(),
            sprites: Vec::new(),
            image_handles: HashMap::new(),
            vehicle_names,
        };

        let vehicles = [
            ("spacestation", "Earth"),
            ("lander", "Earth"),
            ("pollux", "Earth"),
            ("remora", "Earth"),
            ("bellerophon", "Earth"),
            ("icecream", "Earth"),
            ("pollux", "Luna"),
            ("bellerophon", "Luna"),
            ("remora", "Luna"),
            ("remora", "Luna"),
        ];

        // for (name, parent) in vehicles {
        //     let parent = g.universe.get_planet_by_name(parent);
        //     if let Some(parent) = parent {
        //         let vehicle = g.get_vehicle_by_model(name);
        //         let orbit = get_random_orbit(&g.universe, parent);
        //         if let (Some(orbit), Some(vehicle)) = (orbit, vehicle) {
        //             g.spawn_with_random_perturbance(orbit, vehicle);
        //         }
        //     }
        // }

        for (vehicle, _) in vehicles {
            let vehicle = g.get_vehicle_by_model(vehicle);

            let p = randvec(1000.0, 6000.0).as_dvec2();

            if let Some(v) = vehicle {
                g.spawn_new_at(v, EntityId(0), PV::pos(p));
            }
        }

        let ast = Asteroid::random(200.0, None);

        g.universe.spawn_asteroid(ast);

        g
    }

    pub fn load_sprites(&mut self, images: &mut Assets<Image>) {
        let mut handles = HashMap::new();

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
            "low-fuel",
            "low-fuel-dim",
            "muted",
            // "radar",
            // "radar-dim",
            "ctrl",
            "ctrl-dim",
            "shipscope",
            "launch-icon",
            "prograde-icon",
            "retrograde-icon",
            "clear-icon",
            "heading-icon",
        ] {
            let path = self.args.assets_dir().join(format!("{}.png", name));
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

        self.image_handles = handles;
    }
}

impl Render for GameState {
    fn background_color(state: &GameState) -> Srgba {
        match state.scene {
            SceneType::Orbital => OrbitalContext::background_color(state),
            SceneType::Editor => Editor::background_color(state),
            SceneType::MainMenu => BLACK,
        }
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        match state.scene {
            _ => None,
        }
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        // BOOKMARK debug info

        if state.settings.music_muted {
            let half_span = state.input.screen_bounds.span / 2.0;
            let w = 40.0;
            let dims = Vec2::splat(w);
            let pos = half_span - Vec2::splat(w / 2.0 + 20.0);
            canvas.sprite(pos, 0.0, "muted", ZOrdering::Ui, dims);
        }

        if state.settings.show_debug_info {
            let mut entity_info = Vec::from_iter(state.entity_info.iter());

            entity_info.sort_by_key(|(s, _)| *s);

            let debug_info: String = [
                format!("Wall time: {}", state.wall_time),
                format!("Universe time: {}", state.universe.stamp()),
                format!(
                    "Ideal universe ticks per game tick: {}",
                    state.universe_ticks_per_game_tick.as_ticks(),
                ),
                format!(
                    "Actual universe ticks per game tick: {}",
                    state.actual_universe_ticks_per_game_tick
                ),
                format!("Render ticks: {}", state.render_ticks),
                format!("Game ticks: {}", state.game_ticks),
                format!("Universe ticks: {}", state.universe.ticks()),
                format!("Execution time: {} us", state.exec_time.as_micros()),
                format!("Entity count: {}", state.entity_count),
            ]
            .into_iter()
            .chain(entity_info.iter().map(|(s, c)| format!(" - {}: {}", s, c)))
            .map(|e| format!("{}\n", e))
            .collect();

            let pos = state.input.screen_bounds.span / 2.0;
            let pos = pos.with_x(-pos.x);

            canvas.rect(
                AABB::from_arbitrary(pos, pos + Vec2::new(700.0, -2000.0)),
                ZOrdering::Debug,
                BLACK.with_alpha(0.95),
            );

            canvas
                .text(debug_info, pos + Vec2::new(6.0, -6.0), 0.5)
                .set_anchor(Anchor::TopLeft)
                .set_z_order(ZOrdering::Debug2);
        }

        match state.scene {
            SceneType::Orbital => OrbitalContext::draw(canvas, state),
            SceneType::Editor => Editor::draw(canvas, state),
            SceneType::MainMenu => MainMenuContext::draw(canvas, state),
        }
    }
}

fn keyboard_control_law(input: &InputState) -> VehicleControl {
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

    ctrl
}

impl GameState {
    pub fn reload(&mut self) {
        *self = GameState::new(self.args.clone());
    }

    pub fn set_piloting(&mut self, id: EntityId) {
        self.orbital_context.piloting = Some(id);
    }

    pub fn get_vehicle_by_model(&self, name: &str) -> Option<Vehicle> {
        let vehicles = crate::scenes::get_list_of_vehicles(self)?;

        if vehicles.is_empty() {
            return None;
        }

        let (_, path) = vehicles.iter().find(|(model, _)| model == name)?;

        let name = get_random_ship_name(&self.vehicle_names);

        let mut vehicle = load_vehicle(path, name, &self.part_database).ok()?;

        vehicle.build_all();

        Some(vehicle)
    }

    pub fn piloting(&self) -> Option<EntityId> {
        self.orbital_context.piloting
    }

    pub fn spawn_with_random_perturbance(
        &mut self,
        global: GlobalOrbit,
        vehicle: Vehicle,
    ) -> Option<EntityId> {
        let GlobalOrbit(parent, orbit) = global;
        let pv_local = orbit.pv(self.universe.stamp()).ok()?;
        let perturb = PV::from_f64(randvec(0.01, 0.1), randvec(1.0, 3.0));
        let orbit = SparseOrbit::from_pv(pv_local + perturb, orbit.body, self.universe.stamp())?;
        self.universe
            .add_orbital_vehicle(vehicle, GlobalOrbit(parent, orbit))
    }

    pub fn spawn_new(&mut self) -> Option<EntityId> {
        let id = self.piloting()?;
        let sv = self.universe.spacecraft.get(&id)?;
        let parent = sv.parent();
        let perturb = PV::from_f64(randvec(0.01, 0.1), randvec(0.1, 0.3));
        let pv = sv.pv() + perturb;
        let vehicle = self.get_vehicle_by_model("buoy")?;
        self.spawn_new_at(vehicle, parent, pv)
    }

    pub fn spawn_new_at(&mut self, vehicle: Vehicle, parent: EntityId, pv: PV) -> Option<EntityId> {
        let body = RigidBody {
            pv,
            angle: 0.0,
            angular_velocity: 0.0,
        };
        let controller = VehicleController::idle();
        let sv = Spacecraft::new(parent, vehicle, body, controller);
        self.universe.spawn_spacecraft(sv)
    }

    pub fn delete_orbiter(&mut self, id: EntityId) -> Option<()> {
        let ov = self.universe.spacecraft.remove(&id)?;
        let parent = ov.parent();
        let pv = ov.pv();
        self.notify(parent, NotificationType::OrbiterDeleted(id), pv.pos);
        Some(())
    }

    pub fn write_editor_to_ownship(&mut self) -> Option<()> {
        let id = match self.piloting() {
            Some(p) => p,
            None => {
                self.notice("No ownship to write to");
                return None;
            }
        };

        let ov = match self.universe.spacecraft.get_mut(&id) {
            Some(v) => v,
            None => {
                self.notice(format!("Failed to find vehicle for id {}", id));
                return None;
            }
        };

        let new_vehicle = self.editor_context.vehicle.clone();

        let old_title = ov.vehicle().name().to_string();
        let new_title = new_vehicle.name().to_string();

        ov.overwrite_vehicle(new_vehicle);

        self.notice(format!(
            "Successfully overwrite vehicle {}, \"{}\" -> \"{}\"",
            id, old_title, new_title
        ));

        Some(())
    }

    pub fn notice(&mut self, s: impl Into<String>) {
        let s = s.into();
        info!("Notice: {s}");
        self.console.log(s);
    }

    pub fn notify(
        &mut self,
        parent: impl Into<Option<EntityId>>,
        kind: NotificationType,
        offset: impl Into<Option<DVec2>>,
    ) {
        let notif = Notification {
            parent: parent.into(),
            offset: offset.into().unwrap_or(DVec2::ZERO),
            jitter: DVec2::ZERO,
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
        match self.scene {
            SceneType::Editor => Editor::save_to_file(self),
            _ => None,
        }
    }

    pub fn load(&mut self) -> Option<()> {
        match self.scene {
            SceneType::Editor => Editor::load_from_file(self),
            _ => None,
        }
    }

    pub fn on_button_event(&mut self, id: OnClick) -> Option<()> {
        self.sounds.play_once("button-up.ogg", 1.0);

        dbg!(&id);

        match id {
            OnClick::CurrentBody(id) => self.orbital_context.following = Some(id),
            OnClick::Orbiter(id) => self.orbital_context.following = Some(id),
            OnClick::Exit => self.shutdown_with_prompt(),
            OnClick::SimSpeed(r) => {
                self.universe_ticks_per_game_tick = r;
            }
            OnClick::TogglePause => self.paused = !self.paused,
            OnClick::Nullopt => (),
            OnClick::Save => {
                self.save();
            }
            OnClick::Load => {
                self.load();
            }
            OnClick::GoToScene(s) => {
                self.set_current_scene(s);
            }
            OnClick::ClearPilot => self.orbital_context.piloting = None,
            OnClick::ClearTarget => {
                if let Some(p) = self.piloting() {
                    if let Some(sv) = self.universe.spacecraft.get_mut(&p) {
                        sv.set_target(None);
                    }
                }
            }
            OnClick::SetPilot(p) => self.orbital_context.piloting = Some(p),
            OnClick::SetTarget(t) => {
                if let Some(p) = self.piloting() {
                    if let Some(sv) = self.universe.spacecraft.get_mut(&p) {
                        sv.set_target(t);
                    }
                }
            }
            OnClick::SelectPart(name) => Editor::set_current_part(self, &name),
            OnClick::ToggleLayer(layer) => self.editor_context.toggle_layer(layer),
            OnClick::LoadVehicle(path) => _ = Editor::load_vehicle(&path, self),
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
            OnClick::SendToSurface(e) => {
                let mut vehicle = self.editor_context.vehicle.clone();
                vehicle.build_all();
                let name = get_random_ship_name(&self.vehicle_names);
                vehicle.set_name(name);
                self.universe.add_surface_vehicle(
                    e,
                    vehicle,
                    (PI / 2.0 + rand(-0.01, 0.01)) as f64,
                    rand(10.0, 30.0) as f64,
                );
            }
            OnClick::NormalizeCraft => self.editor_context.normalize_coordinates(),
            OnClick::ReloadGame => _ = self.reload(),
            OnClick::SetRecipe(id, recipe) => {
                if self.editor_context.vehicle.set_recipe(id, recipe) {
                    self.notice(format!("Set recipe for part {:?} to {:?}", id, recipe));
                } else {
                    self.notice(format!(
                        "Failed to set recipe for part {:?} to {:?}",
                        id, recipe
                    ));
                }
            }
            OnClick::ClearContents(id) => {
                if self.editor_context.vehicle.clear_contents(id) {
                    self.notice(format!("Cleared inventory for part {:?}", id));
                } else {
                    self.notice(format!("Failed to clear inventory for part {:?}", id));
                }
            }
            OnClick::SetControllerPolicy(policy) => {
                self.set_controller_policy(policy);
            }
            OnClick::ZoomToVehicle => {
                self.zoom_to_vehicle(true);
            }
            OnClick::ZoomToOrbit => {
                self.zoom_to_vehicle(false);
            }

            // BOOKMARK unhandled event
            _ => info!("Unhandled button event: {id:?}"),
        };

        Some(())
    }

    pub fn toggle_rcs(&mut self) -> Option<()> {
        let id = self.piloting()?;
        let sv = self.universe.spacecraft.get_mut(&id)?;
        sv.toggle_rcs();
        Some(())
    }

    pub fn zoom_to_vehicle(&mut self, vehicle: bool) -> Option<()> {
        let scale = if vehicle { 0.01 } else { 1000000.0 };
        self.orbital_context.following = Some(self.piloting()?);
        self.orbital_context.camera.set_target_offset(DVec2::ZERO);
        self.orbital_context.camera.set_target_view_distance(scale);
        Some(())
    }

    pub fn reset_camera(&mut self) {
        self.orbital_context.following = None;
        self.orbital_context.camera.follow(EntityId(0), DVec2::ZERO);
        self.orbital_context.camera.set_target_offset(DVec2::ZERO);
        self.orbital_context
            .camera
            .set_target_view_distance(1000000.0);
    }

    pub fn set_controller_policy(&mut self, policy: VehicleControlPolicy) -> Option<()> {
        let piloting = self.piloting()?;
        let sv = self.universe.spacecraft.get_mut(&piloting)?;
        sv.controller.set_policy(policy);
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

    pub fn set_current_scene(&mut self, s: SceneType) -> Option<()> {
        self.scene = s;
        Some(())
    }

    pub fn get_random_vehicle(&self) -> Option<Vehicle> {
        let vehicles = crate::scenes::get_list_of_vehicles(self).unwrap_or(vec![]);

        if vehicles.is_empty() {
            return None;
        }

        let choice = randint(0, vehicles.len() as i32);
        let (_, path) = vehicles.get(choice as usize)?;

        let name = get_random_ship_name(&self.vehicle_names);

        let mut vehicle = load_vehicle(path, name, &self.part_database).ok()?;

        vehicle.build_all();

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
        // TODO this whole function is terrible
        if n == m {
            self.on_button_event(n.clone());
        }
        return Some(());
    }

    pub fn handle_click_events(&mut self) {
        use FrameId::*;
        use MouseButt::*;

        if self.input.on_frame(Left, Up).is_some() {
            self.maybe_trigger_click_event();
        }
    }

    pub fn on_render_tick(&mut self) {
        self.render_ticks += 1;

        if self.input.just_pressed(KeyCode::KeyH) {
            self.reset_camera();
        }

        if self.input.just_pressed(KeyCode::KeyV) {
            self.zoom_to_vehicle(true);
        }

        if self.console.is_active() {
            if let Some((decl, args)) = self.console.process_input(&mut self.input) {
                decl.execute(self, args);
            }
            return;
        }

        if self.input.just_pressed(KeyCode::KeyB) {
            self.spawn_new();
        }

        if self.input.just_pressed(KeyCode::Delete) {
            if let Some(p) = self.piloting() {
                self.delete_orbiter(p);
            }
        }

        if self.input.is_pressed(KeyCode::ShiftLeft) && self.input.is_pressed(KeyCode::ControlLeft)
        {
            let delta = if self.input.just_pressed(KeyCode::Minus) {
                -1.0
            } else if self.input.just_pressed(KeyCode::Equal) {
                1.0
            } else {
                0.0
            };

            self.settings.ui_button_height =
                (self.settings.ui_button_height + delta).clamp(3.0, 40.0);
        }

        self.handle_click_events();

        match self.scene {
            SceneType::Editor => {
                on_editor_render_tick(self);
            }
            SceneType::Orbital => {
                on_orbital_render_tick(self);
            }
            _ => (),
        }
    }

    pub fn sim_slower(&mut self) {
        if let Some(t) = enum_iterator::previous(&self.universe_ticks_per_game_tick) {
            self.universe_ticks_per_game_tick = t;
        }
    }

    pub fn sim_faster(&mut self) {
        if let Some(t) = enum_iterator::next(&self.universe_ticks_per_game_tick) {
            self.universe_ticks_per_game_tick = t;
        }
    }

    pub fn on_game_tick(&mut self) {
        self.game_ticks += 1;

        let mut signals = ControlSignals::new();

        if let Some(id) = self.piloting() {
            let cmd = keyboard_control_law(&self.input);

            let throttle_rate = if self.input.is_pressed(KeyCode::BracketRight) {
                1.0
            } else if self.input.is_pressed(KeyCode::BracketLeft) {
                -1.0
            } else {
                0.0
            };

            if !cmd.is_nullopt() || throttle_rate != 0.0 {
                signals.piloting_commands.insert(id, (cmd, throttle_rate));
            }
        }

        if !signals.is_empty() {
            self.universe_ticks_per_game_tick = SimRate::RealTime;
        }

        // BOOKMARK gameloop
        self.actual_universe_ticks_per_game_tick = 0;
        self.exec_time = std::time::Duration::ZERO;
        if !self.paused {
            (
                self.actual_universe_ticks_per_game_tick,
                self.exec_time,
                self.using_batch_mode,
            ) = self.universe.on_sim_ticks(
                self.universe_ticks_per_game_tick.as_ticks(),
                &signals,
                std::time::Duration::from_millis(10),
                self.settings.draw_thrust_particles,
            )
        }

        self.wall_time += PHYSICS_CONSTANT_DELTA_TIME;

        self.notifications.iter_mut().for_each(|n| n.jitter());

        self.notifications
            .retain(|n| n.wall_time + n.duration() > self.wall_time);

        match self.scene {
            SceneType::Orbital => {
                self.orbital_context.on_game_tick(&mut self.universe);
            }
            SceneType::Editor => {
                Editor::on_game_tick(self);
            }
            _ => (),
        }
    }
}

fn on_game_tick(
    mut state: ResMut<GameState>,
    mut images: ResMut<Assets<Image>>,
    entities: Query<Entity>,
) {
    state.on_game_tick();

    if state.image_handles.is_empty() {
        state.load_sprites(&mut images)
    }

    state.entity_count = entities.iter().count() as u64;

    crate::generate_ship_sprites::proc_gen_ship_sprites(&mut state, &mut images);
}

fn on_render_tick(mut state: ResMut<GameState>) {
    state.on_render_tick();
}

pub const MIN_SIM_SPEED: u32 = 0;
pub const MAX_SIM_SPEED: u32 = 1000000;
