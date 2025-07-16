use crate::onclick::OnClick;
use crate::ui::{BUTTON_HEIGHT, UI_BACKGROUND_COLOR};
use layout::layout::{Node, Size};
use starling::prelude::*;

fn text_node(text: String) -> Node<OnClick> {
    Node::<OnClick>::text(Size::Grow, BUTTON_HEIGHT, text)
}

fn tank_ui(tank: &TankModel, data: &TankInstanceData) -> Vec<Node<OnClick>> {
    vec![
        text_node(format!("Dry mass: {}", tank.dry_mass())),
        text_node(format!("Current mass: {}", data.contents_mass())),
        text_node(format!("Item: {:?}", data.item())),
    ]
}

fn cargo_ui(cargo: &Cargo, data: &CargoInstanceData) -> Vec<Node<OnClick>> {
    data.contents()
        .map(|e| text_node(format!("{:?} {}", e.0, e.1)))
        .collect()
}

pub fn part_ui_layout(instance: &InstantiatedPart) -> Node<OnClick> {
    let header = Node::text(
        Size::Grow,
        BUTTON_HEIGHT,
        format!("{}", instance.prototype().sprite_path()),
    )
    .enabled(false);

    let children = match instance.variant() {
        InstantiatedPartVariant::Tank(t, d) => tank_ui(t, d),
        InstantiatedPartVariant::Cargo(c, d) => cargo_ui(c, d),
        _ => Vec::new(),
    }
    .into_iter();

    Node::new(Size::Grow, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(header)
        .with_children(children)
}
