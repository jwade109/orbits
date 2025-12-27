#![allow(unused)]
#![allow(deprecated)]

use crate::prelude::*;
use crate::starling::prelude::*;
use crate::ui::apply_egui_style;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::smaa::Smaa;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::RenderLayers;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use clap::builder::FalseyValueParser;
use clap::Parser;
use image::DynamicImage;
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

fn new_editor_ui(mut contexts: EguiContexts, mut game: ResMut<GameState>) -> Result {
    let ctx = contexts.ctx_mut()?;

    use egui::Align2;

    egui::Window::new("General")
        .anchor(Align2::RIGHT_TOP, (0.0, 0.0))
        .show(ctx, |ui| {
            apply_egui_style(ui);
            if ui.button("Close").clicked() {
                game.shutdown();
            }
            if ui.button("Save").clicked() {
                game.save();
            }
            if ui.button("Open").clicked() {
                game.load();
            }
        });

    egui::Window::new("Parts").show(ctx, |ui| {
        apply_egui_style(ui);
        let mut names: Vec<_> = game.part_database.keys().cloned().collect();
        names.sort();
        for name in names {
            let resp = ui.button(&name);
            if resp.clicked() {
                Editor::set_current_part(&mut game, &name)
            }
        }
    });

    egui::Window::new("Vehicles").show(ctx, |ui| {
        apply_egui_style(ui);
        if let Some(vehicles) = get_list_of_vehicles(&game) {
            for (name, path) in vehicles {
                if ui.button(name).clicked() {
                    Editor::load_vehicle(&path, &mut game);
                }
            }
        }
    });

    egui::Window::new("Layers").show(ctx, |ui| {
        apply_egui_style(ui);
        for layer in PartLayer::all() {
            let old_active = game.editor_context.is_layer_visible(layer);
            let mut active = old_active;

            if ui.checkbox(&mut active, format!("{:?}", layer)).clicked() {
                game.editor_context.toggle_layer(layer);
            }
        }
    });

    Ok(())
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);

        app.insert_resource(Time::<Fixed>::from_duration(
            PHYSICS_CONSTANT_DELTA_TIME.to_duration(),
        ));

        app.add_systems(EguiPrimaryContextPass, new_editor_ui);

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

    /// Contains CLI arguments
    pub args: ProgramContext,

    /// All the game entities and logic therein. This should be able to run
    /// autonomously without any user input with on_sim_tick.
    #[deprecated]
    pub universe: Universe,

    pub editor_context: Editor,

    /// Wall clock, i.e. time since program began.
    pub wall_time: Nanotime,

    pub physics_duration: Nanotime,
    pub paused: bool,
    pub exec_time: std::time::Duration,
    pub actual_universe_ticks_per_game_tick: u32,
    pub using_batch_mode: bool,
    pub force_batch_mode: bool,

    /// Map of names to parts to their definitions. Loaded from
    /// the assets/parts directory
    pub part_database: HashMap<String, PartPrototype>,

    pub current_orbit: Option<usize>,

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
            editor_context: Editor::new(),
            wall_time: Nanotime::zero(),
            physics_duration: Nanotime::days(7),
            actual_universe_ticks_per_game_tick: 0,
            using_batch_mode: false,
            force_batch_mode: false,
            paused: false,
            exec_time: std::time::Duration::new(0, 0),
            part_database,
            current_orbit: None,
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

        Editor::draw(canvas, state);

        Some(())
    }
}

#[deprecated]
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
        unimplemented!()
    }

    pub fn get_vehicle_by_model(&self, name: &str) -> Option<Vehicle> {
        let vehicles = crate::scenes::get_list_of_vehicles(self)?;

        if vehicles.is_empty() {
            return None;
        }

        let (_, path) = vehicles.iter().find(|(model, _)| model == name)?;

        let name = get_random_ship_name(&self.vehicle_names);

        let mut vehicle = load_vehicle(path, name, &self.part_database).ok()?;

        Some(vehicle)
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
        Some(())
    }

    pub fn notice(&mut self, s: impl Into<String>) {
        let s = s.into();
        info!("Notice: {s}");
    }

    pub fn light_source(&self) -> Vec2 {
        let angle = 2.0 * PI * self.universe.stamp().to_secs() / Nanotime::days(365).to_secs();
        rotate(Vec2::X, angle + PI) * 1000000.0
    }

    pub fn save(&mut self) -> Option<()> {
        Editor::save_to_file(self)
    }

    pub fn load(&mut self) -> Option<()> {
        Editor::load_from_file(self)
    }

    pub fn on_button_event(&mut self, id: OnClick) -> Option<()> {
        self.sounds.play_once("button-up.ogg", 1.0);

        dbg!(&id);

        match id {
            OnClick::Exit => self.shutdown_with_prompt(),
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
            OnClick::SelectPart(name) => Editor::set_current_part(self, &name),
            OnClick::ToggleLayer(layer) => self.editor_context.toggle_layer(layer),
            OnClick::LoadVehicle(path) => _ = Editor::load_vehicle(&path, self),
            OnClick::ConfirmExitDialog => self.shutdown(),
            OnClick::DismissExitDialog => self.is_exit_prompt = false,
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
                let name = get_random_ship_name(&self.vehicle_names);
                vehicle.set_name(name);
                self.universe.add_surface_vehicle(
                    e,
                    vehicle,
                    (PI / 2.0 + rand(-0.01, 0.01)) as f64,
                    rand(10.0, 30.0) as f64,
                );
            }
            OnClick::ReloadGame => _ = self.reload(),
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
        unimplemented!()
    }

    pub fn zoom_to_vehicle(&mut self, vehicle: bool) -> Option<()> {
        Some(())
    }

    pub fn reset_camera(&mut self) {}

    pub fn set_controller_policy(&mut self, policy: VehicleControlPolicy) -> Option<()> {
        unimplemented!()
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

        Some(vehicle)
    }

    pub fn is_hovering_over_ui(&self) -> bool {
        false
    }

    pub fn is_currently_left_clicked_on_ui(&self) -> bool {
        false
    }

    fn maybe_trigger_click_event(&mut self) -> Option<()> {
        None
    }

    pub fn handle_click_events(&mut self) {}

    pub fn on_render_tick(&mut self) {
        self.render_ticks += 1;

        if self.input.just_pressed(KeyCode::KeyH) {
            self.reset_camera();
        }

        if self.input.just_pressed(KeyCode::KeyV) {
            self.zoom_to_vehicle(true);
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

        on_editor_render_tick(self);
    }

    pub fn sim_slower(&mut self) {}

    pub fn sim_faster(&mut self) {}

    pub fn on_game_tick(&mut self) {
        self.game_ticks += 1;

        let mut signals = ControlSignals::new();

        // BOOKMARK gameloop
        self.actual_universe_ticks_per_game_tick = 0;
        self.exec_time = std::time::Duration::ZERO;
        if !self.paused {
            (
                self.actual_universe_ticks_per_game_tick,
                self.exec_time,
                self.using_batch_mode,
            ) = self.universe.on_sim_ticks(
                1,
                &signals,
                std::time::Duration::from_millis(10),
                self.settings.draw_thrust_particles,
            )
        }

        self.wall_time += PHYSICS_CONSTANT_DELTA_TIME;

        Editor::on_game_tick(self);
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
}

fn on_render_tick(mut state: ResMut<GameState>) {
    state.on_render_tick();
}

pub const MIN_SIM_SPEED: u32 = 0;
pub const MAX_SIM_SPEED: u32 = 1000000;
