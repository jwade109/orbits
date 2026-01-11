use bevy::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_egui::EguiContexts;
use game::ui::apply_egui_style;

#[derive(ScheduleLabel, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct SimTick;

#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TickSchedule {
    PerFrame(u32),
    Once,
}

#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Ticks(pub u64);

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
    let mut ticks = world.resource_mut::<TickSchedule>();

    let n = ticks.ticks_per_frame();

    if ticks.is_once() {
        ticks.pause();
    }

    let mut t = world.resource_mut::<Ticks>();
    t.0 += n as u64;

    for _ in 0..n {
        world.run_schedule(SimTick);
    }
}

pub fn tick_control_egui(
    mut contexts: EguiContexts,
    mut ticks: ResMut<TickSchedule>,
    count: Res<Ticks>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Tick Rate Control").show(ctx, |ui| {
        apply_egui_style(ui);

        ui.heading(format!("Tick Rate Control ({})", ticks.ticks_per_frame()));
        ui.label(format!("tick {}", count.0));

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
