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
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VehicleFileStorage {
    pub name: String,
    pub parts: Vec<VehiclePartFileStorage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VehiclePartFileStorage {
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
    parts: Vec<(IVec2, Rotation, String)>,
    current_part: Option<String>,
    rotation: Rotation,
    filepath: Option<PathBuf>,
    title: TextInput,
    invisible_layers: HashSet<PartLayer>,
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

    fn dims_with_rotation(rot: Rotation, part: &PartProto) -> UVec2 {
        match rot {
            Rotation::East | Rotation::West => UVec2::new(part.width, part.height),
            Rotation::North | Rotation::South => UVec2::new(part.height, part.width),
        }
    }

    fn find_part<'a>(state: &'a GameState, name: &String) -> Option<&'a PartProto> {
        state.part_database.get(name)
    }

    pub fn set_current_part(&mut self, name: String) {
        self.current_part = Some(name);
    }

    fn open_file(&mut self, force_new: bool) -> Option<PathBuf> {
        if self.filepath.is_none() || force_new {
            self.filepath = FileDialog::new().set_directory("/").pick_file()
        };
        self.filepath.clone()
    }

    fn visible_parts<'a>(
        &'a self,
        state: &'a GameState,
    ) -> impl Iterator<Item = (&'a IVec2, &'a Rotation, &'a String)> {
        self.parts.iter().filter_map(|(pos, rot, name)| {
            let part = Self::find_part(state, name)?;
            let layer = part.layer;
            if self.invisible_layers.contains(&layer) {
                None
            } else {
                Some((pos, rot, name))
            }
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
                partname: part.clone(),
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

    pub fn load_vehicle(path: &Path, state: &mut GameState) -> Option<()> {
        state.notice(format!("Loading vehicle from {}", path.display()));
        let s = std::fs::read_to_string(path).ok()?;
        let storage: VehicleFileStorage = serde_yaml::from_str(&s).ok()?;
        state.notice(format!("Loaded vehicle \"{}\"", storage.name));

        state.editor_context.parts.clear();
        for ps in storage.parts {
            state
                .editor_context
                .parts
                .push((ps.pos, ps.rot, ps.partname.clone()));
        }
        state.editor_context.title.0 = storage.name;
        state.editor_context.filepath = Some(path.to_path_buf());
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

    fn get_part_at(&self, state: &GameState, p: IVec2) -> Option<(IVec2, Rotation, PartProto)> {
        for (pos, rot, name) in self.visible_parts(state) {
            let part = match Self::find_part(state, name) {
                Some(p) => p,
                None => continue,
            };
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            if pixels.contains(&p) {
                return Some((*pos, *rot, part.clone()));
            }
        }
        None
    }

    fn try_place_part(state: &mut GameState, p: IVec2, new_part: &PartProto) -> Option<()> {
        let new_pixels = Self::occupied_pixels(p, state.editor_context.rotation, new_part);
        for (pos, rot, name) in &state.editor_context.parts {
            let part = match Self::find_part(state, name) {
                Some(p) => p,
                None => continue,
            };
            if part.layer != new_part.layer {
                continue;
            }
            let pixels = Self::occupied_pixels(*pos, *rot, part);
            for q in pixels {
                if new_pixels.contains(&q) {
                    return None;
                }
            }
        }
        state
            .editor_context
            .parts
            .push((p, state.editor_context.rotation, new_part.path.clone()));
        Some(())
    }

    fn remove_part_at(state: &mut GameState, p: IVec2) {
        state.editor_context.parts.retain(|(pos, rot, name)| {
            true
            // let part = match Self::find_part(state, name) {
            //     Some(p) => p,
            //     None => return false,
            // };
            // if state.editor_context.invisible_layers.contains(&part.layer) {
            //     return true;
            // }
            // let pixels = Self::occupied_pixels(*pos, *rot, part);
            // !pixels.contains(&p)
        });
    }

    fn current_part_and_cursor_position<'a>(
        state: &'a GameState,
    ) -> Option<(IVec2, &'a PartProto)> {
        let ctx = &state.editor_context;
        let part = Self::find_part(state, &state.editor_context.current_part.clone()?)?;
        let wh = Self::dims_with_rotation(ctx.rotation, &part).as_ivec2();
        let pos = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let pos = vround(state.editor_context.c2w(pos));
        Some((pos - wh / 2, part))
    }
}

pub fn part_sprite_path(install_dir: &Path, short_path: &str) -> PathBuf {
    install_dir.join(format!("parts/{}.png", short_path))
}

impl Render for EditorContext {
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let filename = match &state.editor_context.filepath {
            Some(p) => format!("[{}]", p.display()),
            None => "[No file open]".to_string(),
        };
        let half_span = state.input.screen_bounds.span * 0.5;
        Some(vec![
            TextLabel {
                text: state.editor_context.title.0.clone(),
                position: Vec2::new(450.0, 30.0) - half_span,
                size: 2.0,
            },
            TextLabel {
                text: format!(
                    "{}\n{} parts / {:?}",
                    filename,
                    state.editor_context.parts.len(),
                    state.editor_context.rotation,
                ),
                position: Vec2::new(0.0, 40.0 - half_span.y),
                size: 0.7,
            },
        ])
    }

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        let ctx = &state.editor_context;
        let mut ret = Vec::new();

        if let Some((p, current_part)) = Self::current_part_and_cursor_position(state) {
            let dims = Self::dims_with_rotation(ctx.rotation, &current_part);
            let path = part_sprite_path(
                &PathBuf::from(state.args.install_dir.clone()),
                &current_part.path,
            )
            .to_str()
            .unwrap()
            .to_string();
            ret.push(StaticSpriteDescriptor {
                position: ctx.w2c(p.as_vec2() + dims.as_vec2() / 2.0),
                angle: ctx.rotation.to_angle(),
                path,
                scale: ctx.scale(),
                z_index: 12.0,
            });

            let current_pixels = Self::occupied_pixels(p, ctx.rotation, &current_part);

            for (pos, rot, name) in &ctx.parts {
                let part = match Self::find_part(state, name) {
                    Some(p) => p,
                    None => continue,
                };
                if part.layer != current_part.layer {
                    continue;
                }
                let pixels = Self::occupied_pixels(*pos, *rot, part);
                for q in pixels {
                    if current_pixels.contains(&q) {
                        ret.push(StaticSpriteDescriptor {
                            position: ctx.w2c(q.as_vec2() + Vec2::ONE / 2.0),
                            angle: 0.0,
                            path: "embedded://game/../assets/collision_pixel.png".into(),
                            scale: ctx.scale(),
                            z_index: 100.0,
                        });
                    }
                }
            }
        }

        ret.extend(
            ctx.visible_parts(state)
                .enumerate()
                .filter_map(|(i, (pos, rot, name))| {
                    let part = Self::find_part(state, name)?;
                    let half_dims = Self::dims_with_rotation(*rot, part).as_vec2() / 2.0;
                    let path =
                        part_sprite_path(&PathBuf::from(state.args.install_dir.clone()), name)
                            .to_str()
                            .unwrap()
                            .to_string();
                    Some(StaticSpriteDescriptor {
                        position: ctx.w2c(pos.as_vec2() + half_dims),
                        angle: rot.to_angle(),
                        path,
                        scale: ctx.scale(),
                        z_index: part.to_z_index() + i as f32 / 100.0,
                    })
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

        let cursor = state.input.position(MouseButt::Hover, FrameId::Current)?;
        let c = ctx.c2w(cursor);

        if let Some((p, rot, part)) = ctx.get_part_at(state, vround(c)) {
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
                draw_diamond(gizmos, ctx.w2c(p.as_vec2()), 7.0, GRAY.with_alpha(alpha));
            }
        }

        Some(())
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
            if state
                .input
                .position(MouseButt::Left, FrameId::Current)
                .is_some()
            {
                if let Some((p, part)) = Self::current_part_and_cursor_position(state) {
                    Self::try_place_part(state, p, &part.clone());
                }
            } else if let Some(p) = state.input.position(MouseButt::Right, FrameId::Current) {
                let p = vround(state.editor_context.c2w(p));
                Self::remove_part_at(state, p);
            } else if state.input.just_pressed(KeyCode::KeyQ) {
                if state.editor_context.current_part.is_some() {
                    state.editor_context.current_part = None;
                } else if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
                    let p = vround(state.editor_context.c2w(p));
                    if let Some((_, rot, part)) = state.editor_context.get_part_at(state, p) {
                        state.editor_context.rotation = rot;
                        state.editor_context.current_part = Some(part.path);
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
