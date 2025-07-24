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

fn tank_ui(
    button_height: f32,
    id: PartId,
    tank: &TankModel,
    data: &TankInstanceData,
) -> Vec<Node<OnClick>> {
    vec![
        text_node(
            button_height,
            format!("Dry mass: {}", tank.dry_mass()),
            None,
        ),
        text_node(
            button_height,
            format!("Current mass: {}", data.contents_mass()),
            None,
        ),
        text_node(button_height, format!("Item: {:?}", data.item()), None),
        text_node(button_height, "Clear Contents", OnClick::ClearContents(id)),
    ]
}

fn cargo_ui(
    button_height: f32,
    id: PartId,
    cargo: &Cargo,
    data: &CargoInstanceData,
) -> Vec<Node<OnClick>> {
    [
        text_node(
            button_height,
            format!("Capacity: {}", cargo.capacity_mass()),
            None,
        ),
        text_node(button_height, "Clear Contents", OnClick::ClearContents(id)),
    ]
    .into_iter()
    .chain(
        data.contents()
            .map(|e| text_node(button_height, format!("{:?} {}", e.0, e.1), None)),
    )
    .collect()
}

fn machine_ui(
    button_height: f32,
    id: PartId,
    _machine: &Machine,
    data: &MachineInstanceData,
) -> Vec<Node<OnClick>> {
    vec![
        text_node(
            button_height,
            format!("Recipe: {:?}", data.recipe),
            OnClick::SetRecipe(id, RecipeListing::random()),
        ),
        text_node(
            button_height,
            format!("Progress: {:0.1}%", data.percent_complete() * 100.0),
            None,
        ),
    ]
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

    let children = match instance.variant() {
        InstantiatedPartVariant::Tank(t, d) => tank_ui(button_height, id, t, d),
        InstantiatedPartVariant::Cargo(c, d) => cargo_ui(button_height, id, c, d),
        InstantiatedPartVariant::Machine(m, d) => machine_ui(button_height, id, m, d),
        _ => Vec::new(),
    }
    .into_iter();

    Node::new(Size::Grow, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR)
        .with_child(header)
        .with_children(children)
}
