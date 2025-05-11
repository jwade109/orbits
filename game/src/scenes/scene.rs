#![allow(unused)]

use crate::mouse::{FrameId, InputState, MouseButt};
use crate::planetary::GameState;
use crate::scenes::{CursorMode, OrbitalContext, OrbitalView};
use crate::ui::{InteractionEvent, OnClick};
use bevy::log::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub enum SceneType {
    OrbitalView(OrbitalContext),
    DockingView(OrbiterId),
    TelescopeView,
    Editor,
    MainMenu,
}

#[derive(Debug, Clone)]
pub struct Scene {
    name: String,
    scene_type: SceneType,
}

impl Scene {
    pub fn orbital(name: impl Into<String>, primary: PlanetId) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::OrbitalView(OrbitalContext::new(primary)),
        }
    }

    pub fn telescope() -> Self {
        Scene {
            name: "Telescope".into(),
            scene_type: SceneType::TelescopeView,
        }
    }

    pub fn editor() -> Self {
        Scene {
            name: "Editor".into(),
            scene_type: SceneType::Editor,
        }
    }

    pub fn docking(name: impl Into<String>, primary: OrbiterId) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::DockingView(primary),
        }
    }

    pub fn main_menu() -> Self {
        Scene {
            name: "Main Menu".into(),
            scene_type: SceneType::MainMenu,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn kind(&self) -> &SceneType {
        &self.scene_type
    }
}
