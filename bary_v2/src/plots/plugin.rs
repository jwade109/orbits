use std::collections::VecDeque;

use bary_v1::ui::apply_egui_style;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use std::time::Duration;

use super::*;

pub fn plot_plugin(app: &mut App) {
    app.insert_resource(Plots::default());
    app.add_systems(Update, draw_plots);
    app.add_systems(EguiPrimaryContextPass, plots_egui);
}

fn average_value(vals: &VecDeque<Duration>) -> Option<Duration> {
    if vals.is_empty() {
        return None;
    }
    let sum: Duration = vals.iter().sum();
    Some(sum / vals.len() as u32)
}

fn plots_egui(mut contexts: EguiContexts, plots: Res<Plots>) {
    let ctx = contexts.ctx_mut().unwrap();

    use egui::Align2;

    egui::Window::new("Timers").show(ctx, |ui| {
        apply_egui_style(ui);
        ui.set_width(400.0);

        for (name, signal) in plots.signals() {
            ui.horizontal(|ui| {
                let mut state = false;
                ui.checkbox(&mut state, name);
                if let Some(avg) = average_value(&signal) {
                    ui.label(format!("Average: {:?}", avg));
                }
            });
        }
    });
}

fn draw_plots(
    plots: Res<Plots>,
    mut gizmos: Gizmos,
    transforms: TransformHelper,
    camera: Single<(Entity, &Camera)>,
    window: Single<&Window>,
) {
    let (entity, camera) = (camera.0, camera.1);
    let camera_transform = transforms.compute_global_transform(entity).unwrap();

    let bottom_right = window.size();
    let bottom_left = bottom_right.with_x(0.0);

    let origin = camera
        .viewport_to_world(&camera_transform, bottom_left)
        .unwrap()
        .origin
        .xy();

    let sx = 0.1;
    let sy = 1.0;

    for (_, signal) in plots.signals() {
        let linestring: Vec<_> = signal
            .iter()
            .enumerate()
            .map(|(i, v)| origin + Vec2::new(i as f32 * sx, (v.as_secs_f64() * 1000.0) as f32 * sy))
            .collect();
        gizmos.linestrip_2d(linestring, RED);
    }
}
