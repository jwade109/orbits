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
    // let ctx = contexts.ctx_mut();
    // // catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);
    // egui::Window::new("Settings").show(ctx, |ui| {
    //     if ui.button("Commit mission").clicked() {
    //         events.send(InteractionEvent::CommitMission);
    //     }
    // });
}
