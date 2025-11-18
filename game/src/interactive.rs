use crate::canvas::Canvas;
use crate::onclick::OnClick;
use starling::prelude::*;

pub trait Interactive: Send + Sync {
    fn on_left_mouse_down(&mut self) -> Option<OnClick>;

    fn on_left_mouse_up(&mut self) -> Option<OnClick>;

    fn on_mouse_move(&mut self, p: &mut Take<Vec2>) -> Option<OnClick>;

    fn on_key(&mut self, _key: &bevy::input::keyboard::KeyboardInput) -> Option<OnClick> {
        None
    }

    fn step(&mut self) -> Option<OnClick>;

    fn draw(&self, _canvas: &mut Canvas) {}
}
