use crate::onclick::OnClick;
use starling::prelude::*;

pub trait Interactive {
    fn on_left_mouse_down(&mut self) -> Option<OnClick>;

    fn on_left_mouse_up(&mut self) -> Option<OnClick>;

    fn on_mouse_move(&mut self, p: &mut Take<Vec2>);

    fn step(&mut self);
}
