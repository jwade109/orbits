use crate::onclick::OnClick;
use crate::ui::{BUTTON_HEIGHT, UI_BACKGROUND_COLOR};
use layout::layout::{Node, Size};
use starling::prelude::*;

fn tank_ui(tank: &Tank) -> Vec<Node<OnClick>> {
    let node = |text: String| Node::<OnClick>::text(Size::Grow, BUTTON_HEIGHT, text);

    vec![
        node(format!("Current mass: {}", tank.current_mass())),
        node(format!("Dry mass: {}", tank.dry_mass())),
        node(format!("Item: {:?}", tank.item())),
    ]
}

pub fn part_ui_layout(instance: &PartInstance) -> Node<OnClick> {
    let header = Node::text(
        Size::Grow,
        BUTTON_HEIGHT,
        format!("{}", instance.part().sprite_path()),
    )
    .enabled(false);

    let children = match instance.part() {
        Part::Tank(t) => tank_ui(t),
        _ => Vec::new(),
    }
    .into_iter();

    Node::new(Size::Grow, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(header)
        .with_children(children)
}
