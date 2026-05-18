use crate::utils::{GlobalKeybinds, load_keybinds_from_file};
use bary_parts::load_parts_from_dir;
use log::debug;
use raylib::prelude::*;
use std::{collections::BTreeMap, path::Path};

pub type MaybeTexture = Option<Texture2D>;

pub type MaybeFont = Option<Font>;

#[derive(Default)]
pub struct Assets {
    pub circle_texture: MaybeTexture,
    pub lato_regular: MaybeFont,
    pub fira_code: MaybeFont,
    pub part_textures: BTreeMap<String, Texture2D>,
    pub terrain_textures: Vec<Texture2D>,
    pub animation: MaybeTexture,
    pub terrain_spritesheet: MaybeTexture,
    pub ship_names: Vec<String>,
    pub keybinds: GlobalKeybinds,
}

pub fn load_names_from_file(
    filename: impl AsRef<Path>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(filename)?
        .lines()
        .filter_map(|s| (!s.is_empty()).then(|| s.to_string()))
        .collect())
}

pub fn load_assets(
    assets: &mut Assets,
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
) {
    debug!("Loading assets");
    assets.circle_texture = rl.load_texture(thread, "assets/parts/frame2/skin.png").ok();
    assets.lato_regular = rl
        .load_font_ex(thread, "assets/fonts/Lato-Regular.ttf", 48, None)
        .ok();
    assets.fira_code = rl
        .load_font_ex(thread, "assets/fonts/FiraCode-Bold.ttf", 128, None)
        .ok();

    assets.ship_names = load_names_from_file("assets/ship_names.txt").unwrap_or(vec![]);

    assets.keybinds = load_keybinds_from_file("assets/keybinds.yaml").unwrap();

    let parts = load_parts_from_dir("assets/parts/").unwrap();

    for (name, _part) in parts {
        let skin_path = format!("assets/parts/{}/skin.png", name);
        if let Ok(tex) = rl.load_texture(thread, &skin_path) {
            assets.part_textures.insert(name, tex);
        }
    }

    for i in 1..=5 {
        let path = format!("assets/terrain/terrain{}.png", i);
        if let Ok(tex) = rl.load_texture(thread, &path) {
            assets.terrain_textures.push(tex);
        }
    }

    assets.terrain_spritesheet = rl
        .load_texture(&thread, "assets/terrain/terrain_sprites.png")
        .ok();
}
