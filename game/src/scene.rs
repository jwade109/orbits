#![allow(unused)]

use starling::prelude::*;

#[derive(Debug, Clone)]
pub enum SceneType {
    OrbitalView(PlanetId),
    MainMenu,
}

#[derive(Debug, Clone)]
pub struct Scene {
    name: String,
    kind: SceneType,
}

impl Scene {
    pub fn new(name: impl Into<String>, kind: SceneType) -> Self {
        Scene {
            name: name.into(),
            kind,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn kind(&self) -> &SceneType {
        &self.kind
    }
}
