use bary_raylib::utils::{BasicApp, InputState, KB};
use raylib::{color::Color, prelude::RaylibDraw};
use std::collections::{HashMap, HashSet};

use rdev::Key::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Action {
    GoUp,
    GoDown,
    GoLeft,
    GoRight,
    Jump,
    Spawn,
}

#[derive(Debug)]
struct ActionState<T: std::hash::Hash> {
    currently_pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_pressed_debounced: HashSet<T>,
    just_released: HashSet<T>,
}

impl<T: std::hash::Hash + Copy + Eq> ActionState<T> {
    fn new(input: &InputState, bindings: &HashMap<KB, T>) -> Self {
        let currently_pressed = input
            .iter_pressed()
            .filter_map(|e| bindings.get(e))
            .map(|e| *e)
            .collect();

        let just_pressed = input
            .iter_just_pressed()
            .filter_map(|e| bindings.get(e))
            .map(|e| *e)
            .collect();

        let just_pressed_debounced = input
            .iter_just_pressed_debounced()
            .filter_map(|e| bindings.get(e))
            .map(|e| *e)
            .collect();

        let just_released = input
            .iter_just_released()
            .filter_map(|e| bindings.get(e))
            .map(|e| *e)
            .collect();

        Self {
            currently_pressed,
            just_pressed,
            just_pressed_debounced,
            just_released,
        }
    }
}

fn main() {
    let mut app = BasicApp::new("Action States");

    use Action::*;

    let bindings = HashMap::from([
        (KB::Key(KeyW), GoUp),
        (KB::Key(KeyS), GoDown),
        (KB::Key(KeyA), GoLeft),
        (KB::Key(KeyD), GoRight),
        (KB::Key(Space), Jump),
        (KB::Key(KeyG), Spawn),
    ]);

    while app.frame() {
        let action = ActionState::new(&app.input, &bindings);

        app.handle.draw(&app.thread, |mut d| {
            d.clear_background(Color::BLACK);
            let s = format!("{:#?}", &app.input);
            let s2 = format!("{:#?}", &action);
            d.draw_text(&s, 100, 100, 28, Color::WHITE);
            d.draw_text(&s2, 700, 100, 28, Color::WHITE);
        });
    }
}
