use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct GlyphInfo {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "originX")]
    pub origin_x: i32,
    #[serde(rename = "originY")]
    pub origin_y: i32,
    pub advance: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FontInfo {
    pub name: String,
    pub size: u32,
    pub bold: bool,
    pub italic: bool,
    pub width: u32,
    pub height: u32,
    pub characters: BTreeMap<char, GlyphInfo>,
}

impl FontInfo {
    pub fn from_file(path: &str) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn get_sample_range(&self, c: char) -> Option<TextureSampleRange> {
        let code_point = self.characters.get(&c)?;

        Some(TextureSampleRange {
            origin_x: code_point.x,
            origin_y: code_point.y,
            sample_width: code_point.width,
            sample_height: code_point.height,
            image_width: self.width,
            image_height: self.height,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TextureSampleRange {
    pub origin_x: u32,
    pub origin_y: u32,
    pub sample_width: u32,
    pub sample_height: u32,
    pub image_width: u32,
    pub image_height: u32,
}
