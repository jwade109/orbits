use crate::camera_controller::*;
use crate::canvas::*;
use crate::craft_editor::*;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::InputState;
use crate::input::{FrameId, MouseButt};
use crate::z_index::ZOrdering;
use bary_core::prelude::GridPlacement;
use bary_core::prelude::*;
use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;
use bevy::math::DVec2;
use bevy::prelude::*;
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum Action {
    Add(PartId),
    Remove(PartCoord, Rotation, PartPrototype),
}

pub fn pixel_dims_with_rotation(rot: Rotation, part: &PartPrototype) -> UVec2 {
    let dims = part.dims();
    match rot {
        Rotation::East | Rotation::West => UVec2::new(dims.x, dims.y),
        Rotation::North | Rotation::South => UVec2::new(dims.y, dims.x),
    }
}

impl Action {
    pub fn to_string(&self) -> String {
        match self {
            Self::Add(id) => format!("Add #{:?}", id),
            Self::Remove(_, _, proto) => format!("Remove {}", proto.part_name()),
        }
    }
}

#[derive(Debug)]
pub struct Editor {
    pub camera: LinearCameraController,
    pub cursor_state: CursorState,
    pub rotation: Rotation,
    pub filepath: Option<PathBuf>,
    pub focus_layer: Option<PartLayer>,
    pub selected_parts: HashSet<PartId>,
    pub snap_info: Option<(PartCoord, UVec2)>,
    pub action_queue: Vec<Action>,
    pub occupied: HashMap<PartLayer, HashMap<PartCoord, PartId>>,
    pub blueprint: Blueprint,
    pub graph: InventoryGraph,

    // menus
    pub show_vehicle_info: bool,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            camera: LinearCameraController::new(DVec2::ZERO, 0.01),
            cursor_state: CursorState::None,
            rotation: Rotation::East,
            filepath: None,
            focus_layer: None,
            selected_parts: HashSet::new(),
            snap_info: None,
            action_queue: Vec::new(),
            occupied: HashMap::new(),
            blueprint: Blueprint::new(),
            graph: InventoryGraph::default(),
            show_vehicle_info: false,
        }
    }

    pub fn remove_part(&mut self, id: PartId) {
        self.blueprint.remove_part(id);
    }

    pub fn undo(&mut self) -> Option<()> {
        let action = self.action_queue.pop()?;
        match action {
            // Action::Add(id) => match self.blueprint.remove_part(id) {
            //     Some(p) => println!("Removed {:?}", p),
            //     None => println!("Failed to remove"),
            // },
            Action::Remove(pos, rot, proto) => self.add_part(pos, rot, proto),
            _ => println!("oh no!"),
        }
        Some(())
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
        self.blueprint = Blueprint::new();
        self.cursor_state = CursorState::None;
        self.update();
    }

    pub fn write_image_to_file(&self, parts: &PartDatabase) {
        write_image_to_file(&self.blueprint, parts, "vehicle");
    }

    pub fn rotate_craft(&mut self) {
        self.blueprint.rotate_ccw();
        self.update();
    }

    pub fn enter_pipe_mode(&mut self) {
        self.cursor_state = CursorState::Pipe(CursorPipeData::default());
    }

    pub fn enter_select_mode(&mut self) {
        self.cursor_state = CursorState::Select(SelectedState::default());
    }

    pub fn update_graph(&mut self) {
        let graph = InventoryGraph::from_blueprint(&self.blueprint);
        self.graph.update(graph);
    }

    pub fn randomize_graph(&mut self) {
        self.graph.randomize_positions();
    }

    pub fn set_current_part(state: &mut GameState, name: String) {
        state.editor_context.cursor_state = CursorState::Part(name);
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

    pub fn on_right_click_down(state: &mut GameState, p: Vec2) {
        info!("on_right_click_down");
        state
            .editor_context
            .remove_part_at(graphics_cast(state.editor_context.c2w(p)));
    }

    pub fn on_ctrl_c(state: &mut GameState) {
        info!("on_ctrl_c");

        if state.editor_context.selected_parts.is_empty() {
            return;
        }

        let mut blueprint = Blueprint::new();
        for id in &state.editor_context.selected_parts {
            if let Some(part) = state.editor_context.blueprint.get_part(*id) {
                blueprint.add_part(part.name.clone(), part.placement, part.layer());
            }
            if let Some(pipe) = state.editor_context.blueprint.get_pipe(*id) {
                blueprint.add_pipe(*pipe);
            }
        }

        blueprint.normalize_coordinates();

        state.editor_context.cursor_state = CursorState::Blueprint(blueprint);
    }

    pub fn on_left_click_down(state: &mut GameState, p: Vec2, is_shift: bool) {
        info!("on_left_click_down");
        let p = state.editor_context.c2w(p);

        if let Some((id, _)) = state.editor_context.get_part_at(graphics_cast(p)) {
            if is_shift {
                state.editor_context.selected_parts.insert(id);
            } else {
                state.editor_context.selected_parts.clear();
                state.editor_context.selected_parts.insert(id);
            }
        } else {
            state.editor_context.selected_parts.clear();
        }

        if let Some(data) = state.editor_context.cursor_state.sel_mut() {
            data.update_start(p);
        }

        if let Some(c) = Editor::current_cursor_coord(state) {
            if let Some(data) = state.editor_context.cursor_state.pipe_mut() {
                data.start_position = Some(c);
            }
        }
    }

    pub fn on_left_click_held(state: &mut GameState) {
        if let Some(c) = Editor::current_cursor_coord(state) {
            if let Some(data) = state.editor_context.cursor_state.pipe_mut() {
                data.end_position = Some(c);
            }
            if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
                let p = state.editor_context.c2w(p);
                if let Some(data) = state.editor_context.cursor_state.sel_mut() {
                    data.update_end(p);
                }
            }

            if let Some(data) = state.editor_context.cursor_state.selected() {
                let mut ids = HashSet::new();
                for c in data.cells() {
                    for layer in PartLayer::all() {
                        if let Some(id) = state.editor_context.blueprint.get_part_at_layer(c, layer)
                        {
                            ids.insert(id);
                        }
                    }
                }
                state.editor_context.selected_parts = ids;
            }
        }
    }

    pub fn on_delete(state: &mut GameState) {
        for id in &state.editor_context.selected_parts {
            state.editor_context.blueprint.remove_part(*id);
        }
        state.editor_context.selected_parts.clear();
        state.editor_context.update_graph();
    }

    pub fn on_left_click_release(state: &mut GameState) {
        info!("on_left_click_release");
        if let Some(data) = state.editor_context.cursor_state.pipe().cloned() {
            if let Some(pipe) = data.pipe_geometry() {
                state.editor_context.blueprint.add_pipe(pipe);
                state.editor_context.update_graph();
            }
        }
        if let Some(data) = state.editor_context.cursor_state.pipe_mut() {
            data.start_position = None;
            data.end_position = None;
        }
        if let Some(sel) = state.editor_context.cursor_state.sel_mut() {
            sel.update_start(None);
            sel.update_end(None);
        }
    }

    pub fn process_holding_shift(state: &mut GameState) {
        if state.input.is_pressed(KeyCode::ShiftLeft) {
            if let Some((pos, proto)) = Editor::current_part_and_cursor_position(state) {
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
    }

    pub fn on_press_q(state: &mut GameState) {
        if state.editor_context.cursor_state.current_part().is_some() {
            state.editor_context.cursor_state = CursorState::None;
        } else if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
            if let Some((_, instance)) = state
                .editor_context
                .get_part_at(graphics_cast(state.editor_context.c2w(p)))
            {
                let instance = instance.clone();
                state.editor_context.rotation = instance.rotation();
                state.editor_context.cursor_state = CursorState::Part(instance.name);
            } else {
                state.editor_context.cursor_state = CursorState::None;
            }
        }
    }

    pub fn on_press_r(state: &mut GameState) {
        if let Some(bp) = state.editor_context.cursor_state.blueprint_mut() {
            bp.rotate_ccw();
        } else if let Some(data) = state.editor_context.cursor_state.pipe_mut() {
            data.x_first = !data.x_first;
        } else {
            state.editor_context.rotation =
                enum_iterator::next_cycle(&state.editor_context.rotation);
        }
    }

    pub fn save_to_file(state: &mut GameState) -> Option<()> {
        let choice: PathBuf = state.editor_context.open_file_to_save()?;
        state.notice(format!("Saving to {}", choice.display()));

        let parts = state
            .editor_context
            .blueprint
            .parts()
            .map(|(_, instance)| VehiclePartFileStorage {
                partname: instance.name.clone(),
                pos: instance.origin(),
                rot: instance.rotation(),
            })
            .collect();

        let pipes = state
            .editor_context
            .blueprint
            .pipes()
            .map(|(_, pipe)| PipeFileStorage { geometry: *pipe })
            .collect();

        let storage = VehicleFileStorage {
            parts,
            pipes: Some(pipes),
        };

        let s = serde_yaml::to_string(&storage).ok()?;
        std::fs::write(choice, s).ok()
    }

    pub fn load_from_file(state: &mut GameState) -> Option<()> {
        let choice = state.editor_context.open_existing_file()?;
        Editor::load_vehicle(&choice, state)
    }

    pub fn load_vehicle(path: &Path, state: &mut GameState) -> Option<()> {
        let vehicle = match load_vehicle(path, &state.part_database) {
            Ok(v) => v,
            Err(e) => {
                state.notice(format!("Failed to load vehicle: {}", e));
                return None;
            }
        };

        state.editor_context.blueprint = vehicle.clone();
        state.editor_context.filepath = Some(path.to_path_buf());
        state.editor_context.update();
        state.editor_context.action_queue.clear();
        state.editor_context.cursor_state = CursorState::None;
        state.editor_context.update_graph();
        Some(())
    }

    pub fn get_part_at(&self, p: Vec2) -> Option<(PartId, &PartInstance)> {
        let pixel_p = PartCoord::from_meters_floored(p);

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
                    return Some((*idx, self.blueprint.get_part(*idx)?));
                }
            }
        }

        None
    }

    fn update(&mut self) {
        self.update_graph();
        self.occupied.clear();
        for (id, instance) in self.blueprint.parts() {
            let pixels = instance.placement.cells();
            if let Some(occ) = self.occupied.get_mut(&instance.layer()) {
                for p in pixels {
                    occ.insert(p, *id);
                }
            } else {
                let mut occ = HashMap::new();
                for p in pixels {
                    occ.insert(p, *id);
                }
                self.occupied.insert(instance.layer(), occ);
            }
        }
    }

    fn add_part(&mut self, p: PartCoord, rot: Rotation, proto: PartPrototype) {
        let placement = GridPlacement::new(p, rot, proto.dims);
        self.blueprint
            .add_part(proto.name.clone(), placement, proto.layer());
        self.update();
    }

    pub fn try_place_part(
        &mut self,
        p: PartCoord,
        new_part: PartPrototype,
        rot: Rotation,
    ) -> Option<()> {
        let layer = new_part.layer();

        if !self.is_layer_visible(layer) {
            return None;
        }

        let gp = GridPlacement::new(p, rot, new_part.dims);

        let new_pixels = gp.cells();

        if let Some(occ) = self.occupied.get(&layer) {
            for p in new_pixels {
                if occ.contains_key(&p) {
                    return None;
                }
            }
        }

        let id = self.blueprint.add_part(new_part.name, gp, layer);

        self.action_queue.push(Action::Add(id));

        self.update();

        Some(())
    }

    pub fn remove_part_at(&mut self, p: Vec2) {
        let pixel_p = PartCoord::from_meters_floored(p);
        if self.blueprint.remove_part_at(pixel_p, self.focus_layer) {
            println!("TODO restore undo behavior!");
            // self.action_queue.push(Action::Remove(
            //     part.origin(),
            //     part.rotation(),
            //     part.proto,
            // ));
        }
        self.update();
    }

    pub fn current_cursor_coord(state: &GameState) -> Option<PartCoord> {
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = PartCoord::from_meters_floored(state.editor_context.c2w(pos));
        Some(pos)
    }

    pub fn current_part_and_cursor_position(
        state: &GameState,
    ) -> Option<(PartCoord, PartPrototype)> {
        let ctx = &state.editor_context;
        let part = state.editor_context.cursor_state.current_part()?;
        let part = state.part_database.get(part)?;
        let wh = pixel_dims_with_rotation(ctx.rotation, part).as_ivec2();
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = PartCoord::from_meters_floored(state.editor_context.c2w(pos));
        let pos = if let Some((snap_pos, dims)) = state.editor_context.snap_info {
            let dims = dims.as_ivec2();
            let delta = pos - snap_pos;
            let inner = delta.inner();
            let xi = if inner.x < 0 {
                inner.x / dims.x - 1
            } else {
                inner.x / dims.x
            };
            let yi = if inner.y < 0 {
                inner.y / dims.y - 1
            } else {
                inner.y / dims.y
            };
            snap_pos + PartCoord::new(IVec2::new(xi * dims.x, yi * dims.y))
        } else {
            pos - PartCoord::new(wh / 2)
        };
        Some((pos, part.clone()))
    }
}

fn draw_highlight_box(
    canvas: &mut Canvas,
    aabb: AABB,
    ctx: &impl CameraProjection,
    color: Srgba,
    z: ZOrdering,
) {
    canvas.hollow_rect(ctx.w2c_aabb(aabb), z, color, gcast(0.08 * ctx.scale()));
}

fn highlight_part(
    canvas: &mut Canvas,
    instance: &PartInstance,
    ctx: &impl CameraProjection,
    color: Srgba,
    z: ZOrdering,
) {
    let wh = instance.dims_meters();
    let p = instance.origin_meters();
    let aabb = AABB::from_arbitrary(p, p + wh);

    draw_highlight_box(canvas, aabb, ctx, color, z);
}

fn draw_blueprint(
    canvas: &mut Canvas,
    offset: PartCoord,
    blueprint: &Blueprint,
    ctx: &impl CameraProjection,
    focus_layer: Option<PartLayer>,
) {
    let offset = offset.to_meters();

    // axes
    {
        canvas.line(
            ctx.w2c(offset.as_dvec2()),
            ctx.w2c((offset + Vec2::X * 5.0).as_dvec2()),
            ZOrdering::Debug,
            RED,
        );
        canvas.line(
            ctx.w2c(offset.as_dvec2()),
            ctx.w2c((offset + Vec2::Y * 5.0).as_dvec2()),
            ZOrdering::Debug,
            GREEN,
        );
    }

    for layer in PartLayer::draw_order() {
        for (_, instance) in blueprint.parts().filter(|(_, p)| p.layer() == layer) {
            let alpha = match (focus_layer, layer) {
                (None, _) => 1.0,
                (Some(PartLayer::Internal), PartLayer::Internal) => 1.0,
                (Some(PartLayer::Internal), _) => 0.02,
                (Some(PartLayer::Structural), PartLayer::Structural) => 1.0,
                (Some(PartLayer::Structural), _) => 0.02,
                (Some(PartLayer::Exterior), PartLayer::Exterior) => 1.0,
                (Some(PartLayer::Exterior), _) => 0.02,
                _ => continue,
            };

            let sprite_dims = instance.placement.part_aligned_dims().to_meters();
            let center = instance.center_meters().as_dvec2();
            let sprite_name = instance.name.clone();

            let z_index = match layer {
                PartLayer::Exterior => ZOrdering::EditorExteriorPart,
                PartLayer::Internal => ZOrdering::EditorInteriorPart,
                PartLayer::Structural => ZOrdering::EditorStructuralPart,
                _ => continue,
            };

            canvas
                .sprite(
                    ctx.w2c(offset.as_dvec2() + center),
                    gcast(instance.rotation().to_angle()),
                    sprite_name,
                    z_index,
                    graphics_cast(sprite_dims.as_dvec2() * ctx.scale()),
                )
                .set_color(WHITE.with_alpha(alpha));
        }
    }

    for (_, pipe) in blueprint.pipes() {
        draw_pipe(canvas, pipe, offset, ctx, GRAY_400, GRAY_600);
    }
}

fn draw_cell(canvas: &mut Canvas, c: PartCoord, ctx: &impl CameraProjection, color: Srgba) {
    let lower = ctx.w2c(c.to_meters().as_dvec2());
    let upper = ctx.w2c((c + PartCoord::new(IVec2::ONE)).to_meters().as_dvec2());
    canvas.rect(AABB::from_arbitrary(lower, upper), ZOrdering::Debug2, color);
}

fn draw_pipe(
    canvas: &mut Canvas,
    pipe: &PipeGeometry,
    offset: Vec2,
    ctx: &impl CameraProjection,
    pipe_color: Srgba,
    node_color: Srgba,
) {
    let pipe_thickness = PartCoord::CELL_WIDTH * 0.2 * ctx.scale() as f32;
    let node_radius = PartCoord::CELL_WIDTH * 0.2 * ctx.scale() as f32;
    let node_z = ZOrdering::EditorPipeJoint;
    let pipe_z = ZOrdering::EditorPipe;
    match pipe.segments() {
        PipeSegments::Single(a, b) => {
            let a = ctx.w2c(offset.as_dvec2() + a.to_meters_center().as_dvec2());
            let b = ctx.w2c(offset.as_dvec2() + b.to_meters_center().as_dvec2());
            canvas.fill_circle(a.extend(node_z.as_f32()), node_radius, node_color);
            canvas.fill_circle(b.extend(node_z.as_f32()), node_radius, node_color);
            canvas.line_t(a, b, pipe_z, pipe_thickness, pipe_color);
        }
        PipeSegments::Double(a, b, c) => {
            let a = ctx.w2c(offset.as_dvec2() + a.to_meters_center().as_dvec2());
            let b = ctx.w2c(offset.as_dvec2() + b.to_meters_center().as_dvec2());
            let c = ctx.w2c(offset.as_dvec2() + c.to_meters_center().as_dvec2());
            canvas.fill_circle(a.extend(node_z.as_f32()), node_radius, node_color);
            canvas.fill_circle(c.extend(node_z.as_f32()), node_radius, node_color);
            canvas.line_t(a, b, pipe_z, pipe_thickness, pipe_color);
            canvas.line_t(b, c, pipe_z, pipe_thickness, pipe_color);
        }
    }
}

pub fn draw_inventory_graph(canvas: &mut Canvas, graph: &InventoryGraph, offset: Vec2, scale: f32) {
    let z = ZOrdering::Debug2;

    let w2p = |p: Vec2| -> Vec2 { offset + p * scale };

    for (_, node) in &graph.nodes {
        let rect = AABB::from_wh(10.0 * scale, 10.0 * scale).with_center(w2p(node.pos));
        let color = GREEN;
        canvas.hollow_rect(rect, z, color, 1.0);
    }

    for (a, b) in &graph.edges {
        let Some(a) = graph.nodes.get(a) else {
            continue;
        };
        let Some(b) = graph.nodes.get(b) else {
            continue;
        };

        canvas.line(w2p(a.pos), w2p(b.pos), z, GREEN.with_alpha(0.4));
    }
}

pub fn draw_editor(canvas: &mut Canvas, state: &GameState) -> Option<()> {
    let ctx = &state.editor_context;
    draw_cross(&mut canvas.gizmos, ctx.w2c(DVec2::ZERO), 10.0, GRAY);

    if let Some((pos, dims)) = ctx.snap_info {
        let lower = pos.to_meters();
        let upper = (pos + PartCoord::new(dims.as_ivec2())).to_meters();
        let aabb = AABB::from_arbitrary(lower, upper);
        draw_aabb(canvas, ctx.w2c_aabb(aabb), GREEN);
    }

    if let Some(sel) = ctx.cursor_state.selected() {
        if let Some(aabb) = sel.aabb() {
            draw_aabb(canvas, ctx.w2c_aabb(aabb), RED);
        }
    }

    if let Some(p) = state.input.current() {
        let p = p.extend(ZOrdering::EditorCursor.as_f32());
        canvas.circle(p, 4.0, WHITE);
    }

    let radius = ctx.blueprint.bounding_radius();

    // gridlines
    {
        let n = 100;
        for x in -n..=n {
            let top = PartCoord::new(IVec2::new(x, 50));
            let bottom = PartCoord::new(IVec2::new(x, -50));
            let t = ctx.w2c(top.to_meters().as_dvec2());
            let b = ctx.w2c(bottom.to_meters().as_dvec2());
            canvas.gizmos.line_2d(t, b, GRAY.with_alpha(0.02));
        }

        for y in -n..=n {
            let top = PartCoord::new(IVec2::new(50, y));
            let bottom = PartCoord::new(IVec2::new(-50, y));
            let t = ctx.w2c(top.to_meters().as_dvec2());
            let b = ctx.w2c(bottom.to_meters().as_dvec2());
            canvas.gizmos.line_2d(t, b, GRAY.with_alpha(0.02));
        }
    }

    // current cursor position
    {
        if let Some(cursor) = Editor::current_cursor_coord(state) {
            draw_cell(canvas, cursor, ctx, RED.with_alpha(0.5));
        }
    }

    // pipe
    {
        if let Some(p) = state.editor_context.cursor_state.pipe() {
            if let Some(geo) = p.pipe_geometry() {
                draw_pipe(canvas, &geo, Vec2::ZERO, ctx, GRAY_400, GRAY_600);
            }
        }
    }

    if let Some((p, current_part)) = Editor::current_part_and_cursor_position(state) {
        let gp = GridPlacement::new(p, ctx.rotation, current_part.dims);
        let current_pixels = gp.cells();

        let mut visited_parts = HashSet::new();

        if let Some(occ) = ctx.occupied.get(&current_part.layer()) {
            for q in current_pixels {
                if let Some(idx) = occ.get(&q) {
                    if visited_parts.contains(idx) {
                        continue;
                    }
                    visited_parts.insert(*idx);
                    if let Some(instance) = ctx.blueprint.get_part(*idx) {
                        highlight_part(
                            canvas,
                            instance,
                            ctx,
                            RED.with_alpha(0.6),
                            ZOrdering::EditorConflictHighlight,
                        );
                    }
                }
            }
        }
    }

    if ctx.show_vehicle_info {
        draw_circle(
            &mut canvas.gizmos,
            ctx.w2c(DVec2::ZERO),
            gcast(radius * ctx.scale()),
            RED.with_alpha(0.1),
        );
    }

    draw_blueprint(
        canvas,
        PartCoord::new(IVec2::ZERO),
        &ctx.blueprint,
        ctx,
        ctx.focus_layer,
    );

    if let Some(cursor) = state.input.position(MouseButt::Hover, FrameId::Current) {
        let c = ctx.c2w(cursor);

        if let Some(bp) = ctx.cursor_state.blueprint() {
            let c = PartCoord::from_meters_floored(c);
            draw_blueprint(canvas, c, bp, ctx, ctx.focus_layer);
        }

        if Editor::current_part_and_cursor_position(state).is_none() {
            if let Some((id, _)) = ctx.get_part_at(graphics_cast(c)) {
                if let Some(instance) = ctx.blueprint.get_part(id) {
                    highlight_part(
                        canvas,
                        instance,
                        ctx,
                        TEAL.with_alpha(0.6),
                        ZOrdering::EditorMouseoverPartHighlight,
                    );
                }
            }
        }
    }

    for part in &ctx.selected_parts {
        let Some(instance) = ctx.blueprint.get_part(*part) else {
            continue;
        };
        highlight_part(
            canvas,
            instance,
            ctx,
            GREEN.with_alpha(0.4),
            ZOrdering::EditorMouseoverPartHighlight,
        );
    }

    if let Some((p, current_part)) = Editor::current_part_and_cursor_position(state) {
        let dims = pixel_dims_with_rotation(ctx.rotation, &current_part);
        let sprite_dims = current_part.dims();
        canvas.sprite(
            ctx.w2c((p.inner().as_dvec2() + dims.as_dvec2() / 2.0) / GRID_CELLS_PER_METER as f64),
            gcast(ctx.rotation.to_angle()),
            current_part.part_name().clone(),
            ZOrdering::EditorCursor,
            sprite_dims.as_vec2() / GRID_CELLS_PER_METER * gcast(ctx.scale()),
        );
    }

    draw_inventory_graph(canvas, &ctx.graph, Vec2::Y * 500.0, 0.4);

    Some(())
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

impl CameraProjection for Editor {
    fn origin(&self) -> DVec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f64 {
        self.camera.scale()
    }

    fn offset(&self) -> DVec2 {
        self.camera.offset()
    }

    fn distance(&self) -> f64 {
        self.camera.distance()
    }

    fn angle(&self) -> f64 {
        self.camera.angle()
    }
}

impl Editor {
    pub fn on_game_tick(state: &mut GameState) {
        state.editor_context.camera.on_game_tick();
    }
}

pub fn write_image_to_file(blueprint: &Blueprint, parts: &PartDatabase, name: &str) -> Option<()> {
    let outpath: String = format!("/tmp/{}.png", name);
    println!("Writing blueprint to path {}", outpath);
    let img = generate_image(blueprint, parts)?;
    img.save(outpath).ok()
}

pub fn on_editor_render_tick(state: &mut GameState) {
    state
        .editor_context
        .camera
        .handle_input(&state.input, &state.settings);

    if state.input.is_pressed(KeyCode::ControlLeft) && state.input.just_pressed(KeyCode::KeyC) {
        Editor::on_ctrl_c(state);
    }

    if state.input.just_pressed(KeyCode::Delete) {
        Editor::on_delete(state);
    }

    if let Some(p) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
        let is_shift = state.input.is_pressed(KeyCode::ShiftLeft);
        Editor::on_left_click_down(state, p, is_shift);
    }

    if let Some(_) = state.input.on_frame(MouseButt::Left, FrameId::Current) {
        Editor::on_left_click_held(state);
    }

    if let Some(_) = state.input.on_frame(MouseButt::Left, FrameId::Up) {
        Editor::on_left_click_release(state);
    }

    Editor::process_holding_shift(state);

    if let Some(_) = state.input.position(MouseButt::Left, FrameId::Current) {
        // place a single part
        if let Some((p, part)) = Editor::current_part_and_cursor_position(state) {
            state
                .editor_context
                .try_place_part(p, part, state.editor_context.rotation);
        }

        if let Some(bp) = state.editor_context.cursor_state.blueprint() {
            if let Some(pos) = state.input.on_frame(MouseButt::Left, FrameId::Down) {
                let pos = PartCoord::from_meters_floored(state.editor_context.c2w(pos));
                let bp = bp.clone();
                for (_, part) in bp.parts() {
                    let proto = state.part_database.get(&part.name).unwrap().clone();
                    state.editor_context.try_place_part(
                        pos + part.origin(),
                        proto,
                        part.rotation(),
                    );
                }
                for (_, part) in bp.pipes() {
                    state
                        .editor_context
                        .blueprint
                        .add_pipe(part.with_offset(pos));
                }
                state.editor_context.update_graph();
            }
        }
    }

    if let Some(p) = state.input.on_frame(MouseButt::Right, FrameId::Down) {
        Editor::on_right_click_down(state, p);
    }

    if state.input.just_pressed(KeyCode::KeyQ) {
        Editor::on_press_q(state);
    }

    if state.input.just_pressed(KeyCode::KeyR) {
        Editor::on_press_r(state);
    }

    if state.input.is_pressed(KeyCode::ControlLeft) && state.input.just_pressed(KeyCode::KeyZ) {
        state.editor_context.undo();
    }

    state.editor_context.graph.step(0.01);
}
