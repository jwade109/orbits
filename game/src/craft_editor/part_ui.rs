use crate::onclick::OnClick;
use crate::ui::UI_BACKGROUND_COLOR;
use layout::layout::{Node, Size};
use starling::prelude::*;

fn text_node(
    button_height: f32,
    text: impl Into<String>,
    onclick: impl Into<Option<OnClick>>,
) -> Node<OnClick> {
    let onclick = onclick.into();
    if let Some(onclick) = onclick {
        Node::button(text, onclick, Size::Grow, button_height)
    } else {
        Node::<OnClick>::text(Size::Grow, button_height, text)
    }
}

pub fn part_ui_layout(
    button_height: f32,
    id: PartId,
    instance: &InstantiatedPart,
) -> Node<OnClick> {
    let header = Node::text(
        Size::Grow,
        button_height,
        format!("{:?} {}", id, instance.prototype().sprite_path()),
    )
    .enabled(false);

    Node::new(Size::Grow, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(header)
}
