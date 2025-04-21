#![allow(unused)]

use crate::mouse::{FrameId, MouseButt};
use crate::ui::OnClick;
use bevy::log::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub enum SceneType {
    OrbitalView(OrbitalScene),
    MainMenu,
}

#[derive(Debug, Clone)]
pub struct OrbitalScene {
    primary: PlanetId,
}

impl OrbitalScene {
    fn new(primary: PlanetId) -> Self {
        Self { primary }
    }
}

#[derive(Debug, Clone)]
pub struct BasicScene {
    name: String,
    scene_type: SceneType,
    ui: Tree<OnClick>,
}

pub trait Scene {
    fn name(&self) -> &String;

    fn kind(&self) -> &SceneType;

    fn on_mouse_event(&mut self, button: MouseButt, id: FrameId);

    fn ui(&self) -> &Tree<OnClick>;

    fn set_ui(&mut self, ui: Tree<OnClick>);

    fn on_load(&mut self);

    fn on_exit(&mut self);
}

impl BasicScene {
    pub fn orbital(name: impl Into<String>, primary: PlanetId) -> Self {
        BasicScene {
            name: name.into(),
            scene_type: SceneType::OrbitalView(OrbitalScene { primary }),
            ui: Tree::new(),
        }
    }

    pub fn main_menu() -> Self {
        BasicScene {
            name: "Main Menu".into(),
            scene_type: SceneType::MainMenu,
            ui: Tree::new(),
        }
    }
}

impl Scene for BasicScene {
    fn name(&self) -> &String {
        &self.name
    }

    fn kind(&self) -> &SceneType {
        &self.scene_type
    }

    fn on_mouse_event(&mut self, button: MouseButt, id: FrameId) {
        info!("{} {:?} {:?}", self.name, button, id);
    }

    fn ui(&self) -> &Tree<OnClick> {
        &self.ui
    }

    fn set_ui(&mut self, ui: Tree<OnClick>) {
        self.ui = ui;
    }

    fn on_load(&mut self) {
        info!("On load: {}", &self.name);
    }

    fn on_exit(&mut self) {
        info!("On exit: {}", &self.name);
    }
}
