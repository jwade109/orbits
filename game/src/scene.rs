#![allow(unused)]

use crate::mouse::{FrameId, MouseButt};
use crate::ui::OnClick;
use bevy::log::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub enum SceneType {
    OrbitalView(OrbitalScene),
    DockingView(OrbiterId),
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
pub struct Scene {
    name: String,
    scene_type: SceneType,
    ui: Tree<OnClick>,
}

impl Scene {
    pub fn orbital(name: impl Into<String>, primary: PlanetId) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::OrbitalView(OrbitalScene { primary }),
            ui: Tree::new(),
        }
    }

    pub fn docking(name: impl Into<String>, primary: OrbiterId) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::DockingView(primary),
            ui: Tree::new(),
        }
    }

    pub fn main_menu() -> Self {
        Scene {
            name: "Main Menu".into(),
            scene_type: SceneType::MainMenu,
            ui: Tree::new(),
        }
    }
}

impl Scene {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn kind(&self) -> &SceneType {
        &self.scene_type
    }

    pub fn on_mouse_event(&mut self, button: MouseButt, id: FrameId) {
        info!("{} {:?} {:?}", self.name, button, id);
    }

    pub fn ui(&self) -> &Tree<OnClick> {
        &self.ui
    }

    pub fn set_ui(&mut self, ui: Tree<OnClick>) {
        self.ui = ui;
    }

    pub fn on_load(&mut self) {
        info!("On load: {}", &self.name);
    }

    pub fn on_exit(&mut self) {
        info!("On exit: {}", &self.name);
    }
}
