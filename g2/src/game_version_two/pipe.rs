use bevy::color::palettes::tailwind::GRAY_400;
use game::ui::apply_egui_style;

use crate::game_version_two::*;

pub struct InventoryTransferPlugin;

impl Plugin for InventoryTransferPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, process_pipes.in_set(Sets::Physics))
            .add_systems(PostUpdate, draw_pipes.in_set(Sets::Draw))
            .add_systems(EguiPrimaryContextPass, debug_ui);
    }
}

#[derive(Debug)]
struct DebugPanelState {
    item: Item,
    count: u64,
}

impl Default for DebugPanelState {
    fn default() -> Self {
        Self {
            item: Item::H2,
            count: 500,
        }
    }
}

pub fn item_dropdown(ui: &mut egui::Ui, selected: &mut Item, title: &str, filter: &ItemFilter) {
    egui::ComboBox::from_label(title)
        .selected_text(format!("{:?}", selected))
        .show_ui(ui, |ui| {
            for item in Item::all_that_passes(filter) {
                let text = format!("{:?}", item);
                ui.selectable_value(selected, item, text);
            }
        });
}

fn debug_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut panel_state: Local<DebugPanelState>,
    selected: Res<SelectedSpacecraft>,
    pipes: Query<(Entity, &Pipe)>,
) -> Result {
    let primary = match selected.selected {
        Some(id) => id,
        None => return Ok(()),
    };

    let secondary = match selected.secondary {
        Some(id) => id,
        None => return Ok(()),
    };

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Transfer Inventory").show(ctx, |ui| {
        apply_egui_style(ui);

        ui.label(format!("{:?}", panel_state));

        ui.add(
            egui::Slider::new(&mut panel_state.count, 0..=100)
                .clamping(egui::SliderClamping::Never)
                .text("Count"),
        );

        let filter = ItemFilter::Any;

        item_dropdown(ui, &mut panel_state.item, "Item", &filter);

        if ui.button("Initiate").clicked() {
            let transfer = Pipe {
                from: primary,
                to: secondary,
                item: panel_state.item,
                count: panel_state.count,
                status: MachineStatus::Off,
            };

            commands.spawn(transfer);
        }

        for (e, transfer) in pipes {
            ui.separator();
            ui.label(format!("{:#?}", transfer));
            running_status_widget(ui, transfer.status);
            if ui.button("Cancel").clicked() {
                commands.entity(e).despawn();
            }
        }
    });

    Ok(())
}

#[derive(Component, Debug)]
pub struct Pipe {
    from: Entity,
    to: Entity,
    item: Item,
    count: u64,
    status: MachineStatus,
}

fn process_pipes(mut pipe: Query<&mut Pipe>, mut inventories: Query<&mut Inventory>) {
    for mut pipe in pipe {
        if pipe.from == pipe.to {
            continue;
        }

        let [mut src, mut dst] = ok_or_continue!(inventories.get_many_mut([pipe.from, pipe.to]));

        pipe.status = atomic_transfer(&mut src, &mut dst, pipe.item, pipe.count);
    }
}

const Z_PIPE_LAYER: f32 = 0.06;

fn draw_pipes(
    mut painter: ShapePainter,
    pipes: Query<&Pipe>,
    transforms: Query<&GlobalTransform, With<Inventory>>,
) {
    for transfer in pipes {
        let a = ok_or_continue!(transforms.get(transfer.from));
        let b = ok_or_continue!(transforms.get(transfer.to));

        painter.reset();
        painter.set_translation(Vec3::Z * Z_PIPE_LAYER);

        for (color, thickness) in [(GRAY_400, 0.07)] {
            painter.set_color(color);
            painter.thickness = thickness;
            painter.cap = Cap::Square;
            painter.line(
                a.translation().with_z(Z_PIPE_LAYER),
                b.translation().with_z(Z_PIPE_LAYER),
            );
        }
    }
}

/// computes where the bend in a pipe is, based on its starting and
/// ending location. pipes that begin and end on the same x- or y-value
/// do not bend, so this will return None.
fn get_bend_location(from: impl Into<IVec2>, to: impl Into<IVec2>, x_first: bool) -> Option<IVec2> {
    let from = from.into();
    let to = to.into();

    if from.x == to.x || from.y == to.y {
        return None;
    }

    // x_first determines which axis will be traversed first.
    // x_first = true:
    //     (0, 0) to (1, 1) should pass through (1, 0)
    // x_first = false:
    //     (0, 0) to (1, 1) should pass through (0, 1)

    Some(if x_first {
        IVec2::new(to.x, from.y)
    } else {
        IVec2::new(from.x, to.y)
    })
}

#[test]
fn pipe_path_computation() {
    assert_eq!(
        get_bend_location((0, 0), (1, 1), true),
        Some(IVec2::new(1, 0))
    );

    assert_eq!(
        get_bend_location((0, 0), (1, 1), false),
        Some(IVec2::new(0, 1))
    );

    assert_eq!(
        get_bend_location((3, 2), (-4, 10), true),
        Some(IVec2::new(-4, 2))
    );

    assert_eq!(
        get_bend_location((3, 2), (-4, 10), false),
        Some(IVec2::new(3, 10))
    );
}
