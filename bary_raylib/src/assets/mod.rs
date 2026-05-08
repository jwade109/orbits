use bary_core::prelude::randint;
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
    pub ship_names: Vec<String>,
}

pub fn load_names_from_file(
    filename: impl AsRef<Path>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(filename)?
        .lines()
        .filter_map(|s| (!s.is_empty()).then(|| s.to_string()))
        .collect())
}

pub fn get_random_ship_name(names: &Vec<String>) -> String {
    if names.is_empty() {
        return String::new();
    }
    let idx = randint(0, names.len() as i32) as usize;
    names[idx].clone()
}

pub fn load_assets(
    assets: &mut Assets,
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
) {
    debug!("Loading assets");
    assets.circle_texture = rl.load_texture(thread, "assets/circle.png").ok();
    assets.lato_regular = rl
        .load_font_ex(thread, "assets/fonts/Lato-Regular.ttf", 48, None)
        .ok();
    assets.fira_code = rl
        .load_font_ex(thread, "assets/fonts/FiraCode-Bold.ttf", 128, None)
        .ok();

    assets.ship_names = load_names_from_file("assets/ship_names.txt").unwrap_or(vec![]);

    // for (proto, tex) in assets.part_textures.values_mut() {
    //     let filename = format!("assets/parts/{}/skin.png", proto.part_name());
    //     *tex = rl.load_texture(thread, &filename).ok();
    // }
}
