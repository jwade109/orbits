use crate::args::ProgramContext;
use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::craft_editor::*;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::InputState;
use crate::input::{FrameId, MouseButt};
use crate::names::*;
use crate::onclick::OnClick;
use crate::scenes::{CameraProjection, Render};
use crate::ui::*;
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use layout::layout::{Node, Size, Tree};
use rfd::FileDialog;
use starling::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum Action {
    Add(IVec2, Rotation, PartPrototype),
    Remove(IVec2, Rotation, PartPrototype),
}

impl Action {
    pub fn to_string(&self) -> String {
        match self {
            Self::Add(_, _, proto) => format!("Add {}", proto.part_name()),
            Self::Remove(_, _, proto) => format!("Remove {}", proto.part_name()),
        }
    }
}

#[derive(Debug)]
pub struct EditorContext {
    camera: LinearCameraController,
    cursor_state: CursorState,
    rotation: Rotation,
    filepath: Option<PathBuf>,
    focus_layer: Option<PartLayer>,
    selected_part: Option<PartId>,
    snap_info: Option<(IVec2, UVec2)>,
    action_queue: Vec<Action>,
    occupied: HashMap<PartLayer, HashMap<IVec2, PartId>>,
    pub vehicle: Vehicle,
    particles: ThrustParticleEffects,
    build_particles: Vec<BuildParticle>,

    atmo: i32,

    // menus
    pub show_vehicle_info: bool,
    pub parts_menu_collapsed: bool,
    pub vehicles_menu_collapsed: bool,
    pub layers_menu_collapsed: bool,

    // construction bots
    pub bots: Vec<ConBot>,
}

impl EditorContext {
    pub fn new() -> Self {
        EditorContext {
            camera: LinearCameraController::new(DVec2::ZERO, 18.0, 1100.0),
            cursor_state: CursorState::None,
            rotation: Rotation::East,
            filepath: None,
            focus_layer: None,
            selected_part: None,
            snap_info: None,
            action_queue: Vec::new(),
            occupied: HashMap::new(),
            vehicle: Vehicle::new(),
            particles: ThrustParticleEffects::new(),
            build_particles: Vec::new(),
            atmo: 3,
            show_vehicle_info: false,
            parts_menu_collapsed: false,
            vehicles_menu_collapsed: true,
            layers_menu_collapsed: false,
            bots: (0..24)
                .map(|_| {
                    let p = randvec(10.0, 50.0);
                    let v = randvec(3.0, 6.0);
                    ConBot::new(PV::from_f64(p, v))
                })
                .collect(),
        }
    }

    pub fn remove_part(&mut self, id: PartId) {
        self.vehicle.remove_part(id);
    }

    pub fn undo(&mut self) -> Option<()> {
        let action = self.action_queue.pop()?;
        match action {
            Action::Add(pos, _, proto) => match self.vehicle.remove_part_at(pos, proto.layer()) {
                Ok(p) => println!("Removed {:?}", p),
                Err(s) => println!("Failed to remove: {}", s),
            },
            Action::Remove(pos, rot, proto) => self.add_part(pos, rot, proto),
        }
        Some(())
    }

    pub fn selected_part(&self) -> Option<&InstantiatedPart> {
        self.vehicle.get_part(self.selected_part?)
    }

    pub fn cursor_box(&self, input: &InputState) -> Option<AABB> {
        let p1 = input.position(MouseButt::Left, FrameId::Down)?;
        let p2 = input.position(MouseButt::Left, FrameId::Current)?;
        Some(AABB::from_arbitrary(
            vround_f64(self.c2w(p1)).as_vec2(),
            vround_f64(self.c2w(p2)).as_vec2(),
        ))
    }

    pub fn new_craft(&mut self) {
        self.filepath = None;
        self.vehicle = Vehicle::new();
        self.cursor_state = CursorState::None;
        self.update();
    }

    pub fn write_image_to_file(&self, args: &ProgramContext) {
        write_image_to_file(&self.vehicle, args, "vehicle");
    }

    pub fn rotate_craft(&mut self) {
        let new_instances: Vec<_> = self
            .vehicle
            .parts()
            .map(|(_, instance)| instance.rotated())
            .collect();
        self.vehicle.clear();
        for instance in new_instances {
            self.vehicle
                .add_part(instance.prototype(), instance.origin(), instance.rotation());
        }
        self.update();
    }

    pub fn normalize_coordinates(&mut self) {
        self.vehicle.normalize_coordinates();
        self.update();
    }

    pub fn set_current_part(state: &mut GameState, name: &String) {
        if let Some(part) = state.part_database.get(name).cloned() {
            state.editor_context.cursor_state = CursorState::Part(part);
        }
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

    pub fn is_layer_visible(&self, layer: PartLayer) -> bool {
        if let Some(focus) = self.focus_layer {
            focus == layer
        } else {
            true
        }
    }

    pub fn toggle_layer(&mut self, layer: PartLayer) {
        self.focus_layer = if self.focus_layer == Some(layer) {
            None
        } else {
            Some(layer)
        };
    }

    pub fn save_to_file(state: &mut GameState) -> Option<()> {
        let choice: PathBuf = state.editor_context.open_file_to_save()?;
        state.notice(format!("Saving to {}", choice.display()));

        let parts = state
            .editor_context
            .vehicle
            .parts()
            .map(|(_, instance)| VehiclePartFileStorage {
                partname: instance.prototype().sprite_path().to_string(),
                pos: instance.origin(),
                rot: instance.rotation(),
            })
            .collect();

        let storage = VehicleFileStorage {
            name: state.editor_context.vehicle.model().to_string(),
            parts,
            lines: state.editor_context.vehicle.pipes().collect(),
        };

        let s = serde_yaml::to_string(&storage).ok()?;
        std::fs::write(choice, s).ok()
    }

    pub fn load_from_file(state: &mut GameState) -> Option<()> {
        let choice = state.editor_context.open_existing_file()?;
        EditorContext::load_vehicle(&choice, state)
    }

    pub fn load_vehicle(path: &Path, state: &mut GameState) -> Option<()> {
        let name = get_random_ship_name(&state.vehicle_names);
        let vehicle = match load_vehicle(path, name, &state.part_database) {
            Ok(v) => v,
            Err(e) => {
                state.notice(format!("Failed to load vehicle: {}", e));
                return None;
            }
        };

        state.editor_context.vehicle = vehicle;
        state.editor_context.filepath = Some(path.to_path_buf());
        state.editor_context.update();
        state.editor_context.vehicles_menu_collapsed = true;
        state.editor_context.action_queue.clear();
        Some(())
    }

    fn get_part_at(&self, p: Vec2) -> Option<(PartId, &InstantiatedPart)> {
        let pixel_p = vround(p * PIXELS_PER_METER);

        for layer in [
            PartLayer::Exterior,
            PartLayer::Structural,
            PartLayer::Internal,
        ] {
            if !self.is_layer_visible(layer) {
                continue;
            }

            if let Some(occ) = self.occupied.get(&layer) {
                if let Some(idx) = occ.get(&pixel_p) {
                    return Some((*idx, self.vehicle.get_part(*idx)?));
                }
            }
        }

        None
    }

    fn update(&mut self) {
        self.occupied.clear();
        for (id, instance) in self.vehicle.parts() {
            let pixels = occupied_pixels(
                instance.origin(),
                instance.rotation(),
                &instance.prototype(),
            );
            if let Some(occ) = self.occupied.get_mut(&instance.prototype().layer()) {
                for p in pixels {
                    occ.insert(p, *id);
                }
            } else {
                let mut occ = HashMap::new();
                for p in pixels {
                    occ.insert(p, *id);
                }
                self.occupied.insert(instance.prototype().layer(), occ);
            }
        }
    }

    fn add_part(&mut self, p: IVec2, rot: Rotation, proto: PartPrototype) {
        self.vehicle.add_part(proto, p, rot);
        self.update();
    }

    fn try_place_part(&mut self, p: IVec2, new_part: PartPrototype) -> Option<()> {
        let layer = new_part.layer();

        if !self.is_layer_visible(layer) {
            return None;
        }

        let new_pixels = occupied_pixels(p, self.rotation, &new_part);

        if let Some(occ) = self.occupied.get(&layer) {
            for p in &new_pixels {
                if occ.contains_key(p) {
                    return None;
                }
            }
        }

        self.vehicle.add_part(new_part.clone(), p, self.rotation);

        self.action_queue
            .push(Action::Add(p, self.rotation, new_part));

        self.update();
        Some(())
    }

    fn remove_part_at(&mut self, p: Vec2) {
        let pixel_p = vround(p * PIXELS_PER_METER);
        if let Ok(part) = self.vehicle.remove_part_at(pixel_p, self.focus_layer) {
            self.action_queue.push(Action::Remove(
                part.origin(),
                part.rotation(),
                part.prototype(),
            ));
        }
        self.update();
    }

    fn current_part_and_cursor_position(state: &GameState) -> Option<(IVec2, PartPrototype)> {
        let ctx = &state.editor_context;
        let part = state.editor_context.cursor_state.current_part()?;
        let wh = pixel_dims_with_rotation(ctx.rotation, &part).as_ivec2();
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = vround_f64(state.editor_context.c2w(pos) * PIXELS_PER_METER as f64);
        let pos = if let Some((snap_pos, dims)) = state.editor_context.snap_info {
            let dims = dims.as_ivec2();
            let delta = pos - snap_pos;
            let xi = if delta.x < 0 {
                delta.x / dims.x - 1
            } else {
                delta.x / dims.x
            };
            let yi = if delta.y < 0 {
                delta.y / dims.y - 1
            } else {
                delta.y / dims.y
            };
            snap_pos + IVec2::new(xi * dims.x, yi * dims.y)
        } else {
            pos - wh / 2
        };
        Some((pos, part))
    }
}

fn draw_highlight_box(canvas: &mut Canvas, aabb: AABB, ctx: &impl CameraProjection, color: Srgba) {
    let w1 = 2.0 / PIXELS_PER_METER;
    let w2 = 4.0 / PIXELS_PER_METER;

    let x1 = Vec2::X * w1;
    let x2 = Vec2::X * w2;

    let y1 = Vec2::Y * w1;
    let y2 = Vec2::Y * w2;

    let left = AABB::from_arbitrary(aabb.lower() - x1, aabb.top_left() - x2);
    let right = AABB::from_arbitrary(aabb.bottom_right() + x1, aabb.upper() + x2);

    let upper = AABB::from_arbitrary(aabb.top_left() + y1, aabb.upper() + y2);
    let lower = AABB::from_arbitrary(aabb.lower() - y1, aabb.bottom_right() - y2);

    for aabb in [upper, lower, left, right] {
        canvas.rect(ctx.w2c_aabb(aabb), color).z_index = 100.0;
    }
}

fn highlight_part(
    canvas: &mut Canvas,
    instance: &InstantiatedPart,
    ctx: &impl CameraProjection,
    color: Srgba,
) {
    let wh = instance.dims_meters();
    let p = instance.origin_meters();
    let q = p + wh;
    let r = p + Vec2::X * wh.x;
    let s = p + Vec2::Y * wh.y;
    let aabb = AABB::from_arbitrary(p, p + wh);

    draw_highlight_box(canvas, aabb, ctx, color);

    for p in [p, q, r, s] {
        let p = p.as_dvec2();
        let p = ctx.w2c(p);
        draw_cross(&mut canvas.gizmos, p, gcast(0.1 * ctx.scale()), color);
    }
}

pub fn draw_particles(
    canvas: &mut Canvas,
    ctx: &impl CameraProjection,
    particles: &ThrustParticleEffects,
) {
    for particle in &particles.particles {
        let p = ctx.w2c(particle.pv.pos);
        let age = particle.age.to_secs();
        let alpha = (1.0 - age / particle.lifetime.to_secs())
            .powi(3)
            .clamp(0.0, 1.0)
            * (particle.atmo * 0.8 + 0.2);
        let c1 = Srgba::from_f32_array(particle.initial_color);
        let c2 = Srgba::from_f32_array(particle.final_color);
        let color = c1.mix(&c2, age.clamp(0.0, 1.0).sqrt());
        let size = 1.0 + age * 12.0;
        let ramp_up = (age * 40.0).clamp(0.0, 1.0);
        let stretch = (8.0 * (1.0 - age * 2.0)).max(1.0);
        canvas
            .sprite(
                p,
                particle.angle,
                "cloud",
                None,
                Vec2::new(size * stretch * ramp_up, size * ramp_up) * gcast(ctx.scale()),
            )
            .set_color(color.with_alpha(color.alpha * alpha));
    }
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

        let other_buttons = other_buttons(state.settings.ui_button_height);
        // let actions = action_queue(&state.editor_context.action_queue);

        let part_buttons = if let Some(id) = state.editor_context.selected_part {
            if let Some(instance) = state.editor_context.vehicle.get_part(id) {
                Some(part_ui_layout(
                    state.settings.ui_button_height,
                    id,
                    instance,
                ))
            } else {
                None
            }
        } else {
            None
        };

        let right_column = Node::column(400)
            .invisible()
            .with_child(other_buttons)
            // .with_child(actions)
            .with_child(part_buttons);

        let main_area = Node::grow()
            .invisible()
            .with_child(parts)
            .with_child(
                Node::fit()
                    .down()
                    .with_padding(0.0)
                    .invisible()
                    .with_child(layers),
            )
            .with_child(vehicles)
            .with_child(Node::grow().invisible())
            .with_child(right_column);

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
        draw_cross(&mut canvas.gizmos, ctx.w2c(DVec2::ZERO), 10.0, GRAY);

        if let Some((pos, dims)) = ctx.snap_info {
            let lower = pos.as_vec2();
            let upper = lower + dims.as_vec2();
            let aabb = AABB::from_arbitrary(lower, upper);
            draw_aabb(&mut canvas.gizmos, ctx.w2c_aabb(aabb), GREEN);
        }

        draw_particles(canvas, ctx, &ctx.particles);

        match &ctx.cursor_state {
            CursorState::None | CursorState::Part(_) => {
                if let Some(p) = state.input.current() {
                    canvas.circle(p, 4.0, WHITE);
                }
            }
        }

        let radius = ctx.vehicle.bounding_radius();
        let bounds = ctx.vehicle.aabb();

        let filename = match &state.editor_context.filepath {
            Some(p) => format!("[{}]", p.display()),
            None => "[No file open]".to_string(),
        };

        let vehicle_info = vehicle_info(&ctx.vehicle);

        let info: String = [
            filename,
            format!("{} parts", state.editor_context.vehicle.parts().count()),
            format!("Rotation: {:?}", state.editor_context.rotation),
        ]
        .into_iter()
        .map(|s| format!("{s}\n"))
        .collect();

        let info = format!("{}{}", info, vehicle_info);

        let world_pos = Vec2::new(0.0, bounds.lower().y - 1.0).as_dvec2();
        canvas
            .text(info, ctx.w2c(world_pos), gcast(0.01 * ctx.scale()))
            .anchor_top_left();
        let world_pos = Vec2::new(0.0, bounds.upper().y + 1.0).as_dvec2();
        canvas
            .text(
                format!(
                    "{}-type vessel\n\"{}\"",
                    state.editor_context.vehicle.model(),
                    state.editor_context.vehicle.name()
                ),
                ctx.w2c(world_pos),
                gcast(0.01 * ctx.scale()),
            )
            .anchor_bottom_left();

        // axes
        {
            let length = bounds.span.x as f64 * 1.5;
            let width = bounds.span.y as f64 * 1.5;
            let o = ctx.w2c(DVec2::ZERO);
            let p = ctx.w2c(DVec2::X * length);
            let q = ctx.w2c(DVec2::Y * width);
            let np = ctx.w2c(-DVec2::X * length);
            let nq = ctx.w2c(-DVec2::Y * width);
            canvas.gizmos.line_2d(o, p, RED.with_alpha(0.3));
            canvas.gizmos.line_2d(o, q, GREEN.with_alpha(0.3));
            canvas.gizmos.line_2d(o, np, RED.with_alpha(0.1));
            canvas.gizmos.line_2d(o, nq, GREEN.with_alpha(0.1));
        }

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let current_pixels = occupied_pixels(p, ctx.rotation, &current_part);

            let mut visited_parts = HashSet::new();

            if let Some(occ) = ctx.occupied.get(&current_part.layer()) {
                for q in &current_pixels {
                    if let Some(idx) = occ.get(q) {
                        if visited_parts.contains(idx) {
                            continue;
                        }
                        visited_parts.insert(*idx);
                        if let Some(instance) = ctx.vehicle.get_part(*idx) {
                            highlight_part(canvas, instance, ctx, RED.with_alpha(0.6));
                        }
                    }
                }
            }
        }

        if ctx.show_vehicle_info {
            draw_aabb(
                &mut canvas.gizmos,
                ctx.w2c_aabb(bounds),
                TEAL.with_alpha(0.1),
            );

            draw_circle(
                &mut canvas.gizmos,
                ctx.w2c(DVec2::ZERO),
                gcast(radius * ctx.scale()),
                RED.with_alpha(0.1),
            );

            // COM
            let com = ctx.vehicle.center_of_mass();
            draw_circle(&mut canvas.gizmos, ctx.w2c(com), 7.0, ORANGE);
            draw_x(&mut canvas.gizmos, ctx.w2c(com), 7.0, WHITE);

            // thrust envelope
            for (rcs, color) in [(false, RED), (true, BLUE)] {
                let positions: Vec<_> = linspace_f64(0.0, 2.0 * PI_64, 200)
                    .into_iter()
                    .map(|a| {
                        let thrust = ctx.vehicle.current_thrust_along_heading(a, rcs);
                        let r = (1.0 + thrust.abs().sqrt() / 100.0) * ctx.vehicle.bounding_radius();
                        ctx.w2c(rotate_f64(DVec2::X * r, a))
                    })
                    .collect();
                canvas.gizmos.linestrip_2d(positions, color.with_alpha(0.6));
            }
        }

        for (_, part) in ctx.vehicle.parts() {
            if let Some((t, _)) = part.as_thruster() {
                let u = rotate_f64(DVec2::X, part.rotation().to_angle());
                let thrust_vector = u * (t.max_thrust() / 1000.0).sqrt();
                let start = part.origin().as_dvec2() + part.dims_grid().as_dvec2() / 2.0;
                let end = start + thrust_vector;
                let start = ctx.w2c(start);
                let end = ctx.w2c(end);
                canvas.gizmos.line_2d(start, end, RED);
            }
        }

        for layer in PartLayer::draw_order() {
            if layer == PartLayer::Plumbing
                && (ctx.focus_layer == Some(PartLayer::Internal)
                    || ctx.focus_layer == Some(PartLayer::Plumbing)
                    || ctx.focus_layer.is_none())
            {
                // draw the pipes themselves
                let is_focus = ctx.focus_layer == Some(PartLayer::Plumbing);
                for pipe in ctx.vehicle.pipes() {
                    let color = if is_focus { PURPLE } else { DARK_SLATE_GRAY };
                    let p = pipe.as_vec2() / PIXELS_PER_METER;
                    let q = (pipe + IVec2::ONE).as_vec2() / PIXELS_PER_METER;
                    let aabb = AABB::from_arbitrary(p, q).scale_about_center(1.2);
                    canvas.rect(ctx.w2c_aabb(aabb), color);
                }

                for group in ctx.vehicle.conn_groups() {
                    for joint in group.points() {
                        let p = joint.as_vec2() / PIXELS_PER_METER;
                        let q = (joint + IVec2::ONE).as_vec2() / PIXELS_PER_METER;
                        let aabb = AABB::from_arbitrary(p, q).scale_about_center(1.5);
                        canvas.rect(ctx.w2c_aabb(aabb), ORANGE);
                    }
                }

                // highlight parts in this connectivity group
                if is_focus {
                    for (group_id, group) in ctx.vehicle.conn_groups().enumerate() {
                        let color = crate::sprites::hashable_to_color(&group_id);
                        let color: Srgba = color.into();
                        for id in group.ids() {
                            if let Some(part) = ctx.vehicle.get_part(id) {
                                highlight_part(canvas, part, ctx, color.with_alpha(0.02));
                            }
                        }
                    }
                }
                continue;
            }

            for (_, instance) in ctx
                .vehicle
                .parts()
                .filter(|(_, p)| p.prototype().layer() == layer)
            {
                let detailed_part_info =
                    ctx.focus_layer == Some(PartLayer::Internal) && ctx.show_vehicle_info;

                let alpha = match (ctx.focus_layer, layer) {
                    (None, _) => 1.0,
                    (_, PartLayer::Plumbing) => continue,
                    (Some(PartLayer::Internal), PartLayer::Internal) => 1.0,
                    (Some(PartLayer::Internal), _) => 0.02,
                    (Some(PartLayer::Plumbing), PartLayer::Internal) => 0.7,
                    (Some(PartLayer::Plumbing), _) => 0.02,
                    (Some(PartLayer::Structural), PartLayer::Structural) => 1.0,
                    (Some(PartLayer::Structural), _) => 0.02,
                    (Some(PartLayer::Exterior), PartLayer::Exterior) => 1.0,
                    (Some(PartLayer::Exterior), _) => 0.02,
                };

                let dims = instance.dims_meters().as_dvec2();
                let sprite_dims = instance.prototype().dims_meters();
                let center = ctx.w2c(instance.origin_meters().as_dvec2() + dims / 2.0);
                let p = instance.percent_built();
                let sprite_name = instance.prototype().sprite_path().to_string();
                let sprite_name = if p == 1.0 {
                    sprite_name.to_string()
                } else {
                    let idx = (p * 10.0).floor() as i32;
                    format!("{}-building-{}", sprite_name, idx)
                };

                canvas
                    .sprite(
                        center,
                        gcast(instance.rotation().to_angle()),
                        sprite_name,
                        None,
                        graphics_cast(sprite_dims.as_dvec2() * ctx.scale()),
                    )
                    .set_color(WHITE.with_alpha(alpha));

                if detailed_part_info {
                    if let Some((t, d)) = instance.as_tank() {
                        let pct = t.percent_filled(d);
                        let dims = rotate(
                            (sprite_dims - Vec2::splat(2.0)) * gcast(ctx.scale()),
                            gcast(instance.rotation().to_angle()),
                        )
                        .abs();
                        let lower = center - dims / 2.0;
                        let upper = lower + Vec2::new(dims.x, dims.y * gcast(pct));
                        let aabb = AABB::from_arbitrary(lower, upper);
                        let color: Srgba = crate::sprites::hashable_to_color(&d.item()).into();
                        canvas.rect(aabb, color.with_alpha(0.7));

                        if let Some(item) = d.item() {
                            let s = aabb.span.x.min(aabb.span.y) * 0.7;
                            let path = item.to_sprite_name();
                            canvas
                                .sprite(aabb.center, 0.0, "cloud", None, Vec2::splat(s))
                                .set_color(BLACK);
                            canvas.sprite(aabb.center, 0.0, path, None, Vec2::splat(s));
                        }
                    }

                    if let Some((_, d)) = instance.as_machine() {
                        let pct = d.percent_complete();
                        let dims = rotate(
                            (sprite_dims - Vec2::splat(2.0)) * gcast(ctx.scale()),
                            gcast(instance.rotation().to_angle()),
                        )
                        .abs();
                        let lower = center - dims / 2.0;
                        let upper = lower + Vec2::new(dims.x * pct, 2.0 * gcast(ctx.scale()));
                        let aabb = AABB::from_arbitrary(lower, upper);
                        canvas.rect(aabb, RED.with_alpha(0.7));
                    }

                    if let Some((c, d)) = instance.as_cargo() {
                        let dims = rotate(
                            (sprite_dims - Vec2::splat(2.0)) * gcast(ctx.scale()),
                            gcast(instance.rotation().to_angle()),
                        )
                        .abs();

                        let mut lower = center - dims / 2.0;

                        for (item, mass) in d.contents() {
                            let pct = mass.to_kg_f64() / c.capacity_mass().to_kg_f64();
                            let upper = lower + Vec2::new(dims.x, dims.y * gcast(pct));
                            let aabb = AABB::from_arbitrary(lower, upper);
                            let color = crate::sprites::hashable_to_color(&item);
                            canvas.rect(aabb, color.with_alpha(0.4));

                            let s = aabb.span.x.min(aabb.span.y) * 0.7;
                            let path = item.to_sprite_name();
                            canvas
                                .sprite(aabb.center, 0.0, "cloud", None, Vec2::splat(s))
                                .set_color(BLACK);

                            canvas.sprite(aabb.center, 0.0, path, None, Vec2::splat(s));

                            lower.y += dims.y * gcast(pct);
                        }
                    }
                }
            }
        }

        if let Some(cursor) = state.input.position(MouseButt::Hover, FrameId::Current) {
            let c = ctx.c2w(cursor);

            // let discrete = vround(c);

            // for dx in -20..=20 {
            //     for dy in -20..=20 {
            //         let s = IVec2::new(dx, dy);
            //         let p = discrete - s;
            //         let d = (s.length_squared() as f32).sqrt();
            //         let alpha = 0.2 * (1.0 - d / 100.0);
            //         if alpha > 0.01 {
            //             draw_diamond(
            //                 &mut canvas.gizmos,
            //                 ctx.w2c(p.as_vec2()),
            //                 7.0,
            //                 GRAY.with_alpha(alpha),
            //             );
            //         }
            //     }
            // }

            if Self::current_part_and_cursor_position(state).is_none() {
                if let Some((id, _)) = ctx.get_part_at(graphics_cast(c)) {
                    if let Some(instance) = ctx.vehicle.get_part(id) {
                        highlight_part(canvas, instance, ctx, TEAL.with_alpha(0.6));
                        for (other, other_instance) in ctx.vehicle.parts() {
                            if ctx.vehicle.is_connected(id, *other) {
                                highlight_part(canvas, other_instance, ctx, YELLOW.with_alpha(0.4))
                            }
                        }
                    }
                }
            }
        }

        if let Some(instance) = ctx.selected_part() {
            highlight_part(canvas, instance, ctx, GREEN.with_alpha(0.4));
            canvas.text(format!("{:#?}", instance), Vec2::new(300.0, 400.0), 0.6);
        }

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let dims = pixel_dims_with_rotation(ctx.rotation, &current_part);
            let sprite_dims = current_part.dims();
            canvas.sprite(
                ctx.w2c((p.as_dvec2() + dims.as_dvec2() / 2.0) / PIXELS_PER_METER as f64),
                gcast(ctx.rotation.to_angle()),
                current_part.sprite_path().to_string(),
                None,
                sprite_dims.as_vec2() / PIXELS_PER_METER * gcast(ctx.scale()),
            );
        }

        for particle in &ctx.build_particles {
            let p = ctx.w2c(particle.pos());
            canvas
                .sprite(
                    p,
                    0.0,
                    "error",
                    None,
                    Vec2::splat(0.03) * gcast(ctx.scale()),
                )
                .set_color(YELLOW.with_alpha(particle.opacity()));
        }

        for bot in &ctx.bots {
            canvas.sprite(
                ctx.w2c(bot.pos()),
                gcast(bot.angle()),
                "conbot",
                None,
                Vec2::splat(0.3) * gcast(ctx.scale()),
            );
            if let Some(t) = bot.target_pos() {
                canvas.circle(ctx.w2c(t), 12.0, PURPLE.with_alpha(0.2));
            }
        }

        Some(())
    }
}

fn expandable_menu(button_height: f32, text: &str, onclick: OnClick) -> Node<OnClick> {
    Node::structural(300, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(Node::button(text, onclick, Size::Grow, button_height))
}

fn part_selection(state: &GameState) -> Node<OnClick> {
    let mut part_names: Vec<_> = state.part_database.keys().collect();
    part_names.sort();

    let mut n = expandable_menu(
        state.settings.ui_button_height,
        "Parts",
        OnClick::TogglePartsMenuCollapsed,
    );

    if !state.editor_context.parts_menu_collapsed {
        n.add_child(Node::hline());
        n.add_children(part_names.into_iter().map(|s| {
            let onclick = OnClick::SelectPart(s.clone());
            Node::button(s, onclick, Size::Grow, state.settings.ui_button_height)
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

    let mut n = expandable_menu(
        state.settings.ui_button_height,
        "Vehicles",
        OnClick::ToggleVehiclesMenuCollapsed,
    );

    if !state.editor_context.vehicles_menu_collapsed {
        n.add_child(Node::hline());
        n.add_children(vehicles.into_iter().map(|(name, path)| {
            let onclick = OnClick::LoadVehicle(path);
            Node::button(name, onclick, Size::Grow, state.settings.ui_button_height)
        }));
    }

    n
}

#[allow(unused)]
fn action_queue(button_height: f32, queue: &Vec<Action>) -> Node<OnClick> {
    Node::structural(Size::Grow, Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .down()
        .with_children(
            queue
                .iter()
                .map(|a| Node::text(Size::Grow, button_height, format!("{}", a.to_string()))),
        )
}

fn other_buttons(button_height: f32) -> Node<OnClick> {
    let rotate = Node::button("Rotate", OnClick::RotateCraft, Size::Grow, button_height);

    let normalize = Node::button(
        "Normalize",
        OnClick::NormalizeCraft,
        Size::Grow,
        button_height,
    );

    let new_button = Node::button("New", OnClick::OpenNewCraft, Size::Grow, button_height);

    let toggle_info = Node::button(
        "Info",
        OnClick::ToggleVehicleInfo,
        Size::Grow,
        button_height,
    );

    let send_to_surface = Node::button(
        "Send to Surface",
        OnClick::SendToSurface,
        Size::Grow,
        button_height,
    );

    Node::structural(Size::Grow, Size::Fit)
        .with_color(UI_BACKGROUND_COLOR)
        .down()
        .with_child(new_button)
        .with_child(Node::hline())
        .with_child(rotate)
        .with_child(normalize)
        .with_child(Node::hline())
        .with_child(toggle_info)
        .with_child(send_to_surface)
}

fn layer_selection(state: &GameState) -> Node<OnClick> {
    let mut n = expandable_menu(
        state.settings.ui_button_height,
        "Layers",
        OnClick::ToggleLayersMenuCollapsed,
    );

    if !state.editor_context.layers_menu_collapsed {
        n.add_child(Node::hline());
        n.add_children(enum_iterator::all::<PartLayer>().into_iter().map(|p| {
            let s = format!("{:?}", p);
            let onclick = OnClick::ToggleLayer(p);
            let mut n = Node::button(s, onclick, Size::Grow, state.settings.ui_button_height);
            if !state.editor_context.is_layer_visible(p) {
                n = n.with_color(GRAY.to_f32_array());
            }
            n
        }));
    }

    n
}

impl CameraProjection for EditorContext {
    fn origin(&self) -> DVec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f64 {
        self.camera.scale()
    }
}

impl EditorContext {
    pub fn on_render_tick(state: &mut GameState) {
        state.editor_context.camera.handle_input(&state.input);

        if state.is_hovering_over_ui() {
            return;
        }

        if state.input.is_pressed(KeyCode::KeyB) {
            for _ in 0..100 {
                state.editor_context.vehicle.build_once();
            }
        }

        if let Some(p) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
            let p = state.editor_context.c2w(p);
            if let Some((id, _)) = state.editor_context.get_part_at(graphics_cast(p)) {
                state.editor_context.selected_part = Some(id)
            } else {
                state.editor_context.selected_part = None;
            }
        }

        if state.input.is_pressed(KeyCode::ShiftLeft) {
            if let Some((pos, proto)) = EditorContext::current_part_and_cursor_position(state) {
                if state.editor_context.snap_info.is_none() {
                    let rot = state.editor_context.rotation;
                    let dims = pixel_dims_with_rotation(rot, &proto);
                    state.editor_context.snap_info = Some((pos, dims));
                }
            } else {
                state.editor_context.snap_info = None;
            }
        } else {
            state.editor_context.snap_info = None;
        }

        if let Some(_) = state.input.position(MouseButt::Left, FrameId::Current) {
            if let Some((p, part)) = EditorContext::current_part_and_cursor_position(state) {
                state.editor_context.try_place_part(p, part);
            }
        } else if let Some(p) = state.input.on_frame(MouseButt::Right, FrameId::Down) {
            state
                .editor_context
                .remove_part_at(graphics_cast(state.editor_context.c2w(p)));
        } else if state.input.just_pressed(KeyCode::KeyQ) {
            if state.editor_context.cursor_state.current_part().is_some() {
                state.editor_context.cursor_state = CursorState::None;
            } else if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
                if let Some((_, instance)) = state
                    .editor_context
                    .get_part_at(graphics_cast(state.editor_context.c2w(p)))
                {
                    let instance = instance.clone();
                    state.editor_context.rotation = instance.rotation();
                    state.editor_context.cursor_state =
                        CursorState::Part(instance.prototype().clone());
                } else {
                    state.editor_context.cursor_state = CursorState::None;
                }
            }
        }

        if state.input.just_pressed(KeyCode::KeyR) {
            state.editor_context.rotation =
                enum_iterator::next_cycle(&state.editor_context.rotation);
        }

        if state.editor_context.focus_layer == Some(PartLayer::Plumbing) {
            if let Some(p) = state.input.position(MouseButt::Left, FrameId::Current) {
                let p = vfloor(graphics_cast(state.editor_context.c2w(p)) * PIXELS_PER_METER);
                state.editor_context.vehicle.add_pipe(p);
            }
            if let Some(p) = state.input.position(MouseButt::Right, FrameId::Current) {
                let p = vfloor(graphics_cast(state.editor_context.c2w(p)) * PIXELS_PER_METER);
                state.editor_context.vehicle.remove_pipe(p);
            }
        }

        if state.input.is_pressed(KeyCode::ControlLeft) && state.input.just_pressed(KeyCode::KeyZ) {
            state.editor_context.undo();
        }

        if state.input.just_pressed(KeyCode::KeyO) {
            state.editor_context.atmo += 1;
        }

        if state.input.just_pressed(KeyCode::KeyL) {
            state.editor_context.atmo -= 1;
        }

        state.editor_context.atmo = state.editor_context.atmo.clamp(0, 10);
    }

    pub fn on_game_tick(state: &mut GameState) {
        state.editor_context.camera.on_game_tick();

        let ctx = &mut state.editor_context;

        let all_parts: HashSet<_> = ctx
            .vehicle
            .parts()
            .filter_map(|(id, p)| (p.percent_built() < 1.0).then(|| *id))
            .collect();

        let assigned_parts: HashSet<_> = ctx.bots.iter().filter_map(|b| b.target_part()).collect();

        let mut unbuilt_parts: Vec<_> = ctx
            .vehicle
            .parts()
            .filter_map(|(id, p)| {
                (p.percent_built() < 1.0 && !assigned_parts.contains(id)).then(|| {
                    let origin = p.origin_meters();
                    let dims = p.dims_meters();
                    (*id, AABB::from_arbitrary(origin, origin + dims))
                })
            })
            .collect();

        for bot in &mut ctx.bots {
            if let Some(id) = bot.target_part() {
                if !all_parts.contains(&id) {
                    bot.clear_target_part();
                    bot.set_target_pos(randvec(50.0, 60.0).as_dvec2())
                }
            }

            if bot.target_part().is_none() {
                if let Some(pos) = bot.target_pos() {
                    if pos.length() < 10.0 {
                        bot.set_target_pos(randvec(50.0, 60.0).as_dvec2())
                    }
                }
            }

            if bot.target_part().is_none() && !unbuilt_parts.is_empty() {
                let n = randint(0, unbuilt_parts.len() as i32);
                let (id, bounds) = unbuilt_parts[n as usize];
                unbuilt_parts.remove(n as usize);
                bot.set_target_part(id);
                let pos = bounds.uniform_sample().as_dvec2();
                bot.set_target_pos(pos);
            }

            bot.on_sim_tick();
        }

        for bot in &ctx.bots {
            let tpos = match bot.target_pos() {
                Some(pos) => pos,
                None => continue,
            };

            if bot.pos().distance(tpos) > 1.0 {
                continue;
            }

            if let Some(id) = bot.target_part() {
                for _ in 0..10 {
                    let particle = BuildParticle::new(bot.pos());
                    ctx.build_particles.push(particle);
                    ctx.vehicle.build_part(id);
                }
            }
        }

        ctx.vehicle.on_sim_tick();

        ctx.vehicle.set_all_thrusters(1.0);

        for particle in &mut ctx.build_particles {
            particle.on_sim_tick();
        }

        let atmo = ctx.atmo as f32 / 10.0;

        add_particles_from_vehicle(&mut ctx.particles, &ctx.vehicle, &RigidBody::ZERO, atmo);
        ctx.particles.step();

        ctx.build_particles.retain(|p| p.opacity() > 0.0);
    }
}

pub fn write_image_to_file(vehicle: &Vehicle, ctx: &ProgramContext, name: &str) -> Option<()> {
    let outpath: String = format!("/tmp/{}.png", name);
    println!(
        "Writing vehicle {} to path {}",
        vehicle.discriminator(),
        outpath
    );
    let img = generate_image(vehicle, &ctx.parts_dir(), false)?;
    img.save(outpath).ok()
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

        let vehicles = get_list_of_vehicles(&g).expect("Expected list of vehicles");
        dbg!(vehicles);

        for name in ["remora", "lander", "pollux", "manta", "spacestation"] {
            let vehicle = g.get_vehicle_by_model(name).expect("Expected a vehicle");
            write_image_to_file(&vehicle, &args, name);
        }
    }
}
