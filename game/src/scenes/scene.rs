#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneType {
    Orbital,
    DockingView,
    Telescope,
    Editor,
    MainMenu,
    CommsPanel,
    Surface,
}

#[derive(Debug, Clone)]
pub struct Scene {
    name: String,
    scene_type: SceneType,
}

impl Scene {
    pub fn orbital() -> Self {
        Scene {
            name: "Orbital".into(),
            scene_type: SceneType::Orbital,
        }
    }

    pub fn telescope() -> Self {
        Scene {
            name: "Telescope".into(),
            scene_type: SceneType::Telescope,
        }
    }

    pub fn editor() -> Self {
        Scene {
            name: "Editor".into(),
            scene_type: SceneType::Editor,
        }
    }

    pub fn docking() -> Self {
        Scene {
            name: "Docking".into(),
            scene_type: SceneType::DockingView,
        }
    }

    pub fn main_menu() -> Self {
        Scene {
            name: "Main Menu".into(),
            scene_type: SceneType::MainMenu,
        }
    }

    pub fn comms() -> Self {
        Scene {
            name: "Comms".into(),
            scene_type: SceneType::CommsPanel,
        }
    }

    pub fn surface() -> Self {
        Scene {
            name: "Surface".into(),
            scene_type: SceneType::Surface,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn kind(&self) -> &SceneType {
        &self.scene_type
    }
}
