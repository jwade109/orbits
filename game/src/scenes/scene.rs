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
    MainMenu,
}

impl SceneType {
    fn on_interaction(&mut self, inter: &InteractionEvent) {
        match self {
            Self::OrbitalView(os) => os.on_interaction(inter),
            _ => (),
        }
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

    pub fn current_clicked_gui_element(&self, input: &InputState) -> Option<OnClick> {
        let a = input.position(MouseButt::Left, FrameId::Down);
        let b = input.position(MouseButt::Right, FrameId::Down);
        let p = a.or(b)?;
        let q = Vec2::new(p.x, input.screen_bounds.span.y - p.y);
        self.ui.at(q).map(|n| n.id()).flatten().cloned()
    }

    pub fn mouse_if_world<'a>(&self, input: &'a InputState) -> Option<&'a InputState> {
        let id = self.current_clicked_gui_element(input)?;
        (id == OnClick::World).then(|| input)
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

    pub fn on_interaction(&mut self, inter: &InteractionEvent) {
        &self.scene_type.on_interaction(inter);
    }
}
