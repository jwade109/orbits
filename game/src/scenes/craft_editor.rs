use crate::args::ProgramContext;
use crate::drawing::*;
use crate::mouse::InputState;
use crate::mouse::{FrameId, MouseButt};
use crate::planetary::GameState;
use crate::scenes::{CameraProjection, Render, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::prelude::*;
use enum_iterator::next_cycle;
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
    occupied: HashMap<PartLayer, HashSet<IVec2>>,
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
        }
    }

    pub fn mass(&self) -> f32 {
        self.parts.iter().map(|(_, _, part)| part.data.mass).sum()
    }

    pub fn cursor_box(&self, input: &InputState) -> Option<AABB> {
        let p1 = input.position(MouseButt::Left, FrameId::Down)?;
        let p2 = input.position(MouseButt::Left, FrameId::Current)?;
        Some(AABB::from_arbitrary(
            vround(self.c2w(p1)).as_vec2(),
            vround(self.c2w(p2)).as_vec2(),
        ))
    }

    pub fn dims_with_rotation(rot: Rotation, part: &PartProto) -> UVec2 {
        match rot {
            Rotation::East | Rotation::West => UVec2::new(part.width, part.height),
            Rotation::North | Rotation::South => UVec2::new(part.height, part.width),
        }
    }

    pub fn set_current_part(state: &mut GameState, name: &String) {
        state.editor_context.current_part = state.part_database.get(name).cloned();
    }

    fn open_file(&mut self, force_new: bool) -> Option<PathBuf> {
        if self.filepath.is_none() || force_new {
            self.filepath = FileDialog::new().set_directory("/").pick_file()
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
        let choice = state.editor_context.open_file(false)?;
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
        let choice = state.editor_context.open_file(true)?;
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
        state.editor_context.update_occupied();
        Some(())
    }

    fn occupied_pixels(pos: IVec2, rot: Rotation, part: &PartProto) -> Vec<IVec2> {
        let mut ret = vec![];
        let wh = Self::dims_with_rotation(rot, part);
        for w in 0..wh.x {
            for h in 0..wh.y {
                let p = pos + UVec2::new(w, h).as_ivec2();
                ret.push(p);
            }
        }
        ret
    }

    fn get_part_at(&self, p: IVec2) -> Option<(IVec2, Rotation, PartProto)> {
        for (pos, rot, part) in self.visible_parts() {
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            if pixels.contains(&p) {
                return Some((*pos, *rot, part.clone()));
            }
        }
        None
    }

    fn update_occupied(&mut self) {
        self.occupied.clear();
        for (pos, rot, part) in &self.parts {
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            if let Some(occ) = self.occupied.get_mut(&part.data.layer) {
                occ.extend(&pixels);
            } else {
                self.occupied
                    .insert(part.data.layer, HashSet::from_iter(pixels.into_iter()));
            }
        }
    }

    fn try_place_part(&mut self, p: IVec2, new_part: PartProto) -> Option<()> {
        let new_pixels = Self::occupied_pixels(p, self.rotation, &new_part);
        if let Some(occ) = self.occupied.get(&new_part.data.layer) {
            for p in &new_pixels {
                if occ.contains(p) {
                    return None;
                }
            }
        }
        self.parts.push((p, self.rotation, new_part));
        self.update_occupied();
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
        self.update_occupied();
    }

    fn current_part_and_cursor_position(state: &GameState) -> Option<(IVec2, PartProto)> {
        let ctx = &state.editor_context;
        let part = state.editor_context.current_part.clone()?;
        let wh = Self::dims_with_rotation(ctx.rotation, &part).as_ivec2();
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = vround(state.editor_context.c2w(pos));
        Some((pos - wh / 2, part))
    }
}

pub fn part_sprite_path(ctx: &ProgramContext, short_path: &str) -> String {
    ctx.parts_dir()
        .join(format!("{}/skin.png", short_path))
        .to_str()
        .unwrap()
        .to_string()
}

impl Render for EditorContext {
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let filename = match &state.editor_context.filepath {
            Some(p) => format!("[{}]", p.display()),
            None => "[No file open]".to_string(),
        };

        let info_lines = [
            filename,
            format!("Title: {:?}", &state.editor_context.title.0),
            format!("{} parts", state.editor_context.parts.len()),
            format!("Rotation: {:?}", state.editor_context.rotation),
            format!("Mass: {} kg", state.editor_context.mass()),
        ];

        let half_span = state.input.screen_bounds.span * 0.5;

        let mut labels: Vec<TextLabel> = info_lines
            .into_iter()
            .enumerate()
            .map(|(i, s)| TextLabel {
                text: s,
                position: Vec2::new(half_span.x - 350.0, half_span.y - (200.0 + i as f32 * 30.0)),
                size: 0.8,
            })
            .collect();

        if let Some(p) = state.editor_context.current_part.as_ref() {
            let t = TextLabel {
                text: format!("{:#?}", &p.data),
                position: Vec2::ZERO,
                size: 0.8,
            };
            labels.push(t);
        }

        Some(labels)
    }

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        let ctx = &state.editor_context;
        let mut ret = Vec::new();

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let dims = Self::dims_with_rotation(ctx.rotation, &current_part);
            ret.push(StaticSpriteDescriptor::new(
                ctx.w2c(p.as_vec2() + dims.as_vec2() / 2.0),
                ctx.rotation.to_angle(),
                part_sprite_path(&state.args, &current_part.path),
                ctx.scale(),
                12.0,
            ));

            // let current_pixels = Self::occupied_pixels(p, ctx.rotation, &current_part);

            // for (layer, occupied) in &ctx.occupied {
            //     if layer != &current_part.data.layer {
            //         continue;
            //     }
            //     for p in &current_pixels {
            //         if occupied.contains(p) {
            //             ret.push(StaticSpriteDescriptor {
            //                 position: ctx.w2c(p.as_vec2() + Vec2::ONE / 2.0),
            //                 angle: 0.0,
            //                 path: "embedded://game/../assets/collision_pixel.png".into(),
            //                 scale: ctx.scale(),
            //                 z_index: 100.0,
            //             });
            //         }
            //     }
            // }
        }

        ret.extend(
            ctx.visible_parts()
                .enumerate()
                .map(|(i, (pos, rot, part))| {
                    let half_dims = Self::dims_with_rotation(*rot, part).as_vec2() / 2.0;
                    StaticSpriteDescriptor::new(
                        ctx.w2c(pos.as_vec2() + half_dims),
                        rot.to_angle(),
                        part_sprite_path(&state.args, &part.path),
                        ctx.scale(),
                        part.to_z_index() + i as f32 / 100.0,
                    )
                }),
        );

        Some(ret)
    }

    fn background_color(_state: &GameState) -> bevy::color::Srgba {
        GRAY.with_luminance(0.06)
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        let ctx = &state.editor_context;
        draw_cross(gizmos, ctx.w2c(Vec2::ZERO), 10.0, GRAY);

        let vehicle = Vehicle::from_parts(Nanotime::zero(), ctx.parts.clone());

        let radius = vehicle.bounding_radius();

        let cursor = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let c = ctx.c2w(cursor);

        draw_circle(
            gizmos,
            ctx.w2c(Vec2::ZERO),
            radius * ctx.scale(),
            RED.with_alpha(0.1),
        );

        if let Some((p, rot, part)) = ctx.get_part_at(vround(c)) {
            let wh = Self::dims_with_rotation(rot, &part).as_ivec2();
            let q = p + wh;
            let r = p + IVec2::X * wh.x;
            let s = p + IVec2::Y * wh.y;
            for p in [p, q, r, s] {
                let p = p.as_vec2();
                let alpha = 1.0 - p.distance(c) / 100.0;
                draw_cross(gizmos, ctx.w2c(p), 0.5 * ctx.scale(), RED.with_alpha(alpha));
            }
        }

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
                    draw_diamond(gizmos, ctx.w2c(p.as_vec2()), 7.0, GRAY.with_alpha(alpha));
                }
            }
        }

        Some(())
    }

    fn ui(_state: &GameState) -> Option<layout::layout::Tree<crate::onclick::OnClick>> {
        todo!()
    }
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
                let p = vround(state.editor_context.c2w(p));
                state.editor_context.remove_part_at(p);
            } else if state.input.just_pressed(KeyCode::KeyQ) {
                if state.editor_context.current_part.is_some() {
                    state.editor_context.current_part = None;
                } else if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
                    let p = vround(state.editor_context.c2w(p));
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
