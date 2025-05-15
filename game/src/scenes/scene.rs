use starling::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneType {
    Orbital,
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
    pub fn orbital(name: impl Into<String>) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::Orbital,
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
