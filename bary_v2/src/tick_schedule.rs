use crate::Settings;
use bary_v1::ui::apply_egui_style;
use bevy::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_egui::EguiContexts;

#[derive(ScheduleLabel, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct SimTick;

#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TickSchedule {
    PerFrame(u32),
    Once,
}

#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct TickStatistics {
    ticks: u64,
    ticks_last_frame: u32,
    dt: std::time::Duration,
}

impl TickSchedule {
    pub fn pause(&mut self) {
        *self = Self::PerFrame(0);
    }

    pub fn unpause(&mut self) {
        *self = Self::PerFrame(1);
    }

    pub fn is_paused(&self) -> bool {
        match self {
            Self::PerFrame(0) => true,
            Self::Once => true,
            _ => false,
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused() {
            self.unpause();
        } else {
            self.pause();
        }
    }

    pub fn ticks_per_frame(&self) -> u32 {
        match self {
            TickSchedule::PerFrame(n) => *n,
            TickSchedule::Once => 1,
        }
    }

    pub fn is_once(&self) -> bool {
        *self == Self::Once
    }

    pub fn tick_once(&mut self) {
        *self = TickSchedule::Once;
    }

    pub fn set_rate(&mut self, n: u32) {
        *self = TickSchedule::PerFrame(n);
    }

    pub fn speed_up(&mut self) {
        *self = match self {
            TickSchedule::PerFrame(n) => TickSchedule::PerFrame(*n + 1),
            _ => TickSchedule::PerFrame(1),
        }
    }

    pub fn slow_down(&mut self) {
        *self = match self {
            TickSchedule::PerFrame(n) => {
                if *n > 0 {
                    TickSchedule::PerFrame(*n - 1)
                } else {
                    TickSchedule::PerFrame(0)
                }
            }
            _ => TickSchedule::PerFrame(0),
        }
    }
}

pub fn world_tick_driver_system(world: &mut World) {
    let mut ticks: Mut<'_, TickSchedule> = world.resource_mut::<TickSchedule>();

    let n = ticks.ticks_per_frame();

    if ticks.is_once() {
        ticks.pause();
    }

    let start = std::time::Instant::now();

    let mut ticks = 0;

    let mut dt = std::time::Duration::ZERO;

    for _ in 0..n {
        world.run_schedule(SimTick);
        let now = std::time::Instant::now();

        ticks += 1;

        dt = now - start;
        if dt > std::time::Duration::from_millis(12) {
            break;
        }
    }

    let mut t = world.resource_mut::<TickStatistics>();

    t.ticks += ticks;
    t.ticks_last_frame = ticks as u32;
    t.dt = dt;
}

pub fn tick_control_egui(
    mut contexts: EguiContexts,
    mut ticks: ResMut<TickSchedule>,
    stats: Res<TickStatistics>,
    settings: Res<Settings>,
) -> Result {
    if !settings.show_time_controls {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Tick Rate Control").show(ctx, |ui| {
        apply_egui_style(ui);

        ui.label(format!("tick {}", stats.ticks));
        ui.label(format!(
            "last frame: {}/{}",
            stats.ticks_last_frame,
            ticks.ticks_per_frame()
        ));
        ui.label(format!("dt: {:?}", stats.dt));

        let ptext = if ticks.is_paused() {
            "[>] Play"
        } else {
            "[||] Pause"
        };

        if ui.button(ptext).clicked() {
            ticks.toggle_pause();
        }

        if ui.button("Real Time").clicked() {
            ticks.set_rate(1);
        }

        ui.horizontal(|ui| {
            if ui.button("20").clicked() {
                ticks.set_rate(20);
            }
            if ui.button("40").clicked() {
                ticks.set_rate(40);
            }
            if ui.button("100").clicked() {
                ticks.set_rate(100);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("<<").clicked() {
                ticks.slow_down();
            }
            if ui.button(">>").clicked() {
                ticks.speed_up();
            }
        });

        if ui.button("Tick Once").clicked() {
            ticks.tick_once();
        }
    });

    Ok(())
}
