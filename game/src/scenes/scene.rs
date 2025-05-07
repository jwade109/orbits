#![allow(unused)]

use crate::mouse::{FrameId, InputState, MouseButt};
use crate::planetary::GameState;
use crate::scenes::{CursorMode, OrbitalContext, OrbitalView, TelescopeContext};
use crate::ui::{InteractionEvent, OnClick};
use bevy::log::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub enum SceneType {
    OrbitalView(OrbitalContext),
    DockingView(OrbiterId),
    TelescopeView(TelescopeContext),
    Editor,
    MainMenu,
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
            scene_type: SceneType::OrbitalView(OrbitalContext::new(primary)),
            ui: Tree::new(),
        }
    }

    pub fn telescope() -> Self {
        Scene {
            name: "Telescope".into(),
            scene_type: SceneType::TelescopeView(TelescopeContext::new()),
            ui: Tree::new(),
        }
    }

    pub fn editor() -> Self {
        Scene {
            name: "Editor".into(),
            scene_type: SceneType::Editor,
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

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn kind(&self) -> &SceneType {
        &self.scene_type
    }

    // TODO deduplicate
    pub fn is_hovering_over_ui(&self, input: &InputState) -> bool {
        let wb = input.screen_bounds.span;
        let p = match input.position(MouseButt::Hover, FrameId::Current) {
            Some(p) => p,
            None => return false,
        };
        self.ui.at(p, wb).map(|n| n.is_visible()).unwrap_or(false)
    }

    pub fn mouse_if_not_on_gui<'a>(&self, input: &'a InputState) -> Option<&'a InputState> {
        let is_on_world = !self.is_hovering_over_ui(input);
        is_on_world.then(|| input)
    }

    pub fn orbital_view<'a>(&'a self, input: &'a InputState) -> Option<OrbitalView<'a>> {
        match &self.scene_type {
            SceneType::OrbitalView(info) => Some(OrbitalView {
                info,
                input,
                scene: &self,
            }),
            _ => None,
        }
    }

    pub fn ui(&self) -> &Tree<OnClick> {
        &self.ui
    }

    pub fn set_ui(&mut self, ui: Tree<OnClick>) {
        self.ui = ui;
    }
}
