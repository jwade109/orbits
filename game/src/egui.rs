use crate::planetary::GameState;
use crate::ui::InteractionEvent;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use starling::orbiter::GroupId;
use std::ops::DerefMut;

pub fn ui_example_system(
    mut contexts: EguiContexts,
    state: Res<GameState>,
    mut events: EventWriter<InteractionEvent>,
    mut group_name: Local<String>,
) {
    let ctx = contexts.ctx_mut();
    // catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);
    egui::Window::new("Settings").show(ctx, |ui| {
        if ui.button("Commit mission").clicked() {
            events.send(InteractionEvent::CommitMission);
        }

        for (gid, _) in &state.constellations {
            let button = ui.button(format!("{}", gid));
            if button.clicked_by(egui::PointerButton::Primary) {
                events.send(InteractionEvent::ToggleGroup(gid.clone()));
            }
            if button.clicked_by(egui::PointerButton::Secondary) {
                events.send(InteractionEvent::DisbandGroup(gid.clone()));
            }
        }

        ui.separator();

        let mut group_name = group_name.deref_mut();
        ui.add(egui::TextEdit::singleline(group_name));

        let n_orbiters = state.track_list.len();
        let enabled = n_orbiters > 0 && !group_name.is_empty();

        ui.add_enabled_ui(enabled, |ui| {
            if ui
                .button(format!(
                    "Group {} orbiters as \"{}\"",
                    n_orbiters, &group_name
                ))
                .clicked()
            {
                events.send(InteractionEvent::CreateGroup(GroupId(group_name.clone())));
            }
        });
    });
}
