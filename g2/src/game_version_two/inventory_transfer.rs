use crate::game_version_two::*;

pub struct InventoryTransferPlugin;

impl Plugin for InventoryTransferPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, process_transfers.in_set(Sets::Physics))
            .add_systems(PostUpdate, draw_transfers.in_set(Sets::Draw))
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

pub fn apply_egui_style(ui: &mut egui::Ui) {
    let x = ui.style_mut();
    x.spacing.window_margin = egui::Margin::same(40);
    x.spacing.item_spacing.y = 5.0;
    x.spacing.button_padding.x = 5.0;
    x.spacing.button_padding.y = 5.0;
    x.visuals.dark_mode = false;
    for x in &mut x.text_styles {
        x.1.size *= 1.2;
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
    transfers: Query<(Entity, &DebugInventoryTransfer)>,
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
            let transfer = DebugInventoryTransfer {
                from: primary,
                to: secondary,
                item: panel_state.item,
                count: panel_state.count,
                status: MachineStatus::Off,
            };

            commands.spawn(transfer);
        }

        for (e, transfer) in transfers {
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
pub struct DebugInventoryTransfer {
    from: Entity,
    to: Entity,
    item: Item,
    count: u64,
    status: MachineStatus,
}

fn process_transfers(
    mut transfers: Query<&mut DebugInventoryTransfer>,
    mut inventories: Query<&mut Inventory>,
) {
    for mut transfer in transfers {
        if transfer.from == transfer.to {
            continue;
        }

        let [mut src, mut dst] =
            ok_or_continue!(inventories.get_many_mut([transfer.from, transfer.to]));

        transfer.status = atomic_transfer(&mut src, &mut dst, transfer.item, transfer.count);
    }
}

const Z_INVENTORY_TRANSFER: f32 = 500.0;

fn draw_transfers(
    mut painter: ShapePainter,
    transfers: Query<&DebugInventoryTransfer>,
    transforms: Query<&GlobalTransform, With<Inventory>>,
) {
    for transfer in transfers {
        let a = ok_or_continue!(transforms.get(transfer.from));
        let b = ok_or_continue!(transforms.get(transfer.to));

        painter.reset();
        painter.set_translation(Vec3::Z * Z_INVENTORY_TRANSFER);
        painter.set_color(RED);
        painter.thickness = 12.0;
        painter.thickness_type = ThicknessType::Pixels;
        painter.line(
            a.translation().with_z(Z_INVENTORY_TRANSFER),
            b.translation().with_z(Z_INVENTORY_TRANSFER),
        );
    }
}
