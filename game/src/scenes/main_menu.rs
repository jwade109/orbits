#![allow(unused)]

use crate::onclick::OnClick;
use crate::planetary::GameState;
use crate::scenes::{Render, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use layout::layout::{Node, Size, Tree};
use starling::prelude::*;
use std::collections::HashMap;

pub struct MainMenuContext;

impl Default for MainMenuContext {
    fn default() -> Self {
        MainMenuContext {}
    }
}

impl Render for MainMenuContext {
    fn background_color(state: &GameState) -> Srgba {
        BLACK
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        None
    }

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        None
    }

    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let dims = state.input.screen_bounds.span;
        let time = compile_time::datetime_str!();
        let s = format!("Compiled on {}", time).to_uppercase();
        let p = Vec2::new(-dims.x / 2.0 + 200.0, -dims.y / 2.0 + 100.0);

        Some(vec![TextLabel::new(s, p, 0.6)])
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        None
    }
}
