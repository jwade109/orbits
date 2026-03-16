use log::debug;
use raylib::prelude::*;
use std::collections::BTreeMap;

pub type MaybeTexture = Option<Texture2D>;

pub type MaybeFont = Option<Font>;

#[derive(Default)]
pub struct Assets {
    pub circle_texture: MaybeTexture,
    pub lato_regular: MaybeFont,
    pub fira_code: MaybeFont,
    pub part_textures: BTreeMap<String, Texture2D>,
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

    // for (proto, tex) in assets.part_textures.values_mut() {
    //     let filename = format!("assets/parts/{}/skin.png", proto.part_name());
    //     *tex = rl.load_texture(thread, &filename).ok();
    // }
}
