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
use clap::Parser;
use image::{DynamicImage, RgbaImage};
use std::collections::HashMap;
use std::path::Path;

pub struct GamePlugin;

fn new_editor_ui(
    mut contexts: EguiContexts,
    mut game: ResMut<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    use egui::Align2;

    egui::Window::new("General")
        .anchor(Align2::RIGHT_TOP, (0.0, 0.0))
        .show(ctx, |ui| {
            apply_egui_style(ui);

            ui.label(format!("{:?}", game.editor_context.filepath));
            ui.label(format!("{:?}", game.editor_context.focus_layer));

            if ui.button("Close").clicked() {
                game.shutdown();
            }
            if ui.button("Save").clicked() {
                game.save();
            }
            if ui.button("Open").clicked() {
                game.load();
            }
            if ui.button("Clear").clicked() {
                game.editor_context.blueprint.clear();
            }
            if ui.button("Rotate").clicked() {
                game.editor_context.rotate_craft();
            }
            if ui.button("Pipe").clicked() {
                game.editor_context.enter_pipe_mode();
            }
            if ui.button("Select").clicked() {
                game.editor_context.enter_select_mode();
            }
            if ui.button("Update Graph").clicked() {
                game.editor_context.update_graph();
            }
            if ui.button("Randomize Graph").clicked() {
                game.editor_context.randomize_graph();
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

    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft);

    egui::Window::new("Vehicles").show(ctx, |ui| {
        apply_egui_style(ui);
        if let Some(vehicles) = get_list_of_vehicles(&game) {
            for (name, path) in vehicles {
                if ui.button(name).clicked() {
                    if ctrl_pressed {
                        if let Ok(bp) = load_vehicle(&path, &game.part_database) {
                            game.editor_context.cursor_state = CursorState::Blueprint(bp);
                        }
                    } else {
                        Editor::load_vehicle(&path, &mut game);
                    }
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

    egui::Window::new("Part Info").show(ctx, |ui| {
        let Some(cursor) = Editor::current_cursor_coord(&game) else {
            return;
        };

        ui.label(format!("Cursor: {:?}", cursor));

        for layer in PartLayer::all() {
            let Some(bp) = game.editor_context.blueprint.get_part_at(cursor, layer) else {
                continue;
            };

            let Some(part) = game.editor_context.blueprint.get_part(bp) else {
                continue;
            };

            ui.separator();
            ui.heading(format!("{:?} layer", layer));
            ui.label(format!("{}", part.proto.part_name()));
        }
    });

    egui::Window::new("Pipes").show(ctx, |ui| {
        let n = game.editor_context.blueprint.pipes().count();
        ui.label(format!("{} pipes", n));
        ui.separator();
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

        app.add_systems(FixedUpdate, on_game_tick);
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
    pub cursor_position: Vec2,

    pub settings: Settings,

    /// Contains all states related to window size, mouse clicks and positions,
    /// and button presses and holds.
    pub input: InputState,

    /// Contains CLI arguments
    pub args: ProgramContext,

    pub editor_context: Editor,

    /// Map of names to parts to their definitions. Loaded from
    /// the assets/parts directory
    pub part_database: HashMap<String, PartPrototype>,

    pub is_exit_prompt: bool,

    pub text_labels: Vec<TextLabel>,
    pub sprites: Vec<StaticSpriteDescriptor>,
    pub image_handles: HashMap<String, (Handle<Image>, UVec2)>,

    pub vehicle_names: Vec<String>,
}

fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

impl GameState {
    pub fn new(args: ProgramContext) -> Self {
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

        let vehicle_names = match load_names_from_file(&args.names_path()) {
            Ok(n) => n,
            Err(e) => {
                error!("Failed to load vehicle names: {e}");
                Vec::new()
            }
        };

        GameState {
            cursor_position: Vec2::ZERO,
            settings,
            input: InputState::default(),
            args: args.clone(),
            editor_context: Editor::new(),
            part_database,
            is_exit_prompt: false,
            text_labels: Vec::new(),
            sprites: Vec::new(),
            image_handles: HashMap::new(),
            vehicle_names,
        }
    }

    pub fn load_sprites(&mut self, images: &mut Assets<Image>) {
        let mut handles = HashMap::new();

        for (name, _) in &self.part_database {
            let path = self.args.part_sprite_path(name);
            if let Some(img) = read_image(Path::new(&path)) {
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
            if let Some(img) = read_image(&path) {
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

    pub fn get_vehicle_by_model(&self, name: &str) -> Option<Blueprint> {
        let vehicles = crate::scenes::get_list_of_vehicles(self)?;

        if vehicles.is_empty() {
            return None;
        }

        let (_, path) = vehicles.iter().find(|(model, _)| model == name)?;

        let vehicle = load_vehicle(path, &self.part_database).ok()?;

        Some(vehicle)
    }

    pub fn notice(&mut self, s: impl Into<String>) {
        let s = s.into();
        info!("Notice: {s}");
    }

    pub fn save(&mut self) -> Option<()> {
        Editor::save_to_file(self)
    }

    pub fn load(&mut self) -> Option<()> {
        Editor::load_from_file(self)
    }

    pub fn on_button_event(&mut self, id: OnClick) -> Option<()> {
        match id {
            OnClick::Exit => self.shutdown_with_prompt(),
            OnClick::Nullopt => (),
            OnClick::Save => {
                self.save();
            }
            OnClick::Load => {
                self.load();
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
            OnClick::ReloadGame => _ = self.reload(),

            // BOOKMARK unhandled event
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

    pub fn get_random_vehicle(&self) -> Option<Blueprint> {
        let vehicles = crate::scenes::get_list_of_vehicles(self).unwrap_or(vec![]);

        if vehicles.is_empty() {
            return None;
        }

        let choice = randint(0, vehicles.len() as i32);
        let (_, path) = vehicles.get(choice as usize)?;

        let vehicle = load_vehicle(path, &self.part_database).ok()?;

        Some(vehicle)
    }

    pub fn on_render_tick(&mut self) {
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

        on_editor_render_tick(self);
    }

    pub fn sim_slower(&mut self) {}

    pub fn sim_faster(&mut self) {}

    pub fn on_game_tick(&mut self) {
        Editor::on_game_tick(self);
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
pub const MAX_SIM_SPEED: u32 = 1000000;
