use crate::layout::Tree;

pub trait Interactable<Message> {
    fn show(&self) -> Tree<Message>;

    fn update(&mut self, message: Message);
}
