use crate::game_version_two::*;

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

fn apply_egui_style(ui: &mut egui::Ui) {
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

fn add_computer_widget(ui: &mut egui::Ui, computer: &mut Computer) {
    ui.label(format!("{:#?}", computer));

    let s = if computer.on { "ON " } else { "OFF" };

    ui.separator();

    if ui.button(s).clicked() {
        computer.toggle();
    }
    running_status_widget(ui, computer.status);

    ui.separator();

    ui.label(format!("Mode: {:?}", &computer.mode));

    let before = computer.mode;

    egui::ComboBox::from_label("")
        .selected_text(format!("{:?}", computer.mode))
        .show_ui(ui, |ui| {
            for mode in enum_iterator::all::<ComputerMode>() {
                let st = format!("{:?}", mode);
                ui.selectable_value(&mut computer.mode, mode, st);
            }
        });

    match &mut computer.mode {
        ComputerMode::None => (),
        ComputerMode::Manual => (),
        ComputerMode::AttitudeHold => {
            ui.label("Attitude Hold");
            ui.horizontal(|ui| {
                ui.label("HDG");
                ui.add(egui::Slider::new(&mut computer.attitude, -5.0..=5.0));
            });
        }
        ComputerMode::VelocityHold => {
            ui.label("Velocity Hold");
            ui.horizontal(|ui| {
                ui.label("HDG");
                ui.add(egui::Slider::new(&mut computer.attitude, -5.0..=5.0));
            });
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(egui::Slider::new(&mut computer.velocity.x, -500.0..=500.0));
            });
            ui.horizontal(|ui| {
                ui.label("Y");
                ui.add(egui::Slider::new(&mut computer.velocity.y, -500.0..=500.0));
            });
        }
        ComputerMode::PositionHold => {
            ui.label("Position Hold");
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(egui::Slider::new(
                    &mut computer.position.x,
                    -1000.0..=1000.0,
                ));
            });
            ui.horizontal(|ui| {
                ui.label("Y");
                ui.add(egui::Slider::new(
                    &mut computer.position.y,
                    -1000.0..=1000.0,
                ));
            });
            ui.horizontal(|ui| {
                ui.label("HDG");
                ui.add(egui::Slider::new(&mut computer.attitude, -5.0..=5.0));
            });
        }
    }
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

        ui.horizontal(|ui| {
            if ui.button("Fill").clicked() {
                slot.fill();
            }
            if ui.button("Empty").clicked() {
                slot.empty();
            }
            if ui.button("Add").clicked() {
                if let Some(item) = slot.item() {
                    slot.store(item, 1);
                }
            }
            if ui.button("Add Lots").hovered() {
                let volume = slot.capacity();
                if let Some(item) = slot.item() {
                    let n_items = (volume / item.volume_per_unit()).floor() as u64;
                    slot.store_partial(item, n_items / 100);
                }
            }
        });

        if let Some((item, count)) = slot.contents() {
            let c = item.color().to_u8_array();
            let color = egui::Color32::from_rgb(c[0], c[1], c[2]);

            ui.horizontal(|ui| {
                let size = bevy_inspector_egui::egui::Vec2::new(10.0, 10.0);
                egui::color_picker::show_color(ui, color, size);
                ui.label(format!(
                    "{:?} {} {}/{} ({}) {}",
                    item,
                    count,
                    slot.occupied_volume(),
                    slot.capacity(),
                    slot.mass(),
                    if slot.is_full() { "*" } else { "" },
                ));
            });

            ui.add(egui::ProgressBar::new(slot.fill_percentage()).fill(color));
        } else {
            ui.label("(Empty)");
        }
    }
}

fn add_inventory_widget(
    id: Option<Entity>,
    ui: &mut egui::Ui,
    inventories: &mut Query<&mut Inventory>,
) {
    let id = if let Some(id) = id {
        id
    } else {
        return;
    };

    ui.label(format!("Entity {id}"));

    let mut inv = match inventories.get_mut(id) {
        Ok(inv) => inv,
        _ => return,
    };

    add_inv_widget(ui, &mut inv);
}

fn running_status_widget(ui: &mut egui::Ui, status: MachineStatus) {
    let color = match status {
        MachineStatus::Off => egui::Color32::GRAY,
        MachineStatus::NoRecipe => egui::Color32::RED,
        MachineStatus::Running => egui::Color32::GREEN,
        MachineStatus::NoRoom => egui::Color32::YELLOW,
        MachineStatus::Starved => egui::Color32::YELLOW,
    };

    ui.horizontal(|ui| {
        egui::color_picker::show_color(ui, color, bevy_inspector_egui::egui::Vec2::new(10.0, 10.0));
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
        commands.send_event(SetRecipe {
            target: id,
            recipe: current,
        });
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
) {
    if let Ok((instance, _)) = parts.get(e) {
        ui.collapsing("Part Data", |ui| {
            ui.label(format!("{:#?}", instance.0));
        });
    }

    if let Ok(mut inventory) = inventories.get_mut(e) {
        ui.separator();
        ui.heading("Inventory");
        add_inv_widget(ui, &mut inventory);
    }

    if let Ok(mut thruster) = thrusters.get_mut(e) {
        ui.separator();
        ui.heading("Thruster");
        add_thruster_widget(ui, &mut thruster);
    }

    if let Ok(mut computer) = computers.get_mut(e) {
        ui.separator();
        ui.heading("Computer");
        add_computer_widget(ui, &mut computer);
    }

    if let Ok(mut machine) = machines.get_mut(e) {
        ui.separator();
        ui.heading("Machine");
        add_machine_widget(e, commands, ui, &mut machine);
    }

    if let Ok(mut docking_port) = docking_ports.get_mut(e) {
        ui.separator();
        ui.heading("Docking Port");
        ui.label(format!("{:#?}", docking_port));
        // add_machine_widget(e, commands, ui, &mut machine);
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
    con: Query<&mut ConstructionState>,
    cursor: Res<CursorInfo>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let e = cursor.hovered.or(cursor.selected);

    if let Some(e) = e {
        egui::panel::SidePanel::new(Side::Right, "Part Info").show(ctx, |ui| {
            apply_egui_style(ui);
            ui.set_width(350.0);

            if let Some(e) = cursor.selected {
                part_ui(
                    ui,
                    e,
                    &mut commands,
                    parts,
                    &mut inventories,
                    &mut thrusters,
                    &mut computers,
                    &mut machines,
                    &mut docking_ports,
                );
            }

            if let Some(e) = cursor.hovered {
                part_ui(
                    ui,
                    e,
                    &mut commands,
                    parts,
                    &mut inventories,
                    &mut thrusters,
                    &mut computers,
                    &mut machines,
                    &mut docking_ports,
                );
            }
        });
    }

    egui::panel::SidePanel::new(egui::containers::panel::Side::Left, "Debug").show(ctx, |ui| {
        apply_egui_style(ui);
        ui.set_width(350.0);

        ui.collapsing("Machine", |ui| {
            machine_editor_widget(&mut commands, cursor.selected, ui, machines);
        });

        ui.collapsing("Inventory", |ui| {
            add_inventory_widget(cursor.selected, ui, &mut inventories);
        });

        ui.collapsing("Construction", |ui| {
            con_state_widget(cursor.selected, ui, con);
        });

        ui.collapsing("Text Notifications", |ui| {
            ui.text_edit_multiline(&mut state.message_text);
            ui.color_edit_button_rgb(&mut state.message_color);

            if ui.button("Spawn Text").clicked() {
                commands.send_event(SpawnAnimText {
                    text: state.message_text.clone(),
                    color: Srgba::from_f32_array([
                        state.message_color[0],
                        state.message_color[1],
                        state.message_color[2],
                        1.0,
                    ]),
                    pos: None,
                });
            }
        });

        let x = ui.collapsing("Selected Grid", |ui| {
            if let Some((_, parent)) = cursor.selected.map(|e| parts.get(e).ok()).flatten() {
                ui.label(format!("Spacecraft: {:#?}", parent.0));
                if let Ok(grid) = grids.get(parent.0) {
                    ui.label(format!("{:#?}", grid));
                }
            }
        });

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
                commands.send_event(SpacecraftEvent::SpawnVehicle {
                    name: state.sc_name.clone(),
                    pos: state.sc_pos,
                    angle: rand(-0.2, 0.3),
                });
            }

            if ui.button("Spawn Part").clicked() {
                commands.send_event(SpacecraftEvent::SpawnPart {
                    name: state.sc_name.clone(),
                    pos: state.sc_pos,
                    angle: rand(-0.2, 0.3),
                });
            }
        });
    });
    Ok(())
}
