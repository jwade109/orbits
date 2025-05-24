use crate::onclick::OnClick;
use crate::planetary::GameState;
use crate::scenes::{Render, StaticSpriteDescriptor, TextLabel};
use crate::ui::{BUTTON_HEIGHT, UI_BACKGROUND_COLOR};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use layout::layout::{Node, Tree};
use starling::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CommsContext {
    connections: HashMap<OrbiterId, HashSet<OrbiterId>>,
}

impl Default for CommsContext {
    fn default() -> Self {
        Self {
            connections: HashMap::from([
                (OrbiterId(12), HashSet::from([OrbiterId(14), OrbiterId(21)])),
                (OrbiterId(9), HashSet::from([OrbiterId(20), OrbiterId(3)])),
            ]),
        }
    }
}

fn interactive_numerical_display(mut num: i64, inset: f32) -> Node<OnClick> {
    let mut wrapper = Node::fit()
        .with_padding(0.0)
        .with_child_gap(2.0)
        .with_color(UI_BACKGROUND_COLOR);

    if inset > 0.0 {
        wrapper.add_child(Node::new(inset, BUTTON_HEIGHT).invisible());
    }

    let mut children = vec![];

    while num > 0 {
        let i = num % 10;
        let s = format!("{}", i);
        let disp = Node::button(s, OnClick::Nullopt, 30, BUTTON_HEIGHT);
        children.push(disp);
        num /= 10;
    }

    wrapper.add_children(children.into_iter().rev());

    wrapper
}

impl Render for CommsContext {
    fn background_color(_state: &GameState) -> Srgba {
        TEAL.with_luminance(0.1)
    }

    fn draw_gizmos(_gizmos: &mut Gizmos, _state: &GameState) -> Option<()> {
        None
    }

    fn sprites(_state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        None
    }

    fn text_labels(_state: &GameState) -> Option<Vec<TextLabel>> {
        None
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        let mut t = Tree::new();

        let dims = state.input.screen_bounds.span;

        let mut root = Node::new(dims.x, dims.y).invisible().down().tight();

        root.add_child(crate::ui::top_bar(state));

        let mut wrapper = Node::grow().invisible().down();

        for (src, dsts) in &state.coms_context.connections {
            let n = interactive_numerical_display(src.0, 0.0);
            wrapper.add_child(n);
            for dst in dsts {
                let n = interactive_numerical_display(dst.0, BUTTON_HEIGHT / 2.0);
                wrapper.add_child(n);
            }
        }

        root.add_child(wrapper);

        t.add_layout(root, Vec2::ZERO);

        Some(t)
    }
}
