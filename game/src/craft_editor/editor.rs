use crate::args::ProgramContext;
use crate::camera_controller::*;
use crate::canvas::*;
use crate::craft_editor::*;
use crate::drawing::*;
use crate::game::GameState;
use crate::input::InputState;
use crate::input::{FrameId, MouseButt};
use crate::scenes::*;
use crate::starling::prelude::*;
use crate::z_index::ZOrdering;
use bevy::color::palettes::css::*;
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
    pub selected_part: Option<PartId>,
    pub snap_info: Option<(PartCoord, UVec2)>,
    pub action_queue: Vec<Action>,
    pub occupied: HashMap<PartLayer, HashMap<PartCoord, PartId>>,
    pub blueprint: Blueprint,

    pub atmo: i32,

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
            selected_part: None,
            snap_info: None,
            action_queue: Vec::new(),
            occupied: HashMap::new(),
            blueprint: Blueprint::new(),
            atmo: 3,
            show_vehicle_info: false,
        }
    }

    pub fn remove_part(&mut self, id: PartId) {
        self.blueprint.remove_part(id);
    }

    pub fn undo(&mut self) -> Option<()> {
        let action = self.action_queue.pop()?;
        match action {
            Action::Add(id) => match self.blueprint.remove_part(id) {
                Some(p) => println!("Removed {:?}", p),
                None => println!("Failed to remove"),
            },
            Action::Remove(pos, rot, proto) => self.add_part(pos, rot, proto),
        }
        Some(())
    }

    pub fn selected_part(&self) -> Option<&InstantiatedPart> {
        self.blueprint.get_part(self.selected_part?)
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

    pub fn write_image_to_file(&self, args: &ProgramContext) {
        write_image_to_file(&self.blueprint, args, "vehicle");
    }

    pub fn rotate_craft(&mut self) {
        self.blueprint.rotate();
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
            .blueprint
            .parts()
            .map(|(_, instance)| VehiclePartFileStorage {
                partname: instance.prototype().sprite_path().to_string(),
                pos: instance.origin(),
                rot: instance.rotation(),
            })
            .collect();

        let storage = VehicleFileStorage { parts };

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

        state.editor_context.blueprint = vehicle;
        state.editor_context.filepath = Some(path.to_path_buf());
        state.editor_context.update();
        state.editor_context.action_queue.clear();
        state.editor_context.cursor_state = CursorState::None;
        Some(())
    }

    pub fn get_part_at(&self, p: Vec2) -> Option<(PartId, &InstantiatedPart)> {
        let pixel_p = PartCoord::from_meters(p);

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
        self.occupied.clear();
        for (id, instance) in self.blueprint.parts() {
            let pixels = occupied_cells(
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

    fn add_part(&mut self, p: PartCoord, rot: Rotation, proto: PartPrototype) {
        self.blueprint.add_part(proto, p, rot);
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

        let new_pixels = occupied_cells(p, rot, &new_part);

        if let Some(occ) = self.occupied.get(&layer) {
            for p in &new_pixels {
                if occ.contains_key(p) {
                    return None;
                }
            }
        }

        let id = self.blueprint.add_part(new_part.clone(), p, rot);

        self.action_queue.push(Action::Add(id));

        self.update();
        Some(())
    }

    pub fn remove_part_at(&mut self, p: Vec2) {
        let pixel_p = PartCoord::from_meters(p);
        if let Ok(part) = self.blueprint.remove_part_at(pixel_p, self.focus_layer) {
            self.action_queue.push(Action::Remove(
                part.origin(),
                part.rotation(),
                part.prototype(),
            ));
        }
        self.update();
    }

    pub fn current_part_and_cursor_position(
        state: &GameState,
    ) -> Option<(PartCoord, PartPrototype)> {
        let ctx = &state.editor_context;
        let part = state.editor_context.cursor_state.current_part()?;
        let wh = pixel_dims_with_rotation(ctx.rotation, &part).as_ivec2();
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = PartCoord::from_meters(state.editor_context.c2w(pos));
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
        Some((pos, part))
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
    instance: &InstantiatedPart,
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
    offset: DVec2,
    blueprint: &Blueprint,
    ctx: &impl CameraProjection,
    focus_layer: Option<PartLayer>,
) {
    for layer in PartLayer::draw_order() {
        for (_, instance) in blueprint
            .parts()
            .filter(|(_, p)| p.prototype().layer() == layer)
        {
            let alpha = match (focus_layer, layer) {
                (None, _) => 1.0,
                (Some(PartLayer::Internal), PartLayer::Internal) => 1.0,
                (Some(PartLayer::Internal), _) => 0.02,
                (Some(PartLayer::Structural), PartLayer::Structural) => 1.0,
                (Some(PartLayer::Structural), _) => 0.02,
                (Some(PartLayer::Exterior), PartLayer::Exterior) => 1.0,
                (Some(PartLayer::Exterior), _) => 0.02,
            };

            let sprite_dims = instance.prototype().dims_meters();
            let center = instance.center_meters().as_dvec2();
            let sprite_name = instance.prototype().sprite_path().to_string();

            let z_index = match layer {
                PartLayer::Exterior => ZOrdering::EditorExteriorPart,
                PartLayer::Internal => ZOrdering::EditorInteriorPart,
                PartLayer::Structural => ZOrdering::EditorStructuralPart,
            };

            canvas
                .sprite(
                    ctx.w2c(offset + center),
                    gcast(instance.rotation().to_angle()),
                    sprite_name,
                    z_index,
                    graphics_cast(sprite_dims.as_dvec2() * ctx.scale()),
                )
                .set_color(WHITE.with_alpha(alpha));
        }
    }
}

impl Render for Editor {
    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        let ctx = &state.editor_context;
        draw_cross(&mut canvas.gizmos, ctx.w2c(DVec2::ZERO), 10.0, GRAY);

        if let Some((pos, dims)) = ctx.snap_info {
            let lower = pos.to_meters();
            let upper = (pos + PartCoord::new(dims.as_ivec2())).to_meters();
            let aabb = AABB::from_arbitrary(lower, upper);
            draw_aabb(canvas, ctx.w2c_aabb(aabb), GREEN);
        }

        if let Some(p) = state.input.current() {
            let p = p.extend(ZOrdering::EditorCursor.as_f32());
            canvas.circle(p, 4.0, WHITE);
        }

        let radius = ctx.blueprint.bounding_radius();

        let filename = match &state.editor_context.filepath {
            Some(p) => format!("[{}]", p.display()),
            None => "[No file open]".to_string(),
        };

        let vehicle_info = String::new();

        let info: String = [
            filename,
            format!("{} parts", state.editor_context.blueprint.parts().count()),
            format!("Rotation: {:?}", state.editor_context.rotation),
        ]
        .into_iter()
        .map(|s| format!("{s}\n"))
        .collect();

        let info = format!("{}{}", info, vehicle_info);

        // TODO re-add info!

        // axes
        {
            let length = 30.0;
            let width = 30.0;
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

        // gridlines
        {
            for x in -30..=30 {
                let top = PartCoord::new(IVec2::new(x, 50));
                let bottom = PartCoord::new(IVec2::new(x, -50));
                let t = ctx.w2c(top.to_meters().as_dvec2());
                let b = ctx.w2c(bottom.to_meters().as_dvec2());
                canvas.gizmos.line_2d(t, b, GRAY.with_alpha(0.3));
            }

            for y in -30..=30 {
                let top = PartCoord::new(IVec2::new(50, y));
                let bottom = PartCoord::new(IVec2::new(-50, y));
                let t = ctx.w2c(top.to_meters().as_dvec2());
                let b = ctx.w2c(bottom.to_meters().as_dvec2());
                canvas.gizmos.line_2d(t, b, GRAY.with_alpha(0.3));
            }
        }

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let current_pixels = occupied_cells(p, ctx.rotation, &current_part);

            let mut visited_parts = HashSet::new();

            if let Some(occ) = ctx.occupied.get(&current_part.layer()) {
                for q in &current_pixels {
                    if let Some(idx) = occ.get(q) {
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

        draw_blueprint(canvas, DVec2::ZERO, &ctx.blueprint, ctx, ctx.focus_layer);

        if let Some(cursor) = state.input.position(MouseButt::Hover, FrameId::Current) {
            let c = ctx.c2w(cursor);

            if let Some(bp) = ctx.cursor_state.blueprint() {
                let c = (c * 20.0).round() / 20.0;
                draw_blueprint(canvas, c, bp, ctx, ctx.focus_layer);
            }

            if Self::current_part_and_cursor_position(state).is_none() {
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

        if let Some(instance) = ctx.selected_part() {
            highlight_part(
                canvas,
                instance,
                ctx,
                GREEN.with_alpha(0.4),
                ZOrdering::EditorMouseoverPartHighlight,
            );
        }

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let dims = pixel_dims_with_rotation(ctx.rotation, &current_part);
            let sprite_dims = current_part.dims();
            canvas.sprite(
                ctx.w2c(
                    (p.inner().as_dvec2() + dims.as_dvec2() / 2.0) / GRID_CELLS_PER_METER as f64,
                ),
                gcast(ctx.rotation.to_angle()),
                current_part.sprite_path().to_string(),
                ZOrdering::EditorCursor,
                sprite_dims.as_vec2() / GRID_CELLS_PER_METER * gcast(ctx.scale()),
            );
        }

        Some(())
    }
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

    fn parent(&self) -> EntityId {
        self.camera.parent()
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

pub fn write_image_to_file(vehicle: &Blueprint, ctx: &ProgramContext, name: &str) -> Option<()> {
    let outpath: String = format!("/tmp/{}.png", name);
    println!("Writing vehicle to path {}", outpath);
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

        let args = ProgramContext::new(Some(dir));

        let g = GameState::new(args.clone());

        let vehicles = get_list_of_vehicles(&g).expect("Expected list of vehicles");
        dbg!(vehicles);

        for name in ["remora", "lander", "pollux", "manta", "spacestation"] {
            let vehicle = g.get_vehicle_by_model(name).expect("Expected a vehicle");
            write_image_to_file(&vehicle, &args, name);
        }
    }
}
