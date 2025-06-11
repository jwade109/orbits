use crate::args::ProgramContext;
use crate::canvas::Canvas;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::InputState;
use crate::input::{FrameId, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render, TextLabel};
use crate::ui::*;
use bevy::color::palettes::css::*;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::prelude::*;
use enum_iterator::next_cycle;
use image::{DynamicImage, RgbaImage};
use layout::layout::*;
use layout::layout::{Node, Tree};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use starling::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleFileStorage {
    pub name: String,
    pub parts: Vec<VehiclePartFileStorage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehiclePartFileStorage {
    pub partname: String,
    pub pos: IVec2,
    pub rot: Rotation,
}

#[derive(Debug)]
pub struct TextInput(String);

impl TextInput {
    #[allow(unused)]
    fn on_button(&mut self, key: &KeyboardInput) {
        match &key.logical_key {
            Key::Character(c) => {
                if self.0.len() > 30 {
                    self.0.clear();
                }
                self.0 += c;
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct EditorContext {
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
    parts: Vec<(IVec2, Rotation, PartProto)>,
    current_part: Option<PartProto>,
    rotation: Rotation,
    filepath: Option<PathBuf>,
    title: TextInput,
    invisible_layers: HashSet<PartLayer>,
    occupied: HashMap<PartLayer, HashMap<IVec2, usize>>,
    pub vehicle: Vehicle,

    // menus
    pub show_vehicle_info: bool,
    pub parts_menu_collapsed: bool,
    pub vehicles_menu_collapsed: bool,
    pub layers_menu_collapsed: bool,
}

impl EditorContext {
    pub fn new() -> Self {
        EditorContext {
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 20.0,
            target_scale: 18.0,
            parts: Vec::new(),
            current_part: None,
            rotation: Rotation::East,
            filepath: None,
            title: TextInput("".into()),
            invisible_layers: HashSet::new(),
            occupied: HashMap::new(),
            vehicle: Vehicle::from_parts("".into(), Nanotime::zero(), Vec::new()),
            show_vehicle_info: false,
            parts_menu_collapsed: false,
            vehicles_menu_collapsed: true,
            layers_menu_collapsed: false,
        }
    }

    pub fn cursor_box(&self, input: &InputState) -> Option<AABB> {
        let p1 = input.position(MouseButt::Left, FrameId::Down)?;
        let p2 = input.position(MouseButt::Left, FrameId::Current)?;
        Some(AABB::from_arbitrary(
            vround(self.c2w(p1)).as_vec2(),
            vround(self.c2w(p2)).as_vec2(),
        ))
    }

    pub fn new_craft(&mut self) {
        self.title.0 = "".to_string();
        self.filepath = None;
        self.parts.clear();
        self.current_part = None;
        self.update();
    }

    pub fn write_to_image(&self, args: &ProgramContext) {
        write_to_image(&self.vehicle, args, "vehicle");
    }

    pub fn rotate_craft(&mut self) {
        for (p, rot, part) in &mut self.parts {
            let old_half_dims = dims_with_rotation(*rot, part).as_vec2() / 2.0;
            let old_center = p.as_vec2() + old_half_dims;
            let new_center = rotate(old_center, PI / 2.0);
            *rot = enum_iterator::next_cycle(rot);
            let new_half_dims = dims_with_rotation(*rot, part).as_vec2() / 2.0;
            let new_corner = new_center - new_half_dims;
            *p = vround(new_corner);
        }
        self.update();
    }

    pub fn normalize_coordinates(&mut self) {
        if self.parts.is_empty() {
            return;
        }

        let mut min: IVec2 = IVec2::ZERO;
        let mut max: IVec2 = IVec2::ZERO;

        self.parts.iter().for_each(|(p, rot, part)| {
            let dims = dims_with_rotation(*rot, part);
            let q = *p + dims.as_ivec2();
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(q.x);
            max.y = max.y.max(q.y);
        });

        let avg = min + (max - min) / 2;

        self.parts.iter_mut().for_each(|(p, _, _)| {
            *p = *p - avg;
        });

        self.update();
    }

    pub fn set_current_part(state: &mut GameState, name: &String) {
        state.editor_context.current_part = state.part_database.get(name).cloned();
    }

    fn open_existing_file(&mut self) -> Option<PathBuf> {
        if let Some(p) = FileDialog::new().set_directory("/").pick_file() {
            self.filepath = Some(p);
        }
        self.filepath.clone()
    }

    fn open_file_to_save(&mut self) -> Option<PathBuf> {
        if self.filepath.is_none() {
            self.filepath = FileDialog::new().set_directory("/").save_file()
        };
        self.filepath.clone()
    }

    fn visible_parts(&self) -> impl Iterator<Item = &(IVec2, Rotation, PartProto)> {
        self.parts.iter().filter(|(_, _, part)| {
            let layer = part.data.layer;
            !self.invisible_layers.contains(&layer)
        })
    }

    pub fn is_layer_visible(&self, layer: PartLayer) -> bool {
        !self.invisible_layers.contains(&layer)
    }

    pub fn toggle_layer(&mut self, layer: PartLayer) {
        if self.invisible_layers.contains(&layer) {
            self.invisible_layers.remove(&layer);
        } else {
            self.invisible_layers.insert(layer);
        }
    }

    pub fn save_to_file(state: &mut GameState) -> Option<()> {
        let choice = state.editor_context.open_file_to_save()?;
        state.notice(format!("Saving to {}", choice.display()));

        let parts = state
            .editor_context
            .parts
            .iter()
            .map(|(pos, rot, part)| VehiclePartFileStorage {
                partname: part.path.clone(),
                pos: *pos,
                rot: *rot,
            })
            .collect();

        let storage = VehicleFileStorage {
            name: state.editor_context.title.0.clone(),
            parts,
        };

        let s = serde_yaml::to_string(&storage).ok()?;
        std::fs::write(choice, s).ok()
    }

    pub fn load_from_file(state: &mut GameState) -> Option<()> {
        let choice = state.editor_context.open_existing_file()?;
        EditorContext::load_vehicle(&choice, state)
    }

    pub fn load_from_vehicle_file(path: &Path) -> Option<VehicleFileStorage> {
        let s = std::fs::read_to_string(path).ok()?;
        serde_yaml::from_str(&s).ok()
    }

    pub fn load_vehicle(path: &Path, state: &mut GameState) -> Option<()> {
        state.notice(format!("Loading vehicle from {}", path.display()));
        let s = std::fs::read_to_string(path).ok()?;
        let storage: VehicleFileStorage = serde_yaml::from_str(&s).ok()?;
        state.notice(format!("Loaded vehicle \"{}\"", storage.name));

        state.editor_context.parts.clear();
        for ps in storage.parts {
            if let Some(part) = state.part_database.get(&ps.partname) {
                state
                    .editor_context
                    .parts
                    .push((ps.pos, ps.rot, part.clone()));
            }
        }
        state.editor_context.title.0 = storage.name;
        state.editor_context.filepath = Some(path.to_path_buf());
        state.editor_context.update();
        Some(())
    }

    fn occupied_pixels(pos: IVec2, rot: Rotation, part: &PartProto) -> Vec<IVec2> {
        let mut ret = vec![];
        let wh = dims_with_rotation(rot, part);
        for w in 0..wh.x {
            for h in 0..wh.y {
                let p = pos + UVec2::new(w, h).as_ivec2();
                ret.push(p);
            }
        }
        ret
    }

    fn get_part_at(&self, p: IVec2) -> Option<(IVec2, Rotation, PartProto)> {
        for layer in [
            PartLayer::Exterior,
            PartLayer::Structural,
            PartLayer::Internal,
        ] {
            if !self.is_layer_visible(layer) {
                continue;
            }

            if let Some(occ) = self.occupied.get(&layer) {
                if let Some(idx) = occ.get(&p) {
                    return self.parts.get(*idx).cloned();
                }
            }
        }

        None
    }

    fn update(&mut self) {
        self.occupied.clear();
        for (i, (pos, rot, part)) in self.parts.iter().enumerate() {
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            if let Some(occ) = self.occupied.get_mut(&part.data.layer) {
                for p in pixels {
                    occ.insert(p, i);
                }
            } else {
                let mut occ = HashMap::new();
                for p in pixels {
                    occ.insert(p, i);
                }
                self.occupied.insert(part.data.layer, occ);
            }
        }

        self.vehicle = Vehicle::from_parts("".into(), Nanotime::zero(), self.parts.clone());
    }

    fn try_place_part(&mut self, p: IVec2, new_part: PartProto) -> Option<()> {
        let new_pixels = Self::occupied_pixels(p, self.rotation, &new_part);
        if let Some(occ) = self.occupied.get(&new_part.data.layer) {
            for p in &new_pixels {
                if occ.contains_key(p) {
                    return None;
                }
            }
        }
        self.parts.push((p, self.rotation, new_part));
        self.update();
        Some(())
    }

    fn remove_part_at(&mut self, p: IVec2) {
        self.parts.retain(|(pos, rot, part)| {
            if self.invisible_layers.contains(&part.data.layer) {
                return true;
            }
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            !pixels.contains(&p)
        });
        self.update();
    }

    fn current_part_and_cursor_position(state: &GameState) -> Option<(IVec2, PartProto)> {
        let ctx = &state.editor_context;
        let part = state.editor_context.current_part.clone()?;
        let wh = dims_with_rotation(ctx.rotation, &part).as_ivec2();
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = vround(state.editor_context.c2w(pos));
        Some((pos - wh / 2, part))
    }
}

pub fn part_sprite_path(ctx: &ProgramContext, short_path: &str) -> String {
    ctx.parts_dir()
        .join(format!("{}/skin.png", short_path))
        .to_str()
        .unwrap_or("")
        .to_string()
}

pub fn vehicle_info(vehicle: &Vehicle) -> String {
    let bounds = vehicle.aabb();
    let fuel_economy = if vehicle.remaining_dv() > 0.0 {
        vehicle.fuel_mass() / vehicle.remaining_dv()
    } else {
        0.0
    };
    [
        format!("Dry mass: {:0.1} kg", vehicle.dry_mass),
        format!("Fuel: {:0.1} kg", vehicle.fuel_mass()),
        format!("Wet mass: {:0.1} kg", vehicle.wet_mass()),
        format!("Thrusters: {}", vehicle.thruster_count()),
        format!("Thrust: {:0.2} kN", vehicle.thrust() / 1000.0),
        format!("Tanks: {}", vehicle.tank_count()),
        format!("Accel: {:0.2} g", vehicle.accel() / 9.81),
        format!("Ve: {:0.1} s", vehicle.average_linear_exhaust_velocity()),
        format!("DV: {:0.1} m/s", vehicle.remaining_dv()),
        format!("WH: {:0.2}x{:0.2}", bounds.span.x, bounds.span.y),
        format!("Econ: {:0.2} kg-s/m", fuel_economy),
    ]
    .into_iter()
    .map(|s| format!("{s}\n"))
    .collect()
}

impl Render for EditorContext {
    fn background_color(_state: &GameState) -> bevy::color::Srgba {
        GRAY.with_luminance(0.12)
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        use crate::ui::*;

        let vb = state.input.screen_bounds;
        if vb.span.x == 0.0 || vb.span.y == 0.0 {
            return None;
        }

        let top_bar = top_bar(state);
        let parts = part_selection(state);
        let layers = layer_selection(state);
        let vehicles = vehicle_selection(state);
        let other_buttons = other_buttons();

        let main_area = Node::grow()
            .invisible()
            .with_child(parts)
            .with_child(layers)
            .with_child(vehicles)
            .with_child(Node::grow().invisible())
            .with_child(other_buttons);

        let layout = Node::structural(vb.span.x, vb.span.y)
            .tight()
            .invisible()
            .down()
            .with_child(top_bar)
            .with_child(main_area);

        Some(Tree::new().with_layout(layout, Vec2::ZERO))
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.editor_context;
        draw_cross(&mut canvas.gizmos, ctx.w2c(Vec2::ZERO), 10.0, GRAY);

        let radius = ctx.vehicle.bounding_radius();
        let bounds = ctx.vehicle.aabb();

        let filename = match &state.editor_context.filepath {
            Some(p) => format!("[{}]", p.display()),
            None => "[No file open]".to_string(),
        };

        let vehicle_info = vehicle_info(&ctx.vehicle);

        let info: String = [
            filename,
            format!("Title: {:?}", &state.editor_context.title.0),
            format!("{} parts", state.editor_context.parts.len()),
            format!("Rotation: {:?}", state.editor_context.rotation),
        ]
        .into_iter()
        .map(|s| format!("{s}\n"))
        .collect();

        let info = format!("{}{}", info, vehicle_info);

        let half_span = state.input.screen_bounds.span * 0.5;

        canvas.label(
            TextLabel::new(
                info.to_uppercase(),
                Vec2::new(half_span.x - 500.0, half_span.y - 200.0),
                0.7,
            )
            .with_anchor_left(),
        );

        if let Some(p) = state.editor_context.current_part.as_ref() {
            let t = TextLabel::new(format!("{:#?}", &p.data), Vec2::ZERO, 0.8);
            canvas.label(t);
        }

        // axes
        {
            let length = bounds.span.x * PIXELS_PER_METER * 1.5;
            let width = bounds.span.y * PIXELS_PER_METER * 1.5;
            let o = ctx.w2c(Vec2::ZERO);
            let p = ctx.w2c(Vec2::X * length);
            let q = ctx.w2c(Vec2::Y * width);
            canvas.gizmos.line_2d(o, p, RED.with_alpha(0.3));
            canvas.gizmos.line_2d(o, q, GREEN.with_alpha(0.3));
        }

        if let Some(cursor) = state.input.position(MouseButt::Hover, FrameId::Current) {
            let c = ctx.c2w(cursor);

            let discrete = IVec2::new(
                (c.x / 10.0).round() as i32 * 10,
                (c.y / 10.0).round() as i32 * 10,
            );

            for dx in (-100..=100).step_by(10) {
                for dy in (-100..=100).step_by(10) {
                    let s = IVec2::new(dx, dy);
                    let p = discrete - s;
                    let d = (s.length_squared() as f32).sqrt();
                    let alpha = 0.2 * (1.0 - d / 100.0);
                    if alpha > 0.01 {
                        draw_diamond(
                            &mut canvas.gizmos,
                            ctx.w2c(p.as_vec2()),
                            7.0,
                            GRAY.with_alpha(alpha),
                        );
                    }
                }
            }

            if let Some((p, rot, part)) = ctx.get_part_at(vfloor(c)) {
                let wh = dims_with_rotation(rot, &part).as_ivec2();
                let q = p + wh;
                let r = p + IVec2::X * wh.x;
                let s = p + IVec2::Y * wh.y;
                let aabb = aabb_for_part(p, rot, &part);
                draw_and_fill_aabb(&mut canvas.gizmos, ctx.w2c_aabb(aabb), TEAL.with_alpha(0.6));
                for p in [p, q, r, s] {
                    let p = p.as_vec2();
                    draw_cross(
                        &mut canvas.gizmos,
                        ctx.w2c(p),
                        0.5 * ctx.scale(),
                        TEAL.with_alpha(0.6),
                    );
                }
            }
        }

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let current_pixels = Self::occupied_pixels(p, ctx.rotation, &current_part);

            let mut visited_parts = HashSet::new();

            if let Some(occ) = ctx.occupied.get(&current_part.data.layer) {
                for q in &current_pixels {
                    if let Some(idx) = occ.get(q) {
                        if visited_parts.contains(idx) {
                            continue;
                        }
                        visited_parts.insert(*idx);
                        if let Some((pc, rc, partc)) = ctx.parts.get(*idx) {
                            let aabb = aabb_for_part(*pc, *rc, partc);
                            draw_and_fill_aabb(&mut canvas.gizmos, ctx.w2c_aabb(aabb), RED);
                        }
                    }
                }
            }
        }

        if ctx.show_vehicle_info {
            draw_aabb(
                &mut canvas.gizmos,
                ctx.w2c_aabb(bounds.scale(PIXELS_PER_METER)),
                TEAL.with_alpha(0.1),
            );

            draw_circle(
                &mut canvas.gizmos,
                ctx.w2c(Vec2::ZERO),
                radius * ctx.scale() * PIXELS_PER_METER,
                RED.with_alpha(0.1),
            );

            draw_vehicle(
                &mut canvas.gizmos,
                &ctx.vehicle,
                ctx.w2c(Vec2::ZERO),
                ctx.scale * PIXELS_PER_METER,
                0.0,
            );

            // COM
            let com = ctx.vehicle.center_of_mass() * PIXELS_PER_METER;
            draw_circle(&mut canvas.gizmos, ctx.w2c(com), 7.0, ORANGE);
            draw_x(&mut canvas.gizmos, ctx.w2c(com), 7.0, WHITE);

            // thrust envelope
            for (rcs, color) in [(false, RED), (true, BLUE)] {
                let positions: Vec<_> = linspace(0.0, 2.0 * PI, 200)
                    .into_iter()
                    .map(|a| {
                        let thrust: f32 = ctx.vehicle.max_thrust_along_heading(a, rcs);
                        let r = (1.0 + thrust.abs().sqrt() / 100.0)
                            * ctx.vehicle.bounding_radius()
                            * PIXELS_PER_METER;
                        ctx.w2c(rotate(Vec2::X * r, a))
                    })
                    .collect();
                canvas.gizmos.linestrip_2d(positions, color.with_alpha(0.6));
            }
        }

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let dims = dims_with_rotation(ctx.rotation, &current_part);
            canvas.sprite(
                ctx.w2c(p.as_vec2() + dims.as_vec2() / 2.0),
                ctx.rotation.to_angle(),
                part_sprite_path(&state.args, &current_part.path),
                ctx.scale(),
                12.0,
            );
        }

        ctx.visible_parts()
            .enumerate()
            .for_each(|(i, (pos, rot, part))| {
                let half_dims = dims_with_rotation(*rot, part).as_vec2() / 2.0;
                canvas.sprite(
                    ctx.w2c(pos.as_vec2() + half_dims),
                    rot.to_angle(),
                    part_sprite_path(&state.args, &part.path),
                    ctx.scale(),
                    part.to_z_index() + i as f32 / 100.0,
                );
            });

        Some(())
    }
}

fn aabb_for_part(p: IVec2, rot: Rotation, part: &PartProto) -> AABB {
    let wh = dims_with_rotation(rot, part).as_ivec2();
    let q = p + wh;
    AABB::from_arbitrary(p.as_vec2(), q.as_vec2())
}

fn expandable_menu(text: &str, onclick: OnClick) -> Node<OnClick> {
    Node::structural(300, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::button(text, onclick, Size::Grow, BUTTON_HEIGHT))
}

fn part_selection(state: &GameState) -> Node<OnClick> {
    let mut part_names: Vec<_> = state.part_database.keys().collect();
    part_names.sort();

    let mut n = expandable_menu("Parts", OnClick::TogglePartsMenuCollapsed);

    if !state.editor_context.parts_menu_collapsed {
        n.add_child(Node::hline());
        n.add_children(part_names.into_iter().map(|s| {
            let onclick = OnClick::SelectPart(s.clone());
            Node::button(s, onclick, Size::Grow, BUTTON_HEIGHT)
        }));
    }

    n
}

pub fn get_list_of_vehicles(state: &GameState) -> Option<Vec<(String, PathBuf)>> {
    let mut ret = vec![];
    if let Ok(paths) = std::fs::read_dir(&state.args.vehicle_dir()) {
        for path in paths {
            if let Ok(path) = path {
                let s = path.path().file_stem()?.to_string_lossy().to_string();
                ret.push((s, path.path()));
            }
        }
    }
    Some(ret)
}

fn vehicle_selection(state: &GameState) -> Node<OnClick> {
    let vehicles = get_list_of_vehicles(state).unwrap_or(vec![]);

    let mut n = expandable_menu("Vehicles", OnClick::ToggleVehiclesMenuCollapsed);

    if !state.editor_context.vehicles_menu_collapsed {
        n.add_child(Node::hline());
        n.add_children(vehicles.into_iter().map(|(name, path)| {
            let onclick = OnClick::LoadVehicle(path);
            Node::button(name, onclick, Size::Grow, BUTTON_HEIGHT)
        }));
    }

    n
}

fn other_buttons() -> Node<OnClick> {
    let rotate = Node::button("Rotate", OnClick::RotateCraft, Size::Grow, BUTTON_HEIGHT);
    let normalize = Node::button(
        "Normalize",
        OnClick::NormalizeCraft,
        Size::Grow,
        BUTTON_HEIGHT,
    );
    let write = Node::button(
        "To Image",
        OnClick::WriteVehicleToImage,
        Size::Grow,
        BUTTON_HEIGHT,
    );
    let new_button = Node::button("New", OnClick::OpenNewCraft, Size::Grow, BUTTON_HEIGHT);

    let toggle_info = Node::button(
        "Info",
        OnClick::ToggleVehicleInfo,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    let write_to_ownship = Node::button(
        "Modify Ownship",
        OnClick::WriteToOwnship,
        Size::Grow,
        BUTTON_HEIGHT,
    );

    Node::structural(250, Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .down()
        .with_child(new_button)
        .with_child(rotate)
        .with_child(normalize)
        .with_child(write)
        .with_child(toggle_info)
        .with_child(write_to_ownship)
}

fn layer_selection(state: &GameState) -> Node<OnClick> {
    let mut n = expandable_menu("Layers", OnClick::ToggleLayersMenuCollapsed);

    if !state.editor_context.layers_menu_collapsed {
        n.add_child(Node::hline());
        n.add_children(enum_iterator::all::<PartLayer>().into_iter().map(|p| {
            let s = format!("{:?}", p);
            let onclick = OnClick::ToggleLayer(p);
            let mut n = Node::button(s, onclick, Size::Grow, BUTTON_HEIGHT);
            if !state.editor_context.is_layer_visible(p) {
                n = n.with_color(GRAY.to_f32_array());
            }
            n
        }));
    }

    n
}

impl CameraProjection for EditorContext {
    fn origin(&self) -> Vec2 {
        self.center
    }

    fn scale(&self) -> f32 {
        self.scale
    }
}

impl EditorContext {
    pub fn step(state: &mut GameState, dt: f32) {
        let is_hovering = state.is_hovering_over_ui();

        // for input in &state.input.keyboard_events {
        //     state.editor_context.title.on_button(input);
        // }

        let speed = 16.0 * dt * 100.0;

        if !is_hovering {
            if let Some(_) = state.input.position(MouseButt::Left, FrameId::Current) {
                if let Some((p, part)) = Self::current_part_and_cursor_position(state) {
                    state.editor_context.try_place_part(p, part);
                }
            } else if let Some(p) = state.input.position(MouseButt::Right, FrameId::Current) {
                let p = vfloor(state.editor_context.c2w(p));
                state.editor_context.remove_part_at(p);
            } else if state.input.just_pressed(KeyCode::KeyQ) {
                if state.editor_context.current_part.is_some() {
                    state.editor_context.current_part = None;
                } else if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
                    let p = vfloor(state.editor_context.c2w(p));
                    if let Some((_, rot, part)) = state.editor_context.get_part_at(p) {
                        state.editor_context.rotation = rot;
                        state.editor_context.current_part = Some(part);
                    } else {
                        state.editor_context.current_part = None;
                    }
                }
            }
        }

        let ctx = &mut state.editor_context;

        if state.input.is_scroll_down() {
            ctx.target_scale /= 1.5;
        }
        if state.input.is_scroll_up() {
            ctx.target_scale *= 1.5;
        }

        if state.input.just_pressed(KeyCode::KeyR) {
            ctx.rotation = next_cycle(&ctx.rotation);
        }

        if state.input.is_pressed(KeyCode::Equal) {
            ctx.target_scale *= 1.03;
        }
        if state.input.is_pressed(KeyCode::Minus) {
            ctx.target_scale /= 1.03;
        }
        if state.input.is_pressed(KeyCode::KeyD) {
            ctx.target_center.x += speed / ctx.scale;
        }
        if state.input.is_pressed(KeyCode::KeyA) {
            ctx.target_center.x -= speed / ctx.scale;
        }
        if state.input.is_pressed(KeyCode::KeyW) {
            ctx.target_center.y += speed / ctx.scale;
        }
        if state.input.is_pressed(KeyCode::KeyS) {
            ctx.target_center.y -= speed / ctx.scale;
        }

        ctx.scale += (ctx.target_scale - ctx.scale) * 0.1;
        ctx.center += (ctx.target_center - ctx.center) * 0.1;
    }
}

pub fn read_image(path: PathBuf) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

pub fn write_to_image(vehicle: &Vehicle, ctx: &ProgramContext, name: &str) -> Option<()> {
    let outpath = format!("/tmp/{}.png", name);
    println!("Writing vehicle {} to path {}", vehicle.name(), outpath);
    let parts_dir = ctx.parts_dir();
    let (pixel_min, pixel_max) = vehicle.pixel_bounds()?;
    let dims = pixel_max - pixel_min;
    let mut img = DynamicImage::new_rgba8(dims.x as u32, dims.y as u32);
    let to_export = img.as_mut_rgba8().unwrap();
    for (pos, rot, part) in vehicle.parts_by_layer() {
        let path = parts_dir.join(&part.path).join("skin.png");
        let img = match read_image(path.clone()) {
            Some(img) => img,
            None => {
                println!("Failed to read {}", path.display());
                continue;
            }
        };

        let px = (pos.x - pixel_min.x) as u32;
        let py = (pos.y - pixel_min.y) as u32;

        let color = match part.data.class {
            PartClass::Cargo => GREEN,
            PartClass::Thruster(_) => RED,
            PartClass::Tank(_) => ORANGE,
            _ => match part.data.layer {
                PartLayer::Exterior => continue,
                PartLayer::Internal => GRAY,
                PartLayer::Structural => WHITE,
            },
        }
        .mix(&BLACK, 0.3)
        .to_f32_array();

        for x in 0..img.width() {
            for y in 0..img.height() {
                let p = IVec2::new(x as i32, y as i32);
                let xp = img.width() as i32 - p.x - 1;
                let yp = img.height() as i32 - p.y - 1;
                let p = match *rot {
                    Rotation::East => IVec2::new(p.x, yp),
                    Rotation::North => IVec2::new(p.y, p.x),
                    Rotation::West => IVec2::new(xp, p.y),
                    Rotation::South => IVec2::new(yp, xp),
                }
                .as_uvec2();

                let src = img.get_pixel_checked(x, y);
                let dst =
                    to_export.get_pixel_mut_checked(px + p.x, to_export.height() - (py + p.y) - 1);
                if let Some((src, dst)) = src.zip(dst) {
                    if src.0[3] > 0 {
                        for i in 0..4 {
                            dst.0[i] = (color[i] * 255.0) as u8;
                        }
                    }
                }
            }
        }
    }

    to_export.save(outpath).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_vehicle_to_image() {
        let dir = project_root::get_project_root()
            .expect("Expected project root to be discoverable")
            .join("assets");

        dbg!(&dir);

        let args = ProgramContext::new(dir);

        let g = GameState::new(args.clone());

        let vehicles = crate::scenes::craft_editor::get_list_of_vehicles(&g)
            .expect("Expected list of vehicles");
        dbg!(vehicles);

        for name in ["remora", "lander", "pollux", "manta", "spacestation"] {
            let vehicle = g.get_vehicle_by_model(name).expect("Expected a vehicle");
            write_to_image(&vehicle, &args, name);
        }
    }
}
