use crate::*;
use bary_core::prelude::*;
use bary_v1::ui::apply_egui_style;
use bevy::prelude::*;

pub struct DebugPanelState {
    message_color: [f32; 3],
    message_text: String,
    sc_name: String,
    sc_pos: Vec2,
}

impl Default for DebugPanelState {
    fn default() -> Self {
        Self {
            message_color: [0.2, 0.2, 1.0],
            message_text: "This is some example text!\nIt can contain newlines.".to_string(),
            sc_name: "pollux".to_string(),
            sc_pos: Vec2::Y * 50.0,
        }
    }
}

fn con_state_widget(id: Option<Entity>, ui: &mut egui::Ui, mut con: Query<&mut ConstructionState>) {
    let id = if let Some(id) = id {
        id
    } else {
        return;
    };
    ui.label(format!("Entity {id}"));

    let mut con = match con.get_mut(id) {
        Ok(con) => con,
        _ => return,
    };

    ui.label(format!("{:#?}", con));

    let l = con.last;

    ui.add(egui::Slider::new(&mut con.current, 0..=l));

    ui.add(egui::Checkbox::new(&mut con.should_build, "Build"));
}

fn add_thruster_widget(ui: &mut egui::Ui, thruster: &mut Thruster) {
    ui.label(format!("{:#?}", thruster));

    let s = if thruster.on { "ON " } else { "OFF" };

    if ui.button(s).clicked() {
        thruster.toggle();
    }

    running_status_widget(ui, thruster.status);
}

fn add_computer_widget(
    ui: &mut egui::Ui,
    e: Entity,
    computer: &mut Computer,
    commands: &mut Commands,
) {
    let s = if computer.on { "ON " } else { "OFF" };

    ui.separator();

    if ui.button(s).clicked() {
        computer.toggle();
    }
    running_status_widget(ui, computer.status);

    ui.separator();

    ui.label(format!("Iters: {}", computer.iters));
    ui.label(format!("Mode: {:?}", &computer.mode));

    ui.collapsing("Control Vector", |ui| {
        ui.label(format!("{:#?}", &computer.vehicle_control));
    });

    ui.collapsing("Control Status", |ui| {
        ui.label(format!("{:#?}", &computer.control_status));
    });

    if ui.button("Hold Here").clicked() {
        commands.trigger(HoldHereCommand(e));
    }

    egui::ComboBox::from_label("")
        .selected_text(format!("{:?}", computer.mode))
        .show_ui(ui, |ui| {
            for mode in enum_iterator::all::<ComputerMode>() {
                let st = format!("{:?}", mode);
                ui.selectable_value(&mut computer.mode, mode, st);
            }
        });

    match &mut computer.mode {
        ComputerMode::Idle => (),
        ComputerMode::Manual => (),
        ComputerMode::AttitudeHold => {
            ui.label("Attitude Hold");
            ui.horizontal(|ui| {
                ui.label("HDG");
                ui.add(
                    egui::Slider::new(&mut computer.attitude, -5.0..=5.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
        }
        ComputerMode::VelocityHold => {
            ui.label("Velocity Hold");
            ui.horizontal(|ui| {
                ui.label("HDG");
                ui.add(
                    egui::Slider::new(&mut computer.attitude, -5.0..=5.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(
                    egui::Slider::new(&mut computer.velocity.x, -50.0..=50.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Y");
                ui.add(
                    egui::Slider::new(&mut computer.velocity.y, -50.0..=50.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
        }
        ComputerMode::PositionHold => {
            ui.label("Position Hold");
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(
                    egui::Slider::new(&mut computer.position.x, -50.0..=50.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Y");
                ui.add(
                    egui::Slider::new(&mut computer.position.y, -50.0..=50.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
            ui.horizontal(|ui| {
                ui.label("HDG");
                ui.add(
                    egui::Slider::new(&mut computer.attitude, -5.0..=5.0)
                        .clamping(egui::SliderClamping::Never),
                );
            });
        }
    }
}

pub fn toggle_on_off_button(ui: &mut egui::Ui, state: &mut bool) {
    if ui
        .button(if *state { "Turn Off" } else { "Turn On" })
        .clicked()
    {
        *state = !*state;
    }
}

fn add_excavator_widget(ui: &mut egui::Ui, e: Entity, ex: &mut Excavator) {
    ui.heading(format!("Excavator {}", e));

    toggle_on_off_button(ui, &mut ex.is_on);
    running_status_widget(ui, ex.status);
    running_status_widget(ui, ex.last_op_status);

    let pct = ex.timer.fraction();
    ui.add(egui::ProgressBar::new(pct));
}

fn add_inv_widget(ui: &mut egui::Ui, inv: &mut Inventory) {
    ui.label(format!(
        "Mass: {}, {} / {}",
        inv.mass(),
        inv.occupied_volume(),
        inv.capacity()
    ));

    for slot in inv.slots_mut() {
        ui.separator();

        if let Some(name) = slot.name() {
            ui.label(name.to_uppercase());
        }

        let item = slot.item();
        let count = slot.contents().map(|c| c.1).unwrap_or(0);
        let c = item.map(|c| c.color()).unwrap_or(GRAY).to_u8_array();
        let color = egui::Color32::from_rgb(c[0], c[1], c[2]);

        let filter = slot.filter();

        ui.horizontal(|ui| {
            let size = egui::Vec2::new(10.0, 10.0);
            egui::color_picker::show_color(ui, color, size);
            ui.label(format!(
                "{:?} {} {}/{} ({}) {} %{:?}",
                item,
                count,
                slot.occupied_volume(),
                slot.capacity(),
                slot.mass(),
                if slot.is_full() { "*" } else { "" },
                filter,
            ));
        });

        ui.add(egui::ProgressBar::new(slot.fill_percentage()).fill(color));
    }
}

pub fn running_status_widget(ui: &mut egui::Ui, status: MachineStatus) {
    let color = match status {
        MachineStatus::Off => egui::Color32::GRAY,
        MachineStatus::NoRecipe => egui::Color32::RED,
        MachineStatus::Running => egui::Color32::GREEN,
        MachineStatus::NoRoom => egui::Color32::YELLOW,
        MachineStatus::Starved => egui::Color32::YELLOW,
        MachineStatus::Disconnected => egui::Color32::ORANGE,
    };

    ui.horizontal(|ui| {
        egui::color_picker::show_color(ui, color, egui::Vec2::new(10.0, 10.0));
        if status.is_running() {
            ui.add(egui::Spinner::new().color(color));
        }
        ui.label(format!("{:?}", status));
    });
}

fn add_machine_widget(id: Entity, commands: &mut Commands, ui: &mut egui::Ui, mac: &mut Machine) {
    if ui
        .button(if mac.enabled { "Turn Off" } else { "Turn On" })
        .clicked()
    {
        mac.enabled = !mac.enabled;
    }

    ui.add(egui::Slider::new(&mut mac.required_steps, 3..=2000));

    if let Some(recipe) = mac.recipe() {
        if recipe.input_count() > 0 {
            ui.label("Consumes:");
        }

        for (item, count) in recipe.inputs() {
            ui.label(format!(
                "  {:?}: {} ({})",
                item,
                count,
                item.mass_per_unit() * count
            ));
        }

        if recipe.output_count() > 0 {
            ui.label("Produces:");
        }

        for (item, count) in recipe.outputs() {
            ui.label(format!(
                "  {:?}: {} ({})",
                item,
                count,
                item.mass_per_unit() * count
            ));
        }
    }

    ui.label(format!("{} finished", mac.products_finished));

    running_status_widget(ui, mac.status);

    ui.horizontal(|ui| {
        ui.add(egui::ProgressBar::new(mac.progress()));
    });

    let recipes: Vec<_> = RecipeListing::all()
        .map(|l| (l, format!("{:?}", l)))
        .collect();

    let before = mac.recipe;
    let mut current = before;

    egui::ComboBox::from_label("")
        .selected_text(format!("{:?}", current))
        .show_ui(ui, |ui| {
            for (listing, name) in &recipes {
                ui.selectable_value(&mut current, *listing, name);
            }
        });

    if current != before {
        mac.set_recipe(current);
    }
}

fn machine_editor_widget(
    commands: &mut Commands,
    id: Option<Entity>,
    ui: &mut egui::Ui,
    mut machines: Query<&mut Machine>,
) {
    let id = if let Some(id) = id {
        id
    } else {
        return;
    };
    ui.label(format!("Entity: {id}"));

    let mut mac = match machines.get_mut(id) {
        Ok(mac) => mac,
        _ => return,
    };

    add_machine_widget(id, commands, ui, &mut mac);
}

pub fn part_ui(
    ui: &mut egui::Ui,
    e: Entity,
    commands: &mut Commands,
    parts: Query<(&PartInstance, &ChildOf)>,
    inventories: &mut Query<&mut Inventory>,
    thrusters: &mut Query<&mut Thruster>,
    computers: &mut Query<&mut Computer>,
    machines: &mut Query<&mut Machine>,
    docking_ports: &mut Query<&mut DockingPort>,
    excavators: &mut Query<&mut Excavator>,
) {
    if let Ok(mut excavator) = excavators.get_mut(e) {
        add_excavator_widget(ui, e, &mut excavator);
    }

    if let Ok(mut inventory) = inventories.get_mut(e) {
        ui.heading("Inventory");
        add_inv_widget(ui, &mut inventory);
    }

    if let Ok(mut thruster) = thrusters.get_mut(e) {
        ui.heading("Thruster");
        add_thruster_widget(ui, &mut thruster);
    }

    if let Ok(mut computer) = computers.get_mut(e) {
        ui.heading("Computer");
        add_computer_widget(ui, e, &mut computer, commands);
    }

    if let Ok(mut machine) = machines.get_mut(e) {
        ui.heading("Machine");
        add_machine_widget(e, commands, ui, &mut machine);
    }

    if let Ok(docking_port) = docking_ports.get_mut(e) {
        ui.heading(format!("Docking Port {}", e));
        ui.label(format!("{:#?}", docking_port));
        // add_machine_widget(e, commands, ui, &mut machine);
    }

    if let Ok((instance, _)) = parts.get(e) {
        ui.collapsing("Part Data", |ui| {
            ui.label(format!("{:#?}", instance.0));
        });
    }
}

pub fn egui_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut state: Local<DebugPanelState>,
    parts: Query<(&PartInstance, &ChildOf)>,
    grids: Query<&SpacecraftGrid>,
    mut inventories: Query<&mut Inventory>,
    mut thrusters: Query<&mut Thruster>,
    mut computers: Query<&mut Computer>,
    mut machines: Query<&mut Machine>,
    mut docking_ports: Query<&mut DockingPort>,
    mut excavators: Query<&mut Excavator>,
    mut settings: ResMut<Settings>,
    cursor: Res<SelectedSpacecraft>,
    mut mouse: ResMut<CursorWorldPosition>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    if let Some(e) = cursor.primary {
        egui::Window::new("Part Info").show(ctx, |ui| {
            apply_egui_style(ui);
            ui.set_width(350.0);

            if let Some(e) = cursor.primary {
                part_ui(
                    ui,
                    e.part.entity,
                    &mut commands,
                    parts,
                    &mut inventories,
                    &mut thrusters,
                    &mut computers,
                    &mut machines,
                    &mut docking_ports,
                    &mut excavators,
                );
            }
        });
    }

    egui::Window::new("Settings").show(ctx, |ui| {
        apply_egui_style(ui);
        ui.checkbox(&mut settings.draw_spatial_lut, "draw_spatial_lut");
        ui.checkbox(&mut settings.draw_spacecraft_grids, "draw_spacecraft_grids");
        ui.checkbox(&mut settings.draw_terrain_rgb, "draw_terrain_rgb");
        ui.checkbox(&mut settings.show_wireframes, "show_wireframes");
        ui.checkbox(&mut settings.draw_inventories, "draw_inventories");
        ui.checkbox(&mut settings.draw_inventory_cons, "draw_inventory_cons");
        ui.checkbox(&mut settings.draw_blueprints, "draw_blueprints");
        ui.checkbox(&mut settings.draw_docking_info, "draw_docking_info");
        ui.checkbox(&mut settings.draw_camera_debug, "draw_camera_debug");
        ui.checkbox(&mut settings.dig_with_mouse, "dig_with_mouse");
        ui.checkbox(&mut settings.rotation_locked, "rotation_locked");
        ui.checkbox(&mut settings.infinite_fuel, "infinite_fuel");
        ui.checkbox(&mut settings.show_terrain_info, "show_terrain_info");
        ui.checkbox(&mut settings.show_time_controls, "show_time_controls");
        ui.checkbox(&mut settings.show_cursor_info, "show_cursor_info");
    });

    if settings.show_cursor_info {
        egui::Window::new("Cursor Info").show(ctx, |ui| {
            ui.label(format!("{:#?}", cursor));
        });
    }

    if false {
        egui::panel::SidePanel::new(egui::containers::panel::Side::Left, "Debug").show(ctx, |ui| {
            apply_egui_style(ui);
            ui.set_width(350.0);

            ui.collapsing("Text Notifications", |ui| {
                ui.text_edit_multiline(&mut state.message_text);
                ui.color_edit_button_rgb(&mut state.message_color);

                if ui.button("Spawn Text").clicked() {
                    commands.write_message(SpawnAnimText {
                        text: state.message_text.clone(),
                        color: Srgba::from_f32_array([
                            state.message_color[0],
                            state.message_color[1],
                            state.message_color[2],
                            1.0,
                        ]),
                        pos: None,
                        target: None,
                    });
                }
            });

            if let Some(sel) = cursor.primary {
                ui.separator();
                if let Ok(grid) = grids.get(sel.grid.entity) {
                    ui.label(format!("{:#?}", grid));
                }
                ui.separator();
            }

            ui.collapsing("Spawn Spacecraft", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Model/Partname: ");
                    ui.text_edit_singleline(&mut state.sc_name);
                    ui.label("X: ");
                    ui.add(egui::DragValue::new(&mut state.sc_pos.x));
                    ui.label("Y: ");
                    ui.add(egui::DragValue::new(&mut state.sc_pos.y));
                });

                if ui.button("Spawn Spacecraft").clicked() {
                    commands.trigger(SpacecraftEvent::SpawnVehicle {
                        blueprint_name: state.sc_name.clone(),
                        ship_name: "Random Name".to_string(),
                        pos: state.sc_pos,
                        angle: rand(-0.2, 0.3),
                    });
                }

                if ui.button("Spawn Part").clicked() {
                    commands.trigger(SpacecraftEvent::SpawnPart {
                        name: state.sc_name.clone(),
                        pos: state.sc_pos,
                        angle: rand(-0.2, 0.3),
                    });
                }
            });
        });
    }

    mouse.on_egui = ctx.is_pointer_over_area();

    Ok(())
}
